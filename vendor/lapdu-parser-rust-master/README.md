# lapdu-parser-rust

This repository establishes a working pipeline from the external grammar source
to Rust-target ANTLR code generation.

## Source of Truth

- Grammar repository is tracked as submodule: `antlr/lapdu-parser`
- Initialize with:

```bash
git submodule update --init --recursive
```

## Vendored ANTLR Tool

- Jar path: `tools/antlr4rust-target-complete.jar`
- This is the Rust-target ANTLR tool jar.
- Original URL:
  `https://github.com/antlr4rust/antlr4/releases/download/v0.5.0/antlr4-4.13.3-SNAPSHOT-complete.jar`

## Pipeline

Two places run generation against submodule grammars:

1. `build.rs` during `cargo build`
2. `src/main.rs` runtime command during `cargo run`

Both invoke:

```bash
java -jar tools/antlr4rust-target-complete.jar \
  -Dlanguage=Rust -visitor -Xexact-output-dir \
  -o <output-dir> \
  -lib antlr/lapdu-parser/src/main/antlr4/imports \
  antlr/lapdu-parser/src/main/antlr4/lapdu/LapduCombinedLexer.g4 \
  antlr/lapdu-parser/src/main/antlr4/lapdu/LapduCombinedParser.g4
```

`cargo run` writes output to:

- `target/lapdu-runtime-generated`

and verifies key generated files exist.
