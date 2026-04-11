import test from 'node:test'
import assert from 'node:assert/strict'

class FakeStyle {
  [key: string]: string | ((name: string, value: string) => void) | ((name: string) => void)

  setProperty(name: string, value: string): void {
    this[name] = value
  }

  removeProperty(name: string): void {
    delete this[name]
  }
}

class FakeClassList {
  private classes = new Set<string>()

  add(...tokens: string[]): void {
    for (const token of tokens) this.classes.add(token)
  }

  remove(...tokens: string[]): void {
    for (const token of tokens) this.classes.delete(token)
  }

  contains(token: string): boolean {
    return this.classes.has(token)
  }

  toString(): string {
    return Array.from(this.classes).join(' ')
  }
}

class FakeElement {
  readonly tagName: string
  readonly style = new FakeStyle()
  readonly classList = new FakeClassList()
  readonly children: FakeElement[] = []
  readonly listeners = new Map<string, Set<(...args: unknown[]) => void>>()
  parentElement: FakeElement | null = null
  ownerDocument!: FakeDocument
  textContent = ''
  className = ''
  scrollTop = 0
  clientWidth = 800
  clientHeight = 600
  value = ''

  constructor(tagName: string) {
    this.tagName = tagName.toUpperCase()
  }

  appendChild<T extends FakeElement>(child: T): T {
    child.parentElement = this
    child.ownerDocument = this.ownerDocument
    this.children.push(child)
    return child
  }

  removeChild<T extends FakeElement>(child: T): T {
    const index = this.children.indexOf(child)
    if (index >= 0) {
      this.children.splice(index, 1)
      child.parentElement = null
    }
    return child
  }

  remove(): void {
    this.parentElement?.removeChild(this)
  }

  addEventListener(type: string, listener: (...args: unknown[]) => void): void {
    let set = this.listeners.get(type)
    if (!set) {
      set = new Set()
      this.listeners.set(type, set)
    }
    set.add(listener)
  }

  removeEventListener(type: string, listener: (...args: unknown[]) => void): void {
    this.listeners.get(type)?.delete(listener)
  }

  setAttribute(name: string, value: string): void {
    ;(this as Record<string, unknown>)[name] = value
  }

  focus(): void {
    this.ownerDocument.activeElement = this
  }

  blur(): void {
    if (this.ownerDocument.activeElement === this) {
      this.ownerDocument.activeElement = null
    }
  }

  select(): void {}

  getBoundingClientRect(): DOMRect {
    return {
      x: 0,
      y: 0,
      width: this.clientWidth,
      height: this.clientHeight,
      top: 0,
      left: 0,
      right: this.clientWidth,
      bottom: this.clientHeight,
      toJSON: () => ({}),
    } as DOMRect
  }

  contains(node: FakeElement | null): boolean {
    if (!node) return false
    if (node === this) return true
    return this.children.some((child) => child.contains(node))
  }

  getContext(kind: string): { font: string; measureText(text: string): { width: number } } | null {
    if (this.tagName !== 'CANVAS' || kind !== '2d') return null
    let font = ''
    return {
      measureText: (text: string) => {
        const match = /(\d+(?:\.\d+)?)px/.exec(font)
        const fontSize = match ? Number(match[1]) : 14
        return { width: fontSize * 0.6 * text.length }
      },
      set font(value: string) {
        font = value
      },
      get font() {
        return font
      },
    }
  }

  get offsetHeight(): number {
    return this.clientHeight
  }
}

class FakeDocument {
  readonly body: FakeElement
  activeElement: FakeElement | null = null
  private listeners = new Map<string, Set<(...args: unknown[]) => void>>()

  constructor() {
    this.body = this.createElement('body')
  }

  createElement(tagName: string): FakeElement {
    const el = new FakeElement(tagName)
    el.ownerDocument = this
    return el
  }

  addEventListener(type: string, listener: (...args: unknown[]) => void): void {
    let set = this.listeners.get(type)
    if (!set) {
      set = new Set()
      this.listeners.set(type, set)
    }
    set.add(listener)
  }

  removeEventListener(type: string, listener: (...args: unknown[]) => void): void {
    this.listeners.get(type)?.delete(listener)
  }

  execCommand(): boolean {
    return false
  }
}

