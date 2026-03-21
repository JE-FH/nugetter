use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchSettings {
    pub watch_path: String,
    pub destination_path: String,
    #[serde(default)]
    pub start_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPayload {
    pub request_id: String,
    pub source_path: String,
    pub package_id: String,
    pub current_version: String,
    pub next_version: String,
    pub destination_path: String,
    pub destination_file_name: String,
}

#[derive(Debug, Clone)]
pub struct PendingCopyRequest {
    pub request_id: String,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub package_id: String,
    pub next_version: String,
    pub destination_file_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPackageInfo {
    pub package_id: String,
    pub latest_version: String,
}
