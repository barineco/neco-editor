declare const _brand: unique symbol
type Branded<T, B extends string> = T & { [_brand]: B }

// Document space: coordinates returned by WASM caret_rect / selection_rects.
// y = visual_line * line_height (absolute from document top)
export type DocX = Branded<number, 'DocX'>
export type DocY = Branded<number, 'DocY'>

// Layout-space advances before screen placement.
export type InlineAdvance = Branded<number, 'InlineAdvance'>
export type BlockAdvance = Branded<number, 'BlockAdvance'>

// CSS pixel values used at the DOM boundary.
export type CssPx = Branded<number, 'CssPx'>
export type DevicePx = Branded<number, 'DevicePx'>

// Viewport space: coordinates relative to the viewport top edge.
// WASM hit_test expects y in this space.
export type ViewportX = Branded<number, 'ViewportX'>
export type ViewY = Branded<number, 'ViewY'>
export type ViewportY = ViewY

// Screen space: final CSS placement after gutter / padding transforms.
export type ScreenX = Branded<number, 'ScreenX'>
export type ScreenY = Branded<number, 'ScreenY'>

// Container space: coordinates relative to the top-left of .neco-editor DOM container.
// Derived from mouse events as (clientX - rect.left, clientY - rect.top).
export type ContainerX = Branded<number, 'ContainerX'>
export type ContainerY = Branded<number, 'ContainerY'>
export type ContX = ContainerX
export type ContY = ContainerY

export function docX(n: number): DocX { return n as unknown as DocX }
export function docY(n: number): DocY { return n as unknown as DocY }
export function inlineAdvance(n: number): InlineAdvance { return n as unknown as InlineAdvance }
export function blockAdvance(n: number): BlockAdvance { return n as unknown as BlockAdvance }
export function cssPx(n: number): CssPx { return n as unknown as CssPx }
export function devicePx(n: number): DevicePx { return n as unknown as DevicePx }
export function viewportX(n: number): ViewportX { return n as unknown as ViewportX }
export function viewY(n: number): ViewY { return n as unknown as ViewY }
export function screenX(n: number): ScreenX { return n as unknown as ScreenX }
export function screenY(n: number): ScreenY { return n as unknown as ScreenY }
export function containerX(n: number): ContainerX { return n as unknown as ContainerX }
export function containerY(n: number): ContainerY { return n as unknown as ContainerY }
export const contX = containerX
export const contY = containerY

export interface LayoutParams {
  gutterWidth: number
  scrollTop: number
  padTop: number
  lineHeight: number
  devicePixelRatio?: number
}

export interface CoordinateRect {
  x: number
  y: number
  width: number
  height: number
}

export interface ScreenRect {
  x: ScreenX
  y: ScreenY
  width: CssPx
  height: CssPx
}

export class CoordinateMap {
  constructor(private readonly p: Readonly<LayoutParams>) {}

  /**
   * Document → absolute position inside scrollable contentEl.
   * caretEl/selectionEl are absolute children of contentEl.
   * The browser automatically offsets by scrollTop, so we do not subtract it manually.
   * padTop corrects the gap between the absolute anchor (padding box top) and content start.
   */
  docToAbsoluteY(y: DocY): ScreenY {
    return screenY((y as number) + this.p.padTop)
  }

  /**
   * Document → ViewportY (y argument for WASM hitTest).
   * hit_test internally adds scroll_top to convert back to DocY.
   */
  docToViewY(y: DocY): ViewY {
    return ((y as number) - this.p.scrollTop) as ViewY
  }

  containerToViewportX(x: ContainerX): ViewportX {
    return viewportX((x as number) - this.p.gutterWidth)
  }

  containerToViewportY(y: ContainerY): ViewportY {
    return viewY((y as number) - this.p.padTop)
  }

  contentXToScreenX(x: CssPx | number): ScreenX {
    return screenX((x as number) + this.p.gutterWidth)
  }

  docRectToScreenRect(rect: CoordinateRect): ScreenRect {
    return {
      x: this.contentXToScreenX(cssPx(rect.x)),
      y: this.docToAbsoluteY(docY(rect.y)),
      width: cssPx(rect.width),
      height: cssPx(rect.height),
    }
  }

  cssPxToDevicePx(value: CssPx | number): DevicePx {
    return devicePx((value as number) * (this.p.devicePixelRatio ?? 1))
  }

  get gutterWidth(): number { return this.p.gutterWidth }
  get scrollTop(): number { return this.p.scrollTop }
  get padTop(): number { return this.p.padTop }
  get lineHeight(): number { return this.p.lineHeight }
}

// ---------------------------------------------------------------------------
// Standalone transform for use outside render() where a full CoordinateMap
// is not available (e.g. cursor movement, page move).
// ---------------------------------------------------------------------------

/** Convert DocY to ViewY given a scrollTop value. */
export function toViewY(y: DocY, scrollTop: number): ViewY {
  return ((y as number) - scrollTop) as ViewY
}

export function containerOffsetFromMouseEvent(
  container: HTMLElement,
  event: MouseEvent,
): { x: ContainerX; y: ContainerY } {
  const rect = container.getBoundingClientRect()
  return {
    x: containerX(event.clientX - rect.left),
    y: containerY(event.clientY - rect.top),
  }
}

export function screenRect(x: number, y: number, width: number, height: number): ScreenRect {
  return {
    x: screenX(x),
    y: screenY(y),
    width: cssPx(width),
    height: cssPx(height),
  }
}
