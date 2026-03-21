use crate::models::{PendingCopyRequest, WatchSettings};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<AppStateInner>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AppStateInner::default())),
        }
    }
}

#[derive(Default)]
struct AppStateInner {
    settings: Option<WatchSettings>,
    pending_requests: HashMap<String, PendingCopyRequest>,
    stop_sender: Option<mpsc::Sender<()>>,
}

impl AppState {
    pub fn replace_settings_and_watcher(
        &self,
        settings: WatchSettings,
        stop_tx: mpsc::Sender<()>,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|_| "State lock poisoned")?;

        if let Some(existing_stop) = inner.stop_sender.take() {
            let _ = existing_stop.send(());
        }

        inner.settings = Some(settings);
        inner.stop_sender = Some(stop_tx);
        Ok(())
    }

    pub fn settings(&self) -> Result<Option<WatchSettings>, String> {
        let inner = self.inner.lock().map_err(|_| "State lock poisoned")?;
        Ok(inner.settings.clone())
    }

    pub fn insert_pending_request(&self, request: PendingCopyRequest) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|_| "State lock poisoned")?;
        inner.pending_requests.insert(request.request_id.clone(), request);
        Ok(())
    }

    pub fn take_pending_request(&self, request_id: &str) -> Result<Option<PendingCopyRequest>, String> {
        let mut inner = self.inner.lock().map_err(|_| "State lock poisoned")?;
        Ok(inner.pending_requests.remove(request_id))
    }
}
