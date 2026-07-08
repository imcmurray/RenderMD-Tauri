//! Markdown → HTML rendering: the comrak (GFM + syntect) pipeline plus the
//! HTML template assembly. Ported from the GTK app's `main.rs`; the only
//! functional change is the base-href / mermaid-src scheme: instead of
//! `file://` URIs we emit root-relative `/fs/...` paths that the shell's
//! custom protocol handler (preview://localhost on Linux/macOS,
//! http://preview.localhost on Windows) resolves back to disk.

use std::path::Path;

use comrak::plugins::syntect::SyntectAdapter;
use comrak::{markdown_to_html_with_plugins, ComrakOptions, ComrakPlugins};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

use crate::emoji::preprocess_emoji;
use crate::preprocess::{preprocess_alerts, preprocess_mermaid_blocks};
use crate::template::{
    mermaid_script_tag, HTML_TEMPLATE, IMAGE_CLICK_JS, MERMAID_INIT_JS, PREVIEW_CSS_BASE,
    PREVIEW_CSS_DARK, PREVIEW_CSS_LIGHT,
};

const APP_NAME: &str = "RenderMD";

/// Percent-encoding set for filesystem paths embedded in URL *paths*.
///
/// Encodes controls plus the characters that would break out of the path
/// component (`"`, `#`, `<`, `>`, `?`, `` ` ``, `{`, `}`), the space, and
/// `%` itself (so pre-existing percent signs in filenames survive a decode
/// round-trip). `/` is deliberately NOT encoded: path separators must stay
/// literal so relative-URL resolution (`../img.png` against the base href)
/// works in URL space the same way it does on disk. Non-ASCII bytes are
/// always percent-encoded by `utf8_percent_encode` regardless of this set.
pub const PATH_SEGMENT_ENCODE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'%')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// Build the root-relative base href for a document directory:
/// `/fs/<percent-encoded absolute dir>/`.
///
/// Root-relative (no scheme/host) so the same HTML works under both
/// `preview://localhost` (Linux/macOS) and `http://preview.localhost`
/// (Windows). On Windows, backslashes are converted to `/` and the drive
/// path is prefixed with `/` (`C:\docs` → `/fs/C:/docs/`).
pub fn fs_base_href(dir: &Path) -> String {
    let path_str = dir.to_string_lossy();
    #[cfg(windows)]
    let path_str = std::borrow::Cow::<str>::Owned(path_str.replace('\\', "/"));
    let mut path = path_str.into_owned();
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    let encoded = utf8_percent_encode(&path, PATH_SEGMENT_ENCODE).to_string();
    format!("/fs{}/", encoded)
}

pub fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

// Renders a markdown snippet to inline body HTML using the same comrak
// pipeline as the full-document renderer. Used by the hover-swap so the
// previous-version view keeps the original block's typography rather
// than dropping into a monospace diff view.
pub fn render_block_to_inline_html(text: &str, dark: bool) -> String {
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.tagfilter = false;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.header_ids = Some(String::new());
    options.parse.smart = true;
    options.render.unsafe_ = true;
    options.render.github_pre_lang = true;

    let theme = if dark {
        "base16-ocean.dark"
    } else {
        "InspiredGitHub"
    };
    let adapter = SyntectAdapter::new(Some(theme));
    let mut plugins = ComrakPlugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    let with_emoji = preprocess_emoji(text);
    let with_alerts = preprocess_alerts(&with_emoji);
    let (preprocessed, _) = preprocess_mermaid_blocks(&with_alerts);
    markdown_to_html_with_plugins(&preprocessed, &options, &plugins)
}

