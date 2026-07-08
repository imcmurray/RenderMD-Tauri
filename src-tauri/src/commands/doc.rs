//! Document/session commands: text updates, mode, theme.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Runtime, State};

use super::DocInfo;
use crate::state::{AppState, Mode};

/// The editor's debounced text sync. Stores the new text, re-renders, and
/// notifies the preview.
#[tauri::command]
pub fn update_text<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<AppState>>,
    text: String,
) -> u64 {
    let mut s = state.lock().unwrap();
    if text != s.text {
        s.text = text;
        s.is_modified = true;
        // Editing the working copy ends snapshot browsing — the preview
        // must show what's being typed, not a historical revision.
        s.viewing_snapshot = None;
        s.pending_changes = None;
    }
    s.render_preview();
    let _ = app.emit(
        "preview-updated",
        serde_json::json!({ "rev": s.preview_rev }),
    );
    s.preview_rev
}

#[tauri::command]
pub fn set_mode(state: State<'_, Mutex<AppState>>, mode: String) {
    let mut s = state.lock().unwrap();
    s.mode = if mode == "edit" {
        Mode::Edit
    } else {
        Mode::Preview
    };
}

#[tauri::command]
pub fn get_doc(state: State<'_, Mutex<AppState>>) -> DocInfo {
    let s = state.lock().unwrap();
    DocInfo::from_state(&s)
}

/// Version + git SHA for the About dialog.
#[tauri::command]
pub fn get_build_info() -> serde_json::Value {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "gitSha": env!("GIT_SHA"),
    })
}

/// Theme flip from the frontend (`prefers-color-scheme` listener).
#[tauri::command]
pub fn set_dark<R: Runtime>(app: AppHandle<R>, state: State<'_, Mutex<AppState>>, dark: bool) {
    let mut s = state.lock().unwrap();
    if s.dark != dark {
        s.dark = dark;
        s.render_preview();
        let _ = app.emit(
            "preview-updated",
            serde_json::json!({ "rev": s.preview_rev }),
        );
    }
}
