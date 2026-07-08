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

import { updateText } from "./bridge";

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
      ],
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
