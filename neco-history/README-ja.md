# neco-history

[英語版](README.md)

ツリー構造の編集履歴。undo、redo、ブランチ分岐、自動チェックポイントを提供します。

undo 後に新しい編集を行うと、古いタイムラインを破棄せず新しいブランチが作成されます。cmd+z は通常のリニアな undo として動作し、ツリー API によりブランチの可視化や任意ノードへのジャンプが可能です。

## 機能

- 可逆編集: forward/inverse `TextPatch` を保持 (逆パッチは自動生成)
- スナップショット編集: 差分計算が非合理な操作向けにテキスト全文を保持
- ツリー構造のブランチ分岐: undo + 新規編集で兄弟ブランチを作成
- 自動チェックポイント: 定期的な完全スナップショットで遠距離ジャンプを高速化
- 任意ノードへのジャンプ: LCA 経由の undo/redo ステップ列を計算
- `neco-tree` によるポリシーベースの枝刈り

## 使い方

基本的な undo/redo:

```rust
use neco_history::EditHistory;
use neco_textpatch::{TextPatch, apply_patches};

let mut history = EditHistory::new("hello");
let mut text = "hello".to_string();

// 編集を記録
let forward = vec![TextPatch::insert(5, " world")];
history.push_edit("add world", &text, forward.clone());
text = apply_patches(&text, &forward).unwrap();
assert_eq!(text, "hello world");

// Undo
let undo = history.undo().unwrap();
text = apply_patches(&text, undo.inverse_patches.as_ref().unwrap()).unwrap();
assert_eq!(text, "hello");

// Redo
let redo = history.redo().unwrap();
text = apply_patches(&text, redo.forward_patches.as_ref().unwrap()).unwrap();
assert_eq!(text, "hello world");
```

ブランチ分岐:

```rust
use neco_history::EditHistory;
use neco_textpatch::TextPatch;

let mut history = EditHistory::new("abc");
history.push_edit("branch-1", "abc", vec![TextPatch::insert(3, "1")]);
history.undo();
history.push_edit("branch-2", "abc", vec![TextPatch::insert(3, "2")]);

// ルートから 2 つのブランチが存在
assert_eq!(history.tree().root().child_count(), 2);
```

## API

| 項目 | 説明 |
|------|------|
| `EditHistory` | カーソルベースの undo/redo を持つツリー構造の編集履歴 |
| `HistoryEntry` | 各履歴ノードのデータ (label, kind, patches, snapshot) |
| `EntryKind` | `Reversible` (パッチベース) または `Snapshot` (テキスト全文) |
| `UndoResult` | `EditHistory::undo` が返す undo 操作情報 |
| `RedoResult` | `EditHistory::redo` が返す redo 操作情報 |
| `JumpStep` | `jump_to` パス内の 1 ステップ (`Undo` または `Redo`) |

## ライセンス

MIT
