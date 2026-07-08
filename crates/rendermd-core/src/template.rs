//! HTML template + preview CSS/JS constants for the rendered preview.
//!
//! Extracted from the GTK app's `main.rs`. The only functional change from
//! the GTK original is the IPC bridge: the preview document talks to the
//! shell via a `window.__rmdPost(channel, payload)` shim (installed in the
//! template `<head>`, backed by `window.parent.postMessage`) instead of
//! WebKitGTK's `window.webkit.messageHandlers.*`.

// ---- Preview CSS ------------------------------------------------------------
pub const PREVIEW_CSS_LIGHT: &str = r#"
:root {
  --bg: #ffffff;
  --fg: #1f2328;
  --muted: #59636e;
  --accent: #0969da;
  --border: #d1d9e0;
  --code-bg: #f6f8fa;
  --code-fg: #1f2328;
  --kbd-bg: #f6f8fa;
  --table-stripe: #f6f8fa;
  --quote-bg: #f6f8fa;
  --quote-bar: #d1d9e0;
  --alert-note: #0969da;
  --alert-tip: #1a7f37;
  --alert-important: #8250df;
  --alert-warning: #9a6700;
  --alert-caution: #cf222e;
  --change: #d4a017;
}
"#;

pub const PREVIEW_CSS_DARK: &str = r#"
:root {
  --bg: #1e1e2e;
  --fg: #e6edf3;
  --muted: #9da7b1;
  --accent: #79b8ff;
  --border: #30363d;
  --code-bg: #161b22;
  --code-fg: #e6edf3;
  --kbd-bg: #161b22;
  --table-stripe: #161b22;
  --quote-bg: #161b22;
  --quote-bar: #30363d;
  --alert-note: #79b8ff;
  --alert-tip: #3fb950;
  --alert-important: #a371f7;
  --alert-warning: #d29922;
  --alert-caution: #f85149;
  --change: #e3b341;
}
"#;

