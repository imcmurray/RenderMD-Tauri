//! External-change detection and diff rendering: change bars for blocks
//! edited outside the app, plus the line/word diff machinery that powers the
//! hover "previous version" view. Ported verbatim from the GTK app's
//! `main.rs`; the only swap is `glib::Uri::escape_string` →
//! `percent_encoding` (see [`URI_COMPONENT_ENCODE`]).

use std::collections::{HashMap, HashSet};

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::render::{html_escape, render_block_to_inline_html};

/// Matches the old `glib::Uri::escape_string(s, None, false)` behaviour:
/// every ASCII character except the RFC 3986 unreserved set
/// (`A-Z a-z 0-9 - . _ ~`) is percent-encoded, and non-ASCII bytes are
/// always percent-encoded. JavaScript's `decodeURIComponent` round-trips
/// this exactly, which is what the tooltip script relies on.
pub const URI_COMPONENT_ENCODE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

// Multiset line-subtraction diff: lines that appear more often in `new` than
// in `old` are flagged as changed. Imprecise around duplicate lines but
// adequate for prose change-marking and avoids pulling in a diff crate.
// Returned indices are 1-based against `new`.
pub fn compute_changed_lines(old: &str, new: &str) -> HashSet<usize> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for line in old.lines() {
        *counts.entry(line).or_default() += 1;
    }
    let mut changed = HashSet::new();
    for (i, line) in new.lines().enumerate() {
        match counts.get_mut(line) {
            Some(c) if *c > 0 => *c -= 1,
            _ => {
                changed.insert(i + 1);
            }
        }
    }
    changed
}

// Approximate top-level Markdown block detection: split on blank lines, but
// keep fenced code blocks (``` or ~~~) together. Good enough for marking
// changed regions in prose; a list with internal blank lines will be split
// per-item, which is finer-grained than the AST but still informative.
pub fn split_top_level_blocks(text: &str) -> Vec<(usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();
    let mut block_start: Option<usize> = None;
    let mut in_fence = false;
    let mut fence_marker: &str = "";
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if in_fence {
            if trimmed.starts_with(fence_marker) {
                in_fence = false;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if block_start.is_none() {
                block_start = Some(idx + 1);
            }
            in_fence = true;
            fence_marker = if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            };
        } else if line.trim().is_empty() {
            if let Some(start) = block_start.take() {
                blocks.push((start, idx));
            }
        } else if block_start.is_none() {
            block_start = Some(idx + 1);
        }
    }
    if let Some(start) = block_start.take() {
        blocks.push((start, lines.len()));
    }
    blocks
}

pub struct PendingChanges {
    pub changed_lines: HashSet<usize>,
    pub old_text: String,
    pub reload_ts: i64,
}

