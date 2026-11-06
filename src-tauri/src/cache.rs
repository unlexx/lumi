use crate::settings::AppSettings;
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

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
    pub backdrop_path: Option<String>,
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
    pub missing: bool,
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

pub fn backdrops_dir() -> PathBuf {
    let dir = posters_dir().join("backdrops");
    std::fs::create_dir_all(&dir).ok();
    dir
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
    init_schema(&conn)?;
    Ok(conn)
}

pub fn init_schema(conn: &Connection) -> SqlResult<()> {
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
    backdrop_path TEXT,
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
    missing INTEGER NOT NULL DEFAULT 0,
    scanned_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)",
        [],
    )?;

    // Defensive: если старая cache.db без колонки missing — добавим.
    // Штатно (по ARCHITECTURE) cache.db стирается, но ALTER безопасен.
    ensure_column_missing(conn)?;

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

    Ok(())
}

fn ensure_column_missing(conn: &Connection) -> SqlResult<()> {
    let has: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('media_items') WHERE name = 'missing'",
        [],
        |row| row.get(0),
    )?;
    if has == 0 {
        conn.execute(
            "ALTER TABLE media_items ADD COLUMN missing INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
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
        "SELECT uid, path, name, media_type, tmdb_id, title, original_title, overview,
                poster_path, backdrop_path,
                rating, release_date, parsed_title, parsed_year, season, episode,
                episode_name, episode_overview, episode_still_path, episode_meta_fetched,
                trailer_key, trailer_fetched, missing,
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
                backdrop_path: row.get(9)?,
                rating: row.get(10)?,
                release_date: row.get(11)?,
                parsed_title: row.get(12)?,
                parsed_year: row.get(13)?,
                season: row.get(14)?,
                episode: row.get(15)?,
                episode_name: row.get(16)?,
                episode_overview: row.get(17)?,
                episode_still_path: row.get(18)?,
                episode_meta_fetched: row.get::<_, i64>(19)? != 0,
                trailer_key: row.get(20)?,
                trailer_fetched: row.get::<_, i64>(21)? != 0,
                missing: row.get::<_, i64>(22)? != 0,
                scanned_at: row.get(23)?,
                updated_at: row.get(24)?,
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
          poster_path, backdrop_path, rating, release_date, parsed_title, parsed_year,
          season, episode,
          episode_name, episode_overview, episode_still_path, episode_meta_fetched,
          trailer_key, trailer_fetched, missing,
          scanned_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                 ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
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
            item.backdrop_path,
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
            if item.missing { 1i64 } else { 0i64 },
            item.scanned_at,
            item.updated_at,
        ],
    )?;
    Ok(())
}

pub fn update_path(conn: &Connection, uid: &str, new_path: &str) -> SqlResult<()> {
    let now = now_ts();
    conn.execute(
        "UPDATE media_items SET path = ?1, missing = 0, updated_at = ?2 WHERE uid = ?3",
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
    backdrop_path: Option<&str>, // ← новое
    rating: Option<f64>,
    release_date: Option<&str>,
) -> SqlResult<()> {
    let now = now_ts();
    conn.execute(
        "UPDATE media_items
         SET tmdb_id = ?1, title = ?2, original_title = ?3, overview = ?4,
             poster_path = ?5, backdrop_path = ?6, rating = ?7, release_date = ?8,
             updated_at = ?9
         WHERE uid = ?10",
        rusqlite::params![
            tmdb_id,
            title,
            original_title,
            overview,
            poster_path,
            backdrop_path,
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
    backdrop_path: Option<&str>, // ← новое
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
                 poster_path = ?5, backdrop_path = ?6, rating = ?7, release_date = ?8,
                 updated_at = ?9
             WHERE uid = ?10",
            rusqlite::params![
                tmdb_id,
                title,
                original_title,
                overview,
                poster_path,
                backdrop_path,
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
        "SELECT uid, path, name, media_type, tmdb_id, title, original_title, overview, poster_path, backdrop_path,
                rating, release_date, parsed_title, parsed_year, season, episode,
                episode_name, episode_overview, episode_still_path, episode_meta_fetched,
                trailer_key, trailer_fetched, missing,
                scanned_at, updated_at
         FROM media_items
         WHERE missing = 0",
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
            backdrop_path: row.get(9)?,
            rating: row.get(10)?,
            release_date: row.get(11)?,
            parsed_title: row.get(12)?,
            parsed_year: row.get(13)?,
            season: row.get(14)?,
            episode: row.get(15)?,
            episode_name: row.get(16)?,
            episode_overview: row.get(17)?,
            episode_still_path: row.get(18)?,
            episode_meta_fetched: row.get::<_, i64>(19)? != 0,
            trailer_key: row.get(20)?,
            trailer_fetched: row.get::<_, i64>(21)? != 0,
            missing: row.get::<_, i64>(22)? != 0,
            scanned_at: row.get(23)?,
            updated_at: row.get(24)?,
        })
    });

    match rows {
        Ok(iter) => iter
            .filter_map(|r| match r {
                Ok(v) => Some(v),
                Err(e) => {
                    crate::log_info!("get_all_media_items row error: {}", e);
                    None
                }
            })
            .collect(),
        Err(e) => {
            crate::log_info!("get_all_media_items prepare error: {}", e);
            Vec::new()
        }
    }
}

