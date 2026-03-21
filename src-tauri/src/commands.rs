use crate::models::{LocalPackageInfo, WatchSettings};
use crate::nuget;
use crate::state::AppState;
use crate::ui_events;
use semver::Version;
use crate::watcher;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn save_settings(
    settings: WatchSettings,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    validate_paths(&settings)?;

    let watch_path = PathBuf::from(&settings.watch_path);
    let destination_path = PathBuf::from(&settings.destination_path);

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    state.replace_settings_and_watcher(settings, stop_tx)?;

    watcher::spawn_watcher_thread(
        app,
        state.inner().clone(),
        watch_path,
        destination_path,
        stop_rx,
    )?;

    Ok("Settings saved and watcher started.".to_string())
}

#[tauri::command]
pub fn process_copy_request(
    request_id: String,
    approved: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let Some(request) = state.take_pending_request(&request_id)? else {
        return Err("Request not found. It may have already been handled.".to_string());
    };

    let pending_count = state.pending_request_count()?;
    let unacknowledged_count = state.unacknowledged_update_count()?;
    ui_events::update_tray_pending_indicator(&app, pending_count, unacknowledged_count, None);

    if !approved {
        return Ok("Copy request declined.".to_string());
    }

    let target = nuget::repackage_with_new_version(&request)?;

    Ok(format!(
        "Copied package {} to {}",
        request.package_id,
        target.display()
    ))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Option<WatchSettings>, String> {
    state.settings()
}

#[tauri::command]
pub fn get_local_packages(state: State<'_, AppState>) -> Result<Vec<LocalPackageInfo>, String> {
    let Some(settings) = state.settings()? else {
        return Ok(Vec::new());
    };

    nuget::list_local_packages(Path::new(&settings.destination_path))
}

fn validate_paths(settings: &WatchSettings) -> Result<(), String> {
    let watch = Path::new(&settings.watch_path);
    if !watch.exists() || !watch.is_dir() {
        return Err("Watch path must exist and be a directory.".to_string());
    }

    let destination = Path::new(&settings.destination_path);
    if !destination.exists() {
        fs::create_dir_all(destination)
            .map_err(|e| format!("Failed to create destination path: {e}"))?;
    }
    if !destination.is_dir() {
        return Err("Destination path must be a directory.".to_string());
    }

    if let Some(start_version) = &settings.start_version {
        if Version::parse(start_version).is_err() {
            return Err("Start version must be a valid semantic version (for example 1.0.0).".to_string());
        }
    }

    Ok(())
}
