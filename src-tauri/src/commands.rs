//! Tauri IPC commands invoked from the overlay and settings frontends.

use tauri::{AppHandle, Manager};

use crate::hotkey;
use crate::license::{self, LicenseStatus};
use crate::secrets;
use crate::settings::{self, Settings};

#[tauri::command]
pub async fn save_api_key(key: String) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key is empty.".into());
    }
    if !trimmed.starts_with("sk-ant-") {
        return Err("Anthropic API keys start with `sk-ant-`. Double-check the value you pasted.".into());
    }
    secrets::set_api_key(trimmed).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_api_key_status() -> bool {
    secrets::has_api_key()
}

#[tauri::command]
pub fn clear_api_key() -> Result<(), String> {
    secrets::clear_api_key().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> Settings {
    settings::current(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    settings::save(&app, &settings).map_err(|e| e.to_string())?;
    hotkey::register_all(&app, &settings).map_err(|e| format!("Hotkey rebind failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn accept_suggestion(app: AppHandle, index: usize) -> Result<(), String> {
    hotkey::accept(app, index).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn regenerate_suggestions(app: AppHandle) -> Result<(), String> {
    hotkey::regenerate(app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_overlay(app: AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    crate::open_settings(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn license_status() -> LicenseStatus {
    license::status()
}

#[tauri::command]
pub fn activate_license(key: String) -> LicenseStatus {
    license::activate(&key)
}
