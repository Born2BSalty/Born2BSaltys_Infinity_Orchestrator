// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::app::state::Step2ComponentState;

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "bio_install_language_rewrite_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create temp root");
        Self { path }
    }

    fn log(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, text).expect("write log");
        path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn component(raw_line: &str) -> Step2ComponentState {
    Step2ComponentState {
        component_id: "2".to_string(),
        label: "Comp".to_string(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: raw_line.to_string(),
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
        checked: false,
        selected_order: None,
    }
}

fn mod_state(tp_file: &str, raw_lines: &[&str]) -> Step2ModState {
    Step2ModState {
        name: tp_file.to_string(),
        tp_file: tp_file.to_string(),
        tp2_path: format!("/mods/{tp_file}"),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: false,
        hidden_components: Vec::new(),
        components: raw_lines.iter().map(|line| component(line)).collect(),
    }
}

fn state_with_logs(
    game_install: &str,
    first: Option<&PathBuf>,
    second: Option<&PathBuf>,
) -> WizardState {
    let mut state = WizardState::default();
    state.step1.game_install = game_install.to_string();
    state.step1.have_weidu_logs = true;
    if let Some(path) = first {
        state.step1.bgee_log_file = path.to_string_lossy().into_owned();
    }
    if let Some(path) = second {
        state.step1.bg2ee_log_file = path.to_string_lossy().into_owned();
    }
    state
}

fn read(path: &PathBuf) -> String {
    fs::read_to_string(path).expect("read log")
}

#[test]
fn rewrite_sets_scan_language_and_install_language_marker() {
    let root = TempRoot::new();
    let log = root.log(
        "weidu.log",
        "~MOD/MOD.TP2~ #0 #2 // Comp: v1 // @wlb-inputs: y\n",
    );
    let mut state = state_with_logs("BGEE", Some(&log), None);
    state.step2.bgee_mods = vec![mod_state(
        "MOD.TP2",
        &["~MOD/MOD.TP2~ #1 #2 // Comp: v1 // @wlb-inputs: t"],
    )];

    assert_eq!(rewrite_imported_logs(&state), Ok(1));
    assert_eq!(
        read(&log),
        "~MOD/MOD.TP2~ #1 #2 // Comp: v1 // @wlb-inputs: t\n"
    );
}

#[test]
fn rewrite_strips_marker_when_scan_has_none() {
    let root = TempRoot::new();
    let log = root.log(
        "weidu.log",
        "~MOD/MOD.TP2~ #0 #2 // Comp: v1 // @wlb-inputs: y\n",
    );
    let mut state = state_with_logs("BGEE", Some(&log), None);
    state.step2.bgee_mods = vec![mod_state("MOD.TP2", &["~MOD/MOD.TP2~ #1 #2 // Comp: v1"])];

    assert_eq!(rewrite_imported_logs(&state), Ok(1));
    assert_eq!(read(&log), "~MOD/MOD.TP2~ #1 #2 // Comp: v1\n");
}

#[test]
fn rewrite_keeps_lines_of_mods_the_scan_lacks_verbatim() {
    let root = TempRoot::new();
    let original = "// Log of Currently Installed WeiDU Mods\r\n\
                    ~OTHER/OTHER.TP2~   #0  #7 // Other:   v2 //   @wlb-inputs:  y , 2  \r\n\
                    ~MOD/MOD.TP2~ #0 #2 // Comp: v1 // @wlb-inputs: y\r\n\
                    ~MOD/MOD.TP2~ #0 #9 // Gone: v1 // @wlb-inputs: y";
    let log = root.log("weidu.log", original);
    let mut state = state_with_logs("BGEE", Some(&log), None);
    state.step2.bgee_mods = vec![mod_state(
        "MOD.TP2",
        &["~MOD/MOD.TP2~ #1 #2 // Comp: v1 // @wlb-inputs: t"],
    )];

    assert_eq!(rewrite_imported_logs(&state), Ok(1));
    assert_eq!(
        read(&log),
        "// Log of Currently Installed WeiDU Mods\r\n\
         ~OTHER/OTHER.TP2~   #0  #7 // Other:   v2 //   @wlb-inputs:  y , 2  \r\n\
         ~MOD/MOD.TP2~ #1 #2 // Comp: v1 // @wlb-inputs: t\r\n\
         ~MOD/MOD.TP2~ #0 #9 // Gone: v1 // @wlb-inputs: y"
    );
}

#[test]
fn rewrite_leaves_ambiguous_components() {
    let root = TempRoot::new();
    let original = "~MOD/MOD.TP2~ #0 #2 // Comp: v1 // @wlb-inputs: y\n";
    let log = root.log("weidu.log", original);
    let mut state = state_with_logs("BGEE", Some(&log), None);
    state.step2.bgee_mods = vec![
        mod_state(
            "MOD.TP2",
            &["~MOD/MOD.TP2~ #1 #2 // Comp: v1 // @wlb-inputs: t"],
        ),
        mod_state("MOD.TP2", &["~MOD\\MOD.TP2~ #3 #2 // Comp: v1"]),
    ];

    assert_eq!(rewrite_imported_logs(&state), Ok(0));
    assert_eq!(read(&log), original);
}

#[test]
fn rewrite_never_writes_a_marker_with_a_line_break() {
    let root = TempRoot::new();
    let log = root.log(
        "weidu.log",
        "~MOD/MOD.TP2~ #0 #2 // Comp: v1 // @wlb-inputs: y\n~NEXT/NEXT.TP2~ #0 #1 // Next: v1\n",
    );
    let mut state = state_with_logs("BGEE", Some(&log), None);
    state.step2.bgee_mods = vec![mod_state(
        "MOD.TP2",
        &["~MOD/MOD.TP2~ #1 #2 // Comp: v1 // @wlb-inputs: t\nINJECTED"],
    )];

    assert_eq!(rewrite_imported_logs(&state), Ok(1));
    assert_eq!(
        read(&log),
        "~MOD/MOD.TP2~ #1 #2 // Comp: v1\n~NEXT/NEXT.TP2~ #0 #1 // Next: v1\n"
    );
}

#[test]
fn rewrite_covers_both_eet_slots() {
    let root = TempRoot::new();
    let first = root.log("bgee.log", "~A/A.TP2~ #0 #5 // A: v1 // @wlb-inputs: y\n");
    let second = root.log(
        "bg2ee.log",
        "~B/B.TP2~ #0 #3 // B: v1\n~A/A.TP2~ #0 #5 // A: v1 // @wlb-inputs: y\n",
    );
    let mut state = state_with_logs("EET", Some(&first), Some(&second));
    state.step2.bgee_mods = vec![mod_state(
        "A.TP2",
        &["~A/A.TP2~ #2 #5 // A: v1 // @wlb-inputs: t"],
    )];
    state.step2.bg2ee_mods = vec![mod_state(
        "B.TP2",
        &["~B/B.TP2~ #4 #3 // B: v1 // @wlb-inputs: n"],
    )];

    assert_eq!(rewrite_imported_logs(&state), Ok(2));
    assert_eq!(read(&first), "~A/A.TP2~ #2 #5 // A: v1 // @wlb-inputs: t\n");
    assert_eq!(
        read(&second),
        "~B/B.TP2~ #4 #3 // B: v1 // @wlb-inputs: n\n~A/A.TP2~ #0 #5 // A: v1 // @wlb-inputs: y\n"
    );
}

#[test]
fn rewrite_follows_the_eet_core_in_the_second_tree_for_the_bgee_log() {
    let root = TempRoot::new();
    let first = root.log(
        "bgee.log",
        "~EET/EET.TP2~ #0 #0 // EET core: v1 // @wlb-inputs: y\n",
    );
    let second = root.log("bg2ee.log", "~B/B.TP2~ #0 #3 // B: v1\n");
    let mut state = state_with_logs("EET", Some(&first), Some(&second));
    state.step2.bg2ee_mods = vec![mod_state(
        "EET.TP2",
        &["~EET/EET.TP2~ #3 #0 // EET core: v1 // @wlb-inputs: t"],
    )];

    assert_eq!(rewrite_imported_logs(&state), Ok(1));
    assert_eq!(
        read(&first),
        "~EET/EET.TP2~ #3 #0 // EET core: v1 // @wlb-inputs: t\n"
    );
}

#[test]
fn rewrite_does_not_touch_an_unchanged_file() {
    let root = TempRoot::new();
    let original = "~MOD/MOD.TP2~ #1 #2 // Comp: v1 //@wlb-inputs:t\n";
    let log = root.log("weidu.log", original);
    let modified = fs::metadata(&log)
        .and_then(|meta| meta.modified())
        .expect("mtime");
    let mut state = state_with_logs("BGEE", Some(&log), None);
    state.step2.bgee_mods = vec![mod_state(
        "MOD.TP2",
        &["~MOD/MOD.TP2~ #1 #2 // Comp: v1 // @wlb-inputs: t"],
    )];

    assert_eq!(rewrite_imported_logs(&state), Ok(0));
    assert_eq!(read(&log), original);
    assert_eq!(
        fs::metadata(&log)
            .and_then(|meta| meta.modified())
            .expect("mtime"),
        modified
    );
}

#[test]
fn rewrite_names_an_unreadable_log() {
    let root = TempRoot::new();
    let missing = root.path.join("missing.log");
    let mut state = state_with_logs("BGEE", Some(&missing), None);
    state.step2.bgee_mods = vec![mod_state("MOD.TP2", &["~MOD/MOD.TP2~ #1 #2 // Comp: v1"])];

    let err = rewrite_imported_logs(&state).expect_err("a missing log is an error");
    assert!(err.contains(&missing.display().to_string()), "{err}");
}
