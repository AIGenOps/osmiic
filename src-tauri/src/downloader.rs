use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::classifier::detect_language_advanced;
use crate::tagger::{embed_cover_and_metadata, find_ffmpeg};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoMetadata {
    pub url: String,
    pub title: String,
    pub artist: String,
    pub thumbnail: String,
    pub duration: u64,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistItem {
    pub id: String,
    pub title: String,
    pub url: String,
    pub duration: u64,
    pub thumbnail: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProgressEvent {
    pub id: String,
    pub title: String,
    pub percent: f32,
    pub speed: String,
    pub eta: String,
    pub status: String,
    pub language: String,
    pub file_path: Option<String>,
}

pub fn get_ytdlp_bin() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Some(home) = dirs::home_dir() {
            let win_user_bin = home.join("yt-dlp.exe");
            if win_user_bin.exists() {
                return win_user_bin.to_string_lossy().to_string();
            }
        }
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p1 = PathBuf::from(&local_app_data).join("yt-dlp\\yt-dlp.exe");
            if p1.exists() {
                return p1.to_string_lossy().to_string();
            }
            let p2 = PathBuf::from(&local_app_data).join("Microsoft\\WinGet\\Links\\yt-dlp.exe");
            if p2.exists() {
                return p2.to_string_lossy().to_string();
            }
        }
        if PathBuf::from("C:\\yt-dlp\\yt-dlp.exe").exists() {
            return "C:\\yt-dlp\\yt-dlp.exe".to_string();
        }
        if let Ok(output) = Command::new("where").arg("yt-dlp").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").trim().to_string();
                if !path.is_empty() { return path; }
            }
        }
        "yt-dlp.exe".to_string()
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = dirs::home_dir() {
            let user_bin = home.join(".local/bin/yt-dlp");
            if user_bin.exists() {
                return user_bin.to_string_lossy().to_string();
            }
        }
        if PathBuf::from("/usr/local/bin/yt-dlp").exists() {
            return "/usr/local/bin/yt-dlp".to_string();
        }
        if PathBuf::from("/usr/bin/yt-dlp").exists() {
            return "/usr/bin/yt-dlp".to_string();
        }
        if let Ok(output) = Command::new("which").arg("yt-dlp").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() { return path; }
            }
        }
        "yt-dlp".to_string()
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];
    let mut cleaned: String = name
        .chars()
        .map(|c| if invalid_chars.contains(&c) || c.is_control() { '_' } else { c })
        .collect();

    // Remove leading/trailing spaces or dots (illegal on Windows)
    cleaned = cleaned.trim().trim_matches('.').to_string();

    if cleaned.is_empty() || cleaned.chars().all(|c| c == '_') {
        "media_file".to_string()
    } else {
        cleaned
    }
}

pub fn fetch_metadata(url: &str, user_lang: &str) -> Result<VideoMetadata, String> {
    let ytdlp_bin = get_ytdlp_bin();
    let output = Command::new(&ytdlp_bin)
        .args([
            "--dump-single-json",
            "--no-warnings",
            "--no-playlist",
            "--no-colors",
            url
        ])
        .output()
        .map_err(|e| format!("Failed to execute yt-dlp at {}: {}", ytdlp_bin, e))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr);
        let clean_err = err_str.lines()
            .find(|l| l.contains("ERROR:"))
            .unwrap_or_else(|| if err_str.trim().is_empty() { "Invalid URL or video unavailable" } else { err_str.trim() });
        return Err(clean_err.to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let title = v["title"].as_str().unwrap_or("Unknown Title").to_string();
    let artist = v["uploader"].as_str()
        .or_else(|| v["channel"].as_str())
        .unwrap_or("Unknown Artist").to_string();
    let thumbnail = v["thumbnail"].as_str().unwrap_or("").to_string();
    let duration = v["duration"].as_u64().unwrap_or(0);
    let tags = v["tags"].as_array()
        .map(|arr| arr.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>().join(" "))
        .unwrap_or_default();

    let yt_lang = v["language"].as_str();
    let detected_lang = detect_language_advanced(&title, &artist, &tags, yt_lang, user_lang);

    Ok(VideoMetadata {
        url: url.to_string(),
        title,
        artist,
        thumbnail,
        duration,
        language: detected_lang,
    })
}

