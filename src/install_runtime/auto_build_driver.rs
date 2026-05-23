// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use tracing::warn;

use crate::app::modlist_share::import_modlist_share_code;
use crate::app::state::{Step1State, WizardState};
use crate::install_runtime::flag_policies::InstallWorkflow;
use crate::install_runtime::per_install_dirs::{self, PerInstallDirs};
use crate::registry::model::Game;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepOutcome {
    PipelineArmed { dirs: PerInstallDirs },

    DirsOnly { dirs: PerInstallDirs },
}

pub fn prepare_install_dirs_and_maybe_import(
    wizard_state: &mut WizardState,
    destination: &str,
    game: Game,
    workflow: InstallWorkflow,
    share_code: &str,
) -> Result<PrepOutcome, String> {
    let dirs =
        per_install_dirs::derive_per_install_dirs(&mut wizard_state.step1, destination, game)
            .map_err(|err| format!("per-install directory derivation failed: {err}"))?;

    if !is_share_code_consuming(workflow) {
        return Ok(PrepOutcome::DirsOnly { dirs });
    }
    if share_code.trim().is_empty() {
        return Err(
            "share-code-consuming workflow has no share code to import (SPEC §13.12a)".to_string(),
        );
    }

    import_modlist_share_code(wizard_state, share_code.trim())
        .map_err(|err| format!("import_modlist_share_code failed: {err}"))?;

    apply_workflow_install_mode_override(&mut wizard_state.step1, workflow);

    arm_auto_build(wizard_state);

    Ok(PrepOutcome::PipelineArmed { dirs })
}

#[must_use]
pub const fn is_share_code_consuming(workflow: InstallWorkflow) -> bool {
    match workflow {
        InstallWorkflow::PasteAndInstall
        | InstallWorkflow::ForkAndModify
        | InstallWorkflow::ContinuePartialInstall
        | InstallWorkflow::Reinstall => true,
        InstallWorkflow::FreshCreate => false,
    }
}

/// Forces `step1.install_mode = EXACT_WEIDU_LOGS` for Install-paste so
/// BIO's path preflight clears via the `have_weidu_logs && download_archive`
/// exemption while the per-install Mods folder is still empty. Other
/// workflows keep whatever the share code (or upstream code path) declared.
pub(crate) fn apply_workflow_install_mode_override(
    step1: &mut Step1State,
    workflow: InstallWorkflow,
) {
    let forced = match workflow {
        InstallWorkflow::PasteAndInstall => Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS,
        InstallWorkflow::FreshCreate
        | InstallWorkflow::ForkAndModify
        | InstallWorkflow::ContinuePartialInstall
        | InstallWorkflow::Reinstall => return,
    };
    step1.install_mode = forced.to_string();
    step1.sync_install_mode_flags();
}

pub fn arm_download_archive_policy(state: &mut WizardState, mods_archive_folder: &str) {
    state.step1.download_archive = true;

    state.step1.mods_archive_folder = mods_archive_folder.trim().to_string();

    state.step1.download = true;
}

fn arm_auto_build(state: &mut WizardState) {
    state.modlist_auto_build_active = true;
    state.modlist_auto_build_waiting_for_install = false;
    state.current_step = 1;
    state.step2.active_game_tab = if state.step1.game_install == "BGEE" {
        "BGEE".to_string()
    } else {
        "BG2EE".to_string()
    };
    state.step2.pending_saved_log_apply = true;
    state.step2.pending_saved_log_update_preview = true;

    state.step2.scan_status = "Auto Build: preparing imported modlist".to_string();
    state.step5.last_status_text = "Auto Build: preparing imported modlist".to_string();
}

#[must_use]
pub const fn download_gate_open(state: &WizardState) -> bool {
    state.modlist_auto_build_active
        && !state.step2.pending_saved_log_apply
        && !state.step2.pending_saved_log_update_preview
        && !state.step2.update_selected_check_running
        && !state.step2.update_selected_download_running
        && !state.step2.update_selected_extract_running
        && !state.step2.is_scanning
}

#[must_use]
pub const fn pipeline_finished(state: &WizardState) -> bool {
    !state.modlist_auto_build_active
        && !state.step2.pending_saved_log_apply
        && !state.step2.pending_saved_log_update_preview
        && !state.step2.pending_saved_log_download
}

#[must_use]
pub const fn pipeline_reached_install(state: &WizardState) -> bool {
    pipeline_finished(state)
        && (state.step5.start_install_requested
            || state.step5.install_running
            || state.current_step == 4)
}

