// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use tracing::{info, warn};

use crate::config::options::CoreOptions;
use crate::install::plan::InstallPlan;
use crate::mods::discovery::DiscoveryIndex;
use crate::install::weidu_exec;

pub fn run_plan(plan: &InstallPlan, options: &CoreOptions, game_directory: &Path) -> Result<()> {
    let index = DiscoveryIndex::build(&options.mod_directories, options.depth);
    let mut missing = Vec::new();
    let mut resolved_sources = Vec::new();

    for component in &plan.components {
        if let Some(folder) = index.find_folder(component) {
            info!(
                "preflight found mod folder: name={} tp_file={} folder={}",
                component.name,
                component.tp_file,
                folder.display()
            );
            resolved_sources.push((component, folder.to_path_buf()));
        } else {
            warn!(
                "preflight missing mod folder: name={} tp_file={}",
                component.name, component.tp_file
            );
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
    info!("install preflight passed for {} component(s)", plan.components.len());

    for (component, source_folder) in resolved_sources {
        let mod_folder_in_game =
            stage_mod_folder(game_directory, &source_folder, &component.name, options.overwrite)?;
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
