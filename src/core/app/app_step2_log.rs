// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use crate::app::added_mods;
use crate::app::controller::log_apply::{apply_log_to_mods, normalize_path_key};
use crate::app::game_authority::{self, GameSlot};
use crate::app::mod_downloads::{self, ModDownloadsLoad, SourceTier, SourceTiers};
use crate::app::state::{Step1State, Step2LogPendingDownload, WizardState};
use crate::mods::component::Component;
use crate::mods::log_file::LogFile;

pub(crate) fn apply_saved_weidu_log_selection(state: &mut WizardState) {
    match state.step1.game_install.as_str() {
        "BG2EE" => {
            let log_path = resolve_bg2_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, false, log_path);
        }
        "EET" => {
            let bgee_log_path = resolve_bgee_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, true, bgee_log_path);
            let bg2ee_log_path = resolve_bg2_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, false, bg2ee_log_path);
        }
        _ => {
            let log_path = resolve_bgee_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, true, log_path);
        }
    }
}

pub(crate) fn apply_weidu_log_selection_from_path(
    state: &mut WizardState,
    bgee: bool,
    log_path: Option<PathBuf>,
) {
    let Some(path) = log_path else {
        state.step2.scan_status = "No WeiDU log selected".to_string();
        return;
    };

    let log = match LogFile::from_path(&path) {
        Ok(v) => v,
        Err(err) => {
            state.step2.scan_status = format!("Failed to parse log: {err}");
            return;
        }
    };

    let mut next_order = state.step2.next_selection_order;
    match (state.step1.game_install.as_str(), bgee) {
        ("EET", true) => {
            crate::app::compat_logic::clear_step2_compat_state(&mut state.step2.bgee_mods);
            crate::app::compat_logic::clear_step2_compat_state(&mut state.step2.bg2ee_mods);
        }
        (_, true) => {
            crate::app::compat_logic::clear_step2_compat_state(&mut state.step2.bgee_mods);
        }
        _ => {
            crate::app::compat_logic::clear_step2_compat_state(&mut state.step2.bg2ee_mods);
        }
    }
    let matched = match (state.step1.game_install.as_str(), bgee) {
        ("EET", true) => {
            let picked_bgee = apply_log_to_mods(
                &mut state.step2.bgee_mods,
                &log,
                None,
                true,
                &mut next_order,
            );
            let allow = HashSet::from([normalize_path_key(r"EET\EET.TP2")]);
            let picked_eet_core = apply_log_to_mods(
                &mut state.step2.bg2ee_mods,
                &log,
                Some(&allow),
                false,
                &mut next_order,
            );
            picked_bgee + picked_eet_core
        }
        (_, true) => apply_log_to_mods(
            &mut state.step2.bgee_mods,
            &log,
            None,
            true,
            &mut next_order,
        ),
        _ => apply_log_to_mods(
            &mut state.step2.bg2ee_mods,
            &log,
            None,
            true,
            &mut next_order,
        ),
    };
    state.step2.next_selection_order = next_order;
    let compat_error = crate::app::compat_logic::apply_step2_compat_rules(
        &state.step1,
        &mut state.step2.bgee_mods,
        &mut state.step2.bg2ee_mods,
    );
    let label = if bgee {
        game_authority::first_slot_tab(&state.step1.game_install)
    } else {
        "BG2EE"
    };
    state
        .step2
        .log_pending_downloads
        .retain(|pending| pending.game_tab != label);
    state
        .step2
        .log_pending_downloads
        .extend(build_log_pending_downloads(state, &log, label));
    if bgee {
        state.step2.review_edit_bgee_log_applied = true;
    } else {
        state.step2.review_edit_bg2ee_log_applied = true;
    }
    let pending = state.step2.log_pending_downloads.len();
    state.step2.scan_status = compat_error.map_or_else(
        || format!("{label} selected from log: {matched}, pending downloads: {pending}"),
        |err| format!(
            "{label} selected from log: {matched}, pending downloads: {pending} (compat rules load failed: {err})"
        ),
    );
    state.clear_last_step2_sync_signature();
}

pub(crate) fn reseed_added_mod_pending_downloads(state: &mut WizardState) {
    let Some(path) = mod_downloads::active_modlist_downloads_path() else {
        return;
    };
    let modlist_text = std::fs::read_to_string(path).unwrap_or_default();
    let added = added_mods::load_added_mods();
    if added.is_empty() {
        return;
    }
    let tiers = mod_downloads::load_source_tiers(&modlist_text);
    let sources = mod_downloads::load_mod_download_sources();
    seed_added_mod_pending_downloads(state, &sources, &tiers, &added);
}

