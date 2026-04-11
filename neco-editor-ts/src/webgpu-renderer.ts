import { screenX, type ScreenRect } from './coordinates'
import type { RenderLine } from './types'
import {
  type EditorRenderer,
  type RendererMetrics,
  type RendererOptions,
} from './renderer'

type GpuApi = {
  getPreferredCanvasFormat(): string
  requestAdapter(): Promise<GpuAdapter | null>
}

type GpuAdapter = {
  requestDevice(): Promise<GpuDevice>
}

type GpuDevice = {
  queue: {
    writeBuffer(buffer: GpuBuffer, bufferOffset: number, data: Float32Array): void
    copyExternalImageToTexture(
      source: { source: HTMLCanvasElement | OffscreenCanvas },
      destination: { texture: GpuTexture },
      copySize: { width: number; height: number },
    ): void
    submit(commandBuffers: GpuCommandBuffer[]): void
  }
  destroy?(): void
  createBindGroup(descriptor: unknown): GpuBindGroup
  createBuffer(descriptor: { size: number; usage: number }): GpuBuffer
  createCommandEncoder(): GpuCommandEncoder
  createRenderPipeline(descriptor: unknown): GpuRenderPipeline
  createSampler(descriptor: unknown): GpuSampler
  createShaderModule(descriptor: { code: string }): GpuShaderModule
  createTexture(descriptor: {
    size: { width: number; height: number }
    format: string
    usage: number
  }): GpuTexture
}

type GpuBuffer = {
  destroy?(): void
}

type GpuTexture = {
  createView(): GpuTextureView
  destroy?(): void
}

type GpuTextureView = unknown
type GpuSampler = unknown
type GpuShaderModule = unknown
type GpuRenderPipeline = {
  getBindGroupLayout(index: number): unknown
}
type GpuBindGroup = unknown
type GpuCommandBuffer = unknown

type GpuCanvasContext = {
  configure(descriptor: {
    device: GpuDevice
    format: string
    alphaMode: string
  }): void
  getCurrentTexture(): GpuTexture
}

type GpuCommandEncoder = {
  beginRenderPass(descriptor: unknown): GpuRenderPassEncoder
  finish(): GpuCommandBuffer
}

type GpuRenderPassEncoder = {
  setPipeline(pipeline: GpuRenderPipeline): void
  setBindGroup(index: number, bindGroup: GpuBindGroup): void
  setVertexBuffer(slot: number, buffer: GpuBuffer): void
  draw(vertexCount: number): void
  end(): void
}

type Glyph = {
  x: number
  y: number
  width: number
  height: number
  cssWidth: number
  cssHeight: number
  advance: number
}

type Rgba = [number, number, number, number]

const FLOATS_PER_VERTEX = 8
const GLYPH_ATLAS_FORMAT = 'rgba8unorm'
const GPU_BUFFER_USAGE_VERTEX_COPY_DST = 32 | 8
const GPU_TEXTURE_USAGE_COPY_DST_TEXTURE_BINDING_RENDER_ATTACHMENT = 2 | 4 | 16

const WGSL = `
struct VertexOut {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
  @location(0) pos: vec2<f32>,
  @location(1) uv: vec2<f32>,
  @location(2) color: vec4<f32>,
) -> VertexOut {
  var out: VertexOut;
  out.position = vec4<f32>(pos, 0.0, 1.0);
  out.uv = uv;
  out.color = color;
  return out;
}

@group(0) @binding(0) var glyphSampler: sampler;
@group(0) @binding(1) var glyphAtlas: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
  let atlas = textureSample(glyphAtlas, glyphSampler, in.uv);
  return vec4<f32>(in.color.rgb, in.color.a * atlas.a);
}
`

