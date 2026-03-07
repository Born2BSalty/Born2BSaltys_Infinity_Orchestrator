// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step3::compat_modal_issue_text_step3::*;
use crate::ui::step3::compat_modal_step3::compat_model::CompatJumpAction;
use crate::ui::step3::service_step3::export_step3_compat_report;

pub(crate) fn render(ui: &mut egui::Ui, state: &mut WizardState) -> Option<CompatJumpAction> {
    let mut jump_request: Option<CompatJumpAction> = None;
    let mut open = state.step3.compat_modal_open;
    egui::Window::new("Compatibility Issues")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .movable(true)
        .default_size(egui::vec2(760.0, 480.0))
        .min_width(260.0)
        .min_height(140.0)
        .show(ui.ctx(), |ui| {
            render_header(ui, state);
            ui.add_space(6.0);
            render_filter_row(ui, state);
            ui.add_space(6.0);
            render_issue_list(ui, state, &mut jump_request);
            ui.add_space(6.0);
            render_footer(ui, state);
        });
    state.step3.compat_modal_open = open && state.step3.compat_modal_open;
    jump_request
}

fn render_header(ui: &mut egui::Ui, state: &WizardState) {
    ui.horizontal(|ui| {
        ui.label(
            crate::ui::shared::typography_global::strong(format!("{} errors", state.compat.error_count))
                .color(crate::ui::shared::theme_global::error()),
        );
        ui.label(
            crate::ui::shared::typography_global::strong(format!("{} warnings", state.compat.warning_count))
                .color(crate::ui::shared::theme_global::warning_soft()),
        );
    });
}

fn render_filter_row(ui: &mut egui::Ui, state: &mut WizardState) {
    ui.horizontal_wrapped(|ui| {
        ui.label(crate::ui::shared::typography_global::strong("Show"));
        for (id, label) in [
            ("all", "All"),
            ("conflicts", "Conflicts"),
            ("dependencies", "Missing deps"),
            ("conditionals", "Conditionals"),
            ("warnings", "Warnings"),
        ] {
            let is_selected = state.step3.compat_modal_filter.eq_ignore_ascii_case(id);
            if ui.selectable_label(is_selected, label).clicked() {
                state.step3.compat_modal_filter = id.to_string();
            }
        }
    });
}

