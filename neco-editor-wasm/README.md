# neco-editor-wasm

[日本語](README-ja.md)

wasm-bindgen bindings for `neco-editor`. Exposes a single opaque class, `EditorHandle`, that bundles the editor buffer, viewport geometry, syntax highlighting, and search into one WASM export.

## Architecture

Two-layer design:

- **Layer 1**: pure-Rust internal functions. Fully testable without WASM.
- **Layer 2**: `#[wasm_bindgen]` wrappers that marshal types between Rust and JavaScript.

The TypeScript package [`neco-editor-ts`](../neco-editor-ts) wraps `EditorHandle` in a type-safe API. Most consumers should use that instead of calling `EditorHandle` directly.

## EditorHandle API

### Construction

```js
const handle = new EditorHandle(text, language, lineHeight, charWidth, cjkCharWidth, tabWidth)
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `text` | `string` | Initial buffer content |
| `language` | `string` | Language identifier for syntax highlighting (e.g. `"rust"`, `"typescript"`) |
| `lineHeight` | `number` | Line height in pixels |
| `charWidth` | `number` | Character width in pixels (monospace) |
| `cjkCharWidth` | `number` | CJK character width in pixels |
| `tabWidth` | `number` | Tab stop width in characters |

### Editing

| Method | Returns | Description |
|--------|---------|-------------|
| `applyEdit(start, end, newText, label)` | `RangeChange[]` | Replace `[start, end)` with `newText` and record to history |
| `undo()` | `RangeChange[] \| null` | Undo the last edit |
| `redo()` | `RangeChange[] \| null` | Redo the next edit |

### Display

| Method | Returns | Description |
|--------|---------|-------------|
| `getVisibleLines(scrollTop, height)` | `RenderLine[]` | Lines visible in the given pixel range |
| `getCaretRect(offset)` | `Rect` | Pixel rectangle for the caret at `offset` |
| `getSelectionRects(anchor, head)` | `Rect[]` | Pixel rectangles covering the selection |
| `hitTest(x, y, scrollTop)` | `number` | Byte offset closest to pixel coordinate `(x, y)` |
| `tokenizeLine(line)` | `TokenSpan[]` | Tokenize a single line string |
| `getGutterWidth()` | `number` | Width of the line-number gutter in pixels |

### Search

| Method | Returns | Description |
|--------|---------|-------------|
| `search(pattern, isRegex, caseSensitive, wholeWord)` | `SearchMatchInfo[]` | Run a search and cache the results |
| `getSearchMatches()` | `SearchMatchInfo[]` | Return the cached results from the last search |

### State

| Method | Returns | Description |
|--------|---------|-------------|
| `getText()` | `string` | Current buffer text |
| `isDirty()` | `boolean` | Whether the buffer has unsaved edits |
| `markClean()` | `void` | Reset the dirty flag |
| `setReadOnly(value)` | `void` | Toggle read-only mode |
| `isReadOnly()` | `boolean` | Current read-only flag |
| `detectIndent(sampleLines)` | `IndentInfo` | Detect indentation style from the first `sampleLines` lines |

### Viewport

| Method | Returns | Description |
|--------|---------|-------------|
| `updateMetrics(lineHeight, charWidth, cjkCharWidth, tabWidth)` | `void` | Recalculate all geometry after a font or size change |
| `scrollToReveal(offset, scrollTop, containerHeight)` | `number \| null` | Return a new `scrollTop` that reveals `offset`, or `null` if already visible |

### Language features

| Method | Returns | Description |
|--------|---------|-------------|
| `autoIndent(offset)` | `string` | Leading whitespace of the line containing `offset` |
| `autoCloseBracket(charCode)` | `number \| null` | Closing bracket/quote char code for an opening one, or `null` |
| `adjustPasteIndent(text, offset)` | `string` | Adjust indentation of pasted text |
| `findMatchingBracket(offset)` | `BracketPair \| null` | Find the bracket matching the one at `offset` |

### Lifecycle

```js
handle.free()  // explicitly release WASM memory
```

## Building

Requires [wasm-pack](https://rustwasm.github.io/wasm-pack/).

```sh
wasm-pack build --target web --out-dir pkg
```

To target Node.js:

```sh
wasm-pack build --target nodejs --out-dir pkg
```

The generated `pkg/` directory is what `neco-editor-ts` depends on via the `neco-editor-wasm` package reference.

## License

MIT
