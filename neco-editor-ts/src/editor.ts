import { EditorHandle } from 'neco-editor-wasm'
import type {
  RenderLine,
  RangeChange,
  Rect,
  VisualLineFrame,
  SearchMatchInfo,
  DecorationInfo,
  IndentInfo,
  BracketPair,
  SearchOptions,
  TokenSpan,
} from './types'

/**
 * Type-safe wrapper around the WASM EditorHandle.
 *
 * Manages the lifecycle of a single editor buffer with integrated
 * viewport geometry, syntax highlighting, and search.
 */
export class EditorSession {
  private handle: EditorHandle

  constructor(
    text: string,
    language: string,
    lineHeight: number,
    charWidth: number,
    cjkCharWidth: number,
    tabWidth: number,
  ) {
    this.handle = new EditorHandle(text, language, lineHeight, charWidth, cjkCharWidth, tabWidth)
  }

  // -- Editing ---------------------------------------------------------------

  applyEdit(start: number, end: number, newText: string, label?: string): RangeChange[] {
    return this.handle.applyEdit(start, end, newText, label ?? '') as RangeChange[]
  }

  undo(): boolean {
    const result = this.handle.undo()
    return result !== null && result !== undefined
  }

  redo(): boolean {
    const result = this.handle.redo()
    return result !== null && result !== undefined
  }

  // -- Display ---------------------------------------------------------------

  getVisibleLines(scrollTop: number, height: number): RenderLine[] {
    return this.handle.getVisibleLines(scrollTop, height) as RenderLine[]
  }

  getCaretRect(offset: number): Rect {
    return this.handle.getCaretRect(offset) as Rect
  }

  getSelectionRects(anchor: number, head: number): Rect[] {
    return this.handle.getSelectionRects(anchor, head) as Rect[]
  }

  getVisualLineFrame(visualLine: number): VisualLineFrame {
    return this.handle.getVisualLineFrame(visualLine) as VisualLineFrame
  }

  hitTest(x: number, y: number, scrollTop: number): number {
    return this.handle.hitTest(x, y, scrollTop)
  }

  tokenizeLine(line: string): TokenSpan[] {
    return this.handle.tokenizeLine(line) as TokenSpan[]
  }

  // -- Search ----------------------------------------------------------------

  search(pattern: string, options?: SearchOptions): SearchMatchInfo[] {
    return this.handle.search(
      pattern,
      options?.isRegex ?? false,
      options?.caseSensitive ?? true,
      options?.wholeWord ?? false,
    ) as SearchMatchInfo[]
  }

  getSearchMatches(): SearchMatchInfo[] {
    return this.handle.getSearchMatches() as SearchMatchInfo[]
  }

  findPrevious(pattern: string, fromOffset: number, options?: SearchOptions): SearchMatchInfo | null {
    return this.handle.findPrevious(
      pattern,
      fromOffset,
      options?.isRegex ?? false,
      options?.caseSensitive ?? true,
      options?.wholeWord ?? false,
    ) as SearchMatchInfo | null
  }

  replaceAll(pattern: string, replacement: string, options?: SearchOptions): number {
    return this.handle.replaceAll(
      pattern,
      replacement,
      options?.isRegex ?? false,
      options?.caseSensitive ?? true,
      options?.wholeWord ?? false,
    ) as number
  }

  replaceNext(
    pattern: string,
    replacement: string,
    fromOffset: number,
    options?: SearchOptions,
  ): SearchMatchInfo | null {
    return this.handle.replaceNext(
      pattern,
      replacement,
      fromOffset,
      options?.isRegex ?? false,
      options?.caseSensitive ?? true,
      options?.wholeWord ?? false,
    ) as SearchMatchInfo | null
  }

  // -- Decorations ----------------------------------------------------------

  addDecoration(
    start: number,
    end: number,
    tag: number,
    kind: 'highlight' | 'marker' | 'widget',
  ): string {
    return this.handle.addDecoration(start, end, tag, kind) as string
  }

  removeDecoration(id: string): boolean {
    return this.handle.removeDecoration(id) as boolean
  }

  clearDecorationsByTag(tag: number): void {
    this.handle.clearDecorationsByTag(tag)
  }

  queryDecorations(start: number, end: number): DecorationInfo[] {
    return this.handle.queryDecorations(start, end) as DecorationInfo[]
  }

  // -- State -----------------------------------------------------------------

  getText(): string {
    return this.handle.getText()
  }

  getTextByteLength(): number {
    return this.handle.getTextByteLength()
  }

  byteOffsetToUtf16(offset: number): number {
    return this.handle.byteOffsetToUtf16(offset)
  }

  utf16OffsetToByte(offset: number): number {
    return this.handle.utf16OffsetToByte(offset)
  }

  isDirty(): boolean {
    return this.handle.isDirty()
  }

  /** Mark the current state as clean (e.g. after saving). */
  markClean(): void {
    this.handle.markClean()
  }

  detectIndent(sampleLines?: number): IndentInfo {
    return this.handle.detectIndent(sampleLines ?? 100) as IndentInfo
  }

  // -- Viewport --------------------------------------------------------------

  updateMetrics(
    lineHeight: number,
    charWidth: number,
    cjkCharWidth: number,
    tabWidth: number,
  ): void {
    this.handle.updateMetrics(lineHeight, charWidth, cjkCharWidth, tabWidth)
  }

  getGutterWidth(): number {
    return this.handle.getGutterWidth()
  }

  scrollToReveal(offset: number, scrollTop: number, containerHeight: number): number | null {
    const result = this.handle.scrollToReveal(offset, scrollTop, containerHeight)
    if (result === null || result === undefined) return null
    return result as number
  }

  // -- Read-only -------------------------------------------------------------

  setReadOnly(value: boolean): void {
    this.handle.setReadOnly(value)
  }

  isReadOnly(): boolean {
    return this.handle.isReadOnly()
  }

  // -- Auto-indent -----------------------------------------------------------

  autoIndent(offset: number): string {
    return this.handle.autoIndent(offset)
  }

  // -- Auto close bracket ----------------------------------------------------

  autoCloseBracket(ch: number): number | null {
    const result = this.handle.autoCloseBracket(ch)
    if (result === null || result === undefined) return null
    return result as number
  }

  // -- Paste indent adjustment -----------------------------------------------

  adjustPasteIndent(text: string, offset: number): string {
    return this.handle.adjustPasteIndent(text, offset)
  }

  // -- Bracket matching ------------------------------------------------------

  findMatchingBracket(offset: number): BracketPair | null {
    const result = this.handle.findMatchingBracket(offset)
    if (result === null || result === undefined) return null
    return result as BracketPair
  }

  // -- Lifecycle -------------------------------------------------------------

  /** Explicitly free WASM memory. Call when the session is no longer needed. */
  free(): void {
    this.handle.free()
  }
}
