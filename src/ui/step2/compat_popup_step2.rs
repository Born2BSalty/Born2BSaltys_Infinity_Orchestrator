// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_explain;
pub(crate) use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_kind;

fn issue_related_target(
    issue: &crate::ui::step2::compat_types_step2::CompatIssueDisplay,
) -> Option<(String, Option<u32>)> {
    explicit_related_target(
        Some(issue.related_mod.as_str()),
        issue.related_component,
    )
    .or_else(|| extract_first_jump_target(issue.raw_evidence.as_deref()))
}

fn details_related_target(
    details: &crate::ui::step2::state_step2::Step2Details,
) -> Option<(String, Option<u32>)> {
    explicit_related_target(
        details.compat_related_mod.as_deref(),
        details
            .compat_related_component
            .as_deref()
            .and_then(|value| value.parse::<u32>().ok()),
    )
    .or_else(|| extract_first_jump_target(details.compat_evidence.as_deref()))
}

fn explicit_related_target(mod_ref: Option<&str>, component_ref: Option<u32>) -> Option<(String, Option<u32>)> {
    let mod_ref = mod_ref?.trim();
    if mod_ref.is_empty() || mod_ref.eq_ignore_ascii_case("unknown") {
        return None;
    }
    Some((mod_ref.to_string(), component_ref))
}

fn extract_first_jump_target(raw_evidence: Option<&str>) -> Option<(String, Option<u32>)> {
    use crate::ui::step2::prompt_eval_expr_tokens_step2::{tokenize, Token};
    use crate::ui::step2::service_selection_step2::selection_normalize_mod_key;

    let tokens = tokenize(raw_evidence?.trim());
    let mut index = 0usize;

    while index < tokens.len() {
        let Token::Ident(name) = &tokens[index] else {
            index += 1;
            continue;
        };
        if !name.eq_ignore_ascii_case("MOD_IS_INSTALLED") {
            index += 1;
            continue;
        }
        let Some(mod_ref) = token_value(tokens.get(index + 1)) else {
            index += 1;
            continue;
        };
        let Some(component_ref) = token_value(tokens.get(index + 2)).and_then(parse_component_id) else {
            index += 1;
            continue;
        };
        return Some((selection_normalize_mod_key(&mod_ref), Some(component_ref)));
    }

    None
}

fn token_value(token: Option<&crate::ui::step2::prompt_eval_expr_tokens_step2::Token>) -> Option<String> {
    use crate::ui::step2::prompt_eval_expr_tokens_step2::Token;

    match token? {
        Token::Atom(value) | Token::Ident(value) => Some(value.trim().to_string()),
        _ => None,
    }
}

fn parse_component_id(value: String) -> Option<u32> {
    let trimmed = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '~' | '"' | '\''));
    let digits: String = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits.parse::<u32>().ok()
}

pub(crate) mod compat_popup_action_row {
    use eframe::egui;

    use crate::ui::controller::util::open_in_shell;
    use crate::ui::state::WizardState;
    use crate::ui::step2::compat_popup_nav_step2::{
        next_target, refresh_popup_override, select_popup_target, selected_game_tab,
    };
    use crate::ui::step2::compat_popup_step2::compat_popup_details as details;
    use crate::ui::step2::compat_types_step2::CompatIssueDisplay;
    use crate::ui::step2::service_selection_step2::{jump_to_target, rule_source_open_path};
    use crate::ui::step3::state_step3;

