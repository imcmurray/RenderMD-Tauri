// RenderMD shell orchestrator: boot, mode switching, actions, keymap,
// dirty-close guard, statusbar.

import { getCurrentWindow } from "@tauri-apps/api/window";

import * as bridge from "./bridge";
import type { DocInfo, Mode } from "./bridge";
import { Editor } from "./editor";
import { Preview } from "./preview";
import {
  pickFileToOpen,
  pickSavePath,
  askUnsavedChanges,
  imageOptionsDialog,
} from "./dialogs";
import { showToast } from "./toasts";

const els = {
  title: document.getElementById("title")!,
  toggle: document.getElementById("btn-toggle") as HTMLButtonElement,
  open: document.getElementById("btn-open") as HTMLButtonElement,
  save: document.getElementById("btn-save") as HTMLButtonElement,
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