pub const PREVIEW_CSS_BASE: &str = r#"
* { box-sizing: border-box; }
html, body {
  margin: 0;
  padding: 0;
  background: var(--bg);
  color: var(--fg);
}
body {
  font-family: -apple-system, "Inter", "Cantarell", "Noto Sans",
               "Segoe UI", system-ui, sans-serif;
  font-size: 16px;
  line-height: 1.65;
  padding: 48px max(48px, 8vw) 96px;
  max-width: 980px;
  margin: 0 auto;
  word-wrap: break-word;
}
h1, h2, h3, h4, h5, h6 {
  font-weight: 600;
  line-height: 1.25;
  margin-top: 1.6em;
  margin-bottom: 0.6em;
}
h1 { font-size: 2.1em; padding-bottom: 0.3em; border-bottom: 1px solid var(--border); }
h2 { font-size: 1.55em; padding-bottom: 0.3em; border-bottom: 1px solid var(--border); }
h3 { font-size: 1.25em; }
h4 { font-size: 1.05em; }
h5 { font-size: 0.95em; }
h6 { font-size: 0.88em; color: var(--muted); }
p, ul, ol, blockquote, pre, table { margin: 0 0 1em 0; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
strong { font-weight: 600; }
hr {
  border: 0;
  border-top: 1px solid var(--border);
  margin: 2em 0;
}
ul, ol { padding-left: 2em; }
li + li { margin-top: 0.25em; }
li > p { margin: 0.5em 0; }
ul.contains-task-list { list-style: none; padding-left: 1em; }
ul.contains-task-list li.task-list-item { position: relative; padding-left: 0.25em; }
input.task-list-item-checkbox { margin-right: 0.5em; }

blockquote {
  margin: 1em 0;
  padding: 0.5em 1em;
  background: var(--quote-bg);
  border-left: 4px solid var(--quote-bar);
  color: var(--muted);
  border-radius: 4px;
}
blockquote > :last-child { margin-bottom: 0; }

code, kbd, samp, pre {
  font-family: "JetBrains Mono", "Fira Code", "Source Code Pro",
               "Cascadia Code", monospace;
  font-size: 0.92em;
}
:not(pre) > code {
  background: var(--code-bg);
  color: var(--code-fg);
  padding: 0.18em 0.42em;
  border-radius: 6px;
  border: 1px solid var(--border);
}
pre {
  background: var(--code-bg);
  color: var(--code-fg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 14px 18px;
  overflow-x: auto;
  line-height: 1.55;
}
pre code { background: none; border: 0; padding: 0; }

kbd {
  background: var(--kbd-bg);
  border: 1px solid var(--border);
  border-bottom-width: 2px;
  border-radius: 6px;
  padding: 0.1em 0.5em;
  font-size: 0.85em;
}

table {
  border-collapse: collapse;
  width: 100%;
  display: block;
  overflow-x: auto;
}
table th, table td {
  border: 1px solid var(--border);
  padding: 8px 14px;
  text-align: left;
}
/* Column alignment from the GFM separator row. The data-align
   attribute is set by the post-processor; the attribute-selector
   specificity outranks the bare `table td` rule above so the
   left default doesn't fight column-specific alignment. */
table th[data-align="center"], table td[data-align="center"] { text-align: center; }
table th[data-align="right"], table td[data-align="right"] { text-align: right; }
table tr:nth-child(2n) { background: var(--table-stripe); }
table th { font-weight: 600; background: var(--table-stripe); }

img {
  max-width: 100%;
  height: auto;
  border-radius: 6px;
}

.alert {
  border-left: 4px solid;
  padding: 8px 16px;
  margin: 1em 0;
  border-radius: 0 6px 6px 0;
  background: var(--code-bg);
}
.alert-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  margin-bottom: 4px;
}
.alert-title svg { width: 16px; height: 16px; flex-shrink: 0; }
.alert > p:first-of-type { margin-top: 0; }
.alert > p:last-of-type { margin-bottom: 0; }
.alert-note { border-color: var(--alert-note); }
.alert-note .alert-title { color: var(--alert-note); }
.alert-tip { border-color: var(--alert-tip); }
.alert-tip .alert-title { color: var(--alert-tip); }
.alert-important { border-color: var(--alert-important); }
.alert-important .alert-title { color: var(--alert-important); }
.alert-warning { border-color: var(--alert-warning); }
.alert-warning .alert-title { color: var(--alert-warning); }
.alert-caution { border-color: var(--alert-caution); }
.alert-caution .alert-title { color: var(--alert-caution); }
.rmd-changed-marker { display: block; height: 0; margin: 0; padding: 0; }
.rmd-changed-marker + * {
  border-left: 3px solid var(--change);
  padding-left: 0.75em;
  margin-left: -1em;
  cursor: default;
}
.rmd-changed-marker + .rmd-showing-prev {
  background: rgba(227, 179, 65, 0.06);
}
.rmd-prev-empty { color: var(--muted); }
/* Swap content needs to wrap so long lines stay visible — the
 * mouseleave revert means the user can't scroll horizontally. */
.rmd-showing-prev,
.rmd-showing-prev pre,
.rmd-showing-prev code {
  white-space: pre-wrap;
  word-break: break-word;
  overflow-wrap: anywhere;
  overflow-x: hidden;
}
.rmd-img-wrapper {
  position: relative;
  display: inline-block;
  line-height: 0;
  max-width: 100%;
}
.rmd-img-wrapper img { display: block; max-width: 100%; height: auto; }
.rmd-img-wrapper:hover { outline: 1px dashed var(--accent); outline-offset: 1px; }
.rmd-img-handle {
  position: absolute;
  width: 12px;
  height: 12px;
  background: var(--bg);
  border: 2px solid var(--accent);
  border-radius: 2px;
  opacity: 0;
  transition: opacity 0.12s;
  z-index: 5;
}
.rmd-img-wrapper:hover .rmd-img-handle { opacity: 1; }
.rmd-img-handle-nw { top: -7px; left: -7px; cursor: nwse-resize; }
.rmd-img-handle-ne { top: -7px; right: -7px; cursor: nesw-resize; }
.rmd-img-handle-sw { bottom: -7px; left: -7px; cursor: nesw-resize; }
.rmd-img-handle-se { bottom: -7px; right: -7px; cursor: nwse-resize; }
.rmd-img-dragging { opacity: 0.55; }
.rmd-img-dragging .rmd-img-handle { display: none; }
.rmd-img-dragging:hover { outline: 2px solid var(--accent); }
.rmd-cell { cursor: text; transition: background 0.1s; }
.rmd-cell:hover { background: rgba(127, 127, 127, 0.08); }
.rmd-cell-editing {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
  background: var(--bg);
  white-space: pre-wrap;
  word-break: break-word;
  cursor: text;
}
.rmd-cell-editing:focus { outline-color: var(--accent); }
.rmd-table-toolbar {
  position: fixed;
  z-index: 60;
  display: none;
  flex-direction: row;
  gap: 2px;
  padding: 4px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.18);
  font-size: 0.8em;
  user-select: none;
}
.rmd-table-toolbar-btn {
  background: transparent;
  border: none;
  color: var(--fg);
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font: inherit;
  white-space: nowrap;
}
.rmd-table-toolbar-btn:hover {
  background: rgba(127, 127, 127, 0.14);
  color: var(--accent);
}
.rmd-table-toolbar-btn.rmd-table-toolbar-active {
  background: var(--accent);
  color: var(--bg);
}
.rmd-table-toolbar-btn.rmd-table-toolbar-active:hover {
  background: var(--accent);
  color: var(--bg);
  filter: brightness(1.1);
}
.rmd-table-toolbar-sep {
  width: 1px;
  align-self: stretch;
  background: var(--border);
  margin: 2px 4px;
}
.rmd-table-fixed {
  table-layout: fixed;
}
.rmd-table-fixed th, .rmd-table-fixed td {
  overflow: hidden;
  word-break: break-word;
}
th.rmd-cell { position: relative; }
.rmd-th-resize-handle {
  position: absolute;
  top: 0;
  right: -5px;
  width: 10px;
  height: 100%;
  cursor: col-resize;
  z-index: 3;
  user-select: none;
  touch-action: none;
}
/* Grip bar centered on the column boundary. Hidden at rest so tables look
   unchanged; revealed faintly when the pointer is anywhere over the header
   row (so the resize affordance is discoverable), and fully highlighted on
   the handle itself or while dragging. */
