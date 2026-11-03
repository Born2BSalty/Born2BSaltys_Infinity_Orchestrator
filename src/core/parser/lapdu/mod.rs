// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use self::model::ParserFlowNode;
use super::PromptSummaryIndex;

mod hidden_prompts;
mod map_to_bio;
mod model;
mod runner;
mod tra_lookup;

#[must_use]
pub fn collect_prompt_summary_index(
    tp2_path: &Path,
    mods_root: &Path,
    preferred_lang: Option<&str>,
    preferred_game: Option<&str>,
) -> PromptSummaryIndex {
    let run = match runner::run_lapdu_parser(tp2_path, mods_root, preferred_lang) {
        Ok(run) => run,
        Err(err) => {
            return PromptSummaryIndex {
                parser_error_count: 1,
                parser_diagnostic_preview: Some(err.message),
                parser_raw_json: err.raw_json,
                ..PromptSummaryIndex::default()
            };
        }
    };
    let mut index = map_to_bio::build_prompt_summary_index(&run.output, preferred_game);
    index.parser_raw_json = Some(run.raw_json);
    index.parser_tra_language_requested = non_empty(&run.output.tra_language_requested);
    index.parser_tra_language_used = non_empty(&run.output.tra_language_used);
    index.parser_flow_node_count = count_flow_nodes(&run.output.flow);
    index.parser_flow_event_ref_count = count_flow_event_refs(&run.output.flow);
    index.parser_event_with_parent_count = run
        .output
        .events
        .iter()
        .filter(|e| e.parent_id.as_deref().is_some_and(|v| !v.trim().is_empty()))
        .count();
    index.parser_event_with_path_count = run
        .output
        .events
        .iter()
        .filter(|e| !e.path_id.trim().is_empty())
        .count();
    index.parser_option_component_binding_count = run
        .output
        .events
        .iter()
        .flat_map(|e| e.options.iter())
        .map(|o| o.component_ids.len())
        .sum();
    index.parser_flow_preview = collect_flow_preview(&run.output.flow, 8);
    hidden_prompts::merge_hidden_prompts(&mut index, tp2_path, preferred_lang, preferred_game);
    index
}

fn collect_flow_preview(nodes: &[ParserFlowNode], limit: usize) -> Vec<(String, String)> {
    let mut out = Vec::<(String, String)>::new();
    let mut stack: Vec<&ParserFlowNode> = nodes.iter().rev().collect();
    while let Some(node) = stack.pop() {
        if out.len() >= limit {
            break;
        }
        out.push((node.id.clone(), node.label.clone()));
        for child in node.children.iter().rev() {
            stack.push(child);
        }
    }
    out
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn count_flow_nodes(nodes: &[ParserFlowNode]) -> usize {
    let mut total = 0usize;
    let mut stack: Vec<&ParserFlowNode> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        total += 1;
        for child in &node.children {
            stack.push(child);
        }
    }
    total
}

fn count_flow_event_refs(nodes: &[ParserFlowNode]) -> usize {
    let mut total = 0usize;
    let mut stack: Vec<&ParserFlowNode> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        total += node.event_ids.len();
        for child in &node.children {
            stack.push(child);
        }
    }
    total
}

