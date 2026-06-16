use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=tools/antlr4rust-target-complete.jar");

    let workspace_root = env::current_dir().expect("failed to get workspace root");
    let grammar_root = workspace_root.join("antlr/lapdu-parser/src/main/antlr4");
    let parser_grammar = grammar_root.join("lapdu/LapduCombinedParser.g4");
    let lexer_grammar = grammar_root.join("lapdu/LapduCombinedLexer.g4");
    let imports_dir = grammar_root.join("imports");
    let antlr_jar = workspace_root.join("tools/antlr4rust-target-complete.jar");

    if !parser_grammar.exists() || !lexer_grammar.exists() {
        panic!(
            "Missing grammar files at {}. Ensure submodule is initialized: git submodule update --init --recursive",
            grammar_root.display()
        );
    }
    if !antlr_jar.exists() {
        panic!("Missing ANTLR tool jar at {}", antlr_jar.display());
    }
    track_grammar_inputs(&grammar_root);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let generated_dir = out_dir.join("lapdu_generated");

    // Vendored ANTLR tool jar for Rust target generation.
    // Original source URL:
    // https://github.com/antlr4rust/antlr4/releases/download/v0.5.0/antlr4-4.13.3-SNAPSHOT-complete.jar
    run_antlr(
        &antlr_jar,
        &generated_dir,
        &imports_dir,
        &lexer_grammar,
        &parser_grammar,
    );
    patch_generated_parser_symbols(&generated_dir);
    write_generated_module_glue(&out_dir, &generated_dir);
}

fn track_grammar_inputs(grammar_root: &Path) {
    fn visit(path: &Path) {
        if path.is_dir() {
            for entry in std::fs::read_dir(path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
            {
                let entry = entry.unwrap_or_else(|err| {
                    panic!("failed to read dir entry in {}: {err}", path.display())
                });
                visit(&entry.path());
            }
            return;
        }

        if path.extension().and_then(|e| e.to_str()) == Some("g4") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    visit(grammar_root);
}

fn run_antlr(
    antlr_jar: &Path,
    generated_dir: &Path,
    imports_dir: &Path,
    lexer_grammar: &Path,
    parser_grammar: &Path,
) {
    if generated_dir.exists() {
        std::fs::remove_dir_all(generated_dir).unwrap_or_else(|err| {
            panic!(
                "failed to clean generated output directory {}: {err}",
                generated_dir.display()
            )
        });
    }
    std::fs::create_dir_all(generated_dir).unwrap_or_else(|err| {
        panic!(
            "failed to create generated output directory {}: {err}",
            generated_dir.display()
        )
    });

    let status = Command::new("java")
        .args([
            "-jar",
            antlr_jar
                .to_str()
                .unwrap_or_else(|| panic!("non-utf8 jar path: {}", antlr_jar.display())),
            "-Dlanguage=Rust",
            "-visitor",
            "-Xexact-output-dir",
            "-o",
            generated_dir
                .to_str()
                .unwrap_or_else(|| panic!("non-utf8 output path: {}", generated_dir.display())),
            "-lib",
            imports_dir
                .to_str()
                .unwrap_or_else(|| panic!("non-utf8 imports path: {}", imports_dir.display())),
            lexer_grammar.to_str().unwrap_or_else(|| {
                panic!("non-utf8 lexer grammar path: {}", lexer_grammar.display())
            }),
            parser_grammar.to_str().unwrap_or_else(|| {
                panic!("non-utf8 parser grammar path: {}", parser_grammar.display())
            }),
        ])
        .status()
        .expect("failed to launch java for ANTLR generation");

    assert!(
        status.success(),
        "ANTLR generation failed with status: {status}"
    );

    for expected in [
        "lapducombinedlexer.rs",
        "lapducombinedparser.rs",
        "lapducombinedparserbaselistener.rs",
        "lapducombinedparserbasevisitor.rs",
        "lapducombinedparserlistener.rs",
        "lapducombinedparservisitor.rs",
    ] {
        let expected_path = generated_dir.join(expected);
        assert!(
            expected_path.exists(),
            "ANTLR did not produce expected generated file: {}",
            expected_path.display()
        );
    }
}

fn patch_generated_parser_symbols(generated_dir: &Path) {
    let parser_file = generated_dir.join("lapducombinedparser.rs");
    let source = std::fs::read_to_string(&parser_file).unwrap_or_else(|err| {
        panic!(
            "failed to read generated parser {}: {err}",
            parser_file.display()
        )
    });

    let from = "LapduCombinedParserParserContext";
    let to = "LapduCombinedParserContext";
    let renamed = source.replace(from, to);
    let replaced = strip_generated_condition_parens(&renamed);
    if replaced == source {
        panic!(
            "expected to find '{}' in generated parser {}, but found none",
            from,
            parser_file.display()
        );
    }

    std::fs::write(&parser_file, replaced).unwrap_or_else(|err| {
        panic!(
            "failed to patch generated parser {}: {err}",
            parser_file.display()
        )
    });
}

fn strip_generated_condition_parens(source: &str) -> String {
    source
        .lines()
        .map(strip_generated_condition_parens_line)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn strip_generated_condition_parens_line(line: &str) -> String {
    strip_keyword_condition_parens(line, "while")
        .or_else(|| strip_keyword_condition_parens(line, "if"))
        .unwrap_or_else(|| line.to_owned())
}

fn strip_keyword_condition_parens(line: &str, keyword: &str) -> Option<String> {
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    let rest = &line[indent_len..];
    let after_keyword = rest.strip_prefix(keyword)?.strip_prefix(' ')?;
    if !after_keyword.starts_with('(') {
        return None;
    }

    let mut depth = 0usize;
    let mut close_index = None;
    for (index, character) in after_keyword.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    close_index = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }

    let close_index = close_index?;
    let after_close = &after_keyword[close_index + 1..];
    if after_close.trim_start() != "{" {
        return None;
    }

    let condition = &after_keyword[1..close_index];
    Some(format!("{indent}{keyword} {condition}{after_close}"))
}

fn write_generated_module_glue(out_dir: &Path, generated_dir: &Path) {
    let modules = [
        "lapducombinedlexer",
        "lapducombinedparser",
        "lapducombinedparserlistener",
        "lapducombinedparservisitor",
    ];

    let mut content = String::new();
    for module in modules {
        let module_file = generated_dir.join(format!("{module}.rs"));
        content.push_str(&format!(
            "#[path = r#\"{}\"#]\npub mod {};\n\n",
            module_file.display(),
            module
        ));
    }

    let glue_path = out_dir.join("lapdu_generated_mod.rs");
    std::fs::write(&glue_path, content).unwrap_or_else(|err| {
        panic!(
            "failed to write generated module glue file {}: {err}",
            glue_path.display()
        )
    });
}
