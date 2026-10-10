use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Read;

pub fn find_ffmpeg() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        if PathBuf::from("C:\\ffmpeg\\bin\\ffmpeg.exe").exists() {
            return Some("C:\\ffmpeg\\bin\\ffmpeg.exe".into());
        }
        if PathBuf::from("C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe").exists() {
            return Some("C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe".into());
        }
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let winget_p = PathBuf::from(&local_app_data).join("Microsoft\\WinGet\\Links\\ffmpeg.exe");
            if winget_p.exists() {
                return Some(winget_p.to_string_lossy().to_string());
            }
        }
        if let Ok(output) = Command::new("where").arg("ffmpeg").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").trim().to_string();
                if !path.is_empty() { return Some(path); }
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        if PathBuf::from("/usr/bin/ffmpeg").exists() {
            return Some("/usr/bin/ffmpeg".into());
        }
        if PathBuf::from("/usr/local/bin/ffmpeg").exists() {
            return Some("/usr/local/bin/ffmpeg".into());
        }
        if let Ok(output) = Command::new("which").arg("ffmpeg").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() { return Some(path); }
            }
        }
        None
    }
}

pub fn download_best_cover(cover_url: &str, output_path: &str) -> bool {
    if cover_url.is_empty() { return false; }

    let mut urls_to_try = vec![cover_url.to_string()];

    if cover_url.contains("ytimg.com") || cover_url.contains("ggpht.com") {
        let mut base_id = "";
        if let Some(pos) = cover_url.find("/vi/") {
            base_id = cover_url[pos + 4..].split('/').next().unwrap_or("");
        } else if let Some(pos) = cover_url.find("/vi_webp/") {
            base_id = cover_url[pos + 9..].split('/').next().unwrap_or("");
        }

        if !base_id.is_empty() {
            urls_to_try = vec![
                format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", base_id),
                format!("https://img.youtube.com/vi/{}/sddefault.jpg", base_id),
                format!("https://img.youtube.com/vi/{}/hqdefault.jpg", base_id),
                cover_url.to_string(),
            ];
        }
    }

    for url in urls_to_try {
        let res = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(8))
            .call();

        if let Ok(response) = res {
            if response.status() == 200 {
                let mut bytes = Vec::new();
                if response.into_reader().read_to_end(&mut bytes).is_ok() {
                    // Check non-empty valid image (> 2000 bytes)
                    if bytes.len() > 2000
                        && fs::write(output_path, bytes).is_ok() {
                            return true;
                        }
                }
            }
        }
    }

    false
}

pub fn embed_cover_and_metadata(
    file_path: &str,
    title: &str,
    artist: &str,
    language: &str,
    cover_url: &str,
) -> Result<(), String> {
    if !Path::new(file_path).exists() {
        return Err(format!("Target file does not exist: {}", file_path));
    }

    let ffmpeg_bin = match find_ffmpeg() {
        Some(bin) => bin,
        None => return Err("FFmpeg binary not found on system. Please install ffmpeg.".into())
    };

    let temp_cover = format!("{}.cover.jpg", file_path);
    let has_cover = download_best_cover(cover_url, &temp_cover);

    let is_mp3 = file_path.ends_with(".mp3");
    let is_flac = file_path.ends_with(".flac");
    let is_m4a = file_path.ends_with(".m4a");
    let is_audio = is_mp3 || is_flac || is_m4a;

    let ext = if is_mp3 {
        ".mp3"
    } else if is_flac {
        ".flac"
    } else if is_m4a {
        ".m4a"
    } else if file_path.ends_with(".mkv") {
        ".mkv"
    } else {
        ".mp4"
    };

    let temp_output = format!("{}.tagged{}", file_path, ext);
    let mut ffmpeg_cmd = Command::new(&ffmpeg_bin);
    ffmpeg_cmd.args(["-y", "-i", file_path]);

    if has_cover {
        ffmpeg_cmd.args(["-i", &temp_cover]);
    }

    if is_audio {
        // Audio mapping & ID3 tagging
        ffmpeg_cmd.args([
            "-map", "0:a",
            "-map", if has_cover { "1:0" } else { "0:v?" },
            "-c", "copy",
            "-metadata", &format!("title={}", title),
            "-metadata", &format!("artist={}", artist),
            "-metadata", &format!("genre={}", language),
            "-disposition:v:0", "attached_pic"
        ]);

        if is_mp3 {
            ffmpeg_cmd.args(["-id3v2_version", "3"]);
        }
    } else {
        // Video mapping: preserve video + audio streams cleanly
        ffmpeg_cmd.args([
            "-map", "0",
            "-c", "copy",
            "-metadata", &format!("title={}", title),
            "-metadata", &format!("artist={}", artist),
            "-metadata", &format!("genre={}", language)
        ]);

        if has_cover {
            ffmpeg_cmd.args([
                "-map", "1",
                "-disposition:v:1", "attached_pic"
            ]);
        }
    }

    ffmpeg_cmd.arg(&temp_output);

    let output = ffmpeg_cmd.output().map_err(|e| format!("FFmpeg execution failed: {}", e))?;
    if output.status.success() && Path::new(&temp_output).exists() {
        let _ = fs::rename(&temp_output, file_path);
    }

    if has_cover && Path::new(&temp_cover).exists() {
        let _ = fs::remove_file(&temp_cover);
    }

    Ok(())
}

