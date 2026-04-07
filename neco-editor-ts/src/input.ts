/**
 * Keyboard and IME input handling via a hidden textarea.
 *
 * Captures keydown events and converts them into editor commands.
 * Correctly handles IME composition by suppressing commands until
 * compositionend fires.
 */

// ---------------------------------------------------------------------------
// InputCommand
// ---------------------------------------------------------------------------

export type InputCommand =
  | { type: 'insert'; text: string }
  | { type: 'delete'; direction: 'backward' | 'forward' }
  | { type: 'newline' }
  | { type: 'tab' }
  | { type: 'undo' }
  | { type: 'redo' }
  | { type: 'selectAll' }
  | { type: 'moveCursor'; direction: 'left' | 'right' | 'up' | 'down'; extend: boolean }
  | { type: 'moveCursorByWord'; direction: 'left' | 'right'; extend: boolean }
  | { type: 'moveCursorToLineEdge'; direction: 'start' | 'end'; extend: boolean }
  | { type: 'moveCursorToDocumentEdge'; direction: 'start' | 'end'; extend: boolean }
  | { type: 'pageMove'; direction: 'up' | 'down'; extend: boolean }
  | { type: 'copy' }
  | { type: 'cut' }
  | { type: 'paste' }

// ---------------------------------------------------------------------------
// Platform detection
// ---------------------------------------------------------------------------

function detectMac(): boolean {
  // navigator.userAgentData is available in newer Chromium-based browsers.
  // Fall back to the legacy navigator.platform for Safari / Firefox.
  if (typeof navigator !== 'undefined') {
    const uad = (navigator as any).userAgentData
    if (uad && typeof uad.platform === 'string') {
      return uad.platform === 'macOS'
    }
    return /Mac|iPhone|iPad|iPod/.test(navigator.platform ?? '')
  }
  return false
}

const isMac: boolean = detectMac()

/** Returns true when the platform-appropriate modifier is held (Cmd on Mac, Ctrl elsewhere). */
function isPrimaryModifier(e: KeyboardEvent): boolean {
  return isMac ? e.metaKey : e.ctrlKey
}

// ---------------------------------------------------------------------------
// InputHandler
// ---------------------------------------------------------------------------

export class InputHandler {
  private textarea: HTMLTextAreaElement
  private composing = false
  private disposed = false

  private handleKeydown: (e: KeyboardEvent) => void
  private handleCompositionStart: () => void
  private handleCompositionUpdate: (e: CompositionEvent) => void
  private handleCompositionEnd: (e: CompositionEvent) => void
  private handleInput: (e: Event) => void

  constructor(
    private container: HTMLElement,
    private onCommand: (cmd: InputCommand) => void,
  ) {
    // Create and style the hidden textarea
    this.textarea = document.createElement('textarea')
    this.textarea.className = 'neco-input-capture'
    this.textarea.setAttribute('autocapitalize', 'off')
    this.textarea.setAttribute('autocomplete', 'off')
    this.textarea.setAttribute('autocorrect', 'off')
    this.textarea.setAttribute('spellcheck', 'false')
    this.textarea.setAttribute('tabindex', '0')
    this.textarea.setAttribute('aria-label', 'Editor input')
    Object.assign(this.textarea.style, {
      position: 'absolute',
      top: '0',
      left: '0',
      width: '1px',
      height: '1px',
      padding: '0',
      margin: '0',
      border: 'none',
      outline: 'none',
      resize: 'none',
      overflow: 'hidden',
      opacity: '0',
      // Keep the textarea in the DOM flow so screen readers can reach it,
      // but visually hidden. Avoid display:none which prevents focus.
      pointerEvents: 'none',
    })
    container.style.position = container.style.position || 'relative'
    container.appendChild(this.textarea)

    // --- Event handlers ---------------------------------------------------

    this.handleKeydown = (e: KeyboardEvent) => {
      if (this.composing) return

      const cmd = this.translateKey(e)
      if (cmd) {
        e.preventDefault()
        this.onCommand(cmd)
      }
    }

    this.handleCompositionStart = () => {
      this.composing = true
    }

    this.handleCompositionUpdate = (_e: CompositionEvent) => {
      // Intentionally empty; the composition preview is handled by the
      // textarea itself. We may forward the intermediate text in the future.
    }

    this.handleCompositionEnd = (e: CompositionEvent) => {
      this.composing = false
      const text = e.data
      if (text) {
        this.onCommand({ type: 'insert', text })
      }
      // Clear the textarea so subsequent input events start fresh.
      this.textarea.value = ''
    }

    this.handleInput = (e: Event) => {
      if (this.composing) return

      const ie = e as InputEvent
      // For non-composition text input (e.g. dead-key sequences on Linux),
      // the 'input' event carries the final text when keydown didn't produce
      // an insert command.
      if (ie.inputType === 'insertText' && ie.data) {
        this.onCommand({ type: 'insert', text: ie.data })
      }
      // Always clear the textarea value after processing to prevent stale
      // content from interfering with future input.
      this.textarea.value = ''
    }

    // --- Bind events ------------------------------------------------------

    this.textarea.addEventListener('keydown', this.handleKeydown)
    this.textarea.addEventListener('compositionstart', this.handleCompositionStart)
    this.textarea.addEventListener('compositionupdate', this.handleCompositionUpdate)
    this.textarea.addEventListener('compositionend', this.handleCompositionEnd)
    this.textarea.addEventListener('input', this.handleInput)
  }

