//! Decoration data model for text editor highlight, marker, and widget ranges.

use std::fmt;

pub use neco_textview::RangeChange;

/// Errors returned by decoration operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecorError {
    InvalidRange { start: usize, end: usize },
    EmptyHighlight { offset: usize },
}

impl fmt::Display for DecorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecorError::InvalidRange { start, end } => {
                write!(f, "invalid range: start {start} > end {end}")
            }
            DecorError::EmptyHighlight { offset } => {
                write!(f, "empty highlight at offset {offset}")
            }
        }
    }
}

impl std::error::Error for DecorError {}

/// Classification of a decoration range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecorationKind {
    Highlight,
    Marker,
    Widget { block: bool },
}

/// One decoration instance with range, kind, tag, and stacking priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoration {
    start: usize,
    end: usize,
    kind: DecorationKind,
    tag: u32,
    priority: i16,
}

impl Decoration {
    pub fn highlight(start: usize, end: usize, tag: u32) -> Result<Self, DecorError> {
        if start > end {
            return Err(DecorError::InvalidRange { start, end });
        }
        if start == end {
            return Err(DecorError::EmptyHighlight { offset: start });
        }
        Ok(Self {
            start,
            end,
            kind: DecorationKind::Highlight,
            tag,
            priority: 0,
        })
    }

    pub fn marker(line_start: usize, tag: u32) -> Self {
        Self {
            start: line_start,
            end: line_start,
            kind: DecorationKind::Marker,
            tag,
            priority: 0,
        }
    }

    pub fn widget(start: usize, end: usize, tag: u32, block: bool) -> Result<Self, DecorError> {
        if start > end {
            return Err(DecorError::InvalidRange { start, end });
        }
        Ok(Self {
            start,
            end,
            kind: DecorationKind::Widget { block },
            tag,
            priority: 0,
        })
    }

    pub fn with_priority(mut self, priority: i16) -> Self {
        self.priority = priority;
        self
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn kind(&self) -> DecorationKind {
        self.kind
    }

    pub fn tag(&self) -> u32 {
        self.tag
    }

    pub fn priority(&self) -> i16 {
        self.priority
    }
}

/// Stable identifier assigned when a decoration is added to a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecorationId(u64);

impl DecorationId {
    /// Returns the underlying raw identifier for external boundaries.
    pub fn into_raw(self) -> u64 {
        self.0
    }

    /// Reconstructs a decoration identifier from a raw value.
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

#[derive(Debug, Clone)]
struct DecorationEntry {
    id: DecorationId,
    decoration: Decoration,
}

/// Sorted container of decorations with range query and text-change tracking.
#[derive(Debug, Clone)]
pub struct DecorationSet {
    entries: Vec<DecorationEntry>,
    next_id: u64,
}

impl DecorationSet {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 0,
        }
    }

    pub fn add(&mut self, decoration: Decoration) -> DecorationId {
        let id = DecorationId(self.next_id);
        self.next_id += 1;
        let pos = self
            .entries
            .partition_point(|e| e.decoration.start <= decoration.start);
        self.entries.push(DecorationEntry { id, decoration });
        let len = self.entries.len();
        self.entries[pos..len].rotate_right(1);
        id
    }

    pub fn remove(&mut self, id: DecorationId) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn query_range(&self, start: usize, end: usize) -> Vec<(DecorationId, &Decoration)> {
        self.entries
            .iter()
            .filter(|e| {
                let d = &e.decoration;
                if d.start == d.end {
                    d.start >= start && d.start < end
                } else {
                    d.start < end && d.end > start
                }
            })
            .map(|e| (e.id, &e.decoration))
            .collect()
    }

