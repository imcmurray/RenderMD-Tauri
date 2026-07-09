// RenderMD shell orchestrator: boot, mode switching, actions, keymap,
// dirty-close guard, statusbar.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { openUrl, openPath } from "@tauri-apps/plugin-opener";

/** Drive a Rust preview channel from the shell (keyboard shortcuts share
 * the dispatcher with in-frame clicks). */
const invokePreviewChannel = (channel: string, payload = "") =>
  invoke<void>("preview_message", { channel, payload });

import * as bridge from "./bridge";
import type { DocInfo, Mode } from "./bridge";
import { Editor } from "./editor";
import { Preview } from "./preview";
import {
  pickFileToOpen,
  pickSavePath,
  pickExportHtmlPath,
  askUnsavedChanges,
  imageOptionsDialog,
  infoDialog,
  confirmDialog,
} from "./dialogs";
import { showToast } from "./toasts";
import { initUpdater, checkForUpdate, installUpdate } from "./updater";

const els = {
  title: document.getElementById("title")!,
  toggle: document.getElementById("btn-toggle") as HTMLButtonElement,
  open: document.getElementById("btn-open") as HTMLButtonElement,
  save: document.getElementById("btn-save") as HTMLButtonElement,
  menuBtn: document.getElementById("btn-menu") as HTMLButtonElement,
  menu: document.getElementById("app-menu")!,
  editorPane: document.getElementById("editor-pane")!,
  previewPane: document.getElementById("preview-pane") as HTMLIFrameElement,
  statusPath: document.getElementById("status-path")!,
  statusMtime: document.getElementById("status-mtime")!,
  statusMode: document.getElementById("status-mode")!,
};

const editor = new Editor(els.editorPane);
const preview = new Preview(els.previewPane);

let mode: Mode = "edit";
let currentPath: string | null = null;
let currentMtime: string | null = null;
let dirty = false;

// ---------------------------------------------------------------- UI state

let lastRev = 0;

function applyDoc(doc: DocInfo) {
  currentPath = doc.path;
  currentMtime = doc.mtime;
  dirty = doc.dirty;
  lastRev = doc.rev;
  editor.setText(doc.text);
  setUiMode(doc.mode);
  if (doc.rev > 0) preview.show(doc.rev);
  refreshChrome();
}

function fileLabel(): string {
  if (!currentPath) return "Untitled";
  return currentPath.split(/[/\\]/).pop() ?? currentPath;
}

function refreshChrome() {
  const marker = dirty ? "• " : "";
  els.title.textContent = `${marker}${fileLabel()} — RenderMD`;
  els.statusPath.textContent = currentPath ?? "No file";
  els.statusMtime.textContent = currentMtime ?? "";
  els.statusMode.textContent = mode === "edit" ? "Edit" : "Preview";
  els.toggle.textContent = mode === "edit" ? "Preview" : "Edit";
}

function setUiMode(next: Mode) {
  mode = next;
  els.editorPane.hidden = next !== "edit";
  els.previewPane.hidden = next !== "preview";
  if (next === "edit") editor.focus();
  void bridge.setMode(next);
  refreshChrome();
}

async function toggleMode() {
  if (mode === "edit") {
    // Entering preview: capture the top visible line BEFORE the sync
    // re-renders, then land the preview on it (0 = end sentinel).
    const line = editor.topVisibleLine();
    await editor.flushSync();
    setUiMode("preview");
    preview.show(lastRev);
    preview.send({ cmd: "scrollToSourceLine", line });
  } else {
    // Entering edit: ask the frame for its topmost visible source line;
    // the async "topLine" reply scrolls the editor (handler below).
    preview.send({ cmd: "reportTopLine" });
    setUiMode("edit");
  }
}

// topLine reply: -1 = frame was at the end, 0 = unknown, else 1-based line.
preview.on("topLine", (payload) => {
  const n = parseInt(payload, 10);
  if (n === -1) editor.scrollToEnd();
  else if (n >= 1) editor.scrollToLine(n);
});

