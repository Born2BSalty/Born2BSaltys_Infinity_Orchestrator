// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) use crate::ui::step2::action_step2::Step2Action;

const PANEL_PAD_LEFT: i8 = 8;
const PANEL_PAD_RIGHT: i8 = 13;
const PANEL_PAD_Y: i8 = 6;
const TITLE_ROW_H: f32 = 24.0;

pub fn render_pane(
    ui: &mut eframe::egui::Ui,
    state: &mut crate::app::state::WizardState,
    action: &mut Option<Step2Action>,
    right_rect: eframe::egui::Rect,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
    details_open: &mut bool,
) {
    let panel_rect = right_rect.shrink(1.0);
    ui.scope_builder(eframe::egui::UiBuilder::new().max_rect(panel_rect), |ui| {
        let details = crate::ui::step2::service_details_step2::selected_details(state);
        let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
        let frame = eframe::egui::Frame::default()
            .fill(crate::ui::shared::redesign_tokens::redesign_shell_bg(
                palette,
            ))
            .stroke(eframe::egui::Stroke::new(
                crate::ui::shared::redesign_tokens::REDESIGN_BORDER_WIDTH_PX,
                crate::ui::shared::redesign_tokens::redesign_border_strong(palette),
            ))
            .corner_radius(eframe::egui::CornerRadius::same(
                crate::ui::shared::redesign_tokens::REDESIGN_BORDER_RADIUS_U8,
            ))
            .inner_margin(eframe::egui::Margin {
                left: PANEL_PAD_LEFT,
                right: PANEL_PAD_RIGHT,
                top: PANEL_PAD_Y,
                bottom: PANEL_PAD_Y,
            });
        frame.show(ui, |ui| {
            let inner_size = panel_rect.size()
                - eframe::egui::vec2(
                    f32::from(PANEL_PAD_LEFT + PANEL_PAD_RIGHT),
                    f32::from(PANEL_PAD_Y * 2),
                );
            ui.set_min_size(inner_size);
            render_title_row(ui, palette, details_open);
            ui.add_space(4.0);
            if exact_log_mode {
                details_pane_content::render_exact_log_status(ui, state, palette);
            } else {
                details_pane_content::render(ui, &details, action, palette);
            }
        });
    });
}

fn render_title_row(
    ui: &mut eframe::egui::Ui,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
    details_open: &mut bool,
) {
    ui.allocate_ui_with_layout(
        eframe::egui::vec2(ui.available_width(), TITLE_ROW_H),
        eframe::egui::Layout::left_to_right(eframe::egui::Align::Center),
        |ui| {
            ui.label(crate::ui::shared::typography_global::section_title(
                "Details",
            ));
            let spare = (ui.available_width() - 24.0).max(0.0);
            ui.add_space(spare);
            if crate::ui::orchestrator::widgets::render_icon_button(
                ui,
                palette,
                crate::ui::orchestrator::widgets::ButtonIcon::Close,
                "Close details",
                true,
            )
            .clicked()
            {
                *details_open = false;
            }
        },
    );
}

pub mod details_pane_content {
    use eframe::egui;

    use crate::app::state::{WizardState, exact_log_ready_to_install};
    use crate::ui::shared::redesign_tokens::{
        ThemePalette, redesign_error, redesign_pill_warn, redesign_success_bright,
    };
    use crate::ui::shared::typography_global as typo;
    use crate::ui::step2::details_paths_step2::{
        PathsGridLayout, render_component_block, render_paths_grid, render_raw_line,
    };
    use crate::ui::step2::details_selection_step2::{SelectionGridLayout, render_selection_grid};
    use crate::ui::step2::state_step2::Step2Details;

    use super::Step2Action;

