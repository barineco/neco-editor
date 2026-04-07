# Changelog

## 0.2.0

- `RangeChange` の定義元を `neco-textview` に移動し、re-export で互換性を維持
- `neco-textview 0.2` を依存に追加
- `RangeChange` のフィールドアクセスをアクセサメソッド経由に変更
- `RangeChange` の定義元変更を含むため、0.1.x からは破壊的変更

## 0.1.0

- 初回公開
- `Highlight`, `Marker`, `Widget` の decoration データモデルを追加
- `DecorationSet` によるタグ付き管理と範囲クエリを追加
- `map_through_changes` によるテキスト編集追従を追加
