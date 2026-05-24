//! HotkeyDaemon — registers global shortcuts, dispatches by focused window.

use anyhow::{Context, Result};
use serde::Serialize;
use std::sync::Arc;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_notification::NotificationExt;
use tracing::{debug, info, warn};

use crate::reader::{self, ChatSnapshot};
use crate::secrets;
use crate::settings::{self, Settings};
use crate::state::AppState;
use crate::suggester::{current_suggester, Suggestion};

#[derive(Serialize, Clone)]
pub struct SuggestionError {
    pub title: String,
    pub body: String,
}

pub fn register_all(app: &AppHandle, settings: &Settings) -> Result<()> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let summon: Shortcut = settings
        .summon_hotkey
        .parse()
        .with_context(|| format!("parse summon hotkey '{}'", settings.summon_hotkey))?;
    let action: Shortcut = settings
        .action_hotkey
        .parse()
        .with_context(|| format!("parse action hotkey '{}'", settings.action_hotkey))?;

    gs.register(summon).context("register summon hotkey")?;
    gs.register(action).context("register action hotkey")?;

    info!(
        summon = %settings.summon_hotkey,
        action = %settings.action_hotkey,
        "Hotkeys registered"
    );
    Ok(())
}

pub async fn dispatch(app: AppHandle, shortcut: Shortcut) -> Result<()> {
    let settings = settings::current(&app);
    let summon: Shortcut = settings.summon_hotkey.parse()?;
    let action: Shortcut = settings.action_hotkey.parse()?;

    if shortcut == summon {
        debug!("summon hotkey fired");
        on_summon(app, settings).await
    } else if shortcut == action {
        debug!("action hotkey fired");
        on_action(app, settings).await
    } else {
        Ok(())
    }
}

/// Called from the tray menu — same as summon but skips Claude-focus check
/// (user explicitly asked, so show overlay anyway with an empty state).
pub async fn manual_summon(app: AppHandle) -> Result<()> {
    let settings = settings::current(&app);
    on_summon(app, settings).await
}

async fn on_summon(app: AppHandle, settings: Settings) -> Result<()> {
    if !reader::is_claude_focused() {
        notify(
            &app,
            "AI-AI",
            "Open Claude Desktop and click into the chat first.",
        );
        return Ok(());
    }
    let snapshot = match reader::snapshot() {
        Ok(s) => s,
        Err(e) => {
            warn!(?e, "snapshot failed");
            notify(
                &app,
                "AI-AI",
                "Couldn't read Claude's chat. Try clicking into the chat box first.",
            );
            return Ok(());
        }
    };

    if snapshot.last_assistant_msg.trim().is_empty() {
        notify(
            &app,
            "AI-AI",
            "No recent Claude response found — wait for one to finish.",
        );
        return Ok(());
    }

    let Some(api_key) = secrets::get_api_key().ok().flatten() else {
        notify(
            &app,
            "AI-AI — set up your API key",
            "Open Settings from the tray to paste your Anthropic API key.",
        );
        let _ = crate::open_settings(&app);
        return Ok(());
    };

    show_overlay(&app, &snapshot)?;
    // Push steer text to overlay UI so it can show the banner.
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("steer", snapshot.chat_input_text.clone());
    }
    spawn_generation(app.clone(), snapshot, settings, api_key);
    Ok(())
}

async fn on_action(app: AppHandle, settings: Settings) -> Result<()> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        if overlay.is_focused().unwrap_or(false) {
            let _ = overlay.emit("accept-highlighted", ());
            return Ok(());
        }
    }

    let is_chat = reader::is_chat_input_focused().unwrap_or(false);
    if !is_chat {
        debug!("action hotkey ignored: not in Claude chat input");
        return Ok(());
    }

    let snapshot = match reader::snapshot() {
        Ok(s) => s,
        Err(e) => {
            warn!(?e, "snapshot for improve failed");
            return Ok(());
        }
    };
    let draft = snapshot.chat_input_text.trim();
    if draft.is_empty() {
        notify(&app, "AI-AI", "Type something first, then press the hotkey.");
        return Ok(());
    }

    let Some(api_key) = secrets::get_api_key().ok().flatten() else {
        notify(
            &app,
            "AI-AI — set up your API key",
            "Open Settings from the tray to paste your Anthropic API key.",
        );
        return Ok(());
    };

    let draft_owned = draft.to_string();
    tauri::async_runtime::spawn(async move {
        let suggester = current_suggester(api_key, &settings);
        match suggester.improve_prompt(&draft_owned).await {
            Ok(improved) => {
                if let Err(e) =
                    crate::injector::paste(&improved, settings.auto_send_after_paste).await
                {
                    warn!(?e, "paste of improved prompt failed");
                    notify(
                        &app,
                        "AI-AI — paste failed",
                        "Try clicking into Claude's chat box, then retry.",
                    );
                }
            }
            Err(e) => {
                warn!(?e, "improve_prompt failed");
                let msg = friendly_error(&e.to_string());
                notify(&app, "AI-AI — couldn't improve", &msg);
            }
        }
    });
    Ok(())
}