.rmd-th-resize-handle::before {
  content: "";
  position: absolute;
  top: 20%;
  bottom: 20%;
  left: 50%;
  transform: translateX(-50%);
  width: 3px;
  border-radius: 2px;
  background: var(--accent);
  opacity: 0;
  transition: opacity 0.12s ease, top 0.12s ease, bottom 0.12s ease;
}
thead:hover .rmd-th-resize-handle::before {
  opacity: 0.35;
}
.rmd-th-resize-handle:hover::before,
.rmd-th-resize-handle.rmd-resizing::before {
  opacity: 1;
  top: 6%;
  bottom: 6%;
}
.rmd-resizing-table, .rmd-resizing-table * {
  cursor: col-resize !important;
  user-select: none !important;
}
.rmd-history-rail {
  position: fixed;
  left: 8px;
  top: 24px;
  bottom: 24px;
  width: 150px;
  display: flex;
  flex-direction: column;
  align-items: stretch;
  z-index: 50;
  overflow-y: auto;
  scrollbar-width: thin;
  padding: 4px 0;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.18);
}
/* Reserve a gutter so document text never slides under the open rail. */
body:has(.rmd-history-rail) {
  padding-left: max(180px, 8vw);
}
.rmd-history-track {
  position: absolute;
  top: 8px;
  bottom: 8px;
  left: 13px;
  width: 1px;
  background: var(--border);
  pointer-events: none;
}
.rmd-history-circle {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  margin: 0;
  padding: 3px 8px;
  background: transparent;
  border: 0;
  cursor: pointer;
  position: relative;
  z-index: 1;
  text-align: left;
  border-radius: 5px;
  transition: background 0.12s ease;
}
.rmd-history-circle:hover {
  background: var(--code-bg);
}
.rmd-history-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--muted);
  border: 1px solid var(--bg);
  flex-shrink: 0;
  transition: transform 0.12s ease, background 0.12s ease, box-shadow 0.12s ease;
}
.rmd-history-circle:hover .rmd-history-dot {
  transform: scale(1.4);
  background: var(--accent);
  box-shadow: 0 0 6px var(--accent);
}
.rmd-history-circle.rmd-history-active .rmd-history-dot {
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent);
}
.rmd-history-circle.rmd-history-active .rmd-history-when {
  color: var(--fg);
  font-weight: 600;
}
.rmd-history-info {
  display: flex;
  flex-direction: column;
  line-height: 1.25;
  min-width: 0;
}
.rmd-history-when {
  font-size: 10px;
  color: var(--muted);
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}
.rmd-history-stat {
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  display: flex;
  gap: 6px;
}
.rmd-history-add { color: var(--alert-tip); }
.rmd-history-del { color: var(--alert-caution); }
.rmd-history-collapse {
  align-self: flex-end;
  margin: 0 4px 2px 0;
  width: 18px;
  height: 18px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  font-size: 13px;
  line-height: 1;
  cursor: pointer;
  flex-shrink: 0;
  z-index: 2;
  transition: background 0.12s ease, color 0.12s ease;
}
.rmd-history-collapse:hover {
  background: var(--code-bg);
  color: var(--accent);
}
/* Collapsed: compact dots-only column, labels hidden. */
.rmd-history-rail.rmd-collapsed {
  width: 30px;
}
.rmd-history-rail.rmd-collapsed .rmd-history-info {
  display: none;
}
.rmd-history-rail.rmd-collapsed .rmd-history-circle {
  justify-content: center;
  padding: 3px 0;
}
.rmd-history-rail.rmd-collapsed .rmd-history-collapse {
  align-self: center;
  margin: 0 0 2px 0;
}
.rmd-history-rail.rmd-collapsed .rmd-history-track {
  left: 50%;
  transform: translateX(-50%);
}
body:has(.rmd-history-rail.rmd-collapsed) {
  padding-left: max(58px, 8vw);
}
.rmd-history-hint {
  position: fixed;
  left: 14px;
  top: 32px;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--muted);
  opacity: 0.4;
  z-index: 50;
  cursor: pointer;
  transition: opacity 0.12s, transform 0.12s, background 0.12s;
}
.rmd-history-hint:hover {
  opacity: 0.9;
  transform: scale(1.6);
  background: var(--accent);
}
.rmd-minimap {
  position: fixed;
  right: 0;
  top: 8px;
  bottom: 8px;
  width: 14px;
  z-index: 60;
  pointer-events: auto;
}
.rmd-minimap-tick {
  position: absolute;
  left: 3px;
  right: 3px;
  height: 4px;
  background: var(--change);
  border-radius: 2px;
  cursor: pointer;
  opacity: 0.55;
  margin-top: -2px;
  transition: opacity 0.12s ease, transform 0.12s ease;
}
.rmd-minimap-tick:hover {
  opacity: 1;
  transform: scaleX(1.7);
}
.rmd-prev-banner {
  font-size: 0.68em;
  font-weight: 500;
  font-style: italic;
  color: var(--muted);
  margin: 0.5em 0 0 0;
  padding-top: 0.25em;
  border-top: 1px solid var(--border);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.rmd-diff {
  font-family: "JetBrains Mono", "Fira Code", "Cascadia Mono",
               "DejaVu Sans Mono", ui-monospace, monospace;
  font-size: 0.92em;
  line-height: 1.5;
  margin: 0.25em 0;
}
.rmd-diff-line {
  display: flex;
  white-space: pre-wrap;
  padding: 0 0.25em;
  border-radius: 2px;
}
.rmd-diff-mark {
  flex: 0 0 1.4em;
  text-align: center;
  font-weight: 600;
  user-select: none;
  opacity: 0.85;
}
.rmd-diff-text { flex: 1 1 auto; }
.rmd-diff-add { background: rgba(31, 136, 61, 0.14); color: var(--alert-tip); }
.rmd-diff-del { background: rgba(207, 34, 46, 0.14); color: var(--alert-caution); }
.rmd-diff-mod { background: rgba(227, 179, 65, 0.08); }
.rmd-diff-eq  { color: var(--muted); }
.rmd-diff-w-del {
  background: rgba(207, 34, 46, 0.22);
  color: var(--alert-caution);
  text-decoration: line-through;
  border-radius: 2px;
  padding: 0 2px;
}
.rmd-diff-w-add {
  background: rgba(31, 136, 61, 0.22);
  color: var(--alert-tip);
  text-decoration: none;
  border-radius: 2px;
  padding: 0 2px;
}
"#;

// The `__rmdPost` shim in <head> is the preview's single IPC channel back to
// the shell: it forwards {rmd:1, channel, payload} to the parent frame via
// postMessage, where the Tauri shell listens and dispatches. Swallowing
// errors keeps exported/standalone HTML inert.
pub const HTML_TEMPLATE: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<script>window.__rmdPost=function(channel,payload){try{window.parent.postMessage({rmd:1,channel:channel,payload:payload},"*")}catch(e){}};</script>
<title>{TITLE}</title>
<style>
{THEME_CSS}
{BASE_CSS}
</style>
<base href="{BASE_HREF}">
</head>
<body>
{BODY}
{MERMAID_SCRIPT}
{IMAGE_CLICK_JS}
{PREVIEW_BRIDGE_JS}
</body>
</html>
"#;

// Persistent shell↔preview bridge. Replaces the GTK app's per-refresh
// injected one-shot <script>s (scroll targeting) and its
// `evaluate_javascript` queries (topmost-visible line): the shell now sends
// {rmd:1, cmd, ...} messages INTO the frame, and the frame answers on the
// __rmdPost channel. The scroll-targeting and top-line algorithms are ports
// of the GTK versions (settle timers included — mermaid SVGs and late image
// loads must not push the target offscreen).
//
// Line conventions (identical to the GTK app):
//   - `scrollToSourceLine` line 0  = "scroll to end" sentinel
//   - `topLine` reply: -1 = at end, 0 = unknown, else 1-based source line
pub const PREVIEW_BRIDGE_JS: &str = r#"<script>(function(){
  if (!window.__rmdPost) return;
  function topVisibleLine() {
    var doc = document.documentElement;
    if (window.scrollY + window.innerHeight >= doc.scrollHeight - 2) return -1;
    var els = document.querySelectorAll('[data-sourcepos]');
    for (var i = 0; i < els.length; i++) {
      var r = els[i].getBoundingClientRect();
      if (r.height === 0) continue;
      if (r.bottom > 0) {
        var m = (els[i].getAttribute('data-sourcepos') || '').match(/^(\d+):/);
        return m ? parseInt(m[1], 10) : 0;
      }
    }
    return 0;
  }
  function target(line) {
    var els = document.querySelectorAll('[data-sourcepos]');
    var best = null;
    var bestDelta = Infinity;
    for (var i = 0; i < els.length; i++) {
      var sp = els[i].getAttribute('data-sourcepos') || '';
      var m = sp.match(/^(\d+):\d+-(\d+):/);
      if (!m) continue;
      var start = parseInt(m[1], 10);
      var end = parseInt(m[2], 10);
      if (line >= start && line <= end) return els[i];
      var delta = line < start ? start - line : line - end;
      if (delta < bestDelta) { bestDelta = delta; best = els[i]; }
    }
    return best;
  }
  function scrollToSourceLine(line) {
    function go() {
      if (line === 0) { window.scrollTo(0, document.documentElement.scrollHeight); return; }
      var t = target(line);
      if (!t) return;
      var rect = t.getBoundingClientRect();
      window.scrollTo(0, Math.max(0, rect.top + window.scrollY));
    }
    go(); setTimeout(go, 150); setTimeout(go, 600);
  }
  window.addEventListener('message', function(e) {
    var d = e.data || {};
    if (d.rmd !== 1 || !d.cmd) return;
    if (d.cmd === 'scrollToSourceLine') scrollToSourceLine(d.line | 0);
    else if (d.cmd === 'reportTopLine') window.__rmdPost('topLine', String(topVisibleLine()));
    else if (d.cmd === 'focusCell') focusCell(d.tid, d.r, d.c, 10);
    else if (d.cmd === 'print') window.print();
  });
  // TABLE_EDIT_JS may install rmdFocusCell a tick after this script runs —
  // retry briefly, matching the GTK app's injected one-shot.
  function focusCell(tid, r, c, retries) {
    var fn = window.rmdFocusCell;
    if (!fn) {
      if (retries > 0) setTimeout(function() { focusCell(tid, r, c, retries - 1); }, 30);
      return;
    }
    fn(tid, r, c);
  }
  var scrollTimer = null;
  window.addEventListener('scroll', function() {
    if (scrollTimer) return;
    scrollTimer = setTimeout(function() {
      scrollTimer = null;
      window.__rmdPost('previewScrolled', String(topVisibleLine()));
    }, 200);
  }, { passive: true });
})();</script>"#;

