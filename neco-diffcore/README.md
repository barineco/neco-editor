# neco-diffcore

[日本語](README-ja.md)

Line-level and character-level diff using Myers O(ND) algorithm, with hunk grouping, side-by-side layout, and patch generation.

## Diff behavior

`neco-diffcore` runs Myers diff on lines to produce a `DiffResult`. From there you can:

- Group changed lines into `DiffHunk`s with a configurable context window.
- Compute character-level changes within a single pair of lines with `diff_intra_line`.
- Reformat the result into a side-by-side layout with `to_side_by_side`.
- Convert the diff into `neco-textpatch` patches with `diff_to_patches` to apply the change back to the old text.

All positions are byte ranges so callers can use them directly for rendering without a second scan.

## Usage

### Line diff and hunks

```rust
use neco_diffcore::{diff, DiffOp};

let old = "a\nb\nc\nd\n";
let new = "a\nB\nc\nD\n";
let result = diff(old, new);

let hunks = result.to_hunks(1);
assert_eq!(hunks.len(), 2);
assert!(hunks[0].lines().iter().any(|l| l.op() != DiffOp::Equal));
```

### Intra-line diff

```rust
use neco_diffcore::{diff_intra_line, DiffOp};

let intra = diff_intra_line("hello world", "hello there");
assert!(intra.ranges().iter().any(|r| r.op() == DiffOp::Delete));
assert!(intra.ranges().iter().any(|r| r.op() == DiffOp::Insert));
```

### Patch roundtrip

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

| Item | Description |
|------|-------------|
| `DiffOp` | `Equal`, `Insert`, or `Delete` |
| `ByteRange` | `[start, end)` byte span within one side of the diff |
| `DiffLine` | One line in the diff with op, line numbers, and byte ranges for both sides |
| `DiffResult` | Full flat list of `DiffLine`s from `diff` |
| `DiffResult::to_hunks(context_lines)` | Group changes into `DiffHunk`s; nearby groups are merged |
| `DiffHunk` | One contiguous group of changed lines with old/new line coordinates |
| `IntraLineRange` | One changed byte span within a single line, with `DiffOp` |
| `IntraLineDiff` | Character-level diff result for one line pair |
| `SideLine` | One side of a side-by-side row with line number, op, and byte range |
| `SideBySideLine` | Paired left/right lines for two-column rendering; either side may be `None` |
| `diff(old, new)` | Run line-level Myers diff and return a `DiffResult` |
| `diff_intra_line(old_line, new_line)` | Run character-level Myers diff on one line pair |
| `to_side_by_side(result)` | Reformat a `DiffResult` into paired left/right rows |
| `diff_to_patches(new, result)` | Convert a `DiffResult` into `neco-textpatch` patches |

## License

MIT
