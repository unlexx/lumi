use crate::tmdb::MatchResult;
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub uid: String,
    pub path: String,
    pub name: String,
    pub media_type: String,
    pub tmdb_id: Option<u32>,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub rating: Option<f64>,
    pub release_date: Option<String>,
    pub parsed_title: String,
    pub parsed_year: Option<u32>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub scanned_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct WatchStatus {
    pub file_path: String,
    pub position: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Default)]
pub struct WatchInfo {
    pub watched: bool,
    pub position: Option<f64>,
    pub duration: Option<f64>,
}

/// UID файла = хеш от имени файла (с расширением).
/// Используется как первичный ключ в media_items.
pub fn file_uid(name: &str) -> String {
    use xxhash_rust::xxh64::xxh64;
    format!("{:016x}", xxh64(name.as_bytes(), 0))
}

pub fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("dev", "unlexx", "lumi").expect("Не удалось определить директории проекта")
}

pub fn db_path() -> PathBuf {
    let dirs = project_dirs();
    let data_dir = dirs.data_dir();
    std::fs::create_dir_all(data_dir).ok();
    data_dir.join("cache.db")
}

pub fn posters_dir() -> PathBuf {
    let dirs = project_dirs();
    let cache_dir = dirs.cache_dir();
    let posters = cache_dir.join("posters");
    std::fs::create_dir_all(&posters).ok();
    posters
}

