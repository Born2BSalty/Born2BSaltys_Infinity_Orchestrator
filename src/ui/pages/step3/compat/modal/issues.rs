// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{CompatIssueDisplay, Step3ItemState, WizardState};

use super::super::model::{CompatJumpAction, normalize_mod_key};
use super::filters;
use super::issue_text;
use super::target::format_issue_target;

pub(super) fn render_issue_list(
    ui: &mut egui::Ui,
    state: &WizardState,
    jump_request: &mut Option<CompatJumpAction>,
) {
    let filter = state.step3.compat_modal_filter.clone();
    let filtered_issues: Vec<_> = state
        .compat
        .issues
        .iter()
        .filter(|i| filters::matches_issue_filter(i, &filter))
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
                    egui::Color32::from_rgb(220, 100, 100)
                } else {
                    egui::Color32::from_rgb(220, 180, 100)
                };
                let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
                let related = format_issue_target(&issue.related_mod, issue.related_component);
                let graph = issue_text::issue_graph(issue);
                egui::CollapsingHeader::new(
                    egui::RichText::new(format!(
                        "[{}] {}",
                        issue_text::human_kind(&issue.code),
                        graph
                    ))
                    .color(color)
                    .strong(),
                )
                .id_salt(("step3_compat_modal_issue", issue.issue_id.as_str()))
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new(("step3_compat_modal_grid", issue.issue_id.as_str()))
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Severity").strong());
                            ui.label(issue_text::human_severity(issue));
                            ui.end_row();
                            ui.label(egui::RichText::new("Kind").strong());
                            ui.label(issue_text::human_kind(&issue.code));
                            ui.end_row();
                            if let Some(verdict) = issue_text::issue_verdict(issue) {
                                ui.label(egui::RichText::new("Verdict").strong());
                                ui.label(verdict);
                                ui.end_row();
                            }
                            ui.label(egui::RichText::new("Affected").strong());
                            ui.label(&affected);
                            ui.end_row();
                            ui.label(egui::RichText::new("Related").strong());
                            ui.label(issue_text::human_related(issue, &related));
                            ui.end_row();
                            ui.label(egui::RichText::new("Why this appears").strong());
                            ui.label(issue_text::issue_why_this_appears(issue));
                            ui.end_row();
                            ui.label(egui::RichText::new("What to do").strong());
                            ui.label(issue_text::issue_what_to_do(issue));
                            ui.end_row();
                            ui.label(egui::RichText::new("Reason").strong());
                            ui.label(issue_text::issue_reason(issue));
                            ui.end_row();
                            ui.label(egui::RichText::new("Source").strong());
                            ui.label(&issue.source);
                            ui.end_row();
                            ui.label(egui::RichText::new("Graph").strong());
                            ui.monospace(graph);
                            ui.end_row();
                        });
                    if let Some(evidence) = issue.raw_evidence.as_deref() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Evidence").strong());
                        ui.monospace(evidence);
                    }
                    ui.add_space(4.0);

                    let can_jump_affected = issue_target_exists(state, issue, true);
                    let can_jump_related = issue_target_exists(state, issue, false);
                    let can_jump_auto = can_jump_affected || can_jump_related;

                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Targets").strong());
                        let affected_text = if can_jump_affected {
                            egui::RichText::new("Affected: found")
                                .color(egui::Color32::from_rgb(120, 200, 120))
                        } else {
                            egui::RichText::new("Affected: missing")
                                .color(egui::Color32::from_rgb(220, 160, 100))
                        };
                        let related_text = if can_jump_related {
                            egui::RichText::new("Related: found")
                                .color(egui::Color32::from_rgb(120, 200, 120))
                        } else {
                            egui::RichText::new("Related: missing")
                                .color(egui::Color32::from_rgb(220, 160, 100))
                        };
                        ui.label(affected_text);
                        ui.label(" | ");
                        ui.label(related_text);
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        let mut jump_affected = ui.add_enabled(
                            can_jump_affected,
                            egui::Button::new("Jump to affected"),
                        );
                        if !can_jump_affected {
                            jump_affected = jump_affected.on_hover_text("Affected target not present in Step 3 list.");
                        }
                        if jump_affected.clicked() {
                            *jump_request = Some(CompatJumpAction::Affected(issue.issue_id.clone()));
                        }

                        let mut jump_related = ui.add_enabled(
                            can_jump_related,
                            egui::Button::new("Jump to related"),
                        );
                        if !can_jump_related {
                            jump_related = jump_related.on_hover_text("Related target not present in Step 3 list.");
                        }
                        if jump_related.clicked() {
                            *jump_request = Some(CompatJumpAction::Related(issue.issue_id.clone()));
                        }

                        let mut jump_auto = ui.add_enabled(
                            can_jump_auto,
                            egui::Button::new("Jump (auto)"),
                        );
                        if !can_jump_auto {
                            jump_auto = jump_auto.on_hover_text("No jump target from this issue is present in Step 3 list.");
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

fn issue_target_exists(state: &WizardState, issue: &CompatIssueDisplay, affected: bool) -> bool {
    let (mod_name, component) = if affected {
        (&issue.affected_mod, issue.affected_component)
    } else {
        (&issue.related_mod, issue.related_component)
    };
    item_target_exists(&state.step3.bgee_items, mod_name, component)
        || item_target_exists(&state.step3.bg2ee_items, mod_name, component)
}

fn item_target_exists(
    items: &[Step3ItemState],
    mod_name: &str,
    component: Option<u32>,
) -> bool {
    let target_key = normalize_mod_key(mod_name);
    for item in items {
        if item.is_parent {
            continue;
        }
        let item_tp_key = normalize_mod_key(&item.tp_file);
        let item_name_key = normalize_mod_key(&item.mod_name);
        if item_tp_key != target_key && item_name_key != target_key {
            continue;
        }
        if let Some(component_id) = component {
            if item.component_id.parse::<u32>().ok() == Some(component_id) {
                return true;
            }
        } else {
            return true;
        }
    }
    false
}