fn show_overlay(app: &AppHandle, snapshot: &ChatSnapshot) -> Result<()> {
    let window = match app.get_webview_window("overlay") {
        Some(w) => w,
        None => WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
            .title("AI-AI")
            .inner_size(600.0, 400.0)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .transparent(true)
            .focused(true)
            .visible(false)
            .build()
            .context("build overlay window")?,
    };

    let rect = snapshot.chat_input_bounds;
    let overlay_w = 600i32;
    let overlay_h = 400i32;
    if rect.width > 0 && rect.height > 0 {
        let x = rect.x + (rect.width / 2) - (overlay_w / 2);
        let y = (rect.y - overlay_h - 12).max(40);
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(PhysicalSize::new(overlay_w as u32, overlay_h as u32));
    } else if let Some(monitor) = window.current_monitor()? {
        let size = monitor.size();
        let x = (size.width as i32 / 2) - (overlay_w / 2);
        let y = (size.height as i32) - overlay_h - 120;
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(PhysicalSize::new(overlay_w as u32, overlay_h as u32));
    }

    window.show()?;
    window.set_focus()?;
    Ok(())
}

fn spawn_generation(
    app: AppHandle,
    snapshot: ChatSnapshot,
    settings: Settings,
    api_key: String,
) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();
        let entry = state
            .variation
            .next(&snapshot.last_assistant_msg, &snapshot.chat_input_text);

        let suggester = current_suggester(api_key, &settings);
        match suggester
            .generate_followups(
                &snapshot,
                &settings,
                entry.seed.saturating_sub(1),
                &entry.prior_suggestions,
            )
            .await
        {
            Ok(suggestions) => {
                state.variation.remember(
                    &snapshot.last_assistant_msg,
                    &snapshot.chat_input_text,
                    suggestions.clone(),
                );
                state.usage.record_round(suggestions.len());
                *state.current_batch.lock() = suggestions.clone();
                *state.current_snapshot.lock() = Some(snapshot);
                if let Some(overlay) = app.get_webview_window("overlay") {
                    let _ = overlay.emit("suggestions", suggestions);
                }
            }
            Err(e) => {
                warn!(?e, "generate_followups failed");
                if let Some(overlay) = app.get_webview_window("overlay") {
                    let err = categorize_error(&e.to_string());
                    let _ = overlay.emit("suggestion-error", err);
                }
            }
        }
    });
}

pub fn regenerate(app: AppHandle) -> Result<()> {
    let settings = settings::current(&app);
    let snapshot = app
        .state::<Arc<AppState>>()
        .current_snapshot
        .lock()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no snapshot in state"))?;
    let api_key = secrets::get_api_key()?
        .ok_or_else(|| anyhow::anyhow!("no API key configured"))?;
    spawn_generation(app, snapshot, settings, api_key);
    Ok(())
}

pub async fn accept(app: AppHandle, index: usize) -> Result<()> {
    let settings = settings::current(&app);
    let suggestion: Option<Suggestion> = {
        let state = app.state::<Arc<AppState>>();
        let batch = state.current_batch.lock();
        batch.get(index).cloned()
    };
    let Some(s) = suggestion else {
        anyhow::bail!("no suggestion at index {}", index);
    };

    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
    {
        let state = app.state::<Arc<AppState>>();
        state.usage.record_accept();
    }

    tokio::time::sleep(std::time::Duration::from_millis(140)).await;
    crate::injector::paste(&s.text, settings.auto_send_after_paste).await
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

fn categorize_error(raw: &str) -> SuggestionError {
    let lower = raw.to_lowercase();
    if lower.contains("401") || lower.contains("unauthorized") {
        SuggestionError {
            title: "API key rejected".into(),
            body: "Anthropic returned 401. Check your key in Settings.".into(),
        }
    } else if lower.contains("429") || lower.contains("rate") {
        SuggestionError {
            title: "Rate limited".into(),
            body: "Anthropic rate limit hit. Wait a moment and retry.".into(),
        }
    } else if lower.contains("timed out") || lower.contains("timeout") || lower.contains("connect") || lower.contains("dns") {
        SuggestionError {
            title: "Network error".into(),
            body: "Couldn't reach Anthropic. Check your internet connection.".into(),
        }
    } else if lower.contains("no recent assistant") {
        SuggestionError {
            title: "Nothing to react to".into(),
            body: "Wait for Claude to finish its response, then summon again.".into(),
        }
    } else if lower.contains("zero usable suggestions") || lower.contains("decode suggestions") {
        SuggestionError {
            title: "Couldn't parse response".into(),
            body: "Anthropic returned an unexpected shape. Try again — usually it works second time.".into(),
        }
    } else {
        SuggestionError {
            title: "Couldn't generate suggestions".into(),
            body: raw.to_string(),
        }
    }
}

fn friendly_error(raw: &str) -> String {
    categorize_error(raw).body
}
