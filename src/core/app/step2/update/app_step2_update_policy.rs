// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::app_step2_update_source_refs::InstalledRefLookup;
use crate::app::game_authority::{self, GameSlot};
use crate::app::modlist_share::commit_sha_from_installed_ref;
use crate::app::state::WizardState;
use crate::parser::weidu_version::{normalize_version_text, parse_version};

pub(crate) fn mark_update_available(state: &mut WizardState, game_tab: &str, tp_file: &str) {
    let mods = if game_authority::slot_for_tab(game_tab) == GameSlot::First {
        &mut state.step2.bgee_mods
    } else {
        &mut state.step2.bg2ee_mods
    };
    if let Some(mod_state) = mods
        .iter_mut()
        .find(|mod_state| mod_state.tp_file == tp_file)
        && !mod_state.update_locked
    {
        mod_state.package_marker = Some('+');
    }
}

pub(crate) fn source_ref_is_update(
    lookup: &InstalledRefLookup,
    tp_file: &str,
    source_id: &str,
    latest_ref: &str,
) -> bool {
    let source_id = source_id.trim().to_ascii_lowercase();
    lookup
        .source_id_and_ref(tp_file)
        .is_some_and(|(installed_source_id, installed_ref)| {
            installed_source_id.trim().to_ascii_lowercase() == source_id
                && !refs_equal(&installed_ref, latest_ref)
        })
}

pub(crate) fn source_ref_matches(
    lookup: &InstalledRefLookup,
    tp_file: &str,
    source_id: &str,
    latest_ref: &str,
) -> bool {
    let source_id = source_id.trim().to_ascii_lowercase();
    lookup
        .source_id_and_ref(tp_file)
        .is_some_and(|(installed_source_id, installed_ref)| {
            installed_source_id.trim().to_ascii_lowercase() == source_id
                && refs_equal(&installed_ref, latest_ref)
        })
}

pub(crate) fn refs_equal(installed: &str, latest: &str) -> bool {
    let installed = installed.trim();
    let latest = latest.trim();
    match (
        commit_sha_from_installed_ref(installed),
        commit_sha_from_installed_ref(latest),
    ) {
        (Some(left), Some(right)) => shas_equal(&left, &right),
        _ => installed == latest,
    }
}

fn shas_equal(left: &str, right: &str) -> bool {
    let left = left.to_ascii_lowercase();
    let right = right.to_ascii_lowercase();
    let (shorter, longer) = if left.len() <= right.len() {
        (&left, &right)
    } else {
        (&right, &left)
    };
    shorter == longer || (shorter.len() >= 7 && longer.starts_with(shorter.as_str()))
}