export class WebGpuRenderer implements EditorRenderer {
  private container: HTMLElement
  private options: RendererOptions
  private metrics: RendererMetrics
  private gutterBgEl: HTMLElement
  private contentEl: HTMLElement
  private linesEl: HTMLElement
  private canvas: HTMLCanvasElement
  private atlasCanvas: HTMLCanvasElement
  private atlasCtx: CanvasRenderingContext2D
  private glyphs = new Map<string, Glyph>()
  private atlasScale = 1
  private atlasX = 1
  private atlasY = 1
  private atlasRowHeight = 0
  private caretRect: ScreenRect | null = null
  private selectionRects: ScreenRect[] = []
  private compositionText = ''
  private compositionRect: ScreenRect | null = null
  private lines: RenderLine[] = []
  private currentLineNumber = 1
  private ready: Promise<void>
  private device: GpuDevice | null = null
  private context: GpuCanvasContext | null = null
  private format = ''
  private pipeline: GpuRenderPipeline | null = null
  private bindGroup: GpuBindGroup | null = null
  private sampler: GpuSampler | null = null
  private atlasTexture: GpuTexture | null = null
  private vertexBuffer: GpuBuffer | null = null
  private vertexBufferSize = 0
  private disposed = false

  constructor(container: HTMLElement, options: RendererOptions) {
    this.container = container
    this.options = options
    this.metrics = options.metrics ?? {
      lineHeight: 21,
      charWidth: 8,
      cjkCharWidth: 14,
      tabSize: 4,
      monospaceGrid: false,
    }

    const gpu = webGpu()
    if (gpu === null) {
      throw new Error('WebGPU is not supported in this environment')
    }

    this.gutterBgEl = document.createElement('div')
    this.gutterBgEl.className = 'neco-editor-gutter-bg'

    this.contentEl = document.createElement('div')
    this.contentEl.className = 'neco-editor-content'

    this.linesEl = document.createElement('div')
    this.linesEl.className = 'neco-editor-lines'

    this.canvas = document.createElement('canvas')
    this.canvas.className = 'neco-webgpu-canvas'
    this.canvas.style.position = 'absolute'
    this.canvas.style.top = '0'
    this.canvas.style.left = '0'
    this.canvas.style.zIndex = '0'
    this.canvas.style.pointerEvents = 'none'

    this.atlasCanvas = document.createElement('canvas')
    this.atlasCanvas.width = 2048
    this.atlasCanvas.height = 2048
    const atlasCtx = this.atlasCanvas.getContext('2d')
    if (atlasCtx === null) {
      throw new Error('2D canvas is required to build the WebGPU glyph atlas')
    }
    this.atlasCtx = atlasCtx
    this.resetAtlasStyle()

    const padTop = options.padding?.top ?? 0
    const padBottom = options.padding?.bottom ?? 0
    if (padTop > 0) this.contentEl.style.paddingTop = `${padTop}px`
    if (padBottom > 0) this.contentEl.style.paddingBottom = `${padBottom}px`

    container.style.setProperty('--neco-gutter-width', `${options.gutterWidth}px`)
    container.appendChild(this.gutterBgEl)
    this.contentEl.appendChild(this.linesEl)
    this.contentEl.appendChild(this.canvas)
    container.appendChild(this.contentEl)

    this.ready = this.initialize(gpu)
  }

  renderLines(lines: RenderLine[], currentLineNumber: number): void {
    this.lines = lines
    this.currentLineNumber = currentLineNumber
    this.scheduleDraw()
  }

  renderCaret(rect: ScreenRect): void {
    this.caretRect = rect
    this.scheduleDraw()
  }

  renderSelections(rects: ScreenRect[]): void {
    this.selectionRects = rects
    this.scheduleDraw()
  }

  renderComposition(text: string, rect: ScreenRect): number {
    this.compositionText = text
    this.compositionRect = rect
    const width = this.measureTextAdvance(text)
    this.scheduleDraw()
    return screenX((rect.x as number) + width) as number
  }

  clearComposition(): void {
    this.compositionText = ''
    this.compositionRect = null
    this.scheduleDraw()
  }

  updateGutterWidth(width: number): void {
    this.options.gutterWidth = width
    this.container.style.setProperty('--neco-gutter-width', `${width}px`)
    this.scheduleDraw()
  }

  updateMetrics(metrics: RendererMetrics): void {
    this.metrics = metrics
    this.resetGlyphAtlas()
    this.scheduleDraw()
  }

  getContentElement(): HTMLElement {
    return this.contentEl
  }

