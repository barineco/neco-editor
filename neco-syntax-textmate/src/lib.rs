use std::ops::Range;
use std::path::Path;

use syntect::dumps::from_uncompressed_data;
use syntect::parsing::{ParseState, ScopeStack, SyntaxDefinition, SyntaxReference, SyntaxSet};

/// Classifies a token span into a stable highlighting category.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// Matches language keywords and storage scopes.
    Keyword,
    /// Matches named types and type-like support scopes.
    Type,
    /// Matches function and method names.
    Function,
    /// Matches string literal content.
    String,
    /// Matches numeric literals.
    Number,
    /// Matches comment text.
    Comment,
    /// Matches operator scopes before generic keywords.
    Operator,
    /// Matches punctuation and delimiter scopes.
    Punctuation,
    /// Matches variable-like scopes.
    Variable,
    /// Matches non-numeric constant scopes.
    Constant,
    /// Matches tag names such as HTML elements.
    Tag,
    /// Matches attribute and property names.
    Attribute,
    /// Matches escape sequences inside literals.
    Escape,
    /// Fallback when no supported scope prefix matches.
    Plain,
}

/// Describes one byte range and its token kind within a single input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSpan {
    /// Byte range inside the line passed to `SyntaxHighlighter::tokenize_line`.
    pub range: Range<usize>,
    /// Highlighting category inferred for `range`.
    pub kind: TokenKind,
}

/// Reports why loading an external grammar file failed.
#[non_exhaustive]
#[derive(Debug)]
pub enum GrammarLoadError {
    /// Reading the grammar file from disk failed.
    Io(std::io::Error),
    /// Parsing the loaded grammar text failed.
    Parse(String),
}

impl From<std::io::Error> for GrammarLoadError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for GrammarLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for GrammarLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(_) => None,
        }
    }
}

/// Maps a TextMate scope string to the closest supported `TokenKind`.
pub fn scope_to_token_kind(scope: &str) -> TokenKind {
    if scope.starts_with("keyword.operator") {
        TokenKind::Operator
    } else if scope.starts_with("keyword") || scope.starts_with("storage") {
        TokenKind::Keyword
    } else if scope.starts_with("entity.name.function") || scope.starts_with("support.function") {
        TokenKind::Function
    } else if scope.starts_with("entity.name.type")
        || scope.starts_with("entity.name.class")
        || scope.starts_with("entity.name.struct")
        || scope.starts_with("entity.name.enum")
        || scope.starts_with("support.type")
        || scope.starts_with("support.class")
    {
        TokenKind::Type
    } else if scope.starts_with("entity.name.tag") {
        TokenKind::Tag
    } else if scope.starts_with("entity.other.attribute-name") {
        TokenKind::Attribute
    } else if scope.starts_with("string") {
        TokenKind::String
    } else if scope.starts_with("constant.character.escape") {
        TokenKind::Escape
    } else if scope.starts_with("constant.numeric") {
        TokenKind::Number
    } else if scope.starts_with("constant") {
        TokenKind::Constant
    } else if scope.starts_with("comment") {
        TokenKind::Comment
    } else if scope.starts_with("variable") {
        TokenKind::Variable
    } else if scope.starts_with("punctuation") {
        TokenKind::Punctuation
    } else {
        TokenKind::Plain
    }
}

fn resolve_language_alias<'a>(
    syntax_set: &'a SyntaxSet,
    language: &str,
) -> Option<&'a SyntaxReference> {
    syntax_set
        .find_syntax_by_token(language)
        .or_else(|| match language {
            "TypeScript" | "typescript" | "ts" | "tsx" => {
                syntax_set.find_syntax_by_token("JavaScript")
            }
            "KDL" | "kdl" => syntax_set.find_syntax_by_token("KDL2"),
            _ => None,
        })
}

fn display_language_name<'a>(extension: &str, syntax_name: &'a str) -> &'a str {
    match (extension, syntax_name) {
        ("ts", "JavaScript") | ("tsx", "JavaScript") => "TypeScript",
        ("kdl", "KDL2") => "KDL",
        _ => syntax_name,
    }
}

#[derive(Debug, Clone)]
/// Owns the loaded grammar set used to resolve languages and build highlighters.
pub struct GrammarSet {
    syntax_set: SyntaxSet,
}

