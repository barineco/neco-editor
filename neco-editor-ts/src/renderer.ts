import type { RenderLine, Rect } from './types'
import { tokenKindToClass } from './theme'

export interface RendererOptions {
  gutterWidth: number
  showLineNumbers: boolean
  padding?: { top?: number; bottom?: number }
}

/**
 * DOM renderer for the neco-editor.
 *
 * Converts `RenderLine[]` into a per-line gutter cell + content cell structure,
 * and provides overlay layers for the caret and selections.
 *
 * Current implementation replaces all children on each `renderLines` call
 * (no incremental diffing). This is the simplest correct approach and a
 * future optimisation point.
 */
export class Renderer {
  private container: HTMLElement
  private options: RendererOptions

  // Persistent top-level elements
  private gutterBgEl: HTMLElement
  private contentEl: HTMLElement
  private linesEl: HTMLElement
  private caretEl: HTMLElement
  private selectionLayer: HTMLElement

  constructor(container: HTMLElement, options: RendererOptions) {
    this.container = container
    this.options = options

    // -- gutter background ----------------------------------------------------
    // gutterBgEl is a visual-only column; width is driven by --neco-gutter-width.
    this.gutterBgEl = document.createElement('div')
    this.gutterBgEl.className = 'neco-editor-gutter-bg'

    // -- content area ---------------------------------------------------------
    // contentEl starts at left:0; per-line gutter cells occupy the left portion.
    this.contentEl = document.createElement('div')
    this.contentEl.className = 'neco-editor-content'

    this.linesEl = document.createElement('div')
    this.linesEl.className = 'neco-editor-lines'

    this.caretEl = document.createElement('div')
    this.caretEl.className = 'neco-cursor'
    this.caretEl.style.position = 'absolute'

    this.selectionLayer = document.createElement('div')
    this.selectionLayer.className = 'neco-selection-layer'

    this.contentEl.appendChild(this.linesEl)
    this.contentEl.appendChild(this.caretEl)
    this.contentEl.appendChild(this.selectionLayer)

    // Apply optional padding to contentEl
    const padTop = options.padding?.top ?? 0
    const padBottom = options.padding?.bottom ?? 0
    if (padTop > 0) this.contentEl.style.paddingTop = `${padTop}px`
    if (padBottom > 0) this.contentEl.style.paddingBottom = `${padBottom}px`

    // Set the CSS variable so gutterBgEl and per-line gutter cells share the same width.
    container.style.setProperty('--neco-gutter-width', `${options.gutterWidth}px`)

    container.appendChild(this.gutterBgEl)
    container.appendChild(this.contentEl)
  }

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /**
   * Render visible lines into the DOM.
   *
   * Each .neco-line contains a gutter cell and a content cell.
   * `currentLineNumber` is the 1-based line number that should be highlighted
   * as the current cursor line.
   */
  renderLines(lines: RenderLine[], currentLineNumber: number): void {
    this.linesEl.textContent = ''
    for (const line of lines) {
      const lineEl = document.createElement('div')
      lineEl.className = 'neco-line'

      // Gutter cell
      if (this.options.showLineNumbers) {
        const gutterCell = document.createElement('span')
        gutterCell.className = 'neco-line-gutter-cell'
        if (line.lineNumber === currentLineNumber) {
          gutterCell.classList.add('neco-current-line')
        }
        gutterCell.textContent = String(line.lineNumber)
        lineEl.appendChild(gutterCell)
      }

      // Content cell
      const contentCell = document.createElement('span')
      contentCell.className = 'neco-line-content-cell'
      if (line.lineNumber === currentLineNumber) {
        contentCell.classList.add('neco-current-line')
      }

      if (line.tokens.length === 0) {
        if (line.text.length > 0) {
          const span = document.createElement('span')
          span.className = tokenKindToClass('plain')
          span.textContent = line.text
          contentCell.appendChild(span)
        }
      } else {
        // Walk through tokens, filling gaps with plain spans.
        let cursor = 0
        for (const token of line.tokens) {
          // Fill gap before this token (if any) with a plain span.
          if (token.start > cursor) {
            const gapSpan = document.createElement('span')
            gapSpan.className = tokenKindToClass('plain')
            gapSpan.textContent = line.text.substring(cursor, token.start)
            contentCell.appendChild(gapSpan)
          }

          const span = document.createElement('span')
          span.className = tokenKindToClass(token.kind)
          span.textContent = line.text.substring(token.start, token.end)
          contentCell.appendChild(span)

          cursor = token.end
        }

        // Fill trailing gap after the last token.
        if (cursor < line.text.length) {
          const tailSpan = document.createElement('span')
          tailSpan.className = tokenKindToClass('plain')
          tailSpan.textContent = line.text.substring(cursor)
          contentCell.appendChild(tailSpan)
        }
      }

      lineEl.appendChild(contentCell)
      this.linesEl.appendChild(lineEl)
    }
  }

  /** Position the caret element at the given pixel rect. */
  renderCaret(rect: Rect): void {
    const moved =
      this.caretEl.style.left !== `${rect.x}px` ||
      this.caretEl.style.top !== `${rect.y}px`
    this.caretEl.style.left = `${rect.x}px`
    this.caretEl.style.top = `${rect.y}px`
    this.caretEl.style.width = `${rect.width}px`
    this.caretEl.style.height = `${rect.height}px`
    if (moved) {
      this.caretEl.style.animation = 'none'
      this.caretEl.offsetHeight // force reflow
      this.caretEl.style.animation = ''
    }
  }

  /** Render selection highlight rectangles (one per visual line). */
  renderSelections(rects: Rect[]): void {
    this.selectionLayer.textContent = ''
    for (const rect of rects) {
      const selEl = document.createElement('div')
      selEl.className = 'neco-selection'
      selEl.style.position = 'absolute'
      selEl.style.left = `${rect.x}px`
      selEl.style.top = `${rect.y}px`
      selEl.style.width = `${rect.width}px`
      selEl.style.height = `${rect.height}px`
      this.selectionLayer.appendChild(selEl)
    }
  }

  /** Update the gutter column width via CSS variable (gutterBgEl and gutter cells both reference it). */
  updateGutterWidth(width: number): void {
    this.options.gutterWidth = width
    this.container.style.setProperty('--neco-gutter-width', `${width}px`)
  }

  /** Return the content container element (used as scroll container). */
  getContentElement(): HTMLElement {
    return this.contentEl
  }

  /** Return the lines container element. */
  getLinesElement(): HTMLElement {
    return this.linesEl
  }

  /** Return the current pixel size of the content area. */
  getContentRect(): { width: number; height: number } {
    return {
      width: this.contentEl.clientWidth,
      height: this.contentEl.clientHeight,
    }
  }

  /** Remove all rendered content while keeping the structural elements. */
  clear(): void {
    this.linesEl.textContent = ''
    this.selectionLayer.textContent = ''
    this.caretEl.style.left = '0px'
    this.caretEl.style.top = '0px'
    this.caretEl.style.width = '0px'
    this.caretEl.style.height = '0px'
  }

  /** Detach all DOM elements from the container and release references. */
  dispose(): void {
    this.clear()
    this.container.removeChild(this.gutterBgEl)
    this.container.removeChild(this.contentEl)
    this.container.style.removeProperty('--neco-gutter-width')
  }
}
