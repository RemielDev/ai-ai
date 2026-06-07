//! Tauri IPC commands.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

use crate::hotkey;
use crate::license::{self, LicenseStatus};
use crate::secrets;
use crate::settings::{self, Provider, Settings, TrialStatus};
use crate::state::AppState;
use crate::suggester::current_suggester;

#[derive(Serialize)]
pub struct AppInfo {
    pub version: String,
    pub suggestions_today: u32,
}

#[derive(Serialize)]
pub struct ProviderKeyStatus {
    pub anthropic: bool,
    pub openai: bool,
    pub openrouter: bool,
    pub gemini: bool,
}

fn validate_key_format(provider: Provider, key: &str) -> Result<(), String> {
    let hint = provider.key_prefix_hint();
    if !key.starts_with(hint) {
        return Err(format!(
            "{} keys typically start with `{}`. Double-check the value you pasted.",
            provider.label(),
            hint
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn save_api_key(provider: Provider, key: String) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key is empty.".into());
    }
    validate_key_format(provider, trimmed)?;
    secrets::set_api_key(provider, trimmed).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_api_key_status(provider: Provider) -> bool {
    secrets::has_api_key(provider)
}

#[tauri::command]
pub fn get_all_key_status() -> ProviderKeyStatus {
    ProviderKeyStatus {
        anthropic: secrets::has_api_key(Provider::Anthropic),
        openai: secrets::has_api_key(Provider::OpenAI),
        openrouter: secrets::has_api_key(Provider::OpenRouter),
        gemini: secrets::has_api_key(Provider::Gemini),
    }
}

#[tauri::command]
pub fn clear_api_key(provider: Provider) -> Result<(), String> {
    secrets::clear_api_key(provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_api_key(app: AppHandle) -> Result<(), String> {
    let s = settings::current(&app);
    let key = secrets::get_api_key(s.provider)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No API key configured for {}.", s.provider.label()))?;
    let suggester = current_suggester(s.provider, key, &s);
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
    secrets::clear_all();
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
    Ok("You're on the latest version (0.1.0).".into())
}
