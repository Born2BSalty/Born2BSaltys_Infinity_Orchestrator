// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::state::WizardState;

const BGEE_TARGET_DIR: &str = "Baldur's Gate Enhanced Edition";
const BG2EE_TARGET_DIR: &str = "Baldur's Gate II Enhanced Edition";

pub fn apply(state: &mut WizardState, destination_folder: &Path) -> Result<(), String> {
    let pre_eet_dir = destination_folder.join(BGEE_TARGET_DIR);
    let eet_final_dir = destination_folder.join(BG2EE_TARGET_DIR);
    create_dir(&pre_eet_dir, "Pre-EET target")?;
    create_dir(&eet_final_dir, "EET final target")?;

    let step1 = &mut state.step1;
    step1.new_pre_eet_dir_enabled = true;
    step1.new_eet_dir_enabled = true;
    step1.generate_directory_enabled = false;
    step1.eet_pre_dir = pre_eet_dir.to_string_lossy().to_string();
    step1.eet_new_dir = eet_final_dir.to_string_lossy().to_string();
    Ok(())
}

fn create_dir(path: &Path, label: &str) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|err| format!("failed to create {label} folder {}: {err}", path.display()))
}
