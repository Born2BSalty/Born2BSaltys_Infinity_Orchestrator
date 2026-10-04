// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::source_check::{SodDlcState, SourceGame};
use crate::app::state::{Step1State, Step2ComponentState, Step2ModState, Step3ItemState};

use super::super::compat_step3_rules::{Step3CompatMarker, marker_key};
use super::{
    SourceNotice, SourceNoticeSeverity, SourceRemedy, apply_step2, apply_step3,
    invalidate_source_check, preview_issue, refresh_source_check, residue_issue,
    unresolved_sources_issue, unresolved_suffix,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl Deref for TestRoot {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> TestRoot {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join("bio-dlc-source-tests");
    std::fs::create_dir_all(&root).expect("create fixture root");
    let path = root.join(format!(
        "{}-{timestamp}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&path).expect("create unique fixture");
    TestRoot(path)
}

fn archive(root: &Path, relative: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("archive parent"))
        .expect("create archive directory");
    std::fs::write(path, b"synthetic archive existence fixture").expect("write fixture");
}

fn merged_key(root: &Path) {
    std::fs::write(root.join("chitin.key"), b"data/sod-dlc.bif").expect("write merged key");
}

fn source_state(root: &Path, mode: &str) -> Step1State {
    let mut state = Step1State {
        bgee_game_folder: root.to_string_lossy().into_owned(),
        game_install: mode.to_string(),
        ..Step1State::default()
    };
    if mode == "EET" {
        state.eet_bgee_game_folder = root.to_string_lossy().into_owned();
    }
    assert!(refresh_source_check(&mut state));
    state
}

fn component(id: &str, checked: bool) -> Step2ComponentState {
    Step2ComponentState {
        component_id: id.to_string(),
        label: id.to_string(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        is_meta_mode_component: false,
        disabled: false,
        compat_kind: None,
        compat_source: None,
        compat_related_mod: None,
        compat_related_component: None,
        compat_graph: None,
        compat_evidence: None,
        disabled_reason: None,
        checked,
        selected_order: checked.then_some(1),
    }
}

fn mod_state(tp_file: &str, components: &[(&str, bool)]) -> Step2ModState {
    Step2ModState {
        name: tp_file.to_string(),
        tp_file: tp_file.to_string(),
        tp2_path: String::new(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: components.iter().any(|(_, checked)| *checked),
        hidden_components: Vec::new(),
        components: components
            .iter()
            .map(|(id, checked)| component(id, *checked))
            .collect(),
    }
}

fn item(tp_file: &str, id: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: tp_file.to_string(),
        component_id: id.to_string(),
        mod_name: tp_file.to_string(),
        component_label: id.to_string(),
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 1,
        block_id: String::new(),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn markers(
    state: &Step1State,
    tab: &str,
    items: &[Step3ItemState],
    bgee: &[Step3ItemState],
) -> HashMap<String, Step3CompatMarker> {
    let mut result = HashMap::new();
    apply_step3(state, tab, items, bgee, &mut result);
    result
}

fn needs_merge(state: &Step1State) -> bool {
    let tweaks = [item("cdtweaks.tp2", "2010")];
    let result = markers(state, "BGEE", &tweaks, &tweaks);
    result
        .get(&marker_key(&tweaks[0]))
        .is_some_and(|hit| hit.kind == "missing_dep")
}

#[test]
fn detects_root_and_dlc_archives_case_insensitively() {
    for relative in ["sod-dlc.zip", "DLC/SoD-DlC.ZIP"] {
        let root = fixture();
        archive(&root, relative);
        assert!(needs_merge(&source_state(&root, "BGEE")), "{relative}");
    }
}

#[test]
fn no_archive_or_archive_named_directory_does_not_require_merging() {
    let root = fixture();
    assert!(!needs_merge(&source_state(&root, "BGEE")));
    std::fs::create_dir(root.join("sod-dlc.zip")).expect("archive-shaped directory");
    assert!(!needs_merge(&source_state(&root, "BGEE")));
    archive(&root, "unrelated.zip");
    assert!(!needs_merge(&source_state(&root, "BGEE")));
}

#[test]
fn same_source_uses_cache_and_path_changes_clear_stale_requirement() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let mut state = source_state(&root, "BGEE");
    std::fs::rename(root.join("sod-dlc.zip"), root.join("merged-archive.backup"))
        .expect("move synthetic archive");
    assert!(!refresh_source_check(&mut state));
    assert!(needs_merge(&state), "same source must retain cached probe");
    let empty = fixture();
    state.bgee_game_folder = empty.to_string_lossy().into_owned();
    assert!(refresh_source_check(&mut state));
    assert!(!needs_merge(&state));
    state.bgee_game_folder = root.to_string_lossy().into_owned();
    assert!(refresh_source_check(&mut state));
    assert!(!needs_merge(&state), "returning to source must probe again");
}

#[test]
fn empty_and_missing_source_clear_cached_positive() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    for replacement in [
        String::new(),
        root.join("absent").to_string_lossy().into_owned(),
    ] {
        let mut state = source_state(&root, "BGEE");
        state.bgee_game_folder = replacement;
        assert!(refresh_source_check(&mut state));
        assert!(!needs_merge(&state));
        assert!(!refresh_source_check(&mut state));
    }
}

#[test]
fn destination_archive_does_not_trigger_or_satisfy_source_requirement() {
    let source = fixture();
    let destination = fixture();
    archive(&destination, "sod-dlc.zip");
    let mut state = source_state(&source, "EET");
    state.bgee_game_folder.clear();
    state.eet_new_dir = destination.to_string_lossy().into_owned();
    state.eet_pre_dir.clone_from(&state.eet_new_dir);
    state.game.clone_from(&state.eet_new_dir);
    state.bg2ee_game_folder.clone_from(&state.eet_new_dir);
    assert!(refresh_source_check(&mut state));
    assert!(!needs_merge(&state));
    let unmerged = fixture();
    archive(&unmerged, "sod-dlc.zip");
    state.eet_bgee_game_folder = unmerged.to_string_lossy().into_owned();
    assert!(refresh_source_check(&mut state));
    assert!(needs_merge(&state));
    state.eet_new_dir = source.to_string_lossy().into_owned();
    assert!(!refresh_source_check(&mut state));
    assert!(needs_merge(&state));
}

#[test]
fn step2_marks_every_tweaks_component_on_both_tabs_only_when_required() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    for mode in ["BGEE", "EET", "BG2EE", "IWDEE"] {
        let state = source_state(&root, mode);
        let mut first = [mod_state(
            "CDTWEAKS/SETUP-CDTWEAKS.TP2",
            &[("0", false), ("2010", true), ("99999", false)],
        )];
        let mut second = [
            mod_state("cdtweaks.tp2", &[("42", true)]),
            mod_state("unrelated.tp2", &[("0", true)]),
        ];
        apply_step2(&state, &mut first, &mut second);
        let expected = matches!(mode, "BGEE" | "EET").then_some("missing_dep");
        for component in first[0].components.iter().chain(&second[0].components) {
            assert_eq!(component.compat_kind.as_deref(), expected, "{mode}");
            if expected.is_some() {
                assert_eq!(component.compat_related_mod.as_deref(), Some("dlcmerger"));
                assert!(
                    component
                        .compat_evidence
                        .as_deref()
                        .is_some_and(|evidence| evidence.contains("sod-dlc.zip"))
                );
            }
        }
        assert!(second[1].components[0].compat_kind.is_none());
    }
}

#[test]
fn step2_only_checked_bgee_merger_one_or_three_satisfies() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "EET");
    for id in ["1", "2", "3"] {
        for checked in [false, true] {
            for in_bgee in [false, true] {
                let merger = mod_state("DLCMERGER/SETUP-DLCMERGER.TP2", &[(id, checked)]);
                let mut first = vec![mod_state("cdtweaks.tp2", &[("2010", false)])];
                let mut second = vec![mod_state("cdtweaks.tp2", &[("42", true)])];
                if in_bgee {
                    first.push(merger);
                } else {
                    second.push(merger);
                }
                apply_step2(&state, &mut first, &mut second);
                let satisfied = in_bgee && checked && id != "2";
                for mods in [&first, &second] {
                    assert_eq!(
                        mods[0].components[0].compat_kind.is_none(),
                        satisfied,
                        "{id}, checked={checked}, bgee={in_bgee}"
                    );
                }
            }
        }
    }
}

#[test]
fn step3_same_phase_requires_merger_first_for_every_tweaks_component() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "BGEE");
    for id in ["1", "3"] {
        let items = [
            item("cdtweaks.tp2", "2010"),
            item("DLCMERGER.TP2", id),
            item("cdtweaks.tp2", "42"),
        ];
        let result = markers(&state, "BGEE", &items, &items);
        assert_eq!(result.len(), 2);
        assert_eq!(result[&marker_key(&items[0])].kind, "order_block");
        assert_eq!(result[&marker_key(&items[2])].kind, "order_block");
        let reordered = [items[1].clone(), items[0].clone(), items[2].clone()];
        assert_eq!(markers(&state, "BGEE", &reordered, &reordered).len(), 0);
    }
}

