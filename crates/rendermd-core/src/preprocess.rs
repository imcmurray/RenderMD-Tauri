//! Markdown preprocessing passes that run before comrak: GitHub-style
//! alerts and ```mermaid fences. Extracted verbatim from the GTK app's
//! `main.rs`.

use comrak::ComrakOptions;

use crate::render::html_escape;

// GitHub-style alerts: > [!NOTE]/[!TIP]/[!IMPORTANT]/[!WARNING]/[!CAUTION]
// followed by `> body...` lines. Comrak doesn't ship this extension, so we
// detect openers, collect blockquote continuations, render the body to HTML
// recursively (without the syntect plugin to keep alerts free of nested
// concerns), and emit a single-line <div class="alert alert-X">…</div>.
//
// Why single-line: <div> is a CommonMark "type 6" HTML block — terminates
// at the next blank line. If the rendered body had its own newlines (which
// it does: comrak emits paragraphs separated by \n), the block would close
// mid-alert. Replacing newlines with spaces in the rendered HTML keeps the
// whole alert as one logical line that comrak passes through verbatim.

// Octicons (MIT-licensed) — rough matches for what GitHub uses on alerts.
pub const ALERT_ICON_NOTE: &str = r##"<svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>"##;

pub const ALERT_ICON_TIP: &str = r##"<svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.49a.75.75 0 0 1-1.484.211c-.04-.282-.163-.547-.37-.847a8.456 8.456 0 0 0-.542-.68c-.084-.1-.173-.205-.268-.32C3.201 7.75 2.5 6.766 2.5 5.25 2.5 2.31 4.863 0 8 0s5.5 2.31 5.5 5.25c0 1.516-.701 2.5-1.328 3.259-.095.115-.184.22-.268.319-.207.245-.383.453-.541.681-.208.3-.33.565-.37.847a.751.751 0 0 1-1.485-.212c.084-.593.337-1.078.621-1.489.203-.292.45-.584.673-.848.075-.088.147-.173.213-.253.561-.679.985-1.32.985-2.304 0-2.06-1.637-3.75-4-3.75ZM5.75 12h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5ZM6 15.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 0 1.5h-2.5a.75.75 0 0 1-.75-.75Z"/></svg>"##;

pub const ALERT_ICON_IMPORTANT: &str = r##"<svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>"##;

pub const ALERT_ICON_WARNING: &str = r##"<svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>"##;

pub const ALERT_ICON_CAUTION: &str = r##"<svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.14.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>"##;

// Variants table: (token, lowercase variant for CSS class, label, icon).
pub const ALERT_VARIANTS: &[(&str, &str, &str, &str)] = &[
    ("[!NOTE]", "note", "Note", ALERT_ICON_NOTE),
    ("[!TIP]", "tip", "Tip", ALERT_ICON_TIP),
    (
        "[!IMPORTANT]",
        "important",
        "Important",
        ALERT_ICON_IMPORTANT,
    ),
    ("[!WARNING]", "warning", "Warning", ALERT_ICON_WARNING),
    ("[!CAUTION]", "caution", "Caution", ALERT_ICON_CAUTION),
];

pub fn detect_alert_opener(
    line: &str,
) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    let trimmed = line.trim();
    let after_gt = trimmed.strip_prefix('>')?.trim();
    ALERT_VARIANTS.iter().find(|v| after_gt == v.0)
}

pub fn strip_blockquote_prefix(line: &str) -> String {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix('>') {
        // Per CommonMark, a single optional space after `>` is part of the
        // marker and should be stripped.
        rest.strip_prefix(' ').unwrap_or(rest).to_string()
    } else {
        line.to_string()
    }
}

pub fn render_alert_body(body: &str) -> String {
    // Comrak with the same GFM extensions as the main pipeline minus the
    // syntect plugin (alerts almost never carry highlighted code blocks,
    // and skipping syntect keeps the body cheap to render).
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.parse.smart = true;
    options.render.unsafe_ = true;
    comrak::markdown_to_html(body, &options)
}

