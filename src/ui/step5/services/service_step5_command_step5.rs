// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "service_step5_command_display.rs"]
mod display;
#[path = "service_step5_command_text.rs"]
mod text;

pub(crate) use display::{build_command_preview_lines, wrap_display_line};
pub(crate) use text::build_install_command;