pub(crate) fn seed_added_mod_pending_downloads(
    state: &mut WizardState,
    sources: &ModDownloadsLoad,
    tiers: &SourceTiers,
    added: &BTreeSet<String>,
) {
    let scanned: HashSet<String> = state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
        .map(|mod_state| mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file))
        .filter(|key| !key.is_empty())
        .collect();
    let mut listed: HashSet<String> = state
        .step2
        .log_pending_downloads
        .iter()
        .map(|pending| mod_downloads::normalize_mod_download_tp2(&pending.tp_file))
        .collect();
    let game_tab = state.step2.active_game_tab.clone();
    for source in &sources.sources {
        if !matches!(
            tiers.tier_of(&source.tp2, &source.source_id),
            SourceTier::Modlist | SourceTier::User
        ) {
            continue;
        }
        let tp2 = mod_downloads::normalize_mod_download_tp2(&source.tp2);
        if !added.contains(&tp2) {
            continue;
        }
        let scanned_under_alias = source
            .aliases
            .iter()
            .any(|alias| scanned.contains(&mod_downloads::normalize_mod_download_tp2(alias)));
        if tp2.is_empty() || scanned.contains(&tp2) || scanned_under_alias {
            continue;
        }
        if !listed.insert(tp2.clone()) {
            continue;
        }
        let label = if source.name.trim().is_empty() {
            tp2.clone()
        } else {
            source.name.trim().to_string()
        };
        state
            .step2
            .log_pending_downloads
            .push(Step2LogPendingDownload {
                game_tab: game_tab.clone(),
                tp_file: format!("{tp2}.tp2"),
                label,
                requested_version: None,
            });
    }
}

fn build_log_pending_downloads(
    state: &WizardState,
    log: &LogFile,
    game_tab: &str,
) -> Vec<Step2LogPendingDownload> {
    let mods = if game_authority::slot_for_tab(game_tab) == GameSlot::First {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mod_download_sources = crate::app::mod_downloads::load_mod_download_sources();
    let mut installed_tp2 = HashSet::new();
    for mod_state in mods {
        let tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
        if tp2.is_empty() {
            continue;
        }
        installed_tp2.insert(tp2.clone());
        for source in mod_download_sources.find_sources(&tp2) {
            installed_tp2.insert(crate::app::mod_downloads::normalize_mod_download_tp2(
                &source.tp2,
            ));
            for alias in source.aliases {
                installed_tp2.insert(crate::app::mod_downloads::normalize_mod_download_tp2(
                    &alias,
                ));
            }
        }
    }
    let mut pending = Vec::new();
    let mut seen = HashSet::new();
    for component in log.components() {
        push_log_pending_download(&mut pending, &mut seen, &installed_tp2, component, game_tab);
    }
    pending
}

fn push_log_pending_download(
    pending: &mut Vec<Step2LogPendingDownload>,
    seen: &mut HashSet<String>,
    installed_tp2: &HashSet<String>,
    component: &Component,
    game_tab: &str,
) {
    let tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&component.tp_file);
    if tp2.is_empty() || installed_tp2.contains(&tp2) || !seen.insert(tp2) {
        return;
    }
    let label = if component.name.trim().is_empty() {
        component.tp_file.clone()
    } else {
        component.name.clone()
    };
    let requested_version = requested_version_text(&component.version);
    pending.push(Step2LogPendingDownload {
        game_tab: game_tab.to_string(),
        tp_file: component.tp_file.clone(),
        label,
        requested_version,
    });
}

fn requested_version_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || !looks_like_requested_version(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

fn looks_like_requested_version(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if lower.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return true;
    }
    if lower.starts_with('v') && lower.chars().nth(1).is_some_and(|ch| ch.is_ascii_digit()) {
        return true;
    }
    if lower.starts_with("version-")
        && lower
            .chars()
            .nth("version-".len())
            .is_some_and(|ch| ch.is_ascii_digit())
    {
        return true;
    }
    for prefix in ["alpha", "beta", "rc", "pre"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let rest = rest.trim_start_matches([' ', '-', '_']);
            if rest.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
    }
    if let Some((branch, commit)) = lower.split_once('@') {
        return !branch.is_empty()
            && commit.len() >= 7
            && commit.chars().all(|ch| ch.is_ascii_hexdigit());
    }
    false
}

