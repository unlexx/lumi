use crate::parser::{parse_filename, ParsedVideo};
use serde::Serialize;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Clone)]
pub struct VideoFile {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub parsed: ParsedVideo,
}

#[tauri::command]
pub fn scan_videos(folder_path: String) -> Result<Vec<VideoFile>, String> {
    let video_extensions = ["mkv", "mp4", "avi", "mov", "wmv", "flv", "webm"];

    let path = Path::new(&folder_path);
    if !path.exists() {
        return Err(format!("Папка не существует: {}", folder_path));
    }
    if !path.is_dir() {
        return Err(format!("Путь не является папкой: {}", folder_path));
    }

    let mut videos = Vec::new();

    for entry in WalkDir::new(&folder_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_path = entry.path();

        if file_path.is_file() {
            if let Some(ext) = file_path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if video_extensions.contains(&ext_str.as_str()) {
                    let name = file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    let parsed = parse_filename(&name);

                    videos.push(VideoFile {
                        path: file_path.to_string_lossy().to_string(),
                        name,
                        extension: ext_str,
                        parsed,
                    });
                }
            }
        }
    }

    Ok(videos)
}