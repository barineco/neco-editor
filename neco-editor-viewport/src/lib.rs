//! Viewport geometry calculations for editor rendering.
//!
//! Pure geometry: converts between byte offsets, logical/visual lines,
//! and pixel coordinates. No DOM or Canvas dependency.

use neco_textview::{LineIndex, Selection, TextViewError};
use neco_wrap::WrapMap;
use std::fmt;

/// Host-injected font metrics. Plain struct (parameter bag).
#[derive(Debug, Clone, Copy)]
pub struct ViewportMetrics {
    pub line_height: f64,
    pub char_width: f64,
    pub tab_width: u32,
}

/// Computed layout measurements.
#[derive(Debug, Clone, Copy)]
pub struct ViewportLayout {
    pub gutter_width: f64,
    pub content_left: f64,
}

/// Axis-aligned rectangle in pixel coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Errors returned by viewport operations.
#[non_exhaustive]
#[derive(Debug)]
pub enum ViewportError {
    TextView(TextViewError),
}

impl fmt::Display for ViewportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TextView(e) => write!(f, "text view error: {e}"),
        }
    }
}

impl std::error::Error for ViewportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TextView(e) => Some(e),
        }
    }
}

impl From<TextViewError> for ViewportError {
    fn from(e: TextViewError) -> Self {
        Self::TextView(e)
    }
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Compute the visual width in pixels of `text[start_byte..end_byte]` considering tabs.
fn text_width(text: &str, start_byte: usize, end_byte: usize, metrics: &ViewportMetrics) -> f64 {
    let slice = &text[start_byte..end_byte];
    let mut width = 0.0;
    for ch in slice.chars() {
        if ch == '\t' {
            width += f64::from(metrics.tab_width) * metrics.char_width;
        } else {
            width += metrics.char_width;
        }
    }
    width
}

fn u32_to_usize(v: u32) -> usize {
    usize::try_from(v).expect("u32 exceeds usize::MAX")
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Returns `(first_visual_line, last_visual_line)` visible in the viewport.
///
/// Clamps to `0..total_visual_lines - 1`.
pub fn visible_line_range(
    scroll_top: f64,
    container_height: f64,
    wrap_map: &WrapMap,
    metrics: &ViewportMetrics,
) -> (u32, u32) {
    let total = wrap_map.total_visual_lines();
    if total == 0 {
        return (0, 0);
    }
    let max_line = total - 1;

    let first_f = scroll_top / metrics.line_height;
    let first = if first_f < 0.0 {
        0u32
    } else {
        let v = first_f as u64;
        u32::try_from(v.min(u64::from(max_line))).expect("clamped value fits u32")
    };

    let last_f = (scroll_top + container_height) / metrics.line_height;
    let last_raw = if last_f < 0.0 {
        0u64
    } else {
        // Subtract 1 because the range is inclusive; if exactly on a boundary
        // the pixel row above is the last visible line. Use ceil-1 approach:
        // any fractional part means we can see part of that line.
        let ceil = last_f.ceil() as u64;
        if ceil == 0 {
            0
        } else {
            ceil - 1
        }
    };
    let last = u32::try_from(last_raw.min(u64::from(max_line))).expect("clamped value fits u32");

    (first, last)
}

/// Compute the pixel rectangle for the caret at `offset`.
pub fn caret_rect(
    text: &str,
    offset: usize,
    line_index: &LineIndex,
    wrap_map: &WrapMap,
    metrics: &ViewportMetrics,
    layout: &ViewportLayout,
) -> Result<Rect, ViewportError> {
    let line = line_index.line_of_offset(offset)?;
    let line_range = line_index.line_range(line)?;
    let byte_in_line = offset - line_range.start();
    let byte_in_line_u32 =
        u32::try_from(byte_in_line).expect("byte offset in line exceeds u32::MAX");

    let visual_line = wrap_map.to_visual_line(line, byte_in_line_u32);

    // Determine the start byte (within the line) of this visual sub-line.
    let (_, vl_start_in_line) = wrap_map.from_visual_line(visual_line);
    let vl_start_abs = line_range.start() + u32_to_usize(vl_start_in_line);

    let x = layout.content_left + text_width(text, vl_start_abs, offset, metrics);
    let y = f64::from(visual_line) * metrics.line_height;

    Ok(Rect {
        x,
        y,
        width: 2.0,
        height: metrics.line_height,
    })
}

/// Compute the pixel rectangles that cover `selection`.
pub fn selection_rects(
    text: &str,
    selection: &Selection,
    line_index: &LineIndex,
    wrap_map: &WrapMap,
    metrics: &ViewportMetrics,
    layout: &ViewportLayout,
) -> Result<Vec<Rect>, ViewportError> {
    let range = selection.range();
    if range.is_empty() {
        return Ok(Vec::new());
    }

    let start_offset = range.start();
    let end_offset = range.end();

    let start_line = line_index.line_of_offset(start_offset)?;
    let start_line_range = line_index.line_range(start_line)?;
    let start_byte_in_line = start_offset - start_line_range.start();
    let start_byte_u32 =
        u32::try_from(start_byte_in_line).expect("byte offset in line exceeds u32::MAX");
    let first_vl = wrap_map.to_visual_line(start_line, start_byte_u32);

    let end_line = line_index.line_of_offset(end_offset)?;
    let end_line_range = line_index.line_range(end_line)?;
    let end_byte_in_line = end_offset - end_line_range.start();
    let end_byte_u32 =
        u32::try_from(end_byte_in_line).expect("byte offset in line exceeds u32::MAX");
    let last_vl = wrap_map.to_visual_line(end_line, end_byte_u32);

    let mut rects = Vec::new();

    for vl in first_vl..=last_vl {
        let (log_line, vl_start_in_line) = wrap_map.from_visual_line(vl);
        let lr = line_index.line_range(log_line)?;
        let vl_start_abs = lr.start() + u32_to_usize(vl_start_in_line);

        // Determine end of this visual line.
        let total_vl = wrap_map.total_visual_lines();
        let vl_end_abs = if vl + 1 < total_vl {
            let (next_log, next_start_in_line) = wrap_map.from_visual_line(vl + 1);
            if next_log == log_line {
                lr.start() + u32_to_usize(next_start_in_line)
            } else {
                lr.end()
            }
        } else {
            lr.end()
        };

        // Clamp selection to this visual line.
        let sel_start = start_offset.max(vl_start_abs);
        let sel_end = end_offset.min(vl_end_abs);

        if sel_start >= sel_end {
            continue;
        }

        let x = layout.content_left + text_width(text, vl_start_abs, sel_start, metrics);
        let w = text_width(text, sel_start, sel_end, metrics);
        let y = f64::from(vl) * metrics.line_height;

        rects.push(Rect {
            x,
            y,
            width: w,
            height: metrics.line_height,
        });
    }

    Ok(rects)
}

/// Convert a click at pixel `(x, y)` to a byte offset in `text`.
#[allow(clippy::too_many_arguments)]
pub fn hit_test(
    x: f64,
    y: f64,
    scroll_top: f64,
    text: &str,
    line_index: &LineIndex,
    wrap_map: &WrapMap,
    metrics: &ViewportMetrics,
    layout: &ViewportLayout,
) -> usize {
    let total_vl = wrap_map.total_visual_lines();
    if total_vl == 0 {
        return 0;
    }

    // Determine visual line from y coordinate.
    let vl_f = (y + scroll_top) / metrics.line_height;
    let vl_raw = if vl_f < 0.0 { 0u64 } else { vl_f as u64 };
    let vl = u32::try_from(vl_raw.min(u64::from(total_vl - 1))).expect("clamped value fits u32");

    let (log_line, vl_start_in_line) = wrap_map.from_visual_line(vl);
    let lr = match line_index.line_range(log_line) {
        Ok(r) => r,
        Err(_) => return text.len(),
    };
    let vl_start_abs = lr.start() + u32_to_usize(vl_start_in_line);

    // Determine end of this visual line.
    let vl_end_abs = if vl + 1 < total_vl {
        let (next_log, next_start_in_line) = wrap_map.from_visual_line(vl + 1);
        if next_log == log_line {
            lr.start() + u32_to_usize(next_start_in_line)
        } else {
            lr.end()
        }
    } else {
        lr.end()
    };

    // x relative to content area.
    let rel_x = (x - layout.content_left).max(0.0);

    // Walk characters to find the offset.
    let slice = &text[vl_start_abs..vl_end_abs];
    let mut accum = 0.0;
    for (i, ch) in slice.char_indices() {
        let cw = if ch == '\t' {
            f64::from(metrics.tab_width) * metrics.char_width
        } else {
            metrics.char_width
        };
        // If click is within the first half of the character, place caret before it.
        if rel_x < accum + cw * 0.5 {
            return vl_start_abs + i;
        }
        accum += cw;
    }

    // Past end of visual line: clamp to end.
    vl_end_abs
}

/// Compute the line-number gutter width for `total_lines` lines.
pub fn gutter_width(total_lines: u32, metrics: &ViewportMetrics) -> f64 {
    let digit_count = total_lines.max(1).ilog10() + 1;
    f64::from(digit_count) * metrics.char_width + metrics.char_width
}

/// Return the Y pixel coordinate for the top of `visual_line`.
pub fn line_top(visual_line: u32, metrics: &ViewportMetrics) -> f64 {
    f64::from(visual_line) * metrics.line_height
}

/// Compute a new `scroll_top` that reveals the caret at `offset`, or `None` if already visible.
pub fn scroll_to_reveal(
    _text: &str,
    offset: usize,
    scroll_top: f64,
    container_height: f64,
    line_index: &LineIndex,
    wrap_map: &WrapMap,
    metrics: &ViewportMetrics,
) -> Result<Option<f64>, ViewportError> {
    let line = line_index.line_of_offset(offset)?;
    let line_range = line_index.line_range(line)?;
    let byte_in_line = offset - line_range.start();
    let byte_in_line_u32 =
        u32::try_from(byte_in_line).expect("byte offset in line exceeds u32::MAX");
    let vl = wrap_map.to_visual_line(line, byte_in_line_u32);

    let top = f64::from(vl) * metrics.line_height;
    let bottom = top + metrics.line_height;

    if top < scroll_top {
        // Caret is above the viewport.
        Ok(Some(top))
    } else if bottom > scroll_top + container_height {
        // Caret is below the viewport.
        Ok(Some(bottom - container_height))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neco_wrap::{WrapMap, WrapPolicy};

    fn default_metrics() -> ViewportMetrics {
        ViewportMetrics {
            line_height: 20.0,
            char_width: 8.0,
            tab_width: 4,
        }
    }

    fn default_layout() -> ViewportLayout {
        ViewportLayout {
            gutter_width: 40.0,
            content_left: 48.0,
        }
    }

    fn make_wrap_map(text: &str, max_width: u32) -> WrapMap {
        let lines: Vec<&str> = text.split('\n').collect();
        WrapMap::new(lines.iter().copied(), max_width, &WrapPolicy::code())
    }

    // -----------------------------------------------------------------------
    // visible_line_range
    // -----------------------------------------------------------------------

    #[test]
    fn visible_line_range_single_line() {
        let text = "hello";
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let (first, last) = visible_line_range(0.0, 100.0, &wm, &metrics);
        assert_eq!(first, 0);
        assert_eq!(last, 0);
    }

    #[test]
    fn visible_line_range_multi_line() {
        let text = "aaa\nbbb\nccc\nddd\neee";
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        // Container shows 3 lines (60px / 20px = 3 lines).
        let (first, last) = visible_line_range(0.0, 60.0, &wm, &metrics);
        assert_eq!(first, 0);
        assert_eq!(last, 2);
    }

    #[test]
    fn visible_line_range_scrolled() {
        let text = "aaa\nbbb\nccc\nddd\neee";
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        // scroll_top=40 means first visible is line 2.
        let (first, last) = visible_line_range(40.0, 40.0, &wm, &metrics);
        assert_eq!(first, 2);
        assert_eq!(last, 3);
    }

    #[test]
    fn visible_line_range_with_wrapping() {
        // "ab cd ef" with max_width=4 wraps into 3 visual lines.
        let text = "ab cd ef";
        let wm = make_wrap_map(text, 4);
        let metrics = default_metrics();
        let total = wm.total_visual_lines();
        assert_eq!(total, 3);
        let (first, last) = visible_line_range(0.0, 60.0, &wm, &metrics);
        assert_eq!(first, 0);
        assert_eq!(last, 2);
    }

    #[test]
    fn visible_line_range_clamps_past_end() {
        let text = "aaa\nbbb";
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let (first, last) = visible_line_range(0.0, 1000.0, &wm, &metrics);
        assert_eq!(first, 0);
        assert_eq!(last, 1);
    }

    // -----------------------------------------------------------------------
    // caret_rect
    // -----------------------------------------------------------------------

    #[test]
    fn caret_rect_line_start() {
        let text = "hello\nworld";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        let r = caret_rect(text, 0, &li, &wm, &metrics, &layout).unwrap();
        assert!((r.x - layout.content_left).abs() < f64::EPSILON);
        assert!((r.y - 0.0).abs() < f64::EPSILON);
        assert!((r.height - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn caret_rect_line_middle() {
        let text = "hello\nworld";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        // offset 3 = 'l' in "hello", x = content_left + 3 * char_width
        let r = caret_rect(text, 3, &li, &wm, &metrics, &layout).unwrap();
        let expected_x = layout.content_left + 3.0 * metrics.char_width;
        assert!((r.x - expected_x).abs() < f64::EPSILON);
        assert!((r.y - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn caret_rect_second_line() {
        let text = "hello\nworld";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        // offset 6 = start of "world", visual line 1.
        let r = caret_rect(text, 6, &li, &wm, &metrics, &layout).unwrap();
        assert!((r.x - layout.content_left).abs() < f64::EPSILON);
        assert!((r.y - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn caret_rect_wrapped_line() {
        // "ab cd ef" wraps at width 4 into 3 visual lines:
        // vl0: "ab " (bytes 0..3), vl1: "cd " (bytes 3..6), vl2: "ef" (bytes 6..8)
        let text = "ab cd ef";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 4);
        let metrics = default_metrics();
        let layout = default_layout();

        // offset 4 = 'd' in "cd ", which is on visual line 1, column 1.
        let r = caret_rect(text, 4, &li, &wm, &metrics, &layout).unwrap();
        let expected_x = layout.content_left + 1.0 * metrics.char_width;
        assert!((r.x - expected_x).abs() < f64::EPSILON);
        assert!((r.y - 20.0).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // hit_test
    // -----------------------------------------------------------------------

    #[test]
    fn hit_test_basic() {
        let text = "hello\nworld";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        // Click at start of first line.
        let offset = hit_test(
            layout.content_left,
            0.0,
            0.0,
            text,
            &li,
            &wm,
            &metrics,
            &layout,
        );
        assert_eq!(offset, 0);
    }

    #[test]
    fn hit_test_middle_of_line() {
        let text = "hello";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        // Click at x = content_left + 2.5 * char_width -> offset 3 (past midpoint of char 2).
        let x = layout.content_left + 2.5 * metrics.char_width;
        let offset = hit_test(x, 0.0, 0.0, text, &li, &wm, &metrics, &layout);
        assert_eq!(offset, 3);
    }

    #[test]
    fn hit_test_gutter_area_clamps_to_line_start() {
        let text = "hello\nworld";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        // Click in gutter (x=0), second line (y=25).
        let offset = hit_test(0.0, 25.0, 0.0, text, &li, &wm, &metrics, &layout);
        assert_eq!(offset, 6); // start of "world"
    }

    #[test]
    fn hit_test_wrapped_line() {
        let text = "ab cd ef";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 4);
        let metrics = default_metrics();
        let layout = default_layout();

        // Click on visual line 2 (y=40..60), at content_left -> offset 6 ("ef").
        let offset = hit_test(
            layout.content_left,
            45.0,
            0.0,
            text,
            &li,
            &wm,
            &metrics,
            &layout,
        );
        assert_eq!(offset, 6);
    }

    #[test]
    fn hit_test_past_end_of_line() {
        let text = "hi";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        // Click far to the right.
        let offset = hit_test(
            layout.content_left + 500.0,
            0.0,
            0.0,
            text,
            &li,
            &wm,
            &metrics,
            &layout,
        );
        assert_eq!(offset, 2);
    }

    // -----------------------------------------------------------------------
    // gutter_width
    // -----------------------------------------------------------------------

    #[test]
    fn gutter_width_1_line() {
        let metrics = default_metrics();
        let w = gutter_width(1, &metrics);
        // 1 digit + 1 padding = 2 * 8 = 16
        assert!((w - 16.0).abs() < f64::EPSILON);
    }

    #[test]
    fn gutter_width_10_lines() {
        let metrics = default_metrics();
        let w = gutter_width(10, &metrics);
        // 2 digits + 1 padding = 3 * 8 = 24
        assert!((w - 24.0).abs() < f64::EPSILON);
    }

    #[test]
    fn gutter_width_100_lines() {
        let metrics = default_metrics();
        let w = gutter_width(100, &metrics);
        // 3 digits + 1 padding = 4 * 8 = 32
        assert!((w - 32.0).abs() < f64::EPSILON);
    }

    #[test]
    fn gutter_width_1000_lines() {
        let metrics = default_metrics();
        let w = gutter_width(1000, &metrics);
        // 4 digits + 1 padding = 5 * 8 = 40
        assert!((w - 40.0).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // line_top
    // -----------------------------------------------------------------------

    #[test]
    fn line_top_zero() {
        let metrics = default_metrics();
        assert!((line_top(0, &metrics) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_top_five() {
        let metrics = default_metrics();
        assert!((line_top(5, &metrics) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_top_ten() {
        let metrics = default_metrics();
        assert!((line_top(10, &metrics) - 200.0).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // scroll_to_reveal
    // -----------------------------------------------------------------------

    #[test]
    fn scroll_to_reveal_already_visible() {
        let text = "aaa\nbbb\nccc\nddd\neee";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();

        // Offset 4 is line 1, visual line 1. scroll_top=0, container=100.
        let result = scroll_to_reveal(text, 4, 0.0, 100.0, &li, &wm, &metrics).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn scroll_to_reveal_above_viewport() {
        let text = "aaa\nbbb\nccc\nddd\neee";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();

        // Offset 0 is visual line 0 (top=0). scroll_top=40 means it is above.
        let result = scroll_to_reveal(text, 0, 40.0, 40.0, &li, &wm, &metrics).unwrap();
        assert_eq!(result, Some(0.0));
    }

    #[test]
    fn scroll_to_reveal_below_viewport() {
        let text = "aaa\nbbb\nccc\nddd\neee";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();

        // Offset 16 is line 4, visual line 4 (top=80, bottom=100).
        // scroll_top=0, container=40 -> viewport covers 0..40. Line 4 is below.
        let result = scroll_to_reveal(text, 16, 0.0, 40.0, &li, &wm, &metrics).unwrap();
        // new scroll_top = bottom - container_height = 100 - 40 = 60
        assert_eq!(result, Some(60.0));
    }

    // -----------------------------------------------------------------------
    // selection_rects
    // -----------------------------------------------------------------------

    #[test]
    fn selection_rects_empty_selection() {
        let text = "hello";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        let sel = Selection::cursor(2);
        let rects = selection_rects(text, &sel, &li, &wm, &metrics, &layout).unwrap();
        assert!(rects.is_empty());
    }

    #[test]
    fn selection_rects_single_line() {
        let text = "hello";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        let sel = Selection::new(1, 4); // "ell"
        let rects = selection_rects(text, &sel, &li, &wm, &metrics, &layout).unwrap();
        assert_eq!(rects.len(), 1);
        let r = &rects[0];
        let expected_x = layout.content_left + 1.0 * metrics.char_width;
        assert!((r.x - expected_x).abs() < f64::EPSILON);
        assert!((r.width - 3.0 * metrics.char_width).abs() < f64::EPSILON);
    }

    #[test]
    fn selection_rects_multi_line() {
        let text = "aaa\nbbb\nccc";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        // Select from middle of line 0 to middle of line 2.
        let sel = Selection::new(1, 9); // "aa\nbbb\nc"
        let rects = selection_rects(text, &sel, &li, &wm, &metrics, &layout).unwrap();
        assert_eq!(rects.len(), 3);
    }

    // -----------------------------------------------------------------------
    // roundtrip: caret_rect -> hit_test
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_caret_hit_test() {
        let text = "hello\nworld";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 80);
        let metrics = default_metrics();
        let layout = default_layout();

        for offset in [0, 3, 5, 6, 9, 11] {
            let r = caret_rect(text, offset, &li, &wm, &metrics, &layout).unwrap();
            // Hit test at the caret position should return the same offset.
            let got = hit_test(r.x, r.y, 0.0, text, &li, &wm, &metrics, &layout);
            assert_eq!(got, offset, "roundtrip failed at offset {offset}");
        }
    }

    #[test]
    fn roundtrip_caret_hit_test_wrapped() {
        let text = "ab cd ef";
        let li = LineIndex::new(text);
        let wm = make_wrap_map(text, 4);
        let metrics = default_metrics();
        let layout = default_layout();

        for offset in [0, 3, 6] {
            let r = caret_rect(text, offset, &li, &wm, &metrics, &layout).unwrap();
            let got = hit_test(r.x, r.y, 0.0, text, &li, &wm, &metrics, &layout);
            assert_eq!(got, offset, "roundtrip failed at offset {offset}");
        }
    }

    // -----------------------------------------------------------------------
    // text_width helper
    // -----------------------------------------------------------------------

    #[test]
    fn text_width_with_tabs() {
        let metrics = default_metrics();
        let text = "a\tb";
        // 'a' = 8, '\t' = 4*8=32, 'b' = 8 -> total 48
        let w = text_width(text, 0, text.len(), &metrics);
        assert!((w - 48.0).abs() < f64::EPSILON);
    }

    #[test]
    fn text_width_empty() {
        let metrics = default_metrics();
        let w = text_width("hello", 2, 2, &metrics);
        assert!((w - 0.0).abs() < f64::EPSILON);
    }
}