pub fn get_undefined(conn: &Connection) -> Vec<MediaItem> {
    let mut stmt = match conn.prepare(
        "SELECT uid, path, name, media_type, tmdb_id, title, original_title, overview, poster_path, backdrop_path,
                rating, release_date, parsed_title, parsed_year, season, episode,
       episode_name, episode_overview, episode_still_path, episode_meta_fetched, trailer_key, trailer_fetched,
                scanned_at, updated_at
         FROM media_items
         WHERE tmdb_id IS NULL AND missing = 0",
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
            backdrop_path: row.get(9)?,
            rating: row.get(10)?,
            release_date: row.get(11)?,
            parsed_title: row.get(12)?,
            parsed_year: row.get(13)?,
            season: row.get(14)?,
            episode: row.get(15)?,
            episode_name: row.get(16)?,
            episode_overview: row.get(17)?,
            episode_still_path: row.get(18)?,
            episode_meta_fetched: row.get::<_, i64>(19)? != 0,
            trailer_key: row.get(20)?,
            trailer_fetched: row.get::<_, i64>(21)? != 0,
            missing: row.get::<_, i64>(22)? != 0,
            scanned_at: row.get(23)?,
            updated_at: row.get(24)?,
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

pub fn mark_missing(conn: &Connection, uid: &str) -> SqlResult<()> {
    let now = now_ts();
    conn.execute(
        "UPDATE media_items SET missing = 1, updated_at = ?1 WHERE uid = ?2",
        rusqlite::params![now, uid],
    )?;
    Ok(())
}

pub fn clear_missing(conn: &Connection, uid: &str) -> SqlResult<()> {
    let now = now_ts();
    conn.execute(
        "UPDATE media_items SET missing = 0, updated_at = ?1 WHERE uid = ?2 AND missing = 1",
        rusqlite::params![now, uid],
    )?;
    Ok(())
}

/// Помечает `missing = 1` все записи, которые:
///   - имеют `missing = 0` (уже помеченные не трогаем),
///   - лежат под одной из `valid_folders`,
///   - отсутствуют в `found_uids` (собранных со всех валидных папок).
///
/// `valid_folders` — только те папки из folders.json, у которых `is_dir()` = true.
/// Это защита от отмонтированных сетевых дисков: если папка недоступна, её
/// содержимое вообще не рассматривается.
pub fn mark_missing_not_in(
    conn: &Connection,
    found_uids: &HashSet<String>,
    valid_folders: &[String],
) -> SqlResult<usize> {
    if valid_folders.is_empty() {
        return Ok(0);
    }

    let mut stmt = conn.prepare("SELECT uid, path FROM media_items WHERE missing = 0")?;
    let candidates: Vec<String> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .filter(|(_, path)| valid_folders.iter().any(|f| path_under_folder(path, f)))
        .filter(|(uid, _)| !found_uids.contains(uid))
        .map(|(uid, _)| uid)
        .collect();

    if candidates.is_empty() {
        return Ok(0);
    }

    let tx = conn.unchecked_transaction()?;
    for uid in &candidates {
        mark_missing(&tx, uid)?;
    }
    tx.commit()?;
    Ok(candidates.len())
}

/// `true`, если `path` лежит внутри `folder` (по префиксу + разделителю).
/// Голый `starts_with` не годится: `/movies` матчит `/movies2/a.mkv`.
pub fn path_under_folder(path: &str, folder: &str) -> bool {
    if !path.starts_with(folder) {
        return false;
    }
    let rest = &path[folder.len()..];
    if rest.is_empty() {
        return true;
    }
    let first = rest.chars().next().unwrap();
    first == '\\' || first == '/'
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::collections::HashSet;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn sample_item(uid: &str, path: &str) -> MediaItem {
        MediaItem {
            uid: uid.to_string(),
            path: path.to_string(),
            name: path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("f.mkv")
                .to_string(),
            media_type: "movie".to_string(),
            tmdb_id: Some(1),
            title: Some("T".to_string()),
            original_title: None,
            overview: None,
            poster_path: None,
            backdrop_path: None,
            rating: None,
            release_date: None,
            parsed_title: "T".to_string(),
            parsed_year: None,
            season: None,
            episode: None,
            episode_name: None,
            episode_overview: None,
            episode_still_path: None,
            episode_meta_fetched: false,
            trailer_key: None,
            trailer_fetched: false,
            missing: false,
            scanned_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn schema_has_missing_column_default_false() {
        let conn = test_conn();
        insert_media_item(&conn, &sample_item("u1", "/f/a.mkv")).unwrap();
        assert!(!find_by_uid(&conn, "u1").unwrap().missing);
    }

    #[test]
    fn ensure_column_missing_is_idempotent() {
        let conn = test_conn();
        // Повторный вызов не должен падать и не должен дублировать колонку.
        ensure_column_missing(&conn).unwrap();
        ensure_column_missing(&conn).unwrap();
        insert_media_item(&conn, &sample_item("u1", "/f/a.mkv")).unwrap();
    }

    #[test]
    fn mark_missing_sets_flag() {
        let conn = test_conn();
        insert_media_item(&conn, &sample_item("u1", "/f/a.mkv")).unwrap();
        mark_missing(&conn, "u1").unwrap();
        assert!(find_by_uid(&conn, "u1").unwrap().missing);
    }

    #[test]
    fn clear_missing_resets_flag() {
        let conn = test_conn();
        insert_media_item(&conn, &sample_item("u1", "/f/a.mkv")).unwrap();
        mark_missing(&conn, "u1").unwrap();
        clear_missing(&conn, "u1").unwrap();
        assert!(!find_by_uid(&conn, "u1").unwrap().missing);
    }

    #[test]
    fn get_all_media_items_excludes_missing() {
        let conn = test_conn();
        insert_media_item(&conn, &sample_item("u1", "/f/a.mkv")).unwrap();
        insert_media_item(&conn, &sample_item("u2", "/f/b.mkv")).unwrap();
        mark_missing(&conn, "u2").unwrap();
        let all = get_all_media_items(&conn);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].uid, "u1");
    }

    #[test]
    fn get_undefined_excludes_missing() {
        let conn = test_conn();
        let mut a = sample_item("u1", "/f/a.mkv");
        a.tmdb_id = None;
        let mut b = sample_item("u2", "/f/b.mkv");
        b.tmdb_id = None;
        insert_media_item(&conn, &a).unwrap();
        insert_media_item(&conn, &b).unwrap();
        mark_missing(&conn, "u2").unwrap();
        let undef = get_undefined(&conn);
        assert_eq!(undef.len(), 1);
        assert_eq!(undef[0].uid, "u1");
    }

    #[test]
    fn mark_missing_not_in_marks_only_absent_items_in_valid_folders() {
        let conn = test_conn();
        insert_media_item(&conn, &sample_item("u1", "Z:\\movies\\a.mkv")).unwrap();
        insert_media_item(&conn, &sample_item("u2", "Z:\\movies\\b.mkv")).unwrap();
        insert_media_item(&conn, &sample_item("u3", "Y:\\other\\c.mkv")).unwrap();

        let mut found = HashSet::new();
        found.insert("u1".to_string()); // u1 на диске, u2 — нет, u3 вне валидных папок

        let folders = vec!["Z:\\movies".to_string()];
        assert_eq!(mark_missing_not_in(&conn, &found, &folders).unwrap(), 1);

        assert!(!find_by_uid(&conn, "u1").unwrap().missing);
        assert!(find_by_uid(&conn, "u2").unwrap().missing);
        assert!(!find_by_uid(&conn, "u3").unwrap().missing);
    }

    /// Ключевой сценарий LUMI-28: «отключил Z: → ничего не удалилось».
    #[test]
    fn mark_missing_not_in_skips_when_no_valid_folders() {
        let conn = test_conn();
        insert_media_item(&conn, &sample_item("u1", "Z:\\movies\\a.mkv")).unwrap();
        let found: HashSet<String> = HashSet::new();
        let folders: Vec<String> = Vec::new(); // Z: недоступен → is_dir()=false
        assert_eq!(mark_missing_not_in(&conn, &found, &folders).unwrap(), 0);
        assert!(!find_by_uid(&conn, "u1").unwrap().missing);
    }

    #[test]
    fn mark_missing_not_in_is_idempotent() {
        let conn = test_conn();
        insert_media_item(&conn, &sample_item("u1", "Z:\\movies\\a.mkv")).unwrap();
        let folders = vec!["Z:\\movies".to_string()];
        let empty = HashSet::new();
        assert_eq!(mark_missing_not_in(&conn, &empty, &folders).unwrap(), 1);
        // Второй прогон: запись уже missing=1, кандидатов нет.
        assert_eq!(mark_missing_not_in(&conn, &empty, &folders).unwrap(), 0);
    }

    #[test]
    fn update_path_clears_missing() {
        let conn = test_conn();
        insert_media_item(&conn, &sample_item("u1", "/old/a.mkv")).unwrap();
        mark_missing(&conn, "u1").unwrap();
        update_path(&conn, "u1", "/new/a.mkv").unwrap();
        let item = find_by_uid(&conn, "u1").unwrap();
        assert!(!item.missing);
        assert_eq!(item.path, "/new/a.mkv");
    }

    #[test]
    fn path_under_folder_handles_separators_and_prefix_traps() {
        assert!(path_under_folder("Z:\\movies\\a.mkv", "Z:\\movies"));
        assert!(path_under_folder("Z:\\movies/sub/a.mkv", "Z:\\movies"));
        assert!(path_under_folder("Z:\\movies", "Z:\\movies"));
        assert!(!path_under_folder("Z:\\movies2\\a.mkv", "Z:\\movies")); // префикс без разделителя
        assert!(!path_under_folder("Y:\\other\\a.mkv", "Z:\\movies"));
        assert!(!path_under_folder("Z:\\MOVIES\\a.mkv", "Z:\\movies")); // регистр
    }
}
