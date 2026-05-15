// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::prompt_eval_summary::evaluate_prompt_summary_input;
use crate::app::state::Step3ItemState;
use crate::parser::PromptEvalContext;

pub(crate) fn evaluate_step3_item_prompt_summary(
    item: &Step3ItemState,
    prompt_eval: &PromptEvalContext,
) -> String {
    if item.is_parent {
        return String::new();
    }
    evaluate_prompt_summary_input(
        true,
        &item.component_id,
        &item.raw_line,
        item.prompt_summary.as_deref(),
        &item.prompt_events,
        prompt_eval,
    )
}