#[test]
fn step3_eet_cross_phase_ignores_tab_local_order_numbers() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "EET");
    let second = [item("cdtweaks.tp2", "2010"), item("cdtweaks.tp2", "42")];
    for id in ["1", "2", "3"] {
        let mut merger = item("dlcmerger.tp2", id);
        merger.selected_order = 99;
        let first = [merger];
        let result = markers(&state, "BG2EE", &second, &first);
        assert_eq!(result.len(), if id == "2" { 2 } else { 0 });
        assert!(result.values().all(|hit| hit.kind == "missing_dep"));
    }
    assert_eq!(markers(&state, "BG2EE", &second, &[]).len(), 2);
}

#[test]
fn step3_merger_before_tweaks_but_not_first_is_blocked_on_both_tabs() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "EET");
    for id in ["1", "3"] {
        let first = [
            item("unrelated.tp2", "0"),
            item("dlcmerger.tp2", id),
            item("cdtweaks.tp2", "2010"),
        ];
        for tab in ["BGEE", "BG2EE"] {
            let result = markers(&state, tab, &first[2..], &first);
            assert_eq!(result[&marker_key(&first[2])].kind, "order_block");
        }
        let mut parent = item("dlcmerger.tp2", id);
        parent.is_parent = true;
        let reordered = [parent, first[1].clone(), first[2].clone()];
        assert_eq!(markers(&state, "BGEE", &reordered, &reordered).len(), 0);
    }
}

