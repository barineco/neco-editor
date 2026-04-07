# neco-diffcore

[英語版](README.md)

Myers O(ND) アルゴリズムによる行レベル・文字レベルの差分計算。ハンクグループ化、サイドバイサイドレイアウト、パッチ生成に対応します。

## 差分の挙動

`neco-diffcore` は Myers diff を行単位で実行して `DiffResult` を返します。そこから次の操作が使えます。

- 変更行をコンテキスト行数を指定して `DiffHunk` にまとめる。
- `diff_intra_line` で 1 行ペア内の文字レベル変更を計算する。
- `to_side_by_side` で結果を 2 カラムレイアウトに変換する。
- `diff_to_patches` で差分を `neco-textpatch` パッチに変換して旧テキストに適用する。

すべての位置はバイト範囲で表されるため、呼び出し側は再スキャンなしにそのままレンダリングに使えます。

## 使い方

### 行差分とハンク

```rust
use neco_diffcore::{diff, DiffOp};

let old = "a\nb\nc\nd\n";
let new = "a\nB\nc\nD\n";
let result = diff(old, new);

let hunks = result.to_hunks(1);
assert_eq!(hunks.len(), 2);
assert!(hunks[0].lines().iter().any(|l| l.op() != DiffOp::Equal));
```

### 行内差分

```rust
use neco_diffcore::{diff_intra_line, DiffOp};

let intra = diff_intra_line("hello world", "hello there");
assert!(intra.ranges().iter().any(|r| r.op() == DiffOp::Delete));
assert!(intra.ranges().iter().any(|r| r.op() == DiffOp::Insert));
```

### パッチへの往復変換

```rust
use neco_diffcore::{diff, diff_to_patches};
use neco_textpatch::apply_patches;

let old = "line1\nline2\nline3\n";
let new = "line1\nchanged\nline3\n";
let result = diff(old, new);
let patches = diff_to_patches(new, &result).unwrap();
let applied = apply_patches(old, &patches).unwrap();
assert_eq!(applied, new);
```

## API

| 項目 | 説明 |
|------|-------------|
| `DiffOp` | `Equal`、`Insert`、`Delete` の 3 種 |
| `ByteRange` | 差分の片方の側における `[start, end)` バイトスパン |
| `DiffLine` | op・行番号・両側のバイト範囲を持つ差分 1 行 |
| `DiffResult` | `diff` が返す `DiffLine` のフラットなリスト |
| `DiffResult::to_hunks(context_lines)` | 変更をコンテキスト付きの `DiffHunk` にまとめる。近接するグループは統合される |
| `DiffHunk` | 旧/新の行座標を持つ変更行の連続グループ |
| `IntraLineRange` | 1 行内の変更バイトスパン。`DiffOp` を持つ |
| `IntraLineDiff` | 1 行ペアの文字レベル差分結果 |
| `SideLine` | 行番号・op・バイト範囲を持つサイドバイサイドの片側 1 行 |
| `SideBySideLine` | 2 カラムレンダリング用の左右ペア。片側は `None` になることもある |
| `diff(old, new)` | 行レベルの Myers diff を実行して `DiffResult` を返す |
| `diff_intra_line(old_line, new_line)` | 1 行ペアに対して文字レベルの Myers diff を実行する |
| `to_side_by_side(result)` | `DiffResult` を左右ペアの行列に変換する |
| `diff_to_patches(new, result)` | `DiffResult` を `neco-textpatch` のパッチ列に変換する |

## ライセンス

MIT
