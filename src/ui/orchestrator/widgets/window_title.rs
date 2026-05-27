// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_primary};

/// Returns styled `WidgetText` suitable for use as an `egui::Window` title.
///
/// Renders the title in Poppins Medium at 18 px with the theme's primary text
/// color. Pass the result directly to `Window::new(...)`.
#[must_use]
pub fn redesign_window_title(palette: ThemePalette, title: &str) -> egui::WidgetText {
    egui::RichText::new(title)
        .size(18.0)
        .strong()
        .family(egui::FontFamily::Name("poppins_medium".into()))
        .color(redesign_text_primary(palette))
        .into()
}
