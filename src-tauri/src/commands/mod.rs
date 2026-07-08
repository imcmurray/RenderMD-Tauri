pub mod doc;
pub mod file;
pub mod image;
pub mod preview_msg;
pub mod table;

use serde::Serialize;

/// Snapshot of the open document, returned by open/new/get_doc.
#[derive(Serialize)]
pub struct DocInfo {
    pub path: Option<String>,
    pub text: String,
    pub mtime: Option<String>,
    pub mode: crate::state::Mode,
    pub rev: u64,
    pub dirty: bool,
}

impl DocInfo {
    pub fn from_state(s: &crate::state::AppState) -> Self {
        Self {
            path: s
                .file_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            text: s.text.clone(),
            mtime: s
                .file_path
                .as_ref()
                .map(|p| rendermd_core::history::format_mtime(p)),
            mode: s.mode,
            rev: s.preview_rev,
            dirty: s.is_modified,
        }
    }
}
