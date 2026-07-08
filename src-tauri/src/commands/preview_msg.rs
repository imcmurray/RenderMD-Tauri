//! Dispatcher for preview-frame channels forwarded by the shell.
//!
//! The preview document posts tab-delimited string payloads on named
//! channels (the same wire format the GTK app used with WebKit script
//! message handlers). The frontend forwards every channel it doesn't handle
//! locally to this command.
//!
//! Channel → handler mapping fills in per phase:
//! - Phase 4 (here): tableEdit / tableNavigate / tableStructure /
//!   tableSort / tableResizeColumns
//! - Phase 5: imageResize / imageMove
//! - Phase 7: commitClick / toggleHistory / toggleHistoryCollapse

use std::sync::Mutex;

use tauri::{AppHandle, Runtime, State};

use super::{image, table};
use crate::state::AppState;

#[tauri::command]
pub fn preview_message<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<AppState>>,
    channel: String,
    payload: String,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    match channel.as_str() {
        "tableEdit" => table::handle_table_edit(&app, &mut s, &payload),
        "tableNavigate" => table::handle_table_navigate(&app, &mut s, &payload),
        "tableStructure" => table::handle_table_structure(&app, &mut s, &payload),
        "tableSort" => table::handle_table_sort(&app, &mut s, &payload),
        "tableResizeColumns" => table::handle_table_resize_columns(&app, &mut s, &payload),
        "imageResize" => image::handle_image_resize(&app, &mut s, &payload),
        "imageMove" => image::handle_image_move(&app, &mut s, &payload),
        other => {
            // Unknown channels are logged, not errors — a newer preview doc
            // must never hard-fail an older shell.
            eprintln!(
                "preview_message: unhandled channel {other} ({} bytes)",
                payload.len()
            );
        }
    }
    Ok(())
}
