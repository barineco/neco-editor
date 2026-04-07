//! Text position primitives: line/column ↔ byte offset conversion, UTF-16 mapping,
//! and selection/caret model.

use std::fmt;

/// Errors returned by text view operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextViewError {
    InvalidRange { start: usize, end: usize },
    OffsetOutOfBounds { offset: usize, len: usize },
    InvalidUtf8Boundary { offset: usize },
    LineOutOfBounds { line: u32, line_count: u32 },
    Utf16OffsetOutOfBounds { offset: usize, total: usize },
}

impl fmt::Display for TextViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange { start, end } => {
                write!(f, "invalid range: start {start} > end {end}")
            }
            Self::OffsetOutOfBounds { offset, len } => {
                write!(f, "offset {offset} out of bounds (len {len})")
            }
            Self::InvalidUtf8Boundary { offset } => {
                write!(f, "offset {offset} is not on a UTF-8 char boundary")
            }
            Self::LineOutOfBounds { line, line_count } => {
                write!(f, "line {line} out of bounds (line_count {line_count})")
            }
            Self::Utf16OffsetOutOfBounds { offset, total } => {
                write!(f, "UTF-16 offset {offset} out of bounds (total {total})")
            }
        }
    }
}

impl std::error::Error for TextViewError {}

/// 0-based line and column position in a text document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    line: u32,
    column: u32,
}

impl Position {
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    pub const fn line(&self) -> u32 {
        self.line
    }

    pub const fn column(&self) -> u32 {
        self.column
    }
}

/// Byte offset range in a UTF-8 string. `start <= end` is enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
    start: usize,
    end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Result<Self, TextViewError> {
        if start > end {
            return Err(TextViewError::InvalidRange { start, end });
        }
        Ok(Self { start, end })
    }

    pub fn empty(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }

    pub fn intersects(&self, other: &TextRange) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Abstract description of one text range change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeChange {
    start: usize,
    old_end: usize,
    new_end: usize,
}

impl RangeChange {
    pub const fn new(start: usize, old_end: usize, new_end: usize) -> Self {
        Self {
            start,
            old_end,
            new_end,
        }
    }

    pub const fn start(&self) -> usize {
        self.start
    }

    pub const fn old_end(&self) -> usize {
        self.old_end
    }

    pub const fn new_end(&self) -> usize {
        self.new_end
    }
}

/// Directional selection with anchor and head (caret position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    anchor: usize,
    head: usize,
}

