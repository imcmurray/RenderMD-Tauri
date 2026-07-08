// Typed wrappers around the Tauri IPC surface.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Mode = "preview" | "edit";

export interface DocInfo {
  path: string | null;
  text: string;
  mtime: string | null;
  mode: Mode;
  rev: number;
  dirty: boolean;
}

export const getDoc = () => invoke<DocInfo>("get_doc");
export const openFile = (path: string) => invoke<DocInfo>("open_file", { path });
export const newFile = () => invoke<DocInfo>("new_file");
export const saveFile = (text: string) => invoke<DocInfo>("save_file", { text });
export const saveFileAs = (path: string, text: string) =>
  invoke<DocInfo>("save_file_as", { path, text });
export const updateText = (text: string) => invoke<number>("update_text", { text });
export const setMode = (mode: Mode) => invoke<void>("set_mode", { mode });
export const setDark = (dark: boolean) => invoke<void>("set_dark", { dark });

export interface PreviewUpdated {
  rev: number;
  /** Cell to re-focus after the reload (table nav/structure ops). */
  focusCell?: { tid: number; r: number; c: number } | null;
}

export const onPreviewUpdated = (
  handler: (p: PreviewUpdated) => void,
): Promise<UnlistenFn> => listen<PreviewUpdated>("preview-updated", (e) => handler(e.payload));

/** Minimal editor patch mirrored from a Rust-side table/image op. One or
 * more changes applied as a single undoable transaction; offsets are UTF-16
 * code units (CodeMirror positions) in the PRE-transaction document. */
export interface DocPatch {
  changes: { from: number; to: number; insert: string }[];
}

export const onDocPatched = (handler: (p: DocPatch) => void): Promise<UnlistenFn> =>
  listen<DocPatch>("doc-patched", (e) => handler(e.payload));

/** Image-options dialog commit. */
export const imageChange = (src: string, width: string | null, alt: string, remove: boolean) =>
  invoke<void>("image_change", { src, width, alt, remove });

/** Send raw PNG bytes; returns the markdown ref to insert. */
export const pasteImage = (bytes: Uint8Array) =>
  invoke<string>("paste_image", bytes as unknown as ArrayBuffer);

/** External file change while the editor holds unsaved edits. */
export const onExternalChange = (handler: () => void): Promise<UnlistenFn> =>
  listen("external-change", () => handler());

/** Document replaced wholesale (external reload). */
export const onDocReplaced = (handler: (d: DocInfo) => void): Promise<UnlistenFn> =>
  listen<DocInfo>("doc-replaced", (e) => handler(e.payload));

export const reloadFromDisk = () => invoke<void>("reload_from_disk");

export const exportHtml = (dest: string) => invoke<void>("export_html", { dest });

export interface BuildInfo {
  version: string;
  gitSha: string;
  repoUrl: string;
}

export const getBuildInfo = () => invoke<BuildInfo>("get_build_info");

export interface ToastMsg {
  text: string;
}

export const onToast = (handler: (t: ToastMsg) => void): Promise<UnlistenFn> =>
  listen<ToastMsg>("toast", (e) => handler(e.payload));

export interface TablePasteResult {
  markdown: string;
  body_rows: number;
  cols: number;
  origin: string;
}

export const convertTablePaste = (text: string | null, html: string | null) =>
  invoke<TablePasteResult | null>("convert_table_paste", { text, html });
