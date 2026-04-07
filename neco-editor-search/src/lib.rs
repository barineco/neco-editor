//! Text search engine for editor buffers.
//!
//! Provides find and replace operations over UTF-8 text, using
//! `neco_textview::LineIndex` for line/column resolution.

use neco_textview::{LineIndex, TextRange};
use regex::Regex;
use std::fmt;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Search parameters. Plain struct (parameter bag).
pub struct SearchQuery {
    pub pattern: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

/// A single search hit with its byte range and line/column position.
#[derive(Debug)]
pub struct SearchMatch {
    range: TextRange,
    line: u32,
    column: u32,
}

impl SearchMatch {
    /// Byte-offset range of the match.
    pub fn range(&self) -> &TextRange {
        &self.range
    }

    /// 0-based line number.
    pub fn line(&self) -> u32 {
        self.line
    }

    /// 0-based column (byte offset within line).
    pub fn column(&self) -> u32 {
        self.column
    }
}

/// Errors produced by search operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SearchError {
    /// The regex pattern failed to compile.
    InvalidRegex(String),
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegex(msg) => write!(f, "invalid regex: {msg}"),
        }
    }
}

impl std::error::Error for SearchError {}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a `regex::Regex` from a `SearchQuery`.
fn build_regex(query: &SearchQuery) -> Result<Regex, SearchError> {
    let mut pat = if query.is_regex {
        query.pattern.clone()
    } else {
        regex::escape(&query.pattern)
    };

    if query.whole_word {
        pat = format!(r"\b(?:{pat})\b");
    }

    if !query.case_sensitive {
        pat = format!("(?i){pat}");
    }

    Regex::new(&pat).map_err(|e| SearchError::InvalidRegex(e.to_string()))
}

