#![cfg_attr(test, allow(dead_code, unused_imports))]

mod commands;
mod models;
mod nuget;
mod state;
mod ui_events;
mod watcher;

use state::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

const TRAY_ID: &str = "nugetter-tray";
const TRAY_MENU_OPEN: &str = "open";
const TRAY_MENU_QUIT: &str = "quit";
const MAIN_WINDOW_LABEL: &str = "main";

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }

    if let Some(state) = app.try_state::<AppState>() {
        let _ = state.reset_unacknowledged_updates();
        if let Ok(pending_count) = state.pending_request_count() {
            let unacknowledged_count = state.unacknowledged_update_count().unwrap_or(0);
            ui_events::update_tray_pending_indicator(app, pending_count, unacknowledged_count, None);
        }
    }
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItem::with_id(app, TRAY_MENU_OPEN, "Open Nugetter", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, TRAY_MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Nugetter: Running in background");

    // Reuse the bundled app icon so tray branding matches the Nugetter logo.
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_MENU_OPEN => show_main_window(app),
            TRAY_MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(&tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg(not(test))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            setup_tray(app)?;
            let _ = app.notification().request_permission();
            ui_events::update_tray_pending_indicator(&app.handle(), 0, 0, None);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let app = window.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(pending_count) = state.pending_request_count() {
                        let unacknowledged_count = state.unacknowledged_update_count().unwrap_or(0);
                        ui_events::update_tray_pending_indicator(
                            &app,
                            pending_count,
                            unacknowledged_count,
                            None,
                        );
                    }
                }
            }
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::save_settings,
            commands::process_copy_request,
            commands::get_settings,
            commands::get_local_packages
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
pub fn run() {
    // Unit tests compile backend logic without launching the Tauri runtime.
}