pub fn get_playlist_info(url: &str) -> Result<Vec<PlaylistItem>, String> {
    let ytdlp_bin = get_ytdlp_bin();
    let output = Command::new(&ytdlp_bin)
        .arg("--flat-playlist")
        .arg("-J")
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp playlist parsing error: {}", stderr));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse playlist JSON: {}", e))?;

    let mut items = Vec::new();
    if let Some(entries) = val.get("entries").and_then(|e| e.as_array()) {
        for entry in entries {
            let title = entry.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown Title").to_string();
            let url_entry = entry.get("url").and_then(|u| u.as_str())
                .or_else(|| entry.get("webpage_url").and_then(|u| u.as_str()))
                .unwrap_or(url);
            let item_url = if url_entry.starts_with("http") {
                url_entry.to_string()
            } else {
                format!("https://www.youtube.com/watch?v={}", url_entry)
            };
            let duration = entry.get("duration").and_then(|d| d.as_u64()).unwrap_or(0);
            let id = entry.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
            let thumbnail = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id);

            items.push(PlaylistItem {
                id,
                title,
                url: item_url,
                duration,
                thumbnail,
            });
        }
    } else {
        let title = val.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown Title").to_string();
        let duration = val.get("duration").and_then(|d| d.as_u64()).unwrap_or(0);
        let id = val.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
        let thumbnail = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id);
        items.push(PlaylistItem {
            id,
            title,
            url: url.to_string(),
            duration,
            thumbnail,
        });
    }

    Ok(items)
}

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub id: String,
    pub url: String,
    pub mode: String,
    pub req_format: String,
    pub quality: String,
    pub language: String,
    pub title: Option<String>,
    pub base_download_dir: PathBuf,
    pub cookies_from_browser: Option<String>,
    pub cookies_file_path: Option<String>,
    pub download_subtitles: Option<bool>,
    pub subtitle_languages: Option<String>,
    pub duplicate_action: Option<String>,
}

