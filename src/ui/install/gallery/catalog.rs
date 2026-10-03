// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::gallery_feed::index::FeedEntry;
use crate::registry::model::Game;

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

#[must_use]
pub fn entries() -> &'static [FeedEntry] {
    crate::gallery_feed::snapshot::snapshot_entries()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modlist_share::preview_modlist_share_code;
    use crate::mods::component::Component;

    fn by_id<'a>(id: &str) -> &'a FeedEntry {
        entries()
            .iter()
            .find(|entry| entry.id == id)
            .expect("entry is in the catalog")
    }

    #[test]
    fn entry_ids_are_unique() {
        let mut ids: Vec<&str> = entries().iter().map(|e| e.id.as_str()).collect();
        ids.sort_unstable();
        let total = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), total, "gallery ids must be unique");
    }

    #[test]
    fn every_entry_generates_a_code_that_parses_back_with_its_provenance() {
        for entry in entries() {
            let preview = preview_modlist_share_code(&entry.code)
                .expect("catalog entry code must parse back");
            assert_eq!(preview.game_install, entry.game.to_legacy_string());
            assert_eq!(preview.name.as_deref(), Some(entry.name.as_str()));
            assert_eq!(preview.author.as_deref(), Some(entry.author.as_str()));
            assert!(
                preview.allow_auto_install,
                "{} must permit install-as-provided",
                entry.name
            );
        }
    }

    #[test]
    fn eet_essentials_splits_three_bgee_and_eighty_bg2ee_entries() {
        let entry = by_id("eet-essentials");
        let preview = preview_modlist_share_code(&entry.code).expect("parse");
        assert_eq!(preview.bgee_entries, 3);
        assert_eq!(preview.bg2ee_entries, 80);
    }

    #[test]
    fn every_entry_carries_source_overrides() {
        for entry in entries() {
            let preview =
                preview_modlist_share_code(&entry.code).expect("catalog entry code must parse");
            assert!(
                preview.has_source_overrides,
                "{} must carry source overrides",
                entry.name
            );
        }
    }

    const EET_ESSENTIALS_PINS: [(&str, &str, &str, &str, &str); 12] = [
        (
            "DlcMerger",
            "commit",
            "bfd167f7a52dfa6c9e694955a074a85991b0c358",
            "argent77",
            "Argent77",
        ),
        (
            "eefixpack",
            "commit",
            "9db254fe0c046d789381de0022ff37eb2c3a3979",
            "gibberlings3",
            "Gibberlings3",
        ),
        (
            "eet",
            "commit",
            "b164f5997de7a00ef69177d8221c19bd1e34cf10",
            "gibberlings3",
            "Gibberlings3",
        ),
        (
            "EET_END",
            "commit",
            "b164f5997de7a00ef69177d8221c19bd1e34cf10",
            "gibberlings3",
            "Gibberlings3",
        ),
        (
            "cdtweaks",
            "commit",
            "7649ced6cd25865874d787ec1a9abbc67b068729",
            "gibberlings3",
            "Gibberlings3",
        ),
        (
            "HiddenGameplayOptions",
            "commit",
            "ae8571b19d8a157b13367bfe7d379d2dfb456d75",
            "argent77",
            "Argent77",
        ),
        (
            "HQ_SoundClips_BG2EE",
            "commit",
            "17b4444d8fb8fd3324648027a8119045bd6c239a",
            "argent77",
            "Argent77",
        ),
        (
            "LeUI",
            "commit",
            "0e3c400ca4a85cd84933b77da0b22c6c5c322ef2",
            "r-e-d",
            "r-e-d",
        ),
        (
            "remastered_spell_icons",
            "commit",
            "14d4568b326ce04e767815a61489a89c51033b76",
            "renegade0",
            "Renegade0",
        ),
        ("eeex", "tag", "v1.2.0", "bubb13", "Bubb13"),
        (
            "bubb_spell_menu_extended",
            "tag",
            "v5.2",
            "bubb13",
            "Bubb13",
        ),
        ("EET_Tweaks", "tag", "v1.12", "k4thos", "K4thos"),
    ];

    fn overrides_text_for(id: &str) -> String {
        let entry = by_id(id);
        preview_modlist_share_code(&entry.code)
            .expect("parse")
            .source_overrides_text
    }

    fn overrides_mod_block_count(overrides: &str) -> usize {
        toml::from_str::<toml::Value>(overrides)
            .ok()
            .and_then(|value| {
                value
                    .get("mods")
                    .and_then(toml::Value::as_array)
                    .map(Vec::len)
            })
            .unwrap_or(0)
    }

    fn assert_pins(overrides: &str, expected: &[(&str, &str, &str, &str, &str)], label: &str) {
        use crate::app::mod_downloads::{SourceTier, source_tiers_from_texts};

        assert_eq!(
            overrides_mod_block_count(overrides),
            expected.len(),
            "{label}: unexpected number of [[mods]] blocks in overrides"
        );

        let tiers = source_tiers_from_texts(
            include_str!("../../../core/config/default_mod_downloads.toml"),
            "",
            overrides,
        );
        for (tp2, kind, value, source_id, source_label) in expected {
            let (source, tier) = tiers
                .resolve(tp2)
                .unwrap_or_else(|| panic!("{label}: {tp2} must resolve"));
            assert_eq!(
                tier,
                SourceTier::Modlist,
                "{label}: {tp2} must resolve from the modlist overlay, not the stock tier"
            );
            assert_eq!(
                &source.source_id, source_id,
                "{label}: {tp2} source id mismatch"
            );
            assert_eq!(
                &source.source_label, source_label,
                "{label}: {tp2} source label mismatch"
            );
            assert!(
                source.channel.is_none(),
                "{label}: {tp2} must carry no channel"
            );
            match *kind {
                "commit" => {
                    let commit = source.commit.as_deref().unwrap_or_default();
                    assert_eq!(
                        commit.len(),
                        40,
                        "{label}: {tp2} commit must be 40 hex characters"
                    );
                    assert!(
                        commit.chars().all(|c| c.is_ascii_hexdigit()),
                        "{label}: {tp2} commit must be hex"
                    );
                    assert_eq!(commit, *value, "{label}: {tp2} commit mismatch");
                    assert!(
                        source.tag.is_none(),
                        "{label}: {tp2} must carry only a commit"
                    );
                    assert!(
                        source.branch.is_none(),
                        "{label}: {tp2} must carry only a commit"
                    );
                }
                "tag" => {
                    assert_eq!(
                        source.tag.as_deref(),
                        Some(*value),
                        "{label}: {tp2} tag mismatch"
                    );
                    assert!(
                        source.commit.is_none(),
                        "{label}: {tp2} must carry only a tag"
                    );
                    assert!(
                        source.branch.is_none(),
                        "{label}: {tp2} must carry only a tag"
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn every_source_snapshot_pin_is_a_full_commit_sha() {
        assert_pins(
            &overrides_text_for("eet-essentials"),
            &EET_ESSENTIALS_PINS,
            "eet-essentials",
        );

        let subset = |tp2s: &[&str]| -> Vec<(&str, &str, &str, &str, &str)> {
            EET_ESSENTIALS_PINS
                .iter()
                .filter(|(tp2, ..)| tp2s.contains(tp2))
                .copied()
                .collect()
        };

        assert_pins(
            &overrides_text_for("eet-plus-fixes"),
            &subset(&["DlcMerger", "eefixpack", "eet", "EET_END"]),
            "eet-plus-fixes",
        );
        assert_pins(
            &overrides_text_for("bgee-vanilla-plus"),
            &subset(&[
                "DlcMerger",
                "eefixpack",
                "cdtweaks",
                "HiddenGameplayOptions",
                "LeUI",
                "remastered_spell_icons",
                "eeex",
                "bubb_spell_menu_extended",
            ]),
            "bgee-vanilla-plus",
        );
        assert_pins(
            &overrides_text_for("bgee-vanilla-plus-no-dlc"),
            &subset(&[
                "eefixpack",
                "HiddenGameplayOptions",
                "LeUI",
                "remastered_spell_icons",
                "eeex",
                "bubb_spell_menu_extended",
            ]),
            "bgee-vanilla-plus-no-dlc",
        );
        assert_pins(
            &overrides_text_for("iwdee-essentials"),
            &subset(&["cdtweaks"]),
            "iwdee-essentials",
        );
    }

    #[test]
    fn no_dlc_list_carries_no_cdtweaks_source_block() {
        let no_dlc_overrides = overrides_text_for("bgee-vanilla-plus-no-dlc");
        assert!(!no_dlc_overrides.contains("tp2 = \"cdtweaks\""));

        let with_dlc_overrides = overrides_text_for("bgee-vanilla-plus");
        assert!(with_dlc_overrides.contains("tp2 = \"cdtweaks\""));
    }

    fn log_lines(text: &str) -> Vec<&str> {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .collect()
    }

    fn parsed_lines(text: &str) -> Vec<Component> {
        log_lines(text)
            .into_iter()
            .map(|line| Component::parse_weidu_line(line).expect("payload line parses"))
            .collect()
    }

    #[test]
    fn eet_essentials_order_holds_the_reference_spine() {
        let entry = by_id("eet-essentials");
        let preview = preview_modlist_share_code(&entry.code).expect("parse");
        let first_tab = parsed_lines(&preview.bgee_log_text);
        let second_tab = parsed_lines(&preview.bg2ee_log_text);

        let bgee_keys: Vec<(&str, &str)> = first_tab
            .iter()
            .map(|c| (c.name.as_str(), c.component.as_str()))
            .collect();
        assert_eq!(
            bgee_keys,
            vec![("DlcMerger", "1"), ("EEFixPack", "0"), ("EEFixPack", "2")]
        );

        let first_bg2ee: Vec<(&str, &str)> = second_tab[..3]
            .iter()
            .map(|c| (c.name.as_str(), c.component.as_str()))
            .collect();
        assert_eq!(
            first_bg2ee,
            vec![("EEFixPack", "0"), ("EEFixPack", "2"), ("EET", "0")]
        );
        let last_bg2ee = second_tab.last().expect("bg2ee mods non-empty");
        assert_eq!(last_bg2ee.name, "EET_end");
        assert_eq!(last_bg2ee.component, "0");

        let eeex_ids: Vec<&str> = second_tab
            .iter()
            .filter(|c| c.name == "EEex")
            .map(|c| c.component.as_str())
            .collect();
        assert_eq!(eeex_ids, vec!["0", "1", "2", "3", "4", "5", "6", "7"]);

        for components in [&first_tab, &second_tab] {
            let mut seen_runs: Vec<(&str, &str)> = Vec::new();
            let mut previous: Option<(&str, &str)> = None;
            for component in components {
                let key = (component.name.as_str(), component.tp_file.as_str());
                if previous != Some(key) {
                    assert!(
                        !seen_runs.contains(&key),
                        "{key:?} appears in two separate runs"
                    );
                    seen_runs.push(key);
                }
                previous = Some(key);
            }
        }
    }

    #[test]
    fn non_bg2ee_targets_land_in_the_first_game_log() {
        let entry = by_id("iwdee-essentials");
        let preview = preview_modlist_share_code(&entry.code).expect("parse");
        assert_eq!(preview.bgee_entries, 1);
        assert_eq!(preview.bg2ee_entries, 0);
    }

    #[test]
    fn fixpack_lists_install_core_fixes_and_game_text_update_on_every_tab() {
        let fixes_entry = by_id("eet-plus-fixes");
        let fixes_preview = preview_modlist_share_code(&fixes_entry.code).expect("parse");
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

        let vanilla_entry = by_id("bgee-vanilla-plus");
        let vanilla_preview = preview_modlist_share_code(&vanilla_entry.code).expect("parse");
        assert_eq!(vanilla_preview.bgee_entries, 66);

        let vanilla_lines = log_lines(&vanilla_preview.bgee_log_text);
        assert!(vanilla_lines[0].contains("DLCMERGER.TP2~ #0 #1"));
        assert!(vanilla_lines[1].contains("SETUP-EEFIXPACK.TP2~ #0 #0"));
        assert!(vanilla_lines[2].contains("SETUP-EEFIXPACK.TP2~ #0 #2"));
        assert!(vanilla_lines[3].contains("EEEX.TP2~ #0 #0"));
        assert!(!vanilla_preview.bgee_log_text.contains("EET\\EET.TP2"));
        assert!(!vanilla_preview.bgee_log_text.contains("EET_TWEAKS.TP2"));
        assert!(
            !vanilla_preview
                .bgee_log_text
                .contains("HQ_SOUNDCLIPS_BG2EE.TP2")
        );
        assert!(
            !vanilla_preview
                .bgee_log_text
                .contains("SETUP-CDTWEAKS.TP2~ #0 #4031")
        );
        assert!(
            !vanilla_preview
                .bgee_log_text
                .contains("SETUP-CDTWEAKS.TP2~ #0 #4041")
        );
        assert!(
            !vanilla_preview
                .bgee_log_text
                .contains("SETUP-CDTWEAKS.TP2~ #0 #4061")
        );
        assert!(
            !vanilla_preview
                .bgee_log_text
                .contains("SETUP-CDTWEAKS.TP2~ #0 #4071")
        );

        let no_dlc_entry = by_id("bgee-vanilla-plus-no-dlc");
        let no_dlc_preview = preview_modlist_share_code(&no_dlc_entry.code).expect("parse");
        assert_eq!(no_dlc_preview.bgee_entries, 40);

        let no_dlc_lines = log_lines(&no_dlc_preview.bgee_log_text);
        assert!(no_dlc_lines[0].contains("SETUP-EEFIXPACK.TP2~ #0 #0"));
        assert!(no_dlc_lines[1].contains("SETUP-EEFIXPACK.TP2~ #0 #2"));
        assert!(!no_dlc_preview.bgee_log_text.contains("DLCMERGER.TP2"));
        assert!(!no_dlc_preview.bgee_log_text.contains("CDTWEAKS"));
        assert_eq!(
            no_dlc_lines,
            vanilla_lines
                .iter()
                .filter(
                    |line| !line.contains("DLCMERGER.TP2") && !line.contains("SETUP-CDTWEAKS.TP2")
                )
                .copied()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn eet_lists_carry_the_bg1_folder_prompt_on_the_eet_core_line() {
        for id in ["eet-essentials", "eet-plus-fixes"] {
            let entry = by_id(id);
            let preview = preview_modlist_share_code(&entry.code).expect("parse");
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
    fn versioned_lines_keep_their_labels() {
        fn split_label(label: &str) -> (String, String) {
            let mut parts = label.splitn(2, "->");
            let name = parts.next().unwrap_or_default().trim().to_string();
            let sub = parts.next().unwrap_or_default().trim().to_string();
            (name, sub)
        }

        fn assert_versioned(components: &[Component], labels: &[&str]) {
            assert_eq!(components.len(), labels.len());
            for (component, label) in components.iter().zip(labels.iter()) {
                if component.version.is_empty() {
                    continue;
                }
                let (expected_name, expected_sub) = split_label(label);
                assert_eq!(component.component_name, expected_name);
                assert_eq!(component.sub_component, expected_sub);
            }
        }

        let fixes_entry = by_id("eet-plus-fixes");
        let fixes_preview = preview_modlist_share_code(&fixes_entry.code).expect("parse");
        assert_versioned(
            &parsed_lines(&fixes_preview.bgee_log_text),
            &[
                "Merge DLC into game -> Siege of Dragonspear",
                "Core Fixes",
                "Game Text Update",
            ],
        );
        assert_versioned(
            &parsed_lines(&fixes_preview.bg2ee_log_text),
            &[
                "Core Fixes",
                "Game Text Update",
                "EET core (resource importation)",
                "EET end (last mod in install order)",
            ],
        );

        let vanilla_entry = by_id("bgee-vanilla-plus");
        let vanilla_preview = preview_modlist_share_code(&vanilla_entry.code).expect("parse");
        let vanilla_lines = parsed_lines(&vanilla_preview.bgee_log_text);
        assert_versioned(
            &vanilla_lines[..4],
            &[
                "Merge DLC into game -> Siege of Dragonspear",
                "Core Fixes",
                "Game Text Update",
                "Quick Menu Core",
            ],
        );

        let no_dlc_entry = by_id("bgee-vanilla-plus-no-dlc");
        let no_dlc_preview = preview_modlist_share_code(&no_dlc_entry.code).expect("parse");
        let no_dlc_lines = parsed_lines(&no_dlc_preview.bgee_log_text);
        assert_versioned(
            &no_dlc_lines[..3],
            &["Core Fixes", "Game Text Update", "Quick Menu Core"],
        );

        let iwdee_entry = by_id("iwdee-essentials");
        let iwdee_preview = preview_modlist_share_code(&iwdee_entry.code).expect("parse");
        assert_versioned(
            &parsed_lines(&iwdee_preview.bgee_log_text),
            &["Increase Ammo Stacking"],
        );

        let eet_essentials_entry = by_id("eet-essentials");
        let eet_essentials_preview =
            preview_modlist_share_code(&eet_essentials_entry.code).expect("parse");
        assert_versioned(
            &parsed_lines(&eet_essentials_preview.bgee_log_text),
            &[
                "Merge DLC into game -> Siege of Dragonspear",
                "Core Fixes",
                "Game Text Update",
            ],
        );
    }

    #[test]
    fn eet_essentials_shows_real_versions_in_the_inside_model() {
        use crate::app::mod_downloads::source_tiers_from_texts;
        use crate::ui::install::inside_model;

        let entry = by_id("eet-essentials");
        let preview = preview_modlist_share_code(&entry.code).expect("parse");
        let tiers = source_tiers_from_texts(
            include_str!("../../../core/config/default_mod_downloads.toml"),
            "",
            &preview.source_overrides_text,
        );
        let model = inside_model::build(&preview, &tiers);

        let version_of = |folder: &str| -> String {
            model
                .sections
                .iter()
                .find_map(|section| {
                    section
                        .mods
                        .iter()
                        .find(|group| group.folder.eq_ignore_ascii_case(folder))
                        .map(|group| group.version.clone())
                })
                .unwrap_or_else(|| panic!("{folder} must appear in the inside model"))
        };

        assert_eq!(version_of("DlcMerger"), "1.8");
        assert_eq!(version_of("EEFixPack"), "");
        assert_eq!(version_of("EET"), "v14.0");
        assert_eq!(version_of("CDTweaks"), "v18");
        assert_eq!(version_of("LeUI"), "4.9.1");
    }

    const BG2EE_FOLDERS_NOT_CARRIED_OVER: [&str; 5] = [
        "EET",
        "EET_Tweaks",
        "EET_end",
        "HQ_SoundClips_BG2EE",
        "EEFixPack",
    ];
    const DROPPED_CDTWEAKS_COMPONENTS: [&str; 4] = ["4031", "4041", "4061", "4071"];

    fn should_drop_bg2ee_line(line: &str) -> bool {
        let component = Component::parse_weidu_line(line).expect("payload line parses");
        BG2EE_FOLDERS_NOT_CARRIED_OVER
            .iter()
            .any(|folder| folder.eq_ignore_ascii_case(&component.name))
            || (component.name.eq_ignore_ascii_case("CDTweaks")
                && DROPPED_CDTWEAKS_COMPONENTS.contains(&component.component.as_str()))
    }

    fn derive_with_dlc_bgee_log(
        eet_essentials_preview: &crate::app::modlist_share::ModlistSharePreview,
    ) -> String {
        let header = [
            "// Log of Currently Installed WeiDU Mods",
            "// The top of the file is the 'oldest' mod",
            "// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]",
        ];
        let mut out: Vec<&str> = header.to_vec();
        out.extend(log_lines(&eet_essentials_preview.bgee_log_text));
        out.extend(
            log_lines(&eet_essentials_preview.bg2ee_log_text)
                .into_iter()
                .filter(|line| !should_drop_bg2ee_line(line)),
        );
        out.join("\n")
    }

    #[test]
    fn bgee_vanilla_plus_is_eet_essentials_minus_the_eet_only_pieces() {
        let eet_essentials_entry = by_id("eet-essentials");
        let eet_essentials_preview =
            preview_modlist_share_code(&eet_essentials_entry.code).expect("parse");
        let expected_with_dlc = derive_with_dlc_bgee_log(&eet_essentials_preview);

        let vanilla_entry = by_id("bgee-vanilla-plus");
        let vanilla_preview = preview_modlist_share_code(&vanilla_entry.code).expect("parse");
        assert_eq!(vanilla_preview.bgee_log_text, expected_with_dlc);

        let expected_no_dlc = expected_with_dlc
            .lines()
            .filter(|line| !line.contains("DLCMERGER.TP2") && !line.contains("SETUP-CDTWEAKS.TP2"))
            .collect::<Vec<_>>()
            .join("\n");
        let no_dlc_entry = by_id("bgee-vanilla-plus-no-dlc");
        let no_dlc_preview = preview_modlist_share_code(&no_dlc_entry.code).expect("parse");
        assert_eq!(no_dlc_preview.bgee_log_text, expected_no_dlc);
    }
}
