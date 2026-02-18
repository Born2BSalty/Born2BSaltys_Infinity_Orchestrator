// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod display;
mod invocation;

pub use display::{build_command_preview_lines, wrap_display_line};
pub use invocation::{build_install_command, build_install_invocation, build_resume_invocation};