pub fn download_item<F>(
    req: DownloadRequest,
    cancel_token: Arc<AtomicBool>,
    progress_cb: F,
) -> Result<String, String>
where
    F: Fn(ProgressEvent),
{
    // Pre-flight check for FFmpeg
    let _ = &req.quality;
    if find_ffmpeg().is_none() {
        #[cfg(target_os = "windows")]
        let err_msg = "FFmpeg dependency missing. Please install FFmpeg (e.g. via 'winget install ffmpeg' or from https://www.gyan.dev/ffmpeg/builds/) and ensure ffmpeg.exe is in your system PATH.";
        #[cfg(not(target_os = "windows"))]
        let err_msg = "FFmpeg dependency missing. Please install ffmpeg using your system package manager (e.g. sudo apt install ffmpeg or sudo dnf install ffmpeg).";

        return Err(err_msg.into());
    }

    // Guard format against mode mismatches
    let (is_audio, real_format) = if req.mode == "audio" {
        let fmt = if req.req_format == "mp4" || req.req_format == "mkv" { "mp3" } else { &req.req_format };
        (true, fmt)
    } else {
        let fmt = if req.req_format == "mp3" || req.req_format == "flac" || req.req_format == "m4a" { "mp4" } else { &req.req_format };
        (false, fmt)
    };

    let media_type_folder = if is_audio { "Music" } else { "Videos" };
    let category_dir = req.base_download_dir.join(media_type_folder).join(&req.language);
    std::fs::create_dir_all(&category_dir).map_err(|e| format!("Failed to create folder: {}", e))?;

    let dup_action = req.duplicate_action.as_deref().unwrap_or("skip");

    // Pre-flight check for existing duplicate file on disk if dup_action is "skip"
    if dup_action == "skip" {
        if let Some(ref t) = req.title {
            let trimmed_t = t.trim();
            if !trimmed_t.is_empty() && !trimmed_t.starts_with("http://") && !trimmed_t.starts_with("https://") {
                let sanitized = sanitize_filename(trimmed_t);
                let expected_file = category_dir.join(format!("{}.{}", sanitized, real_format));
                if expected_file.exists() {
                    if let Ok(meta) = std::fs::metadata(&expected_file) {
                        if meta.len() > 0 {
                            let path_str = expected_file.to_string_lossy().to_string();
                            progress_cb(ProgressEvent {
                                id: req.id.clone(),
                                title: "File already exists on disk (Skipped)".into(),
                                percent: 100.0,
                                speed: "Skipped".into(),
                                eta: "Done".into(),
                                status: "completed".into(),
                                language: req.language.clone(),
                                file_path: Some(path_str.clone()),
                            });
                            return Ok(path_str);
                        }
                    }
                }
            }
        }
    }

    let output_template = if dup_action == "rename" {
        category_dir.join("%(title)s (%(id)s).%(ext)s")
    } else {
        category_dir.join("%(title)s.%(ext)s")
    };
    let out_str = output_template.to_string_lossy().to_string();

    let ytdlp_bin = get_ytdlp_bin();
    let mut cmd = Command::new(&ytdlp_bin);
    cmd.env("PYTHONUNBUFFERED", "1");
    cmd.arg("--no-colors");
    cmd.arg("--newline");
    cmd.arg("--progress");
    cmd.arg("--no-playlist");

    if dup_action == "overwrite" {
        cmd.arg("--force-overwrites");
    } else {
        cmd.arg("--no-overwrites");
    }

    if let Some(browser) = &req.cookies_from_browser {
        if !browser.is_empty() && browser != "none" {
            cmd.args(["--cookies-from-browser", browser]);
        }
    }
    if let Some(cookie_path) = &req.cookies_file_path {
        if !cookie_path.is_empty() && PathBuf::from(cookie_path).exists() {
            cmd.args(["--cookies", cookie_path]);
        }
    }

    if req.download_subtitles.unwrap_or(false) {
        let sub_langs = req.subtitle_languages.as_deref().unwrap_or("en");
        cmd.args(["--write-subs", "--write-auto-subs", "--sub-lang", sub_langs, "--embed-subs"]);
    }

    cmd.arg("--progress-template").arg("%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress.status)s");
    cmd.args(["-o", &out_str]);

    if is_audio {
        cmd.args(["-x", "--audio-format", real_format, "--audio-quality", "0"]);
    } else {
        cmd.args(["-f", "bestvideo+bestaudio/best", "--merge-output-format", real_format]);
    }

    cmd.arg(&req.url);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("yt-dlp launch error: {}", e))?;

    // Initial progress notification
    progress_cb(ProgressEvent {
        id: req.id.clone(),
        title: "Starting download...".into(),
        percent: 1.0,
        speed: "Connecting...".into(),
        eta: "Starting...".into(),
        status: "downloading".into(),
        language: req.language.clone(),
        file_path: None,
    });

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    // Drain stderr in background thread
    let stderr_handle = std::thread::spawn(move || {
        let mut err_buf = String::new();
        let reader = BufReader::new(stderr);
        for l in reader.lines().map_while(Result::ok) {
            err_buf.push_str(&l);
            err_buf.push('\n');
        }
        err_buf
    });

    let reader = BufReader::new(stdout);
    let mut final_saved_path: Option<String> = None;

    for l in reader.lines().map_while(Result::ok) {
        // Check for Cancellation Signal
        if cancel_token.load(Ordering::Relaxed) {
            let _ = child.kill();
            return Err("Download cancelled by user".into());
        }

        let l_trimmed = l.trim();

        if l_trimmed.starts_with('/') && (l_trimmed.ends_with(&format!(".{}", real_format)) || l_trimmed.contains(media_type_folder)) {
            final_saved_path = Some(l_trimmed.to_string());
        }

        let parts: Vec<&str> = l_trimmed.split('|').collect();
        if parts.len() == 4 && (l_trimmed.contains('%') || parts[3] == "downloading" || parts[3] == "finished") {
            let percent_clean = parts[0].trim().trim_end_matches('%');
            let percent = percent_clean.parse::<f32>().unwrap_or(1.0).clamp(0.1, 100.0);
            let speed = if parts[1].trim().is_empty() { "N/A".to_string() } else { parts[1].trim().to_string() };
            let eta = if parts[2].trim().is_empty() { "N/A".to_string() } else { parts[2].trim().to_string() };
            let status = parts[3].trim().to_string();

            progress_cb(ProgressEvent {
                id: req.id.clone(),
                title: "Downloading...".into(),
                percent,
                speed,
                eta,
                status: if status == "finished" { "processing".into() } else { "downloading".into() },
                language: req.language.clone(),
                file_path: None,
            });
        } else if l.contains("[download]") && l.contains('%') {
            let parts: Vec<&str> = l.split_whitespace().collect();
            let mut percent = 1.0;
            let mut speed = "N/A".to_string();
            let mut eta = "N/A".to_string();

            for (idx, part) in parts.iter().enumerate() {
                if part.contains('%') {
                    let clean: String = part.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
                    if let Ok(val) = clean.parse::<f32>() {
                        percent = val.clamp(0.1, 100.0);
                    }
                }
                if part.contains("MiB/s") || part.contains("KiB/s") || part.contains("B/s") {
                    speed = part.chars().filter(|c| !c.is_control()).collect();
                }
                if *part == "ETA" && idx + 1 < parts.len() {
                    eta = parts[idx + 1].chars().filter(|c| !c.is_control()).collect();
                }
            }

            progress_cb(ProgressEvent {
                id: req.id.clone(),
                title: "Downloading...".into(),
                percent,
                speed,
                eta,
                status: "downloading".into(),
                language: req.language.clone(),
                file_path: None,
            });
        } else if l.contains("[Merger]") || l.contains("[ExtractAudio]") || l.contains("[ffmpeg]") {
            progress_cb(ProgressEvent {
                id: req.id.clone(),
                title: "Processing FFmpeg remux & metadata...".into(),
                percent: 95.0,
                speed: "Processing".into(),
                eta: "Finishing".into(),
                status: "processing".into(),
                language: req.language.clone(),
                file_path: None,
            });
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    let stderr_output = stderr_handle.join().unwrap_or_default();

    if cancel_token.load(Ordering::Relaxed) {
        return Err("Download cancelled by user".into());
    }

    if !status.success() {
        let clean_err = stderr_output.lines()
            .find(|l| l.contains("ERROR:"))
            .unwrap_or_else(|| if stderr_output.trim().is_empty() { "yt-dlp download failed. Check URL or network." } else { stderr_output.trim() });
        return Err(clean_err.to_string());
    }

    // Try embedding thumbnail art & metadata
    if let Ok(meta) = fetch_metadata(&req.url, &req.language) {
        let safe_title = sanitize_filename(&meta.title);
        let target_path = final_saved_path.clone().unwrap_or_else(|| {
            category_dir.join(format!("{}.{}", safe_title, real_format)).to_string_lossy().to_string()
        });

        if std::path::Path::new(&target_path).exists() {
            let _ = embed_cover_and_metadata(
                &target_path,
                &meta.title,
                &meta.artist,
                &req.language,
                &meta.thumbnail
            );
            return Ok(target_path);
        }
    }

    Ok(final_saved_path.unwrap_or_else(|| "Completed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Normal Title"), "Normal Title");
        assert_eq!(sanitize_filename("Review: 10/10? *Amazing* <Video> | \"Test\""), "Review_ 10_10_ _Amazing_ _Video_ _ _Test_");
        assert_eq!(sanitize_filename("   leading and trailing spaces.   "), "leading and trailing spaces");
        assert_eq!(sanitize_filename("///:::***???"), "media_file");
        assert_eq!(sanitize_filename("Title with Accents and CJK 日本語.mp3"), "Title with Accents and CJK 日本語.mp3");
    }

    #[test]
    fn test_disk_duplicate_check() {
        let temp_dir = std::env::temp_dir().join("osmiic_test_dup");
        let music_dir = temp_dir.join("Music").join("General");
        let _ = std::fs::create_dir_all(&music_dir);
        let test_file = music_dir.join("Existing Track.mp3");
        std::fs::write(&test_file, b"dummy audio content").unwrap();

        let cancel = Arc::new(AtomicBool::new(false));
        let progress_called = Arc::new(AtomicBool::new(false));
        let progress_clone = progress_called.clone();
        let req = DownloadRequest {
            id: "id_1".into(),
            url: "https://fakeurl.com".into(),
            mode: "audio".into(),
            req_format: "mp3".into(),
            quality: "best".into(),
            language: "General".into(),
            title: Some("Existing Track".into()),
            base_download_dir: temp_dir.clone(),
            cookies_from_browser: None,
            cookies_file_path: None,
            download_subtitles: Some(false),
            subtitle_languages: Some("en".into()),
            duplicate_action: Some("skip".into()),
        };

        let res = download_item(
            req,
            cancel,
            move |_| progress_clone.store(true, Ordering::Relaxed),
        );

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), test_file.to_string_lossy().to_string());
        assert!(progress_called.load(Ordering::Relaxed));

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
