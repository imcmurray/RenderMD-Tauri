// Dialogs: native file pickers via the dialog plugin, and an in-DOM
// three-button Save/Discard/Cancel prompt (the plugin only does two-button
// ask/confirm; the GTK app used a three-button AdwAlertDialog).

import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";

const MD_FILTERS = [
  { name: "Markdown", extensions: ["md", "markdown"] },
  { name: "Text", extensions: ["txt"] },
];

export async function pickFileToOpen(): Promise<string | null> {
  const picked = await openDialog({ multiple: false, filters: MD_FILTERS });
  return typeof picked === "string" ? picked : null;
}

export async function pickSavePath(defaultName?: string): Promise<string | null> {
  return saveDialog({ filters: MD_FILTERS, defaultPath: defaultName });
}

export type DirtyChoice = "save" | "discard" | "cancel";

/** Three-way dirty-document prompt. Resolves when the user chooses. */
export function askUnsavedChanges(fileLabel: string): Promise<DirtyChoice> {
  return new Promise((resolve) => {
    const dlg = document.createElement("dialog");
    dlg.className = "rmd-dialog";
    dlg.innerHTML = `
      <h3>Save changes?</h3>
      <p>“${escapeHtml(fileLabel)}” has unsaved changes.</p>
      <div class="rmd-dialog-buttons">
        <button data-choice="cancel">Cancel</button>
        <button data-choice="discard" class="destructive">Discard</button>
        <button data-choice="save" class="suggested" autofocus>Save</button>
      </div>`;
    dlg.addEventListener("click", (e) => {
      const btn = (e.target as HTMLElement).closest("button[data-choice]");
      if (!btn) return;
      const choice = btn.getAttribute("data-choice") as DirtyChoice;
      dlg.close();
      dlg.remove();
      resolve(choice);
    });
    dlg.addEventListener("cancel", () => {
      // Esc key
      dlg.remove();
      resolve("cancel");
    });
    document.body.appendChild(dlg);
    dlg.showModal();
  });
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}
