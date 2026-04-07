//! Line-level and character-level diff computation with hunk mapping and side-by-side view support.

use neco_textpatch::{TextPatch, TextPatchError};

/// Diff operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffOp {
    Equal,
    Insert,
    Delete,
}

/// Byte range reference into an original or new text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRange {
    start: usize,
    end: usize,
}

impl ByteRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
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
}

/// One line in a diff result with its operation and source ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    op: DiffOp,
    old_line: Option<u32>,
    new_line: Option<u32>,
    old_range: Option<ByteRange>,
    new_range: Option<ByteRange>,
}

impl DiffLine {
    pub fn op(&self) -> DiffOp {
        self.op
    }

    pub fn old_line(&self) -> Option<u32> {
        self.old_line
    }

    pub fn new_line(&self) -> Option<u32> {
        self.new_line
    }

    pub fn old_range(&self) -> Option<ByteRange> {
        self.old_range
    }

    pub fn new_range(&self) -> Option<ByteRange> {
        self.new_range
    }
}

/// Group of contiguous changes with surrounding context lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    lines: Vec<DiffLine>,
}

impl DiffHunk {
    pub fn old_start(&self) -> u32 {
        self.old_start
    }

    pub fn old_count(&self) -> u32 {
        self.old_count
    }

    pub fn new_start(&self) -> u32 {
        self.new_start
    }

    pub fn new_count(&self) -> u32 {
        self.new_count
    }

    pub fn lines(&self) -> &[DiffLine] {
        &self.lines
    }
}

/// Result of a line-level diff computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffResult {
    lines: Vec<DiffLine>,
}

impl DiffResult {
    pub fn lines(&self) -> &[DiffLine] {
        &self.lines
    }

    pub fn to_hunks(&self, context_lines: u32) -> Vec<DiffHunk> {
        let ctx = context_lines as usize;
        let n = self.lines.len();
        if n == 0 {
            return Vec::new();
        }

        let change_indices: Vec<usize> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.op != DiffOp::Equal)
            .map(|(i, _)| i)
            .collect();

        if change_indices.is_empty() {
            return Vec::new();
        }

        let mut groups: Vec<(usize, usize)> = Vec::new();
        let mut group_start = change_indices[0].saturating_sub(ctx);
        let mut group_end = (change_indices[0] + ctx).min(n - 1);

        for &ci in &change_indices[1..] {
            let lo = ci.saturating_sub(ctx);
            let hi = (ci + ctx).min(n - 1);
            if lo <= group_end + 1 {
                group_end = hi;
            } else {
                groups.push((group_start, group_end));
                group_start = lo;
                group_end = hi;
            }
        }
        groups.push((group_start, group_end));

        groups
            .into_iter()
            .map(|(start, end)| {
                let hunk_lines: Vec<DiffLine> = self.lines[start..=end].to_vec();
                let mut old_start = u32::MAX;
                let mut old_count = 0u32;
                let mut new_start = u32::MAX;
                let mut new_count = 0u32;
                for line in &hunk_lines {
                    if let Some(ol) = line.old_line {
                        if ol < old_start {
                            old_start = ol;
                        }
                        old_count += 1;
                    }
                    if let Some(nl) = line.new_line {
                        if nl < new_start {
                            new_start = nl;
                        }
                        new_count += 1;
                    }
                }
                if old_start == u32::MAX {
                    old_start = 0;
                }
                if new_start == u32::MAX {
                    new_start = 0;
                }
                DiffHunk {
                    old_start,
                    old_count,
                    new_start,
                    new_count,
                    lines: hunk_lines,
                }
            })
            .collect()
    }
}

/// Character-level diff range within a single line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntraLineRange {
    start: usize,
    end: usize,
    op: DiffOp,
}

impl IntraLineRange {
    pub fn new(start: usize, end: usize, op: DiffOp) -> Self {
        Self { start, end, op }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn op(&self) -> DiffOp {
        self.op
    }
}

/// Character-level diff result for one line pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntraLineDiff {
    ranges: Vec<IntraLineRange>,
}