fn preview(
    game: &str,
    first: &str,
    second: &str,
) -> crate::app::modlist_share::ModlistSharePreview {
    crate::app::modlist_share::ModlistSharePreview {
        bio_version: String::new(),
        game_install: game.to_string(),
        game_version: None,
        install_mode: "custom".to_string(),
        bgee_entries: 0,
        bg2ee_entries: 0,
        has_source_overrides: false,
        has_installed_refs: false,
        bgee_log_text: first.to_string(),
        bg2ee_log_text: second.to_string(),
        source_overrides_text: String::new(),
        installed_refs_text: String::new(),
        mod_config_count: 0,
        mod_configs_text: String::new(),
        allow_auto_install: true,
        name: None,
        author: None,
        description: None,
        forked_from: Vec::new(),
        unresolved_mods: Vec::new(),
    }
}

const TWEAKS_LOG: &str = "~CDTWEAKS/SETUP-CDTWEAKS.TP2~ #0 #2010 // Custom label: v1";
const OTHER_LOG: &str = "~OTHER/OTHER.TP2~ #0 #0 // CDTweaks DLCmerger: v1";
const MERGER_FIRST_LOG: &str = "~DLCMERGER/SETUP-DLCMERGER.TP2~ #0 #1 // DLC Merger: v1";

const MISSING_TEXT: &str = "Your BGEE source contains DLC that needs merging. This modlist includes CDTweaks, which requires DLC Merger for this source.";
const LATE_TEXT: &str = "Move DLC Merger (#1 or #3) first in the BGEE installation order.";
const INFO_FIRST_TEXT: &str = "This modlist needs Siege of Dragonspear. Your BGEE source has the DLC archive; DLC Merger merges it during the install.";
const WARN_MERGED_TEXT: &str = "This modlist includes DLC Merger, but your BGEE source already has Siege of Dragonspear merged. DLC Merger stops with 'already merged'; remove it before installing.";
const WARN_ABSENT_TEXT: &str = "This modlist needs Siege of Dragonspear, and your BGEE source does not have it. DLC Merger will fail; install from a source that includes the DLC.";

fn missing_notice() -> SourceNotice {
    SourceNotice {
        severity: SourceNoticeSeverity::Warning,
        text: MISSING_TEXT.to_string(),
        remedy: SourceRemedy::OrderMerger,
    }
}

