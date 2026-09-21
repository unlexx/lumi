use crate::config::{load_folders, MediaType};
use crate::parser::{parse_filename, ParsedVideo};
use serde::Serialize;
use std::path::Path;
use tauri::Emitter;
use walkdir::WalkDir;
use rusqlite::Connection;

#[derive(Serialize, Clone)]
pub struct VideoFile {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub parsed: ParsedVideo,
    pub tmdb: Option<TmdbInfo>,
    pub media_type: MediaType,
    pub watched: bool,
    pub position: Option<f64>,
    pub duration: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct TmdbInfo {
    pub id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub rating: Option<f64>,
}

// В Rust (scanner.rs или новый tv.rs)

#[derive(Serialize, Clone)]
pub struct TvShow {
    pub title: String, // "House of the Dragon"
    pub year: Option<u32>,
    pub seasons: Vec<Season>,
    pub tmdb: Option<TmdbInfo>, // для LUMI-9b
}

#[derive(Serialize, Clone)]
pub struct Season {
    pub number: u32, // 3
    pub episodes: Vec<Episode>,
}

#[derive(Serialize, Clone)]
pub struct Episode {
    pub number: u32, // 1
    pub path: String,
    pub name: String, // полное имя файла
    pub parsed: ParsedVideo,
    pub watched: bool,
    pub position: Option<f64>,
    pub duration: Option<f64>,
}

#[derive(Serialize)]
pub struct Library {
    pub movies: Vec<VideoFile>,
    pub tv_shows: Vec<TvShow>,
}

#[derive(Serialize)]
pub struct ContinueItem {
    pub path: String,
    pub title: String,
    pub poster_url: Option<String>,
    pub position: f64,
    pub duration: f64,
    pub progress: f64,
    pub media_type: MediaType,
}

#[tauri::command]
pub async fn scan_all(app: tauri::AppHandle) -> Result<Library, String> {
    crate::log_info!("=== scan_all started ===");

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

    let conn = crate::cache::init_db().map_err(|e| e.to_string())?;

    let video_extensions = ["mkv", "mp4", "avi", "mov", "wmv", "flv", "webm"];
    let mut new_uids: Vec<String> = Vec::new();

    // === Этап 1: сканирование файлов ===
    for (idx, folder) in folders.iter().enumerate() {
        let path = Path::new(&folder.path);
        if !path.exists() || !path.is_dir() {
            crate::log_info!("Skipping invalid folder: {}", folder.path);
            continue;
        }

        app.emit(
            "scan_progress",
            serde_json::json!({
                "current": idx + 1,
                "total": total_folders,
                "folder": folder.path,
            }),
        )
        .ok();

        crate::log_info!("Scanning: {} ({:?})", folder.path, folder.media_type);

        for entry in WalkDir::new(&folder.path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }

            let ext_str = match file_path.extension() {
                Some(ext) => ext.to_string_lossy().to_lowercase(),
                None => continue,
            };
            if !video_extensions.contains(&ext_str.as_str()) {
                continue;
            }

            let name = file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let full_path = file_path.to_string_lossy().to_string();
            let uid = crate::cache::file_uid(&name);

            // Проверяем, есть ли уже в БД
            if let Some(existing) = crate::cache::find_by_uid(&conn, &uid) {
                // Обновляем путь, если файл переместили
                if existing.path != full_path {
                    let _ = crate::cache::update_path(&conn, &uid, &full_path);
                }
                continue;
            }

            // Новый файл — парсим и вставляем
            let media_type_str = match folder.media_type {
                crate::config::MediaType::Movie => "movie",
                crate::config::MediaType::TvShows => "tv_shows",
            };

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let (parsed_title, parsed_year, season, episode) = match folder.media_type {
                crate::config::MediaType::Movie => {
                    let parsed = parse_filename(&name);
                    (parsed.title, parsed.year, None, None)
                }
                crate::config::MediaType::TvShows => {
                    let tv = crate::tv_parser::parse_tv_filename(&name);
                    (tv.title, tv.year, tv.season, tv.episode)
                }
            };

            let item = crate::cache::MediaItem {
                uid: uid.clone(),
                path: full_path,
                name,
                media_type: media_type_str.to_string(),
                tmdb_id: None,
                title: None,
                original_title: None,
                overview: None,
                poster_path: None,
                rating: None,
                release_date: None,
                parsed_title,
                parsed_year,
                season,
                episode,
                scanned_at: now,
                updated_at: now,
            };

            if let Err(e) = crate::cache::insert_media_item(&conn, &item) {
                crate::log_info!("Failed to insert {}: {}", uid, e);
                continue;
            }

            new_uids.push(uid);
        }
    }