/// Create a `SearchMatch` from a byte-offset range using `LineIndex`.
fn match_from_offsets(
    text: &str,
    line_index: &LineIndex,
    start: usize,
    end: usize,
) -> Result<SearchMatch, SearchError> {
    let range = TextRange::new(start, end).expect("match offsets must satisfy start <= end");
    let pos = line_index
        .offset_to_position(text, start)
        // offset_to_position only fails for out-of-bounds / non-boundary offsets,
        // which cannot happen for regex match positions in valid UTF-8 text.
        .expect("regex match start must be a valid offset");
    Ok(SearchMatch {
        range,
        line: pos.line(),
        column: pos.column(),
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Find every occurrence of `query` in `text`.
pub fn find_all(
    text: &str,
    line_index: &LineIndex,
    query: &SearchQuery,
) -> Result<Vec<SearchMatch>, SearchError> {
    let re = build_regex(query)?;
    let mut results = Vec::new();
    for m in re.find_iter(text) {
        results.push(match_from_offsets(text, line_index, m.start(), m.end())?);
    }
    Ok(results)
}

/// Find the first occurrence of `query` at or after `from_offset`.
///
/// Returns `Ok(None)` when no match exists from that offset onward.
pub fn find_next(
    text: &str,
    line_index: &LineIndex,
    query: &SearchQuery,
    from_offset: usize,
) -> Result<Option<SearchMatch>, SearchError> {
    let re = build_regex(query)?;
    if from_offset > text.len() {
        return Ok(None);
    }
    // Use find_at to search within the full haystack, preserving word boundary
    // semantics even when from_offset falls mid-word.
    match re.find_at(text, from_offset) {
        Some(m) => Ok(Some(match_from_offsets(
            text,
            line_index,
            m.start(),
            m.end(),
        )?)),
        None => Ok(None),
    }
}

/// Replace every occurrence of `query` in `text` with `replacement`.
///
/// Returns the new text and the number of replacements performed.
pub fn replace_all(
    text: &str,
    query: &SearchQuery,
    replacement: &str,
) -> Result<(String, usize), SearchError> {
    let re = build_regex(query)?;
    let count = re.find_iter(text).count();
    let new_text = re.replace_all(text, replacement).into_owned();
    Ok((new_text, count))
}

/// Replace the first occurrence of `query` at or after `from_offset`.
///
/// Returns the new full text and the `SearchMatch` describing the original
/// match position, or `None` if no match was found.
pub fn replace_next(
    text: &str,
    line_index: &LineIndex,
    query: &SearchQuery,
    replacement: &str,
    from_offset: usize,
) -> Result<Option<(String, SearchMatch)>, SearchError> {
    let re = build_regex(query)?;
    if from_offset > text.len() {
        return Ok(None);
    }
    // Use find_at to preserve word boundary semantics at mid-word offsets.
    match re.find_at(text, from_offset) {
        Some(m) => {
            let sm = match_from_offsets(text, line_index, m.start(), m.end())?;

            let mut new_text = String::with_capacity(text.len());
            new_text.push_str(&text[..m.start()]);
            new_text.push_str(replacement);
            new_text.push_str(&text[m.end()..]);

            Ok(Some((new_text, sm)))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Convenience: create a query for plain-text, case-sensitive search.
    fn plain(pattern: &str) -> SearchQuery {
        SearchQuery {
            pattern: pattern.to_string(),
            is_regex: false,
            case_sensitive: true,
            whole_word: false,
        }
    }

    // -----------------------------------------------------------------------
    // build_regex
    // -----------------------------------------------------------------------

    #[test]
    fn build_regex_invalid_returns_error() {
        let q = SearchQuery {
            pattern: "[invalid".to_string(),
            is_regex: true,
            case_sensitive: true,
            whole_word: false,
        };
        let err = build_regex(&q).unwrap_err();
        assert!(matches!(err, SearchError::InvalidRegex(_)));
    }

    #[test]
    fn build_regex_plain_escapes_special_chars() {
        let q = SearchQuery {
            pattern: "a.b".to_string(),
            is_regex: false,
            case_sensitive: true,
            whole_word: false,
        };
        let re = build_regex(&q).expect("should compile");
        assert!(re.is_match("a.b"));
        assert!(!re.is_match("axb"));
    }

    #[test]
    fn build_regex_case_insensitive() {
        let q = SearchQuery {
            pattern: "hello".to_string(),
            is_regex: false,
            case_sensitive: false,
            whole_word: false,
        };
        let re = build_regex(&q).expect("should compile");
        assert!(re.is_match("HELLO"));
        assert!(re.is_match("Hello"));
    }

    #[test]
    fn build_regex_whole_word() {
        let q = SearchQuery {
            pattern: "foo".to_string(),
            is_regex: false,
            case_sensitive: true,
            whole_word: true,
        };
        let re = build_regex(&q).expect("should compile");
        assert!(re.is_match("foo"));
        assert!(!re.is_match("foobar"));
        assert!(!re.is_match("barfoo"));
    }

    // -----------------------------------------------------------------------
    // find_all
    // -----------------------------------------------------------------------

    #[test]
    fn find_all_basic_match() {
        let text = "hello world";
        let li = LineIndex::new(text);
        let matches = find_all(text, &li, &plain("world")).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].range().start(), 6);
        assert_eq!(matches[0].range().end(), 11);
        assert_eq!(matches[0].line(), 0);
        assert_eq!(matches[0].column(), 6);
    }

    #[test]
    fn find_all_multiple_matches() {
        let text = "abcabc";
        let li = LineIndex::new(text);
        let matches = find_all(text, &li, &plain("abc")).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].range().start(), 0);
        assert_eq!(matches[1].range().start(), 3);
    }

    #[test]
    fn find_all_no_match() {
        let text = "hello";
        let li = LineIndex::new(text);
        let matches = find_all(text, &li, &plain("xyz")).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn find_all_case_insensitive() {
        let text = "Hello HELLO hello";
        let li = LineIndex::new(text);
        let q = SearchQuery {
            pattern: "hello".to_string(),
            is_regex: false,
            case_sensitive: false,
            whole_word: false,
        };
        let matches = find_all(text, &li, &q).unwrap();
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn find_all_whole_word() {
        let text = "foo foobar barfoo foo";
        let li = LineIndex::new(text);
        let q = SearchQuery {
            pattern: "foo".to_string(),
            is_regex: false,
            case_sensitive: true,
            whole_word: true,
        };
        let matches = find_all(text, &li, &q).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].range().start(), 0);
        assert_eq!(matches[1].range().start(), 18);
    }

    #[test]
    fn find_all_regex_with_groups() {
        let text = "2024-01-15 and 2025-12-31";
        let li = LineIndex::new(text);
        let q = SearchQuery {
            pattern: r"\d{4}-\d{2}-\d{2}".to_string(),
            is_regex: true,
            case_sensitive: true,
            whole_word: false,
        };
        let matches = find_all(text, &li, &q).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].range().start(), 0);
        assert_eq!(matches[0].range().end(), 10);
        assert_eq!(matches[1].range().start(), 15);
    }

    #[test]
    fn find_all_multiline() {
        let text = "line1\nfoo\nline3\nfoo";
        let li = LineIndex::new(text);
        let matches = find_all(text, &li, &plain("foo")).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line(), 1);
        assert_eq!(matches[0].column(), 0);
        assert_eq!(matches[1].line(), 3);
        assert_eq!(matches[1].column(), 0);
    }

    #[test]
    fn find_all_empty_text() {
        let text = "";
        let li = LineIndex::new(text);
        let matches = find_all(text, &li, &plain("x")).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn find_all_empty_pattern() {
        let text = "abc";
        let li = LineIndex::new(text);
        // Empty pattern matches at every position.
        let matches = find_all(text, &li, &plain("")).unwrap();
        assert_eq!(matches.len(), 4); // positions 0,1,2,3
    }

    // -----------------------------------------------------------------------
    // find_next
    // -----------------------------------------------------------------------

    #[test]
    fn find_next_from_zero() {
        let text = "abc def abc";
        let li = LineIndex::new(text);
        let m = find_next(text, &li, &plain("abc"), 0).unwrap().unwrap();
        assert_eq!(m.range().start(), 0);
    }

    #[test]
    fn find_next_from_middle() {
        let text = "abc def abc";
        let li = LineIndex::new(text);
        let m = find_next(text, &li, &plain("abc"), 1).unwrap().unwrap();
        assert_eq!(m.range().start(), 8);
    }

    #[test]
    fn find_next_past_last_match() {
        let text = "abc def abc";
        let li = LineIndex::new(text);
        let m = find_next(text, &li, &plain("abc"), 9).unwrap();
        assert!(m.is_none());
    }

    #[test]
    fn find_next_from_beyond_text() {
        let text = "abc";
        let li = LineIndex::new(text);
        let m = find_next(text, &li, &plain("abc"), 100).unwrap();
        assert!(m.is_none());
    }

    #[test]
    fn find_next_whole_word_mid_word_offset() {
        // "foobar" contains "bar" but starting at offset 3 (mid-word) should
        // NOT match with whole_word because "bar" is not a standalone word here.
        let text = "foobar baz bar";
        let li = LineIndex::new(text);
        let q = SearchQuery {
            pattern: "bar".to_string(),
            is_regex: false,
            case_sensitive: true,
            whole_word: true,
        };
        let m = find_next(text, &li, &q, 3).unwrap().unwrap();
        // Should skip "bar" inside "foobar" and find the standalone "bar" at offset 11.
        assert_eq!(m.range().start(), 11);
    }

    // -----------------------------------------------------------------------
    // replace_all
    // -----------------------------------------------------------------------

    #[test]
    fn replace_all_basic() {
        let text = "hello world";
        let (new_text, count) = replace_all(text, &plain("world"), "rust").unwrap();
        assert_eq!(new_text, "hello rust");
        assert_eq!(count, 1);
    }

    #[test]
    fn replace_all_multiple() {
        let text = "aaa";
        let (new_text, count) = replace_all(text, &plain("a"), "bb").unwrap();
        assert_eq!(new_text, "bbbbbb");
        assert_eq!(count, 3);
    }

    #[test]
    fn replace_all_no_match() {
        let text = "hello";
        let (new_text, count) = replace_all(text, &plain("xyz"), "abc").unwrap();
        assert_eq!(new_text, "hello");
        assert_eq!(count, 0);
    }

    #[test]
    fn replace_all_empty_text() {
        let text = "";
        let (new_text, count) = replace_all(text, &plain("x"), "y").unwrap();
        assert_eq!(new_text, "");
        assert_eq!(count, 0);
    }

    #[test]
    fn replace_all_regex_backreference() {
        let text = "foo123bar456";
        let q = SearchQuery {
            pattern: r"(\d+)".to_string(),
            is_regex: true,
            case_sensitive: true,
            whole_word: false,
        };
        let (new_text, count) = replace_all(text, &q, "[$1]").unwrap();
        assert_eq!(new_text, "foo[123]bar[456]");
        assert_eq!(count, 2);
    }

    // -----------------------------------------------------------------------
    // replace_next
    // -----------------------------------------------------------------------

    #[test]
    fn replace_next_basic() {
        let text = "abc def abc";
        let li = LineIndex::new(text);
        let (new_text, m) = replace_next(text, &li, &plain("abc"), "XYZ", 0)
            .unwrap()
            .unwrap();
        assert_eq!(new_text, "XYZ def abc");
        assert_eq!(m.range().start(), 0);
        assert_eq!(m.range().end(), 3);
    }

    #[test]
    fn replace_next_from_offset() {
        let text = "abc def abc";
        let li = LineIndex::new(text);
        let (new_text, m) = replace_next(text, &li, &plain("abc"), "XYZ", 1)
            .unwrap()
            .unwrap();
        assert_eq!(new_text, "abc def XYZ");
        assert_eq!(m.range().start(), 8);
    }

    #[test]
    fn replace_next_no_match() {
        let text = "hello";
        let li = LineIndex::new(text);
        let result = replace_next(text, &li, &plain("xyz"), "abc", 0).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn replace_next_from_beyond_text() {
        let text = "abc";
        let li = LineIndex::new(text);
        let result = replace_next(text, &li, &plain("abc"), "x", 100).unwrap();
        assert!(result.is_none());
    }

    // -----------------------------------------------------------------------
    // SearchError Display
    // -----------------------------------------------------------------------

    #[test]
    fn search_error_display() {
        let err = SearchError::InvalidRegex("bad pattern".to_string());
        let s = err.to_string();
        assert!(s.contains("bad pattern"));
    }

    // -----------------------------------------------------------------------
    // Edge: invalid regex in public API
    // -----------------------------------------------------------------------

    #[test]
    fn find_all_invalid_regex() {
        let text = "hello";
        let li = LineIndex::new(text);
        let q = SearchQuery {
            pattern: "[".to_string(),
            is_regex: true,
            case_sensitive: true,
            whole_word: false,
        };
        let err = find_all(text, &li, &q).unwrap_err();
        assert!(matches!(err, SearchError::InvalidRegex(_)));
    }

    #[test]
    fn find_next_invalid_regex() {
        let text = "hello";
        let li = LineIndex::new(text);
        let q = SearchQuery {
            pattern: "[".to_string(),
            is_regex: true,
            case_sensitive: true,
            whole_word: false,
        };
        let err = find_next(text, &li, &q, 0).unwrap_err();
        assert!(matches!(err, SearchError::InvalidRegex(_)));
    }

    #[test]
    fn replace_all_invalid_regex() {
        let q = SearchQuery {
            pattern: "[".to_string(),
            is_regex: true,
            case_sensitive: true,
            whole_word: false,
        };
        let err = replace_all("x", &q, "y").unwrap_err();
        assert!(matches!(err, SearchError::InvalidRegex(_)));
    }

    #[test]
    fn replace_next_invalid_regex() {
        let text = "hello";
        let li = LineIndex::new(text);
        let q = SearchQuery {
            pattern: "[".to_string(),
            is_regex: true,
            case_sensitive: true,
            whole_word: false,
        };
        let err = replace_next(text, &li, &q, "y", 0).unwrap_err();
        assert!(matches!(err, SearchError::InvalidRegex(_)));
    }
}
