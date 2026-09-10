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
    pub wlb_inputs: Option<&'static str>,
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

impl GalleryEntry {
    #[must_use]
    pub fn mod_count(&self) -> usize {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for gallery_mod in self.mods {
            let key = (gallery_mod.mod_name, gallery_mod.tp_file);
            if !seen.contains(&key) {
                seen.push(key);
            }
        }
        seen.len()
    }
}

const BIO_TEAM: &str = "BIO Team";
const COMMUNITY: &str = "Community Collection";

const REQUIREMENTS_EET: &str =
    "Baldur's Gate: Enhanced Edition and Baldur's Gate II: Enhanced Edition";
const REQUIREMENTS_BGEE: &str = "Baldur's Gate: Enhanced Edition";
const REQUIREMENTS_BG2EE: &str = "Baldur's Gate II: Enhanced Edition";
const REQUIREMENTS_IWDEE: &str = "Icewind Dale: Enhanced Edition";

#[must_use]
pub(crate) const fn requirements_for(game: Game) -> &'static str {
    match game {
        Game::BGEE => REQUIREMENTS_BGEE,
        Game::BG2EE => REQUIREMENTS_BG2EE,
        Game::IWDEE => REQUIREMENTS_IWDEE,
        Game::EET => REQUIREMENTS_EET,
    }
}

