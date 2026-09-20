mod cache;
mod config;
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
        .invoke_handler(tauri::generate_handler![
            scanner::scan_all,
            tmdb::get_poster,
            player::play_video,
            config::get_folders,
            config::add_folder,
            config::remove_folder,
            scanner::get_continue_watching,
            scanner::set_watched_bulk,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
