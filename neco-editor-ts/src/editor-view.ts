import { EditorSession } from './editor'
import { measureFontMetrics } from './metrics'
import { DomRenderer, type EditorRenderer, type RendererMetrics, type RendererOptions } from './renderer'
import { WebGpuRenderer } from './webgpu-renderer'
import { ScrollManager, type ScrollState } from './scroll'
import { InputHandler, type InputCommand } from './input'
import { MouseHandler, type MouseCommand } from './mouse'
import { ClipboardHandler, type ClipboardCallbacks } from './clipboard'
import type { RangeChange, Rect, SearchMatchInfo, SearchOptions } from './types'
import { CoordinateMap, docY, screenRect, toViewY } from './coordinates'

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface EditorViewOptions {
  /** Mount target DOM element. */
  container: HTMLElement
  /** Initial text (required when session is not provided). */
  text?: string
  /** Language identifier (required when session is not provided). */
  language?: string
  /** Inject an existing EditorSession (use instead of text/language). */
  session?: EditorSession
  /** Read-only mode. */
  readOnly?: boolean
  /** Tab size in spaces. */
  tabSize?: number
  /** CSS font-family used for measurement and rendering. */
  fontFamily?: string
  /** Font size in CSS pixels. */
  fontSize?: number
  /** Line height override in CSS pixels. */
  lineHeight?: number
  /** Strict 1:2 half-width/full-width grid. */
  monospaceGrid?: boolean
  /** Renderer backend. WebGPU is the default and fails explicitly when unavailable. */
  renderer?: 'webgpu' | 'dom'
  /** Word-wrap mode (reserved for future use). */
  wordWrap?: boolean
  /** Show line numbers in the gutter. */
  lineNumbers?: boolean
  /** Content padding. */
  padding?: { top?: number; bottom?: number }
}

export interface EditorViewState {
  scrollTop: number
  cursorOffset: number
  selectionAnchor: number | null
}

export interface Disposable {
  dispose(): void
}

// ---------------------------------------------------------------------------
// Default option values
// ---------------------------------------------------------------------------

const DEFAULT_FONT_FAMILY = "'Menlo', 'Consolas', 'DejaVu Sans Mono', 'Courier New', monospace"
const DEFAULT_FONT_SIZE = 14
const DEFAULT_TAB_SIZE = 4

// ---------------------------------------------------------------------------
// Event emitter helper
// ---------------------------------------------------------------------------

type Listener<T> = (value: T) => void

class EventEmitter<T = void> {
  private listeners: Listener<T>[] = []

  fire(...args: T extends void ? [] : [value: T]): void {
    const value = args[0] as T
    for (const cb of this.listeners) {
      cb(value)
    }
  }

  on(callback: Listener<T>): Disposable {
    this.listeners.push(callback)
    return {
      dispose: () => {
        const idx = this.listeners.indexOf(callback)
        if (idx !== -1) this.listeners.splice(idx, 1)
      },
    }
  }

  dispose(): void {
    this.listeners.length = 0
  }
}

// ---------------------------------------------------------------------------
// EditorView
// ---------------------------------------------------------------------------

/**
 * Top-level integration class that combines all editor modules into a
 * working editor UI mounted to a DOM container.
 */
export class EditorView {
  // -- State ----------------------------------------------------------------
  private session: EditorSession
  private cursorOffset = 0
  private selectionAnchor: number | null = null
  private disposed = false
  private options: EditorViewOptions
  private tabSize: number
  private fontFamily: string
  private fontSize: number
  private lineHeightOverride: number | undefined
  private lineHeight = 0
  private charWidth = 0
  private cjkCharWidth = 0
  private compositionText = ''

  // -- Sub-modules ----------------------------------------------------------
  private renderer: EditorRenderer
  private scrollManager: ScrollManager
  private inputHandler: InputHandler
  private mouseHandler: MouseHandler
  private clipboardHandler: ClipboardHandler

  // -- DOM ------------------------------------------------------------------
  private container: HTMLElement
  private resizeObserver: ResizeObserver

  // -- Render scheduling ----------------------------------------------------
  private renderRafId: number | null = null

  // -- Events ---------------------------------------------------------------
  private contentChange = new EventEmitter<RangeChange[] | null>()
  private cursorPositionChange = new EventEmitter<number>()
  private selectionChange = new EventEmitter<{ anchor: number; head: number }>()
  private blurEvent = new EventEmitter<void>()
  private focusEvent = new EventEmitter<void>()
  private scrollEvent = new EventEmitter<number>()

  // -- Clean state tracking -------------------------------------------------
  /** The getText() snapshot when markClean() was last called. */
  private cleanText: string

