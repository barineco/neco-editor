//! Word wrap engine for splitting logical lines into visual lines.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakOpportunity {
    Allowed,
    Forbidden,
    Mandatory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    HorizontalLtr,
    VerticalRl,
    VerticalLr,
}

#[derive(Debug, Clone, Copy)]
pub struct LineLayoutPolicy {
    layout_mode: LayoutMode,
    redistribute_inline_width: fn(u32, u32) -> u32,
}

impl LineLayoutPolicy {
    pub fn new(layout_mode: LayoutMode, redistribute_inline_width: fn(u32, u32) -> u32) -> Self {
        Self {
            layout_mode,
            redistribute_inline_width,
        }
    }

    pub fn horizontal_ltr() -> Self {
        Self::new(LayoutMode::HorizontalLtr, preserve_inline_width)
    }

    pub fn layout_mode(&self) -> LayoutMode {
        self.layout_mode
    }

    pub fn redistributed_inline_width(&self, line_width: u32, max_width: u32) -> u32 {
        (self.redistribute_inline_width)(line_width, max_width)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WidthPolicy {
    char_width: fn(char) -> u32,
    tab_width: Option<u32>,
}

impl WidthPolicy {
    pub fn new(char_width: fn(char) -> u32) -> Self {
        Self {
            char_width,
            tab_width: None,
        }
    }

    pub fn monospace_ascii(tab_width: u32) -> Self {
        Self::with_tab_width(monospace_ascii_width, tab_width)
    }

    pub fn cjk_grid(tab_width: u32) -> Self {
        Self::with_tab_width(cjk_grid_width, tab_width)
    }

    pub fn with_tab_width(char_width: fn(char) -> u32, tab_width: u32) -> Self {
        Self {
            char_width,
            tab_width: Some(tab_width),
        }
    }

    pub fn tab_width(&self) -> Option<u32> {
        self.tab_width
    }

    pub fn char_width(&self) -> fn(char) -> u32 {
        self.char_width
    }

    pub fn advance_of(&self, ch: char) -> u32 {
        if ch == '\t' {
            self.tab_width.unwrap_or_else(|| (self.char_width)(ch))
        } else {
            (self.char_width)(ch)
        }
    }

    pub fn text_width(&self, text: &str) -> u32 {
        text.chars().map(|ch| self.advance_of(ch)).sum()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WrapPolicy {
    width_policy: WidthPolicy,
    break_opportunity: fn(&str, usize) -> BreakOpportunity,
}

impl WrapPolicy {
    pub fn new(
        char_width: fn(char) -> u32,
        break_opportunity: fn(&str, usize) -> BreakOpportunity,
    ) -> Self {
        Self::with_width_policy(WidthPolicy::new(char_width), break_opportunity)
    }

    pub fn with_width_policy(
        width_policy: WidthPolicy,
        break_opportunity: fn(&str, usize) -> BreakOpportunity,
    ) -> Self {
        Self {
            width_policy,
            break_opportunity,
        }
    }

    pub fn width_policy(&self) -> WidthPolicy {
        self.width_policy
    }

    pub fn char_width(&self) -> fn(char) -> u32 {
        self.width_policy.char_width()
    }

    pub fn break_opportunity(&self) -> fn(&str, usize) -> BreakOpportunity {
        self.break_opportunity
    }

    pub fn code() -> Self {
        Self::code_with_width_policy(WidthPolicy::cjk_grid(4))
    }

    pub fn code_with_width_policy(width_policy: WidthPolicy) -> Self {
        Self::with_width_policy(width_policy, code_break_opportunity)
    }

    pub fn japanese_basic() -> Self {
        Self::with_width_policy(WidthPolicy::cjk_grid(4), japanese_break_opportunity)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrapPoint {
    byte_offset: u32,
    visual_width: u32,
}

impl WrapPoint {
    pub const fn byte_offset(&self) -> u32 {
        self.byte_offset
    }

    pub const fn visual_width(&self) -> u32 {
        self.visual_width
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualLine {
    start: u32,
    end: u32,
}

impl VisualLine {
    pub const fn start(&self) -> u32 {
        self.start
    }

    pub const fn end(&self) -> u32 {
        self.end
    }

    pub const fn len(&self) -> u32 {
        self.end - self.start
    }

    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualLayoutSpace {
    logical_line: u32,
    visual_line: u32,
    inline_advance: u32,
    block_advance: u32,
    layout_mode: LayoutMode,
}

impl VisualLayoutSpace {
    pub const fn logical_line(&self) -> u32 {
        self.logical_line
    }

    pub const fn visual_line(&self) -> u32 {
        self.visual_line
    }

    pub const fn inline_advance(&self) -> u32 {
        self.inline_advance
    }

    pub const fn block_advance(&self) -> u32 {
        self.block_advance
    }

    pub const fn layout_mode(&self) -> LayoutMode {
        self.layout_mode
    }
}

#[derive(Debug, Clone)]
pub struct WrapMap {
    line_wraps: Vec<Vec<WrapPoint>>,
    max_width: u32,
}

impl WrapMap {
    pub fn new<'a>(
        lines: impl Iterator<Item = &'a str>,
        max_width: u32,
        policy: &WrapPolicy,
    ) -> Self {
        let line_wraps = lines
            .map(|line| wrap_line(line, max_width, policy))
            .collect::<Vec<_>>();
        Self {
            line_wraps,
            max_width,
        }
    }

    pub const fn max_width(&self) -> u32 {
        self.max_width
    }

    pub fn line_count(&self) -> u32 {
        usize_to_u32(self.line_wraps.len(), "line count")
    }

    pub fn visual_line_count(&self, line: u32) -> u32 {
        let index = u32_to_usize(line, "line");
        let wraps = &self.line_wraps[index];
        usize_to_u32(wraps.len(), "visual line count") + 1
    }

    pub fn total_visual_lines(&self) -> u32 {
        self.line_wraps.iter().fold(0u32, |acc, wraps| {
            acc + usize_to_u32(wraps.len(), "visual line count") + 1
        })
    }

    pub fn wrap_points(&self, line: u32) -> &[WrapPoint] {
        let index = u32_to_usize(line, "line");
        &self.line_wraps[index]
    }

    pub fn visual_lines(&self, line: u32, line_len: u32) -> Vec<VisualLine> {
        let mut visual_lines = Vec::new();
        let mut start = 0u32;
        for wrap in self.wrap_points(line) {
            visual_lines.push(VisualLine {
                start,
                end: wrap.byte_offset(),
            });
            start = wrap.byte_offset();
        }
        visual_lines.push(VisualLine {
            start,
            end: line_len,
        });
        visual_lines
    }

    pub fn visual_layout_space(
        &self,
        line: u32,
        local_visual_line: u32,
        line_text: &str,
        policy: &WrapPolicy,
        line_layout_policy: &LineLayoutPolicy,
    ) -> VisualLayoutSpace {
        let line_len = usize_to_u32(line_text.len(), "line len");
        let visual_lines = self.visual_lines(line, line_len);
        let index = u32_to_usize(local_visual_line, "local visual line");
        let visual_line = visual_lines[index];
        let inline_advance = line_layout_policy.redistributed_inline_width(
            policy.width_policy().text_width(
                &line_text[u32_to_usize(visual_line.start, "start")
                    ..u32_to_usize(visual_line.end, "end")],
            ),
            self.max_width,
        );

        VisualLayoutSpace {
            logical_line: line,
            visual_line: local_visual_line,
            inline_advance,
            block_advance: local_visual_line,
            layout_mode: line_layout_policy.layout_mode(),
        }
    }

    pub fn to_visual_line(&self, line: u32, byte_offset_in_line: u32) -> u32 {
        let prior = (0..line)
            .map(|current| self.visual_line_count(current))
            .sum::<u32>();
        let local = self
            .wrap_points(line)
            .iter()
            .take_while(|wrap| wrap.byte_offset() <= byte_offset_in_line)
            .count();
        prior + usize_to_u32(local, "visual line index")
    }

    pub fn from_visual_line(&self, visual_line: u32) -> (u32, u32) {
        let mut remaining = visual_line;
        for (line_index, wraps) in self.line_wraps.iter().enumerate() {
            let count = usize_to_u32(wraps.len(), "visual line count") + 1;
            if remaining < count {
                let start = if remaining == 0 {
                    0
                } else {
                    let wrap_index = u32_to_usize(remaining - 1, "wrap index");
                    wraps[wrap_index].byte_offset()
                };
                return (usize_to_u32(line_index, "line"), start);
            }
            remaining -= count;
        }
        panic!("visual line {visual_line} out of bounds");
    }

    pub fn rewrap_line(&mut self, line: u32, line_text: &str, policy: &WrapPolicy) {
        let index = u32_to_usize(line, "line");
        self.line_wraps[index] = wrap_line(line_text, self.max_width, policy);
    }

    pub fn set_max_width<'a>(
        &mut self,
        max_width: u32,
        lines: impl Iterator<Item = &'a str>,
        policy: &WrapPolicy,
    ) {
        self.max_width = max_width;
        self.line_wraps = lines
            .map(|line| wrap_line(line, max_width, policy))
            .collect::<Vec<_>>();
    }

    pub fn splice_lines<'a>(
        &mut self,
        start_line: u32,
        removed_count: u32,
        new_lines: impl Iterator<Item = &'a str>,
        policy: &WrapPolicy,
    ) {
        let start = u32_to_usize(start_line, "start line");
        let end = start + u32_to_usize(removed_count, "removed line count");
        let replacement = new_lines
            .map(|line| wrap_line(line, self.max_width, policy))
            .collect::<Vec<_>>();
        self.line_wraps.splice(start..end, replacement);
    }
}

pub fn wrap_line(line_text: &str, max_width: u32, policy: &WrapPolicy) -> Vec<WrapPoint> {
    if max_width == 0 {
        return Vec::new();
    }

    let width_policy = policy.width_policy();
    let break_opportunity = policy.break_opportunity();
    let mut wraps = Vec::new();
    let mut total_width = 0u32;
    let mut segment_start_offset = 0u32;
    let mut segment_start_width = 0u32;
    let mut last_allowed = None::<WrapPoint>;

    for (byte_offset, ch) in line_text.char_indices() {
        total_width += width_policy.advance_of(ch);
        let next_offset = byte_offset + ch.len_utf8();
        let next_offset_u32 = usize_to_u32(next_offset, "byte offset");
        let wrap_point = WrapPoint {
            byte_offset: next_offset_u32,
            visual_width: total_width,
        };

        match break_opportunity(line_text, next_offset) {
            BreakOpportunity::Allowed => {
                last_allowed = Some(wrap_point);
            }
            BreakOpportunity::Forbidden => {}
            BreakOpportunity::Mandatory => {
                if next_offset < line_text.len() && next_offset_u32 > segment_start_offset {
                    wraps.push(wrap_point);
                    segment_start_offset = next_offset_u32;
                    segment_start_width = total_width;
                }
                last_allowed = None;
                continue;
            }
        }

        if total_width.saturating_sub(segment_start_width) > max_width {
            if let Some(candidate) = last_allowed {
                if candidate.byte_offset() > segment_start_offset {
                    wraps.push(candidate);
                    segment_start_offset = candidate.byte_offset();
                    segment_start_width = candidate.visual_width();
                }
            }
            last_allowed = None;
        }
    }

    wraps
}

fn monospace_ascii_width(ch: char) -> u32 {
    let _ = ch;
    1
}

fn cjk_grid_width(ch: char) -> u32 {
    if ch.is_ascii() {
        1
    } else {
        east_asian_width(ch)
    }
}

fn preserve_inline_width(line_width: u32, _max_width: u32) -> u32 {
    line_width
}

fn code_break_opportunity(line_text: &str, byte_offset: usize) -> BreakOpportunity {
    if byte_offset == 0 || byte_offset >= line_text.len() {
        return BreakOpportunity::Forbidden;
    }

    match prev_char(line_text, byte_offset) {
        Some(ch) if ch.is_whitespace() || is_code_operator(ch) => BreakOpportunity::Allowed,
        _ => BreakOpportunity::Forbidden,
    }
}

fn japanese_break_opportunity(line_text: &str, byte_offset: usize) -> BreakOpportunity {
    if byte_offset == 0 || byte_offset >= line_text.len() {
        return BreakOpportunity::Forbidden;
    }

    let prev = match prev_char(line_text, byte_offset) {
        Some(ch) => ch,
        None => return BreakOpportunity::Forbidden,
    };
    let next = match next_char(line_text, byte_offset) {
        Some(ch) => ch,
        None => return BreakOpportunity::Forbidden,
    };

    if is_line_start_kinsoku(next) || is_line_end_kinsoku(prev) {
        return BreakOpportunity::Forbidden;
    }

    if is_japanese_wrap_char(prev) && is_japanese_wrap_char(next) {
        BreakOpportunity::Allowed
    } else {
        BreakOpportunity::Forbidden
    }
}

fn prev_char(line_text: &str, byte_offset: usize) -> Option<char> {
    line_text[..byte_offset].chars().next_back()
}

fn next_char(line_text: &str, byte_offset: usize) -> Option<char> {
    line_text[byte_offset..].chars().next()
}

fn is_code_operator(ch: char) -> bool {
    matches!(
        ch,
        '+' | '-'
            | '*'
            | '/'
            | '%'
            | '='
            | '!'
            | '?'
            | '&'
            | '|'
            | '^'
            | '~'
            | ':'
            | ';'
            | ','
            | '.'
    )
}

fn is_line_start_kinsoku(ch: char) -> bool {
    "。、．，：；？！）」』】〉》〕｝―…".contains(ch)
}

fn is_line_end_kinsoku(ch: char) -> bool {
    "（「『【〈《〔｛".contains(ch)
}

fn is_japanese_wrap_char(ch: char) -> bool {
    east_asian_width(ch) == 2
}

fn east_asian_width(ch: char) -> u32 {
    if matches!(
        ch as u32,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0x303E
            | 0x3040..=0x30FF
            | 0x3100..=0x312F
            | 0x3130..=0x318F
            | 0x3190..=0x31EF
            | 0x31F0..=0x31FF
            | 0x3200..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF01..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    ) {
        2
    } else {
        1
    }
}

fn usize_to_u32(value: usize, what: &str) -> u32 {
    u32::try_from(value).unwrap_or_else(|_| panic!("{what} exceeds u32::MAX"))
}

fn u32_to_usize(value: u32, what: &str) -> usize {
    usize::try_from(value).unwrap_or_else(|_| panic!("{what} exceeds usize::MAX"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_line_basic_wrapping() {
        let wraps = wrap_line("ab cd ef", 4, &WrapPolicy::code());
        assert_eq!(
            wraps,
            vec![
                WrapPoint {
                    byte_offset: 3,
                    visual_width: 3,
                },
                WrapPoint {
                    byte_offset: 6,
                    visual_width: 6,
                },
            ]
        );
    }

    #[test]
    fn wrap_line_no_wrap_needed() {
        let wraps = wrap_line("abc", 10, &WrapPolicy::code());
        assert!(wraps.is_empty());
    }

    #[test]
    fn wrap_line_zero_width_disables_wrapping() {
        let wraps = wrap_line("ab cd", 0, &WrapPolicy::code());
        assert!(wraps.is_empty());
    }

    #[test]
    fn wrap_line_cjk_width_is_two() {
        let wraps = wrap_line("あい うえ", 4, &WrapPolicy::code());
        assert_eq!(wraps.len(), 1);
        assert_eq!(wraps[0].byte_offset(), usize_to_u32("あい ".len(), "len"));
        assert_eq!(wraps[0].visual_width(), 5);
    }

    #[test]
    fn width_policy_can_customize_tab_advance() {
        let width_policy = WidthPolicy::cjk_grid(2);
        assert_eq!(width_policy.text_width("a\tb"), 4);

        let wraps = wrap_line(
            "a\tb c",
            4,
            &WrapPolicy::code_with_width_policy(width_policy),
        );
        assert_eq!(wraps.len(), 1);
        assert_eq!(wraps[0].byte_offset(), usize_to_u32("a\tb ".len(), "len"));
    }

    #[test]
    fn legacy_wrap_policy_new_keeps_tab_width_in_char_width_callback() {
        fn legacy_width(ch: char) -> u32 {
            if ch == '\t' {
                7
            } else {
                1
            }
        }

        let policy = WrapPolicy::new(legacy_width, code_break_opportunity);
        assert_eq!(policy.char_width()('\t'), 7);
        assert_eq!(policy.width_policy().advance_of('\t'), 7);
    }

    #[test]
    fn visual_layout_space_tracks_inline_and_block_advances() {
        let text = "ab cd";
        let map = WrapMap::new([text].iter().copied(), 3, &WrapPolicy::code());
        let layout = map.visual_layout_space(
            0,
            1,
            text,
            &WrapPolicy::code(),
            &LineLayoutPolicy::horizontal_ltr(),
        );

        assert_eq!(layout.logical_line(), 0);
        assert_eq!(layout.visual_line(), 1);
        assert_eq!(layout.inline_advance(), 2);
        assert_eq!(layout.block_advance(), 1);
        assert_eq!(layout.layout_mode(), LayoutMode::HorizontalLtr);
    }

    #[test]
    fn line_layout_policy_can_redistribute_inline_width() {
        fn justify_to_max(_line_width: u32, max_width: u32) -> u32 {
            max_width
        }

        let text = "ab cd";
        let map = WrapMap::new([text].iter().copied(), 6, &WrapPolicy::code());
        let layout = map.visual_layout_space(
            0,
            0,
            text,
            &WrapPolicy::code(),
            &LineLayoutPolicy::new(LayoutMode::HorizontalLtr, justify_to_max),
        );

        assert_eq!(layout.inline_advance(), 6);
        assert_eq!(layout.layout_mode(), LayoutMode::HorizontalLtr);
    }

    #[test]
    fn code_policy_breaks_after_space_and_operator() {
        let policy = WrapPolicy::code();
        let break_opportunity = policy.break_opportunity();

        assert_eq!(break_opportunity("a + b", 2), BreakOpportunity::Allowed);
        assert_eq!(break_opportunity("a+b", 2), BreakOpportunity::Allowed);
        assert_eq!(break_opportunity("ab", 1), BreakOpportunity::Forbidden);
        assert_eq!(break_opportunity("ab", 0), BreakOpportunity::Forbidden);
    }

    #[test]
    fn japanese_basic_applies_kinsoku() {
        let policy = WrapPolicy::japanese_basic();
        let break_opportunity = policy.break_opportunity();

        assert_eq!(
            break_opportunity("あい", "あ".len()),
            BreakOpportunity::Allowed
        );
        assert_eq!(
            break_opportunity("あ。", "あ".len()),
            BreakOpportunity::Forbidden
        );
        assert_eq!(
            break_opportunity("（あ", "（".len()),
            BreakOpportunity::Forbidden
        );
    }

    #[test]
    fn wrap_map_construction_and_round_trip() {
        let lines = ["ab cd ef", "xyz"];
        let map = WrapMap::new(lines.iter().copied(), 4, &WrapPolicy::code());

        assert_eq!(map.max_width(), 4);
        assert_eq!(map.line_count(), 2);
        assert_eq!(map.visual_line_count(0), 3);
        assert_eq!(map.visual_line_count(1), 1);
        assert_eq!(map.total_visual_lines(), 4);
        assert_eq!(map.to_visual_line(0, 0), 0);
        assert_eq!(map.to_visual_line(0, 3), 1);
        assert_eq!(map.to_visual_line(1, 0), 3);
        assert_eq!(map.from_visual_line(0), (0, 0));
        assert_eq!(map.from_visual_line(1), (0, 3));
        assert_eq!(map.from_visual_line(3), (1, 0));
        assert_eq!(
            map.visual_lines(0, usize_to_u32(lines[0].len(), "line len")),
            vec![
                VisualLine { start: 0, end: 3 },
                VisualLine { start: 3, end: 6 },
                VisualLine {
                    start: 6,
                    end: usize_to_u32(lines[0].len(), "line len"),
                },
            ]
        );
    }

    #[test]
    fn wrap_map_rewrap_line_updates_points() {
        let lines = ["ab cd", "xy z"];
        let mut map = WrapMap::new(lines.iter().copied(), 3, &WrapPolicy::code());
        assert_eq!(map.visual_line_count(0), 2);

        map.rewrap_line(0, "abcd", &WrapPolicy::code());
        assert_eq!(map.wrap_points(0), &[]);
        assert_eq!(map.visual_line_count(0), 1);
    }

    #[test]
    fn wrap_map_splice_lines_replaces_range() {
        let lines = ["ab cd", "xy z"];
        let mut map = WrapMap::new(lines.iter().copied(), 3, &WrapPolicy::code());

        map.splice_lines(1, 1, ["12 34", "p q"].iter().copied(), &WrapPolicy::code());

        assert_eq!(map.line_count(), 3);
        assert_eq!(map.visual_line_count(1), 2);
        assert_eq!(map.visual_line_count(2), 1);
    }

    #[test]
    fn wrap_map_set_max_width_recomputes_all_lines() {
        let lines = ["ab cd", "xy z"];
        let mut map = WrapMap::new(lines.iter().copied(), 10, &WrapPolicy::code());
        assert_eq!(map.total_visual_lines(), 2);

        map.set_max_width(3, lines.iter().copied(), &WrapPolicy::code());
        assert_eq!(map.max_width(), 3);
        assert_eq!(map.total_visual_lines(), 4);
    }
}
