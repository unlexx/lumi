use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::settings::AppSettings;

pub struct AppState {
    pub conn: Mutex<Connection>,
    settings: Mutex<AppSettings>,
    pub player_active: AtomicBool,
    pub mpv_child: Mutex<Option<Child>>,
}

static PORTABLE: OnceLock<bool> = OnceLock::new();

pub fn set_portable(v: bool) {
    let _ = PORTABLE.set(v);
}

pub fn is_portable() -> bool {
    *PORTABLE.get().unwrap_or(&false)
}

/// Директория, где лежит исполняемый файл.
/// None — если ОС не дала ответ (крайне редко).
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// Корень portable-режима: <exe_dir>/lumi-data/
fn portable_root() -> PathBuf {
    exe_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lumi-data")
}

impl AppState {
    pub fn init() -> SqlResult<Self> {
        let conn = init_db()?;
        let settings = crate::settings::load_settings();
        Ok(Self {
            conn: Mutex::new(conn),
            settings: Mutex::new(settings),
            player_active: AtomicBool::new(false),
            mpv_child: Mutex::new(None),
        })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("SQLite mutex poisoned")
    }

    pub fn settings(&self) -> std::sync::MutexGuard<'_, AppSettings> {
        self.settings.lock().expect("settings mutex poisoned")
    }

    pub fn set_player_active(&self, v: bool) {
        self.player_active.store(v, Ordering::SeqCst);
    }

    pub fn is_player_active(&self) -> bool {
        self.player_active.load(Ordering::SeqCst)
    }

    /// Кладёт child mpv в state. Старый (если был) — убивается.
    pub fn set_mpv_child(&self, child: Child) {
        let mut guard = self.mpv_child.lock().expect("mpv_child mutex poisoned");
        if let Some(mut old) = guard.take() {
            let _ = old.kill();
        }
        *guard = Some(child);
    }

    /// Забирает child mpv из state.
    pub fn take_mpv_child(&self) -> Option<Child> {
        self.mpv_child
            .lock()
            .expect("mpv_child mutex poisoned")
            .take()
    }
}

/// Корень для settings.json / folders.json.
/// Portable → <exe_dir>/lumi-data/; иначе → ProjectDirs::config_dir().
pub fn config_root() -> PathBuf {
    if is_portable() {
        let dir = portable_root();
        std::fs::create_dir_all(&dir).ok();
        dir
    } else {
        project_dirs().config_dir().to_path_buf()
    }
}

/// Корень для cache.db и posters/.
/// Portable → <exe_dir>/lumi-data/; иначе → ProjectDirs::data_dir().
pub fn data_root() -> PathBuf {
    if is_portable() {
        let dir = portable_root();
        std::fs::create_dir_all(&dir).ok();
        dir
    } else {
        let dir = project_dirs().data_dir().to_path_buf();
        std::fs::create_dir_all(&dir).ok();
        dir
    }
}

#[allow(dead_code)]
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
    pub episode_name: Option<String>,
    pub episode_overview: Option<String>,
    pub episode_still_path: Option<String>,
    pub episode_meta_fetched: bool,
    pub trailer_key: Option<String>,
    pub trailer_fetched: bool,
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
    let dir = data_root();
    std::fs::create_dir_all(&dir).ok();
    dir.join("cache.db")
}

pub fn posters_dir() -> PathBuf {
    // В обычном режиме posters лежат в cache_dir (как было раньше),
    // в portable — в data_root/posters.
    let base = if is_portable() {
        data_root()
    } else {
        let cache_dir = project_dirs().cache_dir().to_path_buf();
        std::fs::create_dir_all(&cache_dir).ok();
        cache_dir
    };
    let posters = base.join("posters");
    std::fs::create_dir_all(&posters).ok();
    posters
}

