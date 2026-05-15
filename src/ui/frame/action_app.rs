// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Copy)]
pub struct AppLaunchAction {
    pub dev_mode: bool,
}

#[must_use]
pub const fn launch_gui(dev_mode: bool) -> AppLaunchAction {
    AppLaunchAction { dev_mode }
}