impl GrammarSet {
    /// Returns the bundled grammar set and panics only if embedded data is invalid.
    pub fn default_set() -> Self {
        let data = include_bytes!(concat!(env!("OUT_DIR"), "/syntaxes.packdump"));
        let syntax_set = from_uncompressed_data(data).expect("embedded syntax dump must be valid");
        Self { syntax_set }
    }

    /// Loads one grammar file and restores the previous set if reading or parsing fails.
    pub fn load_grammar(&mut self, path: &Path) -> Result<(), GrammarLoadError> {
        let original = std::mem::take(&mut self.syntax_set);
        let result = (|| {
            let content = std::fs::read_to_string(path)?;
            let name = path.file_stem().and_then(|stem| stem.to_str());
            let syntax_definition = SyntaxDefinition::load_from_str(&content, true, name)
                .map_err(|error| GrammarLoadError::Parse(error.to_string()))?;
            let mut builder = original.clone().into_builder();
            builder.add(syntax_definition);
            Ok(builder.build())
        })();

        match result {
            Ok(syntax_set) => {
                self.syntax_set = syntax_set;
                Ok(())
            }
            Err(error) => {
                self.syntax_set = original;
                Err(error)
            }
        }
    }

    /// Returns the display language name for an extension when a matching grammar exists.
    pub fn detect_language<'a>(&'a self, extension: &str) -> Option<&'a str> {
        match extension {
            "ts" | "tsx" if self.syntax_set.find_syntax_by_token("JavaScript").is_some() => {
                return Some("TypeScript");
            }
            "kdl" if self.syntax_set.find_syntax_by_token("KDL2").is_some() => {
                return Some("KDL");
            }
            _ => {}
        }

        self.syntax_set
            .find_syntax_by_extension(extension)
            .map(|syntax| display_language_name(extension, syntax.name.as_str()))
    }

    /// Lists all loaded language names in their stored display form.
    pub fn languages(&self) -> Vec<&str> {
        self.syntax_set
            .syntaxes()
            .iter()
            .map(|syntax| syntax.name.as_str())
            .collect()
    }
}

#[derive(Debug, Clone)]
/// Tokenizes lines for one language while preserving multi-line parse state.
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    parse_state: ParseState,
    scope_stack: ScopeStack,
    language_name: String,
}

impl SyntaxHighlighter {
    /// Creates a highlighter for `language`, or returns `None` when no loaded grammar matches it.
    pub fn new(grammar_set: &GrammarSet, language: &str) -> Option<Self> {
        let syntax_set = grammar_set.syntax_set.clone();
        let syntax = resolve_language_alias(&syntax_set, language)?;
        let language_name = syntax.name.clone();

        Some(Self {
            parse_state: ParseState::new(syntax),
            scope_stack: ScopeStack::new(),
            syntax_set,
            language_name,
        })
    }

    /// Tokenizes one line, updates continuation state, and falls back to a plain full-line span on parse errors.
    pub fn tokenize_line(&mut self, line: &str) -> Vec<TokenSpan> {
        let ops = match self.parse_state.parse_line(line, &self.syntax_set) {
            Ok(ops) => ops,
            Err(_) => {
                return vec![TokenSpan {
                    range: 0..line.len(),
                    kind: TokenKind::Plain,
                }];
            }
        };

        let mut spans = Vec::new();
        let mut last_index = 0;

        for (index, op) in &ops {
            let index = (*index).min(line.len());
            if index > last_index {
                let kind = self.current_token_kind();
                Self::push_span(&mut spans, last_index..index, kind);
            }

            if self.scope_stack.apply(op).is_err() {
                return vec![TokenSpan {
                    range: 0..line.len(),
                    kind: TokenKind::Plain,
                }];
            }
            last_index = index;
        }

        if last_index < line.len() {
            let kind = self.current_token_kind();
            Self::push_span(&mut spans, last_index..line.len(), kind);
        }

        spans
    }

    /// Clears parser state so the next tokenized line is treated as a new file start.
    pub fn reset(&mut self) {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(&self.language_name)
            .expect("syntax must exist in owned syntax set");
        self.parse_state = ParseState::new(syntax);
        self.scope_stack = ScopeStack::new();
    }

