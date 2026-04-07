# neco-textview

[英語版](README.md)

エディタのカーソル処理に必要なテキスト基本型。バイトオフセット/行列変換、選択範囲、UTF-16 オフセットマッピングを扱います。

## 基本型

`neco-textview` はソーステキストから `LineIndex` を構築します。このインデックスを使って、バイトオフセットを `(行, 列)` の `Position` に変換したり、行のバイト範囲を取得したり、LSP 互換のために UTF-16 コードユニットオフセットとの相互変換を行います。

`Selection` はアンカーとヘッドを独立して保持するため、後ろ方向の選択もそのまま扱えます。`TextRange` はバリデーション付きの `[start, end)` ペアで、API 全体で共通して使います。

## 使い方

### バイトオフセット → 行/列

```rust
use neco_textview::{LineIndex, Position};

let text = "abc\ndef\nghi";
let idx = LineIndex::new(text);

let pos = idx.offset_to_position(text, 4).unwrap();
assert_eq!(pos, Position::new(1, 0));

let back = idx.position_to_offset(text, pos).unwrap();
assert_eq!(back, 4);
```

### UTF-16 オフセット変換

```rust
use neco_textview::Utf16Mapping;

// "aあb": 'あ' は 3 バイトだが UTF-16 では 1 コードユニット
let text = "aあb";
let m = Utf16Mapping::new(text);

assert_eq!(m.byte_to_utf16(4).unwrap(), 2); // 'b' はバイト 4 → UTF-16 オフセット 2
assert_eq!(m.utf16_to_byte(2).unwrap(), 4);
```

### Selection

```rust
use neco_textview::Selection;

let sel = Selection::new(8, 2); // 後ろ方向の選択
assert!(!sel.is_forward());
let r = sel.range();
assert_eq!(r.start(), 2);
assert_eq!(r.end(), 8);
```

## API

| 項目 | 説明 |
|------|-------------|
| `Position` | `u32` 座標の `(行, 列)` ペア |
| `TextRange` | バリデーション付き `[start, end)` バイト範囲 |
| `TextRange::new(start, end)` | `start > end` のとき `Err` を返す |
| `TextRange::empty(offset)` | 1 つのオフセットを起点とした長さ 0 の範囲 |
| `Selection` | アンカー/ヘッドペア。方向を保持する |
| `Selection::cursor(offset)` | 1 つのオフセットに折り畳まれた選択 |
| `Selection::range()` | 方向に関係なく正規化した `TextRange` を返す |
| `LineIndex` | テキストバッファ用の行開始テーブル |
| `LineIndex::offset_to_position` | バイトオフセット → `Position`。UTF-8 境界を検証 |
| `LineIndex::position_to_offset` | `Position` → バイトオフセット |
| `LineIndex::line_range(line)` | 末尾の `\n` を除いた行のバイト範囲 |
| `LineIndex::line_range_with_newline(line)` | 末尾の `\n` を含む行のバイト範囲 |
| `LineIndex::line_of_offset(offset)` | バイトオフセットが属する行番号 |
| `Utf16Mapping` | アンカーベースのバイト ↔ UTF-16 コードユニット変換器 |
| `Utf16Mapping::byte_to_utf16` | 文字境界でないオフセットは `Err` を返す |
| `Utf16Mapping::utf16_to_byte` | サロゲートペアの中間オフセットは `Err` を返す |
| `TextViewError` | 不正な範囲、範囲外オフセット、UTF-8/UTF-16 境界違反 |

## ライセンス

MIT
