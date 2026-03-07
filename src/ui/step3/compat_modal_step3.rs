// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) mod compat_model {
    pub(crate) fn normalize_mod_key(value: &str) -> String {
        let lower = value.to_ascii_lowercase();
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) enum CompatJumpAction {
        Auto(String),
        Affected(String),
        Related(String),
    }
}

pub(crate) mod compat_modal {
    pub(crate) use crate::ui::step3::compat_modal_view_step3::*;
}