// External links in the preview open in the default browser. Only web +
// mail schemes are allowed: a hostile markdown link like [x](smb://evil),
// [x](file:///…), or [x](vscode://…) must never reach the OS opener (SMB
// NetNTLM leak, arbitrary local open, app-protocol abuse).
const SAFE_EXTERNAL = /^(https?|mailto):/i;
preview.on("openExternal", (url) => {
  if (SAFE_EXTERNAL.test(url)) void openUrl(url);
  else showToast("Blocked a link with a non-web scheme");
});

// Relative links resolve server-side, confined to the document's repo/dir
// root: .md files open in RenderMD (dirty-guarded); other local files
// require an explicit confirm before the system opener runs; anything
// outside the root is blocked. All confinement is authoritative in Rust —
// the frame's own (attacker-authored) script can't widen it.
preview.on("linkClick", (resolvedUrl) => {
  void (async () => {
    const res = await bridge.resolveLocalLink(resolvedUrl).catch(() => null);
    if (!res || res.action === "denied") {
      showToast("Blocked a link outside the document's folder");
      return;
    }
    if (res.action === "open-md") {
      void maybeSaveThen(async () => {
        try {
          applyDoc(await bridge.openFile(res.path));
        } catch (e) {
          showToast(String(e));
        }
      });
    } else if (res.action === "open-file") {
      const ok = await confirmDialog(
        "Open this file?",
        `“${res.path}” will open in your system's default application.`,
      );
      if (ok) void openPath(res.path).catch((e) => showToast(String(e)));
    }
  })();
});

// Image click in the preview → options dialog → Rust patch.
preview.on("imageClick", (payload) => {
  const [src = "", width = "", alt = ""] = payload.split("\t");
  if (!src) return;
  void (async () => {
    const result = await imageOptionsDialog(src, width, alt);
    if (result.action === "apply") {
      await bridge.imageChange(src, result.width, result.alt, false);
    } else if (result.action === "remove") {
      await bridge.imageChange(src, null, "", true);
    }
  })();
});

// ---------------------------------------------------------------- actions

/** Runs the dirty-guard, then `next()`. Mirrors the GTK maybe_save_then. */
async function maybeSaveThen(next: () => Promise<void>): Promise<void> {
  await editor.flushSync();
  if (!dirty) return next();
  const choice = await askUnsavedChanges(fileLabel());
  if (choice === "cancel") return;
  if (choice === "save") {
    const ok = await actionSave();
    if (!ok) return; // save-as cancelled
  }
  return next();
}

async function actionOpen() {
  await maybeSaveThen(async () => {
    const path = await pickFileToOpen();
    if (!path) return;
    applyDoc(await bridge.openFile(path));
  });
}

async function actionNew() {
  await maybeSaveThen(async () => {
    applyDoc(await bridge.newFile());
  });
}

async function actionSave(): Promise<boolean> {
  await editor.flushSync();
  try {
    const doc = await bridge.saveFile(editor.getText());
    currentMtime = doc.mtime;
    dirty = false;
    refreshChrome();
    return true;
  } catch (e) {
    if (e === "no-path") return actionSaveAs();
    console.error("save failed:", e);
    return false;
  }
}

async function actionSaveAs(): Promise<boolean> {
  const path = await pickSavePath(fileLabel());
  if (!path) return false;
  const doc = await bridge.saveFileAs(path, editor.getText());
  applyDoc(doc);
  return true;
}

// ---------------------------------------------------------------- wiring

els.open.addEventListener("click", () => void actionOpen());
els.save.addEventListener("click", () => void actionSave());
els.toggle.addEventListener("click", () => void toggleMode());

// ---------------------------------------------------------------- menu

async function actionExportHtml() {
  const base = fileLabel().replace(/\.(md|markdown)$/i, "");
  const dest = await pickExportHtmlPath(`${base}.html`);
  if (!dest) return;
  await editor.flushSync();
  try {
    await bridge.exportHtml(dest);
    showToast(`Exported: ${dest}`);
  } catch (e) {
    showToast(`Export failed: ${e}`);
  }
}

async function actionPrint() {
  // The frame prints itself (window.print is not callable cross-origin);
  // every OS print dialog offers Save-as-PDF. Make sure the preview is
  // current and visible first.
  if (mode === "edit") await toggleMode();
  preview.send({ cmd: "print" });
}

