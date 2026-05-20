// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod create_stub;
pub mod home_stub;
pub mod install_stub;
pub mod settings_stub;
pub mod workspace_stub;

pub use create_stub::render_create_stub;
pub use home_stub::render_home_stub;
pub use install_stub::render_install_stub;
pub use settings_stub::render_settings_stub;
pub use workspace_stub::render_workspace_stub;
