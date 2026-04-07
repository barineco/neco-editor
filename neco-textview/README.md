# neco-textview

[日本語](README-ja.md)

UTF-8 text primitives for editor cursors: byte-offset/line-column conversion, selection ranges, and UTF-16 offset mapping.

## Primitives

`neco-textview` gives each text buffer a `LineIndex` built from its source. From that index you can translate byte offsets to `(line, column)` positions and back, look up byte ranges for a line, and convert between byte offsets and UTF-16 code-unit offsets for LSP-compatible callers.

`Selection` tracks anchor and head independently so backward selections are preserved. `TextRange` is a plain validated `[start, end)` pair used throughout the API.

## Usage

### Byte offset to line/column

```rust
use neco_textview::{LineIndex, Position};

let text = "abc\ndef\nghi";
let idx = LineIndex::new(text);

let pos = idx.offset_to_position(text, 4).unwrap();
assert_eq!(pos, Position::new(1, 0));

let back = idx.position_to_offset(text, pos).unwrap();
assert_eq!(back, 4);
```

### UTF-16 offset conversion

```rust
use neco_textview::Utf16Mapping;

// "aあb": 'あ' is 3 bytes but 1 UTF-16 code unit
let text = "aあb";
let m = Utf16Mapping::new(text);

assert_eq!(m.byte_to_utf16(4).unwrap(), 2); // 'b' at byte 4 → UTF-16 offset 2
assert_eq!(m.utf16_to_byte(2).unwrap(), 4);
```

### Selection

```rust
use neco_textview::Selection;

let sel = Selection::new(8, 2); // backward selection
assert!(!sel.is_forward());
let r = sel.range();
assert_eq!(r.start(), 2);
assert_eq!(r.end(), 8);
```

## API

| Item | Description |
|------|-------------|
| `Position` | `(line, column)` pair using `u32` coordinates |
| `TextRange` | Validated `[start, end)` byte range |
| `TextRange::new(start, end)` | Returns `Err` when `start > end` |
| `TextRange::empty(offset)` | Zero-length range at one offset |
| `Selection` | Anchor/head pair; preserves direction |
| `Selection::cursor(offset)` | Collapsed selection at one offset |
| `Selection::range()` | Normalized `TextRange` regardless of direction |
| `LineIndex` | Precomputed line-start table for a text buffer |
| `LineIndex::offset_to_position` | Byte offset → `Position`, validates UTF-8 boundary |
| `LineIndex::position_to_offset` | `Position` → byte offset |
| `LineIndex::line_range(line)` | Byte range of a line excluding the trailing `\n` |
| `LineIndex::line_range_with_newline(line)` | Byte range including the trailing `\n` |
| `LineIndex::line_of_offset(offset)` | Line number for a byte offset |
| `Utf16Mapping` | Anchor-based byte ↔ UTF-16 code-unit translator |
| `Utf16Mapping::byte_to_utf16` | Returns `Err` on non-char-boundary offsets |
| `Utf16Mapping::utf16_to_byte` | Returns `Err` on surrogate-pair interior offsets |
| `RangeChange` | Abstract description of a text change: `(start, old_end, new_end)` |
| `RangeChange::new(start, old_end, new_end)` | Constructor |
| `RangeChange::start()` | Byte offset where the change begins |
| `RangeChange::old_end()` | Byte offset where the old text ended |
| `RangeChange::new_end()` | Byte offset where the new text ends |
| `TextViewError` | Invalid range, out-of-bounds offset, UTF-8/UTF-16 boundary violations |

## License

MIT
