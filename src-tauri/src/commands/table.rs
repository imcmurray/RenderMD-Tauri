//! Table channel handlers — ports of the GTK `handle_table_*` methods.
//!
//! Same shape as the originals: parse the tab-delimited payload, re-parse
//! tables from the buffer of record, mutate a shadow `String` through the
//! tables model to get an `EditDelta`, then (a) adopt the shadow as the new
//! text and (b) tell the editor to apply the same minimal patch as a single
//! undoable CodeMirror transaction (`doc-patched`, UTF-16 offsets).

use rendermd_core::images::byte_to_utf16_offset;
use rendermd_core::tables::{
    self,
    model::{Alignment, NavDirection, SortDirection},
    EditDelta,
};
use tauri::{AppHandle, Emitter, Runtime};

use crate::state::{AppState, TableSortSnapshot};

pub fn toast<R: Runtime>(app: &AppHandle<R>, text: &str) {
    let _ = app.emit("toast", serde_json::json!({ "text": text }));
}

/// Re-render and notify the preview, carrying the pending focus cell (if
/// any) so the frame can re-enter cell editing after the reload.
pub fn refresh<R: Runtime>(app: &AppHandle<R>, s: &mut AppState) {
    s.render_preview();
    let focus = s
        .pending_focus_cell
        .take()
        .map(|(tid, r, c)| serde_json::json!({ "tid": tid, "r": r, "c": c }));
    let _ = app.emit(
        "preview-updated",
        serde_json::json!({ "rev": s.preview_rev, "focusCell": focus }),
    );
}

/// Adopt the patched shadow text and mirror the minimal edit into the
/// CodeMirror doc (the port of the GTK `apply_buffer_patch`). Offsets cross
/// the IPC boundary as UTF-16 code units — CM positions.
fn apply_text_patch<R: Runtime>(
    app: &AppHandle<R>,
    s: &mut AppState,
    old_text: &str,
    new_text: String,
    delta: &EditDelta,
) {
    let from = byte_to_utf16_offset(old_text, delta.patched_range.start);
    let to = byte_to_utf16_offset(old_text, delta.patched_range.end);
    let insert = &new_text[delta.new_range.clone()];
    let _ = app.emit(
        "doc-patched",
        serde_json::json!({ "from": from, "to": to, "insert": insert }),
    );
    s.text = new_text;
    s.is_modified = true;
    let _ = app.emit("title-changed", serde_json::json!({ "dirty": true }));
}

/// Shared preamble: parse the table id'd `table_id` out of the current text.
/// On a stale id (doc changed between render and click), toast + re-render
/// to resync the preview, mirroring the GTK behavior.
macro_rules! find_table {
    ($app:expr, $s:expr, $tables:expr, $table_id:expr) => {
        match $tables.iter_mut().find(|t| t.id == $table_id) {
            Some(t) => t,
            None => {
                toast($app, "Couldn't locate that table — refreshing");
                refresh($app, $s);
                return;
            }
        }
    };
}

/// `table_id\trow\tcol\tcontent` — click-to-edit cell commit. Content may
/// contain literal newlines (soft breaks); `splitn(4)` keeps them intact.
pub fn handle_table_edit<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, message: &str) {
    let mut parts = message.splitn(4, '\t');
    let table_id: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let row: i32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let col: usize = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let content = parts.next().unwrap_or("");
    if table_id == 0 {
        return;
    }
    // A cell edit invalidates any pre-sort snapshot — restoring the original
    // order would silently drop this edit.
    s.sort_snapshots.remove(&table_id);

    let buffer_text = s.text.clone();
    let mut tables_vec = tables::parse_tables(&buffer_text);
    let table = find_table!(app, s, tables_vec, table_id);

    let mut shadow = buffer_text.clone();
    let delta = match table.update_cell(row, col, content, &mut shadow) {
        Ok(d) => d,
        Err(e) => {
            toast(app, &format!("Edit failed: {e}"));
            return;
        }
    };
    if delta.patched_range.start == delta.patched_range.end && delta.byte_delta == 0 {
        return; // no-op edit
    }
    apply_text_patch(app, s, &buffer_text, shadow, &delta);
    refresh(app, s);
}

/// `table_id\trow\tcol\tdirection` (next|prev|down|up) — Tab/Enter
/// navigation. May create a row; always lands focus on the target cell.
pub fn handle_table_navigate<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, message: &str) {
    let mut parts = message.splitn(4, '\t');
    let table_id: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let row: i32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let col: usize = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let direction = match parts.next().unwrap_or("") {
        "next" => NavDirection::Next,
        "prev" => NavDirection::Prev,
        "down" => NavDirection::Down,
        "up" => NavDirection::Up,
        _ => return,
    };
    if table_id == 0 {
        return;
    }

    let buffer_text = s.text.clone();
    let mut tables_vec = tables::parse_tables(&buffer_text);
    let table = find_table!(app, s, tables_vec, table_id);

    let target = table.navigate(row, col, direction);

    if target.created_row {
        let mut shadow = buffer_text.clone();
        match table.insert_empty_row(target.row as usize, &mut shadow) {
            Ok(delta) => apply_text_patch(app, s, &buffer_text, shadow, &delta),
            Err(e) => {
                toast(app, &format!("Couldn't add row: {e}"));
                return;
            }
        }
    }

    s.pending_focus_cell = Some((table_id, target.row, target.col));
    refresh(app, s);
}

