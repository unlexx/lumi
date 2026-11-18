
## Key concepts

### media_items (SQLite)

Every scanned file is stored, **matched or not**. Primary key = `uid` (xxhash of file name).

| Column | Description |
| :--- | :--- |
| `uid` | xxhash64(file_name) |
| `path`, `name` | full path + file name |
| `media_type` | "movie" \| "tv_shows" |
| `tmdb_id` | NULL if not matched |
| `title`, `original_title`, `overview`, `poster_path`, `backdrop_path`, `rating`, `release_date` | TMDB data |
| `parsed_title`, `parsed_year` | from filename parser |
| `season`, `episode` | for TV shows |
| `episode_name`, `episode_overview`, `episode_still_path` | TMDB episode metadata (lazy) |
| `episode_meta_fetched` | 0/1 — whether episode metadata was fetched |
| `trailer_key` | YouTube key выбранного трейлера (NULL если нет) |
| `trailer_fetched` | 0/1 — ходили ли за трейлером в TMDB |
| `missing` | 0/1 — файл отсутствует на диске (LUMI-28) |
| `scanned_at`, `updated_at` | unix timestamps |
| `tmdb_attempted_at` | unix ts последней попытки TMDB-матчинга; NULL = не пытались (LUMI-29) |

**Схема (migrations):**

- **Additive** (новая колонка): `ensure_column_*` в `init_schema` — при старте
  проверяет `pragma_table_info` и делает `ALTER TABLE ADD COLUMN`, если колонки нет.
  `cache.db` **не нужно стирать**, старые записи получают `DEFAULT` / `NULL`.
  Примеры: `ensure_column_missing` (LUMI-28), `ensure_column_tmdb_attempted_at` (LUMI-29).
- **Destructive** (переименование / удаление / смена типа): полноценных миграций нет,
  `cache.db` по-прежнему стирается.

**Инвариант списка колонок (LUMI-29):** все SELECT-ы по `media_items` обязаны
строиться через `MEDIA_COLS` + `row_to_item` (см. ниже). При добавлении поля
в `MediaItem`:

1. добавить в `MEDIA_COLS` (в конец — чтобы не сдвигать индексы существующих),
2. добавить в `row_to_item` (в тот же индекс),
3. `cargo test` — тест `media_cols_matches_constructor` поймает разъезд.

Историю этого инварианта см. в History → LUMI-29 и Architectural debt.

### media_items read helpers (LUMI-29)

Три примитива в `cache.rs`, единственный способ читать `media_items`:

```rust
const MEDIA_COLS: &str = "uid, path, ..., missing, scanned_at, updated_at, tmdb_attempted_at";

fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaItem>;

fn collect_items<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    ctx: &str,          // для лога ошибок
) -> Vec<MediaItem>;
```

- `find_by_uid` → `query_row` + `row_to_item` вручную (нужен `Option`).
- `get_all_media_items`, `get_undefined`, `get_unmatched_for_retry` → `collect_items`.
- `collect_items` логирует `InvalidColumnIndex` / row-level errors вместо
  проглатывания через `filter_map(|r| r.ok())`. Это то, что маскировало три
  подряд бага (LUMI-26a/28/29), пока не вынесли в общий маппер.

### scan_all pipeline

`scanner::scan_all` — одна команда, но по смыслу это шесть этапов с явными
границами lock/unlock (`state.conn()` берётся на каждый этап, ни разу не держится
через `.await` — LUMI-23):

1. **Scan files.** `walkdir` по `folders.json`, `video_extensions` фильтр,
   инкрементальная вставка (`find_by_uid` → skip/update/insert). Собирает
   `valid_folder_paths` (папки, у которых `is_dir()` = true — **не** отмонтированные)
   и `found_uids` (все uid, реально найденные на диске).
2. **Mark missing (LUMI-28).** `mark_missing_not_in(conn, &found_uids, &valid_folder_paths)`:
   для записей `missing = 0`, чей `path` лежит под одной из `valid_folder_paths`,
   но `uid` не в `found_uids` → `missing = 1`. Отмонтированные папки не участвуют,
   поэтому «отключил сетевой диск → scan_all» ничего не помечает.
