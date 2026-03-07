// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_copy;
pub(crate) use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_explain;
pub(crate) use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_kind;

pub(crate) mod compat_popup_action_row {
    use eframe::egui;
    use crate::ui::state::WizardState;
        use crate::ui::controller::util::open_in_shell;
    
    use crate::ui::step2::compat_popup_step2::compat_popup_actions as actions;
    use crate::ui::step2::compat_popup_step2::compat_popup_issue_text_copy as issue_text_copy;
    use crate::ui::step2::compat_popup_step2::compat_popup_selection_query as selection_query;
    use crate::ui::step2::compat_popup_step2::compat_popup_selection_source as selection_source;

    pub(crate) fn render_action_row(ui: &mut egui::Ui, state: &mut WizardState) {
        ui.horizontal(|ui| {
            if ui.button("Jump To Other").clicked() {
                actions::jump_to_related_component(state);
                state.step2.jump_to_selected_requested = true;
            }
            if ui.button("Jump To This").clicked() {
                actions::jump_to_affected_component(state);
                state.step2.jump_to_selected_requested = true;
            }
            if ui.button("Select Other").clicked() {
                actions::jump_to_related_component(state);
            }
            if ui.button("Next Issue").clicked() {
                actions::jump_to_next_conflict(state);
                state.step2.jump_to_selected_requested = true;
            }
            if ui.button("Copy Issue").clicked()
                && let Some(issue) = selection_query::current_issue_for_selection(state)
            {
                ui.ctx().copy_text(issue_text_copy::format_issue_for_copy(&issue));
            }
            let source_path = selection_source::rule_source_open_path(state);
            let open_source_resp =
                ui.add_enabled(source_path.is_some(), egui::Button::new("Open Rule Source"));
            if let Some(path) = source_path {
                if open_source_resp.clicked() && let Err(err) = open_in_shell(&path) {
                    state.step2.scan_status = format!("Open failed: {err}");
                }
            }
            if ui.button("Close").clicked() {
                state.step2.compat_popup_open = false;
            }
        });
    }
}

pub(crate) mod compat_popup_actions {
    use crate::ui::state::WizardState;
    
    use crate::ui::step2::compat_popup_step2::compat_popup_filters as filters;
    use crate::ui::step2::compat_popup_step2::compat_popup_selection_jump as selection_jump;
    use crate::ui::step2::compat_popup_step2::compat_popup_selection_query as selection_query;

    pub(super) fn jump_to_related_component(state: &mut WizardState) {
        let Some((related_mod, related_component, _, _)) =
            selection_query::issue_targets_for_current_selection(state)
        else {
            return;
        };
        let Some(game_tab) = selection_query::current_game_tab(state) else {
            return;
        };
        selection_jump::jump_to_target(state, &game_tab, &related_mod, related_component);
    }

    pub(super) fn jump_to_affected_component(state: &mut WizardState) {
        let Some((_, _, affected_mod, affected_component)) =
            selection_query::issue_targets_for_current_selection(state)
        else {
            return;
        };
        let Some(game_tab) = selection_query::current_game_tab(state) else {
            return;
        };
        selection_jump::jump_to_target(state, &game_tab, &affected_mod, affected_component);
    }

    pub(super) fn jump_to_next_conflict(state: &mut WizardState) {
        let Some(game_tab) = selection_query::current_game_tab(state) else {
            return;
        };
        let filter = state.step2.compat_popup_filter.clone();
        let issue_list: Vec<(String, String, Option<u32>)> = state
            .compat
            .issues
            .iter()
            .filter(|i| filters::matches_issue_filter(i, &filter))
            .map(|i| (i.issue_id.clone(), i.affected_mod.clone(), i.affected_component))
            .collect();
        if issue_list.is_empty() {
            return;
        }
        let current_issue_id = selection_query::current_issue_id_for_selection(state);
        let start = current_issue_id
            .as_ref()
            .and_then(|id| issue_list.iter().position(|i| &i.0 == id))
            .map(|idx| (idx + 1) % issue_list.len())
            .unwrap_or(0);
        let issue = &issue_list[start];
        selection_jump::jump_to_target(state, &game_tab, &issue.1, issue.2);
    }
}

