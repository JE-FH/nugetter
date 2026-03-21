use crate::models::PromptPayload;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub const EVENT_PACKAGE_DETECTED: &str = "package-detected";
pub const EVENT_WATCHER_STATUS: &str = "watcher-status";
pub const EVENT_WATCHER_ERROR: &str = "watcher-error";

const TRAY_ID: &str = "nugetter-tray";

pub fn update_tray_pending_indicator(
    app: &AppHandle,
    pending_count: usize,
    unacknowledged_count: usize,
    latest_hint: Option<&str>,
) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let is_main_window_visible = app
            .get_webview_window("main")
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false);

        let tooltip = if pending_count == 0 {
            "Nugetter: Watching (no pending updates)".to_string()
        } else {
            let base = format!(
                "Nugetter: {pending_count} pending update(s), {unacknowledged_count} unacknowledged"
            );
            match latest_hint {
                Some(hint) => format!("{base}\nLatest: {hint}"),
                None => base,
            }
        };

        let title = if is_main_window_visible {
            None
        } else if unacknowledged_count == 0 {
            None
        } else {
            Some(format!("({unacknowledged_count})"))
        };

        let _ = tray.set_tooltip(Some(tooltip));
        let _ = tray.set_title(title);
    }
}

pub fn emit_package_detected(app: &AppHandle, payload: PromptPayload) -> Result<(), String> {
    if let Err(err) = app
        .notification()
        .builder()
        .title("Nugetter: New package version")
        .body(format!(
            "{} -> {} is ready to upgrade",
            payload.package_id, payload.next_version
        ))
        .show()
    {
        emit_error(app, format!("Notification failed: {err}"));
    }

    app.emit(EVENT_PACKAGE_DETECTED, payload)
        .map_err(|err| format!("Failed to emit package event: {err}"))
}

pub fn emit_status(app: &AppHandle, status: impl Into<String>) {
    let _ = app.emit(EVENT_WATCHER_STATUS, status.into());
}

pub fn emit_error(app: &AppHandle, error: impl Into<String>) {
    let _ = app.emit(EVENT_WATCHER_ERROR, error.into());
}
