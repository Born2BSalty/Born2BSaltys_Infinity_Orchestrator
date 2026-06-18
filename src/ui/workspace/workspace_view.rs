// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::{
    workspace_header, workspace_hint_line, workspace_nav_bar, workspace_progress_bar,
    workspace_step_router,
};

pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    _modlist_id: &str,
    ctx: &egui::Context,
) {
    let palette = orchestrator.theme_palette;

    workspace_header::render(ui, orchestrator, ctx);
    ui.add_space(10.0);

    workspace_progress_bar::render(ui, palette, &orchestrator.workspace_view);

    let current = orchestrator.workspace_view.current_step;
    workspace_hint_line::render(ui, palette, current);

    let nav_reserve = 84.0;
    let avail_h = ui.available_height();
    let body_h = (avail_h - nav_reserve).max(0.0);
    ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                workspace_step_router::render(ui, orchestrator);
            });
    });

    let disable_prev = orchestrator.workspace_view.install_complete
        || orchestrator.wizard_state.step5.install_running
        || orchestrator.workspace_step5.install_clicked;
    let nav_status = nav_status_text(orchestrator, current);
    let outcome =
        workspace_nav_bar::render(ui, palette, current, disable_prev, nav_status.as_deref());

    if outcome.next_clicked {
        if let Some(next) = current.next() {
            if current == WorkspaceStep::Step2 {
                sync_step3_from_step2_on_nav_edge(orchestrator);
            }
            if current == WorkspaceStep::Step4 {
                auto_save_step4_weidu_logs_on_nav_edge(orchestrator);
            }
            orchestrator.workspace_view.completed_steps.insert(current);
            orchestrator.workspace_view.current_step = next;
        }
    } else if outcome.prev_clicked {
        if let Some(prev) = current.prev() {
            orchestrator.workspace_view.current_step = prev;
        } else {
            orchestrator.nav = NavDestination::Home;
        }
    }
}

fn nav_status_text(orchestrator: &OrchestratorApp, current: WorkspaceStep) -> Option<String> {
    if current != WorkspaceStep::Step2 {
        return None;
    }

    let status = orchestrator.wizard_state.step2.scan_status.trim();
    if status.is_empty() {
        return None;
    }

    Some(
        orchestrator
            .workspace_view
            .step2
            .rescan_drop_warning
            .as_deref()
            .map_or_else(
                || status.to_string(),
                |warning| format!("{status} - {warning}"),
            ),
    )
}

fn sync_step3_from_step2_on_nav_edge(orchestrator: &mut OrchestratorApp) {
    use crate::app::app_nav::{NextAction, decide_next_action};
    use crate::app::app_step3_sync_flow::sync_step3_from_step2;

    let state = &mut orchestrator.wizard_state;

    let saved_step = state.current_step;
    state.current_step = 1;
    let action = decide_next_action(state);
    state.current_step = saved_step;

    if let NextAction::SyncStep3AndAdvance { signature } = action {
        sync_step3_from_step2(state);
        state.set_last_step2_sync_signature(signature);
    }
}

fn auto_save_step4_weidu_logs_on_nav_edge(orchestrator: &OrchestratorApp) {
    if let Err(err) = write_step4_weidu_logs_unconditional(&orchestrator.wizard_state) {
        tracing::warn!(
            target = "orchestrator",
            "Step 4 → Step 5 weidu.log auto-save failed: {err} \
             (install proceeds against the on-disk file as-is — the user's \
             Step 2/3 edits may not reach the runner)"
        );
    }
}

fn write_step4_weidu_logs_unconditional(
    state: &crate::app::state::WizardState,
) -> std::io::Result<()> {
    use crate::app::step5::diagnostics::build_weidu_export_lines;
    use std::path::PathBuf;

    const HEADER: [&str; 3] = [
        "// Log of Currently Installed WeiDU Mods",
        "// The top of the file is the 'oldest' mod",
        "// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]",
    ];

    fn write_target(folder: &str, lines: Vec<String>) -> std::io::Result<()> {
        let folder = folder.trim();
        if folder.is_empty() {
            return Ok(());
        }
        let dir = PathBuf::from(folder);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("weidu.log");
        let mut out: Vec<String> = HEADER
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        out.extend(lines);
        std::fs::write(path, out.join("\n"))?;
        Ok(())
    }

    match state.step1.game_install.as_str() {
        "EET" => {
            write_target(
                &state.step1.eet_bgee_log_folder,
                build_weidu_export_lines(&state.step3.bgee_items),
            )?;
            write_target(
                &state.step1.eet_bg2ee_log_folder,
                build_weidu_export_lines(&state.step3.bg2ee_items),
            )?;
        }
        "BG2EE" => {
            write_target(
                &state.step1.bg2ee_log_folder,
                build_weidu_export_lines(&state.step3.bg2ee_items),
            )?;
        }
        _ => {
            write_target(
                &state.step1.bgee_log_folder,
                build_weidu_export_lines(&state.step3.bgee_items),
            )?;
        }
    }
    Ok(())
}

pub const NAV_BAR_RESERVE_PX: f32 = 84.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_reserve_constant_is_reasonable() {
        const {
            assert!(NAV_BAR_RESERVE_PX >= 64.0);
            assert!(NAV_BAR_RESERVE_PX <= 120.0);
        }
    }

    #[test]
    fn step_advance_logic_matches_wireframe_gonext() {
        let mut completed = std::collections::HashSet::new();
        let current = WorkspaceStep::Step2;
        if let Some(next) = current.next() {
            completed.insert(current);
            assert_eq!(next, WorkspaceStep::Step3);
        }
        assert!(completed.contains(&WorkspaceStep::Step2));
    }
}