    fn current_token_kind(&self) -> TokenKind {
        self.scope_stack
            .as_slice()
            .iter()
            .rev()
            .map(|scope| scope_to_token_kind(&scope.build_string()))
            .find(|kind| *kind != TokenKind::Plain)
            .unwrap_or(TokenKind::Plain)
    }

    fn push_span(spans: &mut Vec<TokenSpan>, range: Range<usize>, kind: TokenKind) {
        if range.is_empty() {
            return;
        }

        if let Some(previous) = spans.last_mut() {
            if previous.kind == kind && previous.range.end == range.start {
                previous.range.end = range.end;
                return;
            }
        }

        spans.push(TokenSpan { range, kind });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::error::Error;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn has_token(spans: &[TokenSpan], kind: TokenKind) -> bool {
        spans.iter().any(|span| span.kind == kind)
    }

    fn token_text<'a>(line: &'a str, spans: &[TokenSpan], kind: TokenKind) -> Vec<&'a str> {
        spans
            .iter()
            .filter(|span| span.kind == kind)
            .map(|span| &line[span.range.clone()])
            .collect()
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "neco-syntax-textmate-{name}-{nanos}.sublime-syntax"
        ))
    }

    #[test]
    fn scope_to_token_kind_covers_supported_variants_and_edges() {
        let cases = [
            ("keyword.control.rust", TokenKind::Keyword),
            ("storage.type.function.rust", TokenKind::Keyword),
            ("entity.name.function.rust", TokenKind::Function),
            ("support.function.builtin.python", TokenKind::Function),
            ("entity.name.type.struct.rust", TokenKind::Type),
            ("entity.name.class.typescript", TokenKind::Type),
            ("entity.name.struct.rust", TokenKind::Type),
            ("entity.name.enum.rust", TokenKind::Type),
            ("support.type.primitive.ts", TokenKind::Type),
            ("support.class.python", TokenKind::Type),
            ("entity.name.tag.html", TokenKind::Tag),
            ("entity.other.attribute-name.html", TokenKind::Attribute),
            ("string.quoted.double", TokenKind::String),
            ("constant.character.escape.rust", TokenKind::Escape),
            ("constant.numeric.decimal", TokenKind::Number),
            ("constant.language.boolean", TokenKind::Constant),
            ("comment.line.double-slash", TokenKind::Comment),
            ("variable.parameter.function", TokenKind::Variable),
            ("punctuation.section.block.begin", TokenKind::Punctuation),
            ("", TokenKind::Plain),
            ("meta.embedded.unknown", TokenKind::Plain),
        ];

        for (scope, expected) in cases {
            assert_eq!(scope_to_token_kind(scope), expected, "scope={scope}");
        }
    }

    #[test]
    fn scope_to_token_kind_prefers_operator_before_keyword() {
        assert_eq!(
            scope_to_token_kind("keyword.operator.assignment"),
            TokenKind::Operator
        );
    }

    #[test]
    fn grammar_set_default_set_finds_default_and_bundled_languages() {
        let grammar_set = GrammarSet::default_set();

        assert_eq!(grammar_set.detect_language("rs"), Some("Rust"));
        assert_eq!(grammar_set.detect_language("ts"), Some("TypeScript"));
        assert_eq!(grammar_set.detect_language("json"), Some("JSON"));
        assert_eq!(grammar_set.detect_language("py"), Some("Python"));
        assert_eq!(grammar_set.detect_language("kdl"), Some("KDL"));
        assert_eq!(grammar_set.detect_language("fish"), Some("Fish"));
        assert_eq!(grammar_set.detect_language("nix"), Some("Nix"));
        assert_eq!(grammar_set.detect_language("typ"), Some("Typst"));
        assert_eq!(grammar_set.detect_language("pkl"), Some("Pkl"));
        assert_eq!(grammar_set.detect_language("mojo"), Some("Mojo"));
        assert_eq!(grammar_set.detect_language("does-not-exist"), None);
        assert!(!grammar_set.languages().is_empty());
    }

    #[test]
    fn syntax_highlighter_tokenizes_rust_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "Rust").expect("Rust syntax must exist");

        let line1 = "fn main() {\n";
        let line2 = "    let x = 42;\n";
        let line3 = "}\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);

        assert!(has_token(&spans1, TokenKind::Keyword));
        assert!(token_text(line1, &spans1, TokenKind::Keyword).contains(&"fn"));
        assert!(has_token(&spans1, TokenKind::Function));
        assert!(token_text(line1, &spans1, TokenKind::Function).contains(&"main"));
        assert!(has_token(&spans1, TokenKind::Punctuation));
        assert!(has_token(&spans2, TokenKind::Keyword));
        assert!(token_text(line2, &spans2, TokenKind::Keyword).contains(&"let"));
        assert!(has_token(&spans2, TokenKind::Number));
        assert!(token_text(line2, &spans2, TokenKind::Number).contains(&"42"));
        assert!(has_token(&spans2, TokenKind::Operator));
        assert!(token_text(line2, &spans2, TokenKind::Operator).contains(&"="));
        assert!(has_token(&spans3, TokenKind::Punctuation));
    }

    #[test]
    fn syntax_highlighter_tokenizes_typescript_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter = SyntaxHighlighter::new(&grammar_set, "TypeScript")
            .expect("TypeScript syntax must exist");

        let line1 = "const foo = hello;\n";
        let line2 = "console.log(foo);\n";
        let line3 = "let n = 1;\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);

        assert!(has_token(&spans1, TokenKind::Keyword));
        assert!(token_text(line1, &spans1, TokenKind::Keyword).contains(&"const"));
        assert!(has_token(&spans1, TokenKind::Operator));
        assert!(token_text(line1, &spans1, TokenKind::Operator).contains(&"="));
        assert!(has_token(&spans2, TokenKind::Function));
        assert!(token_text(line2, &spans2, TokenKind::Function).contains(&"log"));
        assert!(has_token(&spans3, TokenKind::Keyword));
        assert!(token_text(line3, &spans3, TokenKind::Keyword).contains(&"let"));
        assert!(has_token(&spans3, TokenKind::Number));
        assert!(token_text(line3, &spans3, TokenKind::Number).contains(&"1"));
    }

    #[test]
    fn syntax_highlighter_tokenizes_python_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "Python").expect("Python syntax must exist");

        let line1 = "def greet(name):\n";
        let line2 = "    print(f\"Hello {name}\")\n";
        let line3 = "    return 7\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);

        assert!(has_token(&spans1, TokenKind::Keyword));
        assert!(token_text(line1, &spans1, TokenKind::Keyword).contains(&"def"));
        assert!(has_token(&spans1, TokenKind::Variable));
        assert!(token_text(line1, &spans1, TokenKind::Variable).contains(&"name"));
        assert!(has_token(&spans1, TokenKind::Punctuation));
        assert!(has_token(&spans2, TokenKind::Function));
        assert!(token_text(line2, &spans2, TokenKind::Function).contains(&"print"));
        assert!(has_token(&spans2, TokenKind::String));
        assert!(token_text(line2, &spans2, TokenKind::String)
            .iter()
            .any(|text| text.contains("Hello")));
        assert!(has_token(&spans3, TokenKind::Keyword));
        assert!(token_text(line3, &spans3, TokenKind::Keyword).contains(&"return"));
        assert!(has_token(&spans3, TokenKind::Number));
        assert!(token_text(line3, &spans3, TokenKind::Number).contains(&"7"));
    }

    #[test]
    fn syntax_highlighter_tokenizes_json_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "JSON").expect("JSON syntax must exist");

        let line1 = "{\"key\": \"value\",\n";
        let line2 = " \"num\": 42\n";
        let line3 = "}\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);

        assert!(has_token(&spans1, TokenKind::String));
        assert!(token_text(line1, &spans1, TokenKind::String)
            .iter()
            .any(|text| text.contains("key")));
        assert!(token_text(line1, &spans1, TokenKind::String)
            .iter()
            .any(|text| text.contains("value")));
        assert!(has_token(&spans1, TokenKind::Punctuation));
        assert!(token_text(line1, &spans1, TokenKind::Punctuation)
            .iter()
            .any(|text| text.contains('{')));
        assert!(has_token(&spans2, TokenKind::String));
        assert!(token_text(line2, &spans2, TokenKind::String)
            .iter()
            .any(|text| text.contains("num")));
        assert!(has_token(&spans2, TokenKind::Number));
        assert!(token_text(line2, &spans2, TokenKind::Number).contains(&"42"));
        assert!(has_token(&spans3, TokenKind::Punctuation));
        assert!(token_text(line3, &spans3, TokenKind::Punctuation)
            .iter()
            .any(|text| text.contains('}')));
    }

    #[test]
    fn syntax_highlighter_tokenizes_kdl_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "KDL").expect("KDL syntax must exist");

        let line1 = "node \"arg\" key=42\n";
        let line2 = "// comment\n";
        let line3 = "{\n";
        let line4 = "}\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);
        let spans4 = highlighter.tokenize_line(line4);

        assert!(has_token(&spans1, TokenKind::String));
        assert!(token_text(line1, &spans1, TokenKind::String)
            .iter()
            .any(|text| text.contains("arg")));
        assert!(has_token(&spans2, TokenKind::Comment));
        assert!(token_text(line2, &spans2, TokenKind::Comment)
            .iter()
            .any(|text| text.contains("comment")));
        assert!(has_token(&spans3, TokenKind::Punctuation));
        assert!(has_token(&spans4, TokenKind::Punctuation));
    }

    #[test]
    fn syntax_highlighter_tokenizes_fish_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "Fish").expect("Fish syntax must exist");

        let line1 = "function greet\n";
        let line2 = "    echo \"Hello\"\n";
        let line3 = "end\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);

        assert!(has_token(&spans1, TokenKind::Keyword));
        assert!(token_text(line1, &spans1, TokenKind::Keyword).contains(&"function"));
        assert!(has_token(&spans2, TokenKind::String));
        assert!(token_text(line2, &spans2, TokenKind::String)
            .iter()
            .any(|text| text.contains("Hello")));
        assert!(has_token(&spans3, TokenKind::Keyword));
        assert!(token_text(line3, &spans3, TokenKind::Keyword).contains(&"end"));
    }

    #[test]
    fn syntax_highlighter_tokenizes_typst_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "Typst").expect("Typst syntax must exist");

        let line1 = "= Heading\n";
        let line2 = "#let x = 42\n";
        let line3 = "Some text\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);

        assert!(!spans1.is_empty() || !spans2.is_empty() || !spans3.is_empty());
        assert!(has_token(&spans2, TokenKind::Number));
        assert!(token_text(line2, &spans2, TokenKind::Number).contains(&"42"));
    }

    #[test]
    fn syntax_highlighter_tokenizes_nix_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "Nix").expect("Nix syntax must exist");

        let line1 = "{ pkgs ? import <nixpkgs> {} }:\n";
        let line2 = "pkgs.mkShell {\n";
        let line3 = "  buildInputs = [ pkgs.hello ];\n";
        let line4 = "}\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);
        let spans4 = highlighter.tokenize_line(line4);

        assert!(has_token(&spans1, TokenKind::Punctuation));
        assert!(token_text(line1, &spans1, TokenKind::Punctuation).contains(&"{"));
        assert!(
            has_token(&spans1, TokenKind::Keyword)
                || has_token(&spans1, TokenKind::Variable)
                || has_token(&spans2, TokenKind::Keyword)
                || has_token(&spans2, TokenKind::Variable)
                || has_token(&spans3, TokenKind::Keyword)
                || has_token(&spans3, TokenKind::Variable)
        );
        assert!(has_token(&spans4, TokenKind::Punctuation));
        assert!(token_text(line4, &spans4, TokenKind::Punctuation).contains(&"}"));
    }

    #[test]
    fn syntax_highlighter_tokenizes_pkl_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "Pkl").expect("Pkl syntax must exist");

        let line1 = "module MyConfig\n";
        let line2 = "name: String = \"hello\"\n";
        let line3 = "if (count > 0) 42 else 0\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);

        assert!(!spans1.is_empty());
        assert!(has_token(&spans2, TokenKind::String));
        assert!(token_text(line2, &spans2, TokenKind::String)
            .iter()
            .any(|text| text.contains("hello")));
        assert!(has_token(&spans3, TokenKind::Keyword));
        assert!(token_text(line3, &spans3, TokenKind::Keyword).contains(&"if"));
        assert!(has_token(&spans3, TokenKind::Number));
        assert!(token_text(line3, &spans3, TokenKind::Number).contains(&"42"));
    }

    #[test]
    fn syntax_highlighter_tokenizes_mojo_key_tokens() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "Mojo").expect("Mojo syntax must exist");

        let line1 = "fn greet(name: String):\n";
        let line2 = "    print(\"Hello\")\n";
        let line3 = "    let x = 42\n";
        let spans1 = highlighter.tokenize_line(line1);
        let spans2 = highlighter.tokenize_line(line2);
        let spans3 = highlighter.tokenize_line(line3);

        assert!(has_token(&spans1, TokenKind::Keyword));
        assert!(token_text(line1, &spans1, TokenKind::Keyword).contains(&"fn"));
        assert!(has_token(&spans2, TokenKind::Function));
        assert!(token_text(line2, &spans2, TokenKind::Function).contains(&"print"));
        assert!(has_token(&spans2, TokenKind::String));
        assert!(token_text(line2, &spans2, TokenKind::String)
            .iter()
            .any(|text| text.contains("Hello")));
        assert!(has_token(&spans3, TokenKind::Number));
        assert!(token_text(line3, &spans3, TokenKind::Number).contains(&"42"));
    }

    #[test]
    fn syntax_highlighter_reset_restores_initial_state() {
        let grammar_set = GrammarSet::default_set();
        let mut highlighter =
            SyntaxHighlighter::new(&grammar_set, "Rust").expect("Rust syntax must exist");

        let prior_line = "fn main() {\n";
        let target_line = "    let x = 42;\n";

        let _ = highlighter.tokenize_line(prior_line);
        let before_reset = highlighter.tokenize_line(target_line);
        highlighter.reset();
        let _ = highlighter.tokenize_line(prior_line);
        let after_reset = highlighter.tokenize_line(target_line);

        assert_eq!(before_reset, after_reset);
    }

    #[test]
    fn syntax_highlighter_accepts_lowercase_language_aliases() {
        let grammar_set = GrammarSet::default_set();

        assert!(
            SyntaxHighlighter::new(&grammar_set, "typescript").is_some(),
            "lowercase 'typescript' must resolve via alias"
        );
        assert!(
            SyntaxHighlighter::new(&grammar_set, "kdl").is_some(),
            "lowercase 'kdl' must resolve via alias"
        );
        assert!(
            SyntaxHighlighter::new(&grammar_set, "rust").is_some(),
            "lowercase 'rust' must resolve via case-insensitive name match"
        );
    }

    #[test]
    fn load_grammar_succeeds_for_valid_syntax_file() {
        let path = unique_temp_path("valid");
        fs::write(
            &path,
            r#"name: MiniSyntax
file_extensions: [mini]
scope: source.mini
contexts:
  main:
    - match: '\b(todo)\b'
      scope: keyword.control.mini
"#,
        )
        .expect("temporary syntax file must be writable");

        let mut grammar_set = GrammarSet::default_set();
        let result = grammar_set.load_grammar(&path);

        assert!(result.is_ok());
        assert_eq!(grammar_set.detect_language("mini"), Some("MiniSyntax"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_grammar_returns_parse_error_for_invalid_syntax_file() {
        let path = unique_temp_path("invalid");
        fs::write(&path, "not: valid: yaml: [")
            .expect("temporary invalid syntax file must be writable");

        let mut grammar_set = GrammarSet::default_set();
        let result = grammar_set.load_grammar(&path);

        assert!(matches!(result, Err(GrammarLoadError::Parse(_))));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_grammar_returns_io_error_for_missing_file() {
        let path = unique_temp_path("missing");
        let mut grammar_set = GrammarSet::default_set();

        let result = grammar_set.load_grammar(&path);

        assert!(matches!(result, Err(GrammarLoadError::Io(_))));
    }

    #[test]
    fn grammar_load_error_formats_and_exposes_sources() {
        let io_error = GrammarLoadError::Io(std::io::Error::other("disk"));
        assert_eq!(io_error.to_string(), "I/O error: disk");
        assert_eq!(
            io_error
                .source()
                .expect("I/O variant must expose its source")
                .to_string(),
            "disk"
        );

        let parse_error = GrammarLoadError::Parse("bad syntax".to_string());
        assert_eq!(parse_error.to_string(), "parse error: bad syntax");
        assert!(parse_error.source().is_none());
    }
}