  constructor(options: EditorViewOptions) {
    this.options = options
    this.container = options.container
    this.tabSize = options.tabSize ?? DEFAULT_TAB_SIZE
    this.fontFamily = options.fontFamily ?? DEFAULT_FONT_FAMILY
    this.fontSize = options.fontSize ?? DEFAULT_FONT_SIZE
    this.lineHeightOverride = options.lineHeight

    // 1. Add editor root class
    this.container.classList.add('neco-editor')
    this.applyHostFontStyles()

    // 2. Measure font metrics
    const metrics = measureFontMetrics(this.container, this.fontFamily, this.fontSize)
    this.charWidth = metrics.charWidth
    this.cjkCharWidth = metrics.cjkCharWidth
    this.lineHeight = metrics.lineHeight
    if (this.lineHeightOverride !== undefined) {
      this.lineHeight = this.lineHeightOverride
    }

    // 3. Create or reuse EditorSession
    if (options.session) {
      this.session = options.session
    } else {
      this.session = new EditorSession(
        options.text ?? '',
        options.language ?? 'plain',
        this.lineHeight,
        this.charWidth,
        this.effectiveCjkCharWidth(),
        this.tabSize,
      )
    }

    // 4. Update metrics on session
    this.updateSessionMetrics()

    // Apply readOnly if specified
    if (options.readOnly) {
      this.session.setReadOnly(true)
    }

    this.cleanText = this.session.getText()

    // 5. Renderer (created before ScrollManager so we can use its content element)
    const gutterWidth = (options.lineNumbers !== false) ? this.session.getGutterWidth() : 0
    const rendererOpts: RendererOptions = {
      gutterWidth,
      showLineNumbers: options.lineNumbers !== false,
      padding: options.padding,
      metrics: this.rendererMetrics(),
    }
    this.renderer = this.createRenderer(rendererOpts)

    // 6. ScrollManager: uses renderer's content element as scroll container
    const contentEl = this.renderer.getContentElement()
    const containerRect = contentEl.getBoundingClientRect()
    this.scrollManager = new ScrollManager(contentEl, {
      lineHeight: this.lineHeight,
      containerHeight: containerRect.height > 0 ? containerRect.height : this.container.getBoundingClientRect().height,
    })

    // Set initial total lines
    this.updateTotalLines()

    // 7. InputHandler
    this.inputHandler = new InputHandler(contentEl, (cmd) => this.handleInputCommand(cmd))

    // 8. MouseHandler
    this.mouseHandler = new MouseHandler(
      this.container,
      {
        hitTest: (x, y) => {
          const scrollTop = this.scrollManager.getScrollState().scrollTop
          const coords = new CoordinateMap({
            gutterWidth: (this.options.lineNumbers !== false) ? this.session.getGutterWidth() : 0,
            scrollTop,
            padTop: this.options.padding?.top ?? 0,
            lineHeight: this.lineHeight,
            devicePixelRatio: window.devicePixelRatio || 1,
          })
          return this.session.hitTest(
            coords.containerToViewportX(x),
            coords.containerToViewportY(y),
            scrollTop,
          )
        },
        getScrollTop: () => this.scrollManager.getScrollState().scrollTop,
        getText: () => this.session.getText(),
      },
      (cmd) => this.handleMouseCommand(cmd),
    )

    // 9. ClipboardHandler
    const clipCb: ClipboardCallbacks = {
      getSelectedText: () => {
        if (this.selectionAnchor === null) return null
        const text = this.session.getText()
        const start = Math.min(this.selectionAnchor, this.cursorOffset)
        const end = Math.max(this.selectionAnchor, this.cursorOffset)
        return text.substring(
          this.session.byteOffsetToUtf16(start),
          this.session.byteOffsetToUtf16(end),
        )
      },
      getSelection: () => {
        if (this.selectionAnchor === null) return null
        return { anchor: this.selectionAnchor, head: this.cursorOffset }
      },
      applyEdit: (start, end, newText, label) => {
        this.applyEdit(start, end, newText, label)
      },
      getCursor: () => this.cursorOffset,
      adjustPasteIndent: (text, offset) => this.session.adjustPasteIndent(text, offset),
    }
    this.clipboardHandler = new ClipboardHandler(clipCb)

    // 10. ResizeObserver
    this.resizeObserver = new ResizeObserver(() => {
      this.layout()
    })
    this.resizeObserver.observe(this.container)

    // 11. ScrollManager scroll listener
    this.scrollManager.onScroll((_state: ScrollState) => {
      this.scrollEvent.fire(_state.scrollTop)
      this.scheduleRender()
    })

    // 12. Focus / blur events on the container
    this.container.addEventListener('focusin', this.handleFocusIn)
    this.container.addEventListener('focusout', this.handleFocusOut)

    // 13. Initial render
    this.scheduleRender()
  }

  // =========================================================================
  // Public API: State
  // =========================================================================

  getSession(): EditorSession {
    return this.session
  }

  getText(): string {
    return this.session.getText()
  }

  isDirty(): boolean {
    return this.session.getText() !== this.cleanText
  }

  markClean(): void {
    this.cleanText = this.session.getText()
  }