pub fn render_markdown_to_html(
    text: &str,
    base_dir: Option<&Path>,
    dark: bool,
    title: &str,
) -> String {
    // Comrak gets us GFM-style features matching the Python python-markdown +
    // pymdown-extensions setup: tables, strikethrough, autolinks, task lists,
    // footnotes, smart quotes, superscript, description lists.
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.tagfilter = false;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.header_ids = Some(String::new());
    options.parse.smart = true;
    options.render.unsafe_ = true;
    options.render.github_pre_lang = true;
    // Emit `data-sourcepos="L:C-L:C"` on every block element. The mode
    // toggle uses these to scroll the source view to the line under the
    // top-most visible preview block (and vice-versa for the reverse path).
    options.render.sourcepos = true;

    // Pick a syntect theme that flips with light/dark.
    let theme = if dark {
        "base16-ocean.dark"
    } else {
        "InspiredGitHub"
    };
    let adapter = SyntectAdapter::new(Some(theme));
    let mut plugins = ComrakPlugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    let with_emoji = preprocess_emoji(text);
    let with_alerts = preprocess_alerts(&with_emoji);
    let (preprocessed, had_mermaid) = preprocess_mermaid_blocks(&with_alerts);
    let body = markdown_to_html_with_plugins(&preprocessed, &options, &plugins);

    let mermaid_script = if had_mermaid {
        let theme = if dark { "dark" } else { "default" };
        let init = MERMAID_INIT_JS.replace("{THEME}", theme);
        // Reference the bundle via <script src=...> rather than inlining it.
        // Inlining a 3MB <script> alongside a <table> in the body triggers a
        // WebKitGTK pathology where the WebProcess pegs CPU and grows memory
        // unboundedly until OOM. Loading the bundle as an external file via
        // a separate request avoids that interaction entirely.
        format!("{}\n<script>{}</script>", mermaid_script_tag(), init)
    } else {
        String::new()
    };

    let base_href = match base_dir {
        Some(dir) => fs_base_href(dir),
        None => String::new(),
    };

    let theme_css = if dark {
        PREVIEW_CSS_DARK
    } else {
        PREVIEW_CSS_LIGHT
    };
    let title_safe = if title.is_empty() {
        APP_NAME.to_string()
    } else {
        html_escape(title)
    };

    let image_js = IMAGE_CLICK_JS;
    HTML_TEMPLATE
        .replace("{TITLE}", &title_safe)
        .replace("{THEME_CSS}", theme_css)
        .replace("{BASE_CSS}", PREVIEW_CSS_BASE)
        .replace("{BASE_HREF}", &base_href)
        .replace("{BODY}", &body)
        .replace("{MERMAID_SCRIPT}", &mermaid_script)
        .replace("{IMAGE_CLICK_JS}", image_js)
        .replace("{PREVIEW_BRIDGE_JS}", crate::template::PREVIEW_BRIDGE_JS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_entities() {
        assert_eq!(html_escape("&"), "&amp;");
        assert_eq!(html_escape("<"), "&lt;");
        assert_eq!(html_escape(">"), "&gt;");
        assert_eq!(html_escape("\""), "&quot;");
        assert_eq!(html_escape("'"), "&#39;");
        assert_eq!(
            html_escape("a < b && c > d"),
            "a &lt; b &amp;&amp; c &gt; d"
        );
    }

    #[test]
    fn render_smoke_no_mermaid() {
        let html = render_markdown_to_html("# hello\n\ntext", None, false, "doc");
        assert!(html.contains("<h1"));
        assert!(html.contains("hello"));
        // The mermaid loader tag and init call must only appear when the doc
        // actually contains a mermaid block.
        assert!(
            !html.contains(&mermaid_script_tag()),
            "Mermaid script tag should not be injected when no mermaid blocks are present"
        );
        assert!(
            !html.contains("mermaid.run()"),
            "Mermaid init should not be injected when no mermaid blocks are present"
        );
    }

    #[test]
    fn render_injects_mermaid_when_present() {
        let html = render_markdown_to_html(
            "```mermaid\nflowchart TD\n  A --> B\n```\n",
            None,
            false,
            "doc",
        );
        assert!(
            html.contains("mermaid.run()"),
            "init script should be injected"
        );
        // Bundle is referenced as an external <script src=...> served by the
        // shell's protocol handler. (Inlining it broke WebKitGTK.)
        assert!(
            html.contains(&mermaid_script_tag()),
            "bundle should be referenced via the template's script tag"
        );
    }

    #[test]
    fn render_base_href_set_when_dir_given() {
        let dir = std::path::Path::new("/tmp/somewhere");
        let html = render_markdown_to_html("hi", Some(dir), false, "doc");
        assert!(html.contains(r#"<base href="/fs/tmp/somewhere/">"#));
    }

    #[test]
    fn render_base_href_empty_when_no_dir() {
        let html = render_markdown_to_html("hi", None, false, "doc");
        assert!(html.contains(r#"<base href="">"#));
    }

    #[test]
    fn render_emits_data_sourcepos_for_scroll_sync() {
        let html = render_markdown_to_html("# Title\n\nA paragraph.\n", None, false, "doc");
        // Heading from line 1 carries its source position; the paragraph from
        // line 3 carries its own. The mode-toggle scroll sync reads these to
        // map preview blocks back to source lines.
        assert!(
            html.contains(r#"data-sourcepos="1:1-"#),
            "missing heading sourcepos in: {html}"
        );
        assert!(
            html.contains(r#"data-sourcepos="3:1-"#),
            "missing paragraph sourcepos in: {html}"
        );
    }

    #[test]
    fn render_dark_theme_picks_dark_css() {
        let dark = render_markdown_to_html("hi", None, true, "doc");
        let light = render_markdown_to_html("hi", None, false, "doc");
        // The two themes diverge in their CSS variable values; confirm we get
        // different output for the two flags.
        assert_ne!(dark, light);
        // Sanity: both contain the shared base CSS.
        assert!(dark.contains(":root"));
        assert!(light.contains(":root"));
    }

    #[test]
    fn render_mermaid_theme_follows_dark_flag() {
        let input = "```mermaid\nflowchart TD\nA --> B\n```\n";
        let dark = render_markdown_to_html(input, None, true, "doc");
        let light = render_markdown_to_html(input, None, false, "doc");
        assert!(dark.contains("theme: 'dark'"));
        assert!(light.contains("theme: 'default'"));
    }

    #[test]
    fn fs_base_href_plain_unix_path() {
        assert_eq!(
            fs_base_href(std::path::Path::new("/home/user/docs")),
            "/fs/home/user/docs/"
        );
    }

    #[test]
    fn fs_base_href_encodes_specials_but_not_slash() {
        let href = fs_base_href(std::path::Path::new("/a b/c#d/e?f/100%/g`{}"));
        assert_eq!(href, "/fs/a%20b/c%23d/e%3Ff/100%25/g%60%7B%7D/");
    }

    #[test]
    fn fs_base_href_keeps_angle_quote_out() {
        let href = fs_base_href(std::path::Path::new("/x<y>\"z"));
        assert_eq!(href, "/fs/x%3Cy%3E%22z/");
    }

    #[test]
    fn fs_base_href_percent_encodes_non_ascii() {
        // Non-ASCII is always percent-encoded (UTF-8 bytes), independent of
        // the AsciiSet.
        let href = fs_base_href(std::path::Path::new("/döcs"));
        assert_eq!(href, "/fs/d%C3%B6cs/");
    }
}
