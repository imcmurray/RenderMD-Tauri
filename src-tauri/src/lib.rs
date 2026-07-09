//! RenderMD Tauri shell.

mod commands;
mod preview_protocol;
mod settings;
mod state;
mod watcher;

use std::sync::Mutex;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK's DMA-BUF renderer crashes the Wayland connection on some
    // driver/compositor combinations (GDK "Error 71 (Protocol error)
    // dispatching to Wayland display", verified on Arch + Budgie/Wayland).
    // Disable it unless the user has expressed an opinion. Same only-if-unset
    // pattern the GTK app used for GSK_RENDERER.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: called before the app starts any other threads.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    let ui_settings = settings::load();
    let initial_state = AppState {
        history_visible: ui_settings.history_visible,
        history_collapsed: ui_settings.history_collapsed,
        ..AppState::default()
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(Mutex::new(initial_state))
        .register_uri_scheme_protocol("preview", |ctx, request| {
            preview_protocol::handle(ctx, request)
        })
        .setup(|app| {
            // `rendermd file.md` — load a CLI argument before the window
            // renders so get_doc() returns it at boot. With no file, show
            // the welcome page in preview mode.
            let mut loaded = false;
            if let Some(arg) = std::env::args().nth(1) {
                let path = std::path::Path::new(&arg);
                if path.is_file() {
                    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
                    let state = app.state::<Mutex<AppState>>();
                    let mut s = state.lock().unwrap();
                    match commands::file::load_into_state(&mut s, abs.clone()) {
                        Ok(()) => {
                            loaded = true;
                            drop(s);
                            watcher::start_watching(app.handle(), &abs);
                        }
                        Err(e) => eprintln!("rendermd: {e}"),
                    }
                }
            }
            if !loaded {
                let state = app.state::<Mutex<AppState>>();
                let mut s = state.lock().unwrap();
                s.showing_welcome = true;
                s.mode = state::Mode::Preview;
                s.render_preview();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::file::open_file,
            commands::file::new_file,
            commands::file::save_file,
            commands::file::save_file_as,
            commands::doc::update_text,
            commands::doc::set_mode,
            commands::doc::get_doc,
            commands::doc::set_dark,
            commands::doc::get_build_info,
            commands::file::export_html,
            commands::preview_msg::preview_message,
            commands::table::convert_table_paste,
            commands::image::image_change,
            commands::image::paste_image,
            commands::link::resolve_local_link,
            watcher::reload_from_disk,
        ])
        .run(tauri::generate_context!())
        .expect("error while running RenderMD");
}
