use crate::tmdb::{TmdbMovie, TmdbShow};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CachedShow {
    pub tmdb_id: u32,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub rating: Option<f64>,
    pub first_air_date: Option<String>,
}

pub fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("dev", "unlexx", "lumi")
        .expect("Не удалось определить директории проекта")
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

    Ok(conn)
}

#[derive(Debug, Clone)]
pub struct CachedMovie {
    pub tmdb_id: u32,
    pub title: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub rating: Option<f64>,
    pub release_date: Option<String>,
}

pub fn find_cached(conn: &Connection, title: &str) -> Option<CachedMovie> {
    conn.query_row(
        "SELECT tmdb_id, title, overview, poster_path, rating, release_date
         FROM movies
         WHERE LOWER(title) = LOWER(?1)
         LIMIT 1",
        rusqlite::params![title],
        |row| {
            Ok(CachedMovie {
                tmdb_id: row.get(0)?,
                title: row.get(1)?,
                overview: row.get(2)?,
                poster_path: row.get(3)?,
                rating: row.get(4)?,
                release_date: row.get(5)?,
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
         (tmdb_id, title, overview, poster_path, rating, release_date, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            movie.id,
            movie.title,
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
         (tmdb_id, name, overview, poster_path, rating, first_air_date, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            show.id,
            show.name,
            show.overview,
            show.poster_path,
            show.vote_average,
            show.first_air_date,
            now,
        ],
    )?;

    Ok(())
}