
# Lumi — handoff

Стек: Tauri 2 + Vue 3 + TS + Rust + SQLite.
Repo: https://github.com/unlexx/lumi
Docs: ARCHITECTURE.md (актуален)

## Последнее закрытое (свежее сверху)

- **feat(LUMI-29):** retry TMDB lookup для unmatched при сканировании.
  Колонка `tmdb_attempted_at` (additive через `ensure_column_tmdb_attempted_at`).
  `update_tmdb`/`update_tmdb_for_uids` пишут `tmdb_attempted_at = now` при
  успехе; `mark_tmdb_attempted` — при `Ok(None)`; сетевые ошибки колонку не
  трогают. `get_unmatched_for_retry(conn, backoff, limit)` →
  `tmdb_id IS NULL AND missing = 0 AND (attempted_at IS NULL OR < now - backoff)`.
  В `scan_all` — этап 3.5 (movies поштучно, TV группировка, `LIMIT 50`).
  Backoff 24ч, override `LUMI_TMDB_RETRY_BACKOFF_SECS` для тестов.
  **Рефакторинг `MEDIA_COLS`** (в этом же коммите): единый `const MEDIA_COLS`
  + `fn row_to_item` + `fn collect_items(conn, sql, params, ctx)`. Все
  SELECT-ы по `media_items` теперь используют их; guard-тест
  `media_cols_matches_constructor` ловит разъезд списка колонок с `MediaItem`.
  Закрывает класс багов LUMI-26a/28/29.
- **feat(LUMI-28):** soft delete missing files on scan. Колонка
  `missing INTEGER NOT NULL DEFAULT 0` + `ensure_column_missing`.
  `scan_all` собирает `valid_folder_paths` (только `is_dir()` = true) и
  `found_uids`; `mark_missing_not_in` ставит `missing = 1` записям под
  валидными папками, но не найденным на диске. Отмонтированный сетевой
  диск → ничего не помечается. `find_by_uid` намеренно не фильтрует
  `missing`; `get_all_media_items`, `get_undefined`, `get_youtube_search_url`
  — фильтруют `missing = 0`. `watch_status` не затрагивается.
- **feat(LUMI-26a):** backdrop на DetailView. Колонка `backdrop_path` в
  `media_items`, пишется при авто- и ручном матче. `get_backdrop` качает
  `w1280` через прокси, кэш в `posters/backdrops/{tmdb_id}.jpg`,
  отдаётся через `convertFileSrc`. Размытый фон hero с fallback на постер.
  Качество/blur — LUMI-26b.
- **feat(LUMI-25b):** youtube search button in DetailView. Команда
  `tmdb::get_youtube_search_url(uid)` — локализованное `title` + " трейлер"
  (ru-приоритет) → `original_title` + " trailer" → имя файла.
  Домен `www.youtube.com/results?search_query=…`, не `youtu.be`.
  7 юнит-тестов на `build_youtube_query`.
- **feat(LUMI-25):** adaptive refresh rate for mpv playback. `display.rs`
  (Win32 GDI + `CDS_FULLSCREEN`), `DisplayGuard` — RAII-возврат режима
  при drop + `restore_saved_mode()` из `exit_app`. FPS через `ffprobe`
  (`r_frame_rate`). Ошибки WinAPI/ffprobe логируются — воспроизведение
  продолжается на текущей частоте. Проектор как secondary — Known issue.
- **feat(LUMI-24):** трейлеры в DetailView. Lazy-фетч при открытии,
  два запроса к `/videos` (`ru-RU` + `en-US`), merge, приоритет
  `ru → en → любой YouTube`, внутри локали `Trailer > Teaser`,
  official > не-official. Кэш в `media_items` (`trailer_key`,
  `trailer_fetched`), thumbnail — `posters/trailers/{movie|tv}_{tmdb_id}.jpg`.
  Открытие — `tauri-plugin-opener` (`openUrl`).

## Текущая задача



## Соглашения

- title + description для GitHub issue (на русском), потом код.
- Коммиты: `feat(LUMI-XX): ...`, `fix: ...`, `refactor: ...`.
- Критерии приёмки — на русском.
- Перед крупной задачей — скидываю актуальные файлы, не полагаюсь на память чата.
- **`media_items` SELECT** — только через `MEDIA_COLS` + `row_to_item` +
  `collect_items`. Никаких локальных списков колонок и конструкторов
  в отдельных функциях. При добавлении поля в `MediaItem` — `cargo test`
  (`media_cols_matches_constructor`).

## Известные ограничения

- `Blade Runner 2049` парсится как `Blade Runner` + 2049 → неверный TMDB-матч.
- `uid = xxh64(file_name)` — дубли файлов с одинаковым именем в разных папках
  коллизят, второй молча пропускается.
- `WalkDir::filter_map(|e| e.ok())` молча пропускает ошибки обхода
  (permission denied, сбой сети при обходе подкаталога). В комбинации с
  LUMI-28 это может давать ложные `missing = 1` для файлов в «проблемных»
  подкаталогах. Отдельная задача.
- Миграции: additive (новая колонка) — автоматически через `ensure_column_*`
  в `init_schema`, `cache.db` стирать **не нужно**. Destructive (rename /
  drop / смена типа) — полноценных миграций нет, `cache.db` стирается
  (только для предрелизных версий).
- Трейлеры открываются во внешнем браузере, не внутри приложения.
  Встроенный плеер (iframe) не поддерживается на Tauri без ломания CSP
  и autoplay; mpv + yt-dlp — в wishlist.
- Для сериалов `trailer_key` хранится только в `media_items` первого
  эпизода первого сезона (шоу-левел трейлер в одной записи). При желании —
  расширить до батча по всем эпизодам, как `apply_tmdb_match` для tv.
- Backdrop грузится в фиксированном качестве `w1280`. Настройка
  `hi|medium|low` и тюнинг blur/градиента — LUMI-26b.
- TMDB-данные (title, overview, poster_path, backdrop_path, trailer_key)
  дублируются во всех эпизодах сериала. Архитектурный долг — нормализация
  в отдельную таблицу `tmdb_entries` (см. ARCHITECTURE.md → Architectural debt).