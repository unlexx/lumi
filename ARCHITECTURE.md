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
| `episode_name`, `episode_overview`, `episode_still_path` | TMDB episode metadata (lazy) |
| `episode_meta_fetched` | 0/1 — whether episode metadata was fetched |
| `trailer_key` | YouTube key выбранного трейлера (NULL если нет) |
| `trailer_fetched` | 0/1 — ходили ли за трейлером в TMDB |
| `scanned_at`, `updated_at` | unix timestamps |

**Schema changes require wiping `cache.db`** — no migration path.

### watch_status (SQLite)

Keyed by `file_path` (not uid). Tracks playback progress.

- `watched` = 1 when `position / duration >= 0.95`
- `position`, `duration` — saved during mpv playback via IPC
- `set_watched_bulk(paths, watched)` — manual toggle

### TMDB integration

- Proxy: configurable via AppSettings.proxy_url (Option<String> в формате socks5h://host:port)
- All requests need VPN/SOCKS5
- `.env` with `TMDB_API_KEY` in working directory
- Posters: cached to disk, served via `convertFileSrc()`
- Manual match: `apply_tmdb_match(tmdb_id, media_type, uids: Vec<String>)` — updates `media_items` for all uids in a batch
- **Episode metadata:** name / overview / still_path fetched lazily on first `DetailView` open (batch of 4 parallel); cached in `media_items`, previews in `cache/posters/episodes/`
- **Trailers:** `fetch_trailer(tmdb_id, media_type, uid)` — два запроса к `/videos` (`ru-RU` + `en-US`), merge с дедупом по key. Приоритет выбора: `ru` → `en` → любой YouTube; внутри локали — `Trailer > Teaser > прочее`, official > не-official. Результат кэшируется в `media_items` (`trailer_key`, `trailer_fetched`). Ленивый фетч при открытии DetailView — по аналогии с episode metadata. При `trailer_key = NULL` и `trailer_fetched = 1` кнопка в UI скрыта.
- **Trailer thumbnails:** `get_trailer_thumbnail(tmdb_id, media_type, uid)` — читает `trailer_key` из БД, качает `img.youtube.com/vi/{key}/hqdefault.jpg` через тот же proxy, что и постеры, кэширует в `posters/trailers/{movie|tv}_{tmdb_id}.jpg`, отдаёт путь для `convertFileSrc`.
- **Open в браузере:** клик по кнопке → `openUrl('https://youtu.be/{key}')` через `tauri-plugin-opener`. Встроенный плеер и yt-dlp — вне скоупа (см. backlog).

### mpv playback

- Launched with `--input-ipc-server=\\.\pipe\mpvsocket --fullscreen --keep-open=no --idle=no`
- Player binary: configurable via AppSettings.player_path, fallback — mpv из PATH
- Rust polls `time-pos` and `duration` every 2s while mpv alive
- On exit: `mark_watched` saved, `watch_status_updated` event emitted
- Resume: `play_video(path, start_position)` → `mpv --start=<sec>`
- Child-процесс хранится в \AppState.mpv_child`, при выходе приложения гарантированно убивается.

### SQLite connection

Singleton \Mutex<Connection>` + `Mutex<AppSettings>` + `AtomicBool player_active` + `Mutex<Option<Child>> mpv_child` в `AppState`, registered via `tauri::Builder::manage()`.
Lock acquired per sync block, **never held across `.await`** (LUMI-23).

## Tauri commands

| Command | Purpose |
| :--- | :--- |
| `scanner::scan_all` | scan all folders, TMDB match new files, return Library |
| `scanner::get_continue_watching` | return in-progress items |
| `scanner::set_watched_bulk` | manual watched toggle |
| `scanner::get_undefined_items` | unmatched files |
| `tmdb::get_poster` | download/cache poster by tmdb_id |
| `tmdb::search_tmdb_manual` | manual search |
| `tmdb::apply_tmdb_match` | apply manual match to list of uids |
| `tmdb::fetch_poster_preview` | base64 poster for match modal |
| `tmdb::fetch_episode_meta` | fetch episode name/overview/still_path (LUMI-18) |
| `tmdb::get_episode_still` | download/cache episode preview (LUMI-18) |
| `tmdb::fetch_trailer` | fetch/pick trailer key, cache in media_items (LUMI-24) |
| `tmdb::get_trailer_thumbnail` | download/cache YouTube thumbnail (LUMI-24) |
| `player::play_video` | launch mpv with optional start |
| `player::toggle_fullscreen` | F11 |
| `config::get_folders` / `add_folder` / `remove_folder` | folder management |

## Backlog

### Next


### Low priority (wishlist)

- Audio track / subtitle selection before playback
- Skip intro via MKV chapters
- Parallel poster loading
- Dynamic context menu positioning
- Big backdrop on details page
- Episode overview with spoiler toggle
- Кнопка „Проверить соединение“ для прокси

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

- **LUMI-14:** (planned) player path, proxy, portable mode settings
- **LUMI-15:** watch progress on all cards, episode counts for TV shows
- **LUMI-16:** context menu ("⋮") on cards: play, resume, mark watched, match
- **LUMI-17:** full-page DetailView replaces movie/show modals
- **LUMI-18:** episode thumbnails + names from TMDB, lazy batch fetch
- **LUMI-19:** manual TMDB matching modal (search + apply)

### Data architecture (LUMI-20 — LUMI-23)

- **LUMI-20:** `media_items` table with UID (xxhash of filename), stores **all** scanned files including unmatched
- **LUMI-21a:** "Undefined" section for unmatched files, manual match from there
- **LUMI-21b:** Vue refactoring into components / composables (ContextMenu, MatchModal, MovieModal, ShowModal, LibraryView, thin App.vue)
- **LUMI-21c:** manual match updates `media_items` via `save_manual_match_to_item`
- **LUMI-21d:** `uid` in `VideoFile`, context menu matching works
- **LUMI-21e:** remove legacy `movies` / `tv_shows` tables
- **LUMI-21f:** `apply_tmdb_match` takes `uids: Vec<String>`; TV show match updates all episodes
- **LUMI-22:** detect season from parent folder name (`Season N`, `Сезон N`, `N сезон`, `SN`)
- **LUMI-23:** singleton SQLite connection via `tauri::State`, lock per sync block
- **LUMI-24:** трейлеры в DetailView. Кнопка «▶ Трейлер» со спиннером → thumbnail.
  Lazy-фетч при открытии DetailView, ru → en → любой YouTube, внутри локали
  Trailer > Teaser. Кэш в `media_items` (`trailer_key`, `trailer_fetched`),
  thumbnail — в `posters/trailers/{movie|tv}_{tmdb_id}.jpg` через прокси.
  Открытие — `tauri-plugin-opener` (`openUrl`).

## Known issues

- `Blade Runner 2049` parsed as `Blade Runner` + year 2049 → wrong TMDB match
- `uid = xxh64(file_name)` — duplicates with the same name in different folders collide; second occurrence silently skipped
- Windows-only пайп mpv (\\.\pipe\mpvsocket`). Для macOS/Linux — `cfg(target_os)`.
- WebView2 + Escape выбивает из fullscreen в Library при открытии ConfirmDialog. В Settings — не выбивает. Отложено.

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