// Image interactions in the rendered preview:
//   - Hover an image: 4 corner handles fade in.
//   - Drag a corner: live-resize, post `imageResize` (src\twidth\talt) on release.
//   - Mousedown on the image body: tracks for click vs drag.
//       * Released without moving: post `imageClick` to open the options dialog.
//       * Moved past a small threshold: drag-to-move; an insertion line
//         tracks the cursor's nearest gap between top-level blocks. On
//         release, post `imageMove` (src\ttargetIndex).
// All no-ops outside the shell's preview frame (no `__rmdPost` shim), so this
// is safe to embed unconditionally — exported HTML files just see static
// images.
pub const IMAGE_CLICK_JS: &str = r#"<script>
(function() {
  if (!window.__rmdPost) return;
  function send(name, payload) {
    window.__rmdPost(name, payload);
  }
  function setupImage(img) {
    if (img.dataset.rmdSetup) return;
    img.dataset.rmdSetup = "1";
    var wrap = document.createElement("span");
    wrap.className = "rmd-img-wrapper";
    img.parentNode.insertBefore(wrap, img);
    wrap.appendChild(img);
    ["nw","ne","sw","se"].forEach(function(corner) {
      var h = document.createElement("span");
      h.className = "rmd-img-handle rmd-img-handle-" + corner;
      wrap.appendChild(h);
      h.addEventListener("mousedown", function(e) {
        e.preventDefault();
        e.stopPropagation();
        startResize(e, img, corner);
      });
    });
    img.addEventListener("mousedown", function(e) {
      if (e.button !== 0) return;
      if (e.target !== img) return;
      e.preventDefault();
      startMaybeMove(e, img);
    });
    img.addEventListener("dragstart", function(e) { e.preventDefault(); });
  }
  function startResize(e, img, corner) {
    var startX = e.clientX;
    var startW = img.offsetWidth || img.naturalWidth || 200;
    var startH = img.offsetHeight || img.naturalHeight || 200;
    var aspect = startW / Math.max(1, startH);
    function onMove(ev) {
      var dx = ev.clientX - startX;
      var sign = (corner === "ne" || corner === "se") ? 1 : -1;
      var newW = Math.max(40, Math.round(startW + sign * dx));
      img.style.width = newW + "px";
      img.style.height = Math.round(newW / aspect) + "px";
    }
    function onUp() {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      var finalW = Math.round(img.offsetWidth);
      var src = img.getAttribute("src") || "";
      var alt = img.getAttribute("alt") || "";
      send("imageResize", src + "\t" + finalW + "\t" + alt);
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }
  // Drop targets are individual image wrappers in document order. This
  // way the user can drop "between" two images regardless of whether
  // they're each in their own paragraph (stacked) or side-by-side
  // inline within one paragraph.
  function dropTargets(dragWrapper) {
    return Array.from(document.querySelectorAll(".rmd-img-wrapper"))
      .filter(function(w) { return w !== dragWrapper; });
  }
  // "Before" a wrapper: cursor is above it (block layout) OR on the
  // same line and to the left of its midpoint (inline layout).
  function isBefore(x, y, rect) {
    if (y < rect.top) return true;
    if (y > rect.bottom) return false;
    return x < rect.left + rect.width / 2;
  }
  function dropIndexFor(wrapper, x, y) {
    var targets = dropTargets(wrapper);
    for (var i = 0; i < targets.length; i++) {
      if (isBefore(x, y, targets[i].getBoundingClientRect())) return i;
    }
    return targets.length;
  }
  // Make-room animation: as the ghost approaches a gap, blocks at and
  // after the candidate drop position smoothly translate downward by
  // the dragged image's height. The user sees the existing images
  // physically slide aside, previewing the post-drop layout.
  var spacedBlocks = [];
  function applySpacing(wrapper, idx, height) {
    var blocks = dropTargets(wrapper);
    var newSpaced = blocks.slice(idx);
    // Reset any block that's no longer in the "spaced" set.
    spacedBlocks.forEach(function(el) {
      if (newSpaced.indexOf(el) === -1) {
        el.style.transform = "";
      }
    });
    // Apply to the new set. transition is set once and left in place
    // for the duration of the drag.
    newSpaced.forEach(function(el) {
      if (el.dataset.rmdSpaced !== "1") {
        el.style.transition = "transform 0.18s ease";
        el.dataset.rmdSpaced = "1";
      }
      el.style.transform = "translateY(" + height + "px)";
    });
    spacedBlocks = newSpaced;
  }
  function clearSpacing() {
    spacedBlocks.forEach(function(el) {
      el.style.transform = "";
      el.style.transition = "";
      delete el.dataset.rmdSpaced;
    });
    spacedBlocks = [];
  }

  function startMaybeMove(e, img) {
    var wrapper = img.parentElement;
    var startX = e.clientX, startY = e.clientY;
    var rect = wrapper.getBoundingClientRect();
    var grabX = startX - rect.left;
    var grabY = startY - rect.top;
    var lockedWidth = Math.min(rect.width, 360);
    var lockedHeight = Math.round(
      (lockedWidth / Math.max(1, rect.width)) * rect.height
    );
    var moved = false;
    var lastIdx = -1;
    function onMove(ev) {
      var dx = ev.clientX - startX, dy = ev.clientY - startY;
      if (!moved && (Math.abs(dx) > 2 || Math.abs(dy) > 2)) {
        moved = true;
        wrapper.classList.add("rmd-img-dragging");
        // Float the wrapper at cursor position so the cursor smoothly
        // carries the image around.
        wrapper.style.position = "fixed";
        wrapper.style.zIndex = "1000";
        wrapper.style.pointerEvents = "none";
        wrapper.style.width = lockedWidth + "px";
        wrapper.style.height = "auto";
        wrapper.style.margin = "0";
      }
      if (moved) {
        wrapper.style.left = (ev.clientX - grabX) + "px";
        wrapper.style.top = (ev.clientY - grabY) + "px";
        var idx = dropIndexFor(wrapper, ev.clientX, ev.clientY);
        if (idx !== lastIdx) {
          applySpacing(wrapper, idx, lockedHeight);
          lastIdx = idx;
        }
      }
    }
    function onUp(ev) {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      clearSpacing();
      var src = img.getAttribute("src") || "";
      if (!moved) {
        var width = img.getAttribute("width") || "";
        var alt = img.getAttribute("alt") || "";
        send("imageClick", src + "\t" + width + "\t" + alt);
        return;
      }
      // Identify the target image by its src so Rust can locate it in
      // the buffer regardless of paragraph/inline structure. Empty
      // targetSrc means "append at end".
      var targets = dropTargets(wrapper);
      var targetSrc = "";
      if (lastIdx >= 0 && lastIdx < targets.length) {
        var tgtImg = targets[lastIdx].querySelector("img");
        targetSrc = (tgtImg && tgtImg.getAttribute("src")) || "";
      }
      send("imageMove", src + "\t" + targetSrc);
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }
  document.querySelectorAll("img").forEach(setupImage);
})();
</script>
"#;

pub const MERMAID_INIT_JS: &str = r#"
(function() {
  // Source blocks come through as <pre class="mermaid">...</pre> (a CommonMark
  // type-1 HTML block, so blank lines inside the diagram survive parsing).
  // Mermaid expects <div class="mermaid"> with whitespace-trimmed content, so
  // convert here right before mermaid.run().
  document.querySelectorAll('pre.mermaid').forEach(function(pre) {
    var div = document.createElement('div');
    div.className = 'mermaid';
    div.textContent = pre.textContent.replace(/^\s*\n/, '').replace(/\s+$/, '');
    pre.replaceWith(div);
  });
  if (typeof mermaid === 'undefined') return;
  mermaid.initialize({ startOnLoad: false, theme: '{THEME}', securityLevel: 'loose' });
  mermaid.run();
})();
"#;

/// Where the shell's preview protocol serves the vendored Mermaid bundle.
/// The Tauri shell registers a custom protocol handler that answers this
/// path with `mermaid.min.js`; the preview document pulls it in via
/// `<script src=...>` (see [`mermaid_script_tag`]).
///
/// Large scripts must NEVER be inlined into the preview document: WebKitGTK
/// wedges when a multi-megabyte inline `<script>` shares the body with a
/// `<table>` and another `<script>` follows — the WebProcess pegs CPU and
/// grows memory until OOM. Loading the bundle as an external resource
/// side-steps that pathology.
pub const MERMAID_ASSET_PATH: &str = "/assets/mermaid.min.js";

/// The `<script src>` tag referencing the shell-served Mermaid bundle at
/// [`MERMAID_ASSET_PATH`]. Injected into `{MERMAID_SCRIPT}` (ahead of the
/// init snippet) only when the document actually contains mermaid blocks.
pub fn mermaid_script_tag() -> String {
    format!("<script src=\"{}\"></script>", MERMAID_ASSET_PATH)
}
