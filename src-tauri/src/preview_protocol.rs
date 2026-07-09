//! The `preview://` URI scheme — serves the rendered document, the mermaid
//! bundle, and local files referenced by the markdown.
//!
//! Routes (all root-relative so the same HTML works on `preview://localhost`
//! (Linux/macOS) and `http://preview.localhost` (Windows)):
//! - `/doc.html?rev=N`          → current `AppState::preview_html`
//! - `/assets/mermaid.min.js`   → embedded bundle. Served EXTERNALLY on
//!   purpose: inlining a multi-MB script next to a `<table>` wedges
//!   WebKitGTK's WebProcess (busy-loop + OOM). Never inline it.
//! - `/fs/<absolute path>`      → file from disk (images etc.). The rendered
//!   document carries `<base href="/fs/<dir>/">`, so relative image srcs —
//!   including `../` traversals — resolve here in URL space.

use std::path::PathBuf;
use std::sync::Mutex;

use percent_encoding::percent_decode_str;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{Manager, UriSchemeContext};

use crate::state::AppState;

const MERMAID_BUNDLE: &[u8] = include_bytes!("../assets/mermaid.min.js");

/// The embedded bundle, for HTML exports that need a file:// copy.
pub fn mermaid_bundle() -> &'static [u8] {
    MERMAID_BUNDLE
}

/// CSP for the preview document itself. Inline scripts/styles are ours
/// (rendered by rendermd-core); external fetches are limited to this scheme.
const PREVIEW_CSP: &str = "default-src 'none'; \
     script-src 'unsafe-inline' preview: http://preview.localhost; \
     style-src 'unsafe-inline'; \
     img-src preview: http://preview.localhost data:; \
     font-src preview: http://preview.localhost data:; \
     media-src preview: http://preview.localhost";

pub fn handle<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let path = request.uri().path();

    if path == "/doc.html" {
        let state = ctx.app_handle().state::<Mutex<AppState>>();
        let html = state.lock().unwrap().preview_html.clone();
        return Response::builder()
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CONTENT_SECURITY_POLICY, PREVIEW_CSP)
            .header(header::CACHE_CONTROL, "no-store")
            .body(html.into_bytes())
            .unwrap();
    }

    if path == "/assets/mermaid.min.js" {
        return Response::builder()
            .header(header::CONTENT_TYPE, "application/javascript")
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .body(MERMAID_BUNDLE.to_vec())
            .unwrap();
    }

    if let Some(rest) = path.strip_prefix("/fs/") {
        let root = ctx
            .app_handle()
            .state::<Mutex<AppState>>()
            .lock()
            .unwrap()
            .allowed_fs_root
            .clone();
        return serve_local_file(rest, root.as_deref());
    }

    not_found()
}

/// Serve a local file referenced by the document — CONFINED to the allowed
/// root (git top-level or doc dir). A hostile markdown file cannot read
/// outside that tree via `/fs/etc/passwd` or `../` escapes: the path is
/// canonicalized (symlinks + `..` resolved) and rejected unless it stays
/// within the root. See rendermd_core::fsroot.
fn serve_local_file(encoded: &str, root: Option<&std::path::Path>) -> Response<Vec<u8>> {
    let decoded = percent_decode_str(encoded).decode_utf8_lossy();
    // `/fs/home/ianm/notes/img.png` → `/home/ianm/notes/img.png`;
    // on Windows `/fs/C:/Users/...` → `C:/Users/...`.
    #[cfg(windows)]
    let requested = PathBuf::from(decoded.as_ref());
    #[cfg(not(windows))]
    let requested = PathBuf::from(format!("/{decoded}"));

    let Some(fs_path) = rendermd_core::fsroot::resolve_within(root, &requested) else {
        return not_found();
    };

    match std::fs::read(&fs_path) {
        Ok(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, mime_for(&fs_path))
            .header(header::CACHE_CONTROL, "no-cache")
            .body(bytes)
            .unwrap(),
        Err(_) => not_found(),
    }
}

fn mime_for(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("avif") => "image/avif",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("md") | Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(b"not found".to_vec())
        .unwrap()
}