  getLinesElement(): HTMLElement {
    return this.linesEl
  }

  getContentRect(): { width: number; height: number } {
    return {
      width: this.contentEl.clientWidth,
      height: this.contentEl.clientHeight,
    }
  }

  clear(): void {
    this.lines = []
    this.selectionRects = []
    this.caretRect = null
    this.clearComposition()
  }

  dispose(): void {
    this.disposed = true
    this.vertexBuffer?.destroy?.()
    this.atlasTexture?.destroy?.()
    this.device?.destroy?.()
    this.container.removeChild(this.gutterBgEl)
    this.container.removeChild(this.contentEl)
    this.container.style.removeProperty('--neco-gutter-width')
  }

  private async initialize(gpu: GpuApi): Promise<void> {
    const adapter = await gpu.requestAdapter()
    if (adapter === null) {
      throw new Error('WebGPU adapter request failed')
    }
    const device = await adapter.requestDevice()
    if (this.disposed) return

    const context = this.canvas.getContext('webgpu') as GpuCanvasContext | null
    if (context === null) {
      throw new Error('WebGPU canvas context is not available')
    }

    const format = gpu.getPreferredCanvasFormat()
    context.configure({ device, format, alphaMode: 'premultiplied' })

    const shader = device.createShaderModule({ code: WGSL })
    const pipeline = device.createRenderPipeline({
      layout: 'auto',
      vertex: {
        module: shader,
        entryPoint: 'vs_main',
        buffers: [{
          arrayStride: FLOATS_PER_VERTEX * 4,
          attributes: [
            { shaderLocation: 0, offset: 0, format: 'float32x2' },
            { shaderLocation: 1, offset: 8, format: 'float32x2' },
            { shaderLocation: 2, offset: 16, format: 'float32x4' },
          ],
        }],
      },
      fragment: {
        module: shader,
        entryPoint: 'fs_main',
        targets: [{
          format,
          blend: {
            color: {
              srcFactor: 'src-alpha',
              dstFactor: 'one-minus-src-alpha',
              operation: 'add',
            },
            alpha: {
              srcFactor: 'one',
              dstFactor: 'one-minus-src-alpha',
              operation: 'add',
            },
          },
        }],
      },
      primitive: { topology: 'triangle-list' },
    })

    this.device = device
    this.context = context
    this.format = format
    this.pipeline = pipeline
    this.sampler = device.createSampler({
      magFilter: 'linear',
      minFilter: 'linear',
    })
    this.uploadAtlas()
    this.scheduleDraw()
  }

  private scheduleDraw(): void {
    void this.ready.then(() => {
      if (!this.disposed) this.draw()
    }).catch((err: unknown) => {
      console.error('[neco-editor] WebGPU initialization failed:', err)
    })
  }