impl IntraLineDiff {
    pub fn ranges(&self) -> &[IntraLineRange] {
        &self.ranges
    }
}

/// One side of a side-by-side diff row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideLine {
    line: u32,
    op: DiffOp,
    range: ByteRange,
}

impl SideLine {
    pub fn new(line: u32, op: DiffOp, range: ByteRange) -> Self {
        Self { line, op, range }
    }

    pub fn line(&self) -> u32 {
        self.line
    }

    pub fn op(&self) -> DiffOp {
        self.op
    }

    pub fn range(&self) -> ByteRange {
        self.range
    }
}

/// Side-by-side diff row pairing left and right lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideBySideLine {
    left: Option<SideLine>,
    right: Option<SideLine>,
}

impl SideBySideLine {
    pub fn new(left: Option<SideLine>, right: Option<SideLine>) -> Self {
        Self { left, right }
    }

    pub fn left(&self) -> Option<&SideLine> {
        self.left.as_ref()
    }

    pub fn right(&self) -> Option<&SideLine> {
        self.right.as_ref()
    }
}

/// Myers O(ND) diff。edit script を (DiffOp, count) の run-length encoding で返す。
fn myers_diff<T: PartialEq>(old: &[T], new: &[T]) -> Vec<(DiffOp, usize)> {
    let n = old.len();
    let m = new.len();

    if n == 0 && m == 0 {
        return Vec::new();
    }
    if n == 0 {
        return vec![(DiffOp::Insert, m)];
    }
    if m == 0 {
        return vec![(DiffOp::Delete, n)];
    }

    let max = n + m;
    // v は k → x のマッピング。k = x - y で、index は k + offset。
    let size = 2 * max + 1;
    let mut v: Vec<usize> = vec![0; size];
    let off = max; // offset for indexing: v[k + off]

    // trace[d] = v snapshot at step d (before snake extension? no, after)
    // We store v at the START of each d iteration (before computing d's values).
    let mut trace: Vec<Vec<usize>> = Vec::new();

    let mut found_d = 0;
    'search: for d in 0..=max {
        trace.push(v.clone());
        let d_i = d as isize;
        let mut k = -d_i;
        while k <= d_i {
            let ki = (k + off as isize) as usize;
            let mut x = if k == -d_i
                || (k != d_i
                    && v[(k - 1 + off as isize) as usize] < v[(k + 1 + off as isize) as usize])
            {
                v[(k + 1 + off as isize) as usize] // move down (insert)
            } else {
                v[(k - 1 + off as isize) as usize] + 1 // move right (delete)
            };
            let mut y = (x as isize - k) as usize;

            // extend snake
            while x < n && y < m && old[x] == new[y] {
                x += 1;
                y += 1;
            }

            v[ki] = x;

            if x >= n && y >= m {
                found_d = d;
                break 'search;
            }

            k += 2;
        }
    }

    // Backtrack from (n, m)
    let mut x = n;
    let mut y = m;
    let mut edits: Vec<DiffOp> = Vec::new();

    for d in (0..=found_d).rev() {
        let v_prev = &trace[d];
        let k = x as isize - y as isize;
        let d_i = d as isize;

        // Determine prev_k: which k did we come from at step d-1?
        // At step d (1-indexed), we stored trace[d] = v at the start of step d.
        // trace[d] is the state BEFORE computing step d's values.
        // So trace[d] = v at end of step d-1.
        // Wait - we push v.clone() at the START of d loop, before computing.
        // So trace[0] = initial v, trace[1] = v after d=0, etc.
        // trace[d] = v state after d-1 steps completed.

        if d == 0 {
            // All remaining moves must be diagonal (equal)
            while x > 0 && y > 0 {
                x -= 1;
                y -= 1;
                edits.push(DiffOp::Equal);
            }
            break;
        }

        // v_prev = trace[d] = state after d-1 steps
        let prev_k = if k == -d_i
            || (k != d_i
                && v_prev[(k - 1 + off as isize) as usize]
                    < v_prev[(k + 1 + off as isize) as usize])
        {
            k + 1
        } else {
            k - 1
        };

        let prev_x = v_prev[(prev_k + off as isize) as usize];
        let prev_y = (prev_x as isize - prev_k) as usize;

        // Diagonal (snake) from (prev_x, prev_y) after the edit step
        while x > prev_x && y > prev_y {
            x -= 1;
            y -= 1;
            edits.push(DiffOp::Equal);
        }

        // The edit step
        if prev_k == k + 1 {
            // came from k+1 → moved down → insert
            y -= 1;
            edits.push(DiffOp::Insert);
        } else {
            // came from k-1 → moved right → delete
            x -= 1;
            edits.push(DiffOp::Delete);
        }
    }

    edits.reverse();

    // Run-length encode
    let mut result: Vec<(DiffOp, usize)> = Vec::new();
    for op in edits {
        if let Some(last) = result.last_mut() {
            if last.0 == op {
                last.1 += 1;
                continue;
            }
        }
        result.push((op, 1));
    }
    result
}

