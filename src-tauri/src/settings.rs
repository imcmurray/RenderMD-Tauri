//! UI settings persisted across sessions (TOML; replaces the GTK app's
//! glib::KeyFile INI). Window geometry is handled separately by
//! tauri-plugin-window-state — this file only carries app-level flags.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(default)]
pub struct Settings {
    pub history_visible: bool,
    pub history_collapsed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            history_visible: true,
            history_collapsed: false,
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rendermd").join("settings.toml"))
}

pub fn load() -> Settings {
    settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) {
    let Some(path) = settings_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(body) = toml::to_string_pretty(settings) {
        let _ = std::fs::write(path, body);
    }
}
