// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::app::language_names::canonical_language_name;
use crate::app::scan::ScannedComponent;
use crate::app::scan::cache::{ScanCache, cache_get, cache_put};
use crate::app::scan::parse::{normalize_tp_file, parse_component_line};
use crate::app::state::Step2Tp2ProbeReport;
use crate::install::weidu_scan::{self, LanguageEntry};
use crate::parser;

use super::language::candidate_language_ids;

pub(super) struct ScanGroupContext<'a> {
    pub group_label: &'a str,
    pub weidu: &'a Path,
    pub game_dir: &'a Path,
    pub mods_root: &'a Path,
    pub cache: &'a Arc<Mutex<ScanCache>>,
    pub ctx: &'a Arc<String>,
    pub preferred_locale: &'a str,
    pub game_install: &'a str,
}

pub(super) fn scan_tp2_group(
    scan_ctx: &ScanGroupContext<'_>,
    tp2_paths: &[std::path::PathBuf],
) -> Result<(Vec<ScannedComponent>, Vec<Step2Tp2ProbeReport>), String> {
    let mut entries = Vec::<ScannedComponent>::new();
    let mut reports = Vec::<Step2Tp2ProbeReport>::new();
    for tp2 in tp2_paths {
        let work_dir = preferred_scan_work_dir(tp2, scan_ctx.mods_root);
        let mut probe = new_probe(scan_ctx.group_label, tp2, &work_dir);
        let prompt_index = parser::collect_prompt_summary_index(
            tp2,
            scan_ctx.mods_root,
            Some(scan_ctx.preferred_locale),
            Some(scan_ctx.game_install),
        );
        apply_parser_probe_meta(&mut probe, &prompt_index);
        if let Some(cached) = cached_components(scan_ctx, tp2, &prompt_index, &mut probe) {
            entries.extend(cached);
            reports.push(probe);
            continue;
        }
        entries.extend(scan_components_for_tp2(
            scan_ctx,
            tp2,
            &work_dir,
            &prompt_index,
            &mut probe,
        )?);
        reports.push(probe);
    }
    Ok((entries, reports))
}

fn new_probe(group_label: &str, tp2: &Path, work_dir: &Path) -> Step2Tp2ProbeReport {
    Step2Tp2ProbeReport {
        group_label: group_label.to_string(),
        tp2_path: tp2.display().to_string(),
        work_dir: work_dir.display().to_string(),
        ..Step2Tp2ProbeReport::default()
    }
}

fn cached_components(
    scan_ctx: &ScanGroupContext<'_>,
    tp2: &Path,
    prompt_index: &parser::PromptSummaryIndex,
    probe: &mut Step2Tp2ProbeReport,
) -> Option<Vec<ScannedComponent>> {
    let cached = usable_cached_components(cache_get(scan_ctx.cache, scan_ctx.ctx, tp2)?)?;
    probe.used_cache = true;
    probe.selected_from_cache = true;
    let cached = apply_prompt_index(cached, prompt_index);
    probe.parsed_count = cached.len();
    probe.undefined_count = count_undefined_components(&cached);
    Some(cached)
}

fn usable_cached_components(cached: Vec<ScannedComponent>) -> Option<Vec<ScannedComponent>> {
    let every_name_undefined = count_undefined_components(&cached) == cached.len();
    (!every_name_undefined).then_some(cached)
}

