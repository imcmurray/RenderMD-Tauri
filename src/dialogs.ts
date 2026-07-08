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

export async function pickExportHtmlPath(defaultName: string): Promise<string | null> {
  return saveDialog({
    filters: [{ name: "HTML", extensions: ["html", "htm"] }],
    defaultPath: defaultName,
  });
}

/** Simple informational dialog with a title and pre-formatted body HTML. */
export function infoDialog(title: string, bodyHtml: string) {
  const dlg = document.createElement("dialog");
  dlg.className = "rmd-dialog";
  dlg.innerHTML = `
    <h3>${escapeHtml(title)}</h3>
    <div class="rmd-dialog-body">${bodyHtml}</div>
    <div class="rmd-dialog-buttons">
      <button class="suggested" autofocus>Close</button>
    </div>`;
  dlg.querySelector("button")!.addEventListener("click", () => {
    dlg.close();
    dlg.remove();
  });
  dlg.addEventListener("cancel", () => dlg.remove());
  document.body.appendChild(dlg);
  dlg.showModal();
}

export type ImageOptionsResult =
  | { action: "apply"; width: string | null; alt: string }
  | { action: "remove" }
  | { action: "cancel" };

/** Image options: edit alt text / width, or remove the image. The web
 * equivalent of the GTK AlertDialog with the two-entry form. */
export function imageOptionsDialog(
  src: string,
  currentWidth: string,
  currentAlt: string,
): Promise<ImageOptionsResult> {
  return new Promise((resolve) => {
    const dlg = document.createElement("dialog");
    dlg.className = "rmd-dialog";
    dlg.innerHTML = `
      <h3>Image options</h3>
      <p class="rmd-dialog-src">${escapeHtml(src)}</p>
      <label>Alt text
        <input type="text" name="alt" value="${escapeHtml(currentAlt)}">
      </label>
      <label>Width (px, blank for auto)
        <input type="text" name="width" maxlength="8" value="${escapeHtml(currentWidth)}">
      </label>
      <div class="rmd-dialog-buttons">
        <button data-choice="cancel">Cancel</button>
        <button data-choice="remove" class="destructive">Remove</button>
        <button data-choice="apply" class="suggested" autofocus>Apply</button>
      </div>`;

    const done = (result: ImageOptionsResult) => {
      dlg.close();
      dlg.remove();
      resolve(result);
    };
    dlg.addEventListener("click", (e) => {
      const btn = (e.target as HTMLElement).closest("button[data-choice]");
      if (!btn) return;
      const choice = btn.getAttribute("data-choice");
      if (choice === "apply") {
        const alt = (dlg.querySelector('input[name="alt"]') as HTMLInputElement).value;
        const width = (dlg.querySelector('input[name="width"]') as HTMLInputElement).value.trim();
        done({ action: "apply", width: width === "" ? null : width, alt });
      } else if (choice === "remove") {
        done({ action: "remove" });
      } else {
        done({ action: "cancel" });
      }
    });
    dlg.addEventListener("cancel", () => {
      dlg.remove();
      resolve({ action: "cancel" });
    });
    document.body.appendChild(dlg);
    dlg.showModal();
  });
}
