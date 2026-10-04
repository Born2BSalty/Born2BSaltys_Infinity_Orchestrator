// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::mods::discovery::DiscoveryIndex;

pub fn check_missing_mod_folders(
    mods_dir: &Path,
    depth: usize,
    components: &[crate::mods::component::Component],
) -> Result<Vec<PathBuf>> {
    let index = DiscoveryIndex::build(mods_dir, depth)?;
    let mut missing = Vec::new();
    let mut resolved = Vec::new();

    for component in components {
        if let Some(folder) = index.find_folder(component) {
            resolved.push(folder.to_path_buf());
        } else {
            missing.push(format!("{}/{}", component.name, component.tp_file));
        }
    }

    if !missing.is_empty() {
        return Err(anyhow!(
            "missing mod folders for {} component(s): {}",
            missing.len(),
            missing.join(", ")
        ));
    }

    Ok(resolved)
}
