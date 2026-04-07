declare const _brand: unique symbol
type Branded<T, B extends string> = T & { [_brand]: B }

// Document space: coordinates returned by WASM caret_rect / selection_rects.
// y = visual_line * line_height (absolute from document top)
export type DocX = Branded<number, 'DocX'>
export type DocY = Branded<number, 'DocY'>

// Viewport space: coordinates relative to the viewport top edge.
// WASM hit_test expects y in this space.
export type ViewY = Branded<number, 'ViewY'>

// Container space: coordinates relative to the top-left of .neco-editor DOM container.
// Derived from mouse events as (clientX - rect.left, clientY - rect.top).
export type ContX = Branded<number, 'ContX'>
export type ContY = Branded<number, 'ContY'>

export function docX(n: number): DocX { return n as unknown as DocX }
export function docY(n: number): DocY { return n as unknown as DocY }
export function viewY(n: number): ViewY { return n as unknown as ViewY }
export function contX(n: number): ContX { return n as unknown as ContX }
export function contY(n: number): ContY { return n as unknown as ContY }

export interface LayoutParams {
  gutterWidth: number
  scrollTop: number
  padTop: number
  lineHeight: number
}

export class CoordinateMap {
  constructor(private readonly p: Readonly<LayoutParams>) {}

  /**
   * Document → absolute position inside scrollable contentEl.
   * caretEl/selectionEl are absolute children of contentEl.
   * The browser automatically offsets by scrollTop, so we do not subtract it manually.
   * padTop corrects the gap between the absolute anchor (padding box top) and content start.
   */
  docToAbsoluteY(y: DocY): number {
    return (y as number) + this.p.padTop
  }

  /**
   * Document → ViewportY (y argument for WASM hitTest).
   * hit_test internally adds scroll_top to convert back to DocY.
   */
  docToViewY(y: DocY): ViewY {
    return ((y as number) - this.p.scrollTop) as ViewY
  }

  get gutterWidth(): number { return this.p.gutterWidth }
  get scrollTop(): number { return this.p.scrollTop }
  get padTop(): number { return this.p.padTop }
}

// ---------------------------------------------------------------------------
// Standalone transform for use outside render() where a full CoordinateMap
// is not available (e.g. cursor movement, page move).
// ---------------------------------------------------------------------------

/** Convert DocY to ViewY given a scrollTop value. */
export function toViewY(y: DocY, scrollTop: number): ViewY {
  return ((y as number) - scrollTop) as ViewY
}
