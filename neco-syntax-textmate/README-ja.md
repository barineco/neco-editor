# neco-syntax-textmate

[英語版](README.md)

syntect ベースの TextMate grammar トークナイザ。ソースの 1 行を型付きスパンに変換し、シンタックスハイライト、diff の色付け、テキストにセマンティックラベルを付ける用途全般に使えます。

## 仕組み

内部で syntect を使い、TextMate のスコープ名を 14 種類の `TokenKind` enum に縮約する設計です。スコープ文字列を直接扱う必要はありません。言語名を渡して行を 1 行ずつ送ると、バイト範囲と種別が付いたスパンが返ってきます。

行をまたぐ解析状態は引き継がれるため、ブロックコメントや複数行文字列も正しく処理されます。ファイルの先頭からの再解析は `reset()` で対応可能です。

`GrammarSet` は grammar をまとめたラッパーです。デフォルトセットには syntect 組み込みの約 50 言語に加え、KDL、Fish、Typst、Nix、Pkl、Mojo が含まれており、実行時に `.sublime-syntax` ファイルを `load_grammar()` で追加読み込みすることも可能です。

## 使い方

```rust
use neco_syntax_textmate::{GrammarSet, SyntaxHighlighter, TokenKind};

let gs = GrammarSet::default_set();
let mut hl = SyntaxHighlighter::new(&gs, "Rust").unwrap();

let spans = hl.tokenize_line("let x = 42;\n");
for span in &spans {
    if span.kind == TokenKind::Keyword {
        // "let" がここに来る
    }
}
```

言語名を直接指定せず、拡張子から推定する場合:

```rust
let lang = gs.detect_language("rs");
let mut hl = SyntaxHighlighter::new(&gs, lang.unwrap()).unwrap();
```

## API

| 項目 | 説明 |
|------|------|
| `TokenKind` | 14 種の enum: `Keyword`, `Type`, `Function`, `String`, `Number`, `Comment`, `Operator`, `Punctuation`, `Variable`, `Constant`, `Tag`, `Attribute`, `Escape`, `Plain` |
| `TokenSpan` | 1 トークンのバイト範囲と `TokenKind` |
| `TokenSpan::kind` | このスパンの `TokenKind` |
| `GrammarSet` | TextMate grammar セットのラッパー |
| `GrammarSet::default_set()` | 組み込み grammar を読み込む（syntect デフォルト + KDL、Fish、Typst、Nix、Pkl、Mojo） |
| `GrammarSet::load_grammar(path)` | `.sublime-syntax` ファイルを追加読み込み。失敗時は `GrammarLoadError` |
| `GrammarSet::detect_language(ext)` | 拡張子から言語名を推定 |
| `GrammarSet::languages()` | 利用可能な言語名の一覧 |
| `SyntaxHighlighter` | 行単位トークナイザ。行をまたいで解析状態を保持 |
| `SyntaxHighlighter::new(gs, lang)` | セットに含まれない言語名のときは `None` |
| `SyntaxHighlighter::tokenize_line(line)` | 1 行をトークン化して `Vec<TokenSpan>` を返す |
| `SyntaxHighlighter::reset()` | 解析状態をファイル先頭に戻す |
| `scope_to_token_kind(scope)` | TextMate スコープ文字列を `TokenKind` に変換 |
| `GrammarLoadError` | `Io` または `Parse` |

## ライセンス

MIT
