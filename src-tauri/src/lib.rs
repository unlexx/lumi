use tauri::Manager;
mod cache;
mod config;
mod logger;
mod mpv;
mod parser;
mod player;
mod scanner;
mod settings;
mod tmdb;
mod tv_parser;

/// Определяет portable mode по наличию файла `portable.flag` рядом с exe.
fn detect_portable() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .map(|dir| dir.join("portable.flag").is_file())
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();

    let portable = detect_portable();
    cache::set_portable(portable);

    let state = cache::AppState::init().expect("Не удалось инициализировать кэш");

    tauri::Builder::default()
        .manage(state)
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
            tmdb::fetch_episode_meta,
            tmdb::get_episode_still,
            settings::get_settings,
            settings::update_settings,
            exit_app,
            is_player_active,
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

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    // 1. best-effort: послать quit через IPC
    if let Err(e) = crate::mpv::quit() {
        crate::log_info!("mpv quit before exit failed (probably already dead): {}", e);
    }

    // 2. дать mpv шанс завершиться самостоятельно
    std::thread::sleep(std::time::Duration::from_millis(200));

    // 3. если mpv ещё жив — прибить его жёстко
    {
        let state = app.state::<cache::AppState>();
        if let Some(mut child) = state.take_mpv_child() {
            match child.try_wait() {
                Ok(Some(_)) => { /* уже завершился сам — ок */ }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                Err(_) => {
                    // не смогли опросить — на всякий случай kill
                    let _ = child.kill();
                }
            }
        }
    }

    app.exit(0);
}

#[tauri::command]
fn is_player_active(state: tauri::State<'_, cache::AppState>) -> bool {
    state.is_player_active()
}
