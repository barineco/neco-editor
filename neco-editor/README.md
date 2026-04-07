# neco-editor

[日本語](README-ja.md)

Umbrella crate for editor runtime primitives: re-exports ten foundational crates and provides `EditorBuffer`, a unified text buffer that keeps text, line index, decorations, wrap state, and edit history in sync.

## Overview

`neco-editor` re-exports the following crates under one entry point. You can use `neco_editor::textview::LineIndex` or depend on each crate directly.

| Crate | Role |
|-------|------|
| `neco-textview` | Position primitives: `Position`, `TextRange`, `Selection`, `LineIndex`, `Utf16Mapping`, `RangeChange` |
| `neco-textpatch` | Text edit operations: `TextPatch`, `apply_patches`, `inverse_patches` |
| `neco-decor` | Decoration data model: `Decoration`, `DecorationSet` |
| `neco-diffcore` | Diff computation: `diff`, `to_hunks`, `diff_to_patches` |
| `neco-wrap` | Line wrapping: `WrapMap`, `WrapPolicy` |
| `neco-history` | Edit history tree: `EditHistory`, undo/redo |
| `neco-tree` | General-purpose tree: `Tree`, `CursoredTree` (re-exported because `EditHistory` exposes these in its public API) |
| `neco-pathrel` | Path normalization |
| `neco-filetree` | File tree structure |
| `neco-watchnorm` | File watch event normalization |

The only type defined in this crate is `EditorBuffer`. It owns the text and a `LineIndex`, and keeps all subsystems consistent on each edit.

## EditorBuffer

`EditorBuffer` holds a `String` and a `LineIndex` built from it. Calling `apply_patches` applies a batch of `TextPatch` values, rebuilds the `LineIndex`, and returns a `Vec<RangeChange>` for downstream consumers.

`apply_patches_with` goes further: it records the edit to an `EditHistory`, applies the patches, propagates `RangeChange` values to a `DecorationSet`, and optionally rewraps the affected lines in a `WrapMap`. History is recorded before the text changes so that the inverse patch can be computed from the original text.

All optional subsystems (`WrapMap`, `WrapPolicy`, `EditHistory`) are passed as `Option`. Callers that do not use wrap or history pass `None` and pay no overhead.

## Usage

### Basic: apply patches and get range changes

```rust
use neco_editor::{EditorBuffer, neco_textpatch::TextPatch};

let mut buffer = EditorBuffer::new("hello world".to_string());
let patches = [TextPatch::replace(6, 11, "rust").unwrap()];

let changes = buffer.apply_patches(&patches).unwrap();

assert_eq!(buffer.text(), "hello rust");
assert_eq!(buffer.line_index().text_len(), 10);
// changes[0] describes the replaced span: start=6, old_end=11, new_end=10
```

### Full integration: history and decorations

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
        None,        // no WrapMap
        None,        // no WrapPolicy
        Some(&mut history),
        Some("replace word"),
    )
    .unwrap();

assert_eq!(buffer.text(), "hello rust");
// The decoration's end byte was shifted from 11 to 10 by map_through_changes.
assert_eq!(decorations.iter().next().unwrap().1.end(), 10);
assert_eq!(history.current_label(), "replace word");
```

## API

### `EditorBuffer`

| Item | Description |
|------|-------------|
| `EditorBuffer::new(text)` | Construct from an owned `String`; builds the initial `LineIndex` |
| `EditorBuffer::text()` | Borrow the current text |
| `EditorBuffer::line_index()` | Borrow the current `LineIndex` |
| `EditorBuffer::apply_patches(patches)` | Apply patches, rebuild `LineIndex`, return `Vec<RangeChange>` or `TextPatchError` |
| `EditorBuffer::apply_patches_with(patches, decorations, wrap_map, wrap_policy, history, label)` | Apply patches and propagate to all subsystems in order: history, text, decorations, wrap |
| `EditorBuffer::set_read_only(value)` | Toggle read-only mode; when set, `apply_patches` and `apply_patches_with` return an error without modifying the buffer |
| `EditorBuffer::is_read_only()` | Return the current read-only flag |
| `EditorBuffer::detect_indent(sample_lines)` | Heuristically detect the indentation style (tabs or spaces) from the first `sample_lines` lines |
| `EditorBuffer::find_matching_bracket(offset)` | Find the bracket that matches the one at `offset` and return both byte offsets, or `None` |

### `RangeChange` (re-export)

| Item | Description |
|------|-------------|
| `RangeChange::new(start, old_end, new_end)` | Constructor |
| `RangeChange::start()` | Byte offset where the change begins |
| `RangeChange::old_end()` | Byte offset where the old text ended |
| `RangeChange::new_end()` | Byte offset where the new text ends |

## License

MIT