fn urlencode(s: &str) -> String {
    s.chars().map(|c| match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        ' ' => "+".to_string(),
        _ => format!("%{:02X}", c as u8),
    }).collect()
}

pub fn fetch_and_embed_lyrics(artist: &str, title: &str, file_path: &Path) -> Result<String, String> {
    if artist.trim().is_empty() || title.trim().is_empty() {
        return Err("Artist and title are required for lyrics lookup".into());
    }

    let url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}",
        urlencode(artist.trim()),
        urlencode(title.trim())
    );

    let res = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(6))
        .call();

    let lyrics_text = match res {
        Ok(resp) if resp.status() == 200 => {
            if let Ok(json) = resp.into_json::<serde_json::Value>() {
                if let Some(synced) = json.get("syncedLyrics").and_then(|s| s.as_str()) {
                    if !synced.trim().is_empty() {
                        synced.to_string()
                    } else if let Some(plain) = json.get("plainLyrics").and_then(|p| p.as_str()) {
                        plain.to_string()
                    } else {
                        return Err("No lyrics content found".into());
                    }
                } else if let Some(plain) = json.get("plainLyrics").and_then(|p| p.as_str()) {
                    plain.to_string()
                } else {
                    return Err("No lyrics content found".into());
                }
            } else {
                return Err("Failed to parse lyrics JSON response".into());
            }
        },
        _ => return Err("Lyrics not found on LRCLIB".into())
    };

    if file_path.exists() {
        let lrc_path = file_path.with_extension("lrc");
        let _ = fs::write(&lrc_path, &lyrics_text);

        if let Some(ffmpeg_bin) = find_ffmpeg() {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");
            let temp_output = file_path.with_extension(format!("lyricstemp.{}", ext));
            let mut cmd = Command::new(&ffmpeg_bin);
            cmd.args([
                "-y",
                "-i", &file_path.to_string_lossy(),
                "-c", "copy",
                "-metadata", &format!("lyrics={}", lyrics_text),
                &temp_output.to_string_lossy()
            ]);
            if let Ok(out) = cmd.output() {
                if out.status.success() && temp_output.exists() {
                    let _ = fs::rename(&temp_output, file_path);
                }
            }
        }
    }

    Ok(lyrics_text)
}

pub fn update_media_file_tags(
    old_path_str: &str,
    new_title: &str,
    new_artist: &str,
    new_category: &str,
    new_cover_path: Option<&str>,
) -> Result<String, String> {
    let old_path = PathBuf::from(old_path_str);
    if !old_path.exists() {
        return Err("Original file does not exist".into());
    }

    let parent_dir = old_path.parent().ok_or("Invalid file path")?;
    let ext = old_path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");

    if let Some(ffmpeg_bin) = find_ffmpeg() {
        let temp_output = parent_dir.join(format!(".temp_tag_{}.{}", crate::downloader::sanitize_filename(new_title), ext));
        let mut cmd = Command::new(&ffmpeg_bin);
        cmd.args(["-y", "-i", old_path_str]);

        let has_cover = if let Some(cover) = new_cover_path {
            if Path::new(cover).exists() {
                cmd.args(["-i", cover]);
                true
            } else { false }
        } else { false };

        cmd.args([
            "-map", "0",
            "-c", "copy",
            "-metadata", &format!("title={}", new_title),
            "-metadata", &format!("artist={}", new_artist),
            "-metadata", &format!("genre={}", new_category),
        ]);

        if has_cover {
            cmd.args(["-map", "1", "-disposition:v:1", "attached_pic"]);
        }

        cmd.arg(&temp_output);

        if let Ok(output) = cmd.output() {
            if output.status.success() && temp_output.exists() {
                let _ = fs::rename(&temp_output, &old_path);
            }
        }
    }

    let category_parent = parent_dir.parent().unwrap_or(parent_dir);
    let target_category_dir = category_parent.join(new_category);
    let _ = fs::create_dir_all(&target_category_dir);

    let sanitized_title = crate::downloader::sanitize_filename(new_title);
    let final_file_name = format!("{}.{}", sanitized_title, ext);
    let final_target_path = target_category_dir.join(&final_file_name);

    if old_path != final_target_path {
        fs::rename(&old_path, &final_target_path)
            .map_err(|e| format!("Failed to move file to category folder: {}", e))?;
        Ok(final_target_path.to_string_lossy().to_string())
    } else {
        Ok(old_path.to_string_lossy().to_string())
    }
}
