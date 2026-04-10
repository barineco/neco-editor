import type { FontMetrics } from './types'

/** Default line-height multiplier relative to font size. */
const DEFAULT_LINE_HEIGHT_RATIO = 1.5

/**
 * Measure the character width of a monospace font using Canvas measureText.
 *
 * This is separated from the main entry point so tests can provide
 * a mock CanvasRenderingContext2D without needing a real DOM.
 */
export function measureCharWidth(
  ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
  fontFamily: string,
  fontSize: number,
): number {
  ctx.font = `${fontSize}px ${fontFamily}`
  // Measure a representative set of characters to get the monospace advance width.
  // Using 'x' is standard practice; averaging over more characters guards against
  // fonts that report slightly different advances for different glyphs.
  const sample = 'MMMMMMMMMM'
  const metrics = ctx.measureText(sample)
  return metrics.width / sample.length
}

/** Measure the rendered advance of CJK full-width glyphs for the active font stack. */
export function measureCjkCharWidth(
  ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
  fontFamily: string,
  fontSize: number,
): number {
  ctx.font = `${fontSize}px ${fontFamily}`
  const sample = 'ああああああああああ'
  const metrics = ctx.measureText(sample)
  return metrics.width / sample.length
}

/**
 * Compute line height from a container element's computed style.
 *
 * Handles both pixel values ("21px") and unitless multipliers ("1.5"):
 * - "21px" → returned as 21
 * - "1.5"  → returned as Math.round(1.5 * fontSize)
 *
 * Falls back to `fontSize * DEFAULT_LINE_HEIGHT_RATIO` when the
 * computed lineHeight is 'normal' or otherwise unparseable.
 */
export function resolveLineHeight(container: HTMLElement, fontSize: number): number {
  const computed = getComputedStyle(container).lineHeight
  if (computed && computed !== 'normal') {
    const parsed = parseFloat(computed)
    if (Number.isFinite(parsed) && parsed > 0) {
      // Pixel value: use as-is.
      if (computed.endsWith('px')) {
        return parsed
      }
      // Unitless multiplier: scale by fontSize.
      return Math.round(parsed * fontSize)
    }
  }
  return Math.round(fontSize * DEFAULT_LINE_HEIGHT_RATIO)
}

/**
 * Create a CanvasRenderingContext2D for text measurement.
 *
 * Prefers OffscreenCanvas when available (no DOM allocation),
 * otherwise creates a regular HTMLCanvasElement.
 */
function createMeasureContext(): CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D {
  if (typeof OffscreenCanvas !== 'undefined') {
    const canvas = new OffscreenCanvas(1, 1)
    const ctx = canvas.getContext('2d')
    if (ctx) return ctx
  }
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')
  if (!ctx) {
    throw new Error('metrics: failed to obtain Canvas 2D context')
  }
  return ctx
}

/**
 * Measure font metrics for a monospace font rendered in the given container.
 *
 * The container is used only to resolve CSS `line-height`; if no CSS
 * line-height is set, the default ratio (1.5x fontSize) is used.
 *
 * @param container - The DOM element that will host the editor (used for CSS resolution).
 * @param fontFamily - CSS font-family string (e.g. `"Fira Code", monospace`).
 * @param fontSize - Font size in pixels.
 * @returns Measured character width and line height.
 */
export function measureFontMetrics(
  container: HTMLElement,
  fontFamily: string,
  fontSize: number,
): FontMetrics {
  const ctx = createMeasureContext()
  const charWidth = measureCharWidth(ctx, fontFamily, fontSize)
  const cjkCharWidth = measureCjkCharWidth(ctx, fontFamily, fontSize)
  const lineHeight = resolveLineHeight(container, fontSize)
  return { charWidth, cjkCharWidth, lineHeight }
}
