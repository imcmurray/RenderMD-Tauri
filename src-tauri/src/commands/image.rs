//! Image handlers — ports of the GTK image click/resize/move/paste flows.
//!
//! Buffer edits go through the same doc-patched contract as tables: mutate
//! `state.text`, emit UTF-16 changes for CodeMirror to apply as one
//! undoable transaction. The multi-change form (`changes: [...]`) is used
//! by image moves (delete + reinsert in one step); positions all reference
//! the pre-transaction document, matching CM semantics.

use std::path::PathBuf;
use std::sync::Mutex;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use rendermd_core::images::{
    build_image_markup, byte_to_utf16_offset, char_to_byte_offset, find_image_ref,
};
use tauri::{AppHandle, Emitter, Runtime, State};

use super::table::{refresh, toast};
use crate::state::AppState;

/// Matches `glib::Uri::escape_string(s, Some("/-._~"), false)`: RFC 3986
/// unreserved characters plus `/` pass through, everything else is encoded.
/// Keeps pasted-image paths safe inside markdown's `()`.
const IMAGE_PATH_ENCODE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

#[derive(serde::Serialize)]
struct Change {
    from: usize,
    to: usize,
    insert: String,
}

fn emit_changes<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, changes: Vec<Change>) {
    let _ = app.emit("doc-patched", serde_json::json!({ "changes": changes }));
    s.is_modified = true;
    let _ = app.emit("title-changed", serde_json::json!({ "dirty": true }));
}

/// Rewrite (or remove) an image reference in the source. Shared by the
/// preview's resize-handle drag and the frontend's image-options dialog.
pub fn apply_image_change<R: Runtime>(
    app: &AppHandle<R>,
    s: &mut AppState,
    src: &str,
    new_width: Option<&str>,
    new_alt: &str,
    remove: bool,
) {
    let text = s.text.clone();
    let Some((char_start, char_len)) = find_image_ref(&text, src) else {
        toast(app, "Couldn't locate that image in the source");
        return;
    };
    let byte_start = char_to_byte_offset(&text, char_start);
    let byte_end = char_to_byte_offset(&text, char_start + char_len);
    let replacement = if remove {
        String::new()
    } else {
        build_image_markup(src, new_width, new_alt)
    };

    let mut new_text = text.clone();
    new_text.replace_range(byte_start..byte_end, &replacement);

    let change = Change {
        from: byte_to_utf16_offset(&text, byte_start),
        to: byte_to_utf16_offset(&text, byte_end),
        insert: replacement,
    };
    s.text = new_text;
    emit_changes(app, s, vec![change]);
    refresh(app, s);
}

/// `src\twidth\talt` — resize-handle drag commit from the preview.
pub fn handle_image_resize<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, message: &str) {
    let mut parts = message.splitn(3, '\t');
    let src = parts.next().unwrap_or("");
    let width = parts.next().unwrap_or("").trim();
    let alt = parts.next().unwrap_or("");
    if src.is_empty() || width.is_empty() {
        return;
    }
    apply_image_change(app, s, src, Some(width), alt, false);
}

/// `src\ttarget_src` — drag-to-reorder. Insert the dragged image immediately
/// before `target_src` (empty target = append at end), always as its own
/// block-level paragraph. Single undo step via the multi-change transaction.
pub fn handle_image_move<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, message: &str) {
    let mut parts = message.splitn(2, '\t');
    let src = parts.next().unwrap_or("");
    let target_src = parts.next().unwrap_or("");
    if src.is_empty() {
        return;
    }

    let text = s.text.clone();
    let Some((char_start, char_len)) = find_image_ref(&text, src) else {
        toast(app, "Couldn't locate that image in the source");
        refresh(app, s);
        return;
    };
    let del_start = char_to_byte_offset(&text, char_start);
    let del_end = char_to_byte_offset(&text, char_start + char_len);
    let image_expr = text[del_start..del_end].to_string();

    // Work in the deleted-state text to find the insertion point + padding
    // (mirrors the GTK two-step edit), then map the position back to
    // original coordinates for the single CM transaction.
    let mut modified = text.clone();
    modified.replace_range(del_start..del_end, "");

    let insert_char = if target_src.is_empty() {
        modified.chars().count()
    } else {
        match find_image_ref(&modified, target_src) {
            Some((c_off, _)) => c_off,
            None => modified.chars().count(),
        }
    };
    let byte_idx = char_to_byte_offset(&modified, insert_char);
    let before = &modified[..byte_idx];
    let after = &modified[byte_idx..];
    let prefix = if before.is_empty() || before.ends_with("\n\n") {
        ""
    } else if before.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let suffix = if after.is_empty() || after.starts_with("\n\n") {
        ""
    } else if after.starts_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let insertion = format!("{prefix}{image_expr}{suffix}");

    // Final text.
    modified.insert_str(byte_idx, &insertion);

    // Map the insertion point back to pre-delete coordinates.
    let orig_insert_byte = if byte_idx >= del_start {
        byte_idx + (del_end - del_start)
    } else {
        byte_idx
    };
    let insert_utf16 = byte_to_utf16_offset(&text, orig_insert_byte);

    let changes = vec![
        Change {
            from: byte_to_utf16_offset(&text, del_start),
            to: byte_to_utf16_offset(&text, del_end),
            insert: String::new(),
        },
        Change {
            from: insert_utf16,
            to: insert_utf16,
            insert: insertion,
        },
    ];
    s.text = modified;
    emit_changes(app, s, changes);
    refresh(app, s);
}

/// Image-options dialog commit (alt/width edit or removal). Invoked
/// directly by the frontend after its imageClick dialog.
#[tauri::command]
pub fn image_change<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<AppState>>,
    src: String,
    width: Option<String>,
    alt: String,
    remove: bool,
) {
    let mut s = state.lock().unwrap();
    apply_image_change(&app, &mut s, &src, width.as_deref(), &alt, remove);
}

/// Save clipboard-pasted PNG bytes beside the document
/// (`<stem>-assets/paste-<stamp>.png`) and return the markdown reference to
/// insert. Raw request body = the PNG bytes (no JSON round-trip).
#[tauri::command]
pub fn paste_image(
    state: State<'_, Mutex<AppState>>,
    request: tauri::ipc::Request<'_>,
) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected raw image bytes".into());
    };
    if bytes.is_empty() {
        return Err("empty image payload".into());
    }

    let s = state.lock().unwrap();
    let Some(path) = s.file_path.clone() else {
        return Err("Save the document first to paste images".into());
    };
    drop(s);

    let parent: PathBuf = path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or("document has no parent directory")?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "doc".into());
    let assets_dir_name = format!("{stem}-assets");
    let assets_dir = parent.join(&assets_dir_name);
    std::fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = format!("paste-{stamp}.png");
    std::fs::write(assets_dir.join(&filename), bytes).map_err(|e| e.to_string())?;

    let rel_path = format!("{assets_dir_name}/{filename}");
    let encoded = utf8_percent_encode(&rel_path, IMAGE_PATH_ENCODE).to_string();
    Ok(format!("![]({encoded})"))
}