3. **TMDB for new movies.** `new_uids ∩ media_type == "movie"` → `get_or_fetch` →
   `update_tmdb` (пишет `tmdb_attempted_at = now` при успехе).
4. **TMDB for new TV.** Группировка `new_uids ∩ media_type == "tv_shows"` по
   `(parsed_title, parsed_year)` → `get_or_fetch_tv` → `update_tmdb` для каждого эпизода.
5. **TMDB retry for unmatched (LUMI-29).** `get_unmatched_for_retry(conn, backoff, 50)`:
   `tmdb_id IS NULL AND missing = 0 AND (tmdb_attempted_at IS NULL OR
   tmdb_attempted_at < now - backoff)`. Movies — поштучно, TV — группировка как в п.4.
   - `Ok(Some(...))` → `update_tmdb` (пишет `tmdb_attempted_at = now`).
   - `Ok(None)` → `mark_tmdb_attempted` (пишет `tmdb_attempted_at = now`,
     следующий retry не раньше чем через backoff).
   - `Err(e)` (сеть) → **не** пишем `tmdb_attempted_at`, повторим на следующем скане.
   - Backoff по умолчанию 24ч, override через `LUMI_TMDB_RETRY_BACKOFF_SECS` (для тестов).
   - Лимит 50 записей за прогон (TMDB rate-limit, чтоб не долбить по 500 мусорных).
6. **Build library.** `get_all_media_items` (уже `WHERE missing = 0`) → `build_library`.

Логи этапов (по ним удобно проверять чек-листы):

```
=== New files: N ===
[missing] valid folders = X, found uids = Y
=== Marked M items as missing ===
=== TMDB lookup for A new movies ===
=== TMDB lookup for B new TV shows ===
=== TMDB retry for C unmatched items ===
  [retry] → Matched: ... / No match for ... / TMDB error for ...
```

### watch_status (SQLite)

Keyed by `file_path` (not uid). Tracks playback progress.

- `watched` = 1 when `position / duration >= 0.95`
- `position`, `duration` — saved during mpv playback via IPC
- `set_watched_bulk(paths, watched)` — manual toggle
- **Не затрагивается** LUMI-28: soft delete помечает `media_items.missing = 1`,
  но `watch_status` цел — при возврате файла прогресс сохраняется.

### TMDB integration