  private draw(): void {
    const device = this.device
    const context = this.context
    const pipeline = this.pipeline
    if (device === null || context === null || pipeline === null) return

    this.resizeCanvas()
    this.updateAtlasScale()
    this.prepareGlyphsForFrame()
    this.uploadAtlas()
    const bindGroup = this.bindGroup
    if (bindGroup === null) return

    const vertices = this.buildVertices()
    if (vertices.length === 0) return
    this.ensureVertexBuffer(device, vertices.byteLength)
    if (this.vertexBuffer === null) return

    device.queue.writeBuffer(this.vertexBuffer, 0, vertices)
    const encoder = device.createCommandEncoder()
    const pass = encoder.beginRenderPass({
      colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: this.cssColorToRgba('--neco-editor-bg'),
        loadOp: 'clear',
        storeOp: 'store',
      }],
    })
    pass.setPipeline(pipeline)
    pass.setBindGroup(0, bindGroup)
    pass.setVertexBuffer(0, this.vertexBuffer)
    pass.draw(vertices.length / FLOATS_PER_VERTEX)
    pass.end()
    device.queue.submit([encoder.finish()])
  }

  private prepareGlyphsForFrame(): void {
    for (const line of this.lines) {
      if (this.options.showLineNumbers) {
        this.ensureTextGlyphs(String(line.lineNumber))
      }
      this.ensureTextGlyphs(line.text)
    }
    if (this.compositionText.length > 0) {
      this.ensureTextGlyphs(this.compositionText)
    }
  }

  private buildVertices(): Float32Array {
    const dpr = window.devicePixelRatio || 1
    const canvasWidth = this.canvas.width / dpr
    const canvasHeight = this.canvas.height / dpr
    if (canvasWidth <= 0 || canvasHeight <= 0) return new Float32Array()

    const values: number[] = []
    const lineOffsetY = parseTranslateY(this.linesEl.style.transform)
    const currentLineColor = this.cssColorToRgba('--neco-current-line-bg')
    const selectionColor = this.cssColorToRgba('--neco-selection-bg')
    const cursorColor = this.cssColorToRgba('--neco-cursor-color')
    const compositionColor = [0.47, 0.63, 0.82, 0.18] as Rgba

    for (let i = 0; i < this.lines.length; i++) {
      const line = this.lines[i]
      if (line === undefined) continue
      const y = lineOffsetY + i * this.metrics.lineHeight
      if (line.lineNumber === this.currentLineNumber) {
        pushSolidQuad(values, 0, y, canvasWidth, this.metrics.lineHeight, currentLineColor, this.atlasCanvas.width, this.atlasCanvas.height, canvasWidth, canvasHeight)
      }
      if (this.options.showLineNumbers) {
        const gutterColor = this.cssColorToRgba('--neco-gutter-fg')
        const gutterText = String(line.lineNumber)
        const gutterWidth = this.measureTextAdvance(gutterText)
        this.pushText(values, gutterText, this.options.gutterWidth - gutterWidth - 8, y, gutterColor, canvasWidth, canvasHeight)
      }
      this.pushTokenizedLine(values, line, this.options.gutterWidth, y, canvasWidth, canvasHeight)
    }

    for (const rect of this.selectionRects) {
      pushSolidQuad(values, rect.x as number, rect.y as number, rect.width as number, rect.height as number, selectionColor, this.atlasCanvas.width, this.atlasCanvas.height, canvasWidth, canvasHeight)
    }

    if (this.compositionRect !== null && this.compositionText.length > 0) {
      pushSolidQuad(
        values,
        this.compositionRect.x as number,
        this.compositionRect.y as number,
        this.measureTextAdvance(this.compositionText) + 2,
        this.compositionRect.height as number,
        compositionColor,
        this.atlasCanvas.width,
        this.atlasCanvas.height,
        canvasWidth,
        canvasHeight,
      )
      this.pushText(values, this.compositionText, this.compositionRect.x as number, this.compositionRect.y as number, this.cssColorToRgba('--neco-editor-fg'), canvasWidth, canvasHeight)
    }

    if (this.caretRect !== null) {
      pushSolidQuad(values, this.caretRect.x as number, this.caretRect.y as number, this.caretRect.width as number, this.caretRect.height as number, cursorColor, this.atlasCanvas.width, this.atlasCanvas.height, canvasWidth, canvasHeight)
    }

    return new Float32Array(values)
  }

  private pushTokenizedLine(
    values: number[],
    line: RenderLine,
    x: number,
    y: number,
    canvasWidth: number,
    canvasHeight: number,
  ): void {
    if (line.tokens.length === 0) {
      this.pushText(values, line.text, x, y, this.cssColorToRgba('--neco-token-plain'), canvasWidth, canvasHeight)
      return
    }

    let cursor = 0
    let currentX = x
    for (const token of line.tokens) {
      if (token.start > cursor) {
        const text = line.text.substring(cursor, token.start)
        this.pushText(values, text, currentX, y, this.cssColorToRgba('--neco-token-plain'), canvasWidth, canvasHeight)
        currentX += this.measureTextAdvance(text)
      }
      const text = line.text.substring(token.start, token.end)
      this.pushText(values, text, currentX, y, this.tokenColor(token.kind), canvasWidth, canvasHeight)
      currentX += this.measureTextAdvance(text)
      cursor = token.end
    }
    if (cursor < line.text.length) {
      const text = line.text.substring(cursor)
      this.pushText(values, text, currentX, y, this.cssColorToRgba('--neco-token-plain'), canvasWidth, canvasHeight)
    }
  }

  private pushText(
    values: number[],
    text: string,
    x: number,
    y: number,
    color: Rgba,
    canvasWidth: number,
    canvasHeight: number,
  ): void {
    let currentX = x
    for (const ch of text) {
      if (ch === '\t') {
        currentX += this.metrics.charWidth * this.metrics.tabSize
        continue
      }
      const glyph = this.ensureGlyph(ch)
      pushGlyphQuad(values, currentX, y, glyph, color, this.atlasCanvas.width, this.atlasCanvas.height, canvasWidth, canvasHeight)
      currentX += glyph.advance
    }
  }

  private ensureTextGlyphs(text: string): void {
    for (const ch of text) {
      if (ch !== '\t') this.ensureGlyph(ch)
    }
  }

  private ensureGlyph(ch: string): Glyph {
    const existing = this.glyphs.get(ch)
    if (existing !== undefined) return existing

    this.resetAtlasStyle()
    const measured = this.atlasCtx.measureText(ch)
    const glyphWidth = Math.max(1, Math.ceil(measured.width))
    const glyphHeight = Math.max(1, Math.ceil(this.metrics.lineHeight * this.atlasScale))
    const paddedWidth = glyphWidth + 2
    const paddedHeight = glyphHeight + 2

    if (this.atlasX + paddedWidth >= this.atlasCanvas.width) {
      this.atlasX = 1
      this.atlasY += this.atlasRowHeight + 1
      this.atlasRowHeight = 0
    }
    if (this.atlasY + paddedHeight >= this.atlasCanvas.height) {
      this.growAtlas()
    }

    const x = this.atlasX
    const y = this.atlasY
    this.atlasCtx.fillText(ch, x + 1, y + 1)
    const glyph = {
      x,
      y,
      width: paddedWidth,
      height: paddedHeight,
      cssWidth: paddedWidth / this.atlasScale,
      cssHeight: paddedHeight / this.atlasScale,
      advance: this.glyphAdvance(ch),
    }
    this.glyphs.set(ch, glyph)
    this.atlasX += paddedWidth + 1
    this.atlasRowHeight = Math.max(this.atlasRowHeight, paddedHeight)
    return glyph
  }

  private growAtlas(): void {
    const old = this.atlasCanvas
    const next = document.createElement('canvas')
    next.width = old.width * 2
    next.height = old.height * 2
    const ctx = next.getContext('2d')
    if (ctx === null) throw new Error('2D canvas is required to grow the WebGPU glyph atlas')
    ctx.drawImage(old, 0, 0)
    this.atlasCanvas = next
    this.atlasCtx = ctx
    this.resetAtlasStyle()
    this.atlasTexture?.destroy?.()
    this.atlasTexture = null
    this.bindGroup = null
  }

  private updateAtlasScale(): void {
    const nextScale = Math.max(1, window.devicePixelRatio || 1)
    if (nextScale === this.atlasScale) return
    this.atlasScale = nextScale
    this.resetGlyphAtlas()
  }

  private resetGlyphAtlas(): void {
    this.glyphs.clear()
    this.atlasX = 1
    this.atlasY = 1
    this.atlasRowHeight = 0
    this.atlasCtx.clearRect(0, 0, this.atlasCanvas.width, this.atlasCanvas.height)
    this.resetAtlasStyle()
    this.atlasTexture?.destroy?.()
    this.atlasTexture = null
    this.bindGroup = null
  }

  private uploadAtlas(): void {
    const device = this.device
    const pipeline = this.pipeline
    const sampler = this.sampler
    if (device === null || pipeline === null || sampler === null) return

    if (this.atlasTexture === null) {
      this.atlasTexture = device.createTexture({
        size: { width: this.atlasCanvas.width, height: this.atlasCanvas.height },
        format: GLYPH_ATLAS_FORMAT,
        usage: GPU_TEXTURE_USAGE_COPY_DST_TEXTURE_BINDING_RENDER_ATTACHMENT,
      })
      this.bindGroup = device.createBindGroup({
        layout: pipeline.getBindGroupLayout(0),
        entries: [
          { binding: 0, resource: sampler },
          { binding: 1, resource: this.atlasTexture.createView() },
        ],
      })
    }
    device.queue.copyExternalImageToTexture(
      { source: this.atlasCanvas },
      { texture: this.atlasTexture },
      { width: this.atlasCanvas.width, height: this.atlasCanvas.height },
    )
  }

  private ensureVertexBuffer(device: GpuDevice, byteLength: number): void {
    if (this.vertexBuffer !== null && this.vertexBufferSize >= byteLength) return
    this.vertexBuffer?.destroy?.()
    this.vertexBufferSize = Math.max(byteLength, 4096)
    this.vertexBuffer = device.createBuffer({
      size: this.vertexBufferSize,
      usage: GPU_BUFFER_USAGE_VERTEX_COPY_DST,
    })
  }

  private resizeCanvas(): void {
    const dpr = window.devicePixelRatio || 1
    const width = Math.max(1, Math.ceil(this.contentEl.clientWidth * dpr))
    const height = Math.max(1, Math.ceil(Math.max(this.contentEl.scrollHeight, this.contentEl.clientHeight) * dpr))
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width
      this.canvas.height = height
      this.canvas.style.width = `${Math.ceil(width / dpr)}px`
      this.canvas.style.height = `${Math.ceil(height / dpr)}px`
      if (this.context !== null && this.device !== null && this.format.length > 0) {
        this.context.configure({
          device: this.device,
          format: this.format,
          alphaMode: 'premultiplied',
        })
      }
    }
  }

  private measureTextAdvance(text: string): number {
    let width = 0
    for (const ch of text) {
      width += ch === '\t' ? this.metrics.charWidth * this.metrics.tabSize : this.glyphAdvance(ch)
    }
    return width
  }

  private glyphAdvance(ch: string): number {
    if (this.metrics.monospaceGrid) {
      return isWideChar(ch) ? this.metrics.charWidth * 2 : this.metrics.charWidth
    }
    return isWideChar(ch) ? this.metrics.cjkCharWidth : this.metrics.charWidth
  }

  private tokenColor(kind: string): Rgba {
    return this.cssColorToRgba(tokenColorVar(kind))
  }

  private cssColorToRgba(name: string): Rgba {
    const style = getComputedStyle(this.container)
    const value = style.getPropertyValue(name).trim()
    return parseColor(resolveCssVarFallback(value.length > 0 ? value : '#d4d4d4'))
  }

  private resetAtlasStyle(): void {
    this.atlasCtx.fillStyle = '#ffffff'
    this.atlasCtx.fillRect(0, 0, 1, 1)
    this.atlasCtx.font = `${14 * this.atlasScale}px ${editorFontFamily()}`
    this.atlasCtx.textBaseline = 'top'
    this.atlasCtx.fillStyle = '#ffffff'
  }
}

