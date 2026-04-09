/** A rendered line with tokenization data for display. */
export interface RenderLine {
  lineNumber: number
  text: string
  tokens: TokenSpan[]
}

/** A classified token span within a single line. */
import type { BlockAdvance, InlineAdvance } from './coordinates'

export interface TokenSpan {
  start: number
  end: number
  kind: string
}

/** Axis-aligned rectangle in pixel coordinates. */
export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

export type LayoutMode = 'horizontal-ltr' | 'vertical-rl' | 'vertical-lr'

export interface VisualLayoutSpace {
  logicalLine: number
  visualLine: number
  inlineAdvance: InlineAdvance
  blockAdvance: BlockAdvance
  layoutMode: LayoutMode
}

export interface VisualLineFrame extends VisualLayoutSpace {}

/** A search hit with byte range and position. */
export interface SearchMatchInfo {
  start: number
  end: number
  line: number
  column: number
}

/** Detected indentation style. */
export interface IndentInfo {
  style: 'tabs' | 'spaces'
  width?: number
}

/** Structured error from the WASM layer. */
export interface EditorError {
  domain: string
  code: string
  message: string
}

/** Describes how a text range shifted after an edit. */
export interface RangeChange {
  start: number
  oldEnd: number
  newEnd: number
}

/** A matched pair of brackets at byte offsets. */
export interface BracketPair {
  open: number
  close: number
}

/** Options for search operations. */
export interface SearchOptions {
  isRegex?: boolean
  caseSensitive?: boolean
  wholeWord?: boolean
}

/** Measured dimensions for a monospace font at a given size. */
export interface FontMetrics {
  charWidth: number
  lineHeight: number
}