pub fn init_db() -> SqlResult<Connection> {
    let conn = Connection::open(db_path())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS media_items (
    uid TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    media_type TEXT NOT NULL,
    tmdb_id INTEGER,
    title TEXT,
    original_title TEXT,
    overview TEXT,
    poster_path TEXT,
    rating REAL,
    release_date TEXT,
    parsed_title TEXT,
    parsed_year INTEGER,
    season INTEGER,
    episode INTEGER,
    scanned_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_media_tmdb ON media_items(tmdb_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_media_type ON media_items(media_type)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS watch_status (
        file_path TEXT PRIMARY KEY,
        watched INTEGER NOT NULL DEFAULT 0,
        position REAL,
        duration REAL,
        updated_at INTEGER NOT NULL
    )",
        [],
    )?;

    Ok(conn)
}

pub fn mark_watched(
    conn: &Connection,
    file_path: &str,
    position: f64,
    duration: f64,
) -> SqlResult<()> {
    let watched = if duration > 0.0 && position / duration >= 0.95 {
        1i64
    } else {
        0i64
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    conn.execute(
        "INSERT OR REPLACE INTO watch_status
         (file_path, watched, position, duration, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![file_path, watched, position, duration, now],
    )?;

    Ok(())
}

pub fn get_in_progress(conn: &Connection) -> Vec<WatchStatus> {
    let mut stmt = match conn.prepare(
        "SELECT file_path, watched, position, duration, updated_at
         FROM watch_status
         WHERE watched = 0
           AND duration > 0
           AND position / duration >= 0.05
           AND position / duration <= 0.95
         ORDER BY updated_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], |row| {
        Ok(WatchStatus {
            file_path: row.get(0)?,
            position: row.get(2)?,
            duration: row.get(3)?,
        })
    });

    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn get_watch_info(conn: &Connection, file_path: &str) -> WatchInfo {
    conn.query_row(
        "SELECT watched, position, duration
         FROM watch_status
         WHERE file_path = ?1",
        rusqlite::params![file_path],
        |row| {
            Ok(WatchInfo {
                watched: row.get::<_, i64>(0)? != 0,
                position: row.get(1)?,
                duration: row.get(2)?,
            })
        },
    )
    .optional()
    .unwrap_or(None)
    .unwrap_or_default()
}

pub fn set_watched_bulk(conn: &Connection, paths: &[String], watched: bool) -> SqlResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let tx = conn.unchecked_transaction()?;

    for path in paths {
        if watched {
            tx.execute(
                "INSERT INTO watch_status (file_path, watched, position, duration, updated_at)
                 VALUES (?1, 1, 0, 0, ?2)
                 ON CONFLICT(file_path) DO UPDATE SET watched = 1",
                rusqlite::params![path, now],
            )?;
        } else {
            tx.execute(
                "INSERT INTO watch_status (file_path, watched, position, duration, updated_at)
                 VALUES (?1, 0, 0, 0, ?2)
                 ON CONFLICT(file_path) DO UPDATE SET watched = 0, position = 0",
                rusqlite::params![path, now],
            )?;
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn save_manual_match(result: &MatchResult, media_type: &str) -> Result<(), String> {
    let conn = init_db().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let poster_path = result
        .poster_url
        .as_ref()
        .and_then(|url| url.split("/t/p/w500").nth(1))
        .map(|s| s.to_string());

    if media_type == "movie" {
        conn.execute(
            "INSERT OR REPLACE INTO movies
             (tmdb_id, title, original_title, overview, poster_path, rating, release_date, cached_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                result.tmdb_id,
                result.title,
                result.original_title,
                result.overview,
                poster_path,
                result.rating,
                None::<String>,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "INSERT OR REPLACE INTO tv_shows
             (tmdb_id, name, original_name, overview, poster_path, rating, first_air_date, cached_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                result.tmdb_id,
                result.title,
                result.original_title,
                result.overview,
                poster_path,
                result.rating,
                None::<String>,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn find_by_uid(conn: &Connection, uid: &str) -> Option<MediaItem> {
    conn.query_row(
        "SELECT uid, path, name, media_type, tmdb_id, title, original_title, overview, poster_path,
                rating, release_date, parsed_title, parsed_year, season, episode,
                scanned_at, updated_at
         FROM media_items WHERE uid = ?1",
        rusqlite::params![uid],
        |row| {
            Ok(MediaItem {
                uid: row.get(0)?,
                path: row.get(1)?,
                name: row.get(2)?,
                media_type: row.get(3)?,
                tmdb_id: row.get(4)?,
                title: row.get(5)?,
                original_title: row.get(6)?,
                overview: row.get(7)?,
                poster_path: row.get(8)?,
                rating: row.get(9)?,
                release_date: row.get(10)?,
                parsed_title: row.get(11)?,
                parsed_year: row.get(12)?,
                season: row.get(13)?,
                episode: row.get(14)?,
                scanned_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        },
    )
    .optional()
    .unwrap_or(None)
}

pub fn insert_media_item(conn: &Connection, item: &MediaItem) -> SqlResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO media_items
         (uid, path, name, media_type, tmdb_id, title, original_title, overview,
          poster_path, rating, release_date, parsed_title, parsed_year, season,
          episode, scanned_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        rusqlite::params![
            item.uid,
            item.path,
            item.name,
            item.media_type,
            item.tmdb_id,
            item.title,
            item.original_title, // ← добавили
            item.overview,
            item.poster_path,
            item.rating,
            item.release_date,
            item.parsed_title,
            item.parsed_year,
            item.season,
            item.episode,
            item.scanned_at,
            item.updated_at,
        ],
    )?;
    Ok(())
}

pub fn update_path(conn: &Connection, uid: &str, new_path: &str) -> SqlResult<()> {
    let now = now_ts();
    conn.execute(
        "UPDATE media_items SET path = ?1, updated_at = ?2 WHERE uid = ?3",
        rusqlite::params![new_path, now, uid],
    )?;
    Ok(())
}

pub fn update_tmdb(
    conn: &Connection,
    uid: &str,
    tmdb_id: u32,
    title: &str,
    original_title: Option<&str>,
    overview: Option<&str>,
    poster_path: Option<&str>,
    rating: Option<f64>,
    release_date: Option<&str>,
) -> SqlResult<()> {
    let now = now_ts();
    conn.execute(
        "UPDATE media_items
         SET tmdb_id = ?1, title = ?2, original_title = ?3, overview = ?4,
             poster_path = ?5, rating = ?6, release_date = ?7, updated_at = ?8
         WHERE uid = ?9",
        rusqlite::params![
            tmdb_id,
            title,
            original_title,
            overview,
            poster_path,
            rating,
            release_date,
            now,
            uid,
        ],
    )?;
    Ok(())
}

pub fn get_all_media_items(conn: &Connection) -> Vec<MediaItem> {
    let mut stmt = match conn.prepare(
        "SELECT uid, path, name, media_type, tmdb_id, title, original_title, overview, poster_path,
                rating, release_date, parsed_title, parsed_year, season, episode,
                scanned_at, updated_at
         FROM media_items",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], |row| {
        Ok(MediaItem {
            uid: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            media_type: row.get(3)?,
            tmdb_id: row.get(4)?,
            title: row.get(5)?,
            original_title: row.get(6)?,
            overview: row.get(7)?,
            poster_path: row.get(8)?,
            rating: row.get(9)?,
            release_date: row.get(10)?,
            parsed_title: row.get(11)?,
            parsed_year: row.get(12)?,
            season: row.get(13)?,
            episode: row.get(14)?,
            scanned_at: row.get(15)?,
            updated_at: row.get(16)?,
        })
    });

    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn get_undefined(conn: &Connection) -> Vec<MediaItem> {
    let mut stmt = match conn.prepare(
        "SELECT uid, path, name, media_type, tmdb_id, title, original_title, overview, poster_path,
                rating, release_date, parsed_title, parsed_year, season, episode,
                scanned_at, updated_at
         FROM media_items
         WHERE tmdb_id IS NULL",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], |row| {
        Ok(MediaItem {
            uid: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            media_type: row.get(3)?,
            tmdb_id: row.get(4)?,
            title: row.get(5)?,
            original_title: row.get(6)?,
            overview: row.get(7)?,
            poster_path: row.get(8)?,
            rating: row.get(9)?,
            release_date: row.get(10)?,
            parsed_title: row.get(11)?,
            parsed_year: row.get(12)?,
            season: row.get(13)?,
            episode: row.get(14)?,
            scanned_at: row.get(15)?,
            updated_at: row.get(16)?,
        })
    });

    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn save_manual_match_to_item(result: &MatchResult, uid: &str) -> Result<(), String> {
    let conn = init_db().map_err(|e| e.to_string())?;
    let now = now_ts();

    let poster_path = result
        .poster_url
        .as_ref()
        .and_then(|url| url.split("/t/p/w500").nth(1))
        .map(|s| s.to_string());

    conn.execute(
        "UPDATE media_items
         SET tmdb_id = ?1, title = ?2, original_title = ?3, overview = ?4,
             poster_path = ?5, rating = ?6, updated_at = ?7
         WHERE uid = ?8",
        rusqlite::params![
            result.tmdb_id,
            result.title,
            result.original_title,
            result.overview,
            poster_path,
            result.rating,
            now,
            uid,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
