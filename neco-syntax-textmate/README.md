# neco-syntax-textmate

[日本語](README-ja.md)

TextMate grammar tokenizer built on syntect: turns a line of source text into typed spans for syntax highlighting, diff coloring, or any tool that needs semantic labels on text.

## How it works

Internally this crate wraps syntect and maps TextMate scope names down to a fixed 14-variant `TokenKind` enum. You don't deal with scope strings in your own code. Pass in a language name, feed lines one at a time, and get back byte-range spans with a kind attached.

State carries over between lines so multiline constructs (block comments, string literals, heredocs) resolve correctly. Call `reset()` to start fresh at the top of a file.

`GrammarSet` bundles the grammars. The default set includes syntect's built-in ~50 languages plus KDL, Fish, Typst, Nix, Pkl, and Mojo. You can also load additional `.sublime-syntax` files at runtime with `load_grammar()`.

## Usage

```rust
use neco_syntax_textmate::{GrammarSet, SyntaxHighlighter, TokenKind};

let gs = GrammarSet::default_set();
let mut hl = SyntaxHighlighter::new(&gs, "Rust").unwrap();

let spans = hl.tokenize_line("let x = 42;\n");
for span in &spans {
    if span.kind == TokenKind::Keyword {
        // "let" is here
    }
}
```

To detect a language from a file extension rather than naming it directly:

```rust
let lang = gs.detect_language("rs");
let mut hl = SyntaxHighlighter::new(&gs, lang.unwrap()).unwrap();
```

## API

| Item | Description |
|------|-------------|
| `TokenKind` | 14-variant enum: `Keyword`, `Type`, `Function`, `String`, `Number`, `Comment`, `Operator`, `Punctuation`, `Variable`, `Constant`, `Tag`, `Attribute`, `Escape`, `Plain` |
| `TokenSpan` | Byte range and `TokenKind` for one token |
| `TokenSpan::kind` | The `TokenKind` for this span |
| `GrammarSet` | Wrapper around a set of TextMate grammars |
| `GrammarSet::default_set()` | Loads the bundled grammars (syntect defaults + KDL, Fish, Typst, Nix) |
| `GrammarSet::load_grammar(path)` | Loads an additional `.sublime-syntax` file; returns `GrammarLoadError` on failure |
| `GrammarSet::detect_language(ext)` | Guesses a language name from a file extension |
| `GrammarSet::languages()` | Lists all available language names |
| `SyntaxHighlighter` | Line-by-line tokenizer; carries parse state across lines |
| `SyntaxHighlighter::new(gs, lang)` | Returns `None` if the language is not in the set |
| `SyntaxHighlighter::tokenize_line(line)` | Tokenizes one line and returns a `Vec<TokenSpan>` |
| `SyntaxHighlighter::reset()` | Resets parse state to the start of a file |
| `scope_to_token_kind(scope)` | Maps a raw TextMate scope string to `TokenKind` |
| `GrammarLoadError` | `Io` or `Parse` variants |

## License

MIT