/// `table_id\trow\tcol\top` — structural edits + alignment + reformat.
pub fn handle_table_structure<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, message: &str) {
    let mut parts = message.splitn(4, '\t');
    let table_id: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let row: i32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let col: usize = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let op = parts.next().unwrap_or("");
    if table_id == 0 || op.is_empty() {
        return;
    }

    let buffer_text = s.text.clone();
    let mut tables_vec = tables::parse_tables(&buffer_text);

    // Row/column ops change the row set; alignment + reformat don't.
    // Invalidate the sort snapshot only for the former, so "Off" still
    // works after an alignment tweak.
    if matches!(
        op,
        "row-above" | "row-below" | "col-left" | "col-right" | "row-delete" | "col-delete"
    ) {
        s.sort_snapshots.remove(&table_id);
    }

    let table = find_table!(app, s, tables_vec, table_id);

    // `row == -1` is the header. "row-above"/"row-delete" on it are refused;
    // "row-below" inserts at the start of the body.
    let body_row = row.max(0) as usize;
    let mut shadow = buffer_text.clone();
    let result = match op {
        "row-above" => {
            if row == -1 {
                toast(app, "Can't insert a row above the header");
                return;
            }
            table.insert_empty_row(body_row, &mut shadow)
        }
        "row-below" => {
            let at = if row == -1 { 0 } else { body_row + 1 };
            table.insert_empty_row(at, &mut shadow)
        }
        "col-left" => table.insert_column(col, Alignment::None, &mut shadow),
        "col-right" => table.insert_column(col + 1, Alignment::None, &mut shadow),
        "row-delete" => {
            if row == -1 {
                toast(app, "Can't delete the header row");
                return;
            }
            table.delete_row(body_row, &mut shadow)
        }
        "col-delete" => table.delete_column(col, &mut shadow),
        "align-left" => table.set_column_alignment(col, Alignment::Left, &mut shadow),
        "align-center" => table.set_column_alignment(col, Alignment::Center, &mut shadow),
        "align-right" => table.set_column_alignment(col, Alignment::Right, &mut shadow),
        "align-none" => table.set_column_alignment(col, Alignment::None, &mut shadow),
        "reformat" => table.reformat_pretty(&mut shadow),
        _ => return,
    };

    let delta = match result {
        Ok(d) => d,
        Err(e) => {
            toast(app, &format!("Operation failed: {e}"));
            return;
        }
    };
    // set_column_alignment signals a no-op (already-active alignment) with
    // an empty patched_range — skip the patch and refresh entirely.
    if delta.patched_range.is_empty() && delta.byte_delta == 0 {
        return;
    }

    apply_text_patch(app, s, &buffer_text, shadow, &delta);

    // Post-op focus target, using the model's post-op sizes.
    let n_cols = table.alignments.len();
    let n_body = table.rows.len() as i32;
    let focus: Option<(i32, usize)> = match op {
        "row-above" => Some((row + 1, col)),
        "row-below" => Some((if row == -1 { 0 } else { row }, col)),
        "col-left" => Some((row, col + 1)),
        "col-right" => Some((row, col)),
        "row-delete" => {
            if n_body == 0 {
                Some((-1, col))
            } else {
                Some((row.min(n_body - 1), col))
            }
        }
        "col-delete" => {
            if n_cols == 0 {
                None
            } else {
                Some((row, col.min(n_cols - 1)))
            }
        }
        "align-left" | "align-center" | "align-right" | "align-none" => Some((row, col)),
        "reformat" => {
            toast(
                app,
                &format!(
                    "Table reformatted to pretty style ({} column{})",
                    n_cols,
                    if n_cols == 1 { "" } else { "s" }
                ),
            );
            Some((row, col))
        }
        _ => None,
    };
    if let Some((fr, fc)) = focus {
        s.pending_focus_cell = Some((table_id, fr, fc));
    }
    refresh(app, s);
}