const EET_BG1_FOLDER_PROMPT: &str = r"y,C:\BIO\Baldur's Gate Enhanced Edition";

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
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "EET",
                tp_file: "EET.TP2",
                component_id: "0",
                component_label: "EET core (resource importation)",
                target: Game::BG2EE,
                wlb_inputs: Some(EET_BG1_FOLDER_PROMPT),
            },
            GalleryMod {
                mod_name: "EET_end",
                tp_file: "EET_END.TP2",
                component_id: "0",
                component_label: "EET end (last mod in install order)",
                target: Game::BG2EE,
                wlb_inputs: None,
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
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "EEFixPack",
                tp_file: "SETUP-EEFIXPACK.TP2",
                component_id: "0",
                component_label: "Core Fixes",
                target: Game::BGEE,
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "EEFixPack",
                tp_file: "SETUP-EEFIXPACK.TP2",
                component_id: "2",
                component_label: "Game Text Update",
                target: Game::BGEE,
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "EEFixPack",
                tp_file: "SETUP-EEFIXPACK.TP2",
                component_id: "0",
                component_label: "Core Fixes",
                target: Game::BG2EE,
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "EEFixPack",
                tp_file: "SETUP-EEFIXPACK.TP2",
                component_id: "2",
                component_label: "Game Text Update",
                target: Game::BG2EE,
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "EET",
                tp_file: "EET.TP2",
                component_id: "0",
                component_label: "EET core (resource importation)",
                target: Game::BG2EE,
                wlb_inputs: Some(EET_BG1_FOLDER_PROMPT),
            },
            GalleryMod {
                mod_name: "EET_end",
                tp_file: "EET_END.TP2",
                component_id: "0",
                component_label: "EET end (last mod in install order)",
                target: Game::BG2EE,
                wlb_inputs: None,
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
                mod_name: "DlcMerger",
                tp_file: "DLCMERGER.TP2",
                component_id: "1",
                component_label: "Merge DLC into game -> Siege of Dragonspear",
                target: Game::BGEE,
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "EEFixPack",
                tp_file: "SETUP-EEFIXPACK.TP2",
                component_id: "0",
                component_label: "Core Fixes",
                target: Game::BGEE,
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "EEFixPack",
                tp_file: "SETUP-EEFIXPACK.TP2",
                component_id: "2",
                component_label: "Game Text Update",
                target: Game::BGEE,
                wlb_inputs: None,
            },
            GalleryMod {
                mod_name: "CDTweaks",
                tp_file: "SETUP-CDTWEAKS.TP2",
                component_id: "2010",
                component_label: "Increase Ammo Stacking",
                target: Game::BGEE,
                wlb_inputs: None,
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
            tp_file: "SETUP-CDTWEAKS.TP2",
            component_id: "2010",
            component_label: "Increase Ammo Stacking",
            target: Game::IWDEE,
            wlb_inputs: None,
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
            raw_line: raw_line_for(gallery_mod),
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

fn raw_line_for(gallery_mod: &GalleryMod) -> String {
    gallery_mod.wlb_inputs.map_or_else(String::new, |inputs| {
        format!(
            "~{}\\{}~ #0 #{} // {} // @wlb-inputs: {}",
            gallery_mod.mod_name,
            gallery_mod.tp_file,
            gallery_mod.component_id,
            gallery_mod.component_label,
            inputs
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modlist_share::preview_modlist_share_code;

    #[test]
    fn mod_count_counts_distinct_mods_not_components() {
        let by_id = |id: &str| {
            entries()
                .iter()
                .find(|e| e.id == id)
                .expect("entry is in the catalog")
        };
        assert_eq!(by_id("eet-plus-fixes").mod_count(), 4);
        assert_eq!(by_id("bgee-vanilla-plus").mod_count(), 3);
        assert_eq!(by_id("eet-essentials").mod_count(), 3);
        assert_eq!(by_id("iwdee-essentials").mod_count(), 1);
    }

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

    fn log_lines(text: &str) -> Vec<&str> {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .collect()
    }

    #[test]
    fn fixpack_lists_install_core_fixes_and_game_text_update_on_every_tab() {
        let fixes_entry = entries()
            .iter()
            .find(|e| e.id == "eet-plus-fixes")
            .expect("EET + Fixes is in the catalog");
        let fixes_code = share_code(fixes_entry).expect("export");
        let fixes_preview = preview_modlist_share_code(&fixes_code).expect("parse");
        assert_eq!(fixes_preview.bgee_entries, 3);
        assert_eq!(fixes_preview.bg2ee_entries, 4);

        let first_game_lines = log_lines(&fixes_preview.bgee_log_text);
        assert!(first_game_lines[0].contains("DLCMERGER.TP2~ #0 #1"));
        assert!(first_game_lines[1].contains("SETUP-EEFIXPACK.TP2~ #0 #0"));
        assert!(first_game_lines[2].contains("SETUP-EEFIXPACK.TP2~ #0 #2"));

        let second_game_lines = log_lines(&fixes_preview.bg2ee_log_text);
        assert!(second_game_lines[0].contains("SETUP-EEFIXPACK.TP2~ #0 #0"));
        assert!(second_game_lines[1].contains("SETUP-EEFIXPACK.TP2~ #0 #2"));
        assert!(second_game_lines[2].contains("EET.TP2~ #0 #0"));
        assert!(second_game_lines[3].contains("EET_END.TP2~ #0 #0"));

        let vanilla_entry = entries()
            .iter()
            .find(|e| e.id == "bgee-vanilla-plus")
            .expect("BGEE Vanilla+ is in the catalog");
        let vanilla_code = share_code(vanilla_entry).expect("export");
        let vanilla_preview = preview_modlist_share_code(&vanilla_code).expect("parse");
        assert_eq!(vanilla_preview.bgee_entries, 4);

        let vanilla_lines = log_lines(&vanilla_preview.bgee_log_text);
        assert!(vanilla_lines[0].contains("DLCMERGER.TP2~ #0 #1"));
        assert!(vanilla_lines[1].contains("SETUP-EEFIXPACK.TP2~ #0 #0"));
        assert!(vanilla_lines[2].contains("SETUP-EEFIXPACK.TP2~ #0 #2"));
        assert!(vanilla_lines[3].contains("SETUP-CDTWEAKS.TP2~ #0 #2010"));
    }

    #[test]
    fn eet_lists_carry_the_bg1_folder_prompt_on_the_eet_core_line() {
        for id in ["eet-essentials", "eet-plus-fixes"] {
            let entry = entries()
                .iter()
                .find(|e| e.id == id)
                .expect("EET entry is in the catalog");
            let code = share_code(entry).expect("export");
            let preview = preview_modlist_share_code(&code).expect("parse");
            let bg2ee_lines = log_lines(&preview.bg2ee_log_text);
            let marker_lines: Vec<&&str> = bg2ee_lines
                .iter()
                .filter(|line| line.contains("@wlb-inputs:"))
                .collect();
            assert_eq!(
                marker_lines.len(),
                1,
                "{} must carry exactly one prompt marker line",
                entry.name
            );
            let marker_line = marker_lines[0];
            assert!(marker_line.contains("EET.TP2~ #0 #0"));
            assert!(
                marker_line.ends_with(r"// @wlb-inputs: y,C:\BIO\Baldur's Gate Enhanced Edition")
            );
        }
    }

    #[test]
    fn only_eet_core_lines_carry_a_prompt_marker() {
        let total: usize = entries()
            .iter()
            .map(|entry| {
                let code = share_code(entry).expect("export");
                let preview = preview_modlist_share_code(&code).expect("parse");
                let first_game_hits = log_lines(&preview.bgee_log_text)
                    .iter()
                    .filter(|line| line.contains("@wlb-inputs:"))
                    .count();
                let second_game_hits = log_lines(&preview.bg2ee_log_text)
                    .iter()
                    .filter(|line| line.contains("@wlb-inputs:"))
                    .count();
                first_game_hits + second_game_hits
            })
            .sum();
        assert_eq!(total, 2);
    }
}