    crate::log_info!("=== New files: {} ===", new_uids.len());

    // === Этап 2: TMDB для новых фильмов ===
    let all_items = crate::cache::get_all_media_items(&conn);
    let new_movies: Vec<&crate::cache::MediaItem> = all_items
        .iter()
        .filter(|i| new_uids.contains(&i.uid) && i.media_type == "movie")
        .collect();

    crate::log_info!("=== TMDB lookup for {} new movies ===", new_movies.len());

    for item in new_movies {
        match crate::tmdb::get_or_fetch(
            &client,
            &api_key,
            &item.parsed_title,
            item.parsed_year,
        )
        .await
        {
            Ok(Some(movie)) => {
                let _ = crate::cache::update_tmdb(
                    &conn,
                    &item.uid,
                    movie.id,
                    &movie.title,
                    movie.original_title.as_deref(),
                    movie.overview.as_deref(),
                    movie.poster_path.as_deref(),
                    movie.vote_average,
                    movie.release_date.as_deref(),
                );
                crate::log_info!("  → Matched: {}", movie.title);
            }
            Ok(None) => {
                crate::log_info!("  → No match for '{}'", item.parsed_title);
            }
            Err(e) => {
                crate::log_info!("  → TMDB error for '{}': {}", item.parsed_title, e);
            }
        }
    }

    // === Этап 3: TMDB для новых сериалов ===
    // Группируем новые эпизоды по сериалу и делаем один запрос на сериал
    let new_tv: Vec<&crate::cache::MediaItem> = all_items
        .iter()
        .filter(|i| new_uids.contains(&i.uid) && i.media_type == "tv_shows")
        .collect();

    // Группируем по (parsed_title, year)
    use std::collections::HashMap;
    let mut tv_groups: HashMap<(String, Option<u32>), Vec<&crate::cache::MediaItem>> = HashMap::new();
    for item in new_tv {
        let key = (item.parsed_title.to_lowercase(), item.parsed_year);
        tv_groups.entry(key).or_default().push(item);
    }

    crate::log_info!("=== TMDB lookup for {} new TV shows ===", tv_groups.len());

    for ((title, year), episodes) in tv_groups {
        match crate::tmdb::get_or_fetch_tv(&client, &api_key, &title, year).await {
            Ok(Some(tv)) => {
                for ep in &episodes {
                    let _ = crate::cache::update_tmdb(
                        &conn,
                        &ep.uid,
                        tv.id,
                        &tv.name,
                        tv.original_name.as_deref(),
                        tv.overview.as_deref(),
                        tv.poster_path.as_deref(),
                        tv.vote_average,
                        tv.first_air_date.as_deref(),
                    );
                }
                crate::log_info!("  → Matched TV: {} ({} episodes)", tv.name, episodes.len());
            }
            Ok(None) => {
                crate::log_info!("  → No match for TV '{}'", title);
            }
            Err(e) => {
                crate::log_info!("  → TMDB error for TV '{}': {}", title, e);
            }
        }
    }

    // === Этап 4: сборка Library из media_items ===
    let final_items = crate::cache::get_all_media_items(&conn);
    let library = build_library(final_items, &conn);

    Ok(library)
}

#[tauri::command]
pub fn get_continue_watching(paths: Vec<String>) -> Vec<ContinueItem> {
    let conn = match crate::cache::init_db() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let in_progress = crate::cache::get_in_progress(&conn);
    let path_set: std::collections::HashSet<&str> = paths.iter().map(|s| s.as_str()).collect();

    in_progress
        .into_iter()
        .filter(|w| path_set.contains(w.file_path.as_str()))
        .map(|w| ContinueItem {
            title: extract_title_from_path(&w.file_path),
            path: w.file_path,
            poster_url: None, // заполним на фронтенде
            position: w.position,
            duration: w.duration,
            progress: w.position / w.duration,
            media_type: MediaType::Movie, // заполним на фронтенде
        })
        .collect()
}

fn extract_title_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

