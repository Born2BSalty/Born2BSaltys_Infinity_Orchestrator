// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::compat_dlc_source::{bg2ee_source_for, bgee_source_for};
use crate::app::state::Step1State;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameSlot {
    First,
    Second,
}

pub const TAB_BGEE: &str = "BGEE";
pub const TAB_BG2EE: &str = "BG2EE";
pub const TAB_IWDEE: &str = "IWDEE";

#[must_use]
pub fn slot_for_tab(tab: &str) -> GameSlot {
    if tab.trim().eq_ignore_ascii_case(TAB_BG2EE) {
        GameSlot::Second
    } else {
        GameSlot::First
    }
}

#[must_use]
pub fn tabs_for_install(game_install: &str) -> &'static [&'static str] {
    match game_install.trim() {
        "BG2EE" => &[TAB_BG2EE],
        "IWDEE" => &[TAB_IWDEE],
        "EET" => &[TAB_BGEE, TAB_BG2EE],
        _ => &[TAB_BGEE],
    }
}

#[must_use]
pub fn first_slot_tab(game_install: &str) -> &'static str {
    if game_install.trim() == "IWDEE" {
        TAB_IWDEE
    } else {
        TAB_BGEE
    }
}

#[must_use]
pub fn has_slot(game_install: &str, slot: GameSlot) -> bool {
    tabs_for_install(game_install)
        .iter()
        .any(|tab| slot_for_tab(tab) == slot)
}

#[must_use]
pub fn normalized_tab(game_install: &str, current: &str) -> &'static str {
    let tabs = tabs_for_install(game_install);
    tabs.iter()
        .find(|tab| tab.eq_ignore_ascii_case(current.trim()))
        .copied()
        .unwrap_or_else(|| tabs[0])
}

#[must_use]
pub fn source_game_folders<'a>(
    step1: &'a Step1State,
    game_install: &str,
) -> Vec<(&'static str, &'a str)> {
    match game_install.trim() {
        "BGEE" => vec![("BGEE", step1.bgee_game_folder.trim())],
        "BG2EE" => vec![("BG2EE", step1.bg2ee_game_folder.trim())],
        "IWDEE" => vec![("IWDEE", step1.iwdee_game_folder.trim())],
        "EET" => vec![
            ("BGEE", bgee_source_for(step1, "EET")),
            ("BG2EE", bg2ee_source_for(step1, "EET")),
        ],
        _ => vec![],
    }
}

#[must_use]
pub fn single_game_source_folder<'a>(step1: &'a Step1State, game_install: &str) -> &'a str {
    match game_install.trim() {
        "BG2EE" => step1.bg2ee_game_folder.trim(),
        "IWDEE" => step1.iwdee_game_folder.trim(),
        _ => step1.bgee_game_folder.trim(),
    }
}

#[must_use]
pub fn missing_source_folders(step1: &Step1State, game_install: &str) -> Vec<&'static str> {
    source_game_folders(step1, game_install)
        .into_iter()
        .filter(|(_, folder)| folder.is_empty())
        .map(|(label, _)| label)
        .collect()
}

#[must_use]
pub fn missing_source_message(labels: &[&str]) -> Option<String> {
    match labels.len() {
        0 => None,
        1 => Some(format!(
            "The {} game folder is not set. Set it in Settings \u{2192} Paths.",
            labels[0]
        )),
        _ => Some(format!(
            "The {} game folders are not set. Set them in Settings \u{2192} Paths.",
            labels.join(" and ")
        )),
    }
}

#[must_use]
pub fn compat_game_token(tab: &str) -> &'static str {
    match tab.trim() {
        t if t.eq_ignore_ascii_case(TAB_BG2EE) => "bg2ee",
        t if t.eq_ignore_ascii_case(TAB_IWDEE) => "iwdee",
        _ => "bgee",
    }
}

#[must_use]
pub fn log_source_subdir(tab: &str) -> &'static str {
    compat_game_token(tab)
}

#[must_use]
pub fn share_log_key(tab: &str) -> &'static str {
    compat_game_token(tab)
}

