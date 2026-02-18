// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::Path;

use crate::compat::parse_tp2_rules;
use crate::ui::state::Step2ModState;

use super::WizardApp;

pub(in crate::ui::app) fn refresh_validator_tp2_metadata(app: &mut WizardApp) {
    let tp2_metadata = parse_tp2_metadata_from_mods(&app.state.step2.bgee_mods, &app.state.step2.bg2ee_mods);
    app.compat_validator.set_tp2_metadata(tp2_metadata);
}

pub(in crate::ui::app) fn parse_tp2_metadata_from_mods(
    bgee_mods: &[Step2ModState],
    bg2ee_mods: &[Step2ModState],
) -> HashMap<String, crate::compat::Tp2Metadata> {
    let mut metadata = HashMap::new();

    for mod_state in bgee_mods.iter().chain(bg2ee_mods.iter()) {
        if mod_state.tp2_path.is_empty() {
            continue;
        }
        let path = Path::new(&mod_state.tp2_path);
        if !path.exists() {
            continue;
        }
        let key = normalize_tp2_key(&mod_state.tp_file);
        if metadata.contains_key(&key) {
            continue;
        }
        let parsed = parse_tp2_rules(path);
        metadata.insert(key, parsed);
    }

    metadata
}

fn normalize_tp2_key(tp_file: &str) -> String {
    let lower = tp_file.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}