fn scan_components_for_tp2(
    scan_ctx: &ScanGroupContext<'_>,
    tp2: &Path,
    work_dir: &Path,
    prompt_index: &parser::PromptSummaryIndex,
    probe: &mut Step2Tp2ProbeReport,
) -> Result<Vec<ScannedComponent>, String> {
    let expected_tp2 = tp2
        .file_name()
        .and_then(|n| n.to_str())
        .map(normalize_tp_file)
        .unwrap_or_default();
    let languages = candidate_language_ids(
        scan_ctx.weidu,
        tp2,
        scan_ctx.game_dir,
        work_dir,
        scan_ctx.preferred_locale,
    )?;
    probe.language_ids_tried = languages.iter().map(|entry| entry.id.clone()).collect();

    let chosen = choose_language_components(languages, |lang_id| {
        let lines = weidu_scan::list_components_lines(
            scan_ctx.weidu,
            scan_ctx.game_dir,
            work_dir,
            tp2,
            lang_id,
        )
        .map_err(|err| {
            format!(
                "failed to list components for {} (language {}): {err}",
                tp2.display(),
                lang_id
            )
        })?;
        Ok(parse_lines_for_tp2(tp2, &expected_tp2, lines))
    })?;
    let Some((language, components)) = chosen else {
        return Ok(Vec::new());
    };
    probe.selected_language_id = Some(language.id);
    let components = apply_prompt_index(components, prompt_index);
    probe.parsed_count = components.len();
    probe.undefined_count = count_undefined_components(&components);
    cache_put(scan_ctx.cache, scan_ctx.ctx, tp2, components.clone());
    Ok(components)
}

type LanguageComponents = (LanguageEntry, Vec<ScannedComponent>);

fn choose_language_components<F>(
    languages: Vec<LanguageEntry>,
    mut components_for: F,
) -> Result<Option<LanguageComponents>, String>
where
    F: FnMut(&str) -> Result<Vec<ScannedComponent>, String>,
{
    let mut fallback = None::<LanguageComponents>;
    for language in languages {
        let parsed = components_for(&language.id)?;
        if parsed.is_empty() {
            continue;
        }
        if count_undefined_components(&parsed) < parsed.len() {
            return Ok(Some(tag_language(language, parsed)));
        }
        if fallback.is_none() {
            fallback = Some((language, parsed));
        }
    }
    Ok(fallback.map(|(language, parsed)| tag_language(language, parsed)))
}

fn tag_language(
    language: LanguageEntry,
    mut components: Vec<ScannedComponent>,
) -> LanguageComponents {
    let name = canonical_language_name(&language.label);
    for component in &mut components {
        component.language_name.clone_from(&name);
    }
    (language, components)
}

fn apply_prompt_index(
    mut components: Vec<ScannedComponent>,
    prompt_index: &parser::PromptSummaryIndex,
) -> Vec<ScannedComponent> {
    if components.is_empty() {
        return components;
    }

    let mut has_component_prompt = false;
    for component in &mut components {
        if let Some(summary) = prompt_index
            .by_component_id
            .get(component.component_id.trim())
        {
            component.prompt_summary = Some(summary.clone());
            has_component_prompt = true;
        }
        component.prompt_events = prompt_index
            .by_component_id_events
            .get(component.component_id.trim())
            .cloned()
            .unwrap_or_default();
    }

    if has_component_prompt {
        let mut lines = Vec::<String>::new();
        for component in &components {
            let Some(summary) = component.prompt_summary.as_deref() else {
                continue;
            };
            let summary = summary.trim();
            if summary.is_empty() {
                continue;
            }
            lines.push(format!("{}:\n{}", component.display.trim(), summary));
        }
        let mod_summary = if lines.is_empty() {
            prompt_index.mod_summary.clone()
        } else {
            Some(lines.join("\n\n"))
        };
        for component in &mut components {
            component.mod_prompt_summary.clone_from(&mod_summary);
            component
                .mod_prompt_events
                .clone_from(&prompt_index.mod_events);
        }
    } else {
        for component in &mut components {
            component
                .mod_prompt_summary
                .clone_from(&prompt_index.mod_summary);
            component
                .mod_prompt_events
                .clone_from(&prompt_index.mod_events);
        }
    }

    components
}

