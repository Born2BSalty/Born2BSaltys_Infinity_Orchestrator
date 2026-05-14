// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) use crate::ui::step2::action_step2::Step2Action;

pub fn render_pane(
    ui: &mut eframe::egui::Ui,
    state: &mut crate::app::state::WizardState,
    action: &mut Option<Step2Action>,
    right_rect: eframe::egui::Rect,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
) {
    ui.scope_builder(eframe::egui::UiBuilder::new().max_rect(right_rect), |ui| {
        let details = crate::ui::step2::service_details_step2::selected_details(state);
        let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
        ui.group(|ui| {
            ui.set_min_size(right_rect.size() - eframe::egui::vec2(12.0, 12.0));
            ui.label(crate::ui::shared::typography_global::section_title(
                "Details",
            ));
            ui.add_space(4.0);
            if exact_log_mode {
                details_pane_content::render_exact_log_status(ui, state, palette);
            } else {
                details_pane_content::render(ui, &details, action, palette);
            }
        });
    });
}

pub(crate) mod details_pane_content {
    use eframe::egui;

    use crate::app::state::{WizardState, exact_log_ready_to_install};
    use crate::ui::shared::redesign_tokens::{
        ThemePalette, redesign_error, redesign_success, redesign_warning,
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

        egui::ScrollArea::vertical()
            .id_salt("step2_details_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (headline, color) = if ready {
                    (
                        "All required mods are available. You can continue to install.",
                        redesign_success(palette),
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
                            .color(redesign_warning(palette)),
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
        let label_w = 86.0;
        let action_w = 48.0;
        let value_w = (ui.available_width() - label_w - action_w - 24.0).max(120.0);
        let row_h = 20.0;
        let value_chars = ((value_w / 7.2).floor() as usize).max(12);

        ui.label(crate::ui::shared::typography_global::strong(mod_name));
        ui.horizontal(|ui| {
            ui.label(crate::ui::shared::typography_global::strong("Version:"));
            ui.label(details.component_version.as_deref().unwrap_or("Unknown"));
        });
        ui.add_space(4.0);

        render_selection_grid(
            ui,
            details,
            action,
            SelectionGridLayout::new(label_w, value_w, row_h, value_chars, palette),
        );
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);
        render_paths_grid(
            ui,
            details,
            action,
            PathsGridLayout::new(label_w, value_w, row_h, value_chars, palette),
        );
        ui.add_space(6.0);
        render_component_block(ui, details);
        render_raw_line(ui, details);
    }
}