fn late_notice() -> SourceNotice {
    SourceNotice {
        severity: SourceNoticeSeverity::Warning,
        text: LATE_TEXT.to_string(),
        remedy: SourceRemedy::OrderMerger,
    }
}

fn info_first_notice() -> SourceNotice {
    SourceNotice {
        severity: SourceNoticeSeverity::Info,
        text: INFO_FIRST_TEXT.to_string(),
        remedy: SourceRemedy::OrderMerger,
    }
}

#[test]
fn preview_uses_preview_game_and_relevant_logs_even_in_custom_mode() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    for current in ["BGEE", "EET", "BG2EE", "IWDEE"] {
        let mut state = source_state(&root, current);
        state.eet_bgee_game_folder = root.to_string_lossy().into_owned();
        for game in ["BGEE", "EET", "BG2EE", "IWDEE"] {
            let expected = matches!(game, "BGEE" | "EET").then(missing_notice);
            assert_eq!(
                preview_issue(&state, &preview(game, TWEAKS_LOG, "")),
                expected
            );
            assert_eq!(
                preview_issue(&state, &preview(game, OTHER_LOG, TWEAKS_LOG)),
                (game == "EET").then(missing_notice)
            );
        }
    }
}

#[test]
fn preview_requires_merger_one_or_three_first_in_bgee_not_other_phase() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let mut state = source_state(&root, "BG2EE");
    state.eet_bgee_game_folder = root.to_string_lossy().into_owned();
    for game in ["BGEE", "EET"] {
        for id in ["1", "2", "3"] {
            let merger =
                format!("~DLCMERGER\\SETUP-DLCMERGER.TP2~ #0 #{id} // Renamed component: v1");
            let first = format!("// Header\n\n{merger}\n{TWEAKS_LOG}");
            let case_first = if id == "2" {
                Some(missing_notice())
            } else {
                Some(info_first_notice())
            };
            assert_eq!(
                preview_issue(&state, &preview(game, &first, "")),
                case_first
            );
            let late = format!("{OTHER_LOG}\n{first}");
            let expected = Some(if id == "2" {
                missing_notice()
            } else {
                late_notice()
            });
            assert_eq!(preview_issue(&state, &preview(game, &late, "")), expected);
            let late = format!("{TWEAKS_LOG}\n{merger}");
            assert_eq!(preview_issue(&state, &preview(game, &late, "")), expected);
            assert_eq!(
                preview_issue(&state, &preview(game, TWEAKS_LOG, &merger)),
                Some(missing_notice())
            );
            if game == "EET" {
                assert_eq!(
                    preview_issue(&state, &preview(game, &merger, TWEAKS_LOG)),
                    case_first
                );
                let late = format!("{OTHER_LOG}\n{merger}");
                assert_eq!(
                    preview_issue(&state, &preview(game, &late, TWEAKS_LOG)),
                    expected
                );
            }
        }
    }
}

#[test]
fn preview_only_reports_relevant_cached_source_issues() {
    let root = fixture();
    let mut state = source_state(&root, "BGEE");
    let candidate = preview("BGEE", TWEAKS_LOG, "");
    assert_eq!(preview_issue(&state, &candidate), None);
    archive(&root, "sod-dlc.zip");
    assert_eq!(
        preview_issue(&state, &candidate),
        None,
        "must not probe on preview"
    );
    state = source_state(&root, "BGEE");
    assert_eq!(preview_issue(&state, &candidate), Some(missing_notice()));
    std::fs::rename(root.join("sod-dlc.zip"), root.join("merged.backup"))
        .expect("move synthetic archive");
    assert_eq!(preview_issue(&state, &candidate), Some(missing_notice()));
    for first in ["", OTHER_LOG, "// ~CDTWEAKS/SETUP-CDTWEAKS.TP2~ #0 #2010"] {
        assert_eq!(preview_issue(&state, &preview("BGEE", first, "")), None);
    }
    state.bgee_game_folder = fixture().to_string_lossy().into_owned();
    assert_eq!(
        preview_issue(&state, &candidate),
        None,
        "stale path must not warn"
    );
}

#[test]
fn preview_notice_is_info_when_merger_is_first_and_archive_unmerged() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "BGEE");
    let log = format!("{MERGER_FIRST_LOG}\n{TWEAKS_LOG}");
    let notice = preview_issue(&state, &preview("BGEE", &log, "")).expect("info notice");
    assert_eq!(notice.severity, SourceNoticeSeverity::Info);
    assert_eq!(notice.text, INFO_FIRST_TEXT);
}

