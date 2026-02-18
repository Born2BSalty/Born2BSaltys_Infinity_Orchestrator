// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn is_step1_valid(s: &Step1State) -> bool {
    if !has_value(&s.mods_folder) || !has_value(&s.weidu_binary) {
        return false;
    }

    match s.game_install.as_str() {
        "BG2EE" => {
            if !has_value(&s.bg2ee_game_folder) {
                return false;
            }
            if s.have_weidu_logs {
                has_value(&s.bg2ee_log_file)
            } else {
                has_value(&s.bg2ee_log_folder)
            }
        }
        "EET" => {
            if !has_value(&s.eet_bgee_game_folder) || !has_value(&s.eet_bg2ee_game_folder) {
                return false;
            }
            if s.have_weidu_logs {
                has_value(&s.bgee_log_file) && has_value(&s.bg2ee_log_file)
            } else {
                has_value(&s.eet_bgee_log_folder) && has_value(&s.eet_bg2ee_log_folder)
            }
        }
        _ => {
            if !has_value(&s.bgee_game_folder) {
                return false;
            }
            if s.have_weidu_logs {
                has_value(&s.bgee_log_file)
            } else {
                has_value(&s.bgee_log_folder)
            }
        }
    }
}