pub(crate) mod compat_popup_details {
    use eframe::egui;
    use crate::ui::state::{CompatIssueDisplay, WizardState};
    use crate::ui::step2::content_step2::step2_details_select::selected_details;
    use crate::ui::step2::state_step2::Step2Details;

    use crate::ui::step2::compat_popup_step2::compat_popup_issue_text_explain as issue_text_explain;
    use crate::ui::step2::compat_popup_step2::compat_popup_issue_text_kind as issue_text_kind;
    use crate::ui::step2::compat_popup_step2::compat_popup_selection_query as selection_query;

    pub(crate) fn render_details(ui: &mut egui::Ui, state: &WizardState) {
        let details = selected_details(state);
        let title = details
            .component_label
            .as_deref()
            .or(details.mod_name.as_deref())
            .unwrap_or("No component selected");
        ui.label(crate::ui::shared::typography_global::strong(title));
        ui.add_space(6.0);

        if details.compat_kind.is_none() && details.disabled_reason.is_none() {
            ui.label("No compatibility issue data for this item.");
            return;
        }

        let issue = selection_query::current_issue_for_selection(state)
            .or_else(|| synth_issue_from_details(&details));
        let kind = details.compat_kind.as_deref().unwrap_or("unknown");
        ui.horizontal(|ui| {
            ui.label(crate::ui::shared::typography_global::strong("Kind"));
            ui.label(issue_text_kind::human_kind(kind));
            if let Some(issue) = issue.as_ref() {
                let (badge_text, badge_color) = if issue.is_blocking {
                    (
                        "Blocks install",
                        crate::ui::shared::theme_global::error_emphasis(),
                    )
                } else {
                    ("Warning only", crate::ui::shared::theme_global::warning_soft())
                };
                ui.label(crate::ui::shared::typography_global::strong(badge_text).color(badge_color));
            }
        });

        if let Some(role) = details.compat_role.as_deref() {
            ui.horizontal(|ui| {
                ui.label(crate::ui::shared::typography_global::strong("Role"));
                ui.label(role);
            });
        }
        if let Some(issue) = issue.as_ref()
            && let Some(verdict) = issue_text_explain::issue_verdict(issue)
        {
            ui.horizontal(|ui| {
                ui.label(crate::ui::shared::typography_global::strong("Verdict"));
                ui.label(verdict);
            });
        }
        if let Some(issue) = issue.as_ref() {
            ui.add_space(2.0);
            ui.label(crate::ui::shared::typography_global::strong("Why this appears"));
            ui.label(issue_text_explain::issue_why_this_appears(issue));

            ui.add_space(2.0);
            ui.label(crate::ui::shared::typography_global::strong("What to do"));
            ui.label(issue_text_explain::issue_what_to_do(issue));
        }
        if let Some(reason) = details.disabled_reason.as_deref() {
            ui.add_space(2.0);
            ui.label(crate::ui::shared::typography_global::strong("Reason"));
            if let Some(issue) = issue.as_ref() {
                ui.label(issue_text_explain::issue_reason(issue, reason));
            } else {
                ui.label(reason);
            }
        }
        if let Some(source) = details.compat_source.as_deref() {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(crate::ui::shared::typography_global::strong("Source"));
                ui.monospace(source);
            });
        }
        if let Some(related) = details.compat_related_target.as_deref() {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(crate::ui::shared::typography_global::strong("Related target"));
                ui.label(related);
            });
        }
        if let Some(graph) = details.compat_graph.as_deref() {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(crate::ui::shared::typography_global::strong("Graph"));
                ui.monospace(graph);
            });
        }
        if let Some(evidence) = details.compat_evidence.as_deref() {
            ui.add_space(2.0);
            egui::CollapsingHeader::new("Rule detail")
                .default_open(false)
                .show(ui, |ui| {
                    ui.monospace(evidence);
                });
        }
    }

    fn synth_issue_from_details(
        details: &Step2Details,
    ) -> Option<CompatIssueDisplay> {
        let kind = details.compat_kind.as_deref()?.to_ascii_lowercase();
        let code = if kind == "game_mismatch" {
            "GAME_MISMATCH"
        } else if kind == "missing_dep" {
            "REQ_MISSING"
        } else if kind == "conflict" || kind == "not_compatible" {
            "FORBID_HIT"
        } else if kind == "conditional" {
            "CONDITIONAL"
        } else {
            "RULE_HIT"
        };
        let is_blocking = !matches!(code, "CONDITIONAL" | "ORDER_WARN");
        let related = details
            .compat_related_target
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        Some(CompatIssueDisplay {
            issue_id: "synthetic".to_string(),
            code: code.to_string(),
            is_blocking,
            affected_mod: details.tp_file.clone().unwrap_or_else(|| "unknown".to_string()),
            affected_component: details
                .component_id
                .as_deref()
                .and_then(|v| v.parse::<u32>().ok()),
            related_mod: related,
            related_component: None,
            reason: details.disabled_reason.clone().unwrap_or_default(),
            source: details.compat_source.clone().unwrap_or_default(),
            raw_evidence: details.compat_evidence.clone(),
        })
    }
}

