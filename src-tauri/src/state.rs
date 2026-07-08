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
    /// External-reload diff state: yellow change bars + hover word-diffs on
    /// the next render (consumed by it, like the GTK app).
    pub pending_changes: Option<rendermd_core::diff::PendingChanges>,
    /// Live directory watcher for the open document. Dropping it stops the
    /// watch and winds down the debounce thread.
    pub watcher: Option<notify::RecommendedWatcher>,
    /// `git log --follow` cache for the open document; None = not a repo
    /// file (the rail stays out of the preview entirely).
    pub history: Option<Vec<rendermd_core::history::Commit>>,
    pub history_visible: bool,
    pub history_collapsed: bool,
    /// View-only historical revision being browsed via the rail. The buffer
    /// of record is untouched; editing the working copy clears this.
    pub viewing_snapshot: Option<HistorySnapshot>,
}

/// A commit being viewed via the history rail.
pub struct HistorySnapshot {
    pub sha: String,
    pub text: String,
    pub parent_text: Option<String>,
    pub commit_unix_secs: i64,
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
            pending_changes: None,
            watcher: None,
            history: None,
            history_visible: true,
            history_collapsed: false,
            viewing_snapshot: None,
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

        // Snapshot browsing renders the historical text instead of the
        // working copy; re-populate the diff-vs-parent markers if a refresh
        // consumed them (theme flips etc.), matching the GTK refresh.
        let source_text = match &self.viewing_snapshot {
            Some(snap) => {
                if self.pending_changes.is_none() {
                    if let Some(parent) = &snap.parent_text {
                        let changed =
                            rendermd_core::diff::compute_changed_lines(parent, &snap.text);
                        if !changed.is_empty() {
                            self.pending_changes = Some(rendermd_core::diff::PendingChanges {
                                changed_lines: changed,
                                old_text: parent.clone(),
                                reload_ts: snap.commit_unix_secs,
                            });
                        }
                    }
                }
                snap.text.clone()
            }
            None => self.text.clone(),
        };

        // External-reload / snapshot change bars: injected into the text
        // pre-render and consumed (one-shot), matching the GTK refresh.
        // Tables are parsed from the SAME (possibly marker-injected) text so
        // their ids line up with the HTML the injector decorates.
        let text = match self.pending_changes.take() {
            Some(changes) => {
                rendermd_core::diff::inject_change_markers(&source_text, &changes, self.dark)
            }
            None => source_text,
        };

        let html =
            rendermd_core::render::render_markdown_to_html(&text, base_dir, self.dark, &title);

        let mut parsed_tables = tables::parse_tables(&text);
        for t in &mut parsed_tables {
            if let Some(snap) = self.sort_snapshots.get(&t.id) {
                t.sort_indicator = Some((snap.col, snap.direction));
            }
        }
        let html_with_tables = if parsed_tables.is_empty() {
            html
        } else {
            let injected = tables::render::inject_table_attrs(&html, &parsed_tables);
            injected.replacen(
                "</body>",
                &format!("{}\n</body>", tables::render::TABLE_EDIT_JS),
                1,
            )
        };

        // Git history rail, injected before </body> when the file lives in
        // a repo. No markup at all otherwise.
        self.preview_html = match &self.history {
            Some(commits) => {
                let viewing = self.viewing_snapshot.as_ref().map(|s| s.sha.as_str());
                let rail = rendermd_core::history::build_history_rail_html(
                    commits,
                    viewing,
                    self.history_visible,
                    self.history_collapsed,
                );
                if rail.is_empty() {
                    html_with_tables
                } else {
                    html_with_tables.replacen("</body>", &format!("{rail}\n</body>"), 1)
                }
            }
            None => html_with_tables,
        };
        self.preview_rev += 1;
    }
}
