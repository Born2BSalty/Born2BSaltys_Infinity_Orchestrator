// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::{NativeOptions, egui};

use crate::ui::layout::{WINDOW_HEIGHT, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WINDOW_WIDTH};

pub const APP_TITLE: &str = concat!(
    "Born2BSalty's Infinity Orchestrator (BIO) v",
    env!("CARGO_PKG_VERSION")
);

pub fn native_options() -> NativeOptions {
    NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT]),
        ..Default::default()
    }
}
