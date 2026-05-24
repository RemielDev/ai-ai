//! AI-AI — prompt copilot for Claude Desktop.
//!
//! Entry point invoked by `main.rs`. Wires together every unit:
//! HotkeyDaemon, ClaudeReader, Suggester, Overlay UI, Injector.

mod commands;
mod hotkey;
mod injector;
mod license;
mod reader;
mod secrets;
mod settings;
mod state;
mod suggester;

use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::ShortcutState;
use tracing::{error, info};

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,ai_ai_lib=debug")),
        )
        .init();

    info!("AI-AI starting up");

    let app_state = Arc::new(AppState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let app = app.clone();
                    let shortcut = shortcut.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = hotkey::dispatch(app, shortcut).await {
                            error!(?e, "hotkey dispatch failed");
                        }
                    });
                })
                .build(),
        )
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::save_api_key,
            commands::get_api_key_status,
            commands::clear_api_key,
            commands::load_settings,
            commands::save_settings,
            commands::accept_suggestion,
            commands::regenerate_suggestions,
            commands::close_overlay,
            commands::open_settings_window,
            commands::license_status,
            commands::activate_license,
        ])
        .setup(|app| {
            // Tray icon ----------------------------------------------------------
            let settings_item = MenuItem::with_id(app, "open_settings", "Settings…", true, None::<&str>)?;
            let about_item = MenuItem::with_id(app, "about", "About AI-AI", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&settings_item, &about_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("AI-AI — prompt copilot for Claude")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_settings" => {
                        if let Err(e) = open_settings(app) {
                            error!(?e, "failed to open settings");
                        }
                    }
                    "about" => {
                        if let Err(e) = open_about(app) {
                            error!(?e, "failed to open about");
                        }
                    }
                    "quit" => {
                        info!("Quit requested from tray");
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Err(e) = open_settings(tray.app_handle()) {
                            error!(?e, "failed to open settings from tray click");
                        }
                    }
                })
                .build(app)?;

            // Settings storage ---------------------------------------------------
            settings::ensure_loaded(app.handle())?;

            // Register hotkeys ---------------------------------------------------
            let settings_snapshot = settings::current(app.handle());
            hotkey::register_all(app.handle(), &settings_snapshot)?;

            // Welcome on first run -----------------------------------------------
            if settings_snapshot.first_run {
                let _ = open_settings(app.handle());
                settings::mark_first_run_complete(app.handle());
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide windows instead of destroying — quit only via tray.
                if window.label() == "overlay" || window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running AI-AI");
}

pub(crate) fn open_settings(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("AI-AI — Settings")
        .inner_size(720.0, 600.0)
        .min_inner_size(640.0, 520.0)
        .center()
        .resizable(true)
        .build()?;
    Ok(())
}

pub(crate) fn open_about(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("about") {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "about", WebviewUrl::App("about.html".into()))
        .title("About AI-AI")
        .inner_size(420.0, 320.0)
        .resizable(false)
        .center()
        .build()?;
    Ok(())
}
