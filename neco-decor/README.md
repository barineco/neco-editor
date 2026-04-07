# neco-decor

[日本語](README-ja.md)

Byte-range decorations for text buffers: highlights, line markers, and inline/block widgets stored in a sorted set that shifts automatically when the text changes.

## Decoration types

`neco-decor` tracks three kinds of decoration:

- **Highlight**: a non-empty `[start, end)` range, typically for syntax or search coloring.
- **Marker**: a point annotation at a line-start offset, used for gutter icons or diagnostics.
- **Widget**: an inline or block attachment at a byte range, used for virtual text or block decorations.

Each decoration carries a `tag` (caller-defined `u32` category) and an optional `priority` (`i16`). `DecorationSet` keeps entries sorted by start offset and assigns a stable `DecorationId` on insertion.

When text is edited, call `map_through_change` or `map_through_changes` to shift decorations. Highlights clamp to the new boundaries, markers inside a deleted range are dropped, and fully-contained widgets are removed.

## Usage

```rust
use neco_decor::{Decoration, DecorationSet};

let mut set = DecorationSet::new();

let id = set.add(Decoration::highlight(0, 5, 1).unwrap());
set.add(Decoration::marker(10, 2));

// Insert 3 bytes at position 0. Decorations after offset 0 shift right.
set.map_through_change(0, 0, 3);

let hits = set.query_range(3, 8);
assert_eq!(hits.len(), 1);
assert_eq!(hits[0].1.tag(), 1);

assert!(set.remove(id));
assert_eq!(set.len(), 1);
```

## API

| Item | Description |
|------|-------------|
| `DecorationKind` | `Highlight`, `Marker`, or `Widget { block: bool }` |
| `Decoration` | A single decoration with range, kind, tag, and priority |
| `Decoration::highlight(start, end, tag)` | Returns `Err` for empty or inverted ranges |
| `Decoration::marker(line_start, tag)` | Point annotation; `start == end` |
| `Decoration::widget(start, end, tag, block)` | Inline or block attachment; empty range is allowed |
| `Decoration::with_priority(priority)` | Builder method to set render priority |
| `DecorationId` | Stable identity returned by `DecorationSet::add` |
| `DecorationSet` | Sorted collection with insert, remove, and range query |
| `DecorationSet::add` | Insert and return a `DecorationId`; maintains sort order |
| `DecorationSet::remove` | Remove by id; returns `false` when not found |
| `DecorationSet::query_range(start, end)` | All decorations overlapping `[start, end)` |
| `DecorationSet::query_tag(tag)` | All decorations with a given tag |
| `DecorationSet::map_through_change(start, old_end, new_end)` | Shift or drop decorations after a single edit |
| `DecorationSet::map_through_changes(changes)` | Apply multiple `RangeChange` records in sequence |
| `RangeChange` | `(start, old_end, new_end)` triple describing one text edit |
| `DecorError` | Invalid range or empty highlight |

## License

MIT
