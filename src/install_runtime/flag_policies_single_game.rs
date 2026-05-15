// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::state::WizardState;

const BGEE_TARGET_DIR: &str = "Baldur's Gate Enhanced Edition";
const BG2EE_TARGET_DIR: &str = "Baldur's Gate II Enhanced Edition";
const IWDEE_TARGET_DIR: &str = "Icewind Dale Enhanced Edition";

pub fn apply(state: &mut WizardState, destination_folder: &Path) -> Result<(), String> {
    let target_name = match state.step1.game_install.as_str() {
        "BG2EE" => BG2EE_TARGET_DIR,
        "IWDEE" => IWDEE_TARGET_DIR,
        _ => BGEE_TARGET_DIR,
    };
    let target_dir = destination_folder.join(target_name);
    std::fs::create_dir_all(&target_dir).map_err(|err| {
        format!(
            "failed to create generated game folder {}: {err}",
            target_dir.display()
        )
    })?;

    let step1 = &mut state.step1;
    step1.generate_directory_enabled = true;
    step1.new_pre_eet_dir_enabled = false;
    step1.new_eet_dir_enabled = false;
    step1.generate_directory = target_dir.to_string_lossy().to_string();
    Ok(())
}
