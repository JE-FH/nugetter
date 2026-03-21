use crate::models::{PendingCopyRequest, PromptPayload};
use crate::nuget;
use crate::state::AppState;
use crate::ui_events;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);
const PACKAGE_READ_DEBOUNCE_MS: u64 = 1200;
const PACKAGE_READ_RETRY_MS: u64 = 500;
const PACKAGE_READ_MAX_RETRIES: u8 = 4;
const RECENT_PROCESS_COOLDOWN_MS: u64 = 2500;

#[derive(Clone, Copy)]
struct PendingCandidate {
    due_at: Instant,
    attempts: u8,
}

fn debug_log(message: impl AsRef<str>) {
    println!("[nugetter-watcher] {}", message.as_ref());
}

pub fn spawn_watcher_thread(
    app: AppHandle,
    state: AppState,
    watch_path: PathBuf,
    destination_path: PathBuf,
    stop_rx: Receiver<()>,
) -> Result<(), String> {
    thread::Builder::new()
        .name("nugetter-watcher".to_string())
        .spawn(move || {
            debug_log(format!("starting watcher thread for root: {}", watch_path.display()));
            let (event_tx, event_rx) = mpsc::channel::<notify::Result<Event>>();
            let mut watcher = match RecommendedWatcher::new(
                move |res| {
                    let _ = event_tx.send(res);
                },
                Config::default(),
            ) {
                Ok(watcher) => watcher,
                Err(err) => {
                    debug_log(format!("failed to start watcher: {err}"));
                    ui_events::emit_error(&app, format!("Failed to start watcher: {err}"));
                    return;
                }
            };

            if let Err(err) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
                debug_log(format!("failed to watch {}: {err}", watch_path.display()));
                ui_events::emit_error(
                    &app,
                    format!("Failed to watch folder {}: {err}", watch_path.display()),
                );
                return;
            }

            ui_events::emit_status(
                &app,
                format!(
                    "Watching {} recursively for C# project package outputs",
                    watch_path.display()
                ),
            );
            debug_log(format!(
                "watching recursively for package files under {}",
                watch_path.display()
            ));

            let mut pending_candidates: HashMap<PathBuf, PendingCandidate> = HashMap::new();
            let mut recently_processed: HashMap<PathBuf, Instant> = HashMap::new();

            loop {
                if stop_rx.try_recv().is_ok() {
                    debug_log("stop signal received; stopping watcher");
                    ui_events::emit_status(&app, "Watcher stopped");
                    break;
                }

                cleanup_recently_processed(&mut recently_processed);

                match event_rx.recv_timeout(Duration::from_millis(350)) {
                    Ok(Ok(event)) => queue_event_candidates(
                        &watch_path,
                        &mut pending_candidates,
                        &recently_processed,
                        event,
                    ),
                    Ok(Err(err)) => {
                        debug_log(format!("watch event error: {err}"));
                        ui_events::emit_error(&app, format!("Watch event error: {err}"));
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }

                process_due_candidates(
                    &app,
                    &state,
                    &destination_path,
                    &mut pending_candidates,
                    &mut recently_processed,
                );
            }
        })
        .map_err(|e| format!("Failed to spawn watcher thread: {e}"))?;

    Ok(())
}

fn queue_event_candidates(
    watch_root: &Path,
    pending_candidates: &mut HashMap<PathBuf, PendingCandidate>,
    recently_processed: &HashMap<PathBuf, Instant>,
    event: Event,
) {
    if !is_interesting_event(&event.kind) {
        return;
    }

    debug_log(format!(
        "event {:?} with {} path(s)",
        event.kind,
        event.paths.len()
    ));

    for path in event.paths {
        if let Some(reason) = path_rejection_reason(path.as_path(), watch_root) {
            if nuget::is_package_file(path.as_path()) {
                debug_log(format!(
                    "skipped candidate {}: {}",
                    path.display(),
                    reason
                ));
            }
            continue;
        }

        if recently_processed
            .get(&path)
            .map(|ts| ts.elapsed() < Duration::from_millis(RECENT_PROCESS_COOLDOWN_MS))
            .unwrap_or(false)
        {
            debug_log(format!(
                "skipped candidate in recent cooldown window: {}",
                path.display()
            ));
            continue;
        }

        let due_at = Instant::now() + Duration::from_millis(PACKAGE_READ_DEBOUNCE_MS);
        pending_candidates.insert(path.clone(), PendingCandidate { due_at, attempts: 0 });
        debug_log(format!(
            "queued candidate for debounced read ({} ms): {}",
            PACKAGE_READ_DEBOUNCE_MS,
            path.display()
        ));
    }
}

fn path_rejection_reason(path: &Path, watch_root: &Path) -> Option<&'static str> {
    if !nuget::is_package_file(path) {
        return Some("not a .nupkg/.nuget file");
    }

    if !path.is_file() {
        return Some("path is not a file yet");
    }

    if !is_path_from_csharp_project_build(path, watch_root) {
        return Some("no .csproj ancestor found between file and watch root");
    }

    None
}

