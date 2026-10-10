// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod classifier;
mod tagger;
mod downloader;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use downloader::{fetch_metadata, download_item, get_ytdlp_bin, VideoMetadata};
use tokio::sync::Semaphore;
use tagger::find_ffmpeg;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSettings {
    pub download_dir: String,
    pub default_mode: String,
    pub default_audio_format: String,
    pub default_video_format: String,
    pub default_category: String,
    pub auto_open: bool,
    pub auto_embed: bool,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_downloads: usize,
    #[serde(default)]
    pub cookies_from_browser: Option<String>,
    #[serde(default)]
    pub cookies_file_path: Option<String>,
    #[serde(default)]
    pub download_subtitles: bool,
    #[serde(default = "default_sub_langs")]
    pub subtitle_languages: String,
    #[serde(default = "default_dup_action")]
    pub duplicate_action: String,
    #[serde(default = "default_true")]
    pub auto_fetch_lyrics: bool,
}

fn default_max_concurrent() -> usize { 3 }
fn default_sub_langs() -> String { "en".into() }
fn default_dup_action() -> String { "skip".into() }
fn default_true() -> bool { true }

impl Default for UserSettings {
    fn default() -> Self {
        let default_dir = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("./Downloads"))
            .join("OSMIIC")
            .to_string_lossy()
            .to_string();

        Self {
            download_dir: default_dir,
            default_mode: "audio".into(),
            default_audio_format: "mp3".into(),
            default_video_format: "mp4".into(),
            default_category: "Auto".into(),
            auto_open: false,
            auto_embed: true,
            max_concurrent_downloads: 3,
            cookies_from_browser: Some("none".into()),
            cookies_file_path: None,
            download_subtitles: false,
            subtitle_languages: "en".into(),
            duplicate_action: "skip".into(),
            auto_fetch_lyrics: true,
        }
    }
}

struct AppState {
    downloads_dir: Arc<Mutex<PathBuf>>,
    active_cancels: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    settings: Arc<Mutex<UserSettings>>,
    download_semaphore: Arc<Semaphore>,
}

fn get_settings_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("./.config"))
        .join("osmiic")
        .join("settings.json")
}

fn load_settings_from_disk() -> UserSettings {
    let path = get_settings_file_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<UserSettings>(&content) {
                Ok(settings) => return settings,
                Err(e) => {
                    eprintln!("Warning: Failed to parse settings.json: {}. Backing up corrupted settings.", e);
                    let backup_path = path.with_extension("json.bak");
                    let _ = std::fs::rename(&path, backup_path);
                }
            }
        }
    }
    UserSettings::default()
}

fn save_settings_to_disk(settings: &UserSettings) -> Result<(), String> {
    let path = get_settings_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_metadata(url: String, language: String) -> Result<VideoMetadata, String> {
    tokio::task::spawn_blocking(move || fetch_metadata(&url, &language))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Debug, Deserialize)]
pub struct StartDownloadArgs {
    pub id: String,
    pub url: String,
    pub mode: String,
    pub format: String,
    pub quality: String,
    pub language: String,
    pub title: Option<String>,
    pub cookies_from_browser: Option<String>,
    pub cookies_file_path: Option<String>,
    pub download_subtitles: Option<bool>,
    pub subtitle_languages: Option<String>,
    pub duplicate_action: Option<String>,
}