    pub fn query_tag(&self, tag: u32) -> Vec<(DecorationId, &Decoration)> {
        self.entries
            .iter()
            .filter(|e| e.decoration.tag == tag)
            .map(|e| (e.id, &e.decoration))
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (DecorationId, &Decoration)> {
        self.entries.iter().map(|e| (e.id, &e.decoration))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn map_through_change(&mut self, change_start: usize, old_end: usize, new_end: usize) {
        let delta = new_end as isize - old_end as isize;

        self.entries.retain_mut(|entry| {
            let d = &mut entry.decoration;

            // Before the changed range.
            if d.end <= change_start {
                return true;
            }

            // After the changed range.
            if d.start >= old_end {
                d.start = (d.start as isize + delta) as usize;
                d.end = (d.end as isize + delta) as usize;
                return true;
            }

            // Overlapping the changed range.
            match d.kind {
                DecorationKind::Highlight => {
                    if d.start >= change_start {
                        d.start = change_start;
                    }
                    if d.end > old_end {
                        d.end = (d.end as isize + delta) as usize;
                    } else {
                        d.end = new_end;
                    }
                    d.start < d.end
                }
                DecorationKind::Marker => {
                    if d.start >= change_start && d.start < old_end {
                        return false;
                    }
                    true
                }
                DecorationKind::Widget { .. } => {
                    // Drop widgets fully covered by the change.
                    if d.start >= change_start && d.end <= old_end {
                        return false;
                    }
                    // Clamp partially overlapping widgets to the boundary.
                    if d.start < change_start {
                        // Keep start.
                    } else {
                        d.start = change_start;
                    }
                    if d.end > old_end {
                        d.end = (d.end as isize + delta) as usize;
                    } else {
                        d.end = new_end;
                    }
                    true
                }
            }
        });
    }

    pub fn map_through_changes(&mut self, changes: &[RangeChange]) {
        for change in changes {
            self.map_through_change(change.start(), change.old_end(), change.new_end());
        }
    }
}

impl Default for DecorationSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_ok() {
        let d = Decoration::highlight(0, 10, 1).unwrap();
        assert_eq!(d.start(), 0);
        assert_eq!(d.end(), 10);
        assert_eq!(d.kind(), DecorationKind::Highlight);
        assert_eq!(d.tag(), 1);
        assert_eq!(d.priority(), 0);
    }

    #[test]
    fn highlight_empty_range() {
        let err = Decoration::highlight(5, 5, 1).unwrap_err();
        assert_eq!(err, DecorError::EmptyHighlight { offset: 5 });
    }

    #[test]
    fn highlight_invalid_range() {
        let err = Decoration::highlight(10, 5, 1).unwrap_err();
        assert_eq!(err, DecorError::InvalidRange { start: 10, end: 5 });
    }

    #[test]
    fn marker_ok() {
        let d = Decoration::marker(42, 2);
        assert_eq!(d.start(), 42);
        assert_eq!(d.end(), 42);
        assert_eq!(d.kind(), DecorationKind::Marker);
        assert_eq!(d.tag(), 2);
    }

    #[test]
    fn widget_ok() {
        let d = Decoration::widget(10, 20, 3, true).unwrap();
        assert_eq!(d.kind(), DecorationKind::Widget { block: true });
    }

    #[test]
    fn widget_empty_range_ok() {
        let d = Decoration::widget(5, 5, 3, false).unwrap();
        assert_eq!(d.start(), 5);
        assert_eq!(d.end(), 5);
    }

    #[test]
    fn widget_invalid_range() {
        let err = Decoration::widget(20, 10, 3, false).unwrap_err();
        assert_eq!(err, DecorError::InvalidRange { start: 20, end: 10 });
    }

    #[test]
    fn with_priority() {
        let d = Decoration::marker(0, 1).with_priority(5);
        assert_eq!(d.priority(), 5);
    }

    #[test]
    fn set_add_iter_sorted() {
        let mut set = DecorationSet::new();
        set.add(Decoration::marker(20, 1));
        set.add(Decoration::marker(5, 2));
        set.add(Decoration::marker(10, 3));

        let starts: Vec<usize> = set.iter().map(|(_, d)| d.start()).collect();
        assert_eq!(starts, vec![5, 10, 20]);
    }

    #[test]
    fn set_add_remove_len() {
        let mut set = DecorationSet::new();
        let id1 = set.add(Decoration::marker(0, 1));
        let id2 = set.add(Decoration::marker(10, 2));
        assert_eq!(set.len(), 2);

        assert!(set.remove(id1));
        assert_eq!(set.len(), 1);
        assert!(!set.remove(id1));

        assert!(set.remove(id2));
        assert!(set.is_empty());
    }

    #[test]
    fn decoration_id_roundtrips_through_raw_value() {
        let mut set = DecorationSet::new();
        let id = set.add(Decoration::marker(0, 1));
        let raw = id.into_raw();

        assert!(set.remove(DecorationId::from_raw(raw)));
        assert!(set.is_empty());
    }

    #[test]
    fn set_query_range() {
        let mut set = DecorationSet::new();
        set.add(Decoration::highlight(0, 5, 1).unwrap());
        set.add(Decoration::highlight(10, 20, 2).unwrap());
        set.add(Decoration::highlight(30, 40, 3).unwrap());

        let results = set.query_range(3, 15);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].1.tag(), 1);
        assert_eq!(results[1].1.tag(), 2);
    }

    #[test]
    fn set_query_tag() {
        let mut set = DecorationSet::new();
        set.add(Decoration::marker(0, 1));
        set.add(Decoration::marker(10, 2));
        set.add(Decoration::marker(20, 1));

        let results = set.query_tag(1);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn set_query_range_includes_marker() {
        let mut set = DecorationSet::new();
        set.add(Decoration::marker(10, 1));
        set.add(Decoration::marker(20, 2));

        let results = set.query_range(10, 15);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.tag(), 1);

        let results = set.query_range(5, 10);
        assert_eq!(results.len(), 0);

        let results = set.query_range(5, 11);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn map_before_change_unchanged() {
        let mut set = DecorationSet::new();
        set.add(Decoration::highlight(0, 5, 1).unwrap());
        set.map_through_change(10, 15, 20);

        let d = set.iter().next().unwrap().1;
        assert_eq!(d.start(), 0);
        assert_eq!(d.end(), 5);
    }

    #[test]
    fn map_after_change_shifted() {
        let mut set = DecorationSet::new();
        set.add(Decoration::highlight(20, 30, 1).unwrap());
        // Insertion: [10, 15) -> [10, 20), delta = +5.
        set.map_through_change(10, 15, 20);

        let d = set.iter().next().unwrap().1;
        assert_eq!(d.start(), 25);
        assert_eq!(d.end(), 35);
    }

    #[test]
    fn map_highlight_overlap_clamp() {
        let mut set = DecorationSet::new();
        // Highlight [5, 15) overlaps change [10, 20) -> [10, 12).
        set.add(Decoration::highlight(5, 15, 1).unwrap());
        set.map_through_change(10, 20, 12);

        let d = set.iter().next().unwrap().1;
        assert_eq!(d.start(), 5);
        // End is inside the changed range, so clamp it to new_end=12.
        assert_eq!(d.end(), 12);
    }

    #[test]
    fn map_marker_deleted_in_change() {
        let mut set = DecorationSet::new();
        set.add(Decoration::marker(12, 1));
        // Change [10, 20).
        set.map_through_change(10, 20, 15);
        assert!(set.is_empty());
    }

    #[test]
    fn map_widget_fully_contained_deleted() {
        let mut set = DecorationSet::new();
        set.add(Decoration::widget(12, 18, 1, true).unwrap());
        // Change [10, 20).
        set.map_through_change(10, 20, 15);
        assert!(set.is_empty());
    }

    #[test]
    fn map_widget_partial_overlap_clamped() {
        let mut set = DecorationSet::new();
        // Widget [5, 15) partially overlaps change [10, 20) -> [10, 12).
        set.add(Decoration::widget(5, 15, 1, false).unwrap());
        set.map_through_change(10, 20, 12);

        let d = set.iter().next().unwrap().1;
        assert_eq!(d.start(), 5);
        assert_eq!(d.end(), 12);
    }

    #[test]
    fn map_through_changes_sequential() {
        let mut set = DecorationSet::new();
        set.add(Decoration::highlight(20, 30, 1).unwrap());

        let changes = [
            RangeChange::new(0, 5, 10),   // delta +5 -> [25, 35)
            RangeChange::new(0, 3, 3),    // delta 0 -> [25, 35)
            RangeChange::new(40, 40, 42), // delta +2, after decoration, no change
        ];
        set.map_through_changes(&changes);

        let d = set.iter().next().unwrap().1;
        assert_eq!(d.start(), 25);
        assert_eq!(d.end(), 35);
    }

    #[test]
    fn decor_error_display() {
        let e = DecorError::InvalidRange { start: 10, end: 5 };
        assert_eq!(e.to_string(), "invalid range: start 10 > end 5");

        let e = DecorError::EmptyHighlight { offset: 7 };
        assert_eq!(e.to_string(), "empty highlight at offset 7");
    }

    #[test]
    fn set_clear() {
        let mut set = DecorationSet::new();
        set.add(Decoration::marker(0, 1));
        set.add(Decoration::marker(10, 2));
        assert_eq!(set.len(), 2);
        set.clear();
        assert!(set.is_empty());
    }
}
