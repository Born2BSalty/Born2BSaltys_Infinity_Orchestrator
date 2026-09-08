// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::modlist_share::{ShareExportSources, export_modlist_share_code_with};
use crate::app::state::{Step3ItemState, WizardState};
use crate::registry::model::Game;

pub struct GalleryMod {
    pub mod_name: &'static str,
    pub tp_file: &'static str,
    pub component_id: &'static str,
    pub component_label: &'static str,
    pub target: Game,
}

pub struct GalleryEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub author: &'static str,
    pub game: Game,
    pub tags: &'static [&'static str],
    pub starter: bool,
    pub sample: bool,
    pub description: &'static str,
    pub requirements: &'static str,
    pub version: &'static str,
    pub mods: &'static [GalleryMod],
}

const BIO_TEAM: &str = "BIO Team";
const COMMUNITY: &str = "Community Collection";

const REQUIREMENTS_EET: &str =
    "Baldur's Gate: Enhanced Edition and Baldur's Gate II: Enhanced Edition";
const REQUIREMENTS_BGEE: &str = "Baldur's Gate: Enhanced Edition";
const REQUIREMENTS_IWDEE: &str = "Icewind Dale: Enhanced Edition";

const ENTRIES: &[GalleryEntry] = &[
    GalleryEntry {
        id: "eet-essentials",
        name: "EET Essentials",
        author: BIO_TEAM,
        game: Game::EET,
        tags: &["Starter", "Core setup"],
        starter: true,
        sample: true,
        description: "The minimum EET spine: merge the campaigns, then bridge Baldur's Gate into Shadows of Amn so one save carries the whole saga.",
        requirements: REQUIREMENTS_EET,
        version: "1.0.0",
        mods: &[
            GalleryMod {
                mod_name: "DlcMerger",
                tp_file: "DLCMERGER.TP2",
                component_id: "1",
                component_label: "Merge DLC into game -> Siege of Dragonspear",
                target: Game::BGEE,
            },
            GalleryMod {
                mod_name: "EET",
                tp_file: "EET.TP2",
                component_id: "0",
                component_label: "EET core (resource importation)",
                target: Game::BG2EE,
            },
            GalleryMod {
                mod_name: "EET_end",
                tp_file: "EET_END.TP2",
                component_id: "0",
                component_label: "EET end (last mod in install order)",
                target: Game::BG2EE,
            },
        ],
    },
    GalleryEntry {
        id: "eet-plus-fixes",
        name: "EET + Fixes",
        author: BIO_TEAM,
        game: Game::EET,
        tags: &["Starter", "Fixes"],
        starter: true,
        sample: true,
        description: "EET Essentials with the community fixpack layered on top, so the merged saga starts from a patched engine instead of a vanilla one.",
        requirements: REQUIREMENTS_EET,
        version: "1.0.0",
        mods: &[
            GalleryMod {
                mod_name: "DlcMerger",
                tp_file: "DLCMERGER.TP2",
                component_id: "1",
                component_label: "Merge DLC into game -> Siege of Dragonspear",
                target: Game::BGEE,
            },
            GalleryMod {
                mod_name: "EET",
                tp_file: "EET.TP2",
                component_id: "0",
                component_label: "EET core (resource importation)",
                target: Game::BG2EE,
            },
            GalleryMod {
                mod_name: "EEFixPack",
                tp_file: "EEFIXPACK.TP2",
                component_id: "0",
                component_label: "Core Fixes",
                target: Game::BG2EE,
            },
            GalleryMod {
                mod_name: "EET_end",
                tp_file: "EET_END.TP2",
                component_id: "0",
                component_label: "EET end (last mod in install order)",
                target: Game::BG2EE,
            },
        ],
    },
    GalleryEntry {
        id: "bgee-vanilla-plus",
        name: "BGEE Vanilla+",
        author: COMMUNITY,
        game: Game::BGEE,
        tags: &["Vanilla+", "Quality of life"],
        starter: false,
        sample: true,
        description: "Baldur's Gate as it shipped, minus the rough edges: the community fixpack plus a light pass of quality-of-life tweaks.",
        requirements: REQUIREMENTS_BGEE,
        version: "1.0.0",
        mods: &[
            GalleryMod {
                mod_name: "EEFixPack",
                tp_file: "EEFIXPACK.TP2",
                component_id: "0",
                component_label: "Core Fixes",
                target: Game::BGEE,
            },
            GalleryMod {
                mod_name: "CDTweaks",
                tp_file: "CDTWEAKS.TP2",
                component_id: "2010",
                component_label: "Increase Ammo Stacking",
                target: Game::BGEE,
            },
        ],
    },
    GalleryEntry {
        id: "iwdee-essentials",
        name: "Icewind Dale Essentials",
        author: COMMUNITY,
        game: Game::IWDEE,
        tags: &["Starter", "Quality of life"],
        starter: true,
        sample: true,
        description: "A short, safe starting point for Icewind Dale: the tweaks most players turn on first, and nothing that reshapes the campaign.",
        requirements: REQUIREMENTS_IWDEE,
        version: "1.0.0",
        mods: &[GalleryMod {
            mod_name: "CDTweaks",
            tp_file: "CDTWEAKS.TP2",
            component_id: "2010",
            component_label: "Increase Ammo Stacking",
            target: Game::IWDEE,
        }],
    },
];