function webGpu(): GpuApi | null {
  const nav = navigator as Navigator & { gpu?: GpuApi }
  return nav.gpu ?? null
}

function parseTranslateY(value: string): number {
  const match = /translateY\(([-0-9.]+)px\)/.exec(value)
  return match === null ? 0 : Number(match[1])
}

function editorFontFamily(): string {
  return "'Menlo', 'Consolas', 'DejaVu Sans Mono', 'Courier New', monospace"
}

function tokenColorVar(kind: string): string {
  switch (kind) {
    case 'keyword': return '--neco-token-keyword'
    case 'type': return '--neco-token-type'
    case 'function': return '--neco-token-function'
    case 'string': return '--neco-token-string'
    case 'number': return '--neco-token-number'
    case 'comment': return '--neco-token-comment'
    case 'operator': return '--neco-token-operator'
    case 'punctuation': return '--neco-token-punctuation'
    case 'variable': return '--neco-token-variable'
    case 'constant': return '--neco-token-constant'
    case 'tag': return '--neco-token-tag'
    case 'attribute': return '--neco-token-attribute'
    case 'escape': return '--neco-token-escape'
    default: return '--neco-token-plain'
  }
}

function parseColor(value: string): Rgba {
  const canvas = parseColorCanvas()
  canvas.fillStyle = value
  const computed = canvas.fillStyle
  if (computed.startsWith('#')) return parseHexColor(computed)
  const match = /rgba?\(([^)]+)\)/.exec(computed)
  if (match === null) return [0.83, 0.83, 0.83, 1]
  const parts = match[1]?.split(',').map((part) => part.trim()) ?? []
  const r = Number(parts[0] ?? 212) / 255
  const g = Number(parts[1] ?? 212) / 255
  const b = Number(parts[2] ?? 212) / 255
  const a = Number(parts[3] ?? 1)
  return [r, g, b, a]
}