  // =========================================================================
  // Public API: Editing
  // =========================================================================

  applyEdit(start: number, end: number, newText: string, label?: string): RangeChange[] {
    if (this.session.isReadOnly()) return []

    const changes = this.session.applyEdit(start, end, newText, label)

    const lastChange = changes[changes.length - 1]
    this.cursorOffset = lastChange ? lastChange.newEnd : start + utf8ByteLength(newText)
    this.selectionAnchor = null

    this.updateTotalLines()
    this.contentChange.fire(changes)
    this.cursorPositionChange.fire(this.cursorOffset)
    this.revealCursor()
    this.scheduleRender()

    return changes
  }

  undo(): boolean {
    if (this.session.isReadOnly()) return false

    const result = this.session.undo()
    if (result) {
      // Clamp cursor and selection anchor to new text length
      const len = this.session.getTextByteLength()
      this.cursorOffset = Math.min(this.cursorOffset, len)
      if (this.selectionAnchor !== null) {
        this.selectionAnchor = Math.min(this.selectionAnchor, len)
      }

      this.updateTotalLines()
      this.contentChange.fire(null)
      this.cursorPositionChange.fire(this.cursorOffset)
      this.scheduleRender()
    }
    return result
  }

  redo(): boolean {
    if (this.session.isReadOnly()) return false

    const result = this.session.redo()
    if (result) {
      // Clamp cursor and selection anchor to new text length
      const len = this.session.getTextByteLength()
      this.cursorOffset = Math.min(this.cursorOffset, len)
      if (this.selectionAnchor !== null) {
        this.selectionAnchor = Math.min(this.selectionAnchor, len)
      }

      this.updateTotalLines()
      this.contentChange.fire(null)
      this.cursorPositionChange.fire(this.cursorOffset)
      this.scheduleRender()
    }
    return result
  }

  // =========================================================================
  // Public API: Cursor / Selection
  // =========================================================================

  setCursor(offset: number): void {
    this.cursorOffset = clamp(offset, 0, this.session.getTextByteLength())
    this.selectionAnchor = null
    this.cursorPositionChange.fire(this.cursorOffset)
    this.scheduleRender()
  }

  setSelection(anchor: number, head: number): void {
    const len = this.session.getTextByteLength()
    this.selectionAnchor = clamp(anchor, 0, len)
    this.cursorOffset = clamp(head, 0, len)
    this.selectionChange.fire({ anchor: this.selectionAnchor, head: this.cursorOffset })
    this.cursorPositionChange.fire(this.cursorOffset)
    this.scheduleRender()
  }

  getCursor(): number {
    return this.cursorOffset
  }

  getSelection(): { anchor: number; head: number } | null {
    if (this.selectionAnchor === null) return null
    if (this.selectionAnchor === this.cursorOffset) return null
    return { anchor: this.selectionAnchor, head: this.cursorOffset }
  }

  private byteOffsetToUtf16(offset: number): number {
    return this.session.byteOffsetToUtf16(offset)
  }

  private utf16OffsetToByte(offset: number): number {
    return this.session.utf16OffsetToByte(offset)
  }

  // =========================================================================
  // Public API: Scroll
  // =========================================================================

  revealOffset(offset: number): void {
    const caretRect = this.session.getCaretRect(offset)
    const padTop = this.options.padding?.top ?? 0
    const padBottom = this.options.padding?.bottom ?? 0
    // Convert DocY to scroll-container space (caretEl is at DocY + padTop within contentEl).
    // Inflate caret height by padBottom so the cursor stays above the bottom padding zone
    // when scrolling to reveal (otherwise the cursor lands at the very bottom edge or
    // clips into the bottom padding due to browser scrollHeight quirks).
    const newScrollTop = this.scrollManager.scrollToReveal(
      caretRect.y + padTop,
      caretRect.height + padBottom,
    )
    if (newScrollTop !== null) {
      this.scrollManager.setScrollTop(newScrollTop)
    }
  }

  revealLine(line: number): void {
    const padTop = this.options.padding?.top ?? 0
    const padBottom = this.options.padding?.bottom ?? 0
    const targetY = (line - 1) * this.lineHeight + padTop
    const newScrollTop = this.scrollManager.scrollToReveal(targetY, this.lineHeight + padBottom)
    if (newScrollTop !== null) {
      this.scrollManager.setScrollTop(newScrollTop)
    }
  }

  getScrollTop(): number {
    return this.scrollManager.getScrollState().scrollTop
  }

  setScrollTop(value: number): void {
    this.scrollManager.setScrollTop(value)
  }

  // =========================================================================
  // Public API: View State
  // =========================================================================

  saveViewState(): EditorViewState {
    return {
      scrollTop: this.getScrollTop(),
      cursorOffset: this.cursorOffset,
      selectionAnchor: this.selectionAnchor,
    }
  }

