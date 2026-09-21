
## Key concepts

### media_items (SQLite)

Every scanned file is stored, **matched or not**. Primary key = `uid` (xxhash of file name).

| Column | Description |
| :--- | :--- |
| `uid` | xxhash64(file_name) |
| `path`, `name` | full path + file name |
| `media_type` | "movie" \| "tv_shows" |
| `tmdb_id` | NULL if not matched |
| `title`, `original_title`, `overview`, `poster_path`, `rating`, `release_date` | TMDB data |
| `parsed_title`, `parsed_year` | from filename parser |
| `season`, `episode` | for TV shows |
| `scanned_at`, `updated_at` | unix timestamps |

### watch_status (SQLite)

Keyed by `file_path` (not uid). Tracks playback progress.

- `watched` = 1 when `position / duration >= 0.95`
- `position`, `duration` — saved during mpv playback via IPC
- `set_watched_bulk(paths, watched)` — manual toggle

### TMDB integration

- Proxy: **hardcoded** `socks5h://127.0.0.1:9090` in `tmdb.rs::build_client()`
- All requests need VPN/SOCKS5
- `.env` with `TMDB_API_KEY` in working directory
- Posters: cached to disk, served via `convertFileSrc()`
- Manual match: `apply_tmdb_match(tmdb_id, media_type, uid)` — updates `media_items`

### mpv playback

- Launched with `--input-ipc-server=\\.\pipe\mpvsocket --fullscreen --keep-open=no --idle=no`
- Rust polls `time-pos` and `duration` every 2s while mpv alive
- On exit: `mark_watched` saved, `watch_status_updated` event emitted
- Resume: `play_video(path, start_position)` → `mpv --start=<sec>`

## Tauri commands

| Command | Purpose |
| :--- | :--- |
| `scanner::scan_all` | scan all folders, TMDB match new files, return Library |
| `scanner::get_continue_watching` | return in-progress items |
| `scanner::set_watched_bulk` | manual watched toggle |
| `scanner::get_undefined_items` | unmatched files |
| `tmdb::get_poster` | download/cache poster by tmdb_id |
| `tmdb::search_tmdb_manual` | manual search |
| `tmdb::apply_tmdb_match` | apply manual match |
| `tmdb::fetch_poster_preview` | base64 poster for match modal |
| `player::play_video` | launch mpv with optional start |
| `player::toggle_fullscreen` | F11 |
| `config::get_folders` / `add_folder` / `remove_folder` | folder management |

## Backlog

### In progress
- **LUMI-21b:** Vue refactoring
  - ✅ `types.ts`, composables, `MediaPoster`, `MovieCard`, `ShowCard`
  - ⏳ `ContextMenu.vue`, `MovieModal.vue`, `ShowModal.vue`, `MatchModal.vue`
  - ⏳ `LibraryView.vue`, `App.vue` → thin shell

### Next
- **LUMI-17:** details page for movie/show (replaces modal)
- **LUMI-21e:** remove legacy `movies` / `tv_shows` tables, `find_cached`, `save_movie`, `save_manual_match`
- **LUMI-22:** season detection from folder name
- **LUMI-23:** singleton SQLite connection via `tauri::State`

### Before release
- **LUMI-14:** settings for player path, proxy, portable mode

### Low priority (wishlist)
- Audio track / subtitle selection before playback
- Skip intro via MKV chapters
- Parallel poster loading
- Dynamic context menu positioning
- Improved "remaining X of Y" format

### Post-release
- Android build
- Bluetooth remote support
- Audio SPDIF / WASAPI exclusive config

## Known issues

- `Blade Runner 2049` parsed as `Blade Runner` + year 2049 → wrong TMDB match (LUMI-22)
- TV show matching from context menu doesn't update `media_items` (only from "Undefined" section works)
- `init_db()` called per command — potential `database is locked` under load (LUMI-23)

## Conventions

- **Rust:** modules in `src-tauri/src/`, log via `crate::log_info!`
- **Vue:** `<script setup lang="ts">`, types from `@/types`
- **CSS:** global in `App.vue` for layout, `scoped` in components
- **Commits:** `feat:`, `fix:`, `refactor:` prefixes
- **Branches:** `feature/LUMI-XX`, `fix/...`