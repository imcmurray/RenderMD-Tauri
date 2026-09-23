//! Live Omarchy theme: read `colors.toml` and keep the shell + preview in
//! sync when the user switches themes.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use notify::Watcher;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::state::AppState;

const COLORS_REL: &str = ".local/state/omarchy/current/theme/colors.toml";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub dark: bool,
    pub background: String,
    pub darker_background: String,
    pub lighter_background: String,
    pub foreground: String,
    pub dark_foreground: String,
    pub accent: String,
    pub selection: String,
    pub muted: String,
    pub red: String,
    pub yellow: String,
    pub green: String,
    pub cyan: String,
    pub blue: String,
    pub magenta: String,
}

#[derive(Deserialize, Default)]
struct ColorsFile {
    mode: Option<String>,
    accent: Option<String>,
    selection: Option<String>,
    muted: Option<String>,
    background: Option<String>,
    dark_background: Option<String>,
    darker_background: Option<String>,
    lighter_background: Option<String>,
    foreground: Option<String>,
    dark_foreground: Option<String>,
    red: Option<String>,
    yellow: Option<String>,
    green: Option<String>,
    cyan: Option<String>,
    blue: Option<String>,
    magenta: Option<String>,
}

fn colors_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(COLORS_REL))
}

fn pick(value: Option<String>, fallback: &str) -> String {
    match value {
        Some(v) if !v.is_empty() => v,
        _ => fallback.to_string(),
    }
}

impl Palette {
    pub fn preview_css(&self) -> String {
        format!(
            r#"
:root {{
  --bg: {bg};
  --fg: {fg};
  --muted: {muted};
  --accent: {accent};
  --border: {muted};
  --code-bg: {code};
  --code-fg: {fg};
  --kbd-bg: {code};
  --table-stripe: {code};
  --quote-bg: {code};
  --quote-bar: {accent};
  --alert-note: {blue};
  --alert-tip: {green};
  --alert-important: {magenta};
  --alert-warning: {yellow};
  --alert-caution: {red};
  --change: {yellow};
}}
body {{
  background: {bg};
  color: {fg};
  font-family: "JetBrainsMono Nerd Font", "JetBrains Mono", "iA Writer Mono S",
               ui-sans-serif, system-ui, sans-serif;
}}
"#,
            bg = self.background,
            fg = self.foreground,
            muted = self.muted,
            accent = self.accent,
            code = self.lighter_background,
            blue = self.blue,
            green = self.green,
            magenta = self.magenta,
            yellow = self.yellow,
            red = self.red,
        )
    }
}

pub fn load_palette() -> Option<Palette> {
    let path = colors_path()?;
    load_palette_from(&path)
}

fn load_palette_from(path: &Path) -> Option<Palette> {
    let raw = std::fs::read_to_string(path).ok()?;
    let file: ColorsFile = toml::from_str(&raw).ok()?;

    let background = pick(file.background, "#101010");
    let foreground = pick(file.foreground, "#eeeeee");
    let accent = pick(file.accent, "#5584aa");
    let muted = pick(file.muted.clone(), "#6a6565");
    let dark_foreground = pick(file.dark_foreground, &muted);

    let mode = file.mode.unwrap_or_default();
    let dark = match mode.as_str() {
        "light" => false,
        "dark" => true,
        _ => luminance_is_dark(&background),
    };

    Some(Palette {
        dark,
        darker_background: pick(file.darker_background.or(file.dark_background), &background),
        lighter_background: pick(file.lighter_background, &background),
        background,
        foreground,
        dark_foreground,
        accent,
        selection: pick(file.selection, "#186a9a"),
        muted,
        red: pick(file.red, "#c41e3a"),
        yellow: pick(file.yellow, "#d4c4b0"),
        green: pick(file.green, "#6a6565"),
        cyan: pick(file.cyan, "#a09c9b"),
        blue: pick(file.blue, "#605e5e"),
        magenta: pick(file.magenta, "#8b3a45"),
    })
}

fn luminance_is_dark(hex: &str) -> bool {
    let h = hex.trim().trim_start_matches('#');
    if h.len() < 6 {
        return true;
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0) as f32;
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0) as f32;
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0) as f32;
    (0.299 * r + 0.587 * g + 0.114 * b) / 255.0 < 0.5
}

pub fn apply_to_state(state: &mut AppState, palette: &Palette) {
    state.dark = palette.dark;
    state.omarchy_css = palette.preview_css();
}

/// Watch Omarchy's current-theme files and push updates into the running app.
pub fn start_watching<R: Runtime>(app: &AppHandle<R>) {
    let Some(path) = colors_path() else { return };
    let Some(theme_dir) = path.parent().map(|p| p.to_path_buf()) else {
        return;
    };
    let Some(current_dir) = theme_dir.parent().map(|p| p.to_path_buf()) else {
        return;
    };

    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher =
        match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if res.is_ok() {
                let _ = tx.send(());
            }
        }) {
            Ok(w) => w,
            Err(_) => return,
        };

    for dir in [&current_dir, &theme_dir] {
        let _ = watcher.watch(dir, notify::RecursiveMode::NonRecursive);
    }
    if path.exists() {
        let _ = watcher.watch(&path, notify::RecursiveMode::NonRecursive);
    }

    let thread_app = app.clone();
    std::thread::spawn(move || {
        // Keep the watcher alive for the process lifetime.
        let _watcher = watcher;
        let app = thread_app;
        while rx.recv().is_ok() {
            loop {
                std::thread::sleep(Duration::from_millis(80));
                if rx.try_recv().is_err() {
                    break;
                }
                while rx.try_recv().is_ok() {}
            }
            let Some(palette) = load_palette() else {
                continue;
            };
            {
                let state = app.state::<Mutex<AppState>>();
                let mut s = state.lock().unwrap();
                apply_to_state(&mut s, &palette);
                s.render_preview();
                let rev = s.preview_rev;
                drop(s);
                let _ = app.emit("omarchy-theme", &palette);
                let _ = app.emit("preview-updated", serde_json::json!({ "rev": rev }));
            }
        }
    });
}

#[tauri::command]
pub fn get_omarchy_theme() -> Option<Palette> {
    load_palette()
}
