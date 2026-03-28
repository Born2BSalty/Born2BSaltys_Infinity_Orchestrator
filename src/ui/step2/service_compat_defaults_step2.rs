// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io;

pub(crate) fn ensure_compat_rules_files() -> io::Result<()> {
    let default_path = crate::ui::compat_rules::compat_rules_default_path();
    let user_path = crate::ui::compat_rules::compat_rules_user_path();
    let legacy_user_path = crate::ui::compat_rules::compat_rules_legacy_user_path();

    if let Some(parent) = default_path.parent() {
        fs::create_dir_all(parent)?;
    }

    write_if_changed(&default_path, default_step2_rules_content())?;

    if !user_path.exists() && legacy_user_path.exists() {
        fs::copy(&legacy_user_path, &user_path)?;
    }

    if !user_path.exists() {
        fs::write(&user_path, user_step2_rules_content())?;
    }

    Ok(())
}

fn write_if_changed(path: &std::path::Path, content: &str) -> io::Result<()> {
    match fs::read_to_string(path) {
        Ok(existing) if existing == content => Ok(()),
        _ => fs::write(path, content),
    }
}

fn default_step2_rules_content() -> &'static str {
    include_str!("default_step2_compat_rules.toml")
}

fn user_step2_rules_content() -> &'static str {
    include_str!("user_step2_compat_rules.toml")
}
