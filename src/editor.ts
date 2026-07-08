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
