use crate::tmdb::{MatchResult, TmdbMovie};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CachedShow {
    pub tmdb_id: u32,
    pub name: String,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub rating: Option<f64>,
    pub first_air_date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WatchStatus {
    pub file_path: String,
    pub position: f64,
    pub duration: f64,
}

#[derive(Debug, Clone)]
pub struct CachedMovie {
    pub tmdb_id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub rating: Option<f64>,
    pub release_date: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WatchInfo {
    pub watched: bool,
    pub position: Option<f64>,
    pub duration: Option<f64>,
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
        "CREATE TABLE IF NOT EXISTS movies (
            id INTEGER PRIMARY KEY,
            tmdb_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            original_title TEXT,
            overview TEXT,
            poster_path TEXT,
            rating REAL,
            release_date TEXT,
            cached_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_title_year ON movies(title)",
        [],
    )?;

    // Новая таблица для сериалов
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tv_shows (
            id INTEGER PRIMARY KEY,
            tmdb_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            original_name TEXT,
            overview TEXT,
            poster_path TEXT,
            rating REAL,
            first_air_date TEXT,
            cached_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tv_name ON tv_shows(name)",
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

pub fn find_cached(conn: &Connection, title: &str) -> Option<CachedMovie> {
    conn.query_row(
        "SELECT tmdb_id, title, original_title, overview, poster_path, rating, release_date
         FROM movies
         WHERE LOWER(title) = LOWER(?1)
         LIMIT 1",
        rusqlite::params![title],
        |row| {
            Ok(CachedMovie {
                tmdb_id: row.get(0)?,
                title: row.get(1)?,
                original_title: row.get(2)?,
                overview: row.get(3)?,
                poster_path: row.get(4)?,
                rating: row.get(5)?,
                release_date: row.get(6)?,
            })
        },
    )
    .optional()
    .unwrap_or(None)
}

pub fn save_movie(conn: &Connection, movie: &TmdbMovie) -> SqlResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    conn.execute(
        "INSERT OR REPLACE INTO movies
         (tmdb_id, title, original_title, overview, poster_path, rating, release_date, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            movie.id,
            movie.title,
            movie.original_title,
            movie.overview,
            movie.poster_path,
            movie.vote_average,
            movie.release_date,
            now,
        ],
    )?;

    Ok(())
}

pub fn find_cached_tv(conn: &Connection, name: &str) -> Option<CachedShow> {
    conn.query_row(
        "SELECT tmdb_id, name, overview, poster_path, rating, first_air_date
         FROM tv_shows
         WHERE LOWER(name) = LOWER(?1)
         LIMIT 1",
        rusqlite::params![name],
        |row| {
            Ok(CachedShow {
                tmdb_id: row.get(0)?,
                name: row.get(1)?,
                original_name: row.get(2)?,
                overview: row.get(2)?,
                poster_path: row.get(3)?,
                rating: row.get(4)?,
                first_air_date: row.get(5)?,
            })
        },
    )
    .optional()
    .unwrap_or(None)
}

pub fn save_tv_show(conn: &Connection, show: &crate::tmdb::TmdbShow) -> SqlResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    conn.execute(
        "INSERT OR REPLACE INTO tv_shows
         (tmdb_id, name, original_name, overview, poster_path, rating, first_air_date, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            show.id,
            show.name,
            show.original_name,
            show.overview,
            show.poster_path,
            show.vote_average,
            show.first_air_date,
            now,
        ],
    )?;

    Ok(())
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