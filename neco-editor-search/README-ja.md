# neco-editor-search

[英語版](README.md)

エディタバッファ向けのテキスト検索エンジン。UTF-8 テキスト上でパターン検索と置換を行い、マッチ位置を `neco-textview::LineIndex` 経由で行番号・列番号に変換します。

## 動作の概要

`SearchQuery` は検索パラメータを保持する構造体で、パターン文字列に加え、正規表現モード、大文字小文字の区別、単語単位マッチの各フラグを持ちます。内部で `regex::Regex` にコンパイルし、プレーンテキスト検索ではメタ文字を自動エスケープする仕組みです。

`find_all` はバッファ内の全マッチ、`find_next` は指定バイトオフセット以降の最初のマッチを返す関数で、どちらも `SearchMatch` に行番号と列番号を付与して返却します。

`replace_all` と `replace_next` は置換を行い、新しいテキストを返します。正規表現モードの置換文字列では後方参照 (`$1`, `$2`, ...) も利用可能です。

パターンのコンパイルに失敗した場合は `SearchError::InvalidRegex` を返します。

## 使い方

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

| 項目 | 説明 |
|------|------|
| `SearchQuery` | 検索パラメータ: pattern, is_regex, case_sensitive, whole_word |
| `SearchMatch` | マッチのバイト範囲、行番号、列番号 |
| `SearchMatch::range()` | マッチのバイトオフセット `TextRange` |
| `SearchMatch::line()` | 0 始まりの行番号 |
| `SearchMatch::column()` | 行内の 0 始まりバイト列番号 |
| `SearchError` | パターンコンパイル失敗時の `InvalidRegex(String)` |
| `find_all(text, line_index, query)` | バッファ内の全マッチ |
| `find_next(text, line_index, query, from_offset)` | `from_offset` 以降の最初のマッチ。見つからなければ `None` |
| `replace_all(text, query, replacement)` | 全マッチを置換し `(new_text, count)` を返す |
| `replace_next(text, line_index, query, replacement, from_offset)` | `from_offset` 以降の最初のマッチを置換 |

## ライセンス

MIT