impl Selection {
    pub const fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    pub const fn cursor(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    pub const fn anchor(&self) -> usize {
        self.anchor
    }

    pub const fn head(&self) -> usize {
        self.head
    }

    /// Return the normalized range (start <= end) covered by this selection.
    pub fn range(&self) -> TextRange {
        if self.anchor <= self.head {
            TextRange {
                start: self.anchor,
                end: self.head,
            }
        } else {
            TextRange {
                start: self.head,
                end: self.anchor,
            }
        }
    }

    pub fn is_cursor(&self) -> bool {
        self.anchor == self.head
    }

    pub fn is_forward(&self) -> bool {
        self.anchor <= self.head
    }
}

/// Precomputed line-start offset table for O(log n) line/column ↔ byte offset conversion.
#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Vec<u32>,
    len: u32,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let len = u32::try_from(text.len()).expect("text length exceeds u32::MAX");
        let mut line_starts = vec![0u32];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                let next = u32::try_from(i + 1).expect("offset exceeds u32::MAX");
                line_starts.push(next);
            }
        }
        Self { line_starts, len }
    }

    pub fn line_count(&self) -> u32 {
        u32::try_from(self.line_starts.len()).expect("line count exceeds u32::MAX")
    }

    pub fn text_len(&self) -> u32 {
        self.len
    }

    /// Convert a byte offset to a 0-based line/column position.
    pub fn offset_to_position(&self, text: &str, offset: usize) -> Result<Position, TextViewError> {
        let len = self.len as usize;
        if offset > len {
            return Err(TextViewError::OffsetOutOfBounds { offset, len });
        }
        if offset < len && !text.is_char_boundary(offset) {
            return Err(TextViewError::InvalidUtf8Boundary { offset });
        }
        let line_idx = self.line_of_offset(offset)?;
        let line_start = self.line_starts[line_idx as usize] as usize;
        let column = u32::try_from(offset - line_start).expect("column exceeds u32::MAX");
        Ok(Position::new(line_idx, column))
    }

    /// Convert a 0-based line/column position to a byte offset.
    pub fn position_to_offset(
        &self,
        text: &str,
        position: Position,
    ) -> Result<usize, TextViewError> {
        let line = position.line();
        let lc = self.line_count();
        if line >= lc {
            return Err(TextViewError::LineOutOfBounds {
                line,
                line_count: lc,
            });
        }
        let line_start = self.line_starts[line as usize] as usize;
        let line_end = if line + 1 < lc {
            self.line_starts[(line + 1) as usize] as usize
        } else {
            self.len as usize
        };
        let col = position.column() as usize;
        let offset = line_start + col;
        if offset > line_end {
            return Err(TextViewError::OffsetOutOfBounds {
                offset,
                len: self.len as usize,
            });
        }
        if offset < text.len() && !text.is_char_boundary(offset) {
            return Err(TextViewError::InvalidUtf8Boundary { offset });
        }
        Ok(offset)
    }

    /// Return the byte range of a line excluding its trailing newline.
    pub fn line_range(&self, line: u32) -> Result<TextRange, TextViewError> {
        let lc = self.line_count();
        if line >= lc {
            return Err(TextViewError::LineOutOfBounds {
                line,
                line_count: lc,
            });
        }
        let start = self.line_starts[line as usize] as usize;
        let end_with_nl = if line + 1 < lc {
            self.line_starts[(line + 1) as usize] as usize
        } else {
            self.len as usize
        };
        let end = if end_with_nl > start && line + 1 < lc {
            end_with_nl - 1
        } else {
            end_with_nl
        };
        Ok(TextRange { start, end })
    }

    /// Return the byte range of a line including its trailing newline.
    pub fn line_range_with_newline(&self, line: u32) -> Result<TextRange, TextViewError> {
        let lc = self.line_count();
        if line >= lc {
            return Err(TextViewError::LineOutOfBounds {
                line,
                line_count: lc,
            });
        }
        let start = self.line_starts[line as usize] as usize;
        let end = if line + 1 < lc {
            self.line_starts[(line + 1) as usize] as usize
        } else {
            self.len as usize
        };
        Ok(TextRange { start, end })
    }

    /// Return the 0-based line number that contains the given byte offset.
    pub fn line_of_offset(&self, offset: usize) -> Result<u32, TextViewError> {
        let len = self.len as usize;
        if offset > len {
            return Err(TextViewError::OffsetOutOfBounds { offset, len });
        }
        let idx = self
            .line_starts
            .partition_point(|&s| (s as usize) <= offset);
        let line = if idx == 0 { 0 } else { idx - 1 };
        Ok(u32::try_from(line).expect("line index exceeds u32::MAX"))
    }
}

#[derive(Debug, Clone)]
struct Utf16Anchor {
    byte_offset: u32,
    utf16_offset: u32,
    byte_len: u8,
    utf16_len: u8,
}

/// Bidirectional UTF-8 byte offset ↔ UTF-16 code unit offset mapping.
#[derive(Debug, Clone)]
pub struct Utf16Mapping {
    anchors: Vec<Utf16Anchor>,
    total_bytes: u32,
    total_utf16: u32,
}

impl Utf16Mapping {
    pub fn new(text: &str) -> Self {
        let mut anchors = Vec::new();
        let mut byte_off: u32 = 0;
        let mut utf16_off: u32 = 0;

        for ch in text.chars() {
            let byte_len = ch.len_utf8();
            let utf16_len = ch.len_utf16();

            if byte_len != 1 {
                anchors.push(Utf16Anchor {
                    byte_offset: byte_off,
                    utf16_offset: utf16_off,
                    byte_len: u8::try_from(byte_len).expect("char byte len exceeds u8"),
                    utf16_len: u8::try_from(utf16_len).expect("char utf16 len exceeds u8"),
                });
            }

            byte_off += u32::try_from(byte_len).expect("byte offset exceeds u32");
            utf16_off += u32::try_from(utf16_len).expect("utf16 offset exceeds u32");
        }

        Self {
            anchors,
            total_bytes: byte_off,
            total_utf16: utf16_off,
        }
    }

