// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

use super::install::build_install_invocation;

pub(crate) fn build_install_command(step1: &Step1State) -> String {
    let (program, args) = build_install_invocation(step1);
    let quote = |value: &str| format!("\"{}\"", value);
    let mut parts: Vec<String> = vec![quote(&program)];
    parts.extend(args.into_iter().map(|arg| {
        if arg.contains(' ') {
            quote(&arg)
        } else {
            arg
        }
    }));
    parts.join(" ")
}