pub fn episode_stills_dir() -> PathBuf {
    let dir = posters_dir().join("episodes");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn trailers_dir() -> PathBuf {
    let dir = posters_dir().join("trailers");
    std::fs::create_dir_all(&dir).ok();
    dir
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
    episode_name TEXT,
    episode_overview TEXT,
    episode_still_path TEXT,
    episode_meta_fetched INTEGER NOT NULL DEFAULT 0,
    trailer_key TEXT,
    trailer_fetched INTEGER NOT NULL DEFAULT 0,
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
                ON CONFLICT(file_path) DO UPDATE SET watched = 1, position = 0, duration = 0",
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

pub fn find_by_uid(conn: &Connection, uid: &str) -> Option<MediaItem> {
    conn.query_row(
        "SELECT uid, path, name, media_type, tmdb_id, title, original_title, overview, poster_path,
                rating, release_date, parsed_title, parsed_year, season, episode,
       episode_name, episode_overview, episode_still_path, episode_meta_fetched,
       trailer_key, trailer_fetched,
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
                episode_name: row.get(15)?,
                episode_overview: row.get(16)?,
                episode_still_path: row.get(17)?,
                episode_meta_fetched: row.get::<_, i64>(18)? != 0,
                trailer_key: row.get(19)?,
                trailer_fetched: row.get::<_, i64>(20)? != 0,
                scanned_at: row.get(21)?,
                updated_at: row.get(22)?,
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
          episode,
          episode_name, episode_overview, episode_still_path, episode_meta_fetched,
          trailer_key, trailer_fetched,
          scanned_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                 ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
        rusqlite::params![
            item.uid,
            item.path,
            item.name,
            item.media_type,
            item.tmdb_id,
            item.title,
            item.original_title,
            item.overview,
            item.poster_path,
            item.rating,
            item.release_date,
            item.parsed_title,
            item.parsed_year,
            item.season,
            item.episode,
            item.episode_name,
            item.episode_overview,
            item.episode_still_path,
            if item.episode_meta_fetched {
                1i64
            } else {
                0i64
            },
            item.trailer_key,
            if item.trailer_fetched { 1i64 } else { 0i64 },
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

/// Батч-обновление TMDB-данных для списка uid.
/// Используется при ручном сопоставлении сериала — обновляем все эпизоды разом.
pub fn update_tmdb_for_uids(
    conn: &Connection,
    uids: &[String],
    tmdb_id: u32,
    title: &str,
    original_title: Option<&str>,
    overview: Option<&str>,
    poster_path: Option<&str>,
    rating: Option<f64>,
    release_date: Option<&str>,
) -> SqlResult<usize> {
    let now = now_ts();
    let tx = conn.unchecked_transaction()?;

    let mut count = 0;
    for uid in uids {
        let affected = tx.execute(
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
        count += affected;
    }

    tx.commit()?;
    Ok(count)
}

pub fn get_all_media_items(conn: &Connection) -> Vec<MediaItem> {
    let mut stmt = match conn.prepare(
        "SELECT uid, path, name, media_type, tmdb_id, title, original_title, overview, poster_path,
                rating, release_date, parsed_title, parsed_year, season, episode,
       episode_name, episode_overview, episode_still_path, episode_meta_fetched, trailer_key, trailer_fetched,
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
            episode_name: row.get(15)?,
            episode_overview: row.get(16)?,
            episode_still_path: row.get(17)?,
            episode_meta_fetched: row.get::<_, i64>(18)? != 0,
            trailer_key: row.get(19)?,
            trailer_fetched: row.get::<_, i64>(20)? != 0,
            scanned_at: row.get(21)?,
            updated_at: row.get(22)?,
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
       episode_name, episode_overview, episode_still_path, episode_meta_fetched, trailer_key, trailer_fetched,
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
            episode_name: row.get(15)?,
            episode_overview: row.get(16)?,
            episode_still_path: row.get(17)?,
            episode_meta_fetched: row.get::<_, i64>(18)? != 0,
            trailer_key: row.get(19)?,
            trailer_fetched: row.get::<_, i64>(20)? != 0,
            scanned_at: row.get(21)?,
            updated_at: row.get(22)?,
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

pub fn update_episode_meta(
    conn: &Connection,
    uid: &str,
    episode_name: Option<&str>,
    episode_overview: Option<&str>,
    episode_still_path: Option<&str>,
) -> SqlResult<()> {
    let now = now_ts();
    conn.execute(
        "UPDATE media_items
         SET episode_name = ?1, episode_overview = ?2, episode_still_path = ?3,
             episode_meta_fetched = 1, updated_at = ?4
         WHERE uid = ?5",
        rusqlite::params![episode_name, episode_overview, episode_still_path, now, uid,],
    )?;
    Ok(())
}

/// Читает состояние трейлера для uid.
/// None — если uid не найден.
/// Some((trailer_key, trailer_fetched)).
pub fn get_trailer_state(conn: &Connection, uid: &str) -> Option<(Option<String>, bool)> {
    conn.query_row(
        "SELECT trailer_key, trailer_fetched FROM media_items WHERE uid = ?1",
        rusqlite::params![uid],
        |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)? != 0)),
    )
    .optional()
    .unwrap_or(None)
}

/// Пишет ключ трейлера (или NULL, если трейлера нет) и выставляет trailer_fetched = 1.
pub fn set_trailer(conn: &Connection, uid: &str, trailer_key: Option<&str>) -> SqlResult<()> {
    let now = now_ts();
    conn.execute(
        "UPDATE media_items
         SET trailer_key = ?1, trailer_fetched = 1, updated_at = ?2
         WHERE uid = ?3",
        rusqlite::params![trailer_key, now, uid],
    )?;
    Ok(())
}
