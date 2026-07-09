//! Resolve a preview-frame link click, server-side and confined.
//!
//! Threat: the preview frame runs attacker-authored script (comrak renders
//! raw HTML), so it can post a forged `linkClick` for ANY path. Letting the
//! frontend hand an arbitrary path to `openPath` (system handler) would be a
//! one-message local-file-open / execution primitive. All confinement
//! therefore lives here, authoritative, keyed off the same allowed root the
//! `/fs/` protocol uses.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum LinkResolution {
    /// A markdown file inside the allowed root — safe to open in-app.
    OpenMd { path: String },
    /// Another local file inside the allowed root — the frontend must get
    /// explicit user confirmation before handing it to the system opener.
    OpenFile { path: String },
    /// Outside the allowed root, nonexistent, or unparseable — blocked.
    Denied,
}

/// Map a preview-origin URL (`preview://…/fs/<encoded abs path>` or the
/// Windows `http://preview.localhost/fs/…` form) back to a filesystem path,
/// mirroring the protocol handler.
fn fs_url_to_path(url: &str) -> Option<PathBuf> {
    let after = url.split("/fs/").nth(1)?;
    // Strip any query/fragment the browser appended.
    let after = after.split(['?', '#']).next().unwrap_or(after);
    let decoded = percent_encoding::percent_decode_str(after).decode_utf8_lossy();
    #[cfg(windows)]
    let p = PathBuf::from(decoded.as_ref());
    #[cfg(not(windows))]
    let p = PathBuf::from(format!("/{decoded}"));
    Some(p)
}

#[tauri::command]
pub fn resolve_local_link(state: State<'_, Mutex<AppState>>, url: String) -> LinkResolution {
    let s = state.lock().unwrap();
    let root = s.allowed_fs_root.clone();
    drop(s);

    let Some(requested) = fs_url_to_path(&url) else {
        return LinkResolution::Denied;
    };
    // Same canonicalize-and-confine check the /fs/ reader uses.
    let Some(confined) = rendermd_core::fsroot::resolve_within(root.as_deref(), &requested) else {
        return LinkResolution::Denied;
    };
    let path = confined.to_string_lossy().into_owned();
    match confined
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("md") | Some("markdown") => LinkResolution::OpenMd { path },
        _ => LinkResolution::OpenFile { path },
    }
}