#[must_use]
pub const fn entries() -> &'static [GalleryEntry] {
    ENTRIES
}

pub fn share_code(entry: &GalleryEntry) -> Result<String, String> {
    let mut state = export_state_for(entry);
    state.modlist_share_name = Some(entry.name.to_string());
    state.modlist_share_author = Some(entry.author.to_string());
    export_modlist_share_code_with(&state, &ShareExportSources::default())
}

fn export_state_for(entry: &GalleryEntry) -> WizardState {
    let mut state = WizardState::default();
    state.step1.game_install = entry.game.to_legacy_string().to_string();
    state.step1.sync_install_mode_flags();

    for (index, gallery_mod) in entry.mods.iter().enumerate() {
        let item = Step3ItemState {
            tp_file: gallery_mod.tp_file.to_string(),
            component_id: gallery_mod.component_id.to_string(),
            mod_name: gallery_mod.mod_name.to_string(),
            component_label: gallery_mod.component_label.to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: index + 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        };
        if gallery_mod.target == Game::BG2EE {
            state.step3.bg2ee_items.push(item);
        } else {
            state.step3.bgee_items.push(item);
        }
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modlist_share::preview_modlist_share_code;

    #[test]
    fn catalog_holds_the_four_stub_entries() {
        let names: Vec<&str> = entries().iter().map(|e| e.name).collect();
        assert_eq!(
            names,
            vec![
                "EET Essentials",
                "EET + Fixes",
                "BGEE Vanilla+",
                "Icewind Dale Essentials",
            ]
        );
    }

    #[test]
    fn entry_ids_are_unique() {
        let mut ids: Vec<&str> = entries().iter().map(|e| e.id).collect();
        ids.sort_unstable();
        let total = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), total, "gallery ids must be unique");
    }

    #[test]
    fn every_entry_is_flagged_sample_with_a_version() {
        for entry in entries() {
            assert!(entry.sample, "{} must carry the sample flag", entry.name);
            assert_eq!(entry.version, "1.0.0");
            assert!(!entry.mods.is_empty());
            assert!(!entry.description.trim().is_empty());
            assert!(!entry.requirements.trim().is_empty());
        }
    }

    #[test]
    fn three_entries_are_starter_lists() {
        assert_eq!(entries().iter().filter(|e| e.starter).count(), 3);
    }

    #[test]
    fn every_entry_generates_a_code_that_parses_back_with_its_provenance() {
        for entry in entries() {
            let code = share_code(entry).expect("catalog entry must export a share code");
            let preview =
                preview_modlist_share_code(&code).expect("generated code must parse back");
            assert_eq!(preview.game_install, entry.game.to_legacy_string());
            assert_eq!(preview.name.as_deref(), Some(entry.name));
            assert_eq!(preview.author.as_deref(), Some(entry.author));
            assert!(
                preview.allow_auto_install,
                "{} must permit install-as-provided",
                entry.name
            );
        }
    }

    #[test]
    fn eet_essentials_splits_one_bgee_and_two_bg2ee_entries() {
        let entry = entries()
            .iter()
            .find(|e| e.id == "eet-essentials")
            .expect("EET Essentials is in the catalog");
        let code = share_code(entry).expect("export");
        let preview = preview_modlist_share_code(&code).expect("parse");
        assert_eq!(preview.bgee_entries, 1);
        assert_eq!(preview.bg2ee_entries, 2);
    }

    #[test]
    fn stub_codes_carry_no_source_overrides_or_installed_refs() {
        for entry in entries() {
            let code = share_code(entry).expect("catalog entry must export a share code");
            let preview =
                preview_modlist_share_code(&code).expect("generated code must parse back");
            assert!(
                !preview.has_source_overrides,
                "{} must carry no source overrides",
                entry.name
            );
            assert!(
                !preview.has_installed_refs,
                "{} must carry no installed refs",
                entry.name
            );
            assert_eq!(
                preview.mod_config_count, 0,
                "{} must carry no mod configs",
                entry.name
            );
        }
    }

    #[test]
    fn stub_codes_are_byte_identical_across_calls() {
        for entry in entries() {
            let first = share_code(entry).expect("first export");
            let second = share_code(entry).expect("second export");
            assert_eq!(
                first, second,
                "{} stub code must be deterministic",
                entry.name
            );
        }
    }

    #[test]
    fn non_bg2ee_targets_land_in_the_first_game_log() {
        let entry = entries()
            .iter()
            .find(|e| e.id == "iwdee-essentials")
            .expect("Icewind Dale Essentials is in the catalog");
        let state = export_state_for(entry);
        assert_eq!(state.step3.bgee_items.len(), 1);
        assert!(state.step3.bg2ee_items.is_empty());
        assert_eq!(state.step3.bgee_items[0].selected_order, 1);
    }
}
