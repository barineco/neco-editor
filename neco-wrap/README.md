# neco-wrap

[日本語](README-ja.md)

Word wrap engine for splitting logical lines into visual lines, with pluggable line-breaking and character-width policies.

## How it works

`neco-wrap` splits a logical line into visual lines according to a `WrapPolicy`. A policy bundles two function pointers: one that returns the visual width of a character, and one that decides whether a break is allowed, forbidden, or mandatory at a given byte offset.

`wrap_line` processes one logical line and returns a list of `WrapPoint` values, each marking the byte offset and cumulative visual width where a visual line ends. `WrapMap` applies this over a whole document, tracks the results per logical line, and provides coordinate translation between logical `(line, byte_offset)` and visual line numbers.

Two built-in policies are provided. `WrapPolicy::code` breaks after ASCII whitespace and common operators. `WrapPolicy::japanese_basic` applies kinsoku shaping rules for Japanese text.

## Usage

### Wrapping a single line

```rust
use neco_wrap::{WrapPolicy, wrap_line};

let policy = WrapPolicy::code();
let wraps = wrap_line("ab cd ef", 4, &policy);

// Two wrap points: after "ab " and after "cd "
assert_eq!(wraps.len(), 2);
assert_eq!(wraps[0].byte_offset(), 3);
assert_eq!(wraps[1].byte_offset(), 6);
```

### Managing a whole document with WrapMap

```rust
use neco_wrap::{WrapPolicy, WrapMap};

let lines = ["hello world", "foo bar baz"];
let policy = WrapPolicy::code();
let mut map = WrapMap::new(lines.iter().copied(), 6, &policy);

// Total visual lines across all logical lines
let total = map.total_visual_lines();

// Translate logical (line, byte_offset_in_line) to a visual line number
let vline = map.to_visual_line(0, 6);

// After an edit, rewrap just the changed line
map.rewrap_line(0, "hi", &policy);
```

## API

| Item | Description |
|------|-------------|
| `BreakOpportunity` | `Allowed`, `Forbidden`, or `Mandatory` break classification at a byte offset |
| `WrapPolicy` | Bundles `char_width` and `break_opportunity` function pointers |
| `WrapPolicy::new(char_width, break_opportunity)` | Constructor from two function pointers |
| `WrapPolicy::char_width()` | Returns the stored character-width function |
| `WrapPolicy::break_opportunity()` | Returns the stored break-opportunity function |
| `WrapPolicy::code()` | Built-in policy: breaks after ASCII whitespace and common code operators |
| `WrapPolicy::japanese_basic()` | Built-in policy: breaks between CJK characters with kinsoku rules |
| `WrapPoint` | Byte offset and cumulative visual width at a visual line boundary |
| `WrapPoint::byte_offset()` | Byte offset in the logical line where the visual line ends |
| `WrapPoint::visual_width()` | Cumulative visual column count up to this wrap point |
| `VisualLine` | `[start, end)` byte range of one visual line within a logical line |
| `VisualLine::start()` | Start byte offset |
| `VisualLine::end()` | End byte offset |
| `VisualLine::len()` | Byte length of the visual line |
| `VisualLine::is_empty()` | True when start equals end |
| `WrapMap` | Per-document wrap state: stores wrap points for every logical line |
| `WrapMap::new(lines, max_width, policy)` | Build from a line iterator |
| `WrapMap::max_width()` | The column limit used for wrapping |
| `WrapMap::line_count()` | Number of logical lines |
| `WrapMap::visual_line_count(line)` | Number of visual lines for a logical line |
| `WrapMap::total_visual_lines()` | Total visual line count across the document |
| `WrapMap::wrap_points(line)` | Slice of `WrapPoint` for a logical line |
| `WrapMap::visual_lines(line, line_len)` | `VisualLine` segments for a logical line |
| `WrapMap::to_visual_line(line, byte_offset_in_line)` | Logical position to absolute visual line number |
| `WrapMap::from_visual_line(visual_line)` | Absolute visual line number to `(logical_line, start_byte_offset)` |
| `WrapMap::rewrap_line(line, line_text, policy)` | Recompute wrap points for one logical line after an edit |
| `WrapMap::set_max_width(max_width, lines, policy)` | Change the column limit and recompute all lines |
| `WrapMap::splice_lines(start_line, removed_count, new_lines, policy)` | Replace a range of logical lines, matching a document splice |
| `wrap_line(line_text, max_width, policy)` | Low-level function: wrap one line and return `Vec<WrapPoint>` |

## License

MIT
