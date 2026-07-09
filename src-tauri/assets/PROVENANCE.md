# Vendored assets

## mermaid.min.js

A pre-built minified Mermaid UMD bundle, embedded into the binary
(`include_bytes!` in `src/preview_protocol.rs`) and served to the preview
webview at `preview://localhost/assets/mermaid.min.js`. It is served
**externally, never inlined** — a multi-MB inline `<script>` next to a
`<table>` wedges WebKitGTK's WebProcess (busy-loop + OOM).

- Carried over from the original GTK RenderMD's `data/js/mermaid.min.js`.
- SHA-256 pinned in `mermaid.min.js.sha256` and **verified in CI**
  (`.github/workflows/check.yml`), so any change to the blob is an explicit,
  reviewable event rather than a silent supply-chain drift.
- The minified bundle does not carry a clear top-level Mermaid version
  string; treat a version bump as: replace the file, update the `.sha256`,
  and note the source + version here in the same commit.

To refresh from upstream, prefer a pinned npm download whose integrity hash
you can check, e.g.:

```
npm pack mermaid@<version>          # or download the dist bundle at a tag
# extract dist/mermaid.min.js, then:
sha256sum mermaid.min.js > mermaid.min.js.sha256
```
