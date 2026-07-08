//! rendermd-core — the pure-Rust markdown pipeline shared by RenderMD shells.
//!
//! No GUI, no Tauri, no I/O beyond `std::process::Command` (git) — this crate
//! must stay buildable and testable on a bare CI runner with no system
//! libraries installed.
//!
//! Modules land here in Phase 1 of the port (see plan):
//! - `render` — markdown → HTML (comrak GFM + syntect)
//! - `preprocess` — GitHub alerts + mermaid fences
//! - `emoji` — `:shortcode:` replacement
//! - `template` — HTML template + preview CSS/JS constants
//! - `diff` — external-change bars + word diffs
//! - `history` — git log/show parsing + history rail HTML
//! - `images` — image-ref lookup + offset helpers
//! - `tables` — the table subsystem (parse/model/serialize/render/paste)

pub mod diff;
pub mod emoji;
pub mod history;
pub mod images;
pub mod preprocess;
pub mod render;
pub mod tables;
pub mod template;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_nonempty() {
        assert!(!super::version().is_empty());
    }
}
