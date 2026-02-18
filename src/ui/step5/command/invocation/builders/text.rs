// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

use super::install::build_install_invocation;

pub fn build_install_command(step1: &Step1State) -> String {
    let (program, args) = build_install_invocation(step1);
    let q = |v: &str| format!("\"{}\"", v);
    let mut parts: Vec<String> = vec![q(&program)];
    parts.extend(args.into_iter().map(|a| if a.contains(' ') { q(&a) } else { a }));
    parts.join(" ")
}