fn apply_parser_probe_meta(
    probe: &mut Step2Tp2ProbeReport,
    prompt_index: &parser::PromptSummaryIndex,
) {
    probe
        .parser_source_file
        .clone_from(&prompt_index.parser_source_file);
    probe.parser_event_count = prompt_index.parser_event_count;
    probe.parser_warning_count = prompt_index.parser_warning_count;
    probe.parser_error_count = prompt_index.parser_error_count;
    probe
        .parser_diagnostic_preview
        .clone_from(&prompt_index.parser_diagnostic_preview);
    probe
        .parser_raw_json
        .clone_from(&prompt_index.parser_raw_json);
    probe
        .parser_tra_language_requested
        .clone_from(&prompt_index.parser_tra_language_requested);
    probe
        .parser_tra_language_used
        .clone_from(&prompt_index.parser_tra_language_used);
    probe.parser_flow_node_count = prompt_index.parser_flow_node_count;
    probe.parser_flow_event_ref_count = prompt_index.parser_flow_event_ref_count;
    probe.parser_event_with_parent_count = prompt_index.parser_event_with_parent_count;
    probe.parser_event_with_path_count = prompt_index.parser_event_with_path_count;
    probe.parser_option_component_binding_count =
        prompt_index.parser_option_component_binding_count;
    probe
        .parser_flow_preview
        .clone_from(&prompt_index.parser_flow_preview);
}

fn preferred_scan_work_dir(tp2: &Path, mods_root: &Path) -> PathBuf {
    if let Some(shared_work_dir) = shared_package_scan_work_dir(tp2, mods_root) {
        return shared_work_dir;
    }
    if let Some(parent) = tp2.parent()
        && let Some(parent_of_parent) = parent.parent()
    {
        return parent_of_parent.to_path_buf();
    }
    if let Some(parent) = tp2.parent() {
        return parent.to_path_buf();
    }
    mods_root.to_path_buf()
}

fn shared_package_scan_work_dir(tp2: &Path, mods_root: &Path) -> Option<PathBuf> {
    let source = fs::read_to_string(tp2).ok()?;
    let source_lower = source.to_ascii_lowercase();
    let tp2_parent = tp2.parent()?;
    let own_folder_name = tp2_parent.file_name().and_then(|s| s.to_str())?;
    let mut current = tp2_parent.parent()?;

    loop {
        if !current.starts_with(mods_root) {
            break;
        }
        let folder_name = current.file_name().and_then(|s| s.to_str())?;
        let wraps_own_folder = folder_name.eq_ignore_ascii_case(own_folder_name);
        if !wraps_own_folder && source_references_shared_root(&source_lower, folder_name) {
            let work_dir = current
                .parent()
                .filter(|p| p.starts_with(mods_root))
                .unwrap_or(mods_root);
            return Some(work_dir.to_path_buf());
        }
        if current == mods_root {
            break;
        }
        let next = current.parent()?;
        if next == current {
            break;
        }
        current = next;
    }

    None
}

fn source_references_shared_root(source_lower: &str, folder_name: &str) -> bool {
    let folder_name = folder_name.to_ascii_lowercase();
    let tilde = format!("~{folder_name}/");
    let quote = format!("\"{folder_name}/");
    source_lower.contains(&tilde) || source_lower.contains(&quote)
}

fn parse_lines_for_tp2(
    tp2: &Path,
    expected_tp2: &str,
    lines: Vec<String>,
) -> Vec<ScannedComponent> {
    let mut out = Vec::<ScannedComponent>::new();
    for line in lines {
        if let Some(mut parsed) = parse_component_line(&line) {
            if let Some(found_tp2) = parsed.tp_file.as_deref()
                && normalize_tp_file(found_tp2) != expected_tp2
            {
                continue;
            }
            if parsed.tp_file.is_none() {
                parsed.tp_file = tp2.file_name().map(|n| n.to_string_lossy().to_string());
            }
            out.push(parsed);
        }
    }
    out
}

