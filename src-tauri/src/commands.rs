//! Tauri IPC commands invoked from the overlay, settings, and onboarding frontends.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

use crate::hotkey;
use crate::license::{self, LicenseStatus};
use crate::secrets;
use crate::settings::{self, Settings, TrialStatus};
use crate::state::AppState;
use crate::suggester::current_suggester;

#[derive(Serialize)]
pub struct AppInfo {
    pub version: String,
    pub suggestions_today: u32,
}

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
pub async fn validate_api_key(app: AppHandle) -> Result<(), String> {
    let key = secrets::get_api_key()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No API key configured.".to_string())?;
    let s = settings::current(&app);
    let suggester = current_suggester(key, &s);
    suggester.validate().await.map_err(|e| e.to_string())
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

#[tauri::command]
pub fn trial_status(app: AppHandle) -> TrialStatus {
    settings::trial_status(&app)
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    let state = app.state::<Arc<AppState>>();
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        suggestions_today: state.usage.today(),
    }
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    let mgr = app.autolaunch();
    mgr.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mgr = app.autolaunch();
    if enabled {
        mgr.enable().map_err(|e| e.to_string())?;
    } else {
        mgr.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn reset_all(app: AppHandle) -> Result<(), String> {
    settings::reset(&app).map_err(|e| e.to_string())?;
    let _ = secrets::clear_api_key();
    let s = settings::current(&app);
    hotkey::register_all(&app, &s).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn mark_first_run_done(app: AppHandle) {
    settings::mark_first_run_complete(&app);
}

#[tauri::command]
pub async fn check_for_updates(_app: AppHandle) -> Result<String, String> {
    // Stub. When the Tauri updater is wired with a real endpoint, replace this.
    Ok("You're on the latest version (0.1.0).".into())
}