struct LineInfo {
    byte_start: usize,
    byte_end: usize,
}

fn split_lines(text: &str) -> Vec<LineInfo> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            lines.push(LineInfo {
                byte_start: start,
                byte_end: i + 1,
            });
            start = i + 1;
        }
    }
    if start < text.len() {
        lines.push(LineInfo {
            byte_start: start,
            byte_end: text.len(),
        });
    }
    lines
}

/// Compute a line-level diff between two texts using the Myers O(ND) algorithm.
pub fn diff(old: &str, new: &str) -> DiffResult {
    let old_lines = split_lines(old);
    let new_lines = split_lines(new);

    let old_strs: Vec<&str> = old_lines
        .iter()
        .map(|l| &old[l.byte_start..l.byte_end])
        .collect();
    let new_strs: Vec<&str> = new_lines
        .iter()
        .map(|l| &new[l.byte_start..l.byte_end])
        .collect();

    let ops = myers_diff(&old_strs, &new_strs);

    let mut result_lines = Vec::new();
    let mut old_idx = 0usize;
    let mut new_idx = 0usize;

    for &(op, count) in &ops {
        match op {
            DiffOp::Equal => {
                for _ in 0..count {
                    let ol = &old_lines[old_idx];
                    let nl = &new_lines[new_idx];
                    result_lines.push(DiffLine {
                        op: DiffOp::Equal,
                        old_line: Some(u32::try_from(old_idx).expect("line count fits u32")),
                        new_line: Some(u32::try_from(new_idx).expect("line count fits u32")),
                        old_range: Some(ByteRange::new(ol.byte_start, ol.byte_end)),
                        new_range: Some(ByteRange::new(nl.byte_start, nl.byte_end)),
                    });
                    old_idx += 1;
                    new_idx += 1;
                }
            }
            DiffOp::Delete => {
                for _ in 0..count {
                    let ol = &old_lines[old_idx];
                    result_lines.push(DiffLine {
                        op: DiffOp::Delete,
                        old_line: Some(u32::try_from(old_idx).expect("line count fits u32")),
                        new_line: None,
                        old_range: Some(ByteRange::new(ol.byte_start, ol.byte_end)),
                        new_range: None,
                    });
                    old_idx += 1;
                }
            }
            DiffOp::Insert => {
                for _ in 0..count {
                    let nl = &new_lines[new_idx];
                    result_lines.push(DiffLine {
                        op: DiffOp::Insert,
                        old_line: None,
                        new_line: Some(u32::try_from(new_idx).expect("line count fits u32")),
                        old_range: None,
                        new_range: Some(ByteRange::new(nl.byte_start, nl.byte_end)),
                    });
                    new_idx += 1;
                }
            }
        }
    }

    DiffResult {
        lines: result_lines,
    }
}