pub(crate) fn resolve_bgee_weidu_log_path(s: &Step1State) -> Option<PathBuf> {
    if s.have_weidu_logs && !s.bgee_log_file.trim().is_empty() {
        return Some(PathBuf::from(s.bgee_log_file.trim()));
    }
    let folder = if s.game_install == "EET" {
        s.eet_bgee_log_folder.trim()
    } else {
        s.bgee_log_folder.trim()
    };
    if folder.is_empty() {
        None
    } else {
        Some(PathBuf::from(folder).join("weidu.log"))
    }
}

pub(crate) fn resolve_bg2_weidu_log_path(s: &Step1State) -> Option<PathBuf> {
    if s.have_weidu_logs && !s.bg2ee_log_file.trim().is_empty() {
        return Some(PathBuf::from(s.bg2ee_log_file.trim()));
    }
    let folder = if s.game_install == "EET" {
        s.eet_bg2ee_log_folder.trim()
    } else {
        s.bg2ee_log_folder.trim()
    };
    if folder.is_empty() {
        None
    } else {
        Some(PathBuf::from(folder).join("weidu.log"))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("bio_step2_log_test_{}_{id}", std::process::id()));
            std::fs::create_dir_all(&path).expect("create temp root");
            Self { path }
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn step2_log_label_names_the_lists_own_tab() {
        let root = TempRoot::new();
        let log_path = root.path.join("weidu.log");
        std::fs::write(&log_path, "").expect("write empty log");

        let mut state = WizardState {
            step1: Step1State {
                game_install: "IWDEE".to_string(),
                ..Step1State::default()
            },
            ..WizardState::default()
        };
        apply_weidu_log_selection_from_path(&mut state, true, Some(log_path));

        assert!(
            state
                .step2
                .scan_status
                .starts_with("IWDEE selected from log"),
            "unexpected status: {}",
            state.step2.scan_status
        );
    }

    fn github_block(tp2: &str, name: &str) -> String {
        format!(
            "[[mods]]\nname = \"{name}\"\ntp2 = \"{tp2}\"\n\n  [[mods.sources]]\n  id = \"primary\"\n  label = \"Primary\"\n  type = \"github\"\n  url = \"https://github.com/owner/{tp2}\"\n  repo = \"owner/{tp2}\"\n  default = true\n"
        )
    }

    fn scanned_mod(tp_file: &str) -> crate::app::state::Step2ModState {
        crate::app::state::Step2ModState {
            name: tp_file.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: tp_file.to_string(),
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
            components: Vec::new(),
        }
    }

    fn seed_from_texts(
        state: &mut WizardState,
        default_text: &str,
        user_text: &str,
        modlist_text: &str,
        added: &[&str],
    ) {
        let sources = mod_downloads::load_mod_download_sources_from_texts(
            default_text,
            user_text,
            modlist_text,
        );
        let tiers = mod_downloads::source_tiers_from_texts(default_text, user_text, modlist_text);
        let added: BTreeSet<String> = added.iter().map(ToString::to_string).collect();
        seed_added_mod_pending_downloads(state, &sources, &tiers, &added);
    }

    #[test]
    fn reseed_ignores_modlist_blocks_not_in_the_added_list() {
        let modlist_text = github_block("widget", "Widget");
        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();

        seed_from_texts(&mut state, "", "", &modlist_text, &[]);
        assert!(
            state.step2.log_pending_downloads.is_empty(),
            "{:?}",
            state.step2.log_pending_downloads
        );

        seed_from_texts(&mut state, "", "", &modlist_text, &["widget"]);
        assert_eq!(state.step2.log_pending_downloads.len(), 1);
    }

    #[test]
    fn reseed_adds_a_card_for_a_saved_unscanned_mod() {
        let modlist_text = format!(
            "{}\n{}",
            github_block("widget", "Widget"),
            github_block("widget", "Widget").replace("id = \"primary\"", "id = \"fork\"")
        );
        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();

        seed_from_texts(&mut state, "", "", &modlist_text, &["widget"]);

        assert_eq!(
            state.step2.log_pending_downloads,
            vec![Step2LogPendingDownload {
                game_tab: "BG2EE".to_string(),
                tp_file: "widget.tp2".to_string(),
                label: "Widget".to_string(),
                requested_version: None,
            }]
        );

        seed_from_texts(&mut state, "", "", &modlist_text, &["widget"]);
        assert_eq!(state.step2.log_pending_downloads.len(), 1);
    }

    #[test]
    fn reseed_skips_scanned_mods() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(scanned_mod("widget/setup-widget.tp2"));

        seed_from_texts(
            &mut state,
            "",
            "",
            &github_block("widget", "Widget"),
            &["widget"],
        );

        assert!(
            state.step2.log_pending_downloads.is_empty(),
            "{:?}",
            state.step2.log_pending_downloads
        );
    }

    #[test]
    fn reseed_adds_a_card_for_an_added_mod_saved_to_my_default() {
        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();

        seed_from_texts(
            &mut state,
            "",
            &github_block("gadget", "Gadget"),
            "",
            &["gadget"],
        );

        assert_eq!(
            state.step2.log_pending_downloads,
            vec![Step2LogPendingDownload {
                game_tab: "BG2EE".to_string(),
                tp_file: "gadget.tp2".to_string(),
                label: "Gadget".to_string(),
                requested_version: None,
            }]
        );
    }

    struct IsolatedAmbient {
        previous_modlist_dir: Option<PathBuf>,
        root: TempRoot,
    }

    impl IsolatedAmbient {
        fn create() -> Self {
            let root = TempRoot::new();
            let config_dir = root.path.join("config");
            let modlist_dir = root.path.join("modlist");
            std::fs::create_dir_all(&config_dir).expect("create temp config dir");
            std::fs::create_dir_all(&modlist_dir).expect("create temp modlist dir");
            let previous_modlist_dir = mod_downloads::active_modlist_dir();
            crate::platform_defaults::set_config_dir_override(Some(config_dir));
            mod_downloads::set_active_modlist_dir(Some(modlist_dir));
            Self {
                previous_modlist_dir,
                root,
            }
        }
    }

    impl Drop for IsolatedAmbient {
        fn drop(&mut self) {
            mod_downloads::set_active_modlist_dir(self.previous_modlist_dir.take());
            crate::platform_defaults::clear_config_dir_override_if(&self.root.path.join("config"));
        }
    }

    #[test]
    fn reseed_runs_for_a_list_whose_own_sources_file_is_absent() {
        let _lock = mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let ambient = IsolatedAmbient::create();
        let user_path = mod_downloads::mod_downloads_user_path();
        assert!(user_path.starts_with(&ambient.root.path), "{user_path:?}");
        std::fs::create_dir_all(user_path.parent().expect("user path parent"))
            .expect("create user sources dir");
        std::fs::write(&user_path, github_block("gadget", "Gadget")).expect("write user sources");
        added_mods::record_added_mod("gadget").expect("record gadget");
        let modlist_sources = mod_downloads::active_modlist_downloads_path().expect("modlist path");
        assert!(!modlist_sources.exists());

        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();
        reseed_added_mod_pending_downloads(&mut state);

        assert_eq!(
            state.step2.log_pending_downloads,
            vec![Step2LogPendingDownload {
                game_tab: "BG2EE".to_string(),
                tp_file: "gadget.tp2".to_string(),
                label: "Gadget".to_string(),
                requested_version: None,
            }]
        );
    }

    #[test]
    fn reseed_skips_a_bio_default_block() {
        let default_text = github_block("gizmo", "Gizmo");
        assert!(
            !mod_downloads::load_mod_download_sources_from_texts(&default_text, "", "")
                .find_sources("gizmo")
                .is_empty()
        );
        let mut state = WizardState::default();

        seed_from_texts(&mut state, &default_text, "", "", &["gizmo"]);

        assert!(
            state.step2.log_pending_downloads.is_empty(),
            "{:?}",
            state.step2.log_pending_downloads
        );
    }

    #[test]
    fn keeps_version_like_labels() {
        assert_eq!(requested_version_text("v29").as_deref(), Some("v29"));
        assert_eq!(requested_version_text("1.57").as_deref(), Some("1.57"));
        assert_eq!(
            requested_version_text("Alpha 3").as_deref(),
            Some("Alpha 3")
        );
        assert_eq!(
            requested_version_text("master@149ac53b1470").as_deref(),
            Some("master@149ac53b1470")
        );
    }

    #[test]
    fn drops_component_text_labels() {
        assert_eq!(requested_version_text("BG1 Prologue Expansion"), None);
        assert_eq!(requested_version_text("Main Component"), None);
        assert_eq!(
            requested_version_text("Book 1 - Quiet Before the Storm"),
            None
        );
        assert_eq!(requested_version_text("EE and SoD"), None);
    }
}