pub(crate) fn mod_has_current_version(state: &WizardState, game_tab: &str, tp_file: &str) -> bool {
    let mods = if game_authority::slot_for_tab(game_tab) == GameSlot::First {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    mods.iter()
        .find(|mod_state| mod_state.tp_file == tp_file)
        .is_some_and(|mod_state| {
            mod_state
                .components
                .iter()
                .any(|component| parse_version(&component.raw_line).is_some())
        })
}

pub(crate) fn mod_is_scanned(state: &WizardState, game_tab: &str, tp_file: &str) -> bool {
    let mods = if game_authority::slot_for_tab(game_tab) == GameSlot::First {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    mods.iter().any(|mod_state| mod_state.tp_file == tp_file)
}

pub(crate) fn version_is_update(
    state: &WizardState,
    game_tab: &str,
    tp_file: &str,
    latest_tag: &str,
) -> bool {
    let mods = if game_authority::slot_for_tab(game_tab) == GameSlot::First {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let Some(mod_state) = mods.iter().find(|mod_state| mod_state.tp_file == tp_file) else {
        return false;
    };
    let Some(current) = mod_state
        .components
        .iter()
        .find_map(|component| parse_version(&component.raw_line))
    else {
        return false;
    };
    normalize_version_text(latest_tag) != normalize_version_text(&current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::app_step2_update_source_refs::ModSourceRefsFile;
    use crate::app::state::Step2ModState;

    fn mod_state(tp_file: &str) -> Step2ModState {
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
            checked: false,
            hidden_components: Vec::new(),
            components: Vec::new(),
        }
    }

    #[test]
    fn update_policy_routes_iwdee_to_the_first_container() {
        let mut state = WizardState::default();
        state.step2.bgee_mods.push(mod_state("mod.tp2"));
        mark_update_available(&mut state, "IWDEE", "mod.tp2");
        assert_eq!(state.step2.bgee_mods[0].package_marker, Some('+'));
        assert_eq!(state.step2.bg2ee_mods.len(), 0);
    }

    #[test]
    fn master_at_sha_equals_commit_at_same_sha() {
        assert!(refs_equal(
            "master@3da1d81f96ce96dd052bfeb241fbb477e686289e",
            "commit@3da1d81f96ce96dd052bfeb241fbb477e686289e",
        ));
        assert!(refs_equal(
            " master@3DA1D81F96CE96DD052BFEB241FBB477E686289E ",
            "commit@3da1d81f96ce96dd052bfeb241fbb477e686289e",
        ));
    }

    #[test]
    fn different_shas_are_an_update() {
        assert!(!refs_equal(
            "master@3da1d81f96ce96dd052bfeb241fbb477e686289e",
            "commit@0123456789abcdef0123456789abcdef01234567",
        ));
    }

    #[test]
    fn tag_refs_compare_whole() {
        assert!(!refs_equal("v35.17", "v35.18"));
        assert!(refs_equal("v35.17", "v35.17"));
        assert!(!refs_equal("abcdef12", "commit@abcdef12"));
    }

    fn refs_file(source_ref: &str) -> ModSourceRefsFile {
        let mut refs = ModSourceRefsFile::default();
        refs.refs
            .insert("cdtweaks".to_string(), source_ref.to_string());
        refs.sources
            .insert("cdtweaks".to_string(), "cdtweaks-master".to_string());
        refs
    }

    #[test]
    fn source_ref_matches_reads_the_folder_record_first() {
        let folder_ref = "master@aaaaaaa0123456789abcdef0123456789abcdef";
        let list_ref = "master@bbbbbbb0123456789abcdef0123456789abcdef";
        let both = InstalledRefLookup::from_files(Some(refs_file(folder_ref)), refs_file(list_ref));
        assert!(source_ref_matches(
            &both,
            "cdtweaks.tp2",
            "cdtweaks-master",
            folder_ref
        ));
        assert!(!source_ref_matches(
            &both,
            "cdtweaks.tp2",
            "cdtweaks-master",
            list_ref
        ));
        assert!(!source_ref_is_update(
            &both,
            "cdtweaks.tp2",
            "cdtweaks-master",
            folder_ref
        ));
        assert!(source_ref_is_update(
            &both,
            "cdtweaks.tp2",
            "cdtweaks-master",
            list_ref
        ));

        let list_only = InstalledRefLookup::from_files(None, refs_file(list_ref));
        assert!(source_ref_matches(
            &list_only,
            "cdtweaks.tp2",
            "cdtweaks-master",
            list_ref
        ));
        assert!(!source_ref_matches(
            &list_only,
            "cdtweaks.tp2",
            "cdtweaks-master",
            folder_ref
        ));
    }

    #[test]
    fn short_sha_prefix_matches_full() {
        assert!(refs_equal(
            "commit@3da1d81",
            "master@3da1d81f96ce96dd052bfeb241fbb477e686289e",
        ));
        assert!(refs_equal(
            "master@3da1d81f96ce96dd052bfeb241fbb477e686289e",
            "commit@3da1d81",
        ));
        assert!(!refs_equal(
            "commit@3da1d82",
            "master@3da1d81f96ce96dd052bfeb241fbb477e686289e",
        ));
    }
}
