// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;

use crate::ui::step2::service_compat_defaults_step2::step2_compat_rules_path;
use crate::ui::step2::service_compat_rules_step2::compat_model::{
    Step2CompatRule, Step2CompatRulesFile,
};

pub(crate) fn load_rules() -> Vec<Step2CompatRule> {
    let path = step2_compat_rules_path();
    let content = match fs::read_to_string(&path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let parsed = match toml::from_str::<Step2CompatRulesFile>(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    parsed
        .rules
        .into_iter()
        .filter(|r| r.enabled && !r.r#mod.trim().is_empty())
        .collect()
}