pub fn log_if_pipeline_stopped(state: &WizardState) {
    if pipeline_finished(state) && !pipeline_reached_install(state) {
        warn!(
            target = "orchestrator",
            "auto-build pipeline did not reach install: {}", state.step2.scan_status
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn td() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static C: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "bio_auto_build_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn fresh_create_derives_dirs_no_import_no_arm() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut st = WizardState::default();
        let out = prepare_install_dirs_and_maybe_import(
            &mut st,
            &dest_s,
            Game::BGEE,
            InstallWorkflow::FreshCreate,
            "",
        )
        .expect("fresh-create prep");
        assert!(matches!(out, PrepOutcome::DirsOnly { .. }));
        assert!(
            !st.modlist_auto_build_active,
            "fresh-create must NOT arm the auto-build pipeline"
        );
        assert!(
            !st.step5.start_install_requested,
            "prep never flips start_install_requested itself"
        );

        assert!(st.step1.generate_directory_enabled);
        assert!(!st.step1.mods_folder.is_empty());
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn share_code_consuming_without_code_is_error() {
        let dest = td();
        let mut st = WizardState::default();
        let r = prepare_install_dirs_and_maybe_import(
            &mut st,
            &dest.to_string_lossy(),
            Game::EET,
            InstallWorkflow::PasteAndInstall,
            "   ",
        );
        assert!(r.is_err(), "share-code-consuming needs a code");
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn arm_auto_build_matches_bio_recipe_minus_install_flip_and_download_flag() {
        let mut st = WizardState::default();
        st.step1.game_install = "EET".to_string();
        arm_auto_build(&mut st);
        assert!(st.modlist_auto_build_active);
        assert!(!st.modlist_auto_build_waiting_for_install);
        assert_eq!(st.current_step, 1);
        assert_eq!(
            st.step2.active_game_tab,
            "BGEE".to_string().replace("BGEE", "BG2EE")
        );
        assert!(st.step2.pending_saved_log_apply);
        assert!(st.step2.pending_saved_log_update_preview);
        assert!(
            !st.step2.pending_saved_log_download,
            "#1 — pending_saved_log_download is NOT armed so BIO's serial \
             download never fires (the orchestrator's parallel \
             stream_downloader owns it — no double-download)"
        );
        assert!(
            !st.step5.start_install_requested,
            "arm must NOT pre-flip start_install_requested — the pipeline's \
             start_auto_build_install does that after download/extract (a \
             premature flip installs an empty Mods folder)"
        );

        let mut b = WizardState::default();
        b.step1.game_install = "BGEE".to_string();
        arm_auto_build(&mut b);
        assert_eq!(b.step2.active_game_tab, "BGEE");
    }

    #[test]
    fn download_gate_opens_only_after_apply_and_preview_done() {
        let mut st = WizardState::default();

        assert!(!download_gate_open(&st));

        arm_auto_build(&mut st);

        assert!(
            !download_gate_open(&st),
            "gate must stay closed while apply/update-preview are pending"
        );

        st.step2.pending_saved_log_apply = false;
        st.step2.pending_saved_log_update_preview = false;
        assert!(
            download_gate_open(&st),
            "gate opens once apply+preview are done and nothing is running"
        );

        st.step2.update_selected_download_running = true;
        assert!(!download_gate_open(&st));
        st.step2.update_selected_download_running = false;
        st.step2.is_scanning = true;
        assert!(!download_gate_open(&st));
        st.step2.is_scanning = false;
        st.step2.update_selected_check_running = true;
        assert!(!download_gate_open(&st));
    }

    #[test]
    fn pipeline_finished_and_reached_install_predicates() {
        let mut st = WizardState::default();

        arm_auto_build(&mut st);
        assert!(!pipeline_finished(&st));
        assert!(!pipeline_reached_install(&st));

        st.modlist_auto_build_active = false;
        st.step2.pending_saved_log_apply = false;
        st.step2.pending_saved_log_update_preview = false;
        st.step2.pending_saved_log_download = false;
        st.current_step = 4;
        st.step5.start_install_requested = true;
        assert!(pipeline_finished(&st));
        assert!(pipeline_reached_install(&st));

        let mut stopped = WizardState::default();
        arm_auto_build(&mut stopped);
        stopped.modlist_auto_build_active = false;
        stopped.step2.pending_saved_log_apply = false;
        stopped.step2.pending_saved_log_update_preview = false;
        stopped.step2.pending_saved_log_download = false;
        stopped.step2.scan_status =
            "Auto Build stopped: local path/tool preflight failed".to_string();
        assert!(pipeline_finished(&stopped));
        assert!(
            !pipeline_reached_install(&stopped),
            "a stopped pipeline must NOT count as reached-install"
        );
    }

    #[test]
    fn is_share_code_consuming_matches_spec_13_12_5() {
        assert!(is_share_code_consuming(InstallWorkflow::PasteAndInstall));
        assert!(is_share_code_consuming(InstallWorkflow::ForkAndModify));
        assert!(is_share_code_consuming(
            InstallWorkflow::ContinuePartialInstall
        ));
        assert!(is_share_code_consuming(InstallWorkflow::Reinstall));
        assert!(!is_share_code_consuming(InstallWorkflow::FreshCreate));
    }

    #[test]
    fn arm_download_archive_policy_sets_the_three_step1_fields() {
        let mut st = WizardState::default();
        assert!(
            !st.step1.download_archive,
            "precondition: download_archive defaults false (a download guard)"
        );
        assert!(
            st.step1.mods_archive_folder.is_empty(),
            "precondition: mods_archive_folder defaults empty (a download guard)"
        );

        arm_download_archive_policy(&mut st, r"D:\BG\ModsArchive");

        assert!(
            st.step1.download_archive,
            "download_archive forced ON (SPEC §13.12a always-content-addressed stage)"
        );

        assert_eq!(
            st.step1.mods_archive_folder, r"D:\BG\ModsArchive",
            "mods_archive_folder = the Settings → Paths value"
        );

        assert!(
            st.step1.download,
            "download forced ON (SPEC §13.12 #5 — pipeline path skips apply_flags)"
        );
    }

    #[test]
    fn arm_download_archive_policy_trims_archive_folder() {
        let mut st = WizardState::default();
        arm_download_archive_policy(&mut st, "  C:\\Mods Archive  ");
        assert_eq!(
            st.step1.mods_archive_folder, "C:\\Mods Archive",
            "leading/trailing whitespace trimmed (matches BIO's trim check)"
        );

        let mut st2 = WizardState::default();
        arm_download_archive_policy(&mut st2, "   ");
        assert!(
            st2.step1.mods_archive_folder.is_empty(),
            "whitespace-only ⇒ empty (BIO's own empty-archive guard still applies)"
        );

        assert!(st2.step1.download_archive);
        assert!(st2.step1.download);
    }

    #[test]
    fn paste_flow_forces_exact_weidu_logs_regardless_of_share_install_mode() {
        let mut step1 = Step1State {
            install_mode: Step1State::INSTALL_MODE_BUILD_FROM_SCANNED_MODS.to_string(),
            have_weidu_logs: false,
            ..Default::default()
        };

        apply_workflow_install_mode_override(&mut step1, InstallWorkflow::PasteAndInstall);

        assert_eq!(
            step1.install_mode,
            Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS,
            "paste flow must force EXACT_WEIDU_LOGS even when the share \
             code declared BUILD_FROM_SCANNED_MODS"
        );
        assert!(
            step1.have_weidu_logs,
            "sync_install_mode_flags must set have_weidu_logs = true for \
             EXACT_WEIDU_LOGS so the path preflight clears via the \
             weidu-logs-mode + download_archive exemption"
        );
    }

    #[test]
    fn fork_flow_keeps_share_install_mode_no_override() {
        // Fork-and-modify intentionally passes through the share code's
        // install_mode untouched. Forcing a mode here would either re-create
        // the empty-Mods preflight failure (BUILD_FROM_SCANNED_MODS) or
        // shadow the user-modifying flow Step 2 needs to drive.
        let mut step1 = Step1State {
            install_mode: Step1State::INSTALL_MODE_BUILD_FROM_SCANNED_MODS.to_string(),
            have_weidu_logs: true,
            ..Default::default()
        };

        apply_workflow_install_mode_override(&mut step1, InstallWorkflow::ForkAndModify);

        assert_eq!(
            step1.install_mode,
            Step1State::INSTALL_MODE_BUILD_FROM_SCANNED_MODS,
            "fork workflow must NOT override the share's install_mode"
        );
        assert!(
            step1.have_weidu_logs,
            "fork workflow must not run sync_install_mode_flags"
        );
    }

    #[test]
    fn arm_download_archive_policy_survives_reset_workflow_keep_step1() {
        let mut st = WizardState::default();
        arm_download_archive_policy(&mut st, r"D:\BG\ModsArchive");

        let cloned = st.step1.clone();
        st.step1 = cloned;
        st.reset_workflow_keep_step1();
        assert!(
            st.step1.download_archive,
            "download_archive survives the clone + reset_workflow_keep_step1"
        );
        assert_eq!(
            st.step1.mods_archive_folder, r"D:\BG\ModsArchive",
            "mods_archive_folder survives the import path's step1 handling"
        );
        assert!(
            st.step1.download,
            "download survives reset_workflow_keep_step1 (it keeps step1)"
        );
    }
}