#[tauri::command]
async fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: StartDownloadArgs,
) -> Result<String, String> {
    let download_dir = {
        let guard = state.downloads_dir.lock().unwrap();
        guard.clone()
    };

    let id = request.id.clone();
    let cancel_token = Arc::new(AtomicBool::new(false));
    {
        let mut cancels = state.active_cancels.lock().unwrap();
        cancels.insert(id.clone(), cancel_token.clone());
    }

    let cancels_map = state.active_cancels.clone();
    let semaphore = state.download_semaphore.clone();

    tokio::spawn(async move {
        let app_handle = app.clone();
        let id_clone = id.clone();
        let token_clone = cancel_token.clone();
        let lang_clone = request.language.clone();

        // Emit initial status: Queued waiting for semaphore permit
        let _ = app_handle.emit("download-progress", serde_json::json!({
            "id": id_clone,
            "title": "Waiting for available slot...",
            "percent": 0.0,
            "speed": "",
            "eta": "Queued",
            "status": "queued",
            "language": lang_clone,
            "file_path": null
        }));

        // Check cancellation before acquiring permit
        if token_clone.load(Ordering::Relaxed) {
            let mut cancels = cancels_map.lock().unwrap();
            cancels.remove(&id);
            let _ = app.emit("download-error", serde_json::json!({
                "id": id,
                "error": "Download cancelled by user",
                "is_cancelled": true
            }));
            return;
        }

        // Acquire permit from semaphore asynchronously
        let _permit = match semaphore.acquire_owned().await {
            Ok(p) => p,
            Err(e) => {
                let mut cancels = cancels_map.lock().unwrap();
                cancels.remove(&id);
                let _ = app.emit("download-error", serde_json::json!({
                    "id": id,
                    "error": format!("Semaphore acquire error: {}", e),
                    "is_cancelled": false
                }));
                return;
            }
        };

        // Check cancellation after acquiring permit (before launching download_item)
        if token_clone.load(Ordering::Relaxed) {
            let mut cancels = cancels_map.lock().unwrap();
            cancels.remove(&id);
            let _ = app.emit("download-error", serde_json::json!({
                "id": id,
                "error": "Download cancelled by user",
                "is_cancelled": true
            }));
            return;
        }

        let dl_req = downloader::DownloadRequest {
            id: request.id.clone(),
            url: request.url,
            mode: request.mode,
            req_format: request.format,
            quality: request.quality,
            language: request.language,
            title: request.title,
            base_download_dir: download_dir,
            cookies_from_browser: request.cookies_from_browser,
            cookies_file_path: request.cookies_file_path,
            download_subtitles: request.download_subtitles,
            subtitle_languages: request.subtitle_languages,
            duplicate_action: request.duplicate_action,
        };

        let result = tokio::task::spawn_blocking(move || {
            download_item(
                dl_req,
                token_clone,
                |evt| {
                    let _ = app_handle.emit("download-progress", evt);
                },
            )
        }).await;

        {
            let mut cancels = cancels_map.lock().unwrap();
            cancels.remove(&id);
        }

        match result {
            Ok(Ok(file_path)) => {
                let _ = app.emit("download-complete", serde_json::json!({
                    "id": id,
                    "file_path": file_path,
                    "status": "completed"
                }));
            },
            Ok(Err(err)) => {
                let is_cancel = err.contains("cancelled");
                let _ = app.emit("download-error", serde_json::json!({
                    "id": id,
                    "error": err,
                    "is_cancelled": is_cancel
                }));
            },
            Err(e) => {
                let _ = app.emit("download-error", serde_json::json!({
                    "id": id,
                    "error": e.to_string(),
                    "is_cancelled": false
                }));
            }
        }
    });

    Ok("Task queued".into())
}

#[tauri::command]
async fn cancel_download(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let cancels = state.active_cancels.lock().unwrap();
    if let Some(token) = cancels.get(&id) {
        token.store(true, Ordering::Relaxed);
        return Ok(());
    }
    Err("Task not found or already completed".into())
}

#[tauri::command]
async fn list_library(
    state: State<'_, AppState>,
    search: Option<String>,
    sort_by: Option<String>,
    filter_lang: Option<String>,
    filter_type: Option<String>,
) -> Result<serde_json::Value, String> {
    let downloads_dir = {
        let guard = state.downloads_dir.lock().unwrap();
        guard.clone()
    };

    let result = tokio::task::spawn_blocking(move || {
        let mut music_map: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        let mut video_map: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

        let search_query = search.unwrap_or_default().to_lowercase();
        let sort_mode = sort_by.unwrap_or_else(|| "name".into());
        let lang_filter = filter_lang.unwrap_or_else(|| "All".into());
        let type_filter = filter_type.unwrap_or_else(|| "All".into());

        let music_dir = downloads_dir.join("Music");
        let video_dir = downloads_dir.join("Videos");

        let scan = |dir: &PathBuf, map: &mut HashMap<String, Vec<serde_json::Value>>| {
            if dir.exists() {
                for e in walkdir::WalkDir::new(dir).min_depth(1).max_depth(3).into_iter().flatten() {
                    if e.file_type().is_file() {
                        let fname = e.file_name().to_string_lossy().to_string();
                        if fname.starts_with('.') || fname.ends_with(".cover.jpg") || fname.contains(".tagged") {
                            continue;
                        }

                        if !search_query.is_empty() && !fname.to_lowercase().contains(&search_query) {
                            continue;
                        }

                        let parent_name = e.path().parent()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "General".to_string());
                        let lang_folder = if parent_name == "Music" || parent_name == "Videos" {
                            "General".to_string()
                        } else {
                            parent_name
                        };

                        if lang_filter != "All" && lang_folder.to_lowercase() != lang_filter.to_lowercase() {
                            continue;
                        }

                        let meta = e.metadata().ok();
                        let file_size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                        let modified = meta.as_ref().and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        let item_obj = serde_json::json!({
                            "name": fname,
                            "path": e.path().to_string_lossy().to_string(),
                            "size": file_size,
                            "modified": modified
                        });

                        map.entry(lang_folder).or_default().push(item_obj);
                    }
                }
            }
        };

        if type_filter == "All" || type_filter == "Music" {
            scan(&music_dir, &mut music_map);
        }
        if type_filter == "All" || type_filter == "Videos" {
            scan(&video_dir, &mut video_map);
        }

        // Sort items inside groups
        let sort_vec = |map: &mut HashMap<String, Vec<serde_json::Value>>| {
            for list in map.values_mut() {
                list.sort_by(|a, b| {
                    match sort_mode.as_str() {
                        "date" => b["modified"].as_u64().unwrap_or(0).cmp(&a["modified"].as_u64().unwrap_or(0)),
                        "size" => b["size"].as_u64().unwrap_or(0).cmp(&a["size"].as_u64().unwrap_or(0)),
                        _ => a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")),
                    }
                });
            }
        };

        sort_vec(&mut music_map);
        sort_vec(&mut video_map);

        serde_json::json!({
            "music": music_map,
            "videos": video_map
        })
    }).await.map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