class FakeResizeObserver {
  constructor(_callback: ResizeObserverCallback) {}
  observe(_target: Element): void {}
  disconnect(): void {}
}

const rafQueue: FrameRequestCallback[] = []

function installFakeDom(): { document: FakeDocument } {
  const document = new FakeDocument()
  const windowObject = {
    devicePixelRatio: 1,
    navigator: { platform: 'Linux' },
  }

  ;(globalThis as Record<string, unknown>).document = document
  ;(globalThis as Record<string, unknown>).window = windowObject
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: windowObject.navigator,
  })
  ;(globalThis as Record<string, unknown>).HTMLElement = FakeElement
  ;(globalThis as Record<string, unknown>).HTMLDivElement = FakeElement
  ;(globalThis as Record<string, unknown>).HTMLSpanElement = FakeElement
  ;(globalThis as Record<string, unknown>).HTMLTextAreaElement = FakeElement
  ;(globalThis as Record<string, unknown>).HTMLCanvasElement = FakeElement
  ;(globalThis as Record<string, unknown>).ResizeObserver = FakeResizeObserver
  ;(globalThis as Record<string, unknown>).requestAnimationFrame = (cb: FrameRequestCallback) => {
    rafQueue.push(cb)
    return rafQueue.length
  }
  ;(globalThis as Record<string, unknown>).cancelAnimationFrame = (_id: number) => {}
  ;(globalThis as Record<string, unknown>).getComputedStyle = (element: {
    style: Record<string, string | undefined>
  }) => ({
    lineHeight: element.style.lineHeight ?? 'normal',
  })

  return { document }
}

function flushAnimationFrames(): void {
  while (rafQueue.length > 0) {
    const cb = rafQueue.shift()
    cb?.(0)
  }
}

function findByClass(root: FakeElement, className: string): FakeElement[] {
  const results: FakeElement[] = []
  if (root.className.split(/\s+/).includes(className) || root.classList.contains(className)) {
    results.push(root)
  }
  for (const child of root.children) {
    results.push(...findByClass(child, className))
  }
  return results
}

let wasmReady: Promise<void> | null = null

async function ensureWasm(): Promise<void> {
  if (!wasmReady) {
    wasmReady = (async () => {
      const wasmModule = await import('neco-editor-wasm')
      const { readFile } = await import('node:fs/promises')
      const wasmBytes = await readFile(new URL('../node_modules/neco-editor-wasm/neco_editor_wasm_bg.wasm', import.meta.url))
      wasmModule.initSync(wasmBytes)
    })()
  }
  await wasmReady
}

test('EditorView applies custom font settings to metrics and host styles', async () => {
  installFakeDom()
  await ensureWasm()
  const { EditorView } = await import('../src/editor-view.ts')

  const container = document.createElement('div') as FakeElement
  document.body.appendChild(container)

  const view = new EditorView({
    container: container as unknown as HTMLElement,
    text: 'alpha\nbeta',
    language: 'plain',
    renderer: 'dom',
    fontFamily: '"Fira Code", monospace',
    fontSize: 20,
    lineHeight: 30,
  })

  flushAnimationFrames()

  assert.equal(container.style.fontFamily, '"Fira Code", monospace')
  assert.equal(container.style.fontSize, '20px')
  assert.equal((view as unknown as { lineHeight: number }).lineHeight, 30)

  view.dispose()
})

test('EditorView updates font metrics and rendered line height via updateOptions', async () => {
  installFakeDom()
  await ensureWasm()
  const { EditorView } = await import('../src/editor-view.ts')

  const container = document.createElement('div') as FakeElement
  document.body.appendChild(container)

  const view = new EditorView({
    container: container as unknown as HTMLElement,
    text: 'alpha\nbeta',
    language: 'plain',
    renderer: 'dom',
  })

  flushAnimationFrames()

  const before = (view as unknown as { charWidth: number }).charWidth
  view.updateOptions({ fontSize: 18, lineHeight: 30 })
  flushAnimationFrames()

  const after = (view as unknown as { charWidth: number }).charWidth
  assert.notEqual(after, before)
  assert.equal(container.style.fontSize, '18px')

  const lines = findByClass(container, 'neco-line')
  assert.ok(lines.length > 0)
  for (const line of lines) {
    assert.equal(line.style.height, '30px')
  }

  view.dispose()
})
