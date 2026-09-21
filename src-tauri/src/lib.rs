use tauri::Manager;
mod cache;
mod config;
mod logger;
mod mpv;
mod parser;
mod player;
mod scanner;
mod tmdb;
mod tv_parser;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Загружаем .env при старте приложения
    dotenvy::dotenv().ok();
    cache::init_db().expect("Не удалось инициализировать кэш");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_fullscreen(true).ok();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            player::play_video,
            config::get_folders,
            config::add_folder,
            config::remove_folder,
            scanner::get_continue_watching,
            scanner::set_watched_bulk,
            scanner::scan_all,
            scanner::get_undefined_items,
            tmdb::get_poster,
            tmdb::search_tmdb_manual,
            tmdb::apply_tmdb_match,
            tmdb::fetch_poster_preview,
            toggle_fullscreen,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn toggle_fullscreen(window: tauri::Window) -> Result<(), String> {
    let is_fullscreen = window.is_fullscreen().map_err(|e| e.to_string())?;
    window
        .set_fullscreen(!is_fullscreen)
        .map_err(|e| e.to_string())
}
