// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

mod parser;
mod scanner;
mod tmdb;
mod cache;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Загружаем .env при старте приложения
    dotenvy::dotenv().ok();
    cache::init_db().expect("Не удалось инициализировать кэш");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scanner::scan_videos,
            tmdb::get_poster
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