    /// Convert a UTF-8 byte offset to a UTF-16 code unit offset.
    pub fn byte_to_utf16(&self, byte_offset: usize) -> Result<usize, TextViewError> {
        let total = self.total_bytes as usize;
        if byte_offset > total {
            return Err(TextViewError::OffsetOutOfBounds {
                offset: byte_offset,
                len: total,
            });
        }

        if self.anchors.is_empty() {
            return Ok(byte_offset);
        }

        let idx = self
            .anchors
            .partition_point(|a| (a.byte_offset as usize) <= byte_offset);

        if idx == 0 {
            return Ok(byte_offset);
        }

        let anchor = &self.anchors[idx - 1];
        let ab = anchor.byte_offset as usize;
        let au = anchor.utf16_offset as usize;
        let blen = anchor.byte_len as usize;
        let ulen = anchor.utf16_len as usize;

        if byte_offset > ab && byte_offset < ab + blen {
            return Err(TextViewError::InvalidUtf8Boundary {
                offset: byte_offset,
            });
        }

        if byte_offset == ab {
            return Ok(au);
        }

        // Anchors record only non-ASCII chars, so bytes between anchors are ASCII
        let ascii_past = byte_offset - (ab + blen);
        Ok(au + ulen + ascii_past)
    }

    /// Convert a UTF-16 code unit offset to a UTF-8 byte offset.
    pub fn utf16_to_byte(&self, utf16_offset: usize) -> Result<usize, TextViewError> {
        let total = self.total_utf16 as usize;
        if utf16_offset > total {
            return Err(TextViewError::Utf16OffsetOutOfBounds {
                offset: utf16_offset,
                total,
            });
        }

        if self.anchors.is_empty() {
            return Ok(utf16_offset);
        }

        let idx = self
            .anchors
            .partition_point(|a| (a.utf16_offset as usize) <= utf16_offset);

        if idx == 0 {
            return Ok(utf16_offset);
        }

        let anchor = &self.anchors[idx - 1];
        let ab = anchor.byte_offset as usize;
        let au = anchor.utf16_offset as usize;
        let blen = anchor.byte_len as usize;
        let ulen = anchor.utf16_len as usize;

        if utf16_offset > au && utf16_offset < au + ulen {
            return Err(TextViewError::Utf16OffsetOutOfBounds {
                offset: utf16_offset,
                total,
            });
        }

        if utf16_offset == au {
            return Ok(ab);
        }

        let ascii_past = utf16_offset - (au + ulen);
        Ok(ab + blen + ascii_past)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_new() {
        let p = Position::new(3, 7);
        assert_eq!(p.line(), 3);
        assert_eq!(p.column(), 7);
    }

    #[test]
    fn text_range_new_ok() {
        let r = TextRange::new(2, 5).unwrap();
        assert_eq!(r.start(), 2);
        assert_eq!(r.end(), 5);
        assert_eq!(r.len(), 3);
        assert!(!r.is_empty());
    }

    #[test]
    fn text_range_new_reversed() {
        let err = TextRange::new(5, 2).unwrap_err();
        assert_eq!(err, TextViewError::InvalidRange { start: 5, end: 2 });
    }

    #[test]
    fn text_range_empty() {
        let r = TextRange::empty(10);
        assert_eq!(r.start(), 10);
        assert_eq!(r.end(), 10);
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn text_range_contains() {
        let r = TextRange::new(2, 5).unwrap();
        assert!(r.contains(2));
        assert!(r.contains(4));
        assert!(!r.contains(5));
        assert!(!r.contains(1));
    }

    #[test]
    fn text_range_intersects() {
        let a = TextRange::new(2, 5).unwrap();
        let b = TextRange::new(4, 8).unwrap();
        let c = TextRange::new(5, 8).unwrap();
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
        let e1 = TextRange::empty(3);
        let e2 = TextRange::empty(3);
        assert!(!e1.intersects(&e2));
    }

    #[test]
    fn range_change_new() {
        let change = RangeChange::new(2, 5, 8);
        assert_eq!(change.start(), 2);
        assert_eq!(change.old_end(), 5);
        assert_eq!(change.new_end(), 8);
    }

    #[test]
    fn range_change_start() {
        let change = RangeChange::new(3, 7, 9);
        assert_eq!(change.start(), 3);
    }

    #[test]
    fn range_change_old_end() {
        let change = RangeChange::new(3, 7, 9);
        assert_eq!(change.old_end(), 7);
    }

    #[test]
    fn range_change_new_end() {
        let change = RangeChange::new(3, 7, 9);
        assert_eq!(change.new_end(), 9);
    }

    #[test]
    fn selection_cursor() {
        let s = Selection::cursor(5);
        assert!(s.is_cursor());
        assert_eq!(s.anchor(), 5);
        assert_eq!(s.head(), 5);
        assert!(s.is_forward());
        let r = s.range();
        assert!(r.is_empty());
    }

    #[test]
    fn selection_forward() {
        let s = Selection::new(2, 8);
        assert!(!s.is_cursor());
        assert!(s.is_forward());
        let r = s.range();
        assert_eq!(r.start(), 2);
        assert_eq!(r.end(), 8);
    }

    #[test]
    fn selection_backward() {
        let s = Selection::new(8, 2);
        assert!(!s.is_cursor());
        assert!(!s.is_forward());
        let r = s.range();
        assert_eq!(r.start(), 2);
        assert_eq!(r.end(), 8);
    }

    #[test]
    fn line_index_empty_text() {
        let text = "";
        let idx = LineIndex::new(text);
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.text_len(), 0);

        let pos = idx.offset_to_position(text, 0).unwrap();
        assert_eq!(pos, Position::new(0, 0));

        let off = idx.position_to_offset(text, Position::new(0, 0)).unwrap();
        assert_eq!(off, 0);
    }

    #[test]
    fn line_index_single_line() {
        let text = "hello";
        let idx = LineIndex::new(text);
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.text_len(), 5);

        let pos = idx.offset_to_position(text, 3).unwrap();
        assert_eq!(pos, Position::new(0, 3));

        let off = idx.position_to_offset(text, pos).unwrap();
        assert_eq!(off, 3);

        let pos_end = idx.offset_to_position(text, 5).unwrap();
        assert_eq!(pos_end, Position::new(0, 5));
    }

