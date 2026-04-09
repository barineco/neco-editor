/**
 * Mouse event handling for the editor.
 *
 * Translates raw DOM mouse events into editor commands:
 * click → setCursor, drag → setSelection, dblclick → selectWord.
 */

import {
  containerOffsetFromMouseEvent,
  type ContainerX,
  type ContainerY,
} from './coordinates'

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface MouseHandlerCallbacks {
  /** Convert viewport-relative coordinates to a text offset. */
  hitTest(x: ContainerX, y: ContainerY): number
  /** Current vertical scroll position in pixels. */
  getScrollTop(): number
  /** Full document text (used for word-boundary detection). */
  getText(): string
}

export type MouseCommand =
  | { type: 'setCursor'; offset: number }
  | { type: 'setSelection'; anchor: number; head: number }
  | { type: 'selectWord'; offset: number }

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Maximum interval between two clicks to count as a double-click (ms). */
const DOUBLE_CLICK_THRESHOLD = 300

// ---------------------------------------------------------------------------
// Word boundary helpers
// ---------------------------------------------------------------------------

const WORD_CHAR = /\w/

/**
 * Find the start and end offsets of the word surrounding `offset`.
 * A "word" is a maximal run of `\w` characters.
 * If the character at `offset` is not a word character the range is
 * `[offset, offset]` (zero-width).
 */
export function wordBoundary(text: string, offset: number): [start: number, end: number] {
  if (offset < 0 || offset >= text.length || !WORD_CHAR.test(text.charAt(offset))) {
    return [offset, offset]
  }
  let start = offset
  while (start > 0 && WORD_CHAR.test(text.charAt(start - 1))) {
    start--
  }
  let end = offset
  while (end < text.length && WORD_CHAR.test(text.charAt(end))) {
    end++
  }
  return [start, end]
}

// ---------------------------------------------------------------------------
// MouseHandler
// ---------------------------------------------------------------------------

export class MouseHandler {
  private container: HTMLElement
  private callbacks: MouseHandlerCallbacks
  private onCommand: (cmd: MouseCommand) => void

  /** Anchor offset set on mousedown; null when not dragging. */
  private anchor: number | null = null
  /** Whether a rAF callback is already scheduled for mousemove. */
  private rafPending = false

  /** Timestamp of the last mousedown (for double-click detection). */
  private lastMousedownTime = 0
  /** Offset of the last mousedown (for double-click detection). */
  private lastMousedownOffset = -1

  // Bound listeners (stored so we can remove them in dispose)
  private readonly onMouseDown: (e: MouseEvent) => void
  private readonly onMouseMove: (e: MouseEvent) => void
  private readonly onMouseUp: (e: MouseEvent) => void

  constructor(
    container: HTMLElement,
    callbacks: MouseHandlerCallbacks,
    onCommand: (cmd: MouseCommand) => void,
  ) {
    this.container = container
    this.callbacks = callbacks
    this.onCommand = onCommand

    this.onMouseDown = this.handleMouseDown.bind(this)
    this.onMouseMove = this.handleMouseMove.bind(this)
    this.onMouseUp = this.handleMouseUp.bind(this)

    this.container.addEventListener('mousedown', this.onMouseDown)
  }

  // -------------------------------------------------------------------------
  // Event handlers
  // -------------------------------------------------------------------------

  private handleMouseDown(e: MouseEvent): void {
    // Only handle primary button.
    if (e.button !== 0) return
    e.preventDefault()

    const offset = this.offsetFromEvent(e)
    const now = Date.now()

    // Double-click detection: same approximate position within threshold.
    if (
      now - this.lastMousedownTime < DOUBLE_CLICK_THRESHOLD &&
      this.lastMousedownOffset === offset
    ) {
      this.onCommand({ type: 'selectWord', offset })
      this.lastMousedownTime = 0
      this.lastMousedownOffset = -1
      // Do not start a drag after a double-click.
      this.anchor = null
      return
    }

    this.lastMousedownTime = now
    this.lastMousedownOffset = offset

    // Start a potential drag.
    this.anchor = offset
    this.onCommand({ type: 'setCursor', offset })

    // Listen on document so we track the mouse even if it leaves the container.
    document.addEventListener('mousemove', this.onMouseMove)
    document.addEventListener('mouseup', this.onMouseUp)
  }

  private handleMouseMove(e: MouseEvent): void {
    if (this.anchor === null) return
    if (this.rafPending) return

    this.rafPending = true
    requestAnimationFrame(() => {
      this.rafPending = false
      if (this.anchor === null) return

      const head = this.offsetFromEvent(e)
      if (head !== this.anchor) {
        this.onCommand({ type: 'setSelection', anchor: this.anchor, head })
      }
    })
  }

  private handleMouseUp(_e: MouseEvent): void {
    if (this.anchor !== null) {
      this.anchor = null
    }
    document.removeEventListener('mousemove', this.onMouseMove)
    document.removeEventListener('mouseup', this.onMouseUp)
  }

  // -------------------------------------------------------------------------
  // Coordinate helpers
  // -------------------------------------------------------------------------

  /**
   * Convert a MouseEvent into a text offset by translating page
   * coordinates to container-relative coordinates and delegating to
   * the injected hitTest callback.
   */
  private offsetFromEvent(e: MouseEvent): number {
    const { x, y } = containerOffsetFromMouseEvent(this.container, e)
    return this.callbacks.hitTest(x, y)
  }

  // -------------------------------------------------------------------------
  // Lifecycle
  // -------------------------------------------------------------------------

  dispose(): void {
    this.container.removeEventListener('mousedown', this.onMouseDown)
    document.removeEventListener('mousemove', this.onMouseMove)
    document.removeEventListener('mouseup', this.onMouseUp)
    this.anchor = null
  }
}
