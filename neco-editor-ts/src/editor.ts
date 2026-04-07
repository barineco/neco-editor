import { EditorHandle } from 'neco-editor-wasm'
import type {
  RenderLine,
  RangeChange,
  Rect,
  SearchMatchInfo,
  IndentInfo,
  BracketPair,
  EditorError,
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
    tabWidth: number,
  ) {
    this.handle = new EditorHandle(text, language, lineHeight, charWidth, tabWidth)
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

  // -- State -----------------------------------------------------------------

  getText(): string {
    return this.handle.getText()
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

  updateMetrics(lineHeight: number, charWidth: number, tabWidth: number): void {
    this.handle.updateMetrics(lineHeight, charWidth, tabWidth)
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

  /** Returns the leading whitespace of the line containing `offset`. */
  autoIndent(offset: number): string {
    return this.handle.autoIndent(offset)
  }

  // -- Auto close bracket ----------------------------------------------------

  /** Returns the closing bracket/quote char code for an opening one, or null. */
  autoCloseBracket(ch: number): number | null {
    const result = this.handle.autoCloseBracket(ch)
    if (result === null || result === undefined) return null
    return result as number
  }

  // -- Paste indent adjustment -----------------------------------------------

  /** Adjusts indentation of pasted text. Currently returns input unchanged. */
  adjustPasteIndent(text: string, offset: number): string {
    return this.handle.adjustPasteIndent(text, offset)
  }

  // -- Bracket matching ------------------------------------------------------

  /** Finds the matching bracket at `offset`. Returns pair or null. */
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
