use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use syntect::dumps::dump_to_uncompressed_file;
use syntect::parsing::{SyntaxDefinition, SyntaxSet};

const BUNDLED_GRAMMARS: &[&str] = &[
    "grammars/fish.sublime-syntax",
    "grammars/kdl.sublime-syntax",
    "grammars/nix.sublime-syntax",
    "grammars/typst.sublime-syntax",
    "grammars/pkl/pkl.sublime-syntax",
    "grammars/mojo/mojo.sublime-syntax",
];

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let out_path = out_dir.join("syntaxes.packdump");
    let mut builder = SyntaxSet::load_defaults_newlines().into_builder();

    for grammar in BUNDLED_GRAMMARS {
        println!("cargo:rerun-if-changed={grammar}");

        let grammar_path = manifest_dir.join(grammar);
        let grammar_source = fs::read_to_string(&grammar_path).map_err(|error| {
            io::Error::other(format!(
                "failed to read bundled grammar {}: {error}",
                grammar_path.display()
            ))
        })?;
        let syntax = load_bundled_syntax(&grammar_path, &grammar_source)?;
        builder.add(syntax);
    }
    let syntax_set = builder.build();

    dump_to_uncompressed_file(&syntax_set, out_path)?;
    Ok(())
}

fn load_bundled_syntax(
    grammar_path: &Path,
    grammar_source: &str,
) -> Result<SyntaxDefinition, io::Error> {
    let fallback_name = grammar_path.file_stem().and_then(|stem| stem.to_str());
    match SyntaxDefinition::load_from_str(grammar_source, true, fallback_name) {
        Ok(syntax) => Ok(syntax),
        Err(primary_error) if grammar_path.ends_with("kdl.sublime-syntax") => {
            SyntaxDefinition::load_from_str(KDL_FALLBACK_SYNTAX, true, Some("kdl")).map_err(
                |fallback_error| {
                    io::Error::other(format!(
                        "failed to parse bundled grammar {}: {primary_error}; \
failed to parse KDL fallback grammar: {fallback_error}",
                        grammar_path.display()
                    ))
                },
            )
        }
        Err(error) => Err(io::Error::other(format!(
            "failed to parse bundled grammar {}: {error}",
            grammar_path.display()
        ))),
    }
}

const KDL_FALLBACK_SYNTAX: &str = r#"%YAML 1.2
---
name: KDL2
file_extensions:
  - kdl
  - kdl2
scope: text.kdl.2
contexts:
  main:
    - include: comments
    - include: strings
    - include: numbers
    - match: '\#(?:true|false|null|inf|-inf|nan)\b'
      scope: constant.language.kdl
    - match: '[{}();=]'
      scope: punctuation.separator.kdl
    - match: '[A-Za-z_][A-Za-z0-9_-]*'
      scope: variable.other.kdl
  comments:
    - match: '//'
      push:
        - meta_scope: comment.line.double-slash.kdl
        - match: $
          pop: true
    - match: '/\*'
      push:
        - meta_scope: comment.block.kdl
        - match: '\*/'
          pop: true
  strings:
    - match: '"'
      push:
        - meta_scope: string.quoted.double.kdl
        - match: '\\\\.'
          scope: constant.character.escape.kdl
        - match: '"'
          pop: true
  numbers:
    - match: '[+-]?(?:0x[0-9A-Fa-f_]+|0o[0-7_]+|0b[01_]+|(?:\d[\d_]*)(?:\.\d[\d_]*)?(?:[eE][+-]?\d[\d_]*)?)'
      scope: constant.numeric.kdl
"#;
