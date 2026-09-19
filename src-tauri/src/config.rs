use crate::cache;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Movie,
    TvShows,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderConfig {
    pub path: String,
    #[serde(rename = "type")]
    pub media_type: MediaType,
}

fn config_path() -> PathBuf {
    cache::project_dirs().config_dir().join("folders.json")
}

pub fn load_folders() -> Vec<FolderConfig> {
    let path = config_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save_folders(folders: &[FolderConfig]) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(folders).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_folders() -> Vec<FolderConfig> {
    load_folders()
}

#[tauri::command]
pub fn add_folder(path: String, media_type: MediaType) -> Result<Vec<FolderConfig>, String> {
    let mut folders = load_folders();
    
    // Не добавляем дубликат
    if folders.iter().any(|f| f.path == path) {
        return Err("Эта папка уже добавлена".to_string());
    }
    
    folders.push(FolderConfig { path, media_type });
    save_folders(&folders)?;
    Ok(folders)
}

#[tauri::command]
pub fn remove_folder(path: String) -> Result<Vec<FolderConfig>, String> {
    let mut folders = load_folders();
    folders.retain(|f| f.path != path);
    save_folders(&folders)?;
    Ok(folders)
}