  restoreViewState(state: EditorViewState): void {
    this.cursorOffset = state.cursorOffset
    this.selectionAnchor = state.selectionAnchor
    this.scrollManager.setScrollTop(state.scrollTop)
    this.scheduleRender()
  }

  // =========================================================================
  // Public API: Options
  // =========================================================================

  updateOptions(opts: Partial<EditorViewOptions>): void {
    let metricsChanged = false
    if (opts.readOnly !== undefined) {
      this.options.readOnly = opts.readOnly
      this.session.setReadOnly(opts.readOnly)
    }
    if (opts.tabSize !== undefined) {
      this.tabSize = opts.tabSize
      this.options.tabSize = opts.tabSize
      metricsChanged = true
    }
    if (opts.fontFamily !== undefined) {
      this.fontFamily = opts.fontFamily
      this.options.fontFamily = opts.fontFamily
      metricsChanged = true
    }
    if (opts.fontSize !== undefined) {
      this.fontSize = opts.fontSize
      this.options.fontSize = opts.fontSize
      metricsChanged = true
    }
    if (opts.lineHeight !== undefined) {
      this.lineHeightOverride = opts.lineHeight
      this.options.lineHeight = opts.lineHeight
      metricsChanged = true
    }
    if (opts.monospaceGrid !== undefined) {
      this.options.monospaceGrid = opts.monospaceGrid
      metricsChanged = true
    }
    if (opts.lineNumbers !== undefined) {
      this.options.lineNumbers = opts.lineNumbers
      const gutterWidth = opts.lineNumbers ? this.session.getGutterWidth() : 0
      this.renderer.updateGutterWidth(gutterWidth)
    }
    if (metricsChanged) {
      this.updateMetrics()
      return
    }
    this.scheduleRender()
  }

  // =========================================================================
  // Public API: Search
  // =========================================================================

  search(pattern: string, options?: SearchOptions): SearchMatchInfo[] {
    return this.session.search(pattern, options)
  }

  getSearchMatches(): SearchMatchInfo[] {
    return this.session.getSearchMatches()
  }

  // =========================================================================
  // Public API: Events
  // =========================================================================

  onDidChangeContent(callback: (changes: RangeChange[] | null) => void): Disposable {
    return this.contentChange.on(callback)
  }

  onDidChangeCursorPosition(callback: (offset: number) => void): Disposable {
    return this.cursorPositionChange.on(callback)
  }

  onDidChangeSelection(callback: (selection: { anchor: number; head: number }) => void): Disposable {
    return this.selectionChange.on(callback)
  }

  onDidBlur(callback: () => void): Disposable {
    return this.blurEvent.on(callback)
  }

  onDidFocus(callback: () => void): Disposable {
    return this.focusEvent.on(callback)
  }

  onDidScroll(callback: (scrollTop: number) => void): Disposable {
    return this.scrollEvent.on(callback)
  }

  // =========================================================================
  // Public API: Focus
  // =========================================================================

  focus(): void {
    this.inputHandler.focus()
  }

  hasFocus(): boolean {
    return this.container.contains(document.activeElement)
  }

  // =========================================================================
  // Public API: Layout & Metrics
  // =========================================================================

  updateMetrics(): void {
    this.applyHostFontStyles()
    const metrics = measureFontMetrics(this.container, this.fontFamily, this.fontSize)
    this.charWidth = metrics.charWidth
    this.cjkCharWidth = metrics.cjkCharWidth
    this.lineHeight = metrics.lineHeight
    if (this.lineHeightOverride !== undefined) {
      this.lineHeight = this.lineHeightOverride
    }
    this.updateSessionMetrics()
    this.scrollManager.setLineHeight(this.lineHeight)
    this.updateTotalLines()
    this.scheduleRender()
  }

  private applyHostFontStyles(): void {
    this.container.style.fontFamily = this.fontFamily
    this.container.style.fontSize = `${this.fontSize}px`
  }

  private effectiveCjkCharWidth(): number {
    return (this.options.monospaceGrid ?? false) ? this.charWidth * 2.0 : this.cjkCharWidth
  }

  private updateSessionMetrics(): void {
    this.session.updateMetrics(
      this.lineHeight,
      this.charWidth,
      this.effectiveCjkCharWidth(),
      this.tabSize,
    )
    this.renderer?.updateMetrics(this.rendererMetrics())
  }

  private rendererMetrics(): RendererMetrics {
    return {
      lineHeight: this.lineHeight,
      charWidth: this.charWidth,
      cjkCharWidth: this.cjkCharWidth,
      tabSize: this.tabSize,
      monospaceGrid: this.options.monospaceGrid ?? false,
    }
  }

  private createRenderer(options: RendererOptions): EditorRenderer {
    return this.options.renderer === 'dom'
      ? new DomRenderer(this.container, options)
      : new WebGpuRenderer(this.container, options)
  }

