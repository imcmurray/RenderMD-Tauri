//! External-change watching — port of the GTK watcher glue.
//!
//! Watches the document's PARENT directory (editors and our own atomic save
//! replace files via tmp+rename, which would orphan a file-level watch),
//! filters events to the target filename, coalesces inotify bursts with a
//! 150ms debounce, and suppresses events within 1500ms of our own save
//! (`AppState::last_self_write`, stamped before the rename).

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use notify::Watcher;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::commands::table::{refresh, toast};
use crate::state::AppState;

/// (Re)start watching for the given document path. Replaces any previous
/// watcher (dropping it disconnects the old debounce thread's channel,
/// which exits that thread).
pub fn start_watching<R: Runtime>(app: &AppHandle<R>, path: &Path) {
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let Some(target_name) = path.file_name().map(|n| n.to_os_string()) else {
        return;
    };

    let (tx, rx) = mpsc::channel::<()>();

    let mut watcher: notify::RecommendedWatcher =
        match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let touches_target = event
                    .paths
                    .iter()
                    .any(|p| p.file_name() == Some(target_name.as_os_str()));
                if touches_target {
                    let _ = tx.send(());
                }
            }
        }) {
            Ok(w) => w,
            Err(_) => return,
        };
    if watcher
        .watch(&parent, notify::RecursiveMode::NonRecursive)
        .is_err()
    {
        return;
    }

    // Debounce thread: blocks until an event lands, then drains until the
    // burst goes quiet for 150ms before acting. Exits when the watcher (and
    // with it the sender) is dropped.
    let thread_app = app.clone();
    std::thread::spawn(move || {
        let app = thread_app;
        while rx.recv().is_ok() {
            loop {
                std::thread::sleep(Duration::from_millis(150));
                if rx.try_recv().is_err() {
                    break;
                }
                while rx.try_recv().is_ok() {}
            }
            on_external_change(&app);
        }
    });

    let state = app.state::<Mutex<AppState>>();
    state.lock().unwrap().watcher = Some(watcher);
}

// NOTE: there is no stop_watching — replacing AppState (new_file) or its
// watcher field drops the old watcher, which disconnects the debounce
// thread's channel and winds it down.

fn on_external_change<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<Mutex<AppState>>();
    let mut s = state.lock().unwrap();

    // Self-write suppression: our atomic save stamps last_self_write right
    // before the rename lands.
    if s.last_self_write.elapsed() < Duration::from_millis(1500) {
        return;
    }
    let Some(path) = s.file_path.clone() else {
        return;
    };

    if s.mode == crate::state::Mode::Edit && s.is_modified {
        // Don't clobber unsaved edits — the frontend shows a Reload toast.
        let _ = app.emit("external-change", serde_json::json!({ "dirty": true }));
        return;
    }
    reload_from_disk_inner(app, &mut s, &path);
    toast(app, "Reloaded — file changed on disk");
}

/// Re-read the document, compute change bars vs the previous text, and push
/// the new state to the editor + preview. Keeps the current mode (unlike a
/// fresh open).
pub fn reload_from_disk_inner<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, path: &Path) {
    let new_text = match std::fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => {
            toast(app, &format!("{} is no longer readable", path.display()));
            return;
        }
    };

    let old_text = s.text.clone();
    let changed = rendermd_core::diff::compute_changed_lines(&old_text, &new_text);
    if !changed.is_empty() {
        let reload_ts = chrono::Local::now().timestamp();
        s.pending_changes = Some(rendermd_core::diff::PendingChanges {
            changed_lines: changed,
            old_text,
            reload_ts,
        });
    }
    s.text = new_text;
    s.is_modified = false;

    let _ = app.emit(
        "doc-replaced",
        serde_json::json!(crate::commands::DocInfo::from_state(s)),
    );
    refresh(app, s);
}

/// Toast "Reload" action for the edit-mode-dirty case.
#[tauri::command]
pub fn reload_from_disk<R: Runtime>(app: AppHandle<R>, state: tauri::State<'_, Mutex<AppState>>) {
    let mut s = state.lock().unwrap();
    let Some(path) = s.file_path.clone() else {
        return;
    };
    reload_from_disk_inner(&app, &mut s, &path);
    toast(&app, "Reloaded from disk");
}
