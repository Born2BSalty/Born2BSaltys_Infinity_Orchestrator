// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "service_step5_command_common_args.rs"]
mod common_args;
#[path = "service_step5_command_display.rs"]
mod display;
#[path = "service_step5_command_install.rs"]
mod install;
#[path = "service_step5_command_log_paths.rs"]
mod log_paths;
#[path = "service_step5_command_resume.rs"]
mod resume;
#[path = "service_step5_command_text.rs"]
mod text;

pub(crate) use display::{build_command_preview_lines, wrap_display_line};
pub(crate) use install::build_install_invocation;
pub(crate) use log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};
pub(crate) use resume::{build_resume_invocation, capture_resume_targets};
pub(crate) use text::build_install_command;
