// Preview pane — an iframe navigated to the preview:// document. Each
// re-render bumps `rev`; swapping the src busts the webview cache.

/** Origin of the preview protocol: Windows maps custom schemes to
 * http://<scheme>.localhost, everywhere else it's <scheme>://localhost. */
const PREVIEW_ORIGIN = navigator.userAgent.includes("Windows")
  ? "http://preview.localhost"
  : "preview://localhost";

export class Preview {
  private iframe: HTMLIFrameElement;

  constructor(iframe: HTMLIFrameElement) {
    this.iframe = iframe;
  }

  /** Reload the rendered document at the given revision. */
  show(rev: number) {
    this.iframe.src = `${PREVIEW_ORIGIN}/doc.html?rev=${rev}`;
  }
}
