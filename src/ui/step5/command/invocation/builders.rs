// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod install;
mod resume;
mod text;

pub use install::build_install_invocation;
pub use resume::build_resume_invocation;
pub use text::build_install_command;
