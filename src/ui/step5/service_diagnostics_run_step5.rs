// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;

use crate::ui::state::Step5State;

pub fn begin_new_run(step5: &mut Step5State) -> String {
    prune_old_diagnostics(None);
    let run_id = make_run_id();
    step5.diagnostics_run_id = Some(run_id.clone());
    run_id
}

pub fn current_or_new_run_id(step5: &Step5State) -> String {
    step5.diagnostics_run_id.clone().unwrap_or_else(make_run_id)
}

pub fn run_dir_from_id(run_id: &str) -> PathBuf {
    PathBuf::from("diagnostics").join(format!("run_{run_id}"))
}

pub fn prune_old_diagnostics(keep_run_id: Option<&str>) {
    let diagnostics_dir = Path::new("diagnostics");
    let entries = match fs::read_dir(diagnostics_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() && name.starts_with("run_") {
            let keep_name = keep_run_id.map(|id| format!("run_{id}"));
            if keep_name.as_deref() == Some(name.as_ref()) {
                continue;
            }
            let _ = fs::remove_dir_all(&path);
        }
    }
}

fn make_run_id() -> String {
    Local::now().format("%Y-%m-%d_%H-%M-%S_%3f").to_string()
}
