# Lumi — handoff

Стек: Tauri 2 + Vue 3 + TS + Rust + SQLite.
Repo: https://github.com/unlexx/lumi
Docs: ARCHITECTURE.md (актуален)

## Последнее закрытое (свежее сверху)

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

**LUMI-14: configurable player path, proxy, portable mode.**
Разбито на три подзадачи (можно слить в один PR или в три отдельных):

- **14a:** `AppSettings` (`settings.json`), portable mode (переключение
  `db_path` / `posters_dir` / `config_path` через статический флаг),
  `get_settings` / `update_settings`. Фундамент.
- **14b:** configurable player path. `mpv::launch` принимает `player_path`,
  UI в `Settings.vue` с file picker.
- **14c:** configurable proxy URL. `tmdb::build_client` принимает `proxy_url`,
  все TMDB-команды читают из `State<AppState>`.

**Начинаем с 14a.** Файлы для старта:
`config.rs`, `cache.rs`, `lib.rs`, `Settings.vue`, `package.json`.

Открытый вопрос при старте 14a — нужен ли `Mutex<AppSettings>` в `AppState`
или достаточно `OnceLock`, и как обновлять `proxy_url` без перезапуска.

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
- Миграции схемы нет — при изменениях `media_items` нужно стирать `cache.db`.