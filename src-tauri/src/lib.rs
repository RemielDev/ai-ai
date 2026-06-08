//! AI-AI - prompt copilot for Claude Desktop.

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
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;
use tracing::{error, info};

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_hook();
    init_tracing();
    info!(version = env!("CARGO_PKG_VERSION"), "AI-AI starting up");

    let app_state = Arc::new(AppState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
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
            commands::get_all_key_status,
            commands::clear_api_key,
            commands::validate_api_key,
            commands::load_settings,
            commands::save_settings,
            commands::accept_suggestion,
            commands::regenerate_suggestions,
            commands::close_overlay,
            commands::open_settings_window,
            commands::license_status,
            commands::activate_license,
            commands::trial_status,
            commands::get_app_info,
            commands::get_autostart,
            commands::set_autostart,
            commands::reset_all,
            commands::mark_first_run_done,
            commands::check_for_updates,
        ])
        .setup(|app| {
            // Tray icon ----------------------------------------------------------
            let settings_item = MenuItem::with_id(app, "open_settings", "Settings…", true, None::<&str>)?;
            let summon_item = MenuItem::with_id(app, "summon", "Summon now", true, Some("Ctrl+Shift+Space"))?;
            let about_item = MenuItem::with_id(app, "about", "About AI-AI", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(
                app,
                &[&summon_item, &settings_item, &about_item, &sep, &quit_item],
            )?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(format!("AI-AI v{} - prompt copilot for Claude", env!("CARGO_PKG_VERSION")))
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_settings" => { let _ = open_settings(app); }
                    "about"         => { let _ = open_about(app); }
                    "summon" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = hotkey::manual_summon(app).await;
                        });
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
                        let _ = open_settings(tray.app_handle());
                    }
                })
                .build(app)?;

            settings::ensure_loaded(app.handle())?;
            let settings_snapshot = settings::current(app.handle());
            if let Err(e) = hotkey::register_all(app.handle(), &settings_snapshot) {
                error!(?e, "Initial hotkey registration failed");
            }

            // Onboarding on first run only.
            if settings_snapshot.first_run {
                let _ = open_onboarding(app.handle());
            }

            // Hide the autostart window on launch - we only want the tray.
            let argv: Vec<String> = std::env::args().collect();
            let _silent_start = argv.iter().any(|a| a == "--autostart");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if matches!(window.label(), "overlay" | "settings" | "about") {
                    api.prevent_close();
                    let _ = window.hide();
                }
                // onboarding is allowed to close.
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running AI-AI");
}

pub(crate) fn open_settings(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show()?;
        window.unminimize().ok();
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("AI-AI - Settings")
        .inner_size(760.0, 720.0)
        .min_inner_size(680.0, 560.0)
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
        .inner_size(460.0, 420.0)
        .resizable(false)
        .center()
        .build()?;
    Ok(())
}

pub(crate) fn open_onboarding(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("onboarding") {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "onboarding", WebviewUrl::App("onboarding.html".into()))
        .title("Welcome to AI-AI")
        .inner_size(560.0, 640.0)
        .resizable(false)
        .center()
        .build()?;
    Ok(())
}

fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort crash log to %APPDATA%\ai-ai\crash.log
        if let Some(dir) = dirs_home_dir() {
            let log_dir = dir.join("AppData").join("Roaming").join("ai-ai");
            let _ = std::fs::create_dir_all(&log_dir);
            let path = log_dir.join("crash.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                use std::io::Write;
                let _ = writeln!(
                    file,
                    "[{}] panic: {info}\n",
                    chrono::Utc::now().to_rfc3339()
                );
            }
        }
        prev(info);
    }));
}

fn dirs_home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE").map(std::path::PathBuf::from)
}

fn init_tracing() {
    let log_dir = dirs_home_dir()
        .map(|p| p.join("AppData").join("Roaming").join("ai-ai"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _ = std::fs::create_dir_all(&log_dir);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,ai_ai_lib=debug")),
        )
        .with_ansi(false)
        .init();
}
