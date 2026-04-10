# neco-editor-ts

[English](README.md)

`neco-editor-wasm` をラップするホスト非依存の TypeScript API。2つのクラスを提供します。

- **`EditorSession`**: WASM `EditorHandle` の薄い型安全ラッパー。ビューポートジオメトリ・シンタックスハイライト・検索を統合した1バッファ分の計算レイヤー。DOM 依存なし。
- **`EditorView`**: DOM 要素にマウントする完全なエディタ UI。レンダリング・キーボード/マウス入力・仮想スクロール・クリップボードを処理する。

## インストール

まず `neco-editor-wasm` をビルドします。

```sh
cd ../neco-editor-wasm
wasm-pack build --target web --out-dir pkg
```

次にこのパッケージの依存関係をインストールします。

```sh
npm install
```

## 基本的な使い方

### EditorView（推奨）

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

// 内容変更を購読
view.onDidChangeContent((changes) => {
  console.log('changed', view.getText())
})

// 後片付け
view.dispose()
```

### EditorSession（計算のみ）

レンダリングを自前で行う場合や、ブラウザ外で動かす場合に使います。

```ts
import { EditorSession } from 'neco-editor'

const session = new EditorSession(
  'hello world', // テキスト
  'rust',        // 言語
  20,            // lineHeight (px)
  8,             // charWidth (px)
  14,            // cjkCharWidth (px)
  4,             // tabWidth
)

const changes = session.applyEdit(6, 11, 'neco', 'type')
// changes[0] → { start: 6, oldEnd: 11, newEnd: 10 }

const lines = session.getVisibleLines(0, 400)
// lines[0] → { lineNumber: 1, text: 'hello neco', tokens: [...] }

session.free()
```

## EditorView オプション

| オプション | 型 | デフォルト | 説明 |
|--------|------|---------|------|
| `container` | `HTMLElement` |:| マウント先要素（必須） |
| `text` | `string` | `''` | 初期テキスト |
| `language` | `string` | `'plain'` | シンタックスハイライト用の言語 |
| `session` | `EditorSession` |:| 既存セッションを注入する（`text`/`language` の代替） |
| `readOnly` | `boolean` | `false` | 編集を無効にする |
| `tabSize` | `number` | `4` | タブ幅（スペース数） |
| `monospaceGrid` | `boolean` | `false` | ASCII/CJK を厳密な 1:2 グリッドで扱う |
| `renderer` | `'webgpu' \| 'dom'` | `'webgpu'` | WebGPU renderer または DOM renderer を使う |
| `lineNumbers` | `boolean` | `true` | 行番号ガターを表示する |
| `padding` | `{ top?, bottom? }` |:| コンテンツのパディング（px） |

`renderer: 'webgpu'` は `navigator.gpu`、WebGPU adapter、WebGPU canvas context を必要とします。非対応環境では DOM renderer にフォールバックせず、明示エラーにします。比較確認や WebGPU 非対応ブラウザでは `renderer: 'dom'` を指定してください。

## EditorView イベント

```ts
view.onDidChangeContent((changes) => { /* RangeChange[] | null */ })
view.onDidChangeCursorPosition((offset) => { /* バイトオフセット */ })
view.onDidChangeSelection(({ anchor, head }) => { /* バイトオフセット */ })
view.onDidScroll((scrollTop) => { /* px */ })
view.onDidFocus(() => { })
view.onDidBlur(() => { })
```

イベント購読はすべて `.dispose()` メソッドを持つ `Disposable` を返します。

## EditorView メソッド

| メソッド | 説明 |
|--------|------|
| `getText()` | 現在のバッファテキスト |
| `applyEdit(start, end, newText, label?)` | 単一の編集を適用する |
| `undo()` / `redo()` | アンドゥ/リドゥ |
| `setCursor(offset)` | カーソルを移動する |
| `setSelection(anchor, head)` | 選択範囲を設定する |
| `getCursor()` / `getSelection()` | カーソル/選択範囲を取得する |
| `revealOffset(offset)` | `offset` が見えるようにスクロールする |
| `revealLine(line)` | 指定行番号が見えるようにスクロールする |
| `search(pattern, options?)` | 検索を実行する |
| `getSearchMatches()` | キャッシュ済みの検索結果を返す |
| `isDirty()` / `markClean()` | 保存状態を管理する |
| `saveViewState()` / `restoreViewState(state)` | スクロール位置とカーソルを保存/復元する |
| `updateOptions(opts)` | `readOnly`・`tabSize`・`monospaceGrid`・`lineNumbers` を実行時に変更する |
| `focus()` / `hasFocus()` | フォーカス管理 |
| `layout()` | コンテナリサイズ後にレイアウトを強制再計算する |
| `getSession()` | 内部の `EditorSession` にアクセスする |
| `dispose()` | ビューを破棄し WASM メモリを解放する |

## CSS テーマカスタマイズ

`src/styles.css` をインポートし、`.neco-editor` 上のカスタムプロパティを上書きします。

```css
.neco-editor {
  --neco-editor-bg: #0d1117;
  --neco-editor-fg: #e6edf3;
  --neco-gutter-bg: #0d1117;
  --neco-gutter-fg: #6e7681;
  --neco-selection-bg: rgba(56, 139, 253, 0.3);
  --neco-cursor-color: #58a6ff;

  /* シンタックストークン */
  --neco-token-keyword: #ff7b72;
  --neco-token-string: #a5d6ff;
  --neco-token-number: #79c0ff;
  --neco-token-comment: #8b949e;
  --neco-token-function: #d2a8ff;
  --neco-token-type: #ffa657;
  --neco-token-plain: #e6edf3;
}
```

または、トークンがフォールバックとして参照する上位変数を設定することもできます。

| 変数 | 参照先 |
|----------|-----------------|
| `--editor-bg` | `--neco-editor-bg` |
| `--editor-fg` | `--neco-editor-fg` |
| `--editor-gutter` | `--neco-gutter-bg` |
| `--editor-gutter-fg` | `--neco-gutter-fg` |
| `--editor-selection` | `--neco-selection-bg` |
| `--editor-cursor` | `--neco-cursor-color` |
| `--syntax-keyword` | `--neco-token-keyword`、`--neco-token-type`、`--neco-token-tag` |
| `--syntax-string` | `--neco-token-string` |
| `--syntax-number` | `--neco-token-number`、`--neco-token-constant`、`--neco-token-escape` |
| `--syntax-comment` | `--neco-token-comment` |
| `--syntax-accent` | `--neco-token-function`、`--neco-token-attribute` |
| `--syntax-plain` | `--neco-token-plain`、`--neco-token-operator`、`--neco-token-punctuation`、`--neco-token-variable` |

Codigen 形式のテーマ KDL も読み込めます。読み込んだ CSS 変数はそのまま
エディタに適用できます。

```ts
import { applyTheme, parseThemeKdl } from 'neco-editor'

const theme = parseThemeKdl(themeKdlText)
applyTheme(theme, editorContainer)
```

`parseThemeKdl` は `{ id, name, vars }` を返します。`bg`、`fg`、`syntax`、
`editor`、`terminal`、`override`、`var`、`grad` ノードを読み、Codigen の
テーマと同じ変数名を使います。

## ライセンス

MIT