fn count_undefined_components(components: &[ScannedComponent]) -> usize {
    components
        .iter()
        .filter(|c| c.display.to_ascii_uppercase().contains("UNDEFINED STRING"))
        .count()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::super::language::order_language_entries;
    use super::{
        LanguageEntry, ScanCache, ScanGroupContext, ScannedComponent, Step2Tp2ProbeReport,
        cache_put, cached_components, choose_language_components, parser, preferred_scan_work_dir,
        usable_cached_components,
    };

    static ROOT_SEQ: AtomicUsize = AtomicUsize::new(0);

    struct TempModsRoot {
        root: PathBuf,
    }

    impl TempModsRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "bio_scanworkdir_test_{}_{}",
                std::process::id(),
                ROOT_SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("mods")).expect("create temp mods root");
            Self { root }
        }

        fn mods(&self) -> PathBuf {
            self.root.join("mods")
        }

        fn write_tp2(&self, relative: &str, source: &str) -> PathBuf {
            let tp2 = self.mods().join(relative);
            fs::create_dir_all(tp2.parent().expect("tp2 has a parent")).expect("create mod folder");
            fs::write(&tp2, source).expect("write tp2");
            tp2
        }
    }

    impl Drop for TempModsRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    const ATWEAKS_SOURCE: &str =
        "BACKUP ~atweaks/backup~\nLANGUAGE ~English~ ~english~ ~atweaks/tra/english/setup.tra~\n";
    const SHARED_PACKAGE_SOURCE: &str = "BACKUP ~moda/backup~\nINCLUDE ~bigpack/lib/shared.tpa~\n";

    fn work_dir_for(root: &TempModsRoot, tp2: &Path) -> PathBuf {
        preferred_scan_work_dir(tp2, &root.mods())
    }

    fn component(display: &str) -> ScannedComponent {
        ScannedComponent {
            tp_file: Some("SETUP-ATWEAKS.TP2".to_string()),
            component_id: "100".to_string(),
            display: display.to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            language_name: String::new(),
        }
    }

    #[test]
    fn nested_same_name_wrapper_runs_from_the_wrapper() {
        let root = TempModsRoot::new();
        let tp2 = root.write_tp2("aTweaks/atweaks/setup-atweaks.tp2", ATWEAKS_SOURCE);
        assert_eq!(work_dir_for(&root, &tp2), root.mods().join("aTweaks"));
    }

    #[test]
    fn nested_same_name_wrapper_ignores_letter_case() {
        let root = TempModsRoot::new();
        let tp2 = root.write_tp2("ATWEAKS/atweaks/setup-atweaks.tp2", ATWEAKS_SOURCE);
        assert_eq!(work_dir_for(&root, &tp2), root.mods().join("ATWEAKS"));
    }

    #[test]
    fn shared_package_still_runs_from_above_the_package() {
        let root = TempModsRoot::new();
        let tp2 = root.write_tp2("bigpack/moda/setup-moda.tp2", SHARED_PACKAGE_SOURCE);
        assert_eq!(work_dir_for(&root, &tp2), root.mods());
    }

    #[test]
    fn same_name_wrapper_inside_a_shared_package_keeps_walking() {
        let root = TempModsRoot::new();
        let tp2 = root.write_tp2("bigpack/moda/moda/setup-moda.tp2", SHARED_PACKAGE_SOURCE);
        assert_eq!(work_dir_for(&root, &tp2), root.mods());
    }

    #[test]
    fn plain_layout_runs_from_the_mods_root() {
        let root = TempModsRoot::new();
        let tp2 = root.write_tp2("atweaks/setup-atweaks.tp2", ATWEAKS_SOURCE);
        assert_eq!(work_dir_for(&root, &tp2), root.mods());
    }

    #[test]
    fn all_undefined_cached_result_is_not_served() {
        let cached = vec![
            component("UNDEFINED STRING:   @100"),
            component("Undefined String:   @101"),
        ];
        assert!(usable_cached_components(cached).is_none());
        assert!(usable_cached_components(Vec::new()).is_none());
    }

    fn serve_from_cache(displays: &[&str]) -> (Option<usize>, bool) {
        let root = TempModsRoot::new();
        let tp2 = root.write_tp2("aTweaks/atweaks/setup-atweaks.tp2", ATWEAKS_SOURCE);
        let mods_root = root.mods();
        let missing = root.root.join("missing");
        let cache = Arc::new(Mutex::new(ScanCache::default()));
        let context = Arc::new("test-context".to_string());
        cache_put(
            &cache,
            &context,
            &tp2,
            displays.iter().map(|display| component(display)).collect(),
        );
        let scan_ctx = ScanGroupContext {
            group_label: "atweaks",
            weidu: &missing,
            game_dir: &missing,
            mods_root: &mods_root,
            cache: &cache,
            ctx: &context,
            preferred_locale: "en_US",
            game_install: "BG2EE",
        };
        let mut probe = Step2Tp2ProbeReport::default();
        let served = cached_components(
            &scan_ctx,
            &tp2,
            &parser::PromptSummaryIndex::default(),
            &mut probe,
        );
        (served.map(|components| components.len()), probe.used_cache)
    }

    #[test]
    fn cache_path_refuses_an_all_undefined_entry_and_serves_a_usable_one() {
        assert_eq!(
            serve_from_cache(&["UNDEFINED STRING:   @100", "UNDEFINED STRING:   @101"]),
            (None, false)
        );
        assert_eq!(
            serve_from_cache(&["UNDEFINED STRING:   @100", "Restore innate infravision"]),
            (Some(2), true)
        );
    }

    fn entry(id: &str, label: &str) -> LanguageEntry {
        LanguageEntry {
            id: id.to_string(),
            label: label.to_string(),
        }
    }

    fn scan_with(languages: Vec<LanguageEntry>, locale: &str) -> (String, Vec<String>) {
        let ordered = order_language_entries(languages, locale);
        let (chosen, components) = choose_language_components(ordered, |id| {
            Ok(vec![component(&format!("Component in language {id}"))])
        })
        .expect("fake listing succeeds")
        .expect("a language is chosen");
        (
            chosen.id,
            components
                .into_iter()
                .map(|component| component.language_name)
                .collect(),
        )
    }

    #[test]
    fn scan_records_chosen_language_name() {
        let with_polish = vec![
            entry("0", "English"),
            entry("1", "Deutsch"),
            entry("2", "Polski"),
        ];
        assert_eq!(
            scan_with(with_polish, "pl_PL"),
            ("2".to_string(), vec!["polish".to_string()])
        );

        let without_polish = vec![entry("0", "Deutsch"), entry("1", "American English")];
        assert_eq!(
            scan_with(without_polish, "pl_PL"),
            ("1".to_string(), vec!["english".to_string()])
        );

        assert_eq!(
            scan_with(Vec::new(), "pl_PL"),
            ("0".to_string(), vec!["english".to_string()])
        );
    }

    #[test]
    fn all_undefined_language_falls_back_to_the_next_defined_one() {
        let languages = vec![entry("0", "Polski"), entry("1", "English")];
        let (chosen, components) = choose_language_components(languages, |id| {
            Ok(vec![component(if id == "0" {
                "UNDEFINED STRING:   @100"
            } else {
                "Restore innate infravision"
            })])
        })
        .expect("fake listing succeeds")
        .expect("a language is chosen");
        assert_eq!(chosen.id, "1");
        assert_eq!(components[0].language_name, "english");
    }

    #[test]
    fn partly_defined_cached_result_is_served() {
        let cached = vec![
            component("UNDEFINED STRING:   @100"),
            component("Restore innate infravision to Half-Orc characters"),
        ];
        assert_eq!(usable_cached_components(cached).map(|c| c.len()), Some(2));
    }
}