- Proxy: configurable via AppSettings.proxy_url (Option<String> в формате socks5h://host:port)
- All requests need VPN/SOCKS5
- `.env` with `TMDB_API_KEY` in working directory
- Posters: cached to disk, served via `convertFileSrc()`
- Manual match: `apply_tmdb_match(tmdb_id, media_type, uids: Vec<String>)` — updates `media_items` for all uids in a batch
- **Episode metadata:** name / overview / still_path fetched lazily on first `DetailView` open (batch of 4 parallel); cached in `media_items`, previews in `cache/posters/episodes/`
- **Trailers:** `fetch_trailer(tmdb_id, media_type, uid)` — два запроса к `/videos` (`ru-RU` + `en-US`), merge с дедупом по key. Приоритет выбора: `ru` → `en` → любой YouTube; внутри локали — `Trailer > Teaser > прочее`, official > не-official. Результат кэшируется в `media_items` (`trailer_key`, `trailer_fetched`). Ленивый фетч при открытии DetailView — по аналогии с episode metadata. При `trailer_key = NULL` и `trailer_fetched = 1` кнопка в UI скрыта.
- **Trailer thumbnails:** `get_trailer_thumbnail(tmdb_id, media_type, uid)` — читает `trailer_key` из БД, качает `img.youtube.com/vi/{key}/hqdefault.jpg` через тот же proxy, что и постеры, кэширует в `posters/trailers/{movie|tv}_{tmdb_id}.jpg`, отдаёт путь для `convertFileSrc`.
- **Open в браузере:** клик по кнопке → `openUrl('https://youtube.com/{key}')` через `tauri-plugin-opener`. Встроенный плеер и yt-dlp — вне скоупа (см. backlog).
- **Backdrops:** `get_backdrop(tmdb_id, backdrop_path)` — качает `w1280` через тот же proxy, кэширует в `posters/backdrops/{tmdb_id}.jpg`, отдаёт путь для `convertFileSrc`. Шоу-левел: у сериалов одна подложка на тайтл. `backdrop_path` пишется в `media_items` при авто- и ручном матче. Настройка качества и blur-параметры — LUMI-26b.
- **Retry unmatched (LUMI-29):** см. `scan_all pipeline → этап 5`. TMDB-этапы
  (3, 4) обновляют `tmdb_attempted_at = now` при успехе через `update_tmdb`/
  `update_tmdb_for_uids`; `Ok(None)` — через `mark_tmdb_attempted`;
  сетевые ошибки не трогают колонку.

### mpv playback

- Launched with `--input-ipc-server=\\.\pipe\mpvsocket --fullscreen --keep-open=no --idle=no`
- Player binary: configurable via AppSettings.player_path, fallback — mpv из PATH
- Rust polls `time-pos` and `duration` every 2s while mpv alive
- On exit: `mark_watched` saved, `watch_status_updated` event emitted
- Resume: `play_video(path, start_position)` → `mpv --start=<sec>`
- Child-процесс хранится в `AppState.mpv_child`, при выходе приложения гарантированно убивается.

### SQLite connection

Singleton `Mutex<Connection>` + `Mutex<AppSettings>` + `AtomicBool player_active` + `Mutex<Option<Child>> mpv_child` в `AppState`, registered via `tauri::Builder::manage()`.
Lock acquired per sync block, **never held across `.await`** (LUMI-23).

## Tauri commands

| Command | Purpose |
| :--- | :--- |
| `scanner::scan_all` | scan folders → mark missing → TMDB match new → TMDB retry unmatched → return Library (см. scan_all pipeline) |
| `scanner::get_continue_watching` | return in-progress items |
| `scanner::set_watched_bulk` | manual watched toggle |
| `scanner::get_undefined_items` | unmatched files (`tmdb_id IS NULL AND missing = 0`) |
| `tmdb::get_poster` | download/cache poster by tmdb_id |
| `tmdb::search_tmdb_manual` | manual search |
| `tmdb::apply_tmdb_match` | apply manual match to list of uids |
| `tmdb::fetch_poster_preview` | base64 poster for match modal |
| `tmdb::fetch_episode_meta` | fetch episode name/overview/still_path (LUMI-18) |
| `tmdb::get_episode_still` | download/cache episode preview (LUMI-18) |
| `tmdb::fetch_trailer` | fetch/pick trailer key, cache in media_items (LUMI-24) |
| `tmdb::get_trailer_thumbnail` | download/cache YouTube thumbnail (LUMI-24) |
| `tmdb::get_backdrop` | download/cache backdrop by tmdb_id (LUMI-26a) |
| `tmdb::get_youtube_search_url` | build youtube search URL by uid |
| `player::play_video` | launch mpv with optional start |
| `player::toggle_fullscreen` | F11 |
| `config::get_folders` / `add_folder` / `remove_folder` | folder management |

## Backlog

LUMI-27: галерея backdrop'ов в DetailView. Показывать список backdrop'ов из TMDB /images, давать листать ←/→ или свайпом, лениво грузить выбранный, запоминать выбор в media_backdrops.selected. Кэш — posters/backdrops/{tmdb_id}/{hash}.jpg.

### Next

### Low priority (wishlist)

- Audio track / subtitle selection before playback
- Parallel poster loading
- Dynamic context menu positioning
- Episode overview with spoiler toggle
- Кнопка „Проверить соединение“ для прокси

### Architectural debt

- Нормализация `media_items`: вынести TMDB-данные в отдельную таблицу `tmdb_entries` (один ряд на тайтл), оставить в `media_items` только файловые атрибуты и `tmdb_id` как FK. Сейчас одни и те же `title`/`overview`/`poster_path`/`backdrop_path`/`trailer_key` дублируются во всех эпизодах сериала.
- ~~`media_items` column list drift (LUMI-26a/28/29)~~ — закрыто в LUMI-29,
  см. History.

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
- **LUMI-25:** adaptive refresh rate for mpv playback. `display.rs`
  (Win32 GDI: EnumDisplaySettingsW/ChangeDisplaySettingsExW + CDS_FULLSCREEN),
  `DisplayGuard` — RAII-возврат режима при drop + `restore_saved_mode()`
  из `exit_app` на случай `app.exit`/kill. FPS через `ffprobe` (`r_frame_rate`),
  23.976→23, 29.97→30, 59.94→60. `find_ffprobe()` ищет бинарь в PATH,
  рядом с mpv и в стандартных путях. Ошибки WinAPI/ffprobe логируются —
  воспроизведение продолжается на текущей частоте.
- **LUMI-25b:** кнопка «Найти на YouTube» в DetailView. Команда
  `tmdb::get_youtube_search_url(uid)` строит запрос: локализованное `title`
  + " трейлер" (приоритет ru) → `original_title` + " trailer" → имя файла.
  Домен `www.youtube.com/results?search_query=…`, не `youtu.be`.
  7 юнит-тестов на `build_youtube_query`.

### Library management (LUMI-14 — LUMI-19)

- **LUMI-14a:** `AppSettings` (`settings.json`), portable mode через `portable.flag`
  → `<exe_dir>/lumi-data/`. Команды `get_settings` / `update_settings` /
  `exit_app` / `is_player_active`. `ConfirmDialog.vue` + модалка выхода
  по Escape. RAII-сессии для `player_active` и `mpv_child`.
- **LUMI-14b:** configurable player path. `mpv::launch` принимает `player_path`,
  читает из `AppState`. `None` → `mpv` из PATH.
- **LUMI-14c:** configurable SOCKS5 proxy. `proxy_url: Option<String>`
  (`socks5h://host:port`), UI — input `host:port`. `build_client(Option<&str>)`,
  `None` → без прокси. Обновление без перезапуска.
- **LUMI-15:** watch progress on all cards, episode counts for TV shows
- **LUMI-16:** context menu ("⋮") on cards: play, resume, mark watched, match
- **LUMI-17:** full-page DetailView replaces movie/show modals
- **LUMI-18:** episode thumbnails + names from TMDB, lazy batch fetch
- **LUMI-19:** manual TMDB matching modal (search + apply)

### Data architecture (LUMI-20 — LUMI-29)

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
- **LUMI-26a:** backdrop на DetailView. Колонка `backdrop_path` в `media_items`,
  качается в `w1280` через прокси, кэш в `posters/backdrops/{tmdb_id}.jpg`,
  отдаётся через `convertFileSrc`. Используется как размытый фон hero
  с fallback на постер. Настройка качества и тюнинг blur/градиента — LUMI-26b.
- **LUMI-28:** soft delete отсутствующих файлов при сканировании.
  Колонка `missing INTEGER NOT NULL DEFAULT 0` + `ensure_column_missing`
  (additive migration). `scan_all` собирает `valid_folder_paths` (только
  `is_dir()` = true — защита от отмонтированных сетей) и `found_uids`
  (все uid со всех валидных папок). Этап 1.5: `mark_missing_not_in(conn,
  &found_uids, &valid_folder_paths)` ставит `missing = 1` записям, чей `path`
  лежит под `valid_folder_paths`, но `uid` не найден. `find_by_uid` намеренно
  **не** фильтрует `missing` (нужен для `scan_all`, чтобы снимать флаг при
  возврате файла через `update_path` / `clear_missing`). Все UI-запросы
  (`get_all_media_items`, `get_undefined`, `get_youtube_search_url`) фильтруют
  `missing = 0`. `watch_status` не затрагивается. Юнит-тесты на in-memory БД.
- **LUMI-29:** retry TMDB lookup для unmatched при сканировании.
  Колонка `tmdb_attempted_at INTEGER` (NULL = не пытались) +
  `ensure_column_tmdb_attempted_at` (additive). `update_tmdb` /
  `update_tmdb_for_uids` пишут `tmdb_attempted_at = now` при успехе.
  Новая `mark_tmdb_attempted` — при `Ok(None)` (TMDB ответил «нет совпадений»);
  сетевые ошибки колонку не трогают (retry повторится на следующем скане).
  Новая `get_unmatched_for_retry(conn, backoff, limit)` — `tmdb_id IS NULL
  AND missing = 0 AND (tmdb_attempted_at IS NULL OR tmdb_attempted_at <
  now - backoff) ORDER BY scanned_at ASC LIMIT N`. В `scan_all` — этап 3.5
  (movies поштучно, TV группировка). Backoff по умолчанию 24ч, override
  `LUMI_TMDB_RETRY_BACKOFF_SECS` для тестов; лимит 50/прогон.
  **Рефакторинг `MEDIA_COLS`** (часть LUMI-29): единый `const MEDIA_COLS`
  + `fn row_to_item(&Row)` + `fn collect_items(conn, sql, params, ctx)`.
  `find_by_uid` / `get_all_media_items` / `get_undefined` / `get_unmatched_for_retry`
  перестали содержать собственные SELECT-списки и конструкторы.
  Guard-тест `media_cols_matches_constructor` прогоняет все четыре функции
  на in-memory БД. Закрывает класс багов LUMI-26a/28/29 (SELECT-список
  разъезжался с конструктором `MediaItem`, ошибка маскировалась
  `.optional().unwrap_or(None)` / `filter_map(|r| r.ok())`).

## Known issues
- LUMI-25: переключение частоты работает только на **primary** дисплее
  (`PCWSTR::null()`). Если проектор — вторичный экран (extend), частота
  переключится на ноутбуке. Фикс — перебирать `EnumDisplayDevicesW` и
  применять режим на нужный `\\.\DISPLAYn` (см. обсуждение в LUMI-25).
- `ffprobe` — внешний бинарник. В `find_ffprobe()` есть fallback на папку
  mpv и стандартные пути, но на чистой машине его всё равно нужно
  положить руками. Долгосрочно — mpv IPC `container-fps` (без внешних
  зависимостей).
- `Blade Runner 2049` parsed as `Blade Runner` + year 2049 → wrong TMDB match
- `uid = xxh64(file_name)` — duplicates with the same name in different folders collide; second occurrence silently skipped
- `WalkDir::filter_map(|e| e.ok())` молча пропускает ошибки обхода (permission
  denied, сбой сети во время обхода подкаталога). Если папка доступна, но
  часть подкаталогов — нет, файлы из них попадут в `missing = 1` на этапе 1.5
  (LUMI-28). Отдельная задача — собирать ошибки `WalkDir` и не помечать
  содержимое «проблемных» подкаталогов как missing.
- Windows-only пайп mpv (`\\.\pipe\mpvsocket`). Для macOS/Linux — `cfg(target_os)`.
- WebView2 + Escape выбивает из fullscreen в Library при открытии ConfirmDialog. В Settings — не выбивает. Отложено.
- Для сериалов `trailer_key` хранится только в `media_items` первого эпизода
  первого сезона. Шоу-левел трейлер лежит в одной записи; при желании —
  расширить до батча по всем эпизодам, как `apply_tmdb_match` для tv.

## Conventions

- **Rust:** modules in `src-tauri/src/`, log via `crate::log_info!`
- **Vue:** `<script setup lang="ts">`, types from `@/types`
- **CSS:** global in `App.vue` for layout, `scoped` in components
- **Commits:** `feat:`, `fix:`, `refactor:` prefixes
- **Branches:** `feature/LUMI-XX`, `fix/...`
- **`media_items` SELECT:** только через `MEDIA_COLS` + `row_to_item` +
  `collect_items`. Никаких локальных списков колонок и конструкторов в
  отдельных функциях (см. Key concepts → media_items read helpers).

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