#[tauri::command]
pub fn set_watched_bulk(paths: Vec<String>, watched: bool) -> Result<(), String> {
    let conn = crate::cache::init_db().map_err(|e| e.to_string())?;
    crate::cache::set_watched_bulk(&conn, &paths, watched).map_err(|e| e.to_string())
}

fn build_library(items: Vec<crate::cache::MediaItem>, conn: &Connection) -> Library {
    // Фильмы — только сопоставленные
    let movies: Vec<VideoFile> = items
        .iter()
        .filter(|i| i.media_type == "movie" && i.tmdb_id.is_some())
        .map(|i| media_item_to_video_file(i, conn))
        .collect();

    // Сериалы — группировка
    let tv_shows = build_tv_shows(&items, conn);

    Library { movies, tv_shows }
}

fn media_item_to_video_file(item: &crate::cache::MediaItem, conn: &Connection) -> VideoFile {
    let parsed = parse_filename(&item.name);
    let tmdb = item.tmdb_id.map(|id| TmdbInfo {
        id,
        title: item.title.clone().unwrap_or_default(),
        original_title: item.original_title.clone(),
        overview: item.overview.clone(),
        poster_url: item
            .poster_path
            .as_ref()
            .map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
        rating: item.rating,
    });

    // watch info
   let watch = crate::cache::get_watch_info(conn, &item.path);

    VideoFile {
        path: item.path.clone(),
        name: item.name.clone(),
        extension: item.name.rsplit('.').next().unwrap_or("").to_string(),
        parsed,
        tmdb,
        media_type: crate::config::MediaType::Movie,
        watched: watch.watched,
        position: watch.position,
        duration: watch.duration,
    }
}

fn build_tv_shows(items: &[crate::cache::MediaItem], conn: &Connection) -> Vec<TvShow> {
    use std::collections::HashMap;

    // Группируем эпизоды по (tmdb_id или parsed_title, year)
    let mut groups: HashMap<(Option<u32>, String), Vec<&crate::cache::MediaItem>> = HashMap::new();

    for item in items.iter().filter(|i| i.media_type == "tv_shows") {
        // Пропускаем без season/episode
        if item.season.is_none() || item.episode.is_none() {
            continue;
        }
        // Пропускаем несопоставленные (tmdb_id = None)
        if item.tmdb_id.is_none() {
            continue;
        }
        let key = (item.tmdb_id, item.parsed_title.to_lowercase());
        groups.entry(key).or_default().push(item);
    }

    let conn = crate::cache::init_db().ok();
    let mut shows = Vec::new();

    for ((tmdb_id, _), episodes) in groups {
        let first = episodes[0];

        let mut seasons_map: HashMap<u32, Vec<Episode>> = HashMap::new();
        for ep in &episodes {
            let season_num = ep.season.unwrap();
            let episode_num = ep.episode.unwrap();

            let watch = conn
                .as_ref()
                .map(|c| crate::cache::get_watch_info(c, &ep.path))
                .unwrap_or_default();

            seasons_map.entry(season_num).or_default().push(Episode {
                number: episode_num,
                path: ep.path.clone(),
                name: ep.name.clone(),
                parsed: parse_filename(&ep.name),
                watched: watch.watched,
                position: watch.position,
                duration: watch.duration,
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

        let tmdb = tmdb_id.map(|id| TmdbInfo {
            id,
            title: first.title.clone().unwrap_or_default(),
            original_title: first.original_title.clone(),
            overview: first.overview.clone(),
            poster_url: first
                .poster_path
                .as_ref()
                .map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
            rating: first.rating,
        });

        shows.push(TvShow {
            title: first
                .title
                .clone()
                .unwrap_or_else(|| first.parsed_title.clone()),
            year: first.parsed_year,
            seasons,
            tmdb,
        });
    }

    shows.sort_by(|a, b| a.title.cmp(&b.title));
    shows
}

#[derive(Serialize)]
pub struct UndefinedItem {
    pub uid: String,
    pub path: String,
    pub file_name: String,
    pub display_title: String,
    pub media_type: String,
    pub year: Option<u32>,
}

#[tauri::command]
pub fn get_undefined_items() -> Vec<UndefinedItem> {
    let conn = match crate::cache::init_db() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    crate::cache::get_undefined(&conn)
        .into_iter()
        .map(|i| UndefinedItem {
            uid: i.uid,
            path: i.path,
            file_name: i.name,
            display_title: i.parsed_title,
            media_type: i.media_type,
            year: i.parsed_year,
        })
        .collect()
}