fn render_issue_list(
    ui: &mut egui::Ui,
    state: &WizardState,
    jump_request: &mut Option<CompatJumpAction>,
) {
    let filter = state.step3.compat_modal_filter.clone();
    let filtered_issues: Vec<_> = state
        .compat
        .issues
        .iter()
        .filter(|i| matches_issue_filter(i, &filter))
        .collect();

    let issue_list_h = (ui.available_height() - 90.0).max(40.0);
    egui::ScrollArea::vertical()
        .max_height(issue_list_h)
        .show(ui, |ui| {
            if filtered_issues.is_empty() {
                ui.label("No issues match the selected filter.");
                return;
            }
            for issue in filtered_issues {
                let color = if issue.is_blocking {
                    crate::ui::shared::theme_global::error()
                } else {
                    crate::ui::shared::theme_global::warning_soft()
                };
                let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
                let related = format_issue_target(&issue.related_mod, issue.related_component);
                let graph = issue_graph(issue);
                egui::CollapsingHeader::new(
                    crate::ui::shared::typography_global::strong(format!(
                        "[{}] {}",
                        human_kind(&issue.code),
                        graph
                    ))
                    .color(color),
                )
                .id_salt(("step3_compat_modal_issue", issue.issue_id.as_str()))
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new(("step3_compat_modal_grid", issue.issue_id.as_str()))
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(crate::ui::shared::typography_global::strong("Severity"));
                            ui.label(human_severity(issue));
                            ui.end_row();
                            ui.label(crate::ui::shared::typography_global::strong("Kind"));
                            ui.label(human_kind(&issue.code));
                            ui.end_row();
                            if let Some(verdict) = issue_verdict(issue) {
                                ui.label(crate::ui::shared::typography_global::strong("Verdict"));
                                ui.label(verdict);
                                ui.end_row();
                            }
                            ui.label(crate::ui::shared::typography_global::strong("Affected"));
                            ui.label(&affected);
                            ui.end_row();
                            ui.label(crate::ui::shared::typography_global::strong("Related"));
                            ui.label(human_related(issue, &related));
                            ui.end_row();
                            ui.label(crate::ui::shared::typography_global::strong("Why this appears"));
                            ui.label(issue_why_this_appears(issue));
                            ui.end_row();
                            ui.label(crate::ui::shared::typography_global::strong("What to do"));
                            ui.label(issue_what_to_do(issue));
                            ui.end_row();
                            ui.label(crate::ui::shared::typography_global::strong("Reason"));
                            ui.label(issue_reason(issue));
                            ui.end_row();
                            ui.label(crate::ui::shared::typography_global::strong("Source"));
                            ui.label(&issue.source);
                            ui.end_row();
                            ui.label(crate::ui::shared::typography_global::strong("Graph"));
                            ui.monospace(graph);
                            ui.end_row();
                        });
                    if let Some(evidence) = issue.raw_evidence.as_deref() {
                        ui.add_space(4.0);
                        ui.label(crate::ui::shared::typography_global::strong("Evidence"));
                        ui.monospace(evidence);
                    }
                    ui.add_space(4.0);

                    let can_jump_affected = issue_target_exists(state, issue, true);
                    let can_jump_related = issue_target_exists(state, issue, false);
                    let can_jump_auto = can_jump_affected || can_jump_related;

                    ui.horizontal_wrapped(|ui| {
                        ui.label(crate::ui::shared::typography_global::strong("Targets"));
                        let affected_text = if can_jump_affected {
                            crate::ui::shared::typography_global::strong("Affected: found")
                                .color(crate::ui::shared::theme_global::success_bright())
                        } else {
                            crate::ui::shared::typography_global::strong("Affected: missing")
                                .color(crate::ui::shared::theme_global::warning_soft_alt())
                        };
                        let related_text = if can_jump_related {
                            crate::ui::shared::typography_global::strong("Related: found")
                                .color(crate::ui::shared::theme_global::success_bright())
                        } else {
                            crate::ui::shared::typography_global::strong("Related: missing")
                                .color(crate::ui::shared::theme_global::warning_soft_alt())
                        };
                        ui.label(affected_text);
                        ui.label(" | ");
                        ui.label(related_text);
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        let mut jump_affected =
                            ui.add_enabled(can_jump_affected, egui::Button::new("Jump to affected"));
                        if !can_jump_affected {
                            jump_affected = jump_affected
                                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_AFFECTED_MISSING);
                        }
                        if jump_affected.clicked() {
                            *jump_request = Some(CompatJumpAction::Affected(issue.issue_id.clone()));
                        }

                        let mut jump_related =
                            ui.add_enabled(can_jump_related, egui::Button::new("Jump to related"));
                        if !can_jump_related {
                            jump_related = jump_related
                                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_RELATED_MISSING);
                        }
                        if jump_related.clicked() {
                            *jump_request = Some(CompatJumpAction::Related(issue.issue_id.clone()));
                        }

                        let mut jump_auto =
                            ui.add_enabled(can_jump_auto, egui::Button::new("Jump (auto)"));
                        if !can_jump_auto {
                            jump_auto = jump_auto
                                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_NO_JUMP_TARGET);
                        }
                        if jump_auto.clicked() {
                            *jump_request = Some(CompatJumpAction::Auto(issue.issue_id.clone()));
                        }
                    });
                    if jump_request.is_some() {
                        return;
                    }
                });
                ui.add_space(4.0);
            }
        });
}

fn render_footer(ui: &mut egui::Ui, state: &mut WizardState) {
    ui.horizontal(|ui| {
        if ui.button("Export Compat Report").clicked() {
            match export_step3_compat_report(&state.compat.issues) {
                Ok(path) => {
                    state.step5.last_status_text =
                        format!("Step 3 compat report exported: {}", path.display());
                }
                Err(err) => {
                    state.step5.last_status_text = format!("Step 3 compat export failed: {err}");
                }
            }
        }
        if ui.button("Close").clicked() {
            state.step3.compat_modal_open = false;
        }
    });
}
