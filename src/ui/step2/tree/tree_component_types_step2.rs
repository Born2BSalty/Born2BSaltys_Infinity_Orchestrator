// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step2Selection;
use crate::parser::prompt_eval_expr::PromptEvalContext;
use crate::ui::shared::redesign_tokens::ThemePalette;

pub(crate) type CompatPopupTarget = Option<(String, String, String)>;
pub(crate) type PromptPopupTarget = Option<(String, String)>;

pub(crate) struct ComponentRowsResult {
    pub selection: Option<Step2Selection>,
    pub compat_popup: CompatPopupTarget,
    pub prompt_popup: PromptPopupTarget,
}

pub(crate) struct ComponentRowsContext<'a> {
    pub filter: &'a str,
    pub active_tab: &'a str,
    pub selected: &'a Option<Step2Selection>,
    pub next_selection_order: &'a mut usize,
    pub prompt_eval: &'a PromptEvalContext,
    pub collapse_epoch: u64,
    pub collapse_default_open: bool,
    pub jump_to_selected_requested: &'a mut bool,
    pub tp_file: &'a str,
    pub mod_name: &'a str,
    pub palette: ThemePalette,
}

pub(crate) struct ComponentRenderState<'a> {
    pub selection: &'a mut Option<Step2Selection>,
    pub compat_popup: &'a mut CompatPopupTarget,
    pub prompt_popup: &'a mut PromptPopupTarget,
    pub enforce_single_select_for: &'a mut Vec<usize>,
    pub enforce_collapsible_group_for: &'a mut Vec<usize>,
    pub enforce_meta_for: &'a mut Vec<usize>,
}

pub(crate) fn reborrow_render_state<'a>(
    state: &'a mut ComponentRenderState<'_>,
) -> ComponentRenderState<'a> {
    ComponentRenderState {
        selection: &mut *state.selection,
        compat_popup: &mut *state.compat_popup,
        prompt_popup: &mut *state.prompt_popup,
        enforce_single_select_for: &mut *state.enforce_single_select_for,
        enforce_collapsible_group_for: &mut *state.enforce_collapsible_group_for,
        enforce_meta_for: &mut *state.enforce_meta_for,
    }
}
