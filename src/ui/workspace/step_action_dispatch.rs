// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::app_step2_router;
use crate::app::app_step4_flow;
use crate::app::step4_action::Step4Action;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step2::step2_rescan_reconcile;
use crate::ui::workspace::step2_log_glue;

pub fn dispatch_step2(action: Step2Action, orchestrator: &mut OrchestratorApp) {
    match action {
        Step2Action::SelectBgeeViaLog => {
            step2_log_glue::apply_weidu_log_selection_for_orchestrator(orchestrator, true);
            orchestrator.mark_workspace_dirty();
        }
        Step2Action::SelectBg2eeViaLog => {
            step2_log_glue::apply_weidu_log_selection_for_orchestrator(orchestrator, false);
            orchestrator.mark_workspace_dirty();
        }

        Step2Action::DownloadUpdates => {
            step2_rescan_reconcile::arm_post_download_snapshot(orchestrator);
            handle_step2_via_bio(Step2Action::DownloadUpdates, orchestrator);
            orchestrator.mark_workspace_dirty();
        }

        Step2Action::OpenSelectedReadme(_)
        | Step2Action::OpenSelectedWeb(_)
        | Step2Action::OpenSelectedTp2Folder(_)
        | Step2Action::OpenSelectedTp2(_)
        | Step2Action::OpenSelectedIni(_) => {
            handle_step2_via_bio(action, orchestrator);
        }

        _ => {
            handle_step2_via_bio(action, orchestrator);
            orchestrator.mark_workspace_dirty();
        }
    }
}

fn handle_step2_via_bio(action: Step2Action, orchestrator: &mut OrchestratorApp) {
    app_step2_router::handle_step2_action(
        &mut orchestrator.wizard_state,
        &mut orchestrator.step2_scan_rx,
        &mut orchestrator.step2_cancel,
        &mut orchestrator.step2_progress_queue,
        &mut orchestrator.step2_update_check_rx,
        &mut orchestrator.step2_update_download_rx,
        action,
    );
}

pub fn dispatch_step4(action: Step4Action, orchestrator: &mut OrchestratorApp) {
    app_step4_flow::handle_step4_action(&mut orchestrator.wizard_state, action);
    orchestrator.mark_workspace_dirty();
}
