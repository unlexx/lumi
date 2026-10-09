# Lumi — handoff

Стек: Tauri 2 + Vue 3 + TS + Rust + SQLite.
Repo: https://github.com/unlexx/lumi
Docs: ARCHITECTURE.md (актуален)

## Последнее закрытое (свежее сверху)

- **LUMI-24:** трейлеры в DetailView. Кнопка «▶ Трейлер» со спиннером → thumbnail.
  Lazy-фетч при открытии DetailView: два запроса к TMDB `/videos`
  (`ru-RU` + `en-US`), merge, приоритет ru → en → любой YouTube,
  Trailer > Teaser, official > не-official. Кэш в `media_items`
  (`trailer_key`, `trailer_fetched`), thumbnail —
  `posters/trailers/{movie|tv}_{tmdb_id}.jpg` через прокси.
  Открытие — `tauri-plugin-opener` (`openUrl` на `https://youtu.be/{key}`).
- **LUMI-14a:** `AppSettings` (`settings.json`), portable mode через `portable.flag` → `<exe_dir>/lumi-data/`. Команды `get_settings` / `update_settings` / `exit_app` / `is_player_active`. `ConfirmDialog.vue` + модалка выхода по Escape. RAII-сессии для `player_active` и `mpv_child`, mpv не остаётся висеть при выходе.
- **LUMI-14b:** configurable player path. `mpv::launch` принимает `player_path`, читает из `AppState`. `None` → `mpv` из PATH.
- **LUMI-14c:** configurable SOCKS5 proxy. `proxy_url: Option<String>` (`socks5h://host:port`), UI — input `host:port`. `build_client(Option<&str>)`, `None` → без прокси. Обновление без перезапуска.
- **LUMI-18:** эпизоды в DetailView теперь с превью-кадром (still) и названием из TMDB.
  Lazy-фетч батчами по 4 при открытии DetailView. Кэш в `media_items` + `posters/episodes/`.
- **LUMI-17:** DetailView (полноэкранная страница) заменил `MovieModal` / `ShowModal`.
  `App.vue` — тонкая оболочка: `library` | `settings` | `detail`.
- **LUMI-23:** singleton SQLite через `AppState { Mutex<Connection> }`.
  Lock берётся только в синхронных блоках, никогда не держится через `.await`.
- **LUMI-22:** сезон из имени родительской папки (`Season N`, `Сезон N`, `N сезон`, `SN`).
  Файл выигрывает, папка — только fallback для сезона.
- **LUMI-21f:** `apply_tmdb_match` принимает `uids: Vec<String>`,
  матч сериала обновляет все эпизоды. Закрыт fallback на legacy `save_manual_match`.
- **LUMI-21b:** Vue-рефакторинг завершён — `ContextMenu`, `MatchModal`,
  `MovieModal`, `ShowModal`, `LibraryView` вынесены, `App.vue` тонкий.

## Текущая задача



## Соглашения

- title + description для issue (на русском), потом код.
- Коммиты: `feat(LUMI-XX): ...`, `fix: ...`, `refactor: ...`.
- Критерии приёмки — на русском.
- Перед крупной задачей — скидываю актуальные файлы, не полагаюсь на память чата.

## Известные ограничения

- `Blade Runner 2049` парсится как `Blade Runner` + 2049 → неверный TMDB-матч.
- `uid = xxh64(file_name)` — дубли файлов с одинаковым именем в разных папках
  коллизят, второй молча пропускается.
- Proxy URL, путь к mpv и data dir захардкожены (**LUMI-14 их чинит**).
- Миграции схемы нет — при изменениях `media_items` нужно стирать `cache.db` (только для предрелизных версий).
- Трейлеры открываются во внешнем браузере, не внутри приложения.
  Встроенный плеер (iframe) не поддерживается на Tauri без ломания CSP и autoplay;
  mpv + yt-dlp — в wishlist, может ломаться при обновлениях YouTube.
- Для сериалов `trailer_key` хранится только в `media_items` первого эпизода
  первого сезона (трейлер шоу-левел лежит в одной записи). При желании —
  расширить до батча по всем эпизодам шоу, как `apply_tmdb_match` для tv.