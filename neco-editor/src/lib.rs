pub use neco_decor;
pub use neco_diffcore;
pub use neco_editor_search;
pub use neco_editor_viewport;
pub use neco_filetree;
pub use neco_history;
pub use neco_pathrel;
pub use neco_textpatch;
pub use neco_textview;
pub use neco_tree;
pub use neco_watchnorm;
pub use neco_wrap;

pub use neco_textview::RangeChange;

use neco_decor::DecorationSet;
use neco_history::EditHistory;
use neco_textpatch::{TextPatch, TextPatchError};
use neco_textview::LineIndex;
use neco_wrap::{WrapMap, WrapPolicy};

/// Detected indentation style of a text buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    Tabs,
    Spaces(u32),
}

pub struct EditorBuffer {
    text: String,
    line_index: LineIndex,
}

impl EditorBuffer {
    pub fn new(text: String) -> Self {
        Self {
            line_index: LineIndex::new(&text),
            text,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_index(&self) -> &LineIndex {
        &self.line_index
    }

    /// Detects the predominant indentation style by analyzing the first `sample_lines` lines.
    /// Returns `Spaces(4)` as default when detection is inconclusive.
    pub fn detect_indent(&self, sample_lines: usize) -> IndentStyle {
        let mut tab_count: usize = 0;
        let mut space_count: usize = 0;
        let mut space_widths: Vec<u32> = Vec::new();

        for line in self.text.lines().take(sample_lines) {
            if line.is_empty() {
                continue;
            }
            let first_non_ws = match line.find(|c: char| c != ' ' && c != '\t') {
                Some(pos) if pos > 0 => pos,
                _ => continue,
            };
            let first_char = line.as_bytes()[0];
            if first_char == b'\t' {
                tab_count += 1;
            } else if first_char == b' ' {
                space_count += 1;
                let width =
                    u32::try_from(first_non_ws).expect("leading space count should fit in u32");
                space_widths.push(width);
            }
        }

        if tab_count == 0 && space_count == 0 {
            return IndentStyle::Spaces(4);
        }

        if tab_count >= space_count {
            IndentStyle::Tabs
        } else {
            // Find GCD of all non-zero leading space counts to determine indent width.
            let gcd = space_widths.iter().copied().fold(0u32, gcd_u32);
            if gcd == 0 {
                IndentStyle::Spaces(4)
            } else {
                IndentStyle::Spaces(gcd)
            }
        }
    }

    /// Apply patches, update text and LineIndex, return RangeChanges for downstream consumers.
    pub fn apply_patches(
        &mut self,
        patches: &[TextPatch],
    ) -> Result<Vec<RangeChange>, TextPatchError> {
        let new_text = neco_textpatch::apply_patches(self.text(), patches)?;
        let range_changes = build_range_changes(patches);
        self.text = new_text;
        self.line_index = LineIndex::new(&self.text);
        Ok(range_changes)
    }

    /// Apply patches and propagate to all subsystems in order:
    /// 1. Record to history (before text change, needs original text for inverse patch computation)
    /// 2. Apply patches to text and rebuild LineIndex
    /// 3. Map decorations through changes
    /// 4. Update wrap map (only when both wrap_map and wrap_policy are Some)
    pub fn apply_patches_with(
        &mut self,
        patches: &[TextPatch],
        decorations: &mut DecorationSet,
        wrap_map: Option<&mut WrapMap>,
        wrap_policy: Option<&WrapPolicy>,
        history: Option<&mut EditHistory>,
        label: Option<&str>,
    ) -> Result<(), TextPatchError> {
        let old_line_index = if wrap_map.is_some() {
            Some(self.line_index.clone())
        } else {
            None
        };

        if let Some(history) = history {
            history.push_edit(label.unwrap_or(""), self.text(), patches.to_vec());
        }

        let range_changes = self.apply_patches(patches)?;
        decorations.map_through_changes(&range_changes);

        if let (Some(wrap_map), Some(wrap_policy)) = (wrap_map, wrap_policy) {
            update_wrap_map(
                wrap_map,
                wrap_policy,
                old_line_index
                    .as_ref()
                    .expect("old_line_index set when wrap_map is Some"),
                &self.text,
                &self.line_index,
                patches,
                &range_changes,
            );
        }

        Ok(())
    }
}

fn build_range_changes(patches: &[TextPatch]) -> Vec<RangeChange> {
    let mut ordered = patches.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|(left_index, left_patch), (right_index, right_patch)| {
        left_patch
            .start()
            .cmp(&right_patch.start())
            .then_with(|| left_patch.end().cmp(&right_patch.end()))
            .then_with(|| left_index.cmp(right_index))
    });

    let mut cumulative_delta = 0i64;
    let mut changes = Vec::with_capacity(ordered.len());

    for (_, patch) in ordered {
        let patch_start = i64::try_from(patch.start()).expect("patch start exceeds i64::MAX");
        let patch_end = i64::try_from(patch.end()).expect("patch end exceeds i64::MAX");
        let replacement_len =
            i64::try_from(patch.replacement().len()).expect("replacement len exceeds i64::MAX");

        let adjusted_start = usize::try_from(patch_start + cumulative_delta)
            .expect("validated patch start should stay non-negative");
        let adjusted_old_end = usize::try_from(patch_end + cumulative_delta)
            .expect("validated patch end should stay non-negative");
        let adjusted_new_end = adjusted_start
            .checked_add(usize::try_from(replacement_len).expect("replacement len exceeds usize"))
            .expect("range change new end overflow");

        changes.push(RangeChange::new(
            adjusted_start,
            adjusted_old_end,
            adjusted_new_end,
        ));

        cumulative_delta += replacement_len - (patch_end - patch_start);
    }

    changes
}

fn update_wrap_map(
    wrap_map: &mut WrapMap,
    wrap_policy: &WrapPolicy,
    old_line_index: &LineIndex,
    new_text: &str,
    new_line_index: &LineIndex,
    patches: &[TextPatch],
    range_changes: &[RangeChange],
) {
    if patches.is_empty() {
        return;
    }

    let start_offset = patches.iter().map(TextPatch::start).min().unwrap_or(0);
    let old_end_offset = patches
        .iter()
        .map(TextPatch::end)
        .max()
        .unwrap_or(start_offset);
    let new_end_offset = range_changes
        .iter()
        .map(RangeChange::new_end)
        .max()
        .unwrap_or(start_offset);

    let start_line = old_line_index
        .line_of_offset(start_offset)
        .expect("validated patch start should map to a line");
    let old_end_line = old_line_index
        .line_of_offset(old_end_offset)
        .expect("validated patch end should map to a line");
    let new_end_line = new_line_index
        .line_of_offset(new_end_offset)
        .expect("validated patch end should map to a line");

    let old_line_count = old_end_line - start_line + 1;
    let new_line_count = new_end_line - start_line + 1;

    if old_line_count == new_line_count {
        for line in start_line..=new_end_line {
            let line_text = line_text(new_text, new_line_index, line);
            wrap_map.rewrap_line(line, line_text, wrap_policy);
        }
        return;
    }

    // start_line is computed from old_line_index. Because patches are sorted
    // by start offset and applied front-to-back, no prior patch can shift the
    // line number of the earliest affected offset. start_line is therefore
    // valid in both old and new coordinate spaces.
    let new_lines = collect_line_texts(new_text, new_line_index, start_line, new_line_count);

    wrap_map.splice_lines(
        start_line,
        old_line_count,
        new_lines.into_iter(),
        wrap_policy,
    );
}

fn collect_line_texts<'a>(
    text: &'a str,
    line_index: &LineIndex,
    start_line: u32,
    line_count: u32,
) -> Vec<&'a str> {
    (start_line..start_line + line_count)
        .map(|line| line_text(text, line_index, line))
        .collect()
}