  // -- Public API ----------------------------------------------------------

  focus(): void {
    this.textarea.focus()
  }

  blur(): void {
    this.textarea.blur()
  }

  isComposing(): boolean {
    return this.composing
  }

  dispose(): void {
    if (this.disposed) return
    this.disposed = true

    this.textarea.removeEventListener('keydown', this.handleKeydown)
    this.textarea.removeEventListener('compositionstart', this.handleCompositionStart)
    this.textarea.removeEventListener('compositionupdate', this.handleCompositionUpdate)
    this.textarea.removeEventListener('compositionend', this.handleCompositionEnd)
    this.textarea.removeEventListener('input', this.handleInput)

    this.textarea.remove()
  }

  // -- Key translation -----------------------------------------------------

  private translateKey(e: KeyboardEvent): InputCommand | null {
    const mod = isPrimaryModifier(e)
    const shift = e.shiftKey
    const alt = e.altKey

    // --- Modifier combos --------------------------------------------------

    if (mod && !alt) {
      switch (e.key) {
        case 'z':
        case 'Z':
          return shift ? { type: 'redo' } : { type: 'undo' }
        case 'y':
        case 'Y':
          return { type: 'redo' }
        case 'a':
        case 'A':
          return { type: 'selectAll' }
        case 'c':
        case 'C':
          return { type: 'copy' }
        case 'x':
        case 'X':
          return { type: 'cut' }
        case 'v':
        case 'V':
          return { type: 'paste' }
        // Cmd+Home / Cmd+End (Mac) — document edges
        case 'Home':
          return { type: 'moveCursorToDocumentEdge', direction: 'start', extend: shift }
        case 'End':
          return { type: 'moveCursorToDocumentEdge', direction: 'end', extend: shift }
        // Cmd+ArrowUp / Cmd+ArrowDown (Mac) — document edges
        case 'ArrowUp':
          return isMac
            ? { type: 'moveCursorToDocumentEdge', direction: 'start', extend: shift }
            : { type: 'moveCursor', direction: 'up', extend: shift }
        case 'ArrowDown':
          return isMac
            ? { type: 'moveCursorToDocumentEdge', direction: 'end', extend: shift }
            : { type: 'moveCursor', direction: 'down', extend: shift }
        // Cmd+ArrowLeft/Right (Mac) — line edges; Ctrl+Arrow (non-Mac) — word movement
        case 'ArrowLeft':
          return isMac
            ? { type: 'moveCursorToLineEdge', direction: 'start', extend: shift }
            : { type: 'moveCursorByWord', direction: 'left', extend: shift }
        case 'ArrowRight':
          return isMac
            ? { type: 'moveCursorToLineEdge', direction: 'end', extend: shift }
            : { type: 'moveCursorByWord', direction: 'right', extend: shift }
      }
    }

    // Alt+Arrow — word movement (Mac) or no-op (non-Mac alt is often OS-level)
    if (alt && !mod) {
      switch (e.key) {
        case 'ArrowLeft':
          return isMac
            ? { type: 'moveCursorByWord', direction: 'left', extend: shift }
            : null
        case 'ArrowRight':
          return isMac
            ? { type: 'moveCursorByWord', direction: 'right', extend: shift }
            : null
      }
    }

    // --- Non-modifier keys ------------------------------------------------

    if (!mod && !alt) {
      switch (e.key) {
        case 'Backspace':
          return { type: 'delete', direction: 'backward' }
        case 'Delete':
          return { type: 'delete', direction: 'forward' }
        case 'Enter':
          return { type: 'newline' }
        case 'Tab':
          return { type: 'tab' }
        case 'ArrowLeft':
          return { type: 'moveCursor', direction: 'left', extend: shift }
        case 'ArrowRight':
          return { type: 'moveCursor', direction: 'right', extend: shift }
        case 'ArrowUp':
          return { type: 'moveCursor', direction: 'up', extend: shift }
        case 'ArrowDown':
          return { type: 'moveCursor', direction: 'down', extend: shift }
        case 'Home':
          return { type: 'moveCursorToLineEdge', direction: 'start', extend: shift }
        case 'End':
          return { type: 'moveCursorToLineEdge', direction: 'end', extend: shift }
        case 'PageUp':
          return { type: 'pageMove', direction: 'up', extend: shift }
        case 'PageDown':
          return { type: 'pageMove', direction: 'down', extend: shift }
      }

      // Printable single-character input — let the 'input' event handle it
      // so that dead-key sequences and OS-level transformations are respected.
      // We return null here to avoid preventing the default behaviour.
      if (e.key.length === 1) {
        return null
      }
    }

    return null
  }
}
