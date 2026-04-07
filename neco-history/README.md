# neco-history

[日本語](README-ja.md)

Tree-based edit history with undo, redo, branching, and automatic checkpointing.

When the user undoes and makes a new edit, a new branch is created instead of discarding the old timeline. cmd+z works as normal linear undo, while the tree API enables branch visualization and arbitrary node jumps.

## Features

- Reversible edits: stores forward and inverse `TextPatch`es (inverse computed automatically)
- Snapshot edits: stores a complete text for operations where deltas are impractical
- Tree-structured branching: undo + new edit creates a sibling branch
- Automatic checkpointing: periodic full snapshots for fast long-range jumps
- Jump to any node: computes the undo/redo step sequence via LCA
- Policy-based pruning via `neco-tree`

## Usage

Basic undo/redo:

```rust
use neco_history::EditHistory;
use neco_textpatch::{TextPatch, apply_patches};

let mut history = EditHistory::new("hello");
let mut text = "hello".to_string();

// Record an edit
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

Branching:

```rust
use neco_history::EditHistory;
use neco_textpatch::TextPatch;

let mut history = EditHistory::new("abc");
history.push_edit("branch-1", "abc", vec![TextPatch::insert(3, "1")]);
history.undo();
history.push_edit("branch-2", "abc", vec![TextPatch::insert(3, "2")]);

// Two branches exist from root
assert_eq!(history.tree().root().child_count(), 2);
```

## API

| Item | Description |
|------|-------------|
| `EditHistory` | Tree-structured edit history with cursor-based undo/redo |
| `HistoryEntry` | Data stored in each history node (label, kind, patches, snapshot) |
| `EntryKind` | `Reversible` (patch-based) or `Snapshot` (full text) |
| `UndoResult` | Undo operation info returned by `EditHistory::undo` |
| `RedoResult` | Redo operation info returned by `EditHistory::redo` |
| `JumpStep` | Single step (`Undo` or `Redo`) in a `jump_to` path |

## License

MIT