    pub(crate) fn render_exact_log_status(
        ui: &mut egui::Ui,
        state: &WizardState,
        palette: ThemePalette,
    ) {
        let ready = exact_log_ready_to_install(state);
        let downloadable_missing = state.step2.update_selected_missing_sources.len();
        let manual_sources = state.step2.update_selected_manual_sources.len();
        let no_source_entries = state.step2.update_selected_unknown_sources.len();
        let source_check_failed = state.step2.update_selected_failed_sources.len();
        let exact_version_pending = state
            .step2
            .update_selected_exact_version_retry_requests
            .len();

        configure_scroll_style(ui);
        egui::ScrollArea::vertical()
            .id_salt("step2_details_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (headline, color) = if ready {
                    (
                        "All required mods are available. You can continue to install.",
                        redesign_success_bright(palette),
                    )
                } else {
                    ("Install cannot continue yet.", redesign_error(palette))
                };
                ui.label(typo::strong("Exact-Log Install Status").color(color));
                ui.add_space(4.0);
                ui.label(typo::plain(headline).color(color));
                ui.add_space(8.0);
                if !state.step2.exact_log_mod_list_checked {
                    ui.label(
                        typo::plain("Run Check Mod List to verify required mods.")
                            .color(redesign_pill_warn(palette)),
                    );
                    ui.add_space(8.0);
                }
                ui.label(typo::strong("Required Mod Status"));
                ui.label(format!("Downloadable missing mods: {downloadable_missing}"));
                ui.label(format!("Manual sources: {manual_sources}"));
                ui.label(format!("No source entries: {no_source_entries}"));
                ui.label(format!("Source check failed: {source_check_failed}"));
                ui.label(format!(
                    "Exact version fallback pending: {exact_version_pending}"
                ));
            });
    }

    pub(crate) fn render(
        ui: &mut egui::Ui,
        details: &Step2Details,
        action: &mut Option<Step2Action>,
        palette: ThemePalette,
    ) {
        configure_scroll_style(ui);
        egui::ScrollArea::vertical()
            .id_salt("step2_details_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(mod_name) = &details.mod_name {
                    render_details_content(ui, mod_name, details, action, palette);
                } else {
                    ui.label("Select an item to view details.");
                }
            });
    }

    fn render_details_content(
        ui: &mut egui::Ui,
        mod_name: &str,
        details: &Step2Details,
        action: &mut Option<Step2Action>,
        palette: ThemePalette,
    ) {
        let label_w = 120.0;
        let action_w = 64.0;
        let value_w = (ui.available_width() - label_w - action_w - 16.0).max(120.0);
        let row_h = 20.0;
        let value_chars = floored_columns(value_w, 7.2, 12);

        ui.label(crate::ui::shared::typography_global::strong(mod_name));
        ui.horizontal(|ui| {
            ui.label(crate::ui::shared::typography_global::strong("Version:"));
            ui.label(details.component_version.as_deref().unwrap_or("Unknown"));
        });
        ui.add_space(4.0);

        let selection_layout = SelectionGridLayout {
            palette,
            label_w,
            value_w,
            action_w,
            row_h,
            value_chars,
        };
        render_selection_grid(ui, details, action, selection_layout);
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);
        let paths_layout = PathsGridLayout {
            palette,
            label_w,
            value_w,
            action_w,
            row_h,
            value_chars,
        };
        render_paths_grid(ui, details, action, paths_layout);
        ui.add_space(6.0);
        render_component_block(ui, details, palette);
        render_raw_line(ui, details, palette);
    }

    fn floored_columns(width: f32, column_width: f32, minimum: usize) -> usize {
        let estimate = (width / column_width).floor();
        if estimate.is_finite() {
            estimate
                .to_string()
                .parse::<usize>()
                .unwrap_or(minimum)
                .max(minimum)
        } else {
            minimum
        }
    }

    fn configure_scroll_style(ui: &mut egui::Ui) {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 12.0;
        scroll.bar_inner_margin = 0.0;
        scroll.bar_outer_margin = 2.0;
        ui.style_mut().spacing.scroll = scroll;
    }
}