pub(crate) mod compat_popup_filter_row {
    use eframe::egui;
    use crate::ui::state::WizardState;
        
    pub(crate) fn render_filter_row(ui: &mut egui::Ui, state: &mut WizardState) {
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::ui::shared::typography_global::strong("Show"));
            for (id, label) in [
                ("all", "All"),
                ("conflicts", "Conflicts"),
                ("dependencies", "Missing deps"),
                ("conditionals", "Conditionals"),
                ("warnings", "Warnings"),
            ] {
                let is_selected = state.step2.compat_popup_filter.eq_ignore_ascii_case(id);
                if ui.selectable_label(is_selected, label).clicked() {
                    state.step2.compat_popup_filter = id.to_string();
                }
            }
        });
    }
}

pub(crate) mod compat_popup_filters {
    use crate::ui::state::CompatIssueDisplay;

    pub(super) fn matches_issue_filter(issue: &CompatIssueDisplay, filter: &str) -> bool {
        match filter.to_ascii_lowercase().as_str() {
            "conflicts" => {
                issue.code.eq_ignore_ascii_case("FORBID_HIT")
                    || issue.code.eq_ignore_ascii_case("RULE_HIT")
                    || issue.reason.to_ascii_lowercase().contains("incompatible")
                    || issue.reason.to_ascii_lowercase().contains("conflict")
            }
            "dependencies" => issue.code.eq_ignore_ascii_case("REQ_MISSING"),
            "conditionals" => issue.code.eq_ignore_ascii_case("CONDITIONAL"),
            "warnings" => !issue.is_blocking || issue.code.eq_ignore_ascii_case("ORDER_WARN"),
            _ => true,
        }
    }
}

pub(crate) mod compat_popup_selection_jump {
    use crate::ui::state::WizardState;

    pub(crate) fn jump_to_target(
        state: &mut WizardState,
        game_tab: &str,
        mod_ref: &str,
        component_ref: Option<u32>,
    ) {
        crate::ui::step2::service_step2::jump_to_target(state, game_tab, mod_ref, component_ref);
    }
}

pub(crate) mod compat_popup_selection_query {
    use crate::ui::state::{CompatIssueDisplay, WizardState};

    pub(crate) fn current_game_tab(state: &WizardState) -> Option<String> {
        crate::ui::step2::service_step2::current_game_tab(state)
    }

    pub(crate) fn issue_targets_for_current_selection(
        state: &WizardState,
    ) -> Option<(String, Option<u32>, String, Option<u32>)> {
        crate::ui::step2::service_step2::issue_targets_for_current_selection(state)
    }

    pub(crate) fn current_issue_id_for_selection(state: &WizardState) -> Option<String> {
        crate::ui::step2::service_step2::current_issue_id_for_selection(state)
    }

    pub(crate) fn current_issue_for_selection(state: &WizardState) -> Option<CompatIssueDisplay> {
        crate::ui::step2::service_step2::current_issue_for_selection(state)
    }
}

pub(crate) mod compat_popup_selection_source {
    use crate::ui::state::WizardState;

    pub(crate) fn rule_source_open_path(state: &WizardState) -> Option<String> {
        crate::ui::step2::service_step2::rule_source_open_path(state)
    }
}
