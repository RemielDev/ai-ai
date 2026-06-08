//! Persistent settings via tauri-plugin-store.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "settings.json";
#[allow(dead_code)]
pub const TRIAL_DAYS: i64 = 7;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_provider")]
    pub provider: Provider,
    pub model: String,
    pub summon_hotkey: String,
    pub action_hotkey: String,
    pub suggestion_count: u8,
    pub style_preset: StylePreset,
    pub auto_send_after_paste: bool,
    pub telemetry_opt_in: bool,
    pub first_run: bool,
    #[serde(default = "Utc::now")]
    pub first_launch_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Anthropic,
    OpenAI,
    OpenRouter,
    Gemini,
}

fn default_provider() -> Provider {
    Provider::Gemini
}

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Provider::Anthropic => "Anthropic",
            Provider::OpenAI => "OpenAI",
            Provider::OpenRouter => "OpenRouter",
            Provider::Gemini => "Google Gemini",
        }
    }
    pub fn slug(self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::OpenAI => "openai",
            Provider::OpenRouter => "openrouter",
            Provider::Gemini => "gemini",
        }
    }
    pub fn key_prefix_hint(self) -> &'static str {
        match self {
            Provider::Anthropic => "sk-ant-",
            Provider::OpenAI => "sk-",
            Provider::OpenRouter => "sk-or-",
            Provider::Gemini => "AIza",
        }
    }
    #[allow(dead_code)]
    pub fn console_url(self) -> &'static str {
        match self {
            Provider::Anthropic => "https://console.anthropic.com/settings/keys",
            Provider::OpenAI => "https://platform.openai.com/api-keys",
            Provider::OpenRouter => "https://openrouter.ai/keys",
            Provider::Gemini => "https://aistudio.google.com/apikey",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StylePreset {
    Default,
    Concise,
    Exploratory,
    Technical,
}

impl StylePreset {
    pub fn directive(&self) -> &'static str {
        match self {
            StylePreset::Default => "",
            StylePreset::Concise => "Keep each suggestion short and punchy, under 12 words.",
            StylePreset::Exploratory => "Favor open-ended questions and lateral angles over direct follow-ups.",
            StylePreset::Technical => "Prefer technical depth, precise terminology, and questions about implementation, edge cases, and trade-offs.",
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: Provider::Gemini,
            model: "gemini-2.0-flash".into(),
            summon_hotkey: "Ctrl+Shift+Space".into(),
            action_hotkey: "Ctrl+Shift+Enter".into(),
            suggestion_count: 4,
            style_preset: StylePreset::Default,
            auto_send_after_paste: false,
            telemetry_opt_in: false,
            first_run: true,
            first_launch_at: Utc::now(),
        }
    }
}

pub fn ensure_loaded(app: &AppHandle) -> Result<()> {
    let store = app.store(STORE_FILE)?;
    if store.get("settings").is_none() {
        let default = Settings::default();
        store.set("settings", serde_json::to_value(&default)?);
        store.save()?;
    }
    Ok(())
}

pub fn current(app: &AppHandle) -> Settings {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return Settings::default(),
    };
    match store.get("settings") {
        Some(v) => serde_json::from_value(v.clone()).unwrap_or_default(),
        None => Settings::default(),
    }
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<()> {
    let store = app.store(STORE_FILE)?;
    store.set("settings", serde_json::to_value(settings)?);
    store.save()?;
    Ok(())
}

pub fn mark_first_run_complete(app: &AppHandle) {
    let mut s = current(app);
    if !s.first_run {
        return;
    }
    s.first_run = false;
    let _ = save(app, &s);
}

pub fn reset(app: &AppHandle) -> Result<()> {
    let store = app.store(STORE_FILE)?;
    let fresh = Settings::default();
    store.set("settings", serde_json::to_value(&fresh)?);
    store.save()?;
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct TrialStatus {
    pub in_trial: bool,
    pub days_left: i64,
}

pub fn trial_status(_app: &AppHandle) -> TrialStatus {
    // Open-source build is free forever — there is no trial.
    TrialStatus { in_trial: false, days_left: 0 }
}
