use crate::config::{load_folders, MediaType};
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
    pub tmdb: Option<TmdbInfo>,
    pub media_type: MediaType,
}

#[derive(Serialize, Clone)]
pub struct TmdbInfo {
    pub id: u32,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub rating: Option<f64>,
}

#[tauri::command]
pub async fn scan_all() -> Result<Vec<VideoFile>, String> {
    println!("=== scan_all started ===");

    let folders = load_folders();
    if folders.is_empty() {
        return Ok(Vec::new());
    }

    let api_key = std::env::var("TMDB_API_KEY")
        .map_err(|_| "TMDB_API_KEY not set in .env")?;
    let client = crate::tmdb::build_client().map_err(|e| e.to_string())?;

    let video_extensions = ["mkv", "mp4", "avi", "mov", "wmv", "flv", "webm"];
    let mut videos = Vec::new();

    for folder in &folders {
        let path = Path::new(&folder.path);
        if !path.exists() || !path.is_dir() {
            eprintln!("Skipping invalid folder: {}", folder.path);
            continue;
        }

        println!("Scanning: {} ({:?})", folder.path, folder.media_type);

        for entry in WalkDir::new(&folder.path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }

            if let Some(ext) = file_path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !video_extensions.contains(&ext_str.as_str()) {
                    continue;
                }

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
                    tmdb: None,
                    media_type: folder.media_type.clone(),
                });
            }
        }
    }

    // TMDB lookup
    println!("=== TMDB lookup for {} videos ===", videos.len());
    for video in &mut videos {
        match crate::tmdb::get_or_fetch(
            &client,
            &api_key,
            &video.parsed.title,
            video.parsed.year,
        )
        .await
        {
            Ok(Some(movie)) => {
                let poster_url = movie
                    .poster_path
                    .map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));

                video.tmdb = Some(TmdbInfo {
                    id: movie.id,
                    title: movie.title,
                    overview: movie.overview,
                    poster_url,
                    rating: movie.vote_average,
                });
            }
            Ok(None) => {
                println!("  → No results for '{}'", video.parsed.title);
            }
            Err(e) => {
                eprintln!("TMDB error for '{}': {}", video.parsed.title, e);
            }
        }
    }

    Ok(videos)
}