fn process_due_candidates(
    app: &AppHandle,
    state: &AppState,
    destination_path: &Path,
    pending_candidates: &mut HashMap<PathBuf, PendingCandidate>,
    recently_processed: &mut HashMap<PathBuf, Instant>,
) {
    let now = Instant::now();
    let due_paths: Vec<PathBuf> = pending_candidates
        .iter()
        .filter_map(|(path, pending)| (pending.due_at <= now).then_some(path.clone()))
        .collect();

    for path in due_paths {
        let Some(pending) = pending_candidates.remove(&path) else {
            continue;
        };

        match handle_detected_package(app, state, &path, destination_path) {
            Ok(()) => {
                recently_processed.insert(path.clone(), Instant::now());
                debug_log(format!("processed candidate successfully: {}", path.display()));
            }
            Err(err) => {
                let next_attempt = pending.attempts.saturating_add(1);
                if next_attempt <= PACKAGE_READ_MAX_RETRIES {
                    let retry_due = Instant::now() + Duration::from_millis(PACKAGE_READ_RETRY_MS);
                    pending_candidates.insert(
                        path.clone(),
                        PendingCandidate {
                            due_at: retry_due,
                            attempts: next_attempt,
                        },
                    );
                    debug_log(format!(
                        "read failed; retrying ({}/{}) in {} ms for {}: {}",
                        next_attempt,
                        PACKAGE_READ_MAX_RETRIES,
                        PACKAGE_READ_RETRY_MS,
                        path.display(),
                        err
                    ));
                } else {
                    debug_log(format!(
                        "read failed after retries for {}: {}",
                        path.display(),
                        err
                    ));
                    ui_events::emit_error(app, err);
                }
            }
        }
    }
}

fn cleanup_recently_processed(recently_processed: &mut HashMap<PathBuf, Instant>) {
    recently_processed
        .retain(|_, seen_at| seen_at.elapsed() < Duration::from_millis(RECENT_PROCESS_COOLDOWN_MS));
}

fn is_interesting_event(kind: &EventKind) -> bool {
    matches!(kind, EventKind::Create(_) | EventKind::Modify(_))
}

fn is_path_from_csharp_project_build(path: &Path, watch_root: &Path) -> bool {
    if !path.starts_with(watch_root) {
        return false;
    }

    if !path_has_bin_segment(path) {
        return false;
    }

    has_csproj_ancestor(path, watch_root)
}

fn path_has_bin_segment(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("bin")
    })
}

fn has_csproj_ancestor(path: &Path, watch_root: &Path) -> bool {
    let mut current = path.parent();

    while let Some(dir) = current {
        if directory_contains_csproj(dir) {
            return true;
        }

        if dir == watch_root {
            break;
        }

        current = dir.parent();
    }

    false
}

fn directory_contains_csproj(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };

    entries.flatten().any(|entry| {
        entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("csproj"))
            .unwrap_or(false)
    })
}

