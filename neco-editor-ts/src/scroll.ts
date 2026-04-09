/**
 * Virtual scroll manager for the neco-editor.
 *
 * Maintains a spacer element inside a scroll container so that only
 * the visible line range (plus an overscan buffer) needs to be
 * rendered into the DOM.
 */

import { blockAdvance, cssPx, type BlockAdvance, type CssPx } from './coordinates'

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface ScrollManagerOptions {
  lineHeight: number
  containerHeight: number
  overscan?: number // extra lines to render above/below the viewport (default: 5)
}

export interface ScrollState {
  scrollTop: CssPx
  totalHeight: CssPx
  visibleStartLine: BlockAdvance
  visibleEndLine: BlockAdvance
  /** translateY offset for the lines container. */
  offsetY: CssPx
}

// ---------------------------------------------------------------------------
// ScrollManager
// ---------------------------------------------------------------------------

/**
 * Tracks the scroll position of a container element and exposes the
 * visible line range so that the renderer only creates DOM nodes for
 * lines that are (approximately) on-screen.
 *
 * Internally it creates a spacer `<div>` whose height equals the total
 * virtual height (`totalLines * lineHeight`).  The container element
 * must have `overflow: auto` (or `scroll`) set by the caller.
 */
export class ScrollManager {
  private container: HTMLElement
  private spacer: HTMLDivElement

  private lineHeight: number
  private containerHeight: number
  private overscan: number
  private totalLines = 0

  private rafId: number | null = null
  private listeners: Array<(state: ScrollState) => void> = []
  private boundOnScroll: () => void

  constructor(container: HTMLElement, options: ScrollManagerOptions) {
    this.container = container
    this.lineHeight = options.lineHeight
    this.containerHeight = options.containerHeight
    this.overscan = options.overscan ?? 5

    // Create the spacer element that provides the virtual scroll height.
    this.spacer = document.createElement('div')
    this.spacer.className = 'neco-editor-scroll'
    this.spacer.style.width = '100%'
    this.spacer.style.height = '0px'
    this.spacer.style.pointerEvents = 'none'
    this.container.appendChild(this.spacer)

    this.boundOnScroll = this.handleScroll.bind(this)
    this.container.addEventListener('scroll', this.boundOnScroll, { passive: true })
  }

  // ---- Public API ----------------------------------------------------------

  /** Compute the current scroll state from the container's scrollTop. */
  getScrollState(): ScrollState {
    return this.computeState(this.container.scrollTop)
  }

  /** Programmatically set the scroll position. */
  setScrollTop(value: number): void {
    this.container.scrollTop = value
  }

  /** Update the total number of lines (call after text changes). */
  setTotalLines(count: number): void {
    this.totalLines = count
    this.spacer.style.height = `${this.totalLines * this.lineHeight}px`
  }

  /** Update the container viewport height (call on resize). */
  setContainerHeight(height: number): void {
    this.containerHeight = height
  }

  /** Update the line height (call when font metrics change). */
  setLineHeight(value: number): void {
    this.lineHeight = value
    // Re-apply total height with new line height.
    this.spacer.style.height = `${this.totalLines * this.lineHeight}px`
  }

  /**
   * Calculate the scrollTop needed to reveal a target pixel offset.
   *
   * Returns `null` if the target is already visible within the viewport.
   *
   * @param targetY  - The vertical pixel offset of the target (e.g. caret top).
   * @param caretHeight - The height of the element to reveal (typically lineHeight).
   */
  scrollToReveal(targetY: number, caretHeight: number): number | null {
    const { scrollTop } = this.container
    const viewportBottom = scrollTop + this.containerHeight

    // Already fully visible.
    if (targetY >= scrollTop && targetY + caretHeight <= viewportBottom) {
      return null
    }

    // Target is above the viewport: scroll up so the target sits at the top.
    if (targetY < scrollTop) {
      return targetY
    }

    // Target is below the viewport: scroll down so the target's bottom
    // aligns with the viewport bottom.
    return targetY + caretHeight - this.containerHeight
  }

  /**
   * Register a callback that fires (at most once per animation frame)
   * whenever the container is scrolled.
   */
  onScroll(callback: (state: ScrollState) => void): { dispose(): void } {
    this.listeners.push(callback)
    return {
      dispose: () => {
        const idx = this.listeners.indexOf(callback)
        if (idx !== -1) this.listeners.splice(idx, 1)
      },
    }
  }

  /** Remove the spacer, event listener and cancel any pending rAF. */
  dispose(): void {
    this.container.removeEventListener('scroll', this.boundOnScroll)
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId)
      this.rafId = null
    }
    this.spacer.remove()
    this.listeners.length = 0
  }

  // ---- Internals -----------------------------------------------------------

  private handleScroll(): void {
    // Listeners are batched via rAF for content-rebuild work.
    if (this.rafId !== null) return
    this.rafId = requestAnimationFrame(() => {
      this.rafId = null
      const state = this.getScrollState()
      for (const cb of this.listeners) {
        cb(state)
      }
    })
  }

  private computeState(scrollTop: number): ScrollState {
    const totalHeight = this.totalLines * this.lineHeight

    if (this.lineHeight <= 0 || this.totalLines <= 0) {
      return {
        scrollTop: cssPx(scrollTop),
        totalHeight: cssPx(totalHeight),
        visibleStartLine: blockAdvance(0),
        visibleEndLine: blockAdvance(0),
        offsetY: cssPx(0),
      }
    }

    const rawStart = Math.floor(scrollTop / this.lineHeight)
    const visibleCount = Math.ceil(this.containerHeight / this.lineHeight)

    const visibleStartLine = Math.max(0, rawStart - this.overscan)
    const visibleEndLine = Math.min(this.totalLines, rawStart + visibleCount + this.overscan)
    const offsetY = visibleStartLine * this.lineHeight

    return {
      scrollTop: cssPx(scrollTop),
      totalHeight: cssPx(totalHeight),
      visibleStartLine: blockAdvance(visibleStartLine),
      visibleEndLine: blockAdvance(visibleEndLine),
      offsetY: cssPx(offsetY),
    }
  }
}