    pub(crate) fn render_action_row(ui: &mut egui::Ui, state: &mut WizardState) {
        let issue = details::selected_or_synth_issue(state);
        let can_jump_this = selected_game_tab(state).is_some();
        let can_jump_related = issue.as_ref().is_some_and(can_jump_to_related);
        let can_next = next_target(state).is_some();

        ui.horizontal(|ui| {
            if ui
                .add_enabled(can_jump_this, egui::Button::new("Jump To This"))
                .clicked()
            {
                jump_to_this(state);
            }
            if ui
                .add_enabled(can_jump_related, egui::Button::new("Jump To Related"))
                .clicked()
                && let Some(issue) = issue.as_ref()
            {
                jump_to_related(state, issue);
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

    fn can_jump_to_related(issue: &CompatIssueDisplay) -> bool {
        super::issue_related_target(issue).is_some()
    }

    fn jump_to_this(state: &mut WizardState) {
        if state.current_step == 2
            && let Some((game_tab, mod_ref, component_ref)) = selected_step3_jump_target(state)
            && state_step3::jump_to_target(state, &game_tab, &mod_ref, component_ref)
        {
            state.current_step = 2;
            refresh_popup_override(state);
            return;
        }

        let Some(game_tab) = selected_game_tab(state) else {
            return;
        };
        state.current_step = 1;
        state.step2.active_game_tab = game_tab;
        state.step2.jump_to_selected_requested = true;
        state.step2.compat_popup_issue_override = None;
    }

    fn jump_to_related(state: &mut WizardState, issue: &CompatIssueDisplay) {
        let Some(game_tab) = selected_game_tab(state) else {
            return;
        };
        let Some((related_mod, related_component)) = super::issue_related_target(issue) else {
            return;
        };
        jump_to_target(state, &game_tab, &related_mod, related_component);
        state.step2.active_game_tab = game_tab.clone();
        state.step2.jump_to_selected_requested = true;
        if state.current_step == 2 {
            let _ = state_step3::jump_to_target(
                state,
                &game_tab,
                &related_mod,
                related_component,
            );
            state.current_step = 2;
            refresh_popup_override(state);
        } else {
            state.current_step = 1;
            state.step2.compat_popup_issue_override = None;
        }
    }

    fn jump_to_next(state: &mut WizardState) {
        let Some(target) = next_target(state) else {
            return;
        };
        select_popup_target(state, &target);
    }

    fn selected_step3_jump_target(
        state: &WizardState,
    ) -> Option<(String, String, Option<u32>)> {
        match state.step2.selected.as_ref()? {
            crate::ui::state::Step2Selection::Mod { game_tab, tp_file } => {
                Some((game_tab.clone(), tp_file.clone(), None))
            }
            crate::ui::state::Step2Selection::Component {
                game_tab,
                tp_file,
                component_id,
                ..
            } => Some((
                game_tab.clone(),
                tp_file.clone(),
                component_id.trim().parse::<u32>().ok(),
            )),
        }
    }
}

pub(crate) mod compat_popup_details {
    use eframe::egui;

    use crate::ui::step2::compat_popup_nav_step2::{
        compat_filter_matches, COMPAT_POPUP_FILTER_OPTIONS,
    };
    use crate::ui::step2::compat_popup_step2::compat_popup_issue_text_explain as issue_text_explain;
    use crate::ui::step2::compat_popup_step2::compat_popup_issue_text_kind as issue_text_kind;
    use crate::ui::step2::compat_types_step2::{CompatIssueDisplay, CompatIssueStatusTone};
    use crate::ui::step2::content_step2::step2_details_select::selected_details;
    use crate::ui::step2::state_step2::Step2Details;
    use crate::ui::state::WizardState;

    pub(crate) fn render_details(ui: &mut egui::Ui, state: &mut WizardState) {
        let details = selected_details(state);
        let issue = selected_or_synth_issue(state);
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
            .or(issue.as_ref().map(|issue| issue.kind.as_str()))
            .unwrap_or("unknown");
        if let Some(issue) = issue.as_ref()
            && !kind.eq_ignore_ascii_case("included")
        {
            ui.horizontal(|ui| {
                ui.label(crate::ui::shared::typography_global::strong("Status"));
                let badge_color = match issue.status_tone {
                    CompatIssueStatusTone::Neutral => {
                        crate::ui::shared::theme_global::text_muted()
                    }
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

        if let Some(issue) = issue.as_ref() {
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
            ui.monospace(issue_text_explain::display_source(source));
        }
        if let Some(block) = details.compat_component_block.as_deref() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(
                crate::ui::shared::typography_global::strong("Component block"),
            )
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

    pub(crate) fn selected_or_synth_issue(state: &WizardState) -> Option<CompatIssueDisplay> {
        if let Some(issue) = state.step2.compat_popup_issue_override.clone() {
            return Some(issue);
        }
        synth_issue_from_details(&selected_details(state))
    }

    fn synth_issue_from_details(details: &Step2Details) -> Option<CompatIssueDisplay> {
        let kind = details.compat_kind.as_deref()?.to_ascii_lowercase();
        let code = if kind == "mismatch" || kind == "game_mismatch" {
            "MISMATCH"
        } else if kind == "missing_dep" {
            "REQ_MISSING"
        } else if kind == "conflict" || kind == "not_compatible" {
            "RULE_HIT"
        } else if kind == "included" {
            "INCLUDED"
        } else if kind == "order_block" {
            "ORDER_BLOCK"
        } else if kind == "conditional" {
            "CONDITIONAL"
        } else if kind == "path_requirement" {
            "PATH_REQUIREMENT"
        } else if kind == "deprecated" {
            "DEPRECATED"
        } else {
            "RULE_HIT"
        };
        let (status_label, status_tone) = popup_issue_status(kind.as_str());
        let related = if kind == "mismatch" || kind == "game_mismatch" {
            details.compat_related_target.clone().unwrap_or_default()
        } else {
            details
                .compat_related_target
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        };
        let fallback_related = super::details_related_target(details);
        Some(CompatIssueDisplay {
            kind,
            code: code.to_string(),
            status_label: status_label.to_string(),
            status_tone,
            related_mod: details
                .compat_related_mod
                .clone()
                .or_else(|| fallback_related.as_ref().map(|(related_mod, _)| related_mod.clone()))
                .unwrap_or(related),
            related_component: details
                .compat_related_component
                .as_deref()
                .and_then(|value| value.parse::<u32>().ok())
                .or_else(|| fallback_related.and_then(|(_, related_component)| related_component)),
            reason: details.disabled_reason.clone().unwrap_or_default(),
            source: details.compat_source.clone().unwrap_or_default(),
            raw_evidence: details.compat_evidence.clone(),
        })
    }

    fn popup_issue_status(kind: &str) -> (&'static str, CompatIssueStatusTone) {
        match kind {
            "included" => ("Already included", CompatIssueStatusTone::Neutral),
            "not_needed" => ("Not needed", CompatIssueStatusTone::Neutral),
            "missing_dep" | "order_block" | "warning" | "deprecated" | "conditional" => {
                ("Warning only", CompatIssueStatusTone::Warning)
            }
            _ => ("Resolve before continuing", CompatIssueStatusTone::Blocking),
        }
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
