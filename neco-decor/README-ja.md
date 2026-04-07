# neco-decor

[英語版](README.md)

テキストバッファのバイト範囲デコレーション。ハイライト、行マーカー、インライン/ブロックウィジェットをソート済みセットで管理し、テキスト編集に合わせて自動的にシフトします。

## デコレーションの種類

`neco-decor` は 3 種類のデコレーションを扱います。

- **Highlight**: `[start, end)` の空でない範囲。構文強調や検索ハイライトに使います。
- **Marker**: 行頭オフセットのポイントアノテーション。ガターアイコンや診断マーカーに使います。
- **Widget**: バイト範囲に付けるインラインまたはブロック添付物。仮想テキストやブロックデコレーションに使います。

各デコレーションは呼び出し側が決める `tag`（`u32`）と省略可能な `priority`（`i16`）を持ちます。`DecorationSet` はエントリを開始オフセット順に保持し、挿入時に安定した `DecorationId` を返します。

テキストを編集したら `map_through_change` または `map_through_changes` を呼んでデコレーションをシフトします。ハイライトは新しい境界にクランプされ、削除範囲内のマーカーは除去され、完全に包含されたウィジェットは削除されます。

## 使い方

```rust
use neco_decor::{Decoration, DecorationSet};

let mut set = DecorationSet::new();

let id = set.add(Decoration::highlight(0, 5, 1).unwrap());
set.add(Decoration::marker(10, 2));

// オフセット 0 に 3 バイト挿入。オフセット 0 以降のデコレーションが右にシフト
set.map_through_change(0, 0, 3);

let hits = set.query_range(3, 8);
assert_eq!(hits.len(), 1);
assert_eq!(hits[0].1.tag(), 1);

assert!(set.remove(id));
assert_eq!(set.len(), 1);
```

## API

| 項目 | 説明 |
|------|-------------|
| `DecorationKind` | `Highlight`、`Marker`、`Widget { block: bool }` の 3 種 |
| `Decoration` | 範囲・種別・タグ・優先度を持つデコレーション 1 件 |
| `Decoration::highlight(start, end, tag)` | 空または逆転した範囲は `Err` を返す |
| `Decoration::marker(line_start, tag)` | ポイントアノテーション。`start == end` |
| `Decoration::widget(start, end, tag, block)` | インラインまたはブロック添付物。空範囲も許可 |
| `Decoration::with_priority(priority)` | 描画優先度を設定するビルダーメソッド |
| `DecorationId` | `DecorationSet::add` が返す安定した識別子 |
| `DecorationSet` | 挿入・削除・範囲クエリができるソート済みコレクション |
| `DecorationSet::add` | 挿入して `DecorationId` を返す。ソート順を維持する |
| `DecorationSet::remove` | id で削除。見つからない場合は `false` を返す |
| `DecorationSet::query_range(start, end)` | `[start, end)` に重なるすべてのデコレーション |
| `DecorationSet::query_tag(tag)` | 指定したタグのすべてのデコレーション |
| `DecorationSet::map_through_change(start, old_end, new_end)` | 1 回の編集に応じてデコレーションをシフト・削除する |
| `DecorationSet::map_through_changes(changes)` | 複数の `RangeChange` を順番に適用する |
| `RangeChange` | 1 回のテキスト編集を表す `(start, old_end, new_end)` |
| `DecorError` | 不正な範囲または空のハイライト |

## ライセンス

MIT
