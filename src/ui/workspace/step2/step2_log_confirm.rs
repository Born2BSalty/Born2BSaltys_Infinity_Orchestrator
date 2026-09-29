// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::game_authority::{self, GameSlot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeiduLogImportRow {
    pub tab: &'static str,
    pub first_slot: bool,
}

#[must_use]
pub fn weidu_log_import_text(game_install: &str) -> (String, String) {
    match game_authority::tabs_for_install(game_install) {
        [tab] => {
            let title = format!("Replace {tab} selections from a WeiDU log?");
            let body = format!(
                "This will overwrite every component selection on the {tab} bucket \
                 with the contents of the chosen weidu.log. Make sure the log was \
                 produced from the same mod versions you have downloaded — otherwise \
                 components may resolve to the wrong rows or fail to install."
            );
            (title, body)
        }
        _ => (
            "Replace BGEE and BG2EE selections from WeiDU logs?".to_string(),
            "This will overwrite every component selection on the BGEE and BG2EE \
             buckets with the contents of the chosen weidu.logs. Make sure the logs \
             were produced from the same mod versions you have downloaded — \
             otherwise components may resolve to the wrong rows or fail to install."
                .to_string(),
        ),
    }
}

#[must_use]
pub fn weidu_log_import_rows(game_install: &str) -> Vec<WeiduLogImportRow> {
    game_authority::tabs_for_install(game_install)
        .iter()
        .copied()
        .map(|tab| WeiduLogImportRow {
            tab,
            first_slot: game_authority::slot_for_tab(tab) == GameSlot::First,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_log_games_keep_todays_copy() {
        let (title, body) = weidu_log_import_text("BGEE");
        assert_eq!(title, "Replace BGEE selections from a WeiDU log?");
        assert_eq!(
            body,
            "This will overwrite every component selection on the BGEE \
             bucket with the contents of the chosen weidu.log. Make sure \
             the log was produced from the same mod versions you have \
             downloaded — otherwise components may resolve to the wrong \
             rows or fail to install."
        );
        let (iwd_title, iwd_body) = weidu_log_import_text("IWDEE");
        assert_eq!(iwd_title, "Replace IWDEE selections from a WeiDU log?");
        assert!(iwd_body.contains("on the IWDEE bucket"));
        let (bg2_title, _) = weidu_log_import_text("BG2EE");
        assert_eq!(bg2_title, "Replace BG2EE selections from a WeiDU log?");
    }

    #[test]
    fn eet_copy_names_both_tabs() {
        let (title, body) = weidu_log_import_text("EET");
        assert_eq!(title, "Replace BGEE and BG2EE selections from WeiDU logs?");
        assert_eq!(
            body,
            "This will overwrite every component selection on the BGEE and \
             BG2EE buckets with the contents of the chosen weidu.logs. Make \
             sure the logs were produced from the same mod versions you have \
             downloaded — otherwise components may resolve to the wrong rows \
             or fail to install."
        );
    }

    #[test]
    fn rows_follow_the_game_tab_table() {
        assert_eq!(
            weidu_log_import_rows("EET"),
            vec![
                WeiduLogImportRow {
                    tab: "BGEE",
                    first_slot: true,
                },
                WeiduLogImportRow {
                    tab: "BG2EE",
                    first_slot: false,
                },
            ]
        );
        assert_eq!(
            weidu_log_import_rows("BGEE"),
            vec![WeiduLogImportRow {
                tab: "BGEE",
                first_slot: true,
            }]
        );
        assert_eq!(
            weidu_log_import_rows("BG2EE"),
            vec![WeiduLogImportRow {
                tab: "BG2EE",
                first_slot: false,
            }]
        );
        assert_eq!(
            weidu_log_import_rows("IWDEE"),
            vec![WeiduLogImportRow {
                tab: "IWDEE",
                first_slot: true,
            }]
        );
    }
}
