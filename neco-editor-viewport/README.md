# neco-editor-viewport

[日本語](README-ja.md)

Viewport geometry calculations for editor rendering. Converts between byte offsets, logical/visual lines, and pixel coordinates without depending on any DOM or Canvas API.

## How it works

All functions are stateless and take `ViewportMetrics` (line height, character width, tab width) as input. `ViewportLayout` adds gutter and content-area offsets.

`visible_line_range` returns the first and last visual line numbers visible in a scrolled container. `caret_rect` computes the pixel rectangle for the cursor at a given byte offset. `selection_rects` returns one rectangle per visual line covered by a selection. `hit_test` goes the other direction: given a click at pixel `(x, y)`, it returns the byte offset in the text.

Logical-to-visual line translation relies on `neco-wrap::WrapMap`. Line/column resolution uses `neco-textview::LineIndex`. The viewport layer itself holds no state.

## Usage

```rust
use neco_editor_viewport::{
    ViewportMetrics, ViewportLayout, visible_line_range,
    caret_rect, hit_test, gutter_width,
};
use neco_textview::LineIndex;
use neco_wrap::{WrapMap, WrapPolicy};

let text = "hello\nworld";
let li = LineIndex::new(text);
let lines: Vec<&str> = text.split('\n').collect();
let wm = WrapMap::new(lines.iter().copied(), 80, &WrapPolicy::code());
let metrics = ViewportMetrics {
    line_height: 20.0,
    char_width: 8.0,
    tab_width: 4,
};

// Which visual lines are visible?
let (first, last) = visible_line_range(0.0, 100.0, &wm, &metrics);

// Where should the caret be drawn?
let gw = gutter_width(li.line_count(), &metrics);
let layout = ViewportLayout {
    gutter_width: gw,
    content_left: gw + 8.0,
};
let rect = caret_rect(text, 0, &li, &wm, &metrics, &layout).unwrap();

// Click to offset
let offset = hit_test(rect.x, rect.y, 0.0, text, &li, &wm, &metrics, &layout);
assert_eq!(offset, 0);
```

## API

| Item | Description |
|------|-------------|
| `ViewportMetrics` | Font metrics: line_height, char_width, tab_width |
| `ViewportLayout` | Computed layout: gutter_width, content_left |
| `Rect` | Pixel rectangle with x, y, width, height |
| `ViewportError` | Wraps `TextViewError` from line index operations |
| `visible_line_range(scroll_top, container_height, wrap_map, metrics)` | First and last visible visual line numbers |
| `caret_rect(text, offset, line_index, wrap_map, metrics, layout)` | Pixel rectangle for the caret at `offset` |
| `selection_rects(text, selection, line_index, wrap_map, metrics, layout)` | One rectangle per visual line in the selection |
| `hit_test(x, y, scroll_top, text, line_index, wrap_map, metrics, layout)` | Pixel coordinates to byte offset |
| `gutter_width(total_lines, metrics)` | Line-number gutter width in pixels |
| `line_top(visual_line, metrics)` | Y coordinate for the top of a visual line |
| `scroll_to_reveal(text, offset, scroll_top, container_height, line_index, wrap_map, metrics)` | New scroll_top to reveal the caret, or `None` if already visible |

## License

MIT
