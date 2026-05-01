// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_explain;
pub(crate) use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_kind;

pub(crate) mod compat_popup_action_row {
    use eframe::egui;

    use crate::app::compat_popup_nav;
    use crate::app::controller::util::open_in_shell;
    use crate::app::state::WizardState;
    use crate::ui::step2::compat_popup_nav_step2::{next_target, select_popup_target};
    use crate::ui::step2::compat_popup_step2::compat_popup_details as details;
    use crate::ui::step2::service_selection_step2::rule_source_open_path;

    pub(crate) fn render_action_row(ui: &mut egui::Ui, state: &mut WizardState) {
        let issue = details::selected_or_synth_issue(state);
        let can_jump_this = compat_popup_nav::selected_game_tab(state).is_some();
        let can_jump_related = issue
            .as_ref()
            .is_some_and(compat_popup_nav::can_jump_to_related);
        let can_next = next_target(state).is_some();

        ui.horizontal(|ui| {
            if ui
                .add_enabled(can_jump_this, egui::Button::new("Jump To This"))
                .clicked()
            {
                compat_popup_nav::jump_to_this(state);
            }
            if ui
                .add_enabled(can_jump_related, egui::Button::new("Jump To Related"))
                .clicked()
                && let Some(issue) = issue.as_ref()
            {
                compat_popup_nav::jump_to_related(state, issue);
            }
            if ui
                .add_enabled(can_next, egui::Button::new("Next"))
                .clicked()
            {
                jump_to_next(state);
            }
            let source_path = rule_source_open_path(state);
            let open_source_resp =
                ui.add_enabled(source_path.is_some(), egui::Button::new("Open Rule Source"));
            if let Some(path) = source_path
                && open_source_resp.clicked()
                && let Err(err) = open_in_shell(&path)
            {
                state.step2.scan_status = format!("Open failed: {err}");
            }
            if ui.button("Close").clicked() {
                state.step2.compat_popup_open = false;
            }
        });
    }

    fn jump_to_next(state: &mut WizardState) {
        let Some(target) = next_target(state) else {
            return;
        };
        select_popup_target(state, &target);
    }
}

pub(crate) mod compat_popup_details {
    use eframe::egui;

    use crate::app::compat_issue::CompatIssue;
    use crate::app::compat_issue_text::display_source;
    use crate::app::selected_details::selected_compat_issue;
    use crate::app::state::WizardState;
    use crate::ui::step2::compat_popup_nav_step2::{
        COMPAT_POPUP_FILTER_OPTIONS, compat_filter_matches,
    };
    use crate::ui::step2::compat_popup_step2::compat_popup_issue_text_explain as issue_text_explain;
    use crate::ui::step2::compat_popup_step2::compat_popup_issue_text_kind as issue_text_kind;
    use crate::ui::step2::compat_types_step2::{CompatIssueStatusTone, display_issue};
    use crate::ui::step2::content_step2::step2_details_select::selected_details;