#[test]
fn preview_notice_warns_when_merger_present_on_merged_source() {
    let root = fixture();
    merged_key(&root);
    let state = source_state(&root, "BGEE");
    let notice =
        preview_issue(&state, &preview("BGEE", MERGER_FIRST_LOG, "")).expect("warning notice");
    assert_eq!(notice.severity, SourceNoticeSeverity::Warning);
    assert_eq!(notice.text, WARN_MERGED_TEXT);
    assert_eq!(notice.remedy, SourceRemedy::ChangeSource);
}

#[test]
fn preview_notice_warns_when_merger_present_and_sod_absent() {
    let root = fixture();
    let state = source_state(&root, "BGEE");
    let notice =
        preview_issue(&state, &preview("BGEE", MERGER_FIRST_LOG, "")).expect("warning notice");
    assert_eq!(notice.severity, SourceNoticeSeverity::Warning);
    assert_eq!(notice.text, WARN_ABSENT_TEXT);
    assert_eq!(notice.remedy, SourceRemedy::ChangeSource);
}

#[test]
fn preview_notice_keeps_tweaks_warning_without_merger() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "BGEE");
    let notice = preview_issue(&state, &preview("BGEE", TWEAKS_LOG, "")).expect("warning notice");
    assert_eq!(notice.severity, SourceNoticeSeverity::Warning);
    assert_eq!(notice.text, MISSING_TEXT);
}

#[test]
fn merged_source_with_archive_left_does_not_require_merging() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    merged_key(&root);
    let state = source_state(&root, "BGEE");
    assert!(!needs_merge(&state));
}

#[test]
fn eet_preview_inspects_the_eet_bgee_folder() {
    let merged_root = fixture();
    merged_key(&merged_root);
    let mut state = source_state(&merged_root, "BGEE");
    state.bgee_game_folder.clear();
    let eet_root = fixture();
    archive(&eet_root, "sod-dlc.zip");
    state.eet_bgee_game_folder = eet_root.to_string_lossy().into_owned();
    assert!(refresh_source_check(&mut state));
    let log = format!("{MERGER_FIRST_LOG}\n{TWEAKS_LOG}");
    let notice = preview_issue(&state, &preview("EET", &log, "")).expect("info notice");
    assert_eq!(notice.severity, SourceNoticeSeverity::Info);
    assert_eq!(notice.text, INFO_FIRST_TEXT);
}

#[test]
fn eet_prefers_the_plain_bgee_folder_when_set() {
    let plain_root = fixture();
    archive(&plain_root, "sod-dlc.zip");
    let eet_root = fixture();
    merged_key(&eet_root);
    let mut state = Step1State {
        bgee_game_folder: plain_root.to_string_lossy().into_owned(),
        eet_bgee_game_folder: eet_root.to_string_lossy().into_owned(),
        game_install: "EET".to_string(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    let log = format!("{MERGER_FIRST_LOG}\n{TWEAKS_LOG}");
    let notice = preview_issue(&state, &preview("EET", &log, "")).expect("info notice");
    assert_eq!(notice.severity, SourceNoticeSeverity::Info);
    assert_eq!(notice.text, INFO_FIRST_TEXT);
}

#[test]
fn step3_wrong_phase_and_parent_merger_do_not_satisfy() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "EET");
    let second = [item("dlcmerger.tp2", "1"), item("cdtweaks.tp2", "2010")];
    let mut parent = item("dlcmerger.tp2", "3");
    parent.is_parent = true;
    for first in [vec![], vec![parent]] {
        let result = markers(&state, "BG2EE", &second, &first);
        assert_eq!(result[&marker_key(&second[1])].kind, "missing_dep");
    }
}

#[test]
fn absent_source_dlc_leaves_step2_and_step3_unchanged() {
    let state = source_state(&fixture(), "EET");
    let mut first = [mod_state("cdtweaks.tp2", &[("2010", true)])];
    let mut second = first.clone();
    let before = first.clone();
    apply_step2(&state, &mut first, &mut second);
    assert_eq!(first, before);
    assert_eq!(second, before);
    let items = [item("cdtweaks.tp2", "2010")];
    assert_eq!(markers(&state, "BGEE", &items, &items).len(), 0);
    assert_eq!(markers(&state, "BG2EE", &items, &[]).len(), 0);
}