fn handle_detected_package(
    app: &AppHandle,
    state: &AppState,
    source_path: &Path,
    destination_path: &Path,
) -> Result<(), String> {
    debug_log(format!("reading package metadata for {}", source_path.display()));
    let (package_id, current_version) = nuget::read_package_metadata(source_path)?;
    let configured_start_version = state.settings()?.and_then(|settings| settings.start_version);
    let next_version = nuget::compute_next_version(
        destination_path,
        &package_id,
        &current_version,
        configured_start_version.as_deref(),
    );
    let destination_file_name = format!("{}.{}.nupkg", package_id, next_version);
    let request_id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();

    let request = PendingCopyRequest {
        request_id: request_id.clone(),
        source_path: source_path.to_path_buf(),
        destination_path: destination_path.to_path_buf(),
        package_id: package_id.clone(),
        next_version: next_version.clone(),
        destination_file_name: destination_file_name.clone(),
    };

    state.insert_pending_request(request)?;
    let pending_count = state.pending_request_count()?;
    let unacknowledged_count = state.increment_unacknowledged_updates()?;
    let latest_hint = format!("{} -> {}", package_id, next_version);
    ui_events::update_tray_pending_indicator(
        app,
        pending_count,
        unacknowledged_count,
        Some(&latest_hint),
    );

    debug_log(format!(
        "detected package {} current={} next={} source={} target={} ",
        package_id,
        current_version,
        next_version,
        source_path.display(),
        destination_file_name
    ));

    ui_events::emit_package_detected(
        app,
        PromptPayload {
            request_id,
            source_path: source_path.display().to_string(),
            package_id,
            current_version,
            next_version,
            destination_path: destination_path.display().to_string(),
            destination_file_name,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{is_path_from_csharp_project_build, path_rejection_reason};
    use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn detects_package_under_csproj_bin_tree() {
        let base = temp_test_root("nugetter-watcher-test");
        let package = create_project_package_tree(
            &base,
            "proj-a",
            "bin/Debug",
            "My.Package.1.0.0.nupkg",
            true,
        );

        assert!(is_path_from_csharp_project_build(&package, &base));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn rejects_package_without_csproj_ancestor() {
        let base = temp_test_root("nugetter-watcher-test");
        let package = create_project_package_tree(
            &base,
            "other",
            "bin/Debug",
            "Some.Package.1.0.0.nupkg",
            false,
        );

        assert!(!is_path_from_csharp_project_build(&package, &base));
        assert!(path_rejection_reason(&package, &base).is_some());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn detects_package_with_uppercase_extension() {
        let base = temp_test_root("nugetter-watcher-test");
        let package = create_project_package_tree(
            &base,
            "proj-b",
            "bin/Release",
            "My.Package.1.0.0.NUPKG",
            true,
        );

        assert!(path_rejection_reason(&package, &base).is_none());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn detects_package_directly_in_bin_debug() {
        let base = temp_test_root("nugetter-direct-bin-debug");
        let package = create_project_package_tree(
            &base,
            "proj-direct",
            "bin/Debug",
            "Direct.Package.1.0.0.nupkg",
            true,
        );

        assert!(path_rejection_reason(&package, &base).is_none());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn end_to_end_detects_created_package_file() {
        let base = temp_test_root("nugetter-e2e-test");
        let project_dir = base.join("proj-e2e");
        let bin_dir = project_dir.join("bin/Debug");
        let package = bin_dir.join("Example.Package.1.0.0.nupkg");

        fs::create_dir_all(&bin_dir).expect("create bin dir");
        fs::write(project_dir.join("proj-e2e.csproj"), "<Project />").expect("write csproj");

        let watch_root = base.clone();
        let (detected_tx, detected_rx) = mpsc::channel::<Result<PathBuf, String>>();

        let watcher_thread = thread::spawn(move || {
            let result = wait_for_detectable_package_event(&watch_root, Duration::from_secs(8));
            let _ = detected_tx.send(result);
        });

        // Allow the watcher setup to complete before creating the package.
        thread::sleep(Duration::from_millis(250));
        write_minimal_nupkg(&package, "Example.Package", "1.0.0");

        let detected = detected_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("watcher thread did not return")
            .expect("no detectable package event received");

        watcher_thread.join().expect("watcher thread panicked");
        assert_eq!(detected, package);

        let _ = fs::remove_dir_all(base);
    }

    fn wait_for_detectable_package_event(
        watch_root: &Path,
        timeout: Duration,
    ) -> Result<PathBuf, String> {
        let (event_tx, event_rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = event_tx.send(res);
            },
            Config::default(),
        )
        .map_err(|err| format!("failed to create watcher: {err}"))?;

        watcher
            .watch(watch_root, RecursiveMode::Recursive)
            .map_err(|err| format!("failed to watch {}: {err}", watch_root.display()))?;

        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("timed out waiting for detectable package event".to_string());
            }

            match event_rx.recv_timeout(remaining) {
                Ok(Ok(event)) => {
                    for path in event.paths {
                        if path_rejection_reason(&path, watch_root).is_none() {
                            return Ok(path);
                        }
                    }
                }
                Ok(Err(err)) => return Err(format!("watch error: {err}")),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err("timed out waiting for watcher event".to_string())
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("watch channel disconnected".to_string())
                }
            }
        }
    }

    fn write_minimal_nupkg(path: &Path, package_id: &str, version: &str) {
        let file = fs::File::create(path).expect("create nupkg file");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let nuspec_name = format!("{package_id}.nuspec");
        let nuspec = format!(
            "<?xml version=\"1.0\"?><package><metadata><id>{package_id}</id><version>{version}</version></metadata></package>"
        );

        writer
            .start_file(nuspec_name, options)
            .expect("start nuspec entry");
        writer
            .write_all(nuspec.as_bytes())
            .expect("write nuspec content");
        writer.finish().expect("finish nupkg zip");
    }

    fn temp_test_root(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    fn create_project_package_tree(
        base: &Path,
        project_name: &str,
        bin_relative_path: &str,
        package_file_name: &str,
        with_csproj: bool,
    ) -> PathBuf {
        let project_dir = base.join(project_name);
        let bin_dir = project_dir.join(bin_relative_path);
        let package = bin_dir.join(package_file_name);

        fs::create_dir_all(&bin_dir).expect("create bin dir");
        if with_csproj {
            fs::write(
                project_dir.join(format!("{project_name}.csproj")),
                "<Project />",
            )
            .expect("write csproj");
        }
        fs::write(&package, "fake").expect("write package");

        package
    }
}
