// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

// Shared UI theme tokens.
// Keep color definitions here; other modules should reference these accessors.

pub fn apply_runtime_theme(ctx: &egui::Context) {
    if !cfg!(debug_assertions) {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(text_primary());
        visuals.panel_fill = bg_panel();
        visuals.extreme_bg_color = bg_extreme();
        visuals.widgets.noninteractive.bg_fill = bg_panel();
        visuals.widgets.inactive.corner_radius =
            egui::CornerRadius::same(crate::ui::shared::layout_tokens_global::radius_u8(
                crate::ui::shared::layout_tokens_global::RADIUS_SM,
            ));
        visuals.widgets.active.corner_radius =
            egui::CornerRadius::same(crate::ui::shared::layout_tokens_global::radius_u8(
                crate::ui::shared::layout_tokens_global::RADIUS_SM,
            ));
        visuals.widgets.hovered.corner_radius =
            egui::CornerRadius::same(crate::ui::shared::layout_tokens_global::radius_u8(
                crate::ui::shared::layout_tokens_global::RADIUS_SM,
            ));
        ctx.set_visuals(visuals);
    }
}

// Text + surfaces
#[must_use]
pub const fn text_primary() -> egui::Color32 {
    egui::Color32::from_gray(224)
}

#[must_use]
pub const fn text_muted() -> egui::Color32 {
    egui::Color32::from_gray(190)
}

#[must_use]
pub const fn text_disabled() -> egui::Color32 {
    egui::Color32::from_gray(130)
}

#[must_use]
pub const fn bg_panel() -> egui::Color32 {
    egui::Color32::from_rgb(30, 32, 36)
}

#[must_use]
pub const fn bg_extreme() -> egui::Color32 {
    egui::Color32::from_rgb(20, 22, 26)
}

// Generic accents / semantic state
#[must_use]
pub const fn accent_path() -> egui::Color32 {
    egui::Color32::from_rgb(224, 196, 156)
}

#[must_use]
pub const fn accent_numbers() -> egui::Color32 {
    egui::Color32::from_rgb(136, 176, 255)
}

#[must_use]
pub const fn accent_comment() -> egui::Color32 {
    egui::Color32::from_rgb(124, 196, 124)
}

#[must_use]
pub const fn success() -> egui::Color32 {
    egui::Color32::from_rgb(124, 196, 124)
}

#[must_use]
pub const fn success_bright() -> egui::Color32 {
    egui::Color32::from_rgb(120, 200, 120)
}

#[must_use]
pub const fn warning() -> egui::Color32 {
    egui::Color32::from_rgb(214, 168, 96)
}

#[must_use]
pub const fn warning_soft() -> egui::Color32 {
    egui::Color32::from_rgb(220, 180, 100)
}

#[must_use]
pub const fn warning_emphasis() -> egui::Color32 {
    egui::Color32::from_rgb(214, 174, 84)
}

#[must_use]
pub const fn error() -> egui::Color32 {
    egui::Color32::from_rgb(220, 100, 100)
}

#[must_use]
pub const fn error_emphasis() -> egui::Color32 {
    egui::Color32::from_rgb(220, 96, 96)
}

// Prompt pills
#[must_use]
pub const fn prompt_text() -> egui::Color32 {
    egui::Color32::from_rgb(40, 20, 0)
}

#[must_use]
pub const fn prompt_fill() -> egui::Color32 {
    egui::Color32::from_rgb(245, 195, 95)
}

#[must_use]
pub const fn prompt_stroke() -> egui::Color32 {
    egui::Color32::from_rgb(210, 160, 70)
}

// Compat badges
#[must_use]
pub const fn conflict_fill() -> egui::Color32 {
    egui::Color32::from_rgb(88, 44, 44)
}

#[must_use]
pub const fn info() -> egui::Color32 {
    egui::Color32::from_rgb(120, 186, 230)
}

#[must_use]
pub const fn info_fill() -> egui::Color32 {
    egui::Color32::from_rgb(42, 66, 86)
}

#[must_use]
pub const fn game_mismatch() -> egui::Color32 {
    egui::Color32::from_rgb(203, 110, 188)
}

#[must_use]
pub const fn game_mismatch_fill() -> egui::Color32 {
    egui::Color32::from_rgb(78, 44, 84)
}

#[must_use]
pub const fn conditional() -> egui::Color32 {
    egui::Color32::from_rgb(164, 190, 208)
}

#[must_use]
pub const fn conditional_fill() -> egui::Color32 {
    egui::Color32::from_rgb(52, 66, 78)
}

#[must_use]
pub const fn warning_fill() -> egui::Color32 {
    egui::Color32::from_rgb(78, 62, 34)
}

#[must_use]
pub const fn included() -> egui::Color32 {
    egui::Color32::from_gray(150)
}

#[must_use]
pub const fn included_fill() -> egui::Color32 {
    egui::Color32::from_rgb(56, 72, 56)
}

#[must_use]
pub const fn conflict() -> egui::Color32 {
    egui::Color32::from_rgb(208, 96, 96)
}

#[must_use]
pub const fn conflict_parent() -> egui::Color32 {
    egui::Color32::from_rgb(220, 122, 122)
}

#[must_use]
pub const fn warning_parent() -> egui::Color32 {
    egui::Color32::from_rgb(222, 182, 92)
}

// Step 5 phase/state
#[must_use]
pub const fn status_running() -> egui::Color32 {
    egui::Color32::from_rgb(168, 204, 98)
}

#[must_use]
pub const fn status_preparing() -> egui::Color32 {
    egui::Color32::from_rgb(180, 170, 220)
}

#[must_use]
pub const fn status_idle() -> egui::Color32 {
    egui::Color32::from_gray(170)
}

// Disabled nav buttons
#[must_use]
pub const fn nav_disabled_text() -> egui::Color32 {
    egui::Color32::from_gray(120)
}

#[must_use]
pub const fn nav_disabled_fill() -> egui::Color32 {
    egui::Color32::from_gray(45)
}

#[must_use]
pub const fn nav_disabled_stroke() -> egui::Color32 {
    egui::Color32::from_gray(70)
}

// Terminal/log coloring
#[must_use]
pub const fn terminal_default() -> egui::Color32 {
    egui::Color32::from_rgb(210, 210, 210)
}

#[must_use]
pub const fn terminal_error() -> egui::Color32 {
    egui::Color32::from_rgb(230, 96, 96)
}

#[must_use]
pub const fn terminal_debug() -> egui::Color32 {
    egui::Color32::from_rgb(70, 110, 180)
}

#[must_use]
pub const fn terminal_sent() -> egui::Color32 {
    egui::Color32::from_rgb(110, 190, 255)
}

#[must_use]
pub const fn terminal_info() -> egui::Color32 {
    egui::Color32::from_rgb(168, 204, 98)
}

#[must_use]
pub const fn terminal_amber() -> egui::Color32 {
    egui::Color32::from_rgb(214, 168, 96)
}

#[must_use]
pub const fn terminal_sand() -> egui::Color32 {
    egui::Color32::from_rgb(214, 182, 146)
}

#[must_use]
pub const fn terminal_dim() -> egui::Color32 {
    egui::Color32::from_rgb(150, 150, 150)
}
