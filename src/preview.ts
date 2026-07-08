// Preview pane — an iframe navigated to the preview:// document, plus the
// postMessage bridge to/from the sandboxed preview frame.
//
// INBOUND (frame → shell): the template's __rmdPost shim posts
// {rmd:1, channel, payload}. Channels with a registered local handler are
// consumed in the frontend; everything else is forwarded to the Rust
// `preview_message` dispatcher (tables, images, history...).
//
// OUTBOUND (shell → frame): send({cmd, ...}) — queued until the current
// document finishes loading, so scroll targets survive re-renders.

import { invoke } from "@tauri-apps/api/core";

/** Windows maps custom schemes to http://<scheme>.localhost. */
const PREVIEW_ORIGIN = navigator.userAgent.includes("Windows")
  ? "http://preview.localhost"
  : "preview://localhost";

type LocalHandler = (payload: string) => void;

export class Preview {
  private iframe: HTMLIFrameElement;
  private localHandlers = new Map<string, LocalHandler>();
  private pendingCmds: Record<string, unknown>[] = [];
  private loaded = false;
  private currentRev = -1;

  /** Last topmost-visible source line the frame reported while scrolling.
   * -1 = at end, 0 = unknown, else 1-based. Used to restore position across
   * re-renders. */
  lastScrolledLine = 0;

  constructor(iframe: HTMLIFrameElement) {
    this.iframe = iframe;

    window.addEventListener("message", (e) => {
      const d = e.data as { rmd?: number; channel?: string; payload?: unknown };
      if (!d || d.rmd !== 1 || !d.channel) return;
      const payload = String(d.payload ?? "");

      if (d.channel === "previewScrolled") {
        const n = parseInt(payload, 10);
        if (!Number.isNaN(n)) this.lastScrolledLine = n;
        return;
      }
      const local = this.localHandlers.get(d.channel);
      if (local) {
        local(payload);
        return;
      }
      invoke("preview_message", { channel: d.channel, payload }).catch((err) =>
        console.error(`preview_message(${d.channel}) failed:`, err),
      );
    });

    iframe.addEventListener("load", () => {
      this.loaded = true;
      const queued = this.pendingCmds;
      this.pendingCmds = [];
      for (const cmd of queued) this.post(cmd);
    });
  }

  /** Handle a frame channel in the frontend instead of forwarding to Rust. */
  on(channel: string, handler: LocalHandler) {
    this.localHandlers.set(channel, handler);
  }

  /** Load (or reload) the rendered document at the given revision. */
  show(rev: number) {
    if (rev === this.currentRev && this.loaded) return;
    this.currentRev = rev;
    this.loaded = false;
    this.iframe.src = `${PREVIEW_ORIGIN}/doc.html?rev=${rev}`;
  }

  /** Post a command into the frame, waiting for the document if needed. */
  send(cmd: Record<string, unknown>) {
    if (this.loaded) this.post(cmd);
    else this.pendingCmds.push(cmd);
  }

  private post(cmd: Record<string, unknown>) {
    this.iframe.contentWindow?.postMessage({ rmd: 1, ...cmd }, "*");
  }
}
