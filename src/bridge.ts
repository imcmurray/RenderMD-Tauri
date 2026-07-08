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

/** Minimal editor patch mirrored from a Rust-side table/image op.
 * Offsets are UTF-16 code units (CodeMirror positions). */
export interface DocPatch {
  from: number;
  to: number;
  insert: string;
}

export const onDocPatched = (handler: (p: DocPatch) => void): Promise<UnlistenFn> =>
  listen<DocPatch>("doc-patched", (e) => handler(e.payload));

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
