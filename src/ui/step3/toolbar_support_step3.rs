// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) use crate::app::step3_toolbar::{
    Step3ToolbarSummary, build_toolbar_summary, open_toolbar_issue_popup, tab_has_conflict,
};

use crate::app::state::WizardState;
use crate::ui::step5::service_diagnostics_support_step5::export_diagnostics;

pub(crate) fn export_diagnostics_from_step3(
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    match export_diagnostics(state, None, dev_mode, exe_fingerprint) {
        Ok(path) => {
            state.step5.last_status_text = format!("Diagnostics exported: {}", path.display());
        }
        Err(err) => {
            state.step5.last_status_text = format!("Diagnostics export failed: {err}");
        }
    }
}

pub(crate) fn expand_all_active(state: &mut WizardState) {
    let (_, _, _, _, _, _, _, _, _, _, collapsed_blocks, _, _, _, _) =
        crate::ui::step3::state_step3::active_list_mut(state);
    crate::app::step3_history::expand_all(collapsed_blocks);
}

pub(crate) fn collapse_all_active(state: &mut WizardState) {
    let (items, _, _, _, _, _, _, _, _, _, collapsed_blocks, _, _, _, _) =
        crate::ui::step3::state_step3::active_list_mut(state);
    crate::app::step3_history::collapse_all(items, collapsed_blocks);
}

pub(crate) fn redo_active(state: &mut WizardState) {
    let (items, _, _, _, _, _, _, _, _, _, _, _, _, undo_stack, redo_stack) =
        crate::ui::step3::state_step3::active_list_mut(state);
    crate::app::step3_history::redo(items, undo_stack, redo_stack);
}

pub(crate) fn undo_active(state: &mut WizardState) {
    let (items, _, _, _, _, _, _, _, _, _, _, _, _, undo_stack, redo_stack) =
        crate::ui::step3::state_step3::active_list_mut(state);
    crate::app::step3_history::undo(items, undo_stack, redo_stack);
}