// Insert a `<div class="rmd-changed-marker">` HTML block before each
// top-level block whose line range overlaps `changes.changed_lines`. The
// CSS sibling selector then puts a left border on the next rendered
// element. Each marker also carries the corresponding *old* block text and
// the reload timestamp, which the bundled JS uses to populate a tooltip on
// hover.
pub fn inject_change_markers(text: &str, changes: &PendingChanges, dark: bool) -> String {
    if changes.changed_lines.is_empty() {
        return text.to_string();
    }
    let new_blocks = split_top_level_blocks(text);
    let new_lines: Vec<&str> = text.lines().collect();
    let old_lines: Vec<&str> = changes.old_text.lines().collect();

    let mut markers: HashMap<usize, String> = HashMap::new();
    for (start, end) in new_blocks {
        if !(start..=end).any(|l| changes.changed_lines.contains(&l)) {
            continue;
        }
        // Best-effort positional match: take the same 1-based line range
        // from old_text and new_text, clamped. Imprecise after big inserts
        // / deletes higher up, but a useful approximation.
        let slice = |lines: &[&str]| -> String {
            if lines.is_empty() || start > lines.len() {
                String::new()
            } else {
                let lo = start - 1;
                let hi = end.min(lines.len());
                lines[lo..hi].join("\n")
            }
        };
        let old_block = slice(&old_lines);
        let new_block = slice(&new_lines);
        // Fenced code blocks need a different code path: comrak escapes
        // anything inside <pre><code>, so our <del>/<ins>-annotated
        // markdown shows up as literal HTML tags. For these, build the
        // diff HTML ourselves and bypass comrak.
        let prev_html =
            if block_starts_with_fence(&new_block) || block_starts_with_fence(&old_block) {
                build_code_block_diff_html(&old_block, &new_block)
            } else {
                let annotated_md = build_annotated_diff_md(&old_block, &new_block);
                render_block_to_inline_html(&annotated_md, dark)
            };
        let escaped = utf8_percent_encode(&prev_html, URI_COMPONENT_ENCODE).to_string();
        markers.insert(
            start,
            format!(
                "<div class=\"rmd-changed-marker\" data-prev-html=\"{}\" data-age-ts=\"{}\"></div>\n\n",
                escaped, changes.reload_ts
            ),
        );
    }

    let mut out = String::with_capacity(text.len() + markers.len() * 96);
    for (idx, line) in new_lines.iter().enumerate() {
        if let Some(m) = markers.get(&(idx + 1)) {
            out.push_str(m);
        }
        out.push_str(line);
        out.push('\n');
    }
    // Append a one-shot script that wires native tooltips on the changed
    // blocks. Native browser tooltip delay (~700–1000 ms) gives the
    // "hover for a second" behaviour for free.
    out.push('\n');
    out.push_str(CHANGE_TOOLTIP_JS);
    out
}

