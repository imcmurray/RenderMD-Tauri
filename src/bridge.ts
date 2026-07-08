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
}

export const onPreviewUpdated = (
  handler: (p: PreviewUpdated) => void,
): Promise<UnlistenFn> => listen<PreviewUpdated>("preview-updated", (e) => handler(e.payload));
