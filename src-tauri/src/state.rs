//! Application state — the Rust side owns the buffer of record.

use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Preview,
    Edit,
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
        }
    }
}

impl AppState {
    /// Re-render `preview_html` from the current text and bump the revision.
    pub fn render_preview(&mut self) {
        let title = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let base_dir = self.file_path.as_ref().and_then(|p| p.parent());
        self.preview_html =
            rendermd_core::render::render_markdown_to_html(&self.text, base_dir, self.dark, &title);
        self.preview_rev += 1;
    }
}
