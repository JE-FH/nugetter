use crate::models::PromptPayload;
use tauri::{AppHandle, Emitter};

pub const EVENT_PACKAGE_DETECTED: &str = "package-detected";
pub const EVENT_WATCHER_STATUS: &str = "watcher-status";
pub const EVENT_WATCHER_ERROR: &str = "watcher-error";

pub fn emit_package_detected(app: &AppHandle, payload: PromptPayload) -> Result<(), String> {
    app.emit(EVENT_PACKAGE_DETECTED, payload)
        .map_err(|err| format!("Failed to emit package event: {err}"))
}

pub fn emit_status(app: &AppHandle, status: impl Into<String>) {
    let _ = app.emit(EVENT_WATCHER_STATUS, status.into());
}

pub fn emit_error(app: &AppHandle, error: impl Into<String>) {
    let _ = app.emit(EVENT_WATCHER_ERROR, error.into());
}