/// `table_id\tcol\tdirection` (asc|desc|off) — tri-state column sort.
pub fn handle_table_sort<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, message: &str) {
    let mut parts = message.splitn(3, '\t');
    let table_id: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let col: usize = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let direction = match parts.next().unwrap_or("") {
        "asc" => SortDirection::Ascending,
        "desc" => SortDirection::Descending,
        "off" => SortDirection::None,
        _ => return,
    };
    if table_id == 0 {
        return;
    }

    let buffer_text = s.text.clone();
    let mut tables_vec = tables::parse_tables(&buffer_text);

    // Resolve the new row order BEFORE borrowing the table mutably, since
    // it needs snapshot-map access.
    let new_rows = {
        let Some(table) = tables_vec.iter().find(|t| t.id == table_id) else {
            toast(app, "Couldn't locate that table — refreshing");
            refresh(app, s);
            return;
        };
        if col >= table.alignments.len() {
            toast(app, "Column out of range for sort");
            return;
        }
        match direction {
            SortDirection::None => match s.sort_snapshots.remove(&table_id) {
                Some(snap) => {
                    toast(
                        app,
                        &format!(
                            "Sort cleared — column {} restored to original order",
                            snap.col + 1
                        ),
                    );
                    snap.original_rows
                }
                None => {
                    toast(
                        app,
                        "Original row order not tracked (cleared by a recent edit)",
                    );
                    return;
                }
            },
            SortDirection::Ascending | SortDirection::Descending => {
                // Sort from the snapshot baseline when one exists, so Desc
                // is "desc of original", stable for ties — not "reverse of
                // asc".
                let baseline = s
                    .sort_snapshots
                    .get(&table_id)
                    .map(|snap| snap.original_rows.clone())
                    .unwrap_or_else(|| table.rows.clone());
                let sorted = tables::model::sort_rows(baseline.clone(), col, direction);
                s.sort_snapshots.insert(
                    table_id,
                    TableSortSnapshot {
                        original_rows: baseline,
                        col,
                        direction,
                    },
                );
                let dir_label = if matches!(direction, SortDirection::Ascending) {
                    "ascending"
                } else {
                    "descending"
                };
                toast(app, &format!("Sorted by column {} ({dir_label})", col + 1));
                sorted
            }
        }
    };

    let table = find_table!(app, s, tables_vec, table_id);
    let mut shadow = buffer_text.clone();
    let delta = match table.replace_rows(new_rows, &mut shadow) {
        Ok(d) => d,
        Err(e) => {
            toast(app, &format!("Sort failed: {e}"));
            return;
        }
    };
    apply_text_patch(app, s, &buffer_text, shadow, &delta);

    // Stay on the header cell the user clicked.
    s.pending_focus_cell = Some((table_id, -1, col));
    refresh(app, s);
}

/// `table_id\twidths` — commit column widths from the resize-handle drag.
/// `widths` is comma-separated with empties for unset columns (`180,,120`).
pub fn handle_table_resize_columns<R: Runtime>(
    app: &AppHandle<R>,
    s: &mut AppState,
    message: &str,
) {
    let mut parts = message.splitn(2, '\t');
    let table_id: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let widths_csv = parts.next().unwrap_or("");
    if table_id == 0 {
        return;
    }

    let widths: Vec<Option<u32>> = widths_csv
        .split(',')
        .map(|p| {
            let p = p.trim();
            if p.is_empty() {
                None
            } else {
                p.parse::<u32>().ok()
            }
        })
        .collect();

    let buffer_text = s.text.clone();
    let mut tables_vec = tables::parse_tables(&buffer_text);
    let table = find_table!(app, s, tables_vec, table_id);

    if widths.len() != table.alignments.len() {
        toast(
            app,
            &format!(
                "Resize ignored: got {} widths, table has {} columns",
                widths.len(),
                table.alignments.len()
            ),
        );
        return;
    }
    if widths == table.column_widths {
        return; // no-op drag
    }

    let mut shadow = buffer_text.clone();
    let delta = match table.set_column_widths(widths, &mut shadow) {
        Ok(d) => d,
        Err(e) => {
            toast(app, &format!("Resize failed: {e}"));
            return;
        }
    };
    apply_text_patch(app, s, &buffer_text, shadow, &delta);
    refresh(app, s);
}

/// Smart paste: detect TSV/CSV/HTML/GFM table content and convert to a
/// pretty GFM table. Returns None when the clipboard isn't table-shaped
/// (caller falls back to a normal text paste).
#[derive(serde::Serialize)]
pub struct TablePasteResult {
    pub markdown: String,
    pub body_rows: usize,
    pub cols: usize,
    pub origin: &'static str,
}

#[tauri::command]
pub fn convert_table_paste(text: Option<String>, html: Option<String>) -> Option<TablePasteResult> {
    let detected = tables::detect_table_paste(text.as_deref(), html.as_deref());
    if matches!(detected, tables::TablePaste::None) {
        return None;
    }
    let markdown = tables::paste_to_gfm(&detected, tables::TableStyle::Pretty)?;
    let (rows, cols) = detected.shape()?;
    let origin = match detected {
        tables::TablePaste::Tsv { .. } => "TSV",
        tables::TablePaste::Csv { .. } => "CSV",
        tables::TablePaste::Gfm { .. } => "Markdown",
        tables::TablePaste::Html { .. } => "HTML table",
        tables::TablePaste::None => return None,
    };
    Some(TablePasteResult {
        markdown,
        body_rows: rows.saturating_sub(1),
        cols,
        origin,
    })
}