    #[test]
    fn line_index_multi_line() {
        let text = "abc\ndef\nghi";
        let idx = LineIndex::new(text);
        assert_eq!(idx.line_count(), 3);
        assert_eq!(idx.text_len(), 11);

        let pos = idx.offset_to_position(text, 4).unwrap();
        assert_eq!(pos, Position::new(1, 0));
        let off = idx.position_to_offset(text, pos).unwrap();
        assert_eq!(off, 4);

        let pos2 = idx.offset_to_position(text, 9).unwrap();
        assert_eq!(pos2, Position::new(2, 1));
        let off2 = idx.position_to_offset(text, pos2).unwrap();
        assert_eq!(off2, 9);
    }

    #[test]
    fn line_index_multibyte() {
        let text = "あいう\nえお";
        let idx = LineIndex::new(text);
        assert_eq!(idx.line_count(), 2);

        let pos = idx.offset_to_position(text, 10).unwrap();
        assert_eq!(pos, Position::new(1, 0));
        let off = idx.position_to_offset(text, pos).unwrap();
        assert_eq!(off, 10);

        let pos2 = idx.offset_to_position(text, 3).unwrap();
        assert_eq!(pos2, Position::new(0, 3));
        let off2 = idx.position_to_offset(text, pos2).unwrap();
        assert_eq!(off2, 3);
    }

    #[test]
    fn line_index_roundtrip() {
        let text = "hello\nworld\n";
        let idx = LineIndex::new(text);

        for offset in 0..=text.len() {
            if text.is_char_boundary(offset) {
                let pos = idx.offset_to_position(text, offset).unwrap();
                let back = idx.position_to_offset(text, pos).unwrap();
                assert_eq!(back, offset, "roundtrip failed at offset {offset}");
            }
        }
    }

    #[test]
    fn line_index_offset_out_of_bounds() {
        let text = "abc";
        let idx = LineIndex::new(text);
        let err = idx.offset_to_position(text, 10).unwrap_err();
        assert_eq!(err, TextViewError::OffsetOutOfBounds { offset: 10, len: 3 });
    }

    #[test]
    fn line_index_line_out_of_bounds() {
        let text = "abc\ndef";
        let idx = LineIndex::new(text);
        let err = idx.line_range(5).unwrap_err();
        assert_eq!(
            err,
            TextViewError::LineOutOfBounds {
                line: 5,
                line_count: 2
            }
        );
    }

    #[test]
    fn line_index_invalid_utf8_boundary() {
        let text = "あ";
        let idx = LineIndex::new(text);
        let err = idx.offset_to_position(text, 1).unwrap_err();
        assert_eq!(err, TextViewError::InvalidUtf8Boundary { offset: 1 });
    }

