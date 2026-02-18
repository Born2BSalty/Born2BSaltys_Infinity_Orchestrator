// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use wait_timeout::ChildExt;

use crate::config::options::CoreOptions;
use crate::mods::component::Component;

pub fn execute_component(
    game_directory: &Path,
    mod_folder_in_game: &Path,
    component: &Component,
    options: &CoreOptions,
) -> Result<()> {
    if options.weidu_binary.as_os_str().is_empty() {
        return Err(anyhow!(
            "--weidu-binary is required to execute installs"
        ));
    }
    let tp2_path = mod_folder_in_game.join(&component.tp_file);
    let args = build_args(&tp2_path, component, options);
    let mut child = Command::new(&options.weidu_binary)
        .current_dir(game_directory)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn weidu for component {} {}",
                component.name, component.component
            )
        })?;

    let timeout = Duration::from_secs(options.timeout as u64);
    match child.wait_timeout(timeout)? {
        Some(_) => {
            let output = child.wait_with_output()?;
            if !output.status.success() {
                return Err(anyhow!(
                    "weidu failed for {} {} (exit={:?})\nstdout:\n{}\nstderr:\n{}",
                    component.name,
                    component.component,
                    output.status.code(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            Ok(())
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Err(anyhow!(
                "weidu timed out after {} seconds for {} {}",
                options.timeout,
                component.name,
                component.component
            ))
        }
    }
}

fn build_args(tp2_path: &Path, component: &Component, options: &CoreOptions) -> Vec<String> {
    let mut args = vec![
        normalize_path(tp2_path),
        "--force-install".to_string(),
        component.component.clone(),
        "--use-lang".to_string(),
        options.language.clone(),
        "--language".to_string(),
        component.lang.clone(),
        "--no-exit-pause".to_string(),
    ];
    for token in options
        .weidu_log_mode
        .split(',')
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        if let Some(path) = token.strip_prefix("log ") {
            args.push("--log".to_string());
            args.push(path.trim().to_string());
            continue;
        }
        match token {
            "autolog" => args.push("--autolog".to_string()),
            "logapp" => args.push("--logapp".to_string()),
            "log-extern" => args.push("--log-extern".to_string()),
            "log" => args.push("--log".to_string()),
            _ => {}
        }
    }
    args
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
