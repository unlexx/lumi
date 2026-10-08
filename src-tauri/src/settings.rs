use crate::cache;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub player_path: Option<String>,
    #[serde(default)]
    pub proxy_url: Option<String>,
}

fn settings_path() -> PathBuf {
    cache::config_root().join("settings.json")
}

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    if !path.exists() {
        return AppSettings::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;

    // Атомарная запись: temp + rename, чтобы не поймать битый JSON при падении.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Валидирует player_path, если он задан.
/// None / пустая строка → Ok. Некорректный путь → Err.
fn validate_player_path(p: &Option<String>) -> Result<(), String> {
    let Some(path) = p.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let pb = std::path::Path::new(path);
    if !pb.is_file() {
        return Err(format!("Файл плеера не найден: {path}"));
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, cache::AppState>) -> AppSettings {
    state.settings().clone()
}

#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, cache::AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    validate_player_path(&settings.player_path)?;

    // Нормализуем: пустые строки → None.
    let normalized = AppSettings {
        player_path: settings
            .player_path
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        proxy_url: settings
            .proxy_url
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    };

    save_settings(&normalized)?;

    {
        let mut guard = state.settings();
        *guard = normalized.clone();
    }

    Ok(normalized)
}