function resolveCssVarFallback(value: string): string {
  const match = /^var\([^,]+,\s*(.+)\)$/.exec(value)
  return match?.[1]?.trim() ?? value
}

let colorCanvas: CanvasRenderingContext2D | null = null

function parseColorCanvas(): CanvasRenderingContext2D {
  if (colorCanvas !== null) return colorCanvas
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')
  if (ctx === null) throw new Error('2D canvas is required to parse CSS colors')
  colorCanvas = ctx
  return ctx
}

function parseHexColor(value: string): Rgba {
  const hex = value.slice(1)
  const full = hex.length === 3
    ? hex.split('').map((ch) => ch + ch).join('')
    : hex
  const r = Number.parseInt(full.slice(0, 2), 16) / 255
  const g = Number.parseInt(full.slice(2, 4), 16) / 255
  const b = Number.parseInt(full.slice(4, 6), 16) / 255
  return [r, g, b, 1]
}

function isWideChar(ch: string): boolean {
  const code = ch.codePointAt(0) ?? 0
  return (
    (code >= 0x1100 && code <= 0x11ff) ||
    (code >= 0x2e80 && code <= 0xa4cf) ||
    (code >= 0xac00 && code <= 0xd7a3) ||
    (code >= 0xf900 && code <= 0xfaff) ||
    (code >= 0xff01 && code <= 0xff60) ||
    (code >= 0xffe0 && code <= 0xffe6)
  )
}

