use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn play_video(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| format!("Не удалось открыть файл: {}", e))
}