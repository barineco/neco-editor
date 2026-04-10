# neco-editor-wasm

[English](README.md)

`neco-editor` の wasm-bindgen バインディング。エディタバッファ・ビューポートジオメトリ・シンタックスハイライト・検索を1つにまとめた `EditorHandle` クラスを WASM エクスポートとして公開します。

## アーキテクチャ

2層構成：

- **Layer 1**: 純粋 Rust の内部関数。WASM なしで完全にテスト可能。
- **Layer 2**: Rust と JavaScript の間で型を変換する `#[wasm_bindgen]` ラッパー。

TypeScript パッケージ [`neco-editor-ts`](../neco-editor-ts) が `EditorHandle` を型安全な API でラップしています。通常はそちらを利用してください。

## EditorHandle API

### コンストラクタ

```js
const handle = new EditorHandle(text, language, lineHeight, charWidth, cjkCharWidth, tabWidth)
```

| パラメータ | 型 | 説明 |
|-----------|------|------|
| `text` | `string` | バッファの初期内容 |
| `language` | `string` | シンタックスハイライト用の言語識別子（例: `"rust"`, `"typescript"`） |
| `lineHeight` | `number` | 行の高さ（ピクセル） |
| `charWidth` | `number` | 文字幅（ピクセル、等幅前提） |
| `cjkCharWidth` | `number` | CJK 文字幅（ピクセル） |
| `tabWidth` | `number` | タブ幅（文字数） |

### 編集

| メソッド | 戻り値 | 説明 |
|--------|---------|------|
| `applyEdit(start, end, newText, label)` | `RangeChange[]` | `[start, end)` を `newText` に置換して履歴に記録する |
| `undo()` | `RangeChange[] \| null` | 直前の編集を元に戻す |
| `redo()` | `RangeChange[] \| null` | 次の編集をやり直す |

### 表示

| メソッド | 戻り値 | 説明 |
|--------|---------|------|
| `getVisibleLines(scrollTop, height)` | `RenderLine[]` | 指定ピクセル範囲に表示される行一覧 |
| `getCaretRect(offset)` | `Rect` | `offset` のキャレット位置を示すピクセル矩形 |
| `getSelectionRects(anchor, head)` | `Rect[]` | 選択範囲を覆うピクセル矩形一覧 |
| `hitTest(x, y, scrollTop)` | `number` | ピクセル座標 `(x, y)` に最も近いバイトオフセット |
| `tokenizeLine(line)` | `TokenSpan[]` | 1行のテキストをトークン列に変換する |
| `getGutterWidth()` | `number` | 行番号ガターの幅（ピクセル） |

### 検索

| メソッド | 戻り値 | 説明 |
|--------|---------|------|
| `search(pattern, isRegex, caseSensitive, wholeWord)` | `SearchMatchInfo[]` | 検索を実行して結果をキャッシュする |
| `getSearchMatches()` | `SearchMatchInfo[]` | 直前の検索のキャッシュ結果を返す |

### 状態

| メソッド | 戻り値 | 説明 |
|--------|---------|------|
| `getText()` | `string` | 現在のバッファテキスト |
| `isDirty()` | `boolean` | 未保存の編集があるか |
| `markClean()` | `void` | ダーティフラグをリセットする |
| `setReadOnly(value)` | `void` | 読み取り専用モードを切り替える |
| `isReadOnly()` | `boolean` | 現在の読み取り専用フラグ |
| `detectIndent(sampleLines)` | `IndentInfo` | 先頭 `sampleLines` 行からインデントスタイルを検出する |

### ビューポート

| メソッド | 戻り値 | 説明 |
|--------|---------|------|
| `updateMetrics(lineHeight, charWidth, cjkCharWidth, tabWidth)` | `void` | フォントやサイズ変更後にジオメトリを再計算する |
| `scrollToReveal(offset, scrollTop, containerHeight)` | `number \| null` | `offset` が表示されるよう新しい `scrollTop` を返す。すでに表示中なら `null` |

### 言語機能

| メソッド | 戻り値 | 説明 |
|--------|---------|------|
| `autoIndent(offset)` | `string` | `offset` を含む行の先頭空白を返す |
| `autoCloseBracket(charCode)` | `number \| null` | 開き括弧・引用符に対応する閉じ文字のコードポイント、なければ `null` |
| `adjustPasteIndent(text, offset)` | `string` | ペーストテキストのインデントを調整する |
| `findMatchingBracket(offset)` | `BracketPair \| null` | `offset` にある括弧と対応する括弧を探す |

### ライフサイクル

```js
handle.free()  // WASM メモリを明示的に解放する
```

## ビルド

[wasm-pack](https://rustwasm.github.io/wasm-pack/) が必要です。

```sh
wasm-pack build --target web --out-dir pkg
```

Node.js をターゲットにする場合：

```sh
wasm-pack build --target nodejs --out-dir pkg
```

生成された `pkg/` ディレクトリが `neco-editor-ts` の `neco-editor-wasm` 参照先になります。

## ライセンス

MIT
