# Changelog

## 0.1.2

- `merge_subtree` を単調 merge に変更: 既存の深い children を保持し、兄弟順序を維持する
- File ノードは従来通り直接置換、Directory ノードのみ再帰的にマージする
- 浅い subtree の merge で深い子孫が消失する問題を修正

## 0.1.1

- 初回公開