    pub(crate) fn render_details(ui: &mut egui::Ui, state: &mut WizardState) {
        let details = selected_details(state);
        let issue = selected_or_synth_issue(state);
        let issue_display = issue.as_ref().map(display_issue);
        let base_title = details
            .component_label
            .as_deref()
            .or(details.mod_name.as_deref())
            .unwrap_or("No component selected");
        let title = match details.component_id.as_deref().map(str::trim) {
            Some(id) if !id.is_empty() && !base_title.trim_start().starts_with('#') => {
                format!("#{id} {base_title}")
            }
            _ => base_title.to_string(),
        };
        ui.label(crate::ui::shared::typography_global::strong(title));
        ui.add_space(6.0);

        if issue.is_none() && details.compat_kind.is_none() && details.disabled_reason.is_none() {
            ui.label("No compatibility issue data for this item.");
            return;
        }

        let kind = details
            .compat_kind
            .as_deref()
            .or(issue_display.as_ref().map(|issue| issue.kind.as_str()))
            .unwrap_or("unknown");
        if let Some(issue) = issue_display.as_ref()
            && !kind.eq_ignore_ascii_case("included")
        {
            ui.horizontal(|ui| {
                ui.label(crate::ui::shared::typography_global::strong("Status"));
                let badge_color = match issue.status_tone {
                    CompatIssueStatusTone::Neutral => crate::ui::shared::theme_global::text_muted(),
                    CompatIssueStatusTone::Blocking => {
                        crate::ui::shared::theme_global::error_emphasis()
                    }
                    CompatIssueStatusTone::Warning => {
                        crate::ui::shared::theme_global::warning_soft()
                    }
                };
                ui.label(
                    crate::ui::shared::typography_global::strong(issue.status_label.as_str())
                        .color(badge_color),
                );
            });
        }
        ui.horizontal(|ui| {
            ui.label(crate::ui::shared::typography_global::strong("Kind"));
            ui.label(issue_text_kind::human_kind(kind));
        });

        if let Some(issue) = issue_display.as_ref() {
            ui.add_space(4.0);
            let summary = issue_text_explain::issue_summary(issue, &state.step1.game_install);
            ui.label(summary.clone());
            if let Some(reason) = details.disabled_reason.as_deref() {
                let trimmed = reason.trim();
                if !trimmed.is_empty()
                    && !trimmed.eq_ignore_ascii_case("unknown")
                    && !trimmed.eq_ignore_ascii_case(summary.trim())
                {
                    ui.add_space(2.0);
                    ui.label(trimmed);
                }
            }
        } else if let Some(reason) = details.disabled_reason.as_deref() {
            ui.add_space(4.0);
            ui.label(reason);
        }
        let source = details
            .compat_source
            .as_deref()
            .or(issue.as_ref().map(|issue| issue.source.as_str()));
        if let Some(source) = source {
            ui.add_space(6.0);
            ui.label(crate::ui::shared::typography_global::strong("Rule source"));
            ui.monospace(display_source(source));
        }
        if let Some(block) = details.compat_component_block.as_deref() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(crate::ui::shared::typography_global::strong(
                "Component block",
            ))
            .default_open(false)
            .show(ui, |ui| {
                ui.monospace(block);
            });
        }

        if issue.is_some() || details.compat_kind.is_some() {
            ui.add_space(6.0);
            let current_kind = issue
                .as_ref()
                .map(|issue| issue.kind.as_str())
                .or(details.compat_kind.as_deref());
            render_filter_row(ui, state, current_kind);
        }
    }

    pub(crate) fn selected_or_synth_issue(state: &WizardState) -> Option<CompatIssue> {
        selected_compat_issue(state)
    }

    fn render_filter_row(ui: &mut egui::Ui, state: &mut WizardState, current_kind: Option<&str>) {
        ui.label(crate::ui::shared::typography_global::strong("Filter"));
        ui.horizontal_wrapped(|ui| {
            for option in COMPAT_POPUP_FILTER_OPTIONS {
                let is_selected = state.step2.compat_popup_filter.eq_ignore_ascii_case(option);
                let visuals = ui.visuals();
                let fill = if is_selected {
                    visuals.widgets.active.bg_fill
                } else {
                    visuals.widgets.inactive.bg_fill
                };
                let stroke = if is_selected {
                    visuals.widgets.active.bg_stroke
                } else {
                    visuals.widgets.inactive.bg_stroke
                };
                let mut button = egui::Button::new(*option).fill(fill).stroke(stroke);
                if !compat_filter_matches(option, current_kind) {
                    button = button.stroke(egui::Stroke::new(
                        crate::ui::shared::layout_tokens_global::BORDER_THIN,
                        crate::ui::shared::theme_global::text_disabled(),
                    ));
                }
                if ui.add(button).clicked() {
                    state.step2.compat_popup_filter = (*option).to_string();
                }
            }
        });
    }
}