pub const CHANGE_TOOLTIP_JS: &str = r#"<script>
(function() {
  function fmtAge(s) {
    if (s < 5) return "just now";
    if (s < 60) return s + " seconds ago";
    if (s < 3600) {
      var m = Math.floor(s / 60);
      return m + (m === 1 ? " minute ago" : " minutes ago");
    }
    if (s < 86400) {
      var h = Math.floor(s / 3600);
      return h + (h === 1 ? " hour ago" : " hours ago");
    }
    var d = Math.floor(s / 86400);
    return d + (d === 1 ? " day ago" : " days ago");
  }
  function unwrapToInner(html, expectedTag) {
    // Comrak wraps each block in its outer tag (e.g. <p>, <h2>, <ul>).
    // Since the target element ALREADY is that wrapper, strip the outer
    // tag so we don't end up with nested <p><p>... etc.
    var tmp = document.createElement("div");
    tmp.innerHTML = html;
    var first = tmp.firstElementChild;
    if (first && first.tagName === expectedTag) {
      return first.innerHTML;
    }
    return html;
  }
  function init() {
    document.querySelectorAll(".rmd-changed-marker").forEach(function(m) {
      var t = m.nextElementSibling;
      if (!t) return;
      var ts = parseInt(m.getAttribute("data-age-ts"), 10);
      var prevRaw = m.getAttribute("data-prev-html") || "";
      var prevHtml = "";
      try { prevHtml = decodeURIComponent(prevRaw); } catch (e) { prevHtml = ""; }
      var inner = prevHtml ? unwrapToInner(prevHtml, t.tagName) : "";
      var swapTimer = null;
      t.addEventListener("mouseenter", function() {
        if (t.classList.contains("rmd-showing-prev")) return;
        if (swapTimer) clearTimeout(swapTimer);
        swapTimer = setTimeout(function() {
          swapTimer = null;
          if (!("rmdOriginal" in t.dataset)) {
            t.dataset.rmdOriginal = t.innerHTML;
          }
          var ageS = Math.floor(Date.now() / 1000) - ts;
          var banner = '<div class="rmd-prev-banner">Edited externally ' + fmtAge(ageS) + '</div>';
          var content = inner || '<em class="rmd-prev-empty">(no prior content for this block)</em>';
          t.innerHTML = content + banner;
          t.classList.add("rmd-showing-prev");
        }, 1000);
      });
      t.addEventListener("mouseleave", function() {
        if (swapTimer) { clearTimeout(swapTimer); swapTimer = null; }
        if (t.classList.contains("rmd-showing-prev") && "rmdOriginal" in t.dataset) {
          t.innerHTML = t.dataset.rmdOriginal;
          delete t.dataset.rmdOriginal;
          t.classList.remove("rmd-showing-prev");
        }
      });
    });
    buildMinimap();
  }
  // Right-edge minimap of all changed blocks. Ticks are positioned
  // proportionally to where their target block sits in the document so
  // the user can see at a glance where edits landed in a long file,
  // and click any tick to scroll there.
  function buildMinimap() {
    var existing = document.querySelector(".rmd-minimap");
    if (existing) existing.remove();
    var markers = document.querySelectorAll(".rmd-changed-marker");
    if (!markers.length) return;
    var docHeight = document.documentElement.scrollHeight;
    if (docHeight <= window.innerHeight + 4) return;
    var minimap = document.createElement("div");
    minimap.className = "rmd-minimap";
    markers.forEach(function(m) {
      var target = m.nextElementSibling;
      if (!target) return;
      var rect = target.getBoundingClientRect();
      var topInDoc = rect.top + window.scrollY;
      var ratio = Math.max(0, Math.min(1, topInDoc / docHeight));
      var tick = document.createElement("div");
      tick.className = "rmd-minimap-tick";
      tick.style.top = (ratio * 100) + "%";
      tick.title = "Changed block — click to jump";
      tick.addEventListener("click", function() {
        target.scrollIntoView({ behavior: "smooth", block: "center" });
      });
      minimap.appendChild(tick);
    });
    document.body.appendChild(minimap);
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
  // Recompute on resize so tick positions stay accurate.
  var resizeTimer = null;
  window.addEventListener("resize", function() {
    if (resizeTimer) clearTimeout(resizeTimer);
    resizeTimer = setTimeout(buildMinimap, 120);
  });
})();
</script>
"#;

// Greedy line-level diff plus intra-line word-level diff. Adjacent
// delete/add pairs that share enough vocabulary are rendered as a single
// "modified" line with only the changed words highlighted, instead of
// dumping the full old and new lines as two solid red/green blocks.
// Plain LCS-free; good enough for typical edits.

#[derive(Debug)]
pub enum LineOp<'a> {
    Equal(&'a str),
    Delete(&'a str),
    Add(&'a str),
}

pub fn line_diff_ops<'a>(old_lines: &'a [&'a str], new_lines: &'a [&'a str]) -> Vec<LineOp<'a>> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    for new_line in new_lines {
        let found = old_lines[i..].iter().position(|l| l == new_line);
        match found {
            Some(rel) => {
                for line in old_lines.iter().skip(i).take(rel) {
                    ops.push(LineOp::Delete(line));
                }
                ops.push(LineOp::Equal(new_line));
                i += rel + 1;
            }
            None => ops.push(LineOp::Add(new_line)),
        }
    }
    for line in old_lines.iter().skip(i) {
        ops.push(LineOp::Delete(line));
    }
    ops
}

// Tokenize as runs of alphanumerics, runs of whitespace, or single
// non-word chars. Keeps whitespace as its own token so we don't lose
// spacing when stitching the segments back together.
pub fn tokenize_for_diff(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut iter = s.char_indices().peekable();
    while let Some(&(start, c)) = iter.peek() {
        if c.is_alphanumeric() {
            iter.next();
            let mut end = start + c.len_utf8();
            while let Some(&(_, c2)) = iter.peek() {
                if !c2.is_alphanumeric() {
                    break;
                }
                end += c2.len_utf8();
                iter.next();
            }
            out.push(&s[start..end]);
        } else if c.is_whitespace() {
            iter.next();
            let mut end = start + c.len_utf8();
            while let Some(&(_, c2)) = iter.peek() {
                if !c2.is_whitespace() {
                    break;
                }
                end += c2.len_utf8();
                iter.next();
            }
            out.push(&s[start..end]);
        } else {
            iter.next();
            let end = start + c.len_utf8();
            out.push(&s[start..end]);
        }
    }
    out
}

// Multiset overlap divided by the larger token count. Whitespace and empty
// tokens don't contribute. Returns 1.0 for two empty inputs.
pub fn line_similarity(a: &str, b: &str) -> f64 {
    let keep = |t: &&str| !t.trim().is_empty();
    let a_tokens: Vec<&str> = tokenize_for_diff(a).into_iter().filter(keep).collect();
    let b_tokens: Vec<&str> = tokenize_for_diff(b).into_iter().filter(keep).collect();
    let bigger = a_tokens.len().max(b_tokens.len());
    if bigger == 0 {
        return 1.0;
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for t in &a_tokens {
        *counts.entry(*t).or_default() += 1;
    }
    let mut common = 0usize;
    for t in &b_tokens {
        if let Some(c) = counts.get_mut(t) {
            if *c > 0 {
                *c -= 1;
                common += 1;
            }
        }
    }
    common as f64 / bigger as f64
}

// Builds an annotated *markdown* line for a modified pair: emits the new
// text with inline <del>/<ins> tags wrapping only the changed words, so
// after we feed it back to comrak it renders with the same typography as
// the surrounding paragraph / heading / list item.
pub fn build_annotated_word_diff_md(old: &str, new: &str) -> String {
    let old_tokens = tokenize_for_diff(old);
    let new_tokens = tokenize_for_diff(new);
    let mut out = String::new();
    let mut i = 0usize;
    // Buffer added tokens until the next match so deletions are emitted
    // before insertions — reads more naturally as "old → new".
    let mut pending_adds: Vec<&str> = Vec::new();
    let push_del = |out: &mut String, s: &str| {
        out.push_str("<del class=\"rmd-diff-w-del\">");
        out.push_str(s);
        out.push_str("</del>");
    };
    let push_ins = |out: &mut String, s: &str| {
        out.push_str("<ins class=\"rmd-diff-w-add\">");
        out.push_str(s);
        out.push_str("</ins>");
    };
    for new_t in &new_tokens {
        let found = old_tokens[i..].iter().position(|t| t == new_t);
        match found {
            Some(rel) => {
                for tok in old_tokens.iter().skip(i).take(rel) {
                    push_del(&mut out, tok);
                }
                for a in pending_adds.drain(..) {
                    push_ins(&mut out, a);
                }
                out.push_str(new_t);
                i += rel + 1;
            }
            None => pending_adds.push(new_t),
        }
    }
    for tok in old_tokens.iter().skip(i) {
        push_del(&mut out, tok);
    }
    for a in pending_adds.drain(..) {
        push_ins(&mut out, a);
    }
    out
}

pub fn block_starts_with_fence(text: &str) -> bool {
    let first = text.lines().next().unwrap_or("").trim_start();
    first.starts_with("```") || first.starts_with("~~~")
}

// Inline HTML span for a chunk of word-diffed code. Same shape as the
// markdown-side word-diff but emits HTML directly so it survives being
// placed inside <pre><code>.
pub fn render_word_diff_html(old: &str, new: &str) -> String {
    let old_tokens = tokenize_for_diff(old);
    let new_tokens = tokenize_for_diff(new);
    let mut out = String::new();
    let mut i = 0usize;
    let mut pending_adds: Vec<&str> = Vec::new();
    let push_del = |out: &mut String, s: &str| {
        out.push_str(r#"<span class="rmd-diff-w-del">"#);
        out.push_str(&html_escape(s));
        out.push_str("</span>");
    };
    let push_ins = |out: &mut String, s: &str| {
        out.push_str(r#"<span class="rmd-diff-w-add">"#);
        out.push_str(&html_escape(s));
        out.push_str("</span>");
    };
    for new_t in &new_tokens {
        let found = old_tokens[i..].iter().position(|t| t == new_t);
        match found {
            Some(rel) => {
                for tok in old_tokens.iter().skip(i).take(rel) {
                    push_del(&mut out, tok);
                }
                for a in pending_adds.drain(..) {
                    push_ins(&mut out, a);
                }
                out.push_str(&html_escape(new_t));
                i += rel + 1;
            }
            None => pending_adds.push(new_t),
        }
    }
    for tok in old_tokens.iter().skip(i) {
        push_del(&mut out, tok);
    }
    for a in pending_adds.drain(..) {
        push_ins(&mut out, a);
    }
    out
}

// HTML diff for a fenced code block. Strips the fence lines, runs the
// same line + word diff machinery as build_annotated_diff_md, but emits
// <span> tags directly so they survive inside the <pre><code> wrapper.
pub fn build_code_block_diff_html(old: &str, new: &str) -> String {
    const MOD_THRESHOLD: f64 = 0.30;
    fn strip_fences(text: &str) -> Vec<&str> {
        let mut lines: Vec<&str> = text.lines().collect();
        if let Some(first) = lines.first() {
            let trimmed = first.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                lines.remove(0);
            }
        }
        if let Some(last) = lines.last() {
            let trimmed = last.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                lines.pop();
            }
        }
        lines
    }
    let old_lines = strip_fences(old);
    let new_lines = strip_fences(new);
    let ops = line_diff_ops(&old_lines, &new_lines);

    let mut body = String::new();
    let mut idx = 0usize;
    while idx < ops.len() {
        if let LineOp::Equal(line) = &ops[idx] {
            body.push_str(&html_escape(line));
            body.push('\n');
            idx += 1;
            continue;
        }
        let mut adds: Vec<&str> = Vec::new();
        let mut dels: Vec<&str> = Vec::new();
        while idx < ops.len() {
            match &ops[idx] {
                LineOp::Add(s) => adds.push(s),
                LineOp::Delete(s) => dels.push(s),
                LineOp::Equal(_) => break,
            }
            idx += 1;
        }
        let pair_count = adds.len().min(dels.len());
        for i in 0..pair_count {
            let old_line = dels[i];
            let new_line = adds[i];
            if line_similarity(old_line, new_line) >= MOD_THRESHOLD {
                body.push_str(&render_word_diff_html(old_line, new_line));
                body.push('\n');
            } else {
                body.push_str(r#"<span class="rmd-diff-w-del">"#);
                body.push_str(&html_escape(old_line));
                body.push_str("</span>\n");
                body.push_str(r#"<span class="rmd-diff-w-add">"#);
                body.push_str(&html_escape(new_line));
                body.push_str("</span>\n");
            }
        }
        for line in dels.iter().skip(pair_count) {
            body.push_str(r#"<span class="rmd-diff-w-del">"#);
            body.push_str(&html_escape(line));
            body.push_str("</span>\n");
        }
        for line in adds.iter().skip(pair_count) {
            body.push_str(r#"<span class="rmd-diff-w-add">"#);
            body.push_str(&html_escape(line));
            body.push_str("</span>\n");
        }
    }
    format!("<pre><code>{body}</code></pre>")
}

// Builds annotated markdown for a whole changed block. Equal lines pass
// through unchanged; modified pairs become an annotated word-diff line;
// pure adds/deletes get wrapped in <ins>/<del>. The result is plain
// markdown source that we feed back to comrak, so the rendered hover view
// preserves the original block's styling and just marks what changed.
pub fn build_annotated_diff_md(old: &str, new: &str) -> String {
    const MOD_THRESHOLD: f64 = 0.30;
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let ops = line_diff_ops(&old_lines, &new_lines);

    let mut out = String::new();
    let mut idx = 0usize;
    while idx < ops.len() {
        if let LineOp::Equal(line) = &ops[idx] {
            out.push_str(line);
            out.push('\n');
            idx += 1;
            continue;
        }
        let mut adds: Vec<&str> = Vec::new();
        let mut dels: Vec<&str> = Vec::new();
        while idx < ops.len() {
            match &ops[idx] {
                LineOp::Add(s) => adds.push(s),
                LineOp::Delete(s) => dels.push(s),
                LineOp::Equal(_) => break,
            }
            idx += 1;
        }
        let pair_count = adds.len().min(dels.len());
        for i in 0..pair_count {
            let old_line = dels[i];
            let new_line = adds[i];
            if line_similarity(old_line, new_line) >= MOD_THRESHOLD {
                out.push_str(&build_annotated_word_diff_md(old_line, new_line));
                out.push('\n');
            } else {
                out.push_str("<del class=\"rmd-diff-w-del\">");
                out.push_str(old_line);
                out.push_str("</del>\n");
                out.push_str("<ins class=\"rmd-diff-w-add\">");
                out.push_str(new_line);
                out.push_str("</ins>\n");
            }
        }
        for line in dels.iter().skip(pair_count) {
            out.push_str("<del class=\"rmd-diff-w-del\">");
            out.push_str(line);
            out.push_str("</del>\n");
        }
        for line in adds.iter().skip(pair_count) {
            out.push_str("<ins class=\"rmd-diff-w-add\">");
            out.push_str(line);
            out.push_str("</ins>\n");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_lines_flags_new_and_modified() {
        let changed = compute_changed_lines("a\nb\nc\n", "a\nB\nc\nd\n");
        assert!(changed.contains(&2), "modified line flagged");
        assert!(changed.contains(&4), "appended line flagged");
        assert!(!changed.contains(&1));
        assert!(!changed.contains(&3));
    }

    #[test]
    fn changed_lines_empty_when_identical() {
        assert!(compute_changed_lines("a\nb\n", "a\nb\n").is_empty());
    }

    #[test]
    fn blocks_split_on_blank_lines_but_not_in_fences() {
        let text = "para one\n\n```\ncode\n\nmore code\n```\n\npara two\n";
        let blocks = split_top_level_blocks(text);
        assert_eq!(blocks, vec![(1, 1), (3, 7), (9, 9)]);
    }

    #[test]
    fn uri_component_encode_round_trips_like_decodeuricomponent() {
        // Unreserved chars pass through; everything else is %XX-encoded —
        // the exact contract decodeURIComponent expects.
        let s = "a-z_0.9~ <p>&\"'#%/\u{e9}";
        let enc = utf8_percent_encode(s, URI_COMPONENT_ENCODE).to_string();
        assert_eq!(enc, "a-z_0.9~%20%3Cp%3E%26%22%27%23%25%2F%C3%A9");
    }

    #[test]
    fn word_diff_html_marks_changed_token_only() {
        let out = render_word_diff_html("let x = 1;", "let x = 2;");
        assert!(out.contains(r#"<span class="rmd-diff-w-del">1</span>"#));
        assert!(out.contains(r#"<span class="rmd-diff-w-add">2</span>"#));
        assert!(out.contains("let"));
    }

    #[test]
    fn annotated_diff_md_wraps_pure_add_delete() {
        let out = build_annotated_diff_md("gone\n", "here\n");
        // "gone" vs "here": zero shared tokens → below the modified
        // threshold → full-line del/ins.
        assert!(out.contains("<del class=\"rmd-diff-w-del\">gone</del>"));
        assert!(out.contains("<ins class=\"rmd-diff-w-add\">here</ins>"));
    }

    #[test]
    fn code_block_diff_wraps_in_pre_code() {
        let out = build_code_block_diff_html("```\na\n```", "```\nb\n```");
        assert!(out.starts_with("<pre><code>"));
        assert!(out.ends_with("</code></pre>"));
    }

    #[test]
    fn line_similarity_bounds() {
        assert_eq!(line_similarity("", ""), 1.0);
        assert_eq!(line_similarity("aa bb", "aa bb"), 1.0);
        assert_eq!(line_similarity("aa", "bb"), 0.0);
    }
}
