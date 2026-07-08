//! Application state — the Rust side owns the buffer of record.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use rendermd_core::tables::{
    self,
    model::{Cell, SortDirection},
};
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Preview,
    Edit,
}

/// Pre-sort row snapshot so the tri-state sort control (Asc → Desc → Off)
/// can restore the original row order. Same lifecycle as the GTK app:
/// invalidated by any cell edit or row-set-changing structural op.
pub struct TableSortSnapshot {
    pub original_rows: Vec<Vec<Cell>>,
    pub col: usize,
    pub direction: SortDirection,
}

pub struct AppState {
    pub file_path: Option<PathBuf>,
    /// The buffer of record. CodeMirror's document is a mirror of this.
    pub text: String,
    pub is_modified: bool,
    pub mode: Mode,
    /// Last fully-rendered preview document, served at `preview://localhost/doc.html`.
    pub preview_html: String,
    /// Bumped on every re-render; the frontend busts the iframe cache with it.
    pub preview_rev: u64,
    pub dark: bool,
    /// Stamped immediately before our own atomic-save rename so the file
    /// watcher (Phase 6) can suppress self-generated change events.
    pub last_self_write: Instant,
    /// Tri-state sort baselines, keyed by stable table id.
    pub sort_snapshots: HashMap<u64, TableSortSnapshot>,
    /// Cell to focus after the next preview refresh (Tab/Enter navigation,
    /// structural ops). Consumed by the preview-updated event.
    pub pending_focus_cell: Option<(u64, i32, usize)>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            file_path: None,
            text: String::new(),
            is_modified: false,
            // Empty document opens in edit mode (loaded files flip to preview).
            mode: Mode::Edit,
            preview_html: String::new(),
            preview_rev: 0,
            dark: false,
            last_self_write: Instant::now(),
            sort_snapshots: HashMap::new(),
            pending_focus_cell: None,
        }
    }
}

impl AppState {
    /// Re-render `preview_html` from the current text and bump the revision.
    ///
    /// Pipeline (a port of the GTK `refresh_preview`): markdown → HTML, then
    /// table post-processing — parse tables from source, hydrate transient
    /// sort indicators from the snapshot map, inject `data-*` attributes and
    /// the click-to-edit JS. No-op when the doc has no tables.
    pub fn render_preview(&mut self) {
        let title = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let base_dir = self.file_path.as_ref().and_then(|p| p.parent());
        let html =
            rendermd_core::render::render_markdown_to_html(&self.text, base_dir, self.dark, &title);

        let mut parsed_tables = tables::parse_tables(&self.text);
        for t in &mut parsed_tables {
            if let Some(snap) = self.sort_snapshots.get(&t.id) {
                t.sort_indicator = Some((snap.col, snap.direction));
            }
        }
        self.preview_html = if parsed_tables.is_empty() {
            html
        } else {
            let injected = tables::render::inject_table_attrs(&html, &parsed_tables);
            injected.replacen(
                "</body>",
                &format!("{}\n</body>", tables::render::TABLE_EDIT_JS),
                1,
            )
        };
        self.preview_rev += 1;
    }
}
