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
            egui::CornerRadius::same(crate::ui::shared::layout_tokens_global::RADIUS_SM as u8);
        visuals.widgets.active.corner_radius =
            egui::CornerRadius::same(crate::ui::shared::layout_tokens_global::RADIUS_SM as u8);
        visuals.widgets.hovered.corner_radius =
            egui::CornerRadius::same(crate::ui::shared::layout_tokens_global::RADIUS_SM as u8);
        ctx.set_visuals(visuals);
    }
}

// Text + surfaces
pub fn text_primary() -> egui::Color32 {
    egui::Color32::from_gray(224)
}

pub fn text_muted() -> egui::Color32 {
    egui::Color32::from_gray(190)
}

pub fn text_disabled() -> egui::Color32 {
    egui::Color32::from_gray(130)
}

pub fn bg_panel() -> egui::Color32 {
    egui::Color32::from_rgb(30, 32, 36)
}

pub fn bg_extreme() -> egui::Color32 {
    egui::Color32::from_rgb(20, 22, 26)
}

// Generic accents / semantic state
pub fn accent_path() -> egui::Color32 {
    egui::Color32::from_rgb(224, 196, 156)
}

pub fn accent_numbers() -> egui::Color32 {
    egui::Color32::from_rgb(136, 176, 255)
}

pub fn accent_comment() -> egui::Color32 {
    egui::Color32::from_rgb(124, 196, 124)
}

pub fn success() -> egui::Color32 {
    egui::Color32::from_rgb(124, 196, 124)
}

pub fn success_bright() -> egui::Color32 {
    egui::Color32::from_rgb(120, 200, 120)
}

pub fn warning() -> egui::Color32 {
    egui::Color32::from_rgb(214, 168, 96)
}

pub fn warning_soft() -> egui::Color32 {
    egui::Color32::from_rgb(220, 180, 100)
}

pub fn warning_emphasis() -> egui::Color32 {
    egui::Color32::from_rgb(214, 174, 84)
}

pub fn error() -> egui::Color32 {
    egui::Color32::from_rgb(220, 100, 100)
}

pub fn error_emphasis() -> egui::Color32 {
    egui::Color32::from_rgb(220, 96, 96)
}

// Prompt pills
pub fn prompt_text() -> egui::Color32 {
    egui::Color32::from_rgb(40, 20, 0)
}

pub fn prompt_fill() -> egui::Color32 {
    egui::Color32::from_rgb(245, 195, 95)
}

pub fn prompt_stroke() -> egui::Color32 {
    egui::Color32::from_rgb(210, 160, 70)
}

// Compat badges
pub fn conflict_fill() -> egui::Color32 {
    egui::Color32::from_rgb(88, 44, 44)
}

pub fn info() -> egui::Color32 {
    egui::Color32::from_rgb(120, 186, 230)
}

pub fn info_fill() -> egui::Color32 {
    egui::Color32::from_rgb(42, 66, 86)
}

pub fn game_mismatch() -> egui::Color32 {
    egui::Color32::from_rgb(203, 110, 188)
}

pub fn game_mismatch_fill() -> egui::Color32 {
    egui::Color32::from_rgb(78, 44, 84)
}

pub fn conditional() -> egui::Color32 {
    egui::Color32::from_rgb(164, 190, 208)
}

pub fn conditional_fill() -> egui::Color32 {
    egui::Color32::from_rgb(52, 66, 78)
}

pub fn warning_fill() -> egui::Color32 {
    egui::Color32::from_rgb(78, 62, 34)
}

pub fn included() -> egui::Color32 {
    egui::Color32::from_gray(150)
}

pub fn included_fill() -> egui::Color32 {
    egui::Color32::from_rgb(56, 72, 56)
}

pub fn conflict() -> egui::Color32 {
    egui::Color32::from_rgb(208, 96, 96)
}

pub fn conflict_parent() -> egui::Color32 {
    egui::Color32::from_rgb(220, 122, 122)
}

pub fn warning_parent() -> egui::Color32 {
    egui::Color32::from_rgb(222, 182, 92)
}

// Step 5 phase/state
pub fn status_running() -> egui::Color32 {
    egui::Color32::from_rgb(168, 204, 98)
}

pub fn status_preparing() -> egui::Color32 {
    egui::Color32::from_rgb(180, 170, 220)
}

pub fn status_idle() -> egui::Color32 {
    egui::Color32::from_gray(170)
}

// Disabled nav buttons
pub fn nav_disabled_text() -> egui::Color32 {
    egui::Color32::from_gray(120)
}

pub fn nav_disabled_fill() -> egui::Color32 {
    egui::Color32::from_gray(45)
}

pub fn nav_disabled_stroke() -> egui::Color32 {
    egui::Color32::from_gray(70)
}

// Terminal/log coloring
pub fn terminal_default() -> egui::Color32 {
    egui::Color32::from_rgb(210, 210, 210)
}

pub fn terminal_error() -> egui::Color32 {
    egui::Color32::from_rgb(230, 96, 96)
}

pub fn terminal_debug() -> egui::Color32 {
    egui::Color32::from_rgb(70, 110, 180)
}

pub fn terminal_sent() -> egui::Color32 {
    egui::Color32::from_rgb(110, 190, 255)
}

pub fn terminal_info() -> egui::Color32 {
    egui::Color32::from_rgb(168, 204, 98)
}

pub fn terminal_amber() -> egui::Color32 {
    egui::Color32::from_rgb(214, 168, 96)
}

pub fn terminal_sand() -> egui::Color32 {
    egui::Color32::from_rgb(214, 182, 146)
}

pub fn terminal_dim() -> egui::Color32 {
    egui::Color32::from_rgb(150, 150, 150)
}