async fn get_system_info() -> Result<serde_json::Value, String> {
    let ytdlp_bin = get_ytdlp_bin();
    let ytdlp_ver = if let Ok(out) = std::process::Command::new(&ytdlp_bin).arg("--version").output() {
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    } else {
        "Not installed".to_string()
    };

    let ffmpeg_ver = if let Some(bin) = find_ffmpeg() {
        if let Ok(out) = std::process::Command::new(&bin).arg("-version").output() {
            let first_line = String::from_utf8_lossy(&out.stdout);
            first_line.lines().next().unwrap_or("Installed").to_string()
        } else {
            "Installed".to_string()
        }
    } else {
        "Missing".to_string()
    };

    Ok(serde_json::json!({
        "app_version": "1.0.0",
        "ytdlp_version": ytdlp_ver,
        "ffmpeg_version": ffmpeg_ver
    }))
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<UserSettings, String> {
    let settings = state.settings.lock().unwrap();
    Ok(settings.clone())
}

#[tauri::command]
async fn save_settings(state: State<'_, AppState>, settings: UserSettings) -> Result<(), String> {
    save_settings_to_disk(&settings)?;
    {
        let mut s_guard = state.settings.lock().unwrap();
        *s_guard = settings.clone();
    }
    {
        let mut d_guard = state.downloads_dir.lock().unwrap();
        *d_guard = PathBuf::from(&settings.download_dir);
    }
    Ok(())
}

#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let target = if p.is_file() {
        p.parent().unwrap_or(&p).to_path_buf()
    } else {
        p
    };

    let _ = std::fs::create_dir_all(&target);

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&target).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&target).spawn();
    }
    Ok(())
}

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("File does not exist".into());
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&p).spawn();
    }
    Ok(())
}

#[tauri::command]
async fn update_ytdlp() -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ytdlp_bin = get_ytdlp_bin();
        let output = std::process::Command::new(&ytdlp_bin)
            .arg("-U")
            .output()
            .map_err(|e| format!("Failed to execute yt-dlp update at {}: {}", ytdlp_bin, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if output.status.success() {
            if stdout.is_empty() {
                Ok("yt-dlp is already up to date.".to_string())
            } else {
                Ok(stdout)
            }
        } else {
            let combined = if stdout.is_empty() { stderr } else { format!("{}\n{}", stdout, stderr) };
            if combined.contains("Permission denied") || combined.contains("permission") {
                Ok(format!("yt-dlp update notice:\n{}", combined))
            } else if !combined.trim().is_empty() {
                Ok(combined)
            } else {
                Err("yt-dlp update command failed.".to_string())
            }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_playlist_info(url: String) -> Result<Vec<downloader::PlaylistItem>, String> {
    tokio::task::spawn_blocking(move || downloader::get_playlist_info(&url))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn edit_file_tags(
    path: String,
    title: String,
    artist: String,
    category: String,
    cover_path: Option<String>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || tagger::update_media_file_tags(&path, &title, &artist, &category, cover_path.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn fetch_lyrics(artist: String, title: String, file_path: Option<String>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let p = file_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("temp.mp3"));
        tagger::fetch_and_embed_lyrics(&artist, &title, &p)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn main() {
    let loaded_settings = load_settings_from_disk();
    let initial_dir = PathBuf::from(&loaded_settings.download_dir);
    std::fs::create_dir_all(&initial_dir).unwrap_or_default();

    let initial_semaphore = Arc::new(Semaphore::new(loaded_settings.max_concurrent_downloads.max(1)));

    tauri::Builder::default()
        .manage(AppState {
            downloads_dir: Arc::new(Mutex::new(initial_dir)),
            active_cancels: Arc::new(Mutex::new(HashMap::new())),
            settings: Arc::new(Mutex::new(loaded_settings)),
            download_semaphore: initial_semaphore,
        })
        .invoke_handler(tauri::generate_handler![
            get_metadata,
            start_download,
            cancel_download,
            list_library,
            get_system_info,
            get_settings,
            save_settings,
            open_folder,
            open_file,
            update_ytdlp,
            get_playlist_info,
            edit_file_tags,
            fetch_lyrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running OSMIIC tauri application");
}
