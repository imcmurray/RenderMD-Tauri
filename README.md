# RenderMD

A cross-platform Markdown viewer/editor. Opens `.md` files in a rendered
preview by default; one keystroke (`F5` / `Ctrl+Shift+E`) toggles to a
syntax-highlighted editor.

This is the Tauri 2 successor to the original GTK4 RenderMD (Linux-only).
The markdown pipeline moved here as the pure-Rust `rendermd-core` crate;
the shell is Tauri 2 + CodeMirror 6, which is what makes macOS and Windows
builds possible.

## Features

- **One-key toggle** between rendered preview and the editor.
- **GitHub-flavoured rendering** via comrak: tables, fenced code with syntect
  highlighting, footnotes, task lists, alerts (`> [!NOTE]`), emoji
  shortcodes, smart quotes.
- **Mermaid diagrams**, rendered by the vendored bundle (served externally —
  never inlined; see `crates/rendermd-core/src/template.rs` for why).
- **Interactive tables in the preview**: click-to-edit cells, Tab/Enter
  navigation, insert/delete rows and columns, alignment, tri-state column
  sort with original-order restore, drag column resize — every edit lands
  as a minimal, hand-formatting-preserving patch in the markdown source,
  one undo step each.
- **Smart paste**: TSV/CSV/HTML/markdown tables from the clipboard become
  pretty GFM tables; clipboard images are saved beside the document and
  referenced.
- **Images in the preview**: click for alt/width/remove options, drag
  corners to resize, drag to reorder.
- **Git history rail**: browse the file's last 100 commits with per-commit
  diff bars against the parent revision. View-only; the working copy is
  never touched.
- **External-change watching**: edits from other programs auto-reload with
  yellow change bars and hover word-diffs; unsaved local edits are never
  clobbered (reload prompt instead).
- **Atomic saves** (temp file + rename) so a power blip can't leave a
  half-written file.
- Export to standalone HTML; print (or save as PDF) via the system dialog.
- Live light/dark theme switching following the OS.

## Build

Prerequisites: Rust (stable), Node 20+, and on Linux the Tauri system deps
(`webkit2gtk-4.1`, `gtk3`; e.g. `pacman -S webkit2gtk-4.1 gtk3` or
`apt install libwebkit2gtk-4.1-dev libgtk-3-dev`).

```bash
npm ci
npx tauri dev            # develop
npx tauri build          # produce installers under src-tauri/target/release/bundle/
```

`cargo test -p rendermd-core` runs the (GUI-free) core test suite.

## Layout

```
crates/rendermd-core/   # pure-Rust markdown pipeline + table subsystem (all tests)
src-tauri/              # Tauri shell: state, commands, preview:// protocol, watcher
src/                    # frontend: CodeMirror 6 editor, preview bridge, chrome
```

## Releases

Tagging `v*` builds unsigned installers for macOS (universal .dmg),
Windows (.msi + NSIS), and Linux (.AppImage/.deb/.rpm) and attaches them to
a draft GitHub release. See `.github/workflows/release.yml` for the
unsigned-build installation caveats per OS.