  layout(): void {
    const contentEl = this.renderer.getContentElement()
    const rect = contentEl.getBoundingClientRect()
    this.scrollManager.setContainerHeight(rect.height > 0 ? rect.height : this.container.getBoundingClientRect().height)

    // Update gutter width in case line count changed digit count
    if (this.options.lineNumbers !== false) {
      this.renderer.updateGutterWidth(this.session.getGutterWidth())
    }

    this.scheduleRender()
  }

  // =========================================================================
  // Public API: Lifecycle
  // =========================================================================

  dispose(): void {
    if (this.disposed) return
    this.disposed = true

    // Cancel pending render
    if (this.renderRafId !== null) {
      cancelAnimationFrame(this.renderRafId)
      this.renderRafId = null
    }

    // Remove event listeners
    this.container.removeEventListener('focusin', this.handleFocusIn)
    this.container.removeEventListener('focusout', this.handleFocusOut)

    // Dispose sub-modules
    this.resizeObserver.disconnect()
    this.inputHandler.dispose()
    this.mouseHandler.dispose()
    this.scrollManager.dispose()
    this.renderer.dispose()

    // Dispose events
    this.contentChange.dispose()
    this.cursorPositionChange.dispose()
    this.selectionChange.dispose()
    this.blurEvent.dispose()
    this.focusEvent.dispose()
    this.scrollEvent.dispose()

    // Remove editor class
    this.container.classList.remove('neco-editor')
  }

  // =========================================================================
  // Input command dispatch
  // =========================================================================

  private handleInputCommand(cmd: InputCommand): void {
    if (cmd.type === 'compositionUpdate' || cmd.type === 'compositionCancel') {
      this.compositionText = cmd.type === 'compositionUpdate' ? cmd.text : ''
      this.scheduleRender()
      return
    }

    if (this.session.isReadOnly() && cmd.type !== 'copy' && cmd.type !== 'moveCursor'
      && cmd.type !== 'moveCursorByWord' && cmd.type !== 'moveCursorToLineEdge'
      && cmd.type !== 'moveCursorToDocumentEdge' && cmd.type !== 'pageMove'
      && cmd.type !== 'selectAll') {
      return
    }

    switch (cmd.type) {
      case 'insert':
        this.handleInsert(cmd.text)
        break
      case 'compositionCommit': {
        const [start, end] = this.getEditRange()
        this.compositionText = ''
        this.applyEdit(start, end, cmd.text, 'type')
        break
      }
      case 'delete':
        this.handleDelete(cmd.direction)
        break
      case 'newline':
        this.handleNewline()
        break
      case 'tab':
        this.handleTab()
        break
      case 'undo':
        this.undo()
        break
      case 'redo':
        this.redo()
        break
      case 'selectAll':
        this.setSelection(0, this.session.getTextByteLength())
        break
      case 'moveCursor':
        this.handleMoveCursor(cmd.direction, cmd.extend)
        break
      case 'moveCursorByWord':
        this.handleMoveCursorByWord(cmd.direction, cmd.extend)
        break
      case 'moveCursorToLineEdge':
        this.handleMoveCursorToLineEdge(cmd.direction, cmd.extend)
        break
      case 'moveCursorToDocumentEdge':
        this.handleMoveCursorToDocumentEdge(cmd.direction, cmd.extend)
        break
      case 'pageMove':
        this.handlePageMove(cmd.direction, cmd.extend)
        break
      case 'copy':
        void this.clipboardHandler.copy()
        break
      case 'cut':
        void this.clipboardHandler.cut()
        break
      case 'paste':
        void this.clipboardHandler.paste()
        break
    }
  }

  // =========================================================================
  // Mouse command dispatch
  // =========================================================================

  private handleMouseCommand(cmd: MouseCommand): void {
    this.inputHandler.focus()
    switch (cmd.type) {
      case 'setCursor':
        this.setCursor(cmd.offset)
        break
      case 'setSelection':
        this.setSelection(cmd.anchor, cmd.head)
        break
      case 'selectWord': {
        const text = this.session.getText()
        const [start, end] = wordBoundary(
          text,
          this.byteOffsetToUtf16(cmd.offset),
          (utf16) => this.utf16OffsetToByte(utf16),
        )
        if (start !== end) {
          this.setSelection(start, end)
        } else {
          this.setCursor(cmd.offset)
        }
        break
      }
    }
  }

  // =========================================================================
  // Text editing helpers
  // =========================================================================

  private handleInsert(text: string): void {
    const [start, end] = this.getEditRange()

    // Auto-close bracket
    if (text.length === 1) {
      const closeCode = this.session.autoCloseBracket(text.charCodeAt(0))
      if (closeCode !== null) {
        const closeChar = String.fromCharCode(closeCode)
        const changes = this.applyEdit(start, end, text + closeChar, 'type')
        // Place cursor between the opening and closing bracket
        this.cursorOffset = start + utf8ByteLength(text)
        this.cursorPositionChange.fire(this.cursorOffset)
        // applyEdit already called scheduleRender, revealCursor, etc.
        // but we need to re-fire since we moved the cursor after applyEdit
        this.revealCursor()
        this.scheduleRender()
        void changes // suppress unused warning
        return
      }
    }

    this.applyEdit(start, end, text, 'type')
  }