#[cfg(test)]
mod prompt_display_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct PromptFixture {
        root: PathBuf,
        tp2: PathBuf,
    }

    impl PromptFixture {
        fn new(include: &str, german: &str, english: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "bio_prompt_display_{}_{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let module = root.join("mods/Sample");
            fs::create_dir_all(module.join("lang/de_DE")).unwrap();
            fs::create_dir_all(module.join("lang/en_US")).unwrap();
            fs::create_dir_all(module.join("lib")).unwrap();
            let tp2 = module.join("Sample.tp2");
            fs::write(
                &tp2,
                "BACKUP ~Sample/backup~\nAUTHOR ~Test~\nLANGUAGE ~English~ ~en_US~ ~Sample/lang/en_US/setup.tra~\nLANGUAGE ~Deutsch~ ~de_DE~ ~Sample/lang/de_DE/setup.tra~\nBEGIN @0 DESIGNATED 0\nINCLUDE ~%MOD_FOLDER%/lib/ask.tph~\n",
            )
            .unwrap();
            fs::write(
                module.join("lang/de_DE/setup.tra"),
                format!("@0 = ~Deutsche Komponente~\n{german}"),
            )
            .unwrap();
            fs::write(
                module.join("lang/en_US/setup.tra"),
                format!("@0 = ~English component~\n{english}"),
            )
            .unwrap();
            fs::write(module.join("lib/ask.tph"), include).unwrap();
            Self { root, tp2 }
        }

        fn summary(&self, locale: &str) -> PromptSummaryIndex {
            collect_prompt_summary_index(
                &self.tp2,
                &self.root.join("mods"),
                Some(locale),
                Some("bg2ee"),
            )
        }
    }

    impl Drop for PromptFixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[test]
    fn prompt_display_eet_keeps_german_question_with_runtime_placeholder() {
        let fixture = PromptFixture::new(
            "PRINT @1 // Detected directory. Is this correct? [Y]es or [N]o\nACTION_READLN response\nACTION_IF ~%response%~ STRING_EQUAL_CASE ~y~ BEGIN PRINT ~ok~ END\n",
            "@1 = ~BG:EE gefunden in %bgee_dir%.\nIst dies korrekt? [Y]Ja oder [N]ein~\n",
            "@1 = ~Detected directory %bgee_dir%. Is this correct? [Y]es or [N]o~\n",
        );
        let summary = fixture.summary("de_DE");
        let text = summary.by_component_id.get("0").unwrap();
        assert!(
            text.contains("BG:EE gefunden in %bgee_dir%.\nIst dies korrekt? [Y]Ja oder [N]ein"),
            "{text}"
        );
        assert!(!text.contains("Detected directory"), "{text}");
    }

    #[test]
    fn prompt_display_function_default_uses_parent_german_tra() {
        let fixture = PromptFixture::new(
            "DEFINE_ACTION_FUNCTION ask\nSTR_VAR question = ~@1~\nBEGIN\nPRINT ~%question%~ // Please enter a number\nACTION_READLN answer\nEND\nLAF ask END\n",
            "@1 = ~Bitte Nummer des Kits eingeben:~\n",
            "@1 = ~Please enter number of the kit:~\n",
        );
        let summary = fixture.summary("de_DE");
        let text = summary
            .by_component_id
            .get("0")
            .or(summary.mod_summary.as_ref())
            .unwrap_or_else(|| panic!("No prompt summary: {summary:?}"));
        assert!(text.contains("Bitte Nummer des Kits eingeben:"), "{text}");
        assert!(!text.contains("Please enter"), "{text}");
    }
    #[test]
    fn prompt_display_preserves_author_text_for_three_locales() {
        let cases = [
            ("en_US", "Directory %bgee_dir%: continue [Y]es or [N]o?"),
            (
                "de_DE",
                "Verzeichnis %bgee_dir%: fortsetzen [Y]Ja oder [N]ein?",
            ),
            ("es_ES", "Directorio %bgee_dir%: continuar [S]í o [N]o?"),
        ];
        for (locale, expected) in cases {
            let fixture = PromptFixture::new(
                "PRINT @1 // Misleading English comment [Y]es or [N]o\nACTION_READLN response\n",
                &format!("@1 = ~{expected}~\n"),
                &format!("@1 = ~{expected}~\n"),
            );
            if locale == "es_ES" {
                let mut source = fs::read_to_string(&fixture.tp2).unwrap();
                source = source.replace(
                    "BEGIN @0",
                    "LANGUAGE ~Spanish~ ~es_ES~ ~Sample/lang/es_ES/setup.tra~\nBEGIN @0",
                );
                fs::create_dir_all(fixture.tp2.parent().unwrap().join("lang/es_ES")).unwrap();
                fs::write(
                    fixture.tp2.parent().unwrap().join("lang/es_ES/setup.tra"),
                    format!("@0 = ~Componente~\n@1 = ~{expected}~\n"),
                )
                .unwrap();
                fs::write(&fixture.tp2, source).unwrap();
            }
            let summary = fixture.summary(locale);
            let text = &summary.by_component_id["0"];
            assert!(text.contains(expected), "{locale}: {text}");
            assert!(!text.contains("Misleading English"), "{locale}: {text}");
        }
    }

    #[test]
    fn prompt_display_literal_percent_sentence_is_not_a_placeholder() {
        let fixture = PromptFixture::new(
            "PRINT ~Chance: 25% oder %chance%?~ // English explanation\nACTION_READLN chance\n",
            "",
            "",
        );
        let index = fixture.summary("de_DE");
        assert!(index.by_component_id["0"].contains("Chance: 25% oder %chance%?"));
        assert!(!index.by_component_id["0"].contains("English explanation"));
    }

    #[test]
    fn prompt_display_whole_unresolved_placeholder_keeps_comment_fallback() {
        let fixture = PromptFixture::new(
            "PRINT ~%unbound_question%~ // Authored fallback question\nACTION_READLN choice\n",
            "",
            "",
        );
        let index = fixture.summary("de_DE");
        assert!(index.by_component_id["0"].contains("Authored fallback question"));
    }

    #[test]
    fn prompt_display_no_comment_preserves_original_translated_template() {
        let fixture = PromptFixture::new(
            "PRINT @1\nACTION_READLN response\n",
            "@1 = ~Bitte Pfad %target_dir% eingeben:~\n",
            "@1 = ~Please enter path %target_dir%:~\n",
        );
        let index = fixture.summary("de_DE");
        assert!(index.by_component_id["0"].contains("Bitte Pfad %target_dir% eingeben:"));
    }

    #[test]
    fn prompt_display_function_forwarding_resolves_explicit_caller_text() {
        let fixture = PromptFixture::new(
            "DEFINE_ACTION_FUNCTION ask\nSTR_VAR question = ~~\nBEGIN\nPRINT ~%question%~ // English fallback\nACTION_READLN answer\nEND\nDEFINE_ACTION_FUNCTION wrapper\nSTR_VAR title = ~@1~\nBEGIN\nLAF ask STR_VAR question = EVAL ~%title%~ END\nEND\nLAF wrapper END\n",
            "@1 = ~Bitte neuen Titel <new_kit_title> eingeben:~\n",
            "@1 = ~Please enter a new title <new_kit_title>:~\n",
        );
        let index = fixture.summary("de_DE");
        let text = index
            .by_component_id
            .get("0")
            .or(index.mod_summary.as_ref())
            .unwrap_or_else(|| panic!("No prompt summary: {index:?}"));
        assert!(
            text.contains("Bitte neuen Titel <new_kit_title> eingeben:"),
            "{text}"
        );
        assert!(!text.contains("English fallback"), "{text}");
    }

    #[test]
    fn prompt_display_function_return_alias_keeps_authored_template() {
        let fixture = PromptFixture::new(
            "DEFINE_ACTION_FUNCTION resolve_template\nSTR_VAR string = ~~\nRET resolved\nBEGIN\nOUTER_SPRINT resolved ~%string%~\nEND\nDEFINE_ACTION_FUNCTION ask\nSTR_VAR question = ~@1~\nBEGIN\nLAF resolve_template STR_VAR string = EVAL ~%question%~ RET current_title_confirm = resolved END\nPRINT ~%current_title_confirm%~ // Accept title [A]ccept, [R]etry, [C]ancel\nACTION_READLN answer\nEND\nLAF ask END\n",
            "@1 = ~Neuen Titel \"<new_kit_title>\" anwenden([A]nwenden, Wiede[R]holen, Abbre[C]hen)?~\n",
            "@1 = ~Accept title \"<new_kit_title>\" ([A]ccept, [R]etry, [C]ancel)?~\n",
        );
        let index = fixture.summary("de_DE");
        let text = index
            .by_component_id
            .get("0")
            .or(index.mod_summary.as_ref())
            .unwrap_or_else(|| panic!("No prompt summary: {index:?}"));
        assert!(
            text.contains(
                "Neuen Titel \"<new_kit_title>\" anwenden([A]nwenden, Wiede[R]holen, Abbre[C]hen)?"
            ),
            "{text}"
        );
        assert!(!text.contains("- a ="), "{text}");
        assert!(!text.contains("Accept title"), "{text}");
    }

    #[test]
    fn prompt_display_conflicting_callers_do_not_choose_another_component_text() {
        let fixture = PromptFixture::new(
            "DEFINE_ACTION_FUNCTION ask\nSTR_VAR question = ~~\nBEGIN\nPRINT ~%question%~\nACTION_READLN answer\nEND\nLAF ask STR_VAR question = ~@1~ END\nLAF ask STR_VAR question = ~@2~ END\n",
            "@1 = ~Erste deutsche Frage~\n@2 = ~Zweite deutsche Frage~\n",
            "@1 = ~First English question~\n@2 = ~Second English question~\n",
        );
        let index = fixture.summary("de_DE");
        let text = index.by_component_id.get("0").cloned().unwrap_or_default();
        assert!(!text.contains("Erste deutsche Frage"), "{text}");
        assert!(!text.contains("Zweite deutsche Frage"), "{text}");
    }

    #[test]
    fn prompt_display_cyclic_bindings_terminate_without_inventing_text() {
        let fixture = PromptFixture::new(
            "OUTER_SPRINT question ~%other%~\nOUTER_SPRINT other ~%question%~\nPRINT ~%question%~\nACTION_READLN response\n",
            "@1 = ~Nicht verwendete Frage~\n",
            "@1 = ~Unused question~\n",
        );
        let index = fixture.summary("de_DE");
        let text = index.by_component_id.get("0").cloned().unwrap_or_default();
        assert!(!text.contains("Nicht verwendete Frage"));
        assert_eq!(index.parser_error_count, 0);
    }

    #[test]
    fn prompt_display_locale_changes_preserve_source_branch_identity() {
        let fixture = PromptFixture::new(
            "ACTION_IF GAME_IS ~bg2ee~ BEGIN\nPRINT @1\nACTION_READLN response\nEND\n",
            "@1 = ~Deutsche Frage %value%?~\n",
            "@1 = ~English question %value%?~\n",
        );
        let german = fixture.summary("de_DE");
        let english = fixture.summary("en_US");
        let left = german.by_component_id_events["0"].first().unwrap();
        let right = english.by_component_id_events["0"].first().unwrap();
        assert!(
            left.source_file.ends_with("ask.tph"),
            "{}",
            left.source_file
        );
        assert_eq!(left.line, Some(3));
        assert_eq!(left.source_file, right.source_file);
        assert_eq!(left.node_id, right.node_id);
        assert_eq!(left.line, right.line);
        assert_eq!(left.branch_path, right.branch_path);
        assert_eq!(left.condition, right.condition);
    }

    fn english_only_fixture(question: &str) -> PromptFixture {
        let fixture = PromptFixture::new(
            "PRINT @1\nACTION_READLN answer\n",
            "@1 = ~German text must not be selected~\n",
            &format!("@1 = ~{question}~\n"),
        );
        let source = fs::read_to_string(&fixture.tp2).unwrap().replace(
            "LANGUAGE ~Deutsch~ ~de_DE~ ~Sample/lang/de_DE/setup.tra~\n",
            "",
        );
        let source = source.replace(
            "~en_US~ ~Sample/lang/en_US/setup.tra~",
            "~english~ ~Sample/lang/english/setup.tra~",
        );
        fs::write(&fixture.tp2, source).unwrap();
        fs::rename(
            fixture.root.join("mods/Sample/lang/en_US"),
            fixture.root.join("mods/Sample/lang/english"),
        )
        .unwrap();
        fs::remove_dir_all(fixture.root.join("mods/Sample/lang/de_DE")).unwrap();
        fixture
    }

    #[test]
    fn prompt_display_randomiser_english_fallback_remains_authored_english() {
        let expected = "Please enter the chance for items to be randomly lost as a integer number (e.g. 10 for 10%)";
        let fixture = english_only_fixture(expected);
        let index = fixture.summary("de_DE");
        assert!(index.by_component_id["0"].contains(expected), "{index:?}");
        assert!(!index.by_component_id["0"].contains("German text"));
    }

    #[test]
    fn prompt_display_vienxay_english_fallback_remains_authored_english() {
        let expected = "Choose a portrait for Vienxay (check mod folder for portraits):";
        let fixture = english_only_fixture(expected);
        let index = fixture.summary("de_DE");
        assert!(index.by_component_id["0"].contains(expected), "{index:?}");
        assert!(!index.by_component_id["0"].contains("German text"));
    }

    #[test]
    fn prompt_display_optional_actual_mod_sources_preserve_author_questions() {
        let Some(mods) = std::env::var_os("BIO_PROMPT_DISPLAY_MODS_ROOT") else {
            return;
        };
        let mods = std::path::PathBuf::from(mods);
        let eet = collect_prompt_summary_index(
            &mods.join("EET/EET.tp2"),
            &mods,
            Some("de_DE"),
            Some("bg2ee"),
        );
        let a7 = collect_prompt_summary_index(
            &mods.join("A7-MultiKits/A7-MultiKits.tp2"),
            &mods,
            Some("de_DE"),
            Some("bg2ee"),
        );
        assert_eq!(eet.parser_error_count, 0);
        assert_eq!(a7.parser_error_count, 0);
        let eet_text = eet.by_component_id.get("0").expect("EET core");
        assert!(
            eet_text.contains("BG:EE gefunden in %bgee_dir%."),
            "{eet_text}"
        );
        assert!(
            eet_text.contains("Gib den kompletten Pfad zu deiner BG:EE+SoD-Installation ein"),
            "{eet_text}"
        );
        assert!(!eet_text.contains("Detected BG:EE directory"), "{eet_text}");
        let a7_text = a7
            .by_component_id_events
            .get("11")
            .expect("A7 manual selection")
            .iter()
            .map(|event| event.summary_line.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for expected in [
            "Bitte Nummer des Kits eingeben (leere Eingabe, um mit der Installation fortzusetzen):",
            "Installation fortsetzen ([Y]Ja, [N]ein, [C]Abbruch)?",
            "Es wurden keine Kits ausgewaehlt. Installation abbrechen ([Y]Ja, [N]ein)?",
            "Kit \"<kit_title>\" deselektieren ([Y]Ja, [N]ein)?",
            "Bitte neuen Titel des ausgewaehlten Kits eingeben (leere Eingabe fuer aktuellen Titel):",
            "Neuen Titel \"<new_kit_title>\" anwenden([A]nwenden, Wiede[R]holen, Abbre[C]hen)?",
        ] {
            assert!(a7_text.contains(expected), "Missing {expected}: {a7_text}");
        }
        assert!(!a7_text.contains("Please enter number"), "{a7_text}");
        assert!(!a7_text.contains("- y = Yes"), "{a7_text}");
        println!("ACTUAL EET GERMAN:\n{eet_text}\nACTUAL A7 GERMAN:\n{a7_text}");
        for (path, component, expected) in [
            (
                "randomiser/randomiser.tp2",
                "1400",
                "Please enter the chance for items to be randomly lost as a integer number (e.g. 10 for 10%)",
            ),
            (
                "Vienxay/Vienxay.tp2",
                "0",
                "Choose a portrait for Vienxay (check mod folder for portraits):",
            ),
        ] {
            let index =
                collect_prompt_summary_index(&mods.join(path), &mods, Some("de_DE"), Some("bg2ee"));
            assert_eq!(index.parser_error_count, 0, "{path}");
            let summary = index.by_component_id.get(component).expect(path);
            assert!(summary.contains(expected), "{path}: {summary}");
            println!("ACTUAL ENGLISH FALLBACK {path}:\n{summary}");
        }
    }

    #[test]
    fn prompt_display_duplicate_ret_helpers_resolve_only_when_definitions_agree() {
        let first = "DEFINE_ACTION_FUNCTION resolve_template\nSTR_VAR string = ~~\nRET result\nBEGIN\nOUTER_SPRINT result ~%string%~\nEND\n";
        for (second_body, resolved) in [
            ("OUTER_SPRINT result ~%string%~", true),
            ("OUTER_SPRINT result ~Different authored answer~", false),
        ] {
            let second = format!(
                "DEFINE_ACTION_FUNCTION resolve_template\nSTR_VAR string = ~~\nRET result\nBEGIN\n{second_body}\nEND\n"
            );
            let include = format!(
                "{first}{second}DEFINE_ACTION_FUNCTION ask\nSTR_VAR question = ~@1~\nBEGIN\nLAF resolve_template STR_VAR string = EVAL ~%question%~ RET shown = result END\nPRINT ~%shown%~\nACTION_READLN answer\nEND\nLAF ask END\n"
            );
            let fixture = PromptFixture::new(
                &include,
                "@1 = ~Originale Frage <kit_title>?~\n",
                "@1 = ~Original question <kit_title>?~\n",
            );
            let index = fixture.summary("de_DE");
            let events = index
                .by_component_id_events
                .values()
                .flatten()
                .chain(index.mod_events.iter())
                .map(|event| event.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!events.is_empty(), "{index:?}");
            assert_eq!(
                events.contains("Originale Frage <kit_title>?"),
                resolved,
                "{events}"
            );
            assert!(!events.contains("Different authored answer"), "{events}");
        }
    }
}