async function actionAbout() {
  const info = await bridge.getBuildInfo();
  infoDialog(
    "RenderMD",
    `<p>A cross-platform Markdown viewer/editor.</p>
     <p>Version ${info.version}
        (<a href="${info.repoUrl}/commit/${info.gitSha}">${info.gitSha}</a>)</p>
     <p>Rendering: comrak (GFM) + syntect · mermaid · CodeMirror 6 · Tauri 2</p>
     <p><a href="${info.repoUrl}">Repository</a> ·
        <a href="${info.repoUrl}/issues">Report an issue</a> ·
        <a href="${info.repoUrl}/releases">Releases</a></p>`,
  );
}

// Any <a href="http…"> inside the shell (About and friends) opens in the
// default browser — the app window must never navigate away.
window.addEventListener("click", (e) => {
  const a = (e.target as HTMLElement).closest?.("a[href]");
  if (!a) return;
  const href = a.getAttribute("href") ?? "";
  if (/^https?:\/\//i.test(href)) {
    e.preventDefault();
    void openUrl(href);
  }
});

function actionShortcuts() {
  infoDialog(
    "Keyboard shortcuts",
    `<table class="rmd-shortcuts">
      <tr><td>F5 / Ctrl+Shift+E</td><td>Toggle preview ↔ edit</td></tr>
      <tr><td>Ctrl+N</td><td>New document</td></tr>
      <tr><td>Ctrl+O</td><td>Open…</td></tr>
      <tr><td>Ctrl+S</td><td>Save</td></tr>
      <tr><td>Ctrl+Shift+S</td><td>Save as…</td></tr>
      <tr><td>Ctrl+Z / Ctrl+Shift+Z</td><td>Undo / redo (edit mode)</td></tr>
      <tr><td>Ctrl+Alt+H</td><td>Toggle history rail</td></tr>
      <tr><td>Ctrl+Q / Ctrl+W</td><td>Quit</td></tr>
      <tr><td>Tab / Enter in table cell</td><td>Navigate cells (preview)</td></tr>
    </table>`,
  );
}

els.menuBtn.addEventListener("click", (e) => {
  e.stopPropagation();
  els.menu.hidden = !els.menu.hidden;
});
window.addEventListener("click", () => {
  els.menu.hidden = true;
});
els.menu.addEventListener("click", (e) => {
  const btn = (e.target as HTMLElement).closest("button[data-action]");
  if (!btn) return;
  els.menu.hidden = true;
  switch (btn.getAttribute("data-action")) {
    case "new":
      void actionNew();
      break;
    case "open":
      void actionOpen();
      break;
    case "save":
      void actionSave();
      break;
    case "save-as":
      void actionSaveAs();
      break;
    case "export-html":
      void actionExportHtml();
      break;
    case "print":
      void actionPrint();
      break;
    case "toggle-history":
      void invokePreviewChannel("toggleHistory");
      break;
    case "install-update":
      void installUpdate();
      break;
    case "check-updates":
      void checkForUpdate(true);
      break;
    case "shortcuts":
      actionShortcuts();
      break;
    case "about":
      void actionAbout();
      break;
  }
});

// ------------------------------------------------------------- updater

const updateChip = document.getElementById("status-update") as HTMLButtonElement;
const menuUpdate = document.getElementById("menu-update") as HTMLButtonElement;
const menuUpdateVer = document.getElementById("menu-update-ver")!;

updateChip.addEventListener("click", () => void installUpdate());

initUpdater({
  setAvailable: (version) => {
    const has = version !== null;
    updateChip.hidden = !has;
    menuUpdate.hidden = !has;
    if (has) {
      updateChip.textContent = `⬆ Update to v${version}`;
      menuUpdateVer.textContent = `v${version}`;
    }
  },
  // The same never-lose-work guard as open/close: flush, then
  // Save/Discard/Cancel if dirty. True = safe to install.
  confirmSaved: async () => {
    await editor.flushSync();
    if (!dirty) return true;
    const choice = await askUnsavedChanges(fileLabel());
    if (choice === "cancel") return false;
    if (choice === "save") return actionSave();
    return true;
  },
});