/// Compute a character-level diff between two lines.
pub fn diff_intra_line(old_line: &str, new_line: &str) -> IntraLineDiff {
    let old_chars: Vec<char> = old_line.chars().collect();
    let new_chars: Vec<char> = new_line.chars().collect();

    let ops = myers_diff(&old_chars, &new_chars);

    let mut ranges = Vec::new();
    let mut old_byte = 0usize;
    let mut new_byte = 0usize;
    let mut old_ci = 0usize;
    let mut new_ci = 0usize;

    for &(op, count) in &ops {
        match op {
            DiffOp::Equal => {
                for _ in 0..count {
                    old_byte += old_chars[old_ci].len_utf8();
                    new_byte += new_chars[new_ci].len_utf8();
                    old_ci += 1;
                    new_ci += 1;
                }
            }
            DiffOp::Delete => {
                let start = old_byte;
                for _ in 0..count {
                    old_byte += old_chars[old_ci].len_utf8();
                    old_ci += 1;
                }
                ranges.push(IntraLineRange::new(start, old_byte, DiffOp::Delete));
            }
            DiffOp::Insert => {
                let start = new_byte;
                for _ in 0..count {
                    new_byte += new_chars[new_ci].len_utf8();
                    new_ci += 1;
                }
                ranges.push(IntraLineRange::new(start, new_byte, DiffOp::Insert));
            }
        }
    }

    IntraLineDiff { ranges }
}

/// Convert a diff result to side-by-side display rows.
pub fn to_side_by_side(result: &DiffResult) -> Vec<SideBySideLine> {
    let lines = result.lines();
    let mut output = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        match lines[i].op {
            DiffOp::Equal => {
                let l = &lines[i];
                output.push(SideBySideLine::new(
                    Some(SideLine::new(
                        l.old_line.unwrap(),
                        DiffOp::Equal,
                        l.old_range.unwrap(),
                    )),
                    Some(SideLine::new(
                        l.new_line.unwrap(),
                        DiffOp::Equal,
                        l.new_range.unwrap(),
                    )),
                ));
                i += 1;
            }
            DiffOp::Delete => {
                let mut deletes = Vec::new();
                while i < lines.len() && lines[i].op == DiffOp::Delete {
                    deletes.push(&lines[i]);
                    i += 1;
                }
                let mut inserts = Vec::new();
                while i < lines.len() && lines[i].op == DiffOp::Insert {
                    inserts.push(&lines[i]);
                    i += 1;
                }
                let max_len = deletes.len().max(inserts.len());
                for j in 0..max_len {
                    let left = deletes.get(j).map(|d| {
                        SideLine::new(d.old_line.unwrap(), DiffOp::Delete, d.old_range.unwrap())
                    });
                    let right = inserts.get(j).map(|ins| {
                        SideLine::new(
                            ins.new_line.unwrap(),
                            DiffOp::Insert,
                            ins.new_range.unwrap(),
                        )
                    });
                    output.push(SideBySideLine::new(left, right));
                }
            }
            DiffOp::Insert => {
                let l = &lines[i];
                output.push(SideBySideLine::new(
                    None,
                    Some(SideLine::new(
                        l.new_line.unwrap(),
                        DiffOp::Insert,
                        l.new_range.unwrap(),
                    )),
                ));
                i += 1;
            }
        }
    }

    output
}

