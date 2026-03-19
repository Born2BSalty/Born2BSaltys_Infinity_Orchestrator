// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod map {
    use crate::compat::model::CompatValidationResult;
    use crate::ui::state::{CompatIssueDisplay, CompatState};

    pub(super) fn to_state(result: CompatValidationResult) -> CompatState {
        let issues = map_issues(&result);
        CompatState {
            error_count: result.error_count(),
            warning_count: result.warning_count(),
            issues,
            show_pre_install_modal: false,
        }
    }

    pub(super) fn combine_states(a: CompatState, b: CompatState) -> CompatState {
        let mut issues = a.issues;
        issues.extend(b.issues);
        CompatState {
            error_count: a.error_count + b.error_count,
            warning_count: a.warning_count + b.warning_count,
            issues,
            show_pre_install_modal: false,
        }
    }

    fn map_issues(result: &CompatValidationResult) -> Vec<CompatIssueDisplay> {
        result
            .issues
            .iter()
            .map(|i| CompatIssueDisplay {
                issue_id: i.issue_id.clone(),
                code: i.code.as_str().to_string(),
                is_blocking: i.is_blocking(),
                affected_mod: i.affected_mod.clone(),
                affected_component: i.affected_component,
                related_mod: i.related_mod.clone(),
                related_component: i.related_component,
                reason: i.reason.clone(),
                source: i.source.description(),
                raw_evidence: i.raw_evidence.clone(),
            })
            .collect()
    }
}
mod select {
    use crate::compat::SelectedComponent;
    use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;
    use crate::ui::state::{Step2ModState, Step3ItemState};

    pub(super) fn build_selected_components(items: &[Step3ItemState]) -> Vec<SelectedComponent> {
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_parent)
            .map(|(idx, item)| SelectedComponent {
                mod_name: item.mod_name.clone(),
                tp_file: effective_tp2_for_row(&item.tp_file, &item.raw_line),
                component_id: item.component_id.parse().unwrap_or(0),
                order: idx,
            })
            .collect()
    }

    pub(super) fn build_selected_components_from_step2(
        mods: &[Step2ModState],
    ) -> Vec<SelectedComponent> {
        #[derive(Clone)]
        struct SelectedRow {
            mod_name: String,
            tp_file: String,
            component_id: u32,
            selected_order: usize,
        }

        let mut rows: Vec<SelectedRow> = Vec::new();
        for mod_state in mods {
            for component in mod_state.components.iter().filter(|c| c.checked) {
                rows.push(SelectedRow {
                    mod_name: mod_state.name.clone(),
                    tp_file: effective_tp2_for_row(&mod_state.tp_file, &component.raw_line),
                    component_id: component.component_id.parse().unwrap_or(0),
                    selected_order: component.selected_order.unwrap_or(usize::MAX),
                });
            }
        }
        rows.sort_by_key(|r| r.selected_order);
        rows.into_iter()
            .enumerate()
            .map(|(order, row)| SelectedComponent {
                mod_name: row.mod_name,
                tp_file: row.tp_file,
                component_id: row.component_id,
                order,
            })
            .collect()
    }

    fn effective_tp2_for_row(default_tp_file: &str, raw_line: &str) -> String {
        parse_component_tp2_from_raw(raw_line).unwrap_or_else(|| default_tp_file.to_string())
    }
}
mod validate {
    use crate::compat::CompatValidator;
    use crate::ui::state::{CompatState, WizardState};

    use super::map;
    use super::select;

    pub(super) fn run_validation(validator: &CompatValidator, state: &WizardState) -> CompatState {
        let game = state.step1.game_install.as_str();
        let items = match game {
            "BG2EE" => &state.step3.bg2ee_items,
            _ => &state.step3.bgee_items,
        };

        let selected = select::build_selected_components(items);
        let result = validator.validate(&selected, game);
        map::to_state(result)
    }

    pub(super) fn run_validation_for_both_games(
        validator: &CompatValidator,
        state: &WizardState,
    ) -> CompatState {
        let game = state.step1.game_install.as_str();

        match game {
            "EET" => {
                let bgee_selected = select::build_selected_components(&state.step3.bgee_items);
                let bg2ee_selected = select::build_selected_components(&state.step3.bg2ee_items);

                // In EET installs both streams execute in EET runtime context.
                let bgee_result = validator.validate(&bgee_selected, "EET");
                let bg2ee_result = validator.validate(&bg2ee_selected, "EET");

                let left = map::to_state(bgee_result);
                let right = map::to_state(bg2ee_result);
                map::combine_states(left, right)
            }
            _ => run_validation(validator, state),
        }
    }

    pub(super) fn run_validation_for_step2_checked_order(
        validator: &CompatValidator,
        state: &WizardState,
    ) -> CompatState {
        let game = state.step1.game_install.as_str();

        match game {
            "EET" => {
                let bgee_selected =
                    select::build_selected_components_from_step2(&state.step2.bgee_mods);
                let bg2ee_selected =
                    select::build_selected_components_from_step2(&state.step2.bg2ee_mods);

                // In EET installs both tabs should validate against EET, not standalone BGEE/BG2EE.
                let bgee_result = validator.validate(&bgee_selected, "EET");
                let bg2ee_result = validator.validate(&bg2ee_selected, "EET");

                let left = map::to_state(bgee_result);
                let right = map::to_state(bg2ee_result);
                map::combine_states(left, right)
            }
            "BGEE" => {
                let selected = select::build_selected_components_from_step2(&state.step2.bgee_mods);
                let result = validator.validate(&selected, "BGEE");
                map::to_state(result)
            }
            _ => {
                let selected =
                    select::build_selected_components_from_step2(&state.step2.bg2ee_mods);
                let result = validator.validate(&selected, "BG2EE");
                map::to_state(result)
            }
        }
    }
}

use crate::compat::CompatValidator;
use crate::ui::state::{CompatState, WizardState};

pub fn run_validation_for_both_games(
    validator: &CompatValidator,
    state: &WizardState,
) -> CompatState {
    validate::run_validation_for_both_games(validator, state)
}

pub fn run_validation_for_step2_checked_order(
    validator: &CompatValidator,
    state: &WizardState,
) -> CompatState {
    validate::run_validation_for_step2_checked_order(validator, state)
}
