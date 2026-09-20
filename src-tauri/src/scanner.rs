use crate::config::{load_folders, MediaType};
use crate::parser::{parse_filename, ParsedVideo};
use serde::Serialize;
use std::path::Path;
use walkdir::WalkDir;
use tauri::Emitter;

#[derive(Serialize, Clone)]
pub struct VideoFile {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub parsed: ParsedVideo,
    pub tmdb: Option<TmdbInfo>,
    pub media_type: MediaType,
    pub watched: bool,
}

#[derive(Serialize, Clone)]
pub struct TmdbInfo {
    pub id: u32,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub rating: Option<f64>,
}

// В Rust (scanner.rs или новый tv.rs)

#[derive(Serialize, Clone)]
pub struct TvShow {
    pub title: String,              // "House of the Dragon"
    pub year: Option<u32>,
    pub seasons: Vec<Season>,
    pub tmdb: Option<TmdbInfo>,     // для LUMI-9b
}

#[derive(Serialize, Clone)]
pub struct Season {
    pub number: u32,                // 3
    pub episodes: Vec<Episode>,
}

#[derive(Serialize, Clone)]
pub struct Episode {
    pub number: u32,                // 1
    pub path: String,
    pub name: String,               // полное имя файла
    pub parsed: ParsedVideo,        // из существующего парсера
}

#[derive(Serialize)]
pub struct Library {
    pub movies: Vec<VideoFile>,
    pub tv_shows: Vec<TvShow>,
}

#[tauri::command]
pub async fn scan_all(app: tauri::AppHandle) -> Result<Library, String> {
    println!("=== scan_all started ===");

    let folders = load_folders();
    if folders.is_empty() {
        return Ok(Library {
            movies: Vec::new(),
            tv_shows: Vec::new(),
        });
    }

    let total_folders = folders.len();
    let api_key = std::env::var("TMDB_API_KEY")
        .map_err(|_| "TMDB_API_KEY not set in .env")?;
    let client = crate::tmdb::build_client().map_err(|e| e.to_string())?;

    let video_extensions = ["mkv", "mp4", "avi", "mov", "wmv", "flv", "webm"];
    let mut videos = Vec::new();

    for (idx, folder) in folders.iter().enumerate() {
        let path = Path::new(&folder.path);
        if !path.exists() || !path.is_dir() {
            eprintln!("Skipping invalid folder: {}", folder.path);
            continue;
        }

        // Эмитим прогресс перед обработкой папки
        app.emit(
            "scan_progress",
            serde_json::json!({
                "current": idx + 1,
                "total": total_folders,
                "folder": folder.path,
            }),
        )
        .ok();

        println!("Scanning: {} ({:?})", folder.path, folder.media_type);
let conn = crate::cache::init_db().ok();
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
    watched: conn
        .as_ref()
        .map(|c| crate::cache::is_watched(c, &file_path.to_string_lossy()))
        .unwrap_or(false),
});
            }
        }
    }

    // TMDB lookup
    println!("=== TMDB lookup for {} videos ===", videos.len());
    for video in &mut videos {
        if video.media_type != MediaType::Movie {
            continue;
        }
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

    let (movies, tv_files): (Vec<VideoFile>, Vec<VideoFile>) = videos
        .into_iter()
        .partition(|v| v.media_type == MediaType::Movie);

    let mut tv_shows = group_into_tv_shows(tv_files);

// TMDB lookup для сериалов — один запрос на сериал
println!("=== TMDB lookup for {} TV shows ===", tv_shows.len());
for show in &mut tv_shows {
    match crate::tmdb::get_or_fetch_tv(
        &client,
        &api_key,
        &show.title,
        show.year,
    )
    .await
    {
        Ok(Some(tv)) => {
            let poster_url = tv
                .poster_path
                .map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));

            // Красивое название из TMDB (вместо lowercase из парсера)
            show.title = tv.name.clone();

            show.tmdb = Some(TmdbInfo {
                id: tv.id,
                title: tv.name,           // TmdbInfo использует поле "title"
                overview: tv.overview,
                poster_url,
                rating: tv.vote_average,
            });
        }
        Ok(None) => {
            println!("  → No results for '{}'", show.title);
        }
        Err(e) => {
            eprintln!("TMDB error for '{}': {}", show.title, e);
        }
    }
}

Ok(Library { movies, tv_shows })
}

use std::collections::HashMap;
use crate::tv_parser::parse_tv_filename;

pub fn group_into_tv_shows(videos: Vec<VideoFile>) -> Vec<TvShow> {
    // Ключ: (title_lowercase, year)
    let mut groups: HashMap<(String, Option<u32>), Vec<VideoFile>> = HashMap::new();

    for video in videos.into_iter().filter(|v| v.media_type == MediaType::TvShows) {
        let parsed_tv = parse_tv_filename(&video.name);
        
        // Если не распознали сезон/эпизод — пропускаем (это не серия)
        if parsed_tv.season.is_none() || parsed_tv.episode.is_none() {
            continue;
        }

        let key = (parsed_tv.title.to_lowercase(), parsed_tv.year);
        groups.entry(key).or_default().push(video);
    }

    let mut shows = Vec::new();

    for ((title_lower, year), files) in groups {
        let mut seasons_map: HashMap<u32, Vec<Episode>> = HashMap::new();

        for file in files {
            let parsed_tv = parse_tv_filename(&file.name);
            let season_num = parsed_tv.season.unwrap();
            let episode_num = parsed_tv.episode.unwrap();

            seasons_map.entry(season_num).or_default().push(Episode {
                number: episode_num,
                path: file.path.clone(),
                name: file.name.clone(),
                parsed: file.parsed.clone(),
            });
        }

        let mut seasons: Vec<Season> = seasons_map
            .into_iter()
            .map(|(number, mut episodes)| {
                episodes.sort_by_key(|e| e.number);
                Season { number, episodes }
            })
            .collect();
        seasons.sort_by_key(|s| s.number);

        shows.push(TvShow {
            title: title_lower,   // потом заменим на красивое из TMDB
            year,
            seasons,
            tmdb: None,
        });
    }

    shows.sort_by(|a, b| a.title.cmp(&b.title));
    shows
}