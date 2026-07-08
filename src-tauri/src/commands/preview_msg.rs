//! Dispatcher for preview-frame channels forwarded by the shell.
//!
//! The preview document posts tab-delimited string payloads on named
//! channels (the same wire format the GTK app used with WebKit script
//! message handlers). The frontend forwards every channel it doesn't handle
//! locally to this command.
//!
//! Channel → handler mapping fills in per phase:
//! - Phase 4: tableEdit / tableNavigate / tableStructure / tableSort /
//!   tableResizeColumns
//! - Phase 5: imageResize / imageMove
//! - Phase 7: commitClick / toggleHistory / toggleHistoryCollapse

use std::sync::Mutex;

use tauri::{AppHandle, Runtime, State};

use crate::state::AppState;

#[tauri::command]
pub fn preview_message<R: Runtime>(
    _app: AppHandle<R>,
    _state: State<'_, Mutex<AppState>>,
    channel: String,
    payload: String,
) -> Result<(), String> {
    // Per-phase channel arms land here (see module docs). Unknown channels
    // are logged, not errors — a newer preview doc must never hard-fail an
    // older shell.
    eprintln!(
        "preview_message: unhandled channel {channel} ({} bytes)",
        payload.len()
    );
    Ok(())
}
