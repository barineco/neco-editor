/**
 * Clipboard operations: copy, cut, and paste.
 *
 * Uses the Async Clipboard API (`navigator.clipboard`) when available,
 * falling back to the deprecated `document.execCommand` for environments
 * where the modern API is blocked by security constraints.
 */

/** Callbacks that the host (EditorView) supplies so ClipboardHandler
 *  can query selection state and apply edits without coupling to the
 *  full editor surface. */
export interface ClipboardCallbacks {
  /** Return the currently selected text, or `null` if nothing is selected. */
  getSelectedText(): string | null
  /** Return anchor/head offsets for the current selection, or `null`. */
  getSelection(): { anchor: number; head: number } | null
  /** Apply a text replacement.  `label` is the undo-group label. */
  applyEdit(start: number, end: number, newText: string, label?: string): void
  /** Return the current cursor (caret) offset. */
  getCursor(): number
  /** Adjust pasted text indentation relative to the target offset. */
  adjustPasteIndent(text: string, offset: number): string
}

export class ClipboardHandler {
  private cb: ClipboardCallbacks

  constructor(callbacks: ClipboardCallbacks) {
    this.cb = callbacks
  }

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /** Copy the current selection to the clipboard.
   *  Returns `true` when text was successfully written. */
  async copy(): Promise<boolean> {
    const text = this.cb.getSelectedText()
    if (text === null || text.length === 0) return false
    return this.writeToClipboard(text)
  }

  /** Cut the current selection: copy it, then delete it.
   *  Returns `true` when text was successfully written. */
  async cut(): Promise<boolean> {
    const text = this.cb.getSelectedText()
    const sel = this.cb.getSelection()
    if (text === null || text.length === 0 || sel === null) return false

    const ok = await this.writeToClipboard(text)
    if (!ok) return false

    const start = Math.min(sel.anchor, sel.head)
    const end = Math.max(sel.anchor, sel.head)
    this.cb.applyEdit(start, end, '', 'cut')
    return true
  }

  /** Paste clipboard contents at the cursor / over the current selection.
   *  Returns `true` when text was successfully inserted. */
  async paste(): Promise<boolean> {
    const text = await this.readFromClipboard()
    if (text === null || text.length === 0) return false

    const sel = this.cb.getSelection()
    let start: number
    let end: number
    if (sel !== null) {
      start = Math.min(sel.anchor, sel.head)
      end = Math.max(sel.anchor, sel.head)
    } else {
      start = this.cb.getCursor()
      end = start
    }

    const adjusted = this.cb.adjustPasteIndent(text, start)
    this.cb.applyEdit(start, end, adjusted, 'paste')
    return true
  }

  // ---------------------------------------------------------------------------
  // Clipboard I/O with execCommand fallback
  // ---------------------------------------------------------------------------

  private async writeToClipboard(text: string): Promise<boolean> {
    if (this.hasAsyncClipboard()) {
      try {
        await navigator.clipboard.writeText(text)
        return true
      } catch {
        // Permission denied or other error — fall through to execCommand.
      }
    }
    return this.execCopy(text)
  }

  private async readFromClipboard(): Promise<string | null> {
    if (this.hasAsyncClipboard()) {
      try {
        return await navigator.clipboard.readText()
      } catch {
        // Permission denied — fall through.
      }
    }
    return this.execPaste()
  }

  // ---------------------------------------------------------------------------
  // Feature detection
  // ---------------------------------------------------------------------------

  private hasAsyncClipboard(): boolean {
    return (
      typeof navigator !== 'undefined' &&
      navigator.clipboard !== undefined &&
      typeof navigator.clipboard.writeText === 'function'
    )
  }

  // ---------------------------------------------------------------------------
  // execCommand fallback
  // ---------------------------------------------------------------------------

  /** Write `text` via a temporary textarea + `document.execCommand('copy')`. */
  private execCopy(text: string): boolean {
    const ta = document.createElement('textarea')
    ta.value = text
    // Keep it out of the visible viewport.
    ta.style.position = 'fixed'
    ta.style.left = '-9999px'
    ta.style.top = '-9999px'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    let ok = false
    try {
      ok = document.execCommand('copy')
    } catch {
      // execCommand may throw in some browsers.
    } finally {
      document.body.removeChild(ta)
    }
    return ok
  }

  /** Read text via `document.execCommand('paste')`.
   *  This only works in very few contexts (e.g. browser extensions). */
  private execPaste(): string | null {
    const ta = document.createElement('textarea')
    ta.style.position = 'fixed'
    ta.style.left = '-9999px'
    ta.style.top = '-9999px'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.focus()
    let text: string | null = null
    try {
      if (document.execCommand('paste')) {
        text = ta.value
      }
    } catch {
      // execCommand may throw.
    } finally {
      document.body.removeChild(ta)
    }
    return text
  }
}
