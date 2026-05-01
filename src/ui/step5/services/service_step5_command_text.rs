// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step1State;
use crate::app::step5::command_config::build_install_command_config;

use crate::install::step5_command_install::build_install_invocation;

pub(crate) fn build_install_command(step1: &Step1State) -> String {
    let install_config = build_install_command_config(step1);
    let (program, args) = build_install_invocation(&install_config);
    let quote = |value: &str| format!("\"{}\"", value);
    let mut parts: Vec<String> = vec![quote(&program)];
    parts.extend(
        args.into_iter()
            .map(|arg| if arg.contains(' ') { quote(&arg) } else { arg }),
    );
    parts.join(" ")
}
