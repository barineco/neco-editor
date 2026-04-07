# neco-editor

[英語版](README.md)

エディタランタイム基本型のアンブレラクレート。10 個の基盤クレートを re-export し、テキスト・行インデックス・装飾・折り返し状態・編集履歴を一括管理する `EditorBuffer` を提供します。

## 概要

`neco-editor` は以下のクレートをひとつのエントリポイントにまとめたアンブレラクレートです。`neco_editor::textview::LineIndex` のようにアクセスするか、各クレートに直接依存するかを選べます。

| クレート | 役割 |
|---------|------|
| `neco-textview` | 位置プリミティブ: `Position`、`TextRange`、`Selection`、`LineIndex`、`Utf16Mapping`、`RangeChange` |
| `neco-textpatch` | テキスト編集操作: `TextPatch`、`apply_patches`、`inverse_patches` |
| `neco-decor` | 装飾データモデル: `Decoration`、`DecorationSet` |
| `neco-diffcore` | 差分計算: `diff`、`to_hunks`、`diff_to_patches` |
| `neco-wrap` | 行折り返し: `WrapMap`、`WrapPolicy` |
| `neco-history` | 編集履歴ツリー: `EditHistory`、undo/redo |
| `neco-tree` | 汎用ツリー: `Tree`、`CursoredTree` (`EditHistory` の公開 API が露出するため re-export) |
| `neco-pathrel` | パス正規化 |
| `neco-filetree` | ファイルツリー構造 |
| `neco-watchnorm` | ファイル監視イベント正規化 |

このクレートが独自に定義する型は `EditorBuffer` だけです。テキストと `LineIndex` を所有し、編集のたびに全サブシステムを一貫した状態に保ちます。

## EditorBuffer

`EditorBuffer` は `String` とそこから構築した `LineIndex` を保持する型です。`apply_patches` を呼ぶと `TextPatch` のバッチを適用し、`LineIndex` を再構築し、下流の消費者向けに `Vec<RangeChange>` を返します。

`apply_patches_with` はさらに進んで、編集を `EditHistory` に記録し、パッチを適用し、`RangeChange` を `DecorationSet` に伝搬し、影響を受けた行を `WrapMap` で再折り返しする統合メソッドです。逆パッチの計算に元テキストが必要なため、履歴の記録はテキスト変更より先に行われます。

省略可能なサブシステム（`WrapMap`、`WrapPolicy`、`EditHistory`）はすべて `Option` で受け取る設計です。折り返しや履歴が不要な呼び出し側は `None` を渡すだけで済みます。

## 使い方

### 基本: パッチを適用して変更範囲を得る

```rust
use neco_editor::{EditorBuffer, neco_textpatch::TextPatch};

let mut buffer = EditorBuffer::new("hello world".to_string());
let patches = [TextPatch::replace(6, 11, "rust").unwrap()];

let changes = buffer.apply_patches(&patches).unwrap();

assert_eq!(buffer.text(), "hello rust");
assert_eq!(buffer.line_index().text_len(), 10);
// changes[0] が置換したスパンを記述する: start=6, old_end=11, new_end=10
```

### 統合: 履歴と装飾を同時更新する

```rust
use neco_editor::{
    EditorBuffer,
    neco_textpatch::TextPatch,
    neco_decor::{DecorationSet, Decoration},
    neco_history::EditHistory,
};

let mut buffer = EditorBuffer::new("hello world".to_string());
let mut decorations = DecorationSet::new();
decorations.add(Decoration::highlight(6, 11, 1).unwrap());
let mut history = EditHistory::new(buffer.text());

let patches = [TextPatch::replace(6, 11, "rust").unwrap()];

buffer
    .apply_patches_with(
        &patches,
        &mut decorations,
        None,        // WrapMap なし
        None,        // WrapPolicy なし
        Some(&mut history),
        Some("replace word"),
    )
    .unwrap();

assert_eq!(buffer.text(), "hello rust");
// map_through_changes により装飾の終了バイトが 11 から 10 に更新される
assert_eq!(decorations.iter().next().unwrap().1.end(), 10);
assert_eq!(history.current_label(), "replace word");
```

## API

### `EditorBuffer`

| 項目 | 説明 |
|------|------|
| `EditorBuffer::new(text)` | 所有権のある `String` から構築する。初期 `LineIndex` を同時に構築する |
| `EditorBuffer::text()` | 現在のテキストを借用する |
| `EditorBuffer::line_index()` | 現在の `LineIndex` を借用する |
| `EditorBuffer::apply_patches(patches)` | パッチを適用し `LineIndex` を再構築する。`Vec<RangeChange>` または `TextPatchError` を返す |
| `EditorBuffer::apply_patches_with(patches, decorations, wrap_map, wrap_policy, history, label)` | パッチを適用し、履歴・テキスト・装飾・折り返しの順で全サブシステムを伝搬する |
| `EditorBuffer::set_read_only(value)` | 読み取り専用フラグを切り替える。セット時は `apply_patches` / `apply_patches_with` がバッファを変更せずエラーを返す |
| `EditorBuffer::is_read_only()` | 現在の読み取り専用フラグを返す |
| `EditorBuffer::detect_indent(sample_lines)` | 先頭 `sample_lines` 行からインデントスタイル（タブ or スペース）をヒューリスティックに検出する |
| `EditorBuffer::find_matching_bracket(offset)` | `offset` にある括弧と対応する括弧を探し、両方のバイトオフセットを返す。見つからない場合は `None` |

### `RangeChange`（re-export）

| 項目 | 説明 |
|------|------|
| `RangeChange::new(start, old_end, new_end)` | コンストラクタ |
| `RangeChange::start()` | 変更が始まるバイトオフセット |
| `RangeChange::old_end()` | 変更前テキストの終了バイトオフセット |
| `RangeChange::new_end()` | 変更後テキストの終了バイトオフセット |

## ライセンス

MIT
