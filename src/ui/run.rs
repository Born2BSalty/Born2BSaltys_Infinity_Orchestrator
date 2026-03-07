// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use anyhow::{Result, anyhow};

use crate::ui::frame::action_app::launch_gui;
use crate::ui::frame::frame_window::{APP_TITLE, native_options};
use crate::ui::frame::state_app::build_app_state;
use crate::ui::frame::update_app::configure_startup_visuals;
use crate::ui::shared::typography_global::configure_typography;

pub fn run(dev_mode: bool) -> Result<()> {
    let options = native_options();
    let launch_action = launch_gui(dev_mode);

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| {
            configure_typography(&cc.egui_ctx);
            configure_startup_visuals(&cc.egui_ctx);
            Ok(Box::new(build_app_state(launch_action)))
        }),
    )
    .map_err(|err| anyhow!("failed to launch GUI: {err}"))?;

    Ok(())
}