pub fn preprocess_alerts(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut iter = text.lines().peekable();

    while let Some(line) = iter.next() {
        if let Some(&(_, variant, label, icon)) = detect_alert_opener(line) {
            // Collect continuation lines (blockquote-prefixed).
            let mut body_lines: Vec<String> = Vec::new();
            while let Some(peek) = iter.peek() {
                if !peek.trim_start().starts_with('>') {
                    break;
                }
                body_lines.push(strip_blockquote_prefix(peek));
                iter.next();
            }
            let body_md = body_lines.join("\n");
            let body_html = render_alert_body(&body_md);
            // Flatten newlines so the resulting <div> survives CommonMark's
            // type-6 block parsing (which terminates on a blank line).
            let body_oneline = body_html.replace('\n', " ");

            out.push('\n');
            out.push_str(&format!(
                "<div class=\"alert alert-{}\"><div class=\"alert-title\">{} <span>{}</span></div>{}</div>",
                variant, icon, label, body_oneline
            ));
            out.push_str("\n\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

// Convert ```mermaid fences to <div class="mermaid">SOURCE</div> raw HTML
// before comrak runs. Doing this at the markdown level (rather than
// post-processing comrak's output) sidesteps the SyntectAdapter wrapping
// unknown languages in its own markup. Returns (preprocessed text, had-any).
pub fn preprocess_mermaid_blocks(text: &str) -> (String, bool) {
    let mut out = String::with_capacity(text.len());
    let mut had_mermaid = false;
    let mut in_block = false;
    let mut buf = String::new();
    for line in text.lines() {
        if in_block {
            if line.trim() == "```" {
                // <pre> is a CommonMark "type 1" HTML block — terminates only
                // on </pre>, so blank lines inside the diagram survive intact.
                // (A <div> wrapper would terminate at the first blank line and
                // slice multi-paragraph diagrams in half.)
                out.push_str("\n<pre class=\"mermaid\">\n");
                out.push_str(&html_escape(&buf));
                out.push_str("</pre>\n\n");
                in_block = false;
                had_mermaid = true;
                buf.clear();
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        } else if line.trim() == "```mermaid" {
            in_block = true;
            buf.clear();
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    // Unclosed fence: restore the original lines verbatim so we don't drop content.
    if in_block {
        out.push_str("```mermaid\n");
        out.push_str(&buf);
    }
    (out, had_mermaid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preprocess_no_fences() {
        let (out, had) = preprocess_mermaid_blocks("# hi\n\nplain text\n");
        assert!(!had);
        assert!(out.contains("# hi"));
        assert!(out.contains("plain text"));
        assert!(!out.contains("class=\"mermaid\""));
    }

    #[test]
    fn preprocess_simple_block() {
        let input = "before\n\n```mermaid\nflowchart TD\n  A --> B\n```\n\nafter\n";
        let (out, had) = preprocess_mermaid_blocks(input);
        assert!(had);
        assert!(out.contains(r#"<pre class="mermaid">"#));
        assert!(out.contains("</pre>"));
        assert!(out.contains("flowchart TD"));
        assert!(
            out.contains("A --&gt; B"),
            "diagram body should be HTML-escaped"
        );
        assert!(out.contains("before") && out.contains("after"));
    }

    #[test]
    fn preprocess_blank_lines_inside() {
        // Regression for issue #1: a multi-section flowchart with a blank line
        // between sections must be preserved as a single mermaid block, not
        // sliced at the blank line.
        let input = "```mermaid\nflowchart TD\n  A --> B\n\n  C --> D\n```\n";
        let (out, _) = preprocess_mermaid_blocks(input);
        // The opening <pre> and closing </pre> must bracket BOTH sections.
        let open = out
            .find(r#"<pre class="mermaid">"#)
            .expect("opening tag present");
        let close = out.find("</pre>").expect("closing tag present");
        assert!(open < close);
        let body = &out[open..close];
        assert!(body.contains("A --&gt; B"));
        assert!(body.contains("C --&gt; D"));
    }

    #[test]
    fn preprocess_unclosed_fence() {
        // Unclosed fence: don't lose user content, restore as-is.
        let input = "before\n\n```mermaid\nflowchart TD\n  A --> B\n";
        let (out, had) = preprocess_mermaid_blocks(input);
        assert!(!had, "unclosed fence shouldn't count as a mermaid block");
        assert!(out.contains("```mermaid"));
        assert!(out.contains("flowchart TD"));
        assert!(!out.contains(r#"<pre class="mermaid">"#));
    }

    #[test]
    fn preprocess_indented_fence_current_behavior() {
        // TODO: CommonMark spec says fences with 4+ spaces of indent are an
        // indented code block, not a fence. Current implementation uses
        // line.trim() so it matches any indent. This test pins the current
        // behavior; tighten if/when we hew closer to spec.
        let input = "    ```mermaid\n    flowchart TD\n    ```\n";
        let (_, had) = preprocess_mermaid_blocks(input);
        assert!(had, "current implementation matches indented fences");
    }

    #[test]
    fn preprocess_html_escapes_diagram() {
        let input = "```mermaid\nA[<b>x</b> & \"y\"]\n```\n";
        let (out, _) = preprocess_mermaid_blocks(input);
        assert!(out.contains("&lt;b&gt;"));
        assert!(out.contains("&amp;"));
        assert!(out.contains("&quot;"));
        assert!(!out.contains("<b>x</b>"));
    }

    #[test]
    fn alert_note_emits_div() {
        let out = preprocess_alerts("> [!NOTE]\n> Hello there.\n");
        assert!(out.contains(r#"<div class="alert alert-note">"#));
        assert!(out.contains("<span>Note</span>"));
        assert!(out.contains("Hello there."));
    }

    #[test]
    fn alert_each_variant_recognized() {
        for (token, variant, label, _) in ALERT_VARIANTS {
            let input = format!("> {}\n> body\n", token);
            let out = preprocess_alerts(&input);
            assert!(
                out.contains(&format!(r#"alert-{}""#, variant)),
                "missing class for {}",
                token
            );
            assert!(
                out.contains(&format!("<span>{}</span>", label)),
                "missing label for {}",
                token
            );
        }
    }

    #[test]
    fn alert_collects_multiline_body() {
        let input = "> [!WARNING]\n> first line\n> second line\n> third line\n";
        let out = preprocess_alerts(input);
        // The whole alert must live on one line so the <div> survives
        // CommonMark's blank-line termination of HTML blocks.
        let div_start = out.find("<div class=\"alert").unwrap();
        let line_end = out[div_start..]
            .find('\n')
            .map(|i| div_start + i)
            .unwrap_or(out.len());
        let alert_line = &out[div_start..line_end];
        assert!(alert_line.contains("first line"));
        assert!(alert_line.contains("second line"));
        assert!(alert_line.contains("third line"));
        assert!(alert_line.ends_with("</div>"));
    }

    #[test]
    fn alert_terminates_at_non_blockquote_line() {
        let input = "> [!NOTE]\n> inside\nafter\n";
        let out = preprocess_alerts(input);
        let div_start = out.find("<div class=\"alert").unwrap();
        let line_end = out[div_start..]
            .find('\n')
            .map(|i| div_start + i)
            .unwrap_or(out.len());
        let alert_line = &out[div_start..line_end];
        assert!(alert_line.contains("inside"));
        // "after" must NOT be inside the alert; it should be a separate line below.
        assert!(!alert_line.contains("after"));
        assert!(out[line_end..].contains("after"));
    }

    #[test]
    fn alert_unknown_variant_passes_through_as_blockquote() {
        // [!FOOBAR] is not a known variant — leave the lines untouched
        // so comrak renders them as a regular blockquote.
        let input = "> [!FOOBAR]\n> body\n";
        let out = preprocess_alerts(input);
        assert!(out.contains("[!FOOBAR]"));
        assert!(!out.contains("class=\"alert"));
    }

    #[test]
    fn alert_case_sensitive_matches_github() {
        // Lowercase shouldn't match.
        let out = preprocess_alerts("> [!note]\n> body\n");
        assert!(!out.contains("class=\"alert"));
    }

    #[test]
    fn alert_inline_emoji_in_body_renders() {
        // emoji preprocessing runs first, so the body has 🚀 by the time we
        // collect it.
        let input = "> [!TIP]\n> Ship it 🚀\n";
        let out = preprocess_alerts(input);
        assert!(out.contains("🚀"));
    }
}