  private handleDelete(direction: 'backward' | 'forward'): void {
    const sel = this.getSelection()
    if (sel !== null) {
      // Delete selected text
      const start = Math.min(sel.anchor, sel.head)
      const end = Math.max(sel.anchor, sel.head)
      this.applyEdit(start, end, '', 'delete')
      return
    }

    const text = this.session.getText()
    if (direction === 'backward') {
      if (this.cursorOffset <= 0) return
      const utf16 = this.byteOffsetToUtf16(this.cursorOffset)
      this.applyEdit(
        this.utf16OffsetToByte(previousUtf16Boundary(text, utf16)),
        this.cursorOffset,
        '',
        'delete',
      )
    } else {
      if (this.cursorOffset >= this.session.getTextByteLength()) return
      const utf16 = this.byteOffsetToUtf16(this.cursorOffset)
      this.applyEdit(
        this.cursorOffset,
        this.utf16OffsetToByte(nextUtf16Boundary(text, utf16)),
        '',
        'delete',
      )
    }
  }

  private handleNewline(): void {
    const [start, end] = this.getEditRange()
    const indent = this.session.autoIndent(start)
    this.applyEdit(start, end, '\n' + indent, 'newline')
  }

  private handleTab(): void {
    const [start, end] = this.getEditRange()
    const spaces = ' '.repeat(this.tabSize)
    this.applyEdit(start, end, spaces, 'tab')
  }

  // =========================================================================
  // Cursor movement helpers
  // =========================================================================

  private handleMoveCursor(direction: 'left' | 'right' | 'up' | 'down', extend: boolean): void {
    const text = this.session.getText()
    let newOffset: number

    switch (direction) {
      case 'left':
        // If there is a selection and not extending, collapse to start
        if (!extend && this.selectionAnchor !== null && this.selectionAnchor !== this.cursorOffset) {
          newOffset = Math.min(this.selectionAnchor, this.cursorOffset)
        } else {
          newOffset = this.utf16OffsetToByte(
            previousUtf16Boundary(text, this.byteOffsetToUtf16(this.cursorOffset)),
          )
        }
        break
      case 'right':
        if (!extend && this.selectionAnchor !== null && this.selectionAnchor !== this.cursorOffset) {
          newOffset = Math.max(this.selectionAnchor, this.cursorOffset)
        } else {
          newOffset = this.utf16OffsetToByte(
            nextUtf16Boundary(text, this.byteOffsetToUtf16(this.cursorOffset)),
          )
        }
        break
      case 'up':
      case 'down': {
        const caretRect = this.session.getCaretRect(this.cursorOffset)
        const targetDocY = docY(direction === 'up'
          ? caretRect.y - this.lineHeight
          : caretRect.y + this.lineHeight)
        const scrollTop = this.scrollManager.getScrollState().scrollTop
        // WASM expects ViewY; convert DocY → ViewY
        newOffset = this.session.hitTest(caretRect.x, toViewY(targetDocY, scrollTop) as number, scrollTop)
        break
      }
      default:
        return
    }

    this.moveCursorTo(newOffset, extend)
  }

  private handleMoveCursorByWord(direction: 'left' | 'right', extend: boolean): void {
    const text = this.session.getText()
    let newOffset: number

    if (direction === 'left') {
      newOffset = findWordBoundaryLeft(
        text,
        this.byteOffsetToUtf16(this.cursorOffset),
        (utf16) => this.utf16OffsetToByte(utf16),
      )
    } else {
      newOffset = findWordBoundaryRight(
        text,
        this.byteOffsetToUtf16(this.cursorOffset),
        (utf16) => this.utf16OffsetToByte(utf16),
      )
    }

    this.moveCursorTo(newOffset, extend)
  }

  private handleMoveCursorToLineEdge(direction: 'start' | 'end', extend: boolean): void {
    const text = this.session.getText()
    let newOffset: number

    if (direction === 'start') {
      // Find the start of the current line
      let utf16 = this.byteOffsetToUtf16(this.cursorOffset)
      while (utf16 > 0 && text[utf16 - 1] !== '\n') {
        utf16--
      }
      newOffset = this.utf16OffsetToByte(utf16)
    } else {
      // Find the end of the current line
      let utf16 = this.byteOffsetToUtf16(this.cursorOffset)
      while (utf16 < text.length && text[utf16] !== '\n') {
        utf16++
      }
      newOffset = this.utf16OffsetToByte(utf16)
    }

    this.moveCursorTo(newOffset, extend)
  }

