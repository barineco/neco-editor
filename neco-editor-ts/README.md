# neco-editor-ts

[日本語](README-ja.md)

Host-agnostic TypeScript API for `neco-editor-wasm`. Provides two classes:

- **`EditorSession`**: thin type-safe wrapper around the WASM `EditorHandle`. Manages one editor buffer with integrated viewport geometry, syntax highlighting, and search. No DOM dependency.
- **`EditorView`**: full editor UI mounted to a DOM element. Handles rendering, keyboard/mouse input, virtual scroll, and clipboard.

## Installation

Build `neco-editor-wasm` first:

```sh
cd ../neco-editor-wasm
wasm-pack build --target web --out-dir pkg
```

Then install dependencies for this package:

```sh
npm install
```

## Basic usage

### EditorView (recommended)

```ts
import { EditorView } from 'neco-editor'
import 'neco-editor/src/styles.css'

const view = new EditorView({
  container: document.getElementById('editor')!,
  text: 'hello world',
  language: 'rust',
  lineNumbers: true,
  tabSize: 4,
})

// Listen for content changes
view.onDidChangeContent((changes) => {
  console.log('changed', view.getText())
})

// Clean up
view.dispose()
```

### EditorSession (compute only)

Use `EditorSession` when you want to drive rendering yourself or run outside a browser.

```ts
import { EditorSession } from 'neco-editor'

const session = new EditorSession(
  'hello world', // text
  'rust',        // language
  20,            // lineHeight (px)
  8,             // charWidth (px)
  4,             // tabWidth
)

const changes = session.applyEdit(6, 11, 'neco', 'type')
// changes[0] → { start: 6, oldEnd: 11, newEnd: 10 }

const lines = session.getVisibleLines(0, 400)
// lines[0] → { lineNumber: 1, text: 'hello neco', tokens: [...] }

session.free()
```

## EditorView options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `container` | `HTMLElement` |:| Mount target (required) |
| `text` | `string` | `''` | Initial content |
| `language` | `string` | `'plain'` | Language for syntax highlighting |
| `session` | `EditorSession` |:| Inject an existing session instead of creating one |
| `readOnly` | `boolean` | `false` | Disable editing |
| `tabSize` | `number` | `4` | Tab stop width in spaces |
| `lineNumbers` | `boolean` | `true` | Show line number gutter |
| `padding` | `{ top?, bottom? }` |:| Content padding in pixels |

## EditorView events

```ts
view.onDidChangeContent((changes) => { /* RangeChange[] | null */ })
view.onDidChangeCursorPosition((offset) => { /* byte offset */ })
view.onDidChangeSelection(({ anchor, head }) => { /* byte offsets */ })
view.onDidScroll((scrollTop) => { /* px */ })
view.onDidFocus(() => { })
view.onDidBlur(() => { })
```

All event subscriptions return a `Disposable` with a `.dispose()` method.

## EditorView methods

| Method | Description |
|--------|-------------|
| `getText()` | Current buffer text |
| `applyEdit(start, end, newText, label?)` | Apply a single edit |
| `undo()` / `redo()` | Undo/redo |
| `setCursor(offset)` | Move cursor |
| `setSelection(anchor, head)` | Set selection |
| `getCursor()` / `getSelection()` | Read cursor/selection |
| `revealOffset(offset)` | Scroll to make `offset` visible |
| `revealLine(line)` | Scroll to make line number visible |
| `search(pattern, options?)` | Run a search |
| `getSearchMatches()` | Return cached search results |
| `isDirty()` / `markClean()` | Track save state |
| `saveViewState()` / `restoreViewState(state)` | Persist scroll + cursor |
| `updateOptions(opts)` | Update `readOnly`, `tabSize`, `lineNumbers` at runtime |
| `focus()` / `hasFocus()` | Focus management |
| `layout()` | Force layout recalculation after container resize |
| `getSession()` | Access the underlying `EditorSession` |
| `dispose()` | Tear down the view and release WASM memory |

## CSS theme customization

Import `src/styles.css` and override the custom properties on `.neco-editor`:

```css
.neco-editor {
  --neco-editor-bg: #0d1117;
  --neco-editor-fg: #e6edf3;
  --neco-gutter-bg: #0d1117;
  --neco-gutter-fg: #6e7681;
  --neco-selection-bg: rgba(56, 139, 253, 0.3);
  --neco-cursor-color: #58a6ff;

  /* Syntax tokens */
  --neco-token-keyword: #ff7b72;
  --neco-token-string: #a5d6ff;
  --neco-token-number: #79c0ff;
  --neco-token-comment: #8b949e;
  --neco-token-function: #d2a8ff;
  --neco-token-type: #ffa657;
  --neco-token-plain: #e6edf3;
}
```

Alternatively, set the upstream variables that the tokens fall back to:

| Variable | Fallback used by |
|----------|-----------------|
| `--editor-bg` | `--neco-editor-bg` |
| `--editor-fg` | `--neco-editor-fg` |
| `--editor-gutter` | `--neco-gutter-bg` |
| `--editor-gutter-fg` | `--neco-gutter-fg` |
| `--editor-selection` | `--neco-selection-bg` |
| `--editor-cursor` | `--neco-cursor-color` |
| `--syntax-keyword` | `--neco-token-keyword`, `--neco-token-type`, `--neco-token-tag` |
| `--syntax-string` | `--neco-token-string` |
| `--syntax-number` | `--neco-token-number`, `--neco-token-constant`, `--neco-token-escape` |
| `--syntax-comment` | `--neco-token-comment` |
| `--syntax-accent` | `--neco-token-function`, `--neco-token-attribute` |
| `--syntax-plain` | `--neco-token-plain`, `--neco-token-operator`, `--neco-token-punctuation`, `--neco-token-variable` |

## License

MIT