    #[test]
    fn line_index_line_range() {
        let text = "abc\ndef\n";
        let idx = LineIndex::new(text);

        let r0 = idx.line_range(0).unwrap();
        assert_eq!(r0.start(), 0);
        assert_eq!(r0.end(), 3);

        let r1 = idx.line_range(1).unwrap();
        assert_eq!(r1.start(), 4);
        assert_eq!(r1.end(), 7);

        let r2 = idx.line_range(2).unwrap();
        assert_eq!(r2.start(), 8);
        assert_eq!(r2.end(), 8);

        let rn0 = idx.line_range_with_newline(0).unwrap();
        assert_eq!(rn0.start(), 0);
        assert_eq!(rn0.end(), 4);

        let rn1 = idx.line_range_with_newline(1).unwrap();
        assert_eq!(rn1.start(), 4);
        assert_eq!(rn1.end(), 8);
    }

    #[test]
    fn line_index_line_of_offset() {
        let text = "abc\ndef\nghi";
        let idx = LineIndex::new(text);
        assert_eq!(idx.line_of_offset(0).unwrap(), 0);
        assert_eq!(idx.line_of_offset(3).unwrap(), 0);
        assert_eq!(idx.line_of_offset(4).unwrap(), 1);
        assert_eq!(idx.line_of_offset(8).unwrap(), 2);
        assert_eq!(idx.line_of_offset(11).unwrap(), 2);
    }

    #[test]
    fn utf16_mapping_ascii() {
        let text = "hello world";
        let m = Utf16Mapping::new(text);

        for i in 0..=text.len() {
            assert_eq!(m.byte_to_utf16(i).unwrap(), i);
            assert_eq!(m.utf16_to_byte(i).unwrap(), i);
        }
    }

    #[test]
    fn utf16_mapping_japanese() {
        let text = "aあb";
        let m = Utf16Mapping::new(text);

        assert_eq!(m.byte_to_utf16(0).unwrap(), 0);
        assert_eq!(m.byte_to_utf16(1).unwrap(), 1);
        assert!(m.byte_to_utf16(2).is_err());
        assert!(m.byte_to_utf16(3).is_err());
        assert_eq!(m.byte_to_utf16(4).unwrap(), 2);
        assert_eq!(m.byte_to_utf16(5).unwrap(), 3);

        assert_eq!(m.utf16_to_byte(0).unwrap(), 0);
        assert_eq!(m.utf16_to_byte(1).unwrap(), 1);
        assert_eq!(m.utf16_to_byte(2).unwrap(), 4);
        assert_eq!(m.utf16_to_byte(3).unwrap(), 5);
    }

    #[test]
    fn utf16_mapping_surrogate_pair() {
        let text = "a😀b";
        let m = Utf16Mapping::new(text);

        assert_eq!(m.byte_to_utf16(0).unwrap(), 0);
        assert_eq!(m.byte_to_utf16(1).unwrap(), 1);
        assert!(m.byte_to_utf16(2).is_err());
        assert!(m.byte_to_utf16(3).is_err());
        assert!(m.byte_to_utf16(4).is_err());
        assert_eq!(m.byte_to_utf16(5).unwrap(), 3);
        assert_eq!(m.byte_to_utf16(6).unwrap(), 4);

        assert_eq!(m.utf16_to_byte(0).unwrap(), 0);
        assert_eq!(m.utf16_to_byte(1).unwrap(), 1);
        assert!(m.utf16_to_byte(2).is_err());
        assert_eq!(m.utf16_to_byte(3).unwrap(), 5);
        assert_eq!(m.utf16_to_byte(4).unwrap(), 6);
    }

    #[test]
    fn utf16_mapping_roundtrip() {
        let text = "Hello あいう 😀🎉 world";
        let m = Utf16Mapping::new(text);

        let mut byte_off = 0;
        for ch in text.chars() {
            let u16_off = m.byte_to_utf16(byte_off).unwrap();
            let back = m.utf16_to_byte(u16_off).unwrap();
            assert_eq!(back, byte_off, "roundtrip failed at byte {byte_off}");
            byte_off += ch.len_utf8();
        }
        let u16_end = m.byte_to_utf16(byte_off).unwrap();
        let back_end = m.utf16_to_byte(u16_end).unwrap();
        assert_eq!(back_end, byte_off);
    }

    #[test]
    fn utf16_mapping_out_of_bounds() {
        let text = "abc";
        let m = Utf16Mapping::new(text);

        assert!(m.byte_to_utf16(10).is_err());
        assert!(m.utf16_to_byte(10).is_err());
    }

    #[test]
    fn error_display() {
        let e = TextViewError::InvalidRange { start: 5, end: 2 };
        let s = e.to_string();
        assert!(s.contains("5"));
        assert!(s.contains("2"));
    }
}