function pushGlyphQuad(
  values: number[],
  x: number,
  y: number,
  glyph: Glyph,
  color: Rgba,
  atlasWidth: number,
  atlasHeight: number,
  canvasWidth: number,
  canvasHeight: number,
): void {
  const u0 = glyph.x / atlasWidth
  const v0 = glyph.y / atlasHeight
  const u1 = (glyph.x + glyph.width) / atlasWidth
  const v1 = (glyph.y + glyph.height) / atlasHeight
  pushTexturedQuad(values, x, y, glyph.cssWidth, glyph.cssHeight, u0, v0, u1, v1, color, canvasWidth, canvasHeight)
}

function pushSolidQuad(
  values: number[],
  x: number,
  y: number,
  width: number,
  height: number,
  color: Rgba,
  atlasWidth: number,
  atlasHeight: number,
  canvasWidth: number,
  canvasHeight: number,
): void {
  pushTexturedQuad(values, x, y, width, height, 0, 0, 1 / atlasWidth, 1 / atlasHeight, color, canvasWidth, canvasHeight)
}

function pushTexturedQuad(
  values: number[],
  x: number,
  y: number,
  width: number,
  height: number,
  u0: number,
  v0: number,
  u1: number,
  v1: number,
  color: Rgba,
  canvasWidth: number,
  canvasHeight: number,
): void {
  const x0 = (x / canvasWidth) * 2 - 1
  const x1 = ((x + width) / canvasWidth) * 2 - 1
  const y0 = 1 - (y / canvasHeight) * 2
  const y1 = 1 - ((y + height) / canvasHeight) * 2
  pushVertex(values, x0, y0, u0, v0, color)
  pushVertex(values, x1, y0, u1, v0, color)
  pushVertex(values, x0, y1, u0, v1, color)
  pushVertex(values, x0, y1, u0, v1, color)
  pushVertex(values, x1, y0, u1, v0, color)
  pushVertex(values, x1, y1, u1, v1, color)
}

function pushVertex(values: number[], x: number, y: number, u: number, v: number, color: Rgba): void {
  values.push(x, y, u, v, color[0], color[1], color[2], color[3])
}