  private handleMoveCursorToDocumentEdge(direction: 'start' | 'end', extend: boolean): void {
    const newOffset = direction === 'start' ? 0 : this.session.getTextByteLength()
    this.moveCursorTo(newOffset, extend)
  }

  private handlePageMove(direction: 'up' | 'down', extend: boolean): void {
    const containerHeight = this.container.getBoundingClientRect().height
    const linesPerPage = Math.max(1, Math.floor(containerHeight / this.lineHeight))
    const caretRect = this.session.getCaretRect(this.cursorOffset)
    const delta = direction === 'up' ? -linesPerPage : linesPerPage
    const targetDocY = docY(caretRect.y + delta * this.lineHeight)
    const scrollTop = this.scrollManager.getScrollState().scrollTop
    const newOffset = this.session.hitTest(caretRect.x, toViewY(targetDocY, scrollTop) as number, scrollTop)
    this.moveCursorTo(newOffset, extend)

    // Also scroll the viewport by the same amount
    const currentScrollTop = this.getScrollTop()
    const scrollDelta = delta * this.lineHeight
    this.setScrollTop(Math.max(0, currentScrollTop + scrollDelta))
  }

  /**
   * Move the cursor to `offset`, optionally extending the selection.
   */
  private moveCursorTo(offset: number, extend: boolean): void {
    if (extend) {
      // Start selection from current cursor if no anchor yet
      if (this.selectionAnchor === null) {
        this.selectionAnchor = this.cursorOffset
      }
      this.cursorOffset = offset
      this.selectionChange.fire({ anchor: this.selectionAnchor, head: this.cursorOffset })
    } else {
      this.cursorOffset = offset
      this.selectionAnchor = null
    }

    this.cursorPositionChange.fire(this.cursorOffset)
    this.revealCursor()
    this.scheduleRender()
  }

  // =========================================================================
  // Rendering
  // =========================================================================

  private scheduleRender(): void {
    if (this.renderRafId !== null) return
    this.renderRafId = requestAnimationFrame(() => {
      this.renderRafId = null
      if (this.disposed) return
      this.render()
    })
  }

  private render(): void {
    const scrollState = this.scrollManager.getScrollState()
    const padTop = this.options.padding?.top ?? 0
    const coords = new CoordinateMap({
      gutterWidth: (this.options.lineNumbers !== false) ? this.session.getGutterWidth() : 0,
      scrollTop: scrollState.scrollTop,
      padTop,
      lineHeight: this.lineHeight,
      devicePixelRatio: window.devicePixelRatio || 1,
    })
    const contentRect = this.renderer.getContentRect()
    const height = contentRect.height > 0 ? contentRect.height : this.container.getBoundingClientRect().height

    // Use scrollState's visibleStartLine/visibleEndLine to compute the pixel
    // range that the WASM getVisibleLines expects.
    const visibleTop = scrollState.visibleStartLine * this.lineHeight
    const visibleHeight = (scrollState.visibleEndLine - scrollState.visibleStartLine) * this.lineHeight
    const effectiveHeight = visibleHeight > 0 ? visibleHeight : height

    // Get visible lines from session using the computed visible range
    const lines = this.session.getVisibleLines(visibleTop, effectiveHeight)

    // linesEl is inside scrollable contentEl.
    // translateY(offsetY) places rendered lines at their document-space Y position.
    // The browser's native scrollTop handles the viewport offset.
    // Gutter cells are inside each .neco-line, so they move in lock-step automatically.
    const linesEl = this.renderer.getLinesElement()
    linesEl.style.transform = `translateY(${scrollState.offsetY}px)`

    // Determine current line number from cursor
    let caretRect: Rect | null = null
    try {
      caretRect = this.session.getCaretRect(this.cursorOffset)
    } catch {
      // getCaretRect may fail if cursorOffset is out of range; hide caret
    }

    // Current line number: derive from caret Y position
    const currentLineNumber = caretRect && this.lineHeight > 0
      ? Math.floor(caretRect.y / this.lineHeight) + 1
      : 1

    // Render lines
    this.renderer.renderLines(lines, currentLineNumber)
    for (const lineEl of Array.from(linesEl.children)) {
      ;(lineEl as HTMLElement).style.height = `${this.lineHeight}px`
    }

    // Render caret and selection via the shared screen-space transform.
    if (caretRect) {
      const adjustedCaret = coords.docRectToScreenRect(caretRect)
      if (this.compositionText.length > 0) {
        const compositionRight = this.renderer.renderComposition(this.compositionText, adjustedCaret)
        const compositionCaret = screenRect(
          compositionRight,
          adjustedCaret.y as number,
          adjustedCaret.width as number,
          adjustedCaret.height as number,
        )
        this.inputHandler.setAnchorRect(compositionCaret)
        this.renderer.renderCaret(compositionCaret)
      } else {
        this.inputHandler.setAnchorRect(adjustedCaret)
        this.renderer.clearComposition()
        this.renderer.renderCaret(adjustedCaret)
      }
    } else {
      this.inputHandler.setAnchorRect(screenRect(0, 0, 1, 1))
      this.renderer.clearComposition()
      this.renderer.renderCaret(screenRect(0, 0, 0, 0))
    }

    // Render selections (same content-space → container-space shift as caret)
    if (this.selectionAnchor !== null && this.selectionAnchor !== this.cursorOffset) {
      const selRects = this.session.getSelectionRects(this.selectionAnchor, this.cursorOffset)
      const adjustedSelRects = selRects.map((r) => coords.docRectToScreenRect(r))
      this.renderer.renderSelections(adjustedSelRects)
    } else {
      this.renderer.renderSelections([])
    }

  }

