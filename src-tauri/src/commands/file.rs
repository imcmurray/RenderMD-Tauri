//! File lifecycle commands: open, new, save, save-as, export.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Runtime, State};

use super::DocInfo;
use crate::state::{AppState, Mode};

/// Atomic write: temp file in the destination directory, then a rename over
/// the target. `tempfile::persist` handles Windows replace semantics. The
/// self-write stamp MUST land before the rename so the watcher (Phase 6)
/// suppresses the resulting filesystem event.
fn atomic_write(state: &mut AppState, path: &Path, text: &str) -> Result<(), String> {
    use std::io::Write;

    let dir = path.parent().ok_or("destination has no parent directory")?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    tmp.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    state.last_self_write = Instant::now();
    tmp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn emit_preview_updated<R: Runtime>(app: &AppHandle<R>, rev: u64) {
    let _ = app.emit("preview-updated", serde_json::json!({ "rev": rev }));
}

pub fn load_into_state(state: &mut AppState, path: PathBuf) -> Result<(), String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("open {}: {e}", path.display()))?;
    state.text = String::from_utf8_lossy(&bytes).into_owned();
    state.history = rendermd_core::history::fetch_git_history(&path);
    state.remote_url = rendermd_core::history::remote_web_url(&path);
    state.allowed_fs_root = rendermd_core::fsroot::allowed_root(&path);
    state.showing_welcome = false;
    state.viewing_snapshot = None;
    state.pending_changes = None;
    state.sort_snapshots.clear();
    state.file_path = Some(path);
    state.is_modified = false;
    // Loaded files open in preview mode; empty documents in edit mode.
    state.mode = Mode::Preview;
    state.render_preview();
    Ok(())
}

#[tauri::command]
pub fn open_file<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<AppState>>,
    path: String,
) -> Result<DocInfo, String> {
    let path = PathBuf::from(path);
    let mut s = state.lock().unwrap();
    load_into_state(&mut s, path.clone())?;
    emit_preview_updated(&app, s.preview_rev);
    let info = DocInfo::from_state(&s);
    drop(s); // start_watching locks the state itself
    crate::watcher::start_watching(&app, &path);
    Ok(info)
}

#[tauri::command]
pub fn new_file<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<AppState>>,
) -> Result<DocInfo, String> {
    let mut s = state.lock().unwrap();
    *s = AppState {
        dark: s.dark,
        ..AppState::default()
    };
    // Dropping the old state above also dropped its watcher.
    s.render_preview();
    emit_preview_updated(&app, s.preview_rev);
    Ok(DocInfo::from_state(&s))
}

/// Save to the current path. Fails with "no-path" when the document has
/// never been saved — the frontend reacts by running the save-as dialog.
#[tauri::command]
pub fn save_file(state: State<'_, Mutex<AppState>>, text: String) -> Result<DocInfo, String> {
    let mut s = state.lock().unwrap();
    let path = s.file_path.clone().ok_or("no-path")?;
    atomic_write(&mut s, &path, &text)?;
    s.text = text;
    s.is_modified = false;
    Ok(DocInfo::from_state(&s))
}

/// Export the rendered document as standalone HTML. The in-app render uses
/// root-relative `/fs/...` URLs that only the preview protocol understands,
/// so exports rewrite the base href to `file://` and point the mermaid
/// script at a bundle extracted to the user cache dir (matching how the GTK
/// app's exports worked: viewable locally in any browser).
#[tauri::command]
pub fn export_html(state: State<'_, Mutex<AppState>>, dest: String) -> Result<(), String> {
    use rendermd_core::render::{fs_base_href, PATH_SEGMENT_ENCODE};
    use rendermd_core::template::MERMAID_ASSET_PATH;

    let mut s = state.lock().unwrap();
    let title = s
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let base_dir = s
        .file_path
        .as_ref()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    // Fresh plain render: no table-edit JS, no history rail.
    let mut html = rendermd_core::render::render_markdown_to_html(
        &s.text,
        base_dir.as_deref(),
        s.dark,
        &title,
    );

    if let Some(dir) = &base_dir {
        let fs_base = fs_base_href(dir);
        let dir_str = dir.to_string_lossy().replace('\\', "/");
        let encoded =
            percent_encoding::utf8_percent_encode(&dir_str, PATH_SEGMENT_ENCODE).to_string();
        let file_base = if encoded.starts_with('/') {
            format!("file://{encoded}/")
        } else {
            format!("file:///{encoded}/")
        };
        html = html.replace(
            &format!("<base href=\"{fs_base}\">"),
            &format!("<base href=\"{file_base}\">"),
        );
    }

    // Mermaid: extract the bundle once to the cache dir and reference it.
    if html.contains(MERMAID_ASSET_PATH) {
        let cache = dirs::cache_dir().ok_or("no cache dir")?.join("rendermd");
        std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
        let bundle = cache.join("mermaid.min.js");
        if !bundle.exists() {
            std::fs::write(&bundle, crate::preview_protocol::mermaid_bundle())
                .map_err(|e| e.to_string())?;
        }
        let bundle_str = bundle.to_string_lossy().replace('\\', "/");
        let encoded =
            percent_encoding::utf8_percent_encode(&bundle_str, PATH_SEGMENT_ENCODE).to_string();
        let uri = if encoded.starts_with('/') {
            format!("file://{encoded}")
        } else {
            format!("file:///{encoded}")
        };
        html = html.replace(
            &format!("src=\"{MERMAID_ASSET_PATH}\""),
            &format!("src=\"{uri}\""),
        );
    }

    // Same atomic tmp+rename discipline as document saves.
    let mut dest = PathBuf::from(dest);
    if dest.extension().is_none() {
        dest.set_extension("html");
    }
    atomic_write(&mut s, &dest, &html)?;
    Ok(())
}

#[tauri::command]
pub fn save_file_as<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<AppState>>,
    path: String,
    text: String,
) -> Result<DocInfo, String> {
    let mut s = state.lock().unwrap();
    // Force the .md extension the way the GTK save dialog did.
    let mut path = PathBuf::from(path);
    if path.extension().is_none() {
        path.set_extension("md");
    }
    atomic_write(&mut s, &path, &text)?;
    s.file_path = Some(path.clone());
    s.text = text;
    s.is_modified = false;
    s.render_preview(); // title + base dir changed
    emit_preview_updated(&app, s.preview_rev);
    let info = DocInfo::from_state(&s);
    drop(s);
    crate::watcher::start_watching(&app, &path);
    Ok(info)
}
