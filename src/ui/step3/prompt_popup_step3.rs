// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ComponentState, Step3ItemState};
use crate::ui::step2::state_step2::PromptEvalContext;

pub(crate) fn evaluate_step3_item_prompt_summary(
    item: &Step3ItemState,
    prompt_eval: &PromptEvalContext,
) -> String {
    if item.is_parent {
        return String::new();
    }
    let component = Step2ComponentState {
        component_id: item.component_id.clone(),
        label: item.component_label.clone(),
        raw_line: item.raw_line.clone(),
        prompt_summary: item.prompt_summary.clone(),
        prompt_events: item.prompt_events.clone(),
        is_meta_mode_component: false,
        disabled: false,
        compat_kind: None,
        compat_source: None,
        compat_related_mod: None,
        compat_related_component: None,
        compat_graph: None,
        compat_evidence: None,
        disabled_reason: None,
        checked: true,
        selected_order: Some(item.selected_order),
    };
    crate::ui::step2::prompt_eval_step2::evaluate_component_prompt_summary(&component, prompt_eval)
}

pub(crate) fn format_step3_prompt_popup(item: &Step3ItemState, body: &str) -> (String, String) {
    (
        format!("{} #{}", item.tp_file, item.component_id),
        format!(
            "Component: {} - {}

{}",
            item.component_id.trim(),
            item.component_label.trim(),
            body.trim()
        ),
    )
}
