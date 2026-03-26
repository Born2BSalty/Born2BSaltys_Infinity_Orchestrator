// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::collections::HashSet;

#[path = "validator_helpers.rs"]
mod validator_helpers;
#[path = "validator_rule_handlers.rs"]
mod validator_rule_handlers;
#[path = "validator_rules.rs"]
mod validator_rules;

use super::model::{CompatValidationResult, Tp2Metadata};

#[derive(Debug, Clone)]
pub struct SelectedComponent {
    pub mod_name: String,
    pub tp_file: String,
    pub component_id: u32,
    pub order: usize,
}

pub(super) struct RuleEvalContext<'a> {
    pub metadata: &'a Tp2Metadata,
    pub component: &'a SelectedComponent,
    pub game_mode: &'a str,
    pub selected_set: &'a HashSet<(String, u32)>,
    pub order_map: &'a HashMap<(String, u32), usize>,
}

pub struct CompatValidator {
    tp2_metadata: HashMap<String, Tp2Metadata>,
}

impl CompatValidator {
    pub fn new() -> Self {
        Self {
            tp2_metadata: HashMap::new(),
        }
    }

    pub fn set_tp2_metadata(&mut self, metadata: HashMap<String, Tp2Metadata>) {
        self.tp2_metadata = metadata;
    }

    pub fn validate_step2_selection(
        &self,
        selected: &[SelectedComponent],
        game_mode: &str,
    ) -> CompatValidationResult {
        let selected_set = validator_helpers::build_selected_set(selected);
        let order_map = validator_helpers::build_order_map(selected);
        let mut issues = validator_rules::validate_step2_selection_rules(
            selected,
            game_mode,
            &self.tp2_metadata,
            &selected_set,
            &order_map,
        );
        issues.extend(validator_rules::validate_duplicates(selected));
        CompatValidationResult { issues }
    }

    pub fn validate_step3_order(
        &self,
        selected: &[SelectedComponent],
        game_mode: &str,
    ) -> CompatValidationResult {
        let selected_set = validator_helpers::build_selected_set(selected);
        let order_map = validator_helpers::build_order_map(selected);
        let mut issues = validator_rules::validate_step3_selection_rules(
            selected,
            game_mode,
            &self.tp2_metadata,
            &selected_set,
            &order_map,
        );
        issues.extend(validator_rules::validate_duplicates(selected));
        CompatValidationResult { issues }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compat::model::{CompatIssueCode, Tp2Rule};

    #[test]
    fn require_installed_any_with_any_component_emits_order_warn_when_late() {
        let mut validator = CompatValidator::new();
        let mut tp2_map = HashMap::new();
        tp2_map.insert(
            "moda".to_string(),
            Tp2Metadata {
                tp_file: "setup-moda.tp2".to_string(),
                setup_tra: HashMap::new(),
                component_blocks: HashMap::new(),
                rules: vec![(
                    10,
                    Tp2Rule::RequireInstalledAny {
                        targets: vec![("modb".to_string(), None)],
                        raw_line: "REQUIRE_PREDICATE MOD_IS_INSTALLED ~modb/setup-modb.tp2~"
                            .to_string(),
                        line: 12,
                    },
                )],
            },
        );
        validator.set_tp2_metadata(tp2_map);

        let selected = vec![
            SelectedComponent {
                mod_name: "moda".to_string(),
                tp_file: "setup-moda.tp2".to_string(),
                component_id: 10,
                order: 0,
            },
            SelectedComponent {
                mod_name: "modb".to_string(),
                tp_file: "setup-modb.tp2".to_string(),
                component_id: 1,
                order: 1,
            },
        ];

        let result = validator.validate_step3_order(&selected, "BGEE");
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.code == CompatIssueCode::OrderBlock),
            "expected ORDER_BLOCK for any-component dependency when target mod is after source"
        );

        let step2_result = validator.validate_step2_selection(&selected, "BGEE");
        assert!(
            step2_result
                .issues
                .iter()
                .any(|i| i.code == CompatIssueCode::OrderBlock),
            "expected ORDER_BLOCK in Step 2 when required target mod is selected after source"
        );
    }
}
