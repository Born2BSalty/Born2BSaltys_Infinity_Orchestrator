// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use chrono::Local;

use crate::ui::state::Step5State;

pub fn begin_new_run(step5: &mut Step5State) -> String {
    let run_id = make_run_id();
    step5.diagnostics_run_id = Some(run_id.clone());
    run_id
}

pub fn current_or_new_run_id(step5: &Step5State) -> String {
    step5
        .diagnostics_run_id
        .clone()
        .unwrap_or_else(make_run_id)
}

pub fn run_dir_from_id(run_id: &str) -> PathBuf {
    PathBuf::from("diagnostics").join(format!("run_{run_id}"))
}

fn make_run_id() -> String {
    Local::now().format("%Y-%m-%d_%H-%M-%S_%3f").to_string()
}
