// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn hidden_command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub(super) fn run_weidu(
    weidu_binary: &Path,
    args: &[String],
    working_directory: Option<&Path>,
) -> Result<Output> {
    let mut cmd = hidden_command(weidu_binary);
    if let Some(wd) = working_directory {
        cmd.current_dir(wd);
    }
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--no-exit-pause")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run weidu: {}", weidu_binary.display()))
}
