// In-app update flow.
//
// On startup (and via the menu's "Check for updates"), ask the updater
// endpoint whether a newer signed release exists. If so, surface a chip in
// the status bar + a menu entry. Clicking either runs the dirty-guard
// (never lose an unsaved document to an update), downloads + installs with
// progress toasts, then relaunches into the new version.
//
// Self-update works for the NSIS install (Windows), the .app bundle
// (macOS), and the AppImage (Linux). For .deb/.rpm installs the check
// simply reports nothing — those update through the package manager.

import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

import { showToast } from "./toasts";

let available: Update | null = null;
let installing = false;

/** Hook points the shell provides. */
export interface UpdaterUi {
  /** Show/hide the "Update available" affordances (chip + menu entry). */
  setAvailable: (version: string | null) => void;
  /** The dirty-guard: resolves `true` when it's safe to proceed. */
  confirmSaved: () => Promise<boolean>;
}

let ui: UpdaterUi | null = null;

export function initUpdater(hooks: UpdaterUi) {
  ui = hooks;
  // First check shortly after boot; then every 6 hours for long-running
  // instances.
  setTimeout(() => void checkForUpdate(false), 3000);
  setInterval(() => void checkForUpdate(false), 6 * 60 * 60 * 1000);
}

export async function checkForUpdate(interactive: boolean): Promise<void> {
  try {
    const update = await check();
    if (update) {
      available = update;
      ui?.setAvailable(update.version);
      if (interactive) showToast(`Update available: v${update.version}`);
    } else {
      available = null;
      ui?.setAvailable(null);
      if (interactive) showToast("You're on the latest version");
    }
  } catch (e) {
    // No network, unsigned dev build, or a package-manager install —
    // silently stay quiet unless the user explicitly asked.
    if (interactive) showToast(`Update check failed: ${e}`);
  }
}

export async function installUpdate(): Promise<void> {
  if (!available || installing || !ui) return;
  const ok = await ui.confirmSaved();
  if (!ok) return;

  installing = true;
  const version = available.version;
  showToast(`Downloading v${version}…`);
  try {
    let total = 0;
    let received = 0;
    let lastPct = -20;
    await available.downloadAndInstall((event) => {
      if (event.event === "Started") {
        total = event.data.contentLength ?? 0;
      } else if (event.event === "Progress") {
        received += event.data.chunkLength;
        if (total > 0) {
          const pct = Math.floor((received / total) * 100);
          if (pct >= lastPct + 25) {
            lastPct = pct;
            showToast(`Downloading v${version}: ${pct}%`);
          }
        }
      } else if (event.event === "Finished") {
        showToast("Installing… the app will restart");
      }
    });
    // Windows (NSIS passive) exits the app during install; on macOS/Linux
    // we relaunch explicitly.
    await relaunch();
  } catch (e) {
    installing = false;
    showToast(`Update failed: ${e}`);
  }
}
