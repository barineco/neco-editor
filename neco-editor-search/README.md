# neco-editor-search

[日本語](README-ja.md)

Text search engine for editor buffers. Finds and replaces patterns in UTF-8 text, resolving match positions to line/column via `neco-textview::LineIndex`.

## How it works

`SearchQuery` holds the search parameters: a pattern string plus flags for regex mode, case sensitivity, and whole-word matching. The query compiles into a `regex::Regex` internally. Plain-text patterns are escaped so metacharacters match literally.

`find_all` returns every match in the buffer. `find_next` returns the first match at or after a byte offset, useful for incremental "find next" navigation. Both attach line and column numbers to each `SearchMatch`.

`replace_all` and `replace_next` perform substitution and return the new text. Regex back-references (`$1`, `$2`, ...) work in replacement strings when the query is in regex mode.

All functions return `Result`, propagating `SearchError::InvalidRegex` when the pattern fails to compile.

## Usage

```rust
use neco_editor_search::{SearchQuery, find_all, replace_all};
use neco_textview::LineIndex;

let text = "foo bar foo";
let li = LineIndex::new(text);

let query = SearchQuery {
    pattern: "foo".to_string(),
    is_regex: false,
    case_sensitive: true,
    whole_word: false,
};

let matches = find_all(text, &li, &query).unwrap();
assert_eq!(matches.len(), 2);
assert_eq!(matches[0].line(), 0);
assert_eq!(matches[0].column(), 0);

let (new_text, count) = replace_all(text, &query, "baz").unwrap();
assert_eq!(new_text, "baz bar baz");
assert_eq!(count, 2);
```

## API

| Item | Description |
|------|-------------|
| `SearchQuery` | Search parameters: pattern, is_regex, case_sensitive, whole_word |
| `SearchMatch` | A match with its byte range, line number, and column |
| `SearchMatch::range()` | Byte-offset `TextRange` of the match |
| `SearchMatch::line()` | 0-based line number |
| `SearchMatch::column()` | 0-based byte column within the line |
| `SearchError` | `InvalidRegex(String)` when the pattern fails to compile |
| `find_all(text, line_index, query)` | All matches in the buffer |
| `find_next(text, line_index, query, from_offset)` | First match at or after `from_offset`, or `None` |
| `replace_all(text, query, replacement)` | Replace every match; returns `(new_text, count)` |
| `replace_next(text, line_index, query, replacement, from_offset)` | Replace the first match at or after `from_offset` |

## License

MIT