  // =========================================================================
  // Internal helpers
  // =========================================================================

  /**
   * Returns the edit range: if there is a selection, [start, end];
   * otherwise [cursor, cursor].
   */
  private getEditRange(): [number, number] {
    if (this.selectionAnchor !== null && this.selectionAnchor !== this.cursorOffset) {
      const start = Math.min(this.selectionAnchor, this.cursorOffset)
      const end = Math.max(this.selectionAnchor, this.cursorOffset)
      return [start, end]
    }
    return [this.cursorOffset, this.cursorOffset]
  }

  private revealCursor(): void {
    this.revealOffset(this.cursorOffset)
  }

  private updateTotalLines(): void {
    const text = this.session.getText()
    // Count lines: number of newlines + 1
    let count = 1
    for (let i = 0; i < text.length; i++) {
      if (text[i] === '\n') count++
    }
    this.scrollManager.setTotalLines(count)
    // Update gutter width: line-number digit count may have changed.
    if (this.options.lineNumbers !== false) {
      this.renderer.updateGutterWidth(this.session.getGutterWidth())
    }
  }

  // -- Focus handlers (bound as arrow functions for stable identity) --------

  private handleFocusIn = (): void => {
    this.focusEvent.fire()
  }

  private handleFocusOut = (e: FocusEvent): void => {
    // Only fire blur if focus leaves the container entirely
    if (!this.container.contains(e.relatedTarget as Node)) {
      this.blurEvent.fire()
    }
  }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value))
}

const WORD_RE = /\w/

function utf8ByteLength(text: string): number {
  return new TextEncoder().encode(text).length
}

function previousUtf16Boundary(text: string, utf16Offset: number): number {
  if (utf16Offset <= 0) return 0
  const prev = text.charCodeAt(utf16Offset - 1)
  if (
    utf16Offset >= 2 &&
    prev >= 0xdc00 &&
    prev <= 0xdfff
  ) {
    const lead = text.charCodeAt(utf16Offset - 2)
    if (lead >= 0xd800 && lead <= 0xdbff) {
      return utf16Offset - 2
    }
  }
  return utf16Offset - 1
}

function nextUtf16Boundary(text: string, utf16Offset: number): number {
  if (utf16Offset >= text.length) return text.length
  const cp = text.codePointAt(utf16Offset)
  return utf16Offset + ((cp ?? 0) > 0xffff ? 2 : 1)
}

function findWordBoundaryLeft(
  text: string,
  utf16Offset: number,
  utf16ToByte: (utf16: number) => number,
): number {
  if (utf16Offset <= 0) return 0
  let pos = previousUtf16Boundary(text, utf16Offset)
  // Skip non-word characters
  while (pos > 0 && !WORD_RE.test(text.charAt(pos))) {
    pos = previousUtf16Boundary(text, pos)
  }
  // Skip word characters
  while (pos > 0 && WORD_RE.test(text.charAt(pos - 1))) {
    pos = previousUtf16Boundary(text, pos)
  }
  return utf16ToByte(pos)
}

function findWordBoundaryRight(
  text: string,
  utf16Offset: number,
  utf16ToByte: (utf16: number) => number,
): number {
  const len = text.length
  if (utf16Offset >= len) return utf16ToByte(len)
  let pos = utf16Offset
  // Skip word characters
  while (pos < len && WORD_RE.test(text.charAt(pos))) {
    pos = nextUtf16Boundary(text, pos)
  }
  // Skip non-word characters
  while (pos < len && !WORD_RE.test(text.charAt(pos))) {
    pos = nextUtf16Boundary(text, pos)
  }
  return utf16ToByte(pos)
}

function wordBoundary(
  text: string,
  utf16Offset: number,
  utf16ToByte: (utf16: number) => number,
): [start: number, end: number] {
  if (
    utf16Offset < 0 ||
    utf16Offset >= text.length ||
    !WORD_RE.test(text.charAt(utf16Offset))
  ) {
    const byte = utf16ToByte(Math.max(0, Math.min(text.length, utf16Offset)))
    return [byte, byte]
  }
  return [
    findWordBoundaryLeft(text, utf16Offset, utf16ToByte),
    findWordBoundaryRight(text, utf16Offset, utf16ToByte),
  ]
}