#[test]
fn existing_markers_win_on_both_steps() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "BGEE");
    let mut first = [mod_state("cdtweaks.tp2", &[("2010", true)])];
    first[0].components[0].compat_kind = Some("conflict".to_string());
    first[0].components[0].disabled_reason = Some("existing reason".to_string());
    let before = first.clone();
    apply_step2(&state, &mut first, &mut []);
    assert_eq!(first, before);
    let items = [item("cdtweaks.tp2", "2010"), item("cdtweaks.tp2", "42")];
    let mut result = markers(&state, "BGEE", &items, &items);
    let key = marker_key(&items[0]);
    result.get_mut(&key).expect("source marker").kind = "conditional".to_string();
    apply_step3(&state, "BGEE", &items, &items, &mut result);
    assert_eq!(result[&key].kind, "conditional");
    let other_key = marker_key(&items[1]);
    assert_eq!(result[&other_key].kind, "missing_dep");
    assert_eq!(result[&other_key].related_mod.as_deref(), Some("dlcmerger"));
}

fn weidu_log(root: &Path) {
    std::fs::write(root.join("WeiDU.log"), b"log").expect("write weidu log");
}

#[test]
fn residue_issue_is_none_for_a_clean_folder() {
    let root = fixture();
    let mut state = Step1State {
        bg2ee_game_folder: root.to_string_lossy().into_owned(),
        game_install: "BG2EE".to_string(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    assert_eq!(residue_issue(&state, "BG2EE"), None);
}

#[test]
fn residue_issue_names_the_folder_and_findings_for_a_modded_bg2ee() {
    let root = fixture();
    weidu_log(&root);
    let mut state = Step1State {
        bg2ee_game_folder: root.to_string_lossy().into_owned(),
        game_install: "BG2EE".to_string(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    let notice = residue_issue(&state, "BG2EE").expect("modded bg2ee notice");
    assert_eq!(notice.severity, SourceNoticeSeverity::Warning);
    assert_eq!(notice.remedy, SourceRemedy::CleanSource);
    let root_display = root.to_string_lossy().into_owned();
    assert_eq!(
        notice.text,
        format!("Your BG2EE source at {root_display} is not a clean install: WeiDU.log.")
    );
}

#[test]
fn invalidate_source_check_reprobes_a_folder_cleaned_in_place() {
    let root = fixture();
    weidu_log(&root);
    let mut state = Step1State {
        bg2ee_game_folder: root.to_string_lossy().into_owned(),
        game_install: "BG2EE".to_string(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    assert!(residue_issue(&state, "BG2EE").is_some());
    std::fs::remove_file(root.join("WeiDU.log")).expect("clean the folder in place");
    assert!(!refresh_source_check(&mut state));
    assert!(
        residue_issue(&state, "BG2EE").is_some(),
        "an unchanged path keeps the cached report"
    );
    invalidate_source_check(&mut state);
    assert!(refresh_source_check(&mut state));
    assert_eq!(residue_issue(&state, "BG2EE"), None);
}

#[test]
fn residue_issue_lists_both_eet_folders_when_both_are_modded() {
    let first_root = fixture();
    weidu_log(&first_root);
    let second_root = fixture();
    weidu_log(&second_root);
    let mut state = Step1State {
        eet_bgee_game_folder: first_root.to_string_lossy().into_owned(),
        eet_bg2ee_game_folder: second_root.to_string_lossy().into_owned(),
        game_install: "EET".to_string(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    let notice = residue_issue(&state, "EET").expect("modded eet pair notice");
    let first_text = first_root.to_string_lossy().into_owned();
    let second_text = second_root.to_string_lossy().into_owned();
    assert_eq!(
        notice.text,
        format!(
            "Your BGEE source at {first_text} is not a clean install: WeiDU.log. Your BG2EE source at {second_text} is not a clean install: WeiDU.log."
        )
    );
}

#[test]
fn residue_issue_is_none_when_no_probe_exists() {
    let root = fixture();
    weidu_log(&root);
    let state = Step1State {
        bg2ee_game_folder: root.to_string_lossy().into_owned(),
        game_install: "BG2EE".to_string(),
        ..Step1State::default()
    };
    assert_eq!(residue_issue(&state, "BG2EE"), None);
}

#[test]
fn refresh_probes_every_configured_game_folder() {
    let roots: Vec<_> = (0..5).map(|_| fixture()).collect();
    let text = |index: usize| roots[index].to_string_lossy().into_owned();
    let mut state = Step1State {
        bgee_game_folder: text(0),
        eet_bgee_game_folder: text(1),
        bg2ee_game_folder: text(2),
        eet_bg2ee_game_folder: text(3),
        iwdee_game_folder: text(4),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    assert_eq!(state.dlc_source_check.probes.len(), 5);
    state.iwdee_game_folder = String::new();
    assert!(refresh_source_check(&mut state));
    assert_eq!(state.dlc_source_check.probes.len(), 4);
}

#[test]
fn one_folder_in_two_game_rows_is_probed_once_and_stays_up_to_date() {
    let root = fixture();
    let mut state = Step1State {
        bgee_game_folder: root.to_string_lossy().into_owned(),
        bg2ee_game_folder: root.to_string_lossy().into_owned(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    assert_eq!(state.dlc_source_check.probes.len(), 1);
    assert!(!refresh_source_check(&mut state));
    assert!(!refresh_source_check(&mut state));
}

#[test]
fn role_swap_reprobes_the_folder() {
    let first_root = fixture();
    let second_root = fixture();
    let first_text = first_root.to_string_lossy().into_owned();
    let second_text = second_root.to_string_lossy().into_owned();
    let mut state = Step1State {
        bgee_game_folder: first_text.clone(),
        bg2ee_game_folder: second_text.clone(),
        game_install: "EET".to_string(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    assert_eq!(
        state.dlc_source_check.probes[&first_text].game,
        SourceGame::Bgee
    );
    assert_eq!(
        state.dlc_source_check.probes[&second_text].game,
        SourceGame::Bg2ee
    );

    state.bgee_game_folder.clone_from(&second_text);
    state.bg2ee_game_folder.clone_from(&first_text);
    assert!(refresh_source_check(&mut state));
    assert_eq!(
        state.dlc_source_check.probes[&first_text].game,
        SourceGame::Bg2ee
    );
    assert_eq!(
        state.dlc_source_check.probes[&second_text].game,
        SourceGame::Bgee
    );
}

#[test]
fn unresolved_suffix_is_empty_for_no_names() {
    assert_eq!(unresolved_suffix(&[]), String::new());
}

#[test]
fn unresolved_suffix_is_singular_for_one_name() {
    assert_eq!(
        unresolved_suffix(&["Alpha".to_string()]),
        " 1 mod has no download source: Alpha."
    );
}

#[test]
fn unresolved_suffix_is_plural_for_several_names() {
    assert_eq!(
        unresolved_suffix(&["Alpha".to_string(), "Beta".to_string()]),
        " 2 mods have no download source: Alpha, Beta."
    );
}

#[test]
fn unresolved_sources_issue_is_none_for_an_empty_list() {
    assert_eq!(unresolved_sources_issue(&[]), None);
}

#[test]
fn unresolved_sources_issue_is_singular_for_one_name() {
    let notice = unresolved_sources_issue(&["Alpha".to_string()]).expect("notice");
    assert_eq!(notice.severity, SourceNoticeSeverity::Warning);
    assert_eq!(notice.remedy, SourceRemedy::None);
    assert_eq!(
        notice.text,
        "1 mod has no download source: Alpha. BIO will not be able to download it."
    );
}

#[test]
fn unresolved_sources_issue_is_plural_for_several_names() {
    let notice =
        unresolved_sources_issue(&["Alpha".to_string(), "Beta".to_string()]).expect("notice");
    assert_eq!(
        notice.text,
        "2 mods have no download source: Alpha, Beta. BIO will not be able to download them."
    );
}

#[test]
fn same_path_first_seen_as_bg2ee_is_reprobed_when_it_becomes_bgee() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let mut state = Step1State {
        bg2ee_game_folder: root.to_string_lossy().into_owned(),
        game_install: "BG2EE".to_string(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    let key = root.to_string_lossy().into_owned();
    assert_eq!(
        state.dlc_source_check.probes[&key].report.sod,
        SodDlcState::NotApplicable
    );

    state.bg2ee_game_folder.clear();
    state.bgee_game_folder = key.clone();
    assert!(refresh_source_check(&mut state));
    assert!(matches!(
        state.dlc_source_check.probes[&key].report.sod,
        SodDlcState::Unmerged { .. }
    ));
}
