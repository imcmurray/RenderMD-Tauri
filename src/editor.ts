// CodeMirror 6 editor pane. The Rust side owns the buffer of record; this
// document is a mirror kept in sync via a debounced update_text and, later
// (Phase 4), remote patches applied as transactions.

import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { EditorState, Annotation } from "@codemirror/state";
import {
  defaultKeymap,
  history,
  historyKeymap,
  undo,
  redo,
  indentWithTab,
} from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { markdown } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";

import { updateText, convertTablePaste, pasteImage, type DocPatch } from "./bridge";
import { showToast } from "./toasts";

/** Transactions carrying this annotation came FROM Rust — don't echo back. */
export const remotePatch = Annotation.define<boolean>();

const SYNC_DEBOUNCE_MS = 200;

export class Editor {
  readonly view: EditorView;
  private syncTimer: ReturnType<typeof setTimeout> | null = null;
  /** Called after each successful text sync (dirty-state bookkeeping). */
  onSynced: (() => void) | null = null;

  constructor(parent: HTMLElement) {
    this.view = new EditorView({
      state: this.freshState(""),
      parent,
    });
  }

  private freshState(doc: string): EditorState {
    return EditorState.create({
      doc,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
        markdown({ codeLanguages: languages }),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        EditorView.lineWrapping,
        EditorView.updateListener.of((u) => {
          if (!u.docChanged) return;
          // Remote patches already live in Rust's buffer.
          if (u.transactions.some((t) => t.annotation(remotePatch))) return;
          this.scheduleSync();
        }),
        EditorView.domEventHandlers({
          paste: (event, view) => this.handlePaste(event, view),
        }),
      ],
    });
  }

  /** Smart paste, in priority order: (1) a clipboard image is saved beside
   * the document and inserted as a markdown ref; (2) table-shaped content
   * (TSV/CSV/HTML/GFM) becomes a pretty GFM table with block padding;
   * (3) plain text insert. Mirrors the GTK paste interception. */
  private handlePaste(event: ClipboardEvent, view: EditorView): boolean {
    const cd = event.clipboardData;
    if (!cd) return false;

    const imageItem = Array.from(cd.items).find((i) => i.type.startsWith("image/"));
    if (imageItem) {
      const file = imageItem.getAsFile();
      if (file) {
        event.preventDefault();
        void (async () => {
          try {
            const bytes = new Uint8Array(await file.arrayBuffer());
            const mdRef = await pasteImage(bytes);
            view.dispatch(view.state.replaceSelection(mdRef), {
              userEvent: "input.paste",
              scrollIntoView: true,
            });
            const rel = /\((.*)\)/.exec(mdRef)?.[1] ?? "";
            showToast(`Image saved: ${decodeURIComponent(rel)}`);
          } catch (e) {
            showToast(String(e));
          }
        })();
        return true;
      }
    }

    const text = cd.getData("text/plain") || null;
    const html = cd.getData("text/html") || null;
    if (!text && !html) return false;

    event.preventDefault();
    void (async () => {
      const result = await convertTablePaste(text, html).catch(() => null);
      if (result) {
        this.insertTableAtCursor(view, result.markdown);
        showToast(
          `Pasted as table (${result.body_rows}×${result.cols} from ${result.origin}) — Ctrl+Z to undo`,
        );
      } else if (text) {
        view.dispatch(view.state.replaceSelection(text), {
          userEvent: "input.paste",
          scrollIntoView: true,
        });
      }
    })();
    return true;
  }

  /** Insert a GFM table at the cursor with blank-line padding so it parses
   * as a block (port of the GTK insert_table_at_cursor). One undo step. */
  private insertTableAtCursor(view: EditorView, md: string) {
    const state = view.state;
    const sel = state.selection.main;
    const line = state.doc.lineAt(sel.from);
    const atLineStart = sel.from === line.from;
    const atDocStart = sel.from === 0;
    const prevLineBlank =
      atLineStart && !atDocStart
        ? state.doc.line(line.number - 1).text.trim() === ""
        : atDocStart;

    let prefix = "";
    if (!atLineStart) {
      prefix = "\n\n"; // break out of the current line + blank separator
    } else if (!prevLineBlank) {
      prefix = "\n"; // blank separator above
    }
    const insertion = `${prefix}${md.replace(/\n+$/, "")}\n\n`;

    view.dispatch({
      changes: { from: sel.from, to: sel.to, insert: insertion },
      selection: { anchor: sel.from + insertion.length },
      userEvent: "input.paste",
      scrollIntoView: true,
    });
  }

  /** Apply a Rust-side minimal patch (UTF-16 offsets) as one undoable
   * transaction, annotated so the update listener doesn't echo it back. */
  applyPatch(patch: DocPatch) {
    this.view.dispatch({
      changes: patch.changes,
      annotations: remotePatch.of(true),
      userEvent: "input.remote",
    });
  }

  /** Replace the whole document (file open/new/external reload). */
  setText(text: string) {
    this.cancelSync();
    this.view.setState(this.freshState(text));
  }

  getText(): string {
    return this.view.state.doc.toString();
  }

  focus() {
    this.view.focus();
  }

  undo() {
    undo(this.view);
  }

  redo() {
    redo(this.view);
  }

  /** 1-based top visible line; 0 = "scrolled to end" sentinel (same
   * convention as the GTK app's top_visible_source_line_1based). */
  topVisibleLine(): number {
    const scroller = this.view.scrollDOM;
    if (scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 2) {
      return 0;
    }
    const block = this.view.lineBlockAtHeight(scroller.scrollTop);
    return this.view.state.doc.lineAt(block.from).number;
  }

  /** Put the given 1-based line at the top of the viewport and move the
   * cursor there (yalign=start, so preview↔edit round-trips don't drift). */
  scrollToLine(line1: number) {
    const line = Math.min(Math.max(line1, 1), this.view.state.doc.lines);
    const pos = this.view.state.doc.line(line).from;
    this.view.dispatch({
      selection: { anchor: pos },
      effects: EditorView.scrollIntoView(pos, { y: "start" }),
    });
  }

  /** Pin the editor to the document tail. */
  scrollToEnd() {
    const pos = this.view.state.doc.length;
    this.view.dispatch({
      selection: { anchor: pos },
      effects: EditorView.scrollIntoView(pos, { y: "end" }),
    });
  }

  /** Flush any pending debounce immediately (before save/mode toggle). */
  async flushSync(): Promise<void> {
    if (this.syncTimer !== null) {
      this.cancelSync();
      await this.syncNow();
    }
  }

  private scheduleSync() {
    this.cancelSync();
    this.syncTimer = setTimeout(() => {
      this.syncTimer = null;
      void this.syncNow();
    }, SYNC_DEBOUNCE_MS);
  }

  private cancelSync() {
    if (this.syncTimer !== null) {
      clearTimeout(this.syncTimer);
      this.syncTimer = null;
    }
  }

  private async syncNow(): Promise<void> {
    await updateText(this.getText());
    this.onSynced?.();
  }
}
