// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tracing::info;

use crate::config::options::CoreOptions;
use crate::install::plan::InstallPlan;
use crate::install::weidu_exec;
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

pub fn run_plan(plan: &InstallPlan, options: &CoreOptions, game_directory: &Path) -> Result<()> {
    let resolved_paths =
        check_missing_mod_folders(&options.mod_directories, options.depth, &plan.components)?;
    info!(
        "install preflight passed for {} component(s)",
        plan.components.len()
    );

    for (component, source_folder) in plan.components.iter().zip(resolved_paths) {
        info!(
            "preflight found mod folder: name={} tp_file={} folder={}",
            component.name,
            component.tp_file,
            source_folder.display()
        );
        let mod_folder_in_game = stage_mod_folder(
            game_directory,
            &source_folder,
            &component.name,
            options.overwrite,
        )?;
        info!(
            "install executing component: name={} component={} tp_file={}",
            component.name, component.component, component.tp_file
        );
        weidu_exec::execute_component(game_directory, &mod_folder_in_game, component, options)?;
        info!(
            "install completed component: name={} component={}",
            component.name, component.component
        );
    }
    Ok(())
}

fn stage_mod_folder(
    game_directory: &Path,
    source_folder: &Path,
    mod_name: &str,
    overwrite: bool,
) -> Result<PathBuf> {
    let target_folder = game_directory.join(mod_name);
    if target_folder.exists() {
        if overwrite {
            std::fs::remove_dir_all(&target_folder)?;
        } else {
            return Ok(target_folder);
        }
    }
    copy_dir_all(source_folder, &target_folder)?;
    Ok(target_folder)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
