//! HotkeyDaemon — registers global shortcuts and dispatches them
//! based on which window is foreground.

use anyhow::{Context, Result};
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

pub fn register_all(app: &AppHandle, settings: &Settings) -> Result<()> {
    let gs = app.global_shortcut();
    // Unregister any prior bindings first (rebinding flow).
    let _ = gs.unregister_all();

    let summon: Shortcut = settings
        .summon_hotkey
        .parse()
        .context("parse summon hotkey")?;
    let action: Shortcut = settings
        .action_hotkey
        .parse()
        .context("parse action hotkey")?;

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
        warn!("unknown shortcut fired");
        Ok(())
    }
}

async fn on_summon(app: AppHandle, settings: Settings) -> Result<()> {
    if !reader::is_claude_focused() {
        notify(&app, "AI-AI", "Open Claude Desktop and click into the chat first.");
        return Ok(());
    }
    let snapshot = match reader::snapshot() {
        Ok(s) => s,
        Err(e) => {
            warn!(?e, "snapshot failed");
            notify(&app, "AI-AI", "Couldn't read Claude's chat. Try clicking into the chat box first.");
            return Ok(());
        }
    };

    if snapshot.last_assistant_msg.trim().is_empty() {
        notify(
            &app,
            "AI-AI",
            "No recent Claude response found — wait for one to finish before summoning.",
        );
        return Ok(());
    }

    let Some(api_key) = secrets::get_api_key().ok().flatten() else {
        notify(&app, "AI-AI — set up your API key", "Open Settings from the tray to paste your Anthropic API key.");
        let _ = crate::open_settings(&app);
        return Ok(());
    };

    show_overlay(&app, &snapshot)?;
    spawn_generation(app.clone(), snapshot, settings, api_key, false);
    Ok(())
}

async fn on_action(app: AppHandle, settings: Settings) -> Result<()> {
    // Case A: overlay focused → user is accepting a suggestion.
    if let Some(overlay) = app.get_webview_window("overlay") {
        if overlay.is_focused().unwrap_or(false) {
            // Frontend handles the accept; we just emit the trigger.
            let _ = overlay.emit("accept-highlighted", ());
            return Ok(());
        }
    }

    // Case B: Claude chat input focused → improve the current draft.
    let is_chat = reader::is_chat_input_focused().unwrap_or(false);
    if !is_chat {
        debug!("action hotkey ignored: neither overlay nor Claude chat focused");
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
        notify(&app, "AI-AI", "Type something to improve, then press the hotkey.");
        return Ok(());
    }

    let Some(api_key) = secrets::get_api_key().ok().flatten() else {
        notify(&app, "AI-AI — set up your API key", "Open Settings from the tray to paste your Anthropic API key.");
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
                }
            }
            Err(e) => {
                warn!(?e, "improve_prompt failed");
                notify(
                    &app,
                    "AI-AI — couldn't improve",
                    "Anthropic call failed. Check your API key or try again.",
                );
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
            .inner_size(560.0, 360.0)
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

    // Anchor above the chat input. Fall back to centered-bottom if no rect.
    let rect = snapshot.chat_input_bounds;
    if rect.width > 0 && rect.height > 0 {
        let overlay_w = 560i32;
        let overlay_h = 360i32;
        let x = rect.x + (rect.width / 2) - (overlay_w / 2);
        let y = (rect.y - overlay_h - 12).max(40);
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
    force_variation: bool,
) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();
        let entry = if force_variation {
            state.variation.next(&snapshot.last_assistant_msg, &snapshot.chat_input_text)
        } else {
            // First call for this key seeds at 0; later calls increment.
            state.variation.next(&snapshot.last_assistant_msg, &snapshot.chat_input_text)
        };

        let suggester = current_suggester(api_key, &settings);
        match suggester
            .generate_followups(&snapshot, &settings, entry.seed.saturating_sub(1), &entry.prior_suggestions)
            .await
        {
            Ok(suggestions) => {
                state
                    .variation
                    .remember(&snapshot.last_assistant_msg, &snapshot.chat_input_text, suggestions.clone());
                *state.current_batch.lock() = suggestions.clone();
                *state.current_snapshot.lock() = Some(snapshot);
                if let Some(overlay) = app.get_webview_window("overlay") {
                    let _ = overlay.emit("suggestions", suggestions);
                }
            }
            Err(e) => {
                warn!(?e, "generate_followups failed");
                if let Some(overlay) = app.get_webview_window("overlay") {
                    let _ = overlay.emit("suggestion-error", e.to_string());
                }
            }
        }
    });
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Used by the IPC "regenerate" command — forces a new variation seed.
pub fn regenerate(app: AppHandle) -> Result<()> {
    let settings = settings::current(&app);
    let Some(snapshot) = app
        .state::<Arc<AppState>>()
        .current_snapshot
        .lock()
        .clone()
    else {
        anyhow::bail!("no snapshot in state");
    };
    let api_key = secrets::get_api_key()?
        .ok_or_else(|| anyhow::anyhow!("no API key configured"))?;
    spawn_generation(app, snapshot, settings, api_key, true);
    Ok(())
}

/// Used by the IPC "accept" command.
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

    // Bring Claude back to foreground before pasting.
    // (User's Claude window was already foreground when overlay appeared;
    //  hiding the overlay typically restores it.)
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;

    crate::injector::paste(&s.text, settings.auto_send_after_paste).await
}
