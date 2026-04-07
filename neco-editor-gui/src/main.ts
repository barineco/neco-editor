import { EditorView, type EditorViewOptions } from 'neco-editor'
import 'neco-editor/styles.css'
import initWasm from 'neco-editor-wasm'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'

// Bundled sample text shown on startup
const SAMPLE_TEXT = `// Welcome to Neco Editor
// This is a standalone Tauri harness for neco-editor.

function greet(name: string): string {
  return \`Hello, \${name}!\`
}

const message = greet('world')
console.log(message)

// Try:
// - Edit this text
// - Open a file with the toolbar button
// - Save with Cmd+S (not implemented yet, use toolbar)
// - Scroll with mouse wheel to test gutter sync
`

const LANG_MAP: Record<string, string> = {
  ts: 'typescript',
  tsx: 'typescript',
  js: 'javascript',
  jsx: 'javascript',
  rs: 'rust',
  json: 'json',
  md: 'markdown',
  html: 'html',
  css: 'css',
  toml: 'toml',
}

function detectLanguage(path: string | null): string {
  if (path === null) return 'typescript'
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  return LANG_MAP[ext] ?? 'plain'
}

interface AppState {
  view: EditorView
  currentPath: string | null
  currentLang: string
}

let state: AppState | null = null

function createEditor(text: string, language: string): EditorView {
  const container = document.getElementById('editor')
  if (container === null) throw new Error('#editor element not found')

  // Clear previous view if any
  if (state !== null) {
    state.view.dispose()
    container.innerHTML = ''
  }

  const options: EditorViewOptions = {
    container,
    text,
    language,
    lineNumbers: true,
  }

  const view = new EditorView(options)

  // Wire up events for status bar
  view.onDidChangeCursorPosition((offset: number) => {
    updateCursorPos(view, offset)
  })
  view.onDidChangeContent(() => {
    updateDirty(view)
  })

  return view
}

function updateCursorPos(view: EditorView, offset: number): void {
  const text = view.getText()
  let line = 1
  let col = 1
  for (let i = 0; i < offset; i++) {
    if (text[i] === '\n') {
      line++
      col = 1
    } else {
      col++
    }
  }
  const el = document.getElementById('cursor-pos')
  if (el !== null) el.textContent = `${line}:${col}`
}

function updateDirty(view: EditorView): void {
  const el = document.getElementById('file-dirty')
  if (el !== null) el.textContent = view.isDirty() ? '●' : ''
}

function updateFilePath(path: string | null): void {
  const el = document.getElementById('file-path')
  if (el !== null) el.textContent = path ?? 'untitled'
  const base = path?.split('/').pop() ?? 'Neco Editor'
  document.title = base
}

function updateLangIndicator(lang: string): void {
  const el = document.getElementById('lang-indicator')
  if (el !== null) el.textContent = lang
}

async function handleOpen(): Promise<void> {
  const selected = await open({
    multiple: false,
    directory: false,
  })
  if (typeof selected !== 'string') return

  const text = await invoke<string>('neco_read_file', { path: selected })
  const lang = detectLanguage(selected)
  const view = createEditor(text, lang)
  view.markClean()
  state = { view, currentPath: selected, currentLang: lang }
  updateFilePath(selected)
  updateLangIndicator(lang)
  updateCursorPos(view, view.getCursor())
  updateDirty(view)
  view.focus()
}

async function handleSave(): Promise<void> {
  if (state === null) return
  let path = state.currentPath
  if (path === null) {
    const chosen = await save({})
    if (typeof chosen !== 'string') return
    path = chosen
    state.currentPath = path
    state.currentLang = detectLanguage(path)
    updateFilePath(path)
    updateLangIndicator(state.currentLang)
  }
  const text = state.view.getText()
  await invoke('neco_write_file', { path, contents: text })
  state.view.markClean()
  updateDirty(state.view)
}

async function main(): Promise<void> {
  // Load WASM (vite resolves 'neco-editor-wasm' to the pkg's main module)
  await initWasm()

  // Initial editor with sample text
  const view = createEditor(SAMPLE_TEXT, 'typescript')
  view.markClean()
  state = { view, currentPath: null, currentLang: 'typescript' }
  updateFilePath(null)
  updateLangIndicator('typescript')
  updateCursorPos(view, view.getCursor())
  updateDirty(view)
  view.focus()

  const reportError = (label: string, err: unknown): void => {
    console.error(`[neco-editor-gui] ${label} failed:`, err)
    const el = document.getElementById('file-dirty')
    if (el !== null) el.textContent = `! ${label}: ${String(err)}`
  }

  // Toolbar buttons
  document.getElementById('open-btn')?.addEventListener('click', () => {
    handleOpen().catch((err: unknown) => reportError('open', err))
  })
  document.getElementById('save-btn')?.addEventListener('click', () => {
    handleSave().catch((err: unknown) => reportError('save', err))
  })

  // Keyboard shortcuts (Cmd/Ctrl + S for Save)
  document.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault()
      handleSave().catch((err: unknown) => reportError('save', err))
    }
    if ((e.metaKey || e.ctrlKey) && e.key === 'o') {
      e.preventDefault()
      handleOpen().catch((err: unknown) => reportError('open', err))
    }
  })
}

main().catch((err: unknown) => {
  console.error('[neco-editor-gui] fatal error:', err)
})