fn line_text<'a>(text: &'a str, line_index: &LineIndex, line: u32) -> &'a str {
    let range = line_index
        .line_range(line)
        .expect("line should be in range for wrap update");
    &text[range.start()..range.end()]
}

// ---------------------------------------------------------------------------
// Bracket matching
// ---------------------------------------------------------------------------

/// A matched pair of brackets at byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BracketPair {
    open: usize,
    close: usize,
}

impl BracketPair {
    pub fn open(&self) -> usize {
        self.open
    }

    pub fn close(&self) -> usize {
        self.close
    }
}

/// Returns the matching closing bracket for an opening bracket, or vice versa.
fn matching_bracket(ch: char) -> Option<char> {
    match ch {
        '(' => Some(')'),
        ')' => Some('('),
        '[' => Some(']'),
        ']' => Some('['),
        '{' => Some('}'),
        '}' => Some('{'),
        _ => None,
    }
}

/// Returns true if the character is an opening bracket.
fn is_opening_bracket(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{')
}

/// Finds the matching bracket for the bracket character at `offset`.
///
/// Supports `()`, `[]`, `{}`. Returns `None` if `offset` is not on a
/// bracket character, not on a valid char boundary, or no matching
/// bracket is found.
pub fn find_matching_bracket(text: &str, offset: usize) -> Option<BracketPair> {
    if offset >= text.len() || !text.is_char_boundary(offset) {
        return None;
    }

    let ch = text[offset..].chars().next()?;
    let target = matching_bracket(ch)?;

    if is_opening_bracket(ch) {
        // Scan forward
        let mut depth = 0usize;
        let mut pos = offset;
        for c in text[offset..].chars() {
            if c == ch {
                depth += 1;
            } else if c == target {
                depth -= 1;
                if depth == 0 {
                    return Some(BracketPair {
                        open: offset,
                        close: pos,
                    });
                }
            }
            pos += c.len_utf8();
        }
        None
    } else {
        // Closing bracket: scan backward
        let mut depth = 0usize;
        for (byte_pos, c) in text[..offset + ch.len_utf8()].char_indices().rev() {
            if c == ch {
                depth += 1;
            } else if c == target {
                depth -= 1;
                if depth == 0 {
                    return Some(BracketPair {
                        open: byte_pos,
                        close: offset,
                    });
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Auto-indent
// ---------------------------------------------------------------------------

/// Returns the leading whitespace of the line containing `offset`.
///
/// Used by the UI layer to replicate the current line's indentation on Enter.
pub fn auto_indent(text: &str, line_index: &neco_textview::LineIndex, offset: usize) -> String {
    let line = match line_index.line_of_offset(offset) {
        Ok(l) => l,
        Err(_) => return String::new(),
    };
    let range = match line_index.line_range(line) {
        Ok(r) => r,
        Err(_) => return String::new(),
    };
    let line_text = &text[range.start()..range.end()];
    let indent_len = line_text
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .map(|c| c.len_utf8())
        .sum::<usize>();
    line_text[..indent_len].to_string()
}

// ---------------------------------------------------------------------------
// Auto close bracket
// ---------------------------------------------------------------------------

/// Returns the closing counterpart for an opening bracket or quote character.
///
/// Supports `()`, `[]`, `{}`, `""`, `''`.
pub fn auto_close_bracket(ch: char) -> Option<char> {
    match ch {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

fn gcd_u32(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd_u32(b, a % b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use neco_decor::Decoration;
    use neco_textpatch::apply_patches;

    #[test]
    fn new_exposes_text_and_line_index() {
        let buffer = EditorBuffer::new("alpha\nbeta".to_string());

        assert_eq!(buffer.text(), "alpha\nbeta");
        assert_eq!(buffer.line_index().line_count(), 2);
        assert_eq!(buffer.line_index().text_len(), 10);
    }

    #[test]
    fn apply_patches_updates_text_and_returns_single_range_change() {
        let mut buffer = EditorBuffer::new("hello world".to_string());
        let patches = [TextPatch::replace(6, 11, "rust").unwrap()];

        let changes = buffer.apply_patches(&patches).unwrap();

        assert_eq!(buffer.text(), "hello rust");
        assert_eq!(changes, vec![RangeChange::new(6, 11, 10)]);
        assert_eq!(buffer.line_index().text_len(), 10);
    }

    #[test]
    fn apply_patches_uses_cumulative_delta_for_following_changes() {
        let mut buffer = EditorBuffer::new("abcdef".to_string());
        let patches = [
            TextPatch::replace(1, 3, "WXYZ").unwrap(),
            TextPatch::replace(4, 6, "Q").unwrap(),
        ];

        let changes = buffer.apply_patches(&patches).unwrap();

        assert_eq!(buffer.text(), "aWXYZdQ");
        assert_eq!(
            changes,
            vec![RangeChange::new(1, 3, 5), RangeChange::new(6, 8, 7)]
        );
    }

    #[test]
    fn apply_patches_returns_error_for_invalid_patch() {
        let mut buffer = EditorBuffer::new("abc".to_string());
        let patches = [TextPatch::replace(4, 4, "x").unwrap()];

        let error = buffer.apply_patches(&patches).unwrap_err();

        assert_eq!(
            error,
            TextPatchError::OffsetOutOfBounds { offset: 4, len: 3 }
        );
    }

    #[test]
    fn apply_patches_with_maps_decorations_through_changes() {
        let mut buffer = EditorBuffer::new("hello world".to_string());
        let patches = [TextPatch::replace(6, 11, "rust").unwrap()];
        let mut decorations = DecorationSet::new();
        decorations.add(Decoration::highlight(6, 11, 1).unwrap());

        buffer
            .apply_patches_with(&patches, &mut decorations, None, None, None, None)
            .unwrap();

        let decoration = decorations.iter().next().unwrap().1;
        assert_eq!(decoration.start(), 6);
        assert_eq!(decoration.end(), 10);
    }

    #[test]
    fn apply_patches_with_updates_wrap_map() {
        let mut buffer = EditorBuffer::new("ab cd\nef gh".to_string());
        let patches = [TextPatch::replace(0, 5, "abcd").unwrap()];
        let policy = WrapPolicy::code();
        let mut decorations = DecorationSet::new();
        let mut wrap_map = WrapMap::new(buffer.text().split('\n'), 3, &policy);

        assert_eq!(wrap_map.visual_line_count(0), 2);

        buffer
            .apply_patches_with(
                &patches,
                &mut decorations,
                Some(&mut wrap_map),
                Some(&policy),
                None,
                None,
            )
            .unwrap();

        assert_eq!(wrap_map.visual_line_count(0), 1);
        assert_eq!(wrap_map.wrap_points(0), &[]);
    }

    #[test]
    fn apply_patches_with_records_history_and_undo_restores_original_text() {
        let mut buffer = EditorBuffer::new("hello world".to_string());
        let patches = [TextPatch::replace(6, 11, "rust").unwrap()];
        let mut decorations = DecorationSet::new();
        let mut history = EditHistory::new(buffer.text());

        buffer
            .apply_patches_with(
                &patches,
                &mut decorations,
                None,
                None,
                Some(&mut history),
                Some("replace word"),
            )
            .unwrap();

        assert_eq!(history.current_label(), "replace word");

        let undo = history.undo().unwrap().remove(0);
        let inverse = undo.inverse_patches.unwrap();
        let restored = apply_patches(buffer.text(), &inverse).unwrap();

        assert_eq!(restored, "hello world");
    }

    #[test]
    fn apply_patches_with_works_when_all_optional_systems_are_absent() {
        let mut buffer = EditorBuffer::new("hello".to_string());
        let patches = [TextPatch::insert(5, "!")];
        let mut decorations = DecorationSet::new();

        buffer
            .apply_patches_with(&patches, &mut decorations, None, None, None, None)
            .unwrap();

        assert_eq!(buffer.text(), "hello!");
    }

    #[test]
    fn re_exports_are_available() {
        let _ = neco_textview::LineIndex::new("text");
        let _ = neco_textpatch::TextPatch::insert(0, "x");
        let _ = neco_decor::DecorationSet::new();
        let _ = neco_diffcore::diff("a", "b");
        let _ = neco_wrap::WrapPolicy::code();
        let _ = neco_history::EditHistory::new("");
        let _ = neco_pathrel::PathPolicy::posix();
        let _ = neco_filetree::FileTreeNode {
            name: "a".to_string(),
            path: "/a".to_string(),
            kind: neco_filetree::FileTreeNodeKind::File,
            children: Vec::new(),
            materialization: neco_filetree::DirectoryMaterialization::Complete,
            child_count: None,
        };
        let _ = neco_watchnorm::RawWatchKind::Create;
        let _ = neco_tree::Tree::new(0usize);
        let _ = RangeChange::new(0, 0, 0);
    }

    #[test]
    fn detect_indent_tabs() {
        let buffer = EditorBuffer::new("\tline1\n\t\tline2\nline3\n".to_string());
        assert_eq!(buffer.detect_indent(10), IndentStyle::Tabs);
    }

    #[test]
    fn detect_indent_two_spaces() {
        let buffer = EditorBuffer::new("def foo\n  bar\n  baz\n    qux\n".to_string());
        assert_eq!(buffer.detect_indent(10), IndentStyle::Spaces(2));
    }

    #[test]
    fn detect_indent_four_spaces() {
        let buffer = EditorBuffer::new(
            "fn main() {\n    let x = 1;\n    let y = 2;\n        nested();\n}\n".to_string(),
        );
        assert_eq!(buffer.detect_indent(10), IndentStyle::Spaces(4));
    }

    #[test]
    fn detect_indent_mixed_prefers_majority() {
        // More tab-indented lines than space-indented
        let buffer = EditorBuffer::new("\ta\n\tb\n\tc\n  d\n".to_string());
        assert_eq!(buffer.detect_indent(10), IndentStyle::Tabs);
    }

    #[test]
    fn detect_indent_empty_text() {
        let buffer = EditorBuffer::new(String::new());
        assert_eq!(buffer.detect_indent(10), IndentStyle::Spaces(4));
    }

    #[test]
    fn detect_indent_no_indentation() {
        let buffer = EditorBuffer::new("line1\nline2\nline3\n".to_string());
        assert_eq!(buffer.detect_indent(10), IndentStyle::Spaces(4));
    }

    #[test]
    fn detect_indent_respects_sample_lines_limit() {
        // First 2 lines use tabs, remaining use spaces
        let buffer = EditorBuffer::new("\ta\n\tb\n  c\n  d\n  e\n  f\n".to_string());
        assert_eq!(buffer.detect_indent(2), IndentStyle::Tabs);
    }

    // -- Bracket matching ----------------------------------------------------

    #[test]
    fn find_matching_bracket_simple_parens() {
        let text = "(hello)";
        let pair = find_matching_bracket(text, 0).unwrap();
        assert_eq!(pair.open(), 0);
        assert_eq!(pair.close(), 6);
    }

    #[test]
    fn find_matching_bracket_from_close() {
        let text = "(hello)";
        let pair = find_matching_bracket(text, 6).unwrap();
        assert_eq!(pair.open(), 0);
        assert_eq!(pair.close(), 6);
    }

    #[test]
    fn find_matching_bracket_nested() {
        let text = "((a))";
        let pair = find_matching_bracket(text, 1).unwrap();
        assert_eq!(pair.open(), 1);
        assert_eq!(pair.close(), 3);
        let pair = find_matching_bracket(text, 0).unwrap();
        assert_eq!(pair.open(), 0);
        assert_eq!(pair.close(), 4);
    }

    #[test]
    fn find_matching_bracket_mixed_types() {
        let text = "{[()]}";
        let pair = find_matching_bracket(text, 0).unwrap();
        assert_eq!(pair.open(), 0);
        assert_eq!(pair.close(), 5);
        let pair = find_matching_bracket(text, 1).unwrap();
        assert_eq!(pair.open(), 1);
        assert_eq!(pair.close(), 4);
        let pair = find_matching_bracket(text, 2).unwrap();
        assert_eq!(pair.open(), 2);
        assert_eq!(pair.close(), 3);
    }

    #[test]
    fn find_matching_bracket_not_on_bracket() {
        assert!(find_matching_bracket("hello", 0).is_none());
    }

    #[test]
    fn find_matching_bracket_unmatched() {
        assert!(find_matching_bracket("(hello", 0).is_none());
        assert!(find_matching_bracket("hello)", 5).is_none());
    }

    #[test]
    fn find_matching_bracket_empty_text() {
        assert!(find_matching_bracket("", 0).is_none());
    }

    // -- Auto-indent ---------------------------------------------------------

    #[test]
    fn auto_indent_preserves_spaces() {
        let text = "    hello\n    world";
        let li = neco_textview::LineIndex::new(text);
        assert_eq!(auto_indent(text, &li, 0), "    ");
        assert_eq!(auto_indent(text, &li, 10), "    ");
    }

    #[test]
    fn auto_indent_preserves_tabs() {
        let text = "\thello\n\t\tworld";
        let li = neco_textview::LineIndex::new(text);
        assert_eq!(auto_indent(text, &li, 0), "\t");
        assert_eq!(auto_indent(text, &li, 7), "\t\t");
    }

    #[test]
    fn auto_indent_no_indent_returns_empty() {
        let text = "hello\nworld";
        let li = neco_textview::LineIndex::new(text);
        assert_eq!(auto_indent(text, &li, 0), "");
    }

    #[test]
    fn auto_indent_empty_text() {
        let text = "";
        let li = neco_textview::LineIndex::new(text);
        assert_eq!(auto_indent(text, &li, 0), "");
    }

    // -- Auto close bracket --------------------------------------------------

    #[test]
    fn auto_close_bracket_pairs() {
        assert_eq!(auto_close_bracket('('), Some(')'));
        assert_eq!(auto_close_bracket('['), Some(']'));
        assert_eq!(auto_close_bracket('{'), Some('}'));
        assert_eq!(auto_close_bracket('"'), Some('"'));
        assert_eq!(auto_close_bracket('\''), Some('\''));
    }

    #[test]
    fn auto_close_bracket_non_bracket() {
        assert_eq!(auto_close_bracket('a'), None);
        assert_eq!(auto_close_bracket(')'), None);
        assert_eq!(auto_close_bracket(']'), None);
    }

    #[test]
    fn find_matching_bracket_offset_out_of_bounds() {
        assert!(find_matching_bracket("()", 5).is_none());
    }
}
