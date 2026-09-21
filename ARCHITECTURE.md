## Key concepts

### media_items (SQLite)

Every scanned file is stored, **matched or not**. Primary key = `uid` (xxhash of file name).

| Column                                                                         | Description           |
| :----------------------------------------------------------------------------- | :-------------------- |
| `uid`                                                                          | xxhash64(file_name)   |
| `path`, `name`                                                                 | full path + file name |
| `media_type`                                                                   | "movie" \| "tv_shows" |
| `tmdb_id`                                                                      | NULL if not matched   |
| `title`, `original_title`, `overview`, `poster_path`, `rating`, `release_date` | TMDB data             |
| `parsed_title`, `parsed_year`                                                  | from filename parser  |
| `season`, `episode`                                                            | for TV shows          |
| `scanned_at`, `updated_at`                                                     | unix timestamps       |

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

| Command                                                | Purpose                                                |
| :----------------------------------------------------- | :----------------------------------------------------- |
| `scanner::scan_all`                                    | scan all folders, TMDB match new files, return Library |
| `scanner::get_continue_watching`                       | return in-progress items                               |
| `scanner::set_watched_bulk`                            | manual watched toggle                                  |
| `scanner::get_undefined_items`                         | unmatched files                                        |
| `tmdb::get_poster`                                     | download/cache poster by tmdb_id                       |
| `tmdb::search_tmdb_manual`                             | manual search                                          |
| `tmdb::apply_tmdb_match`                               | apply manual match                                     |
| `tmdb::fetch_poster_preview`                           | base64 poster for match modal                          |
| `player::play_video`                                   | launch mpv with optional start                         |
| `player::toggle_fullscreen`                            | F11                                                    |
| `config::get_folders` / `add_folder` / `remove_folder` | folder management                                      |

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

## History

### Foundation (LUMI-1 — LUMI-6)

- **LUMI-1:** Tauri 2 + Vue 3 + TypeScript scaffold
- **LUMI-2:** video folder scanner (`walkdir`, extension filter)
- **LUMI-3:** movie filename parser (title, year, resolution, source, codec)
- **LUMI-4:** TMDB integration via SOCKS5 proxy, SQLite cache for metadata
- **LUMI-5:** local poster cache (`%LOCALAPPDATA%`), `convertFileSrc`
- **LUMI-6:** poster grid UI, movie details modal, system player

### Playback & tracking (LUMI-7 — LUMI-13)

- **LUMI-7:** TV filename parser (`SxxExx`), grouping into `TvShow` / `Season` / `Episode`
- **LUMI-8:** folder config (`folders.json`) with `movie` / `tv_shows` types, settings UI
- **LUMI-9:** TMDB metadata for TV shows (`/search/tv`), separate cache table
- **LUMI-10:** auto-scan on startup, progress events (`scan_progress`)
- **LUMI-11:** incremental file scanning (skip existing)
- **LUMI-12:** mpv integration with IPC (`\\.\pipe\mpvsocket`), watch tracking (`watch_status`), auto-mark watched at 95%
- **LUMI-13:** "Continue Watching" section with resume from saved position

### Library management (LUMI-14 — LUMI-19)

- **LUMI-14:** (deferred) player path, proxy, portable mode settings
- **LUMI-15:** watch progress on all cards, episode counts for TV shows
- **LUMI-16:** context menu ("⋮") on cards: play, resume, mark watched, match
- **LUMI-17:** (planned) details page
- **LUMI-18:** (planned) manual TMDB matching from library
- **LUMI-19:** manual TMDB matching modal (search + apply)

### Data architecture (LUMI-20 — LUMI-21)

- **LUMI-20:** `media_items` table with UID (xxhash of filename), stores **all** scanned files including unmatched
- **LUMI-21a:** "Undefined" section for unmatched files, manual match from there
- **LUMI-21c:** manual match updates `media_items` via `save_manual_match_to_item`
- **LUMI-21d:** `uid` in `VideoFile`, context menu matching works
- **LUMI-21e:** (planned) remove legacy `movies` / `tv_shows` tables
- **LUMI-21b:** (in progress) Vue refactoring into components / composables

### Future (planned)

- **LUMI-22:** season detection from folder name
- **LUMI-23:** singleton SQLite connection via `tauri::State`
- **LUMI-17:** details page for movie / show
- **LUMI-14:** settings for player, proxy, portable mode
- **Post-release:** Android, Bluetooth remote, audio config

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

## Commit conventions

Format: `<type>: <description>` or `<type>(LUMI-XX): <description>`

Types:

- `feat:` — new feature
- `fix:` — bug fix
- `refactor:` — restructuring without behavior change
- `chore:` — build, deps, config
- `docs:` — documentation

Examples:

- `feat(LUMI-20): persistent media_items table with UID`
- `fix: modal closes on text selection release outside`
- `refactor: extract MovieCard and ShowCard`


## Workflow

For every new task:
1. Create a GitHub issue with title and description.
2. Branch: `feature/LUMI-XX` or `fix/...`.
3. Commit with conventional format.
4. Pull request to `master`.

When starting a task, ask the assistant for the issue title and description first.