/// Convert a diff result to a list of `TextPatch` values that can be applied to the old text.
pub fn diff_to_patches(new: &str, result: &DiffResult) -> Result<Vec<TextPatch>, TextPatchError> {
    let mut patches = Vec::new();
    let lines = result.lines();
    let mut i = 0;

    while i < lines.len() {
        match lines[i].op {
            DiffOp::Equal => {
                i += 1;
            }
            DiffOp::Delete => {
                let delete_start = i;
                while i < lines.len() && lines[i].op == DiffOp::Delete {
                    i += 1;
                }
                let delete_end = i;

                let insert_start = i;
                while i < lines.len() && lines[i].op == DiffOp::Insert {
                    i += 1;
                }
                let insert_end = i;

                let first_del = &lines[delete_start];
                let last_del = &lines[delete_end - 1];
                let del_byte_start = first_del.old_range.unwrap().start();
                let del_byte_end = last_del.old_range.unwrap().end();

                if insert_start < insert_end {
                    let first_ins = &lines[insert_start];
                    let last_ins = &lines[insert_end - 1];
                    let ins_byte_start = first_ins.new_range.unwrap().start();
                    let ins_byte_end = last_ins.new_range.unwrap().end();
                    let replacement = &new[ins_byte_start..ins_byte_end];
                    patches.push(TextPatch::replace(
                        del_byte_start,
                        del_byte_end,
                        replacement,
                    )?);
                } else {
                    patches.push(TextPatch::delete(del_byte_start, del_byte_end)?);
                }
            }
            DiffOp::Insert => {
                let insert_start = i;
                while i < lines.len() && lines[i].op == DiffOp::Insert {
                    i += 1;
                }
                let insert_end = i;

                let first_ins = &lines[insert_start];
                let last_ins = &lines[insert_end - 1];
                let ins_byte_start = first_ins.new_range.unwrap().start();
                let ins_byte_end = last_ins.new_range.unwrap().end();
                let replacement = &new[ins_byte_start..ins_byte_end];

                let offset = if insert_start > 0 {
                    let prev = &lines[insert_start - 1];
                    if let Some(r) = prev.old_range {
                        r.end()
                    } else {
                        0
                    }
                } else {
                    0
                };

                patches.push(TextPatch::insert(offset, replacement));
            }
        }
    }

    Ok(patches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use neco_textpatch::apply_patches;

    #[test]
    fn diff_empty_texts() {
        let result = diff("", "");
        assert!(result.lines().is_empty());
    }

    #[test]
    fn diff_identical_texts() {
        let text = "hello\nworld\n";
        let result = diff(text, text);
        assert!(result.lines().iter().all(|l| l.op() == DiffOp::Equal));
        assert_eq!(result.lines().len(), 2);
    }

    #[test]
    fn diff_all_deleted() {
        let result = diff("a\nb\n", "");
        assert!(result.lines().iter().all(|l| l.op() == DiffOp::Delete));
        assert_eq!(result.lines().len(), 2);
    }

    #[test]
    fn diff_all_inserted() {
        let result = diff("", "x\ny\n");
        assert!(result.lines().iter().all(|l| l.op() == DiffOp::Insert));
        assert_eq!(result.lines().len(), 2);
    }

    #[test]
    fn diff_mixed_changes() {
        let old = "a\nb\nc\nd\n";
        let new = "a\nB\nc\nD\n";
        let result = diff(old, new);
        let ops: Vec<DiffOp> = result.lines().iter().map(|l| l.op()).collect();
        assert_eq!(
            ops,
            vec![
                DiffOp::Equal,
                DiffOp::Delete,
                DiffOp::Insert,
                DiffOp::Equal,
                DiffOp::Delete,
                DiffOp::Insert,
            ]
        );
    }

    #[test]
    fn diff_multibyte() {
        let old = "こんにちは\n世界\n";
        let new = "こんにちは\n宇宙\n";
        let result = diff(old, new);
        assert_eq!(result.lines().len(), 3);
        assert_eq!(result.lines()[0].op(), DiffOp::Equal);
        assert_eq!(result.lines()[1].op(), DiffOp::Delete);
        assert_eq!(result.lines()[2].op(), DiffOp::Insert);
    }

    #[test]
    fn to_hunks_splits_by_context() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
        let new = "1\n2\n3\n4\nFIVE\n6\n7\n8\n9\nTEN\n";
        let result = diff(old, new);
        let hunks = result.to_hunks(1);
        assert_eq!(hunks.len(), 2);
        assert!(hunks[0].lines().iter().any(|l| l.op() != DiffOp::Equal));
        assert!(hunks[1].lines().iter().any(|l| l.op() != DiffOp::Equal));
    }

    #[test]
    fn to_hunks_merges_close_changes() {
        let old = "1\n2\n3\n4\n5\n";
        let new = "ONE\n2\n3\n4\nFIVE\n";
        let result = diff(old, new);
        let hunks = result.to_hunks(3);
        assert_eq!(hunks.len(), 1);
    }

    #[test]
    fn diff_intra_line_detects_changes() {
        let result = diff_intra_line("hello world", "hello there");
        assert!(!result.ranges().is_empty());
        let has_delete = result.ranges().iter().any(|r| r.op() == DiffOp::Delete);
        let has_insert = result.ranges().iter().any(|r| r.op() == DiffOp::Insert);
        assert!(has_delete);
        assert!(has_insert);
    }

    #[test]
    fn diff_intra_line_multibyte() {
        let result = diff_intra_line("あいう", "あえう");
        assert!(!result.ranges().is_empty());
        let has_delete = result.ranges().iter().any(|r| r.op() == DiffOp::Delete);
        let has_insert = result.ranges().iter().any(|r| r.op() == DiffOp::Insert);
        assert!(has_delete);
        assert!(has_insert);
    }

    #[test]
    fn side_by_side_layout() {
        let old = "a\nb\nc\n";
        let new = "a\nB\nc\n";
        let result = diff(old, new);
        let sbs = to_side_by_side(&result);

        assert!(sbs[0].left().is_some() && sbs[0].right().is_some());
        assert_eq!(sbs[0].left().unwrap().op(), DiffOp::Equal);

        let delete_line = sbs
            .iter()
            .find(|l| l.left().is_some_and(|s| s.op() == DiffOp::Delete));
        assert!(delete_line.is_some());

        let insert_line = sbs
            .iter()
            .find(|l| l.right().is_some_and(|s| s.op() == DiffOp::Insert));
        assert!(insert_line.is_some());
    }

    #[test]
    fn side_by_side_insert_only() {
        let old = "a\n";
        let new = "a\nb\n";
        let result = diff(old, new);
        let sbs = to_side_by_side(&result);
        let insert_only = sbs
            .iter()
            .find(|l| l.left().is_none() && l.right().is_some());
        assert!(insert_only.is_some());
    }

    #[test]
    fn side_by_side_delete_only() {
        let old = "a\nb\n";
        let new = "a\n";
        let result = diff(old, new);
        let sbs = to_side_by_side(&result);
        let delete_only = sbs
            .iter()
            .find(|l| l.left().is_some() && l.right().is_none());
        assert!(delete_only.is_some());
    }

    fn roundtrip(old: &str, new: &str) {
        let result = diff(old, new);
        let patches = diff_to_patches(new, &result).expect("patches should be created");
        let applied = apply_patches(old, &patches).expect("patches should apply");
        assert_eq!(applied, new, "roundtrip failed for old={old:?} new={new:?}");
    }

    #[test]
    fn roundtrip_empty_to_empty() {
        roundtrip("", "");
    }

    #[test]
    fn roundtrip_identical() {
        roundtrip("hello\nworld\n", "hello\nworld\n");
    }

    #[test]
    fn roundtrip_all_deleted() {
        roundtrip("a\nb\nc\n", "");
    }

    #[test]
    fn roundtrip_all_inserted() {
        roundtrip("", "x\ny\nz\n");
    }

    #[test]
    fn roundtrip_mixed_changes() {
        roundtrip("a\nb\nc\nd\n", "a\nB\nc\nD\n");
    }

    #[test]
    fn roundtrip_insert_at_beginning() {
        roundtrip("b\nc\n", "a\nb\nc\n");
    }

    #[test]
    fn roundtrip_insert_at_end() {
        roundtrip("a\nb\n", "a\nb\nc\n");
    }

    #[test]
    fn roundtrip_delete_at_beginning() {
        roundtrip("a\nb\nc\n", "b\nc\n");
    }

    #[test]
    fn roundtrip_delete_at_end() {
        roundtrip("a\nb\nc\n", "a\nb\n");
    }

    #[test]
    fn roundtrip_multibyte() {
        roundtrip("こんにちは\n世界\n", "こんにちは\n宇宙\n");
    }

    #[test]
    fn roundtrip_complex() {
        let old = "line1\nline2\nline3\nline4\nline5\n";
        let new = "line1\nmodified2\nline3\nnew_line\nline5\nextra\n";
        roundtrip(old, new);
    }

    #[test]
    fn roundtrip_no_trailing_newline() {
        roundtrip("hello\nworld", "hello\nthere");
    }

    #[test]
    fn roundtrip_consecutive_deletes_and_inserts() {
        roundtrip("a\nb\nc\n", "x\ny\nz\n");
    }
}