// Global accelerators (matching the GTK app's accel table).
window.addEventListener("keydown", (e) => {
  const ctrl = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();
  if (e.key === "F5" || (ctrl && e.shiftKey && key === "e")) {
    e.preventDefault();
    void toggleMode();
  } else if (ctrl && !e.shiftKey && key === "s") {
    e.preventDefault();
    void actionSave();
  } else if (ctrl && e.shiftKey && key === "s") {
    e.preventDefault();
    void actionSaveAs();
  } else if (ctrl && key === "o") {
    e.preventDefault();
    void actionOpen();
  } else if (ctrl && key === "n") {
    e.preventDefault();
    void actionNew();
  } else if (ctrl && e.altKey && key === "h") {
    e.preventDefault();
    void invokePreviewChannel("toggleHistory");
  } else if (ctrl && (key === "?" || (e.shiftKey && key === "/"))) {
    e.preventDefault();
    actionShortcuts();
  } else if (ctrl && (key === "q" || key === "w")) {
    e.preventDefault();
    void requestClose();
  }
});

// Dirty tracking: any non-remote editor change marks the doc dirty
// immediately (before the debounced sync lands).
editor.onSynced = () => {
  if (!dirty) {
    dirty = true;
    refreshChrome();
  }
};

// Live preview refresh. When the visible preview re-renders (table edits,
// theme flips, external reloads), restore the last reported scroll position
// — the iframe src swap would otherwise reset it to the top. Table ops may
// also carry a focus cell to re-enter editing after the reload.
void bridge.onPreviewUpdated(({ rev, focusCell }) => {
  lastRev = rev;
  if (mode === "preview") {
    const line = preview.lastScrolledLine;
    preview.show(rev);
    if (focusCell) {
      preview.send({ cmd: "focusCell", tid: focusCell.tid, r: focusCell.r, c: focusCell.c });
    } else if (line !== 0) {
      // previewScrolled uses -1 for "at end"; scrollToSourceLine uses 0.
      preview.send({ cmd: "scrollToSourceLine", line: line === -1 ? 0 : line });
    }
  }
});

// Rust-side buffer patches (table/image ops) mirrored into the editor.
void bridge.onDocPatched((patch) => {
  editor.applyPatch(patch);
  if (!dirty) {
    dirty = true;
    refreshChrome();
  }
});

// Toasts surfaced by Rust handlers.
void bridge.onToast(({ text }) => showToast(text));

// External change while edit-mode-dirty: never clobber, offer a reload.
void bridge.onExternalChange(() => {
  showToast("File changed on disk — you have unsaved edits", {
    label: "Reload",
    onClick: () => void bridge.reloadFromDisk(),
  });
});

// External reload landed: mirror the new document into the editor without
// changing the current mode. (Change bars ride the next preview render.)
void bridge.onDocReplaced((doc) => {
  currentPath = doc.path;
  currentMtime = doc.mtime;
  dirty = doc.dirty;
  lastRev = doc.rev;
  editor.setText(doc.text);
  refreshChrome();
});

// Dirty-guarded window close.
const appWindow = getCurrentWindow();
let closing = false;

async function requestClose() {
  await maybeSaveThen(async () => {
    closing = true;
    await appWindow.destroy();
  });
}

void appWindow.onCloseRequested((event) => {
  if (closing) return;
  if (dirty) {
    event.preventDefault();
    void requestClose();
  }
});

// Drag-drop file open (OS paths come from Tauri, not DOM drop events).
import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => {
  void getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type !== "drop" || event.payload.paths.length === 0) return;
    const path = event.payload.paths[0];
    if (!/\.(md|markdown|txt)$/i.test(path)) {
      showToast("Drop a .md file to open it");
      return;
    }
    void maybeSaveThen(async () => {
      applyDoc(await bridge.openFile(path));
    });
  });
});

// Theme: keep Rust's renderer in sync with the OS scheme.
const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
void bridge.setDark(darkQuery.matches);
darkQuery.addEventListener("change", (e) => void bridge.setDark(e.matches));

// ---------------------------------------------------------------- boot

async function boot() {
  const doc = await bridge.getDoc();
  applyDoc(doc);
}

void boot();