#[must_use]
pub fn reference_log_name(tab: &str) -> &'static str {
    match tab.trim() {
        t if t.eq_ignore_ascii_case(TAB_BG2EE) => "BG2EE.log",
        t if t.eq_ignore_ascii_case(TAB_IWDEE) => "IWDEE.log",
        _ => "BGEE.log",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_per_install_match_the_table() {
        assert_eq!(tabs_for_install("BGEE"), [TAB_BGEE]);
        assert_eq!(tabs_for_install("BG2EE"), [TAB_BG2EE]);
        assert_eq!(tabs_for_install("IWDEE"), [TAB_IWDEE]);
        assert_eq!(tabs_for_install("EET"), [TAB_BGEE, TAB_BG2EE]);
        assert_eq!(tabs_for_install("garbage"), [TAB_BGEE]);
        assert_eq!(tabs_for_install(""), [TAB_BGEE]);
    }

    #[test]
    fn slot_for_tab_treats_only_bg2ee_as_second() {
        assert_eq!(slot_for_tab("bg2ee"), GameSlot::Second);
        assert_eq!(slot_for_tab("BG2EE"), GameSlot::Second);
        assert_eq!(slot_for_tab(" BG2EE "), GameSlot::Second);
        assert_eq!(slot_for_tab("IWDEE"), GameSlot::First);
        assert_eq!(slot_for_tab("BGEE"), GameSlot::First);
        assert_eq!(slot_for_tab(""), GameSlot::First);
        assert_eq!(slot_for_tab("garbage"), GameSlot::First);
    }

    #[test]
    fn first_slot_tab_is_iwdee_only_for_iwdee() {
        assert_eq!(first_slot_tab("IWDEE"), TAB_IWDEE);
        assert_eq!(first_slot_tab("BGEE"), TAB_BGEE);
        assert_eq!(first_slot_tab("BG2EE"), TAB_BGEE);
        assert_eq!(first_slot_tab("EET"), TAB_BGEE);
        assert_eq!(first_slot_tab("garbage"), TAB_BGEE);
    }

    #[test]
    fn has_slot_matches_the_table() {
        assert!(has_slot("BGEE", GameSlot::First));
        assert!(!has_slot("BGEE", GameSlot::Second));
        assert!(!has_slot("BG2EE", GameSlot::First));
        assert!(has_slot("BG2EE", GameSlot::Second));
        assert!(has_slot("IWDEE", GameSlot::First));
        assert!(!has_slot("IWDEE", GameSlot::Second));
        assert!(has_slot("EET", GameSlot::First));
        assert!(has_slot("EET", GameSlot::Second));
    }

    #[test]
    fn normalized_tab_corrects_a_stale_tab() {
        assert_eq!(normalized_tab("IWDEE", "BG2EE"), TAB_IWDEE);
        assert_eq!(normalized_tab("IWDEE", "BGEE"), TAB_IWDEE);
        assert_eq!(normalized_tab("EET", "BG2EE"), TAB_BG2EE);
        assert_eq!(normalized_tab("EET", "IWDEE"), TAB_BGEE);
        assert_eq!(normalized_tab("BG2EE", "BGEE"), TAB_BG2EE);
        assert_eq!(normalized_tab("BGEE", "bgee"), TAB_BGEE);
    }

    fn fixture_step1() -> Step1State {
        Step1State {
            bgee_game_folder: "  /games/bgee  ".to_string(),
            bg2ee_game_folder: "  /games/bg2ee  ".to_string(),
            iwdee_game_folder: "  /games/iwdee  ".to_string(),
            ..Step1State::default()
        }
    }

    #[test]
    fn source_folders_read_each_games_own_field() {
        let step1 = fixture_step1();
        assert_eq!(
            source_game_folders(&step1, "BGEE"),
            vec![("BGEE", "/games/bgee")]
        );
        assert_eq!(
            source_game_folders(&step1, "BG2EE"),
            vec![("BG2EE", "/games/bg2ee")]
        );
        assert_eq!(
            source_game_folders(&step1, "IWDEE"),
            vec![("IWDEE", "/games/iwdee")]
        );
        assert_eq!(source_game_folders(&step1, "garbage").len(), 0);
        let eet = source_game_folders(&step1, "EET");
        assert_eq!(
            eet,
            vec![
                ("BGEE", bgee_source_for(&step1, "EET")),
                ("BG2EE", bg2ee_source_for(&step1, "EET")),
            ]
        );
    }

    #[test]
    fn single_game_source_folder_per_game() {
        let step1 = fixture_step1();
        assert_eq!(single_game_source_folder(&step1, "BGEE"), "/games/bgee");
        assert_eq!(single_game_source_folder(&step1, "BG2EE"), "/games/bg2ee");
        assert_eq!(single_game_source_folder(&step1, "IWDEE"), "/games/iwdee");
        assert_eq!(single_game_source_folder(&step1, "garbage"), "/games/bgee");
    }

    #[test]
    fn missing_source_folders_and_message() {
        let mut step1 = Step1State {
            bgee_game_folder: "/games/bgee".to_string(),
            bg2ee_game_folder: "/games/bg2ee".to_string(),
            iwdee_game_folder: String::new(),
            ..Step1State::default()
        };
        assert_eq!(missing_source_folders(&step1, "IWDEE"), vec!["IWDEE"]);
        assert_eq!(
            missing_source_message(&missing_source_folders(&step1, "IWDEE")),
            Some(
                "The IWDEE game folder is not set. Set it in Settings \u{2192} Paths.".to_string()
            )
        );
        step1.bgee_game_folder = String::new();
        step1.bg2ee_game_folder = String::new();
        assert_eq!(missing_source_folders(&step1, "EET"), vec!["BGEE", "BG2EE"]);
        assert_eq!(
            missing_source_message(&missing_source_folders(&step1, "EET")),
            Some(
                "The BGEE and BG2EE game folders are not set. Set them in Settings \u{2192} Paths."
                    .to_string()
            )
        );
        step1.iwdee_game_folder = "/games/iwdee".to_string();
        assert_eq!(missing_source_folders(&step1, "IWDEE").len(), 0);
        assert_eq!(
            missing_source_message(&missing_source_folders(&step1, "IWDEE")),
            None
        );
    }

    #[test]
    fn identity_strings_per_tab() {
        for tab in [TAB_BGEE, "bgee"] {
            assert_eq!(compat_game_token(tab), "bgee");
            assert_eq!(log_source_subdir(tab), "bgee");
            assert_eq!(share_log_key(tab), "bgee");
            assert_eq!(reference_log_name(tab), "BGEE.log");
        }
        for tab in [TAB_BG2EE, "bg2ee"] {
            assert_eq!(compat_game_token(tab), "bg2ee");
            assert_eq!(log_source_subdir(tab), "bg2ee");
            assert_eq!(share_log_key(tab), "bg2ee");
            assert_eq!(reference_log_name(tab), "BG2EE.log");
        }
        for tab in [TAB_IWDEE, "iwdee"] {
            assert_eq!(compat_game_token(tab), "iwdee");
            assert_eq!(log_source_subdir(tab), "iwdee");
            assert_eq!(share_log_key(tab), "iwdee");
            assert_eq!(reference_log_name(tab), "IWDEE.log");
        }
    }
}
