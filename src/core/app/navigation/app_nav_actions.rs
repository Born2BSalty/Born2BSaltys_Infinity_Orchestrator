// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::app::scan::cache;
use crate::app::state::{Step1State, WizardState};
use crate::app::step2_worker::Step2ScanEvent;
use crate::app::terminal::EmbeddedTerminal;
use crate::settings::store::SettingsStore;

use super::app_lifecycle;
use super::app_nav::{BackAction, NextAction};

pub(crate) struct ResetContext<'a> {
    pub(crate) state: &'a mut WizardState,
    pub(crate) step2_scan_rx: &'a mut Option<Receiver<Step2ScanEvent>>,
    pub(crate) step2_cancel: &'a mut Option<Arc<AtomicBool>>,
    pub(crate) step2_progress_queue: &'a mut VecDeque<(usize, usize, String)>,
    pub(crate) step5_terminal: &'a mut Option<EmbeddedTerminal>,
    pub(crate) settings_store: &'a SettingsStore,
    pub(crate) last_saved_step1: &'a mut Step1State,
    pub(crate) dev_mode: bool,
    pub(crate) exe_fingerprint: &'a str,
}

pub(crate) fn handle_reset(ctx: &mut ResetContext<'_>) {
    super::app_step2_scan::cancel_step2_scan(ctx.state, ctx.step2_cancel.as_ref());
    *ctx.step2_scan_rx = None;
    *ctx.step2_cancel = None;
    ctx.step2_progress_queue.clear();
    if let Some(term) = ctx.step5_terminal.as_mut() {
        term.shutdown();
    }
    let mut reset_warnings = cache::clear_scan_cache_files();
    if let Err(err) = crate::app::mod_update_locks::clear_mod_update_locks() {
        reset_warnings.push(format!("update lock clear failed: {err}"));
    }
    crate::app::step5::prompt_memory::clear_all();
    ctx.state.reset_workflow_keep_step1();
    if !reset_warnings.is_empty() {
        ctx.state.step2.scan_status = format!("Reset warning: {}", reset_warnings.join(" | "));
    }
    app_lifecycle::save_settings_best_effort(
        ctx.state,
        ctx.settings_store,
        ctx.last_saved_step1,
        ctx.dev_mode,
        ctx.exe_fingerprint,
    );
}

pub(crate) fn handle_back(
    state: &mut WizardState,
    settings_store: &SettingsStore,
    last_saved_step1: &mut Step1State,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    let action = super::app_nav::decide_back_action(state);
    if matches!(
        action,
        BackAction::SyncThenGoBack | BackAction::SyncThenReturnToStep1FromLogs
    ) {
        super::app_step2_sync_flow::sync_step2_from_step3(state);
    }
    super::app_nav::apply_back_action(state, action);
    app_lifecycle::save_settings_best_effort(
        state,
        settings_store,
        last_saved_step1,
        dev_mode,
        exe_fingerprint,
    );
}

pub(crate) fn handle_next(
    state: &mut WizardState,
    settings_store: &SettingsStore,
    last_saved_step1: &mut Step1State,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    if super::app_nav::should_show_step1_clean_confirm(state) {
        state.open_step1_clean_confirm();
        return;
    }
    advance_after_next(
        state,
        settings_store,
        last_saved_step1,
        dev_mode,
        exe_fingerprint,
    );
}

pub(crate) fn handle_clean_confirm_yes(
    state: &mut WizardState,
    settings_store: &SettingsStore,
    last_saved_step1: &mut Step1State,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    state.clear_step1_clean_confirm();
    advance_after_next(
        state,
        settings_store,
        last_saved_step1,
        dev_mode,
        exe_fingerprint,
    );
}

pub(crate) const fn handle_clean_confirm_no(state: &mut WizardState) {
    state.clear_step1_clean_confirm();
}

pub(crate) const fn dismiss_step4_save_error(state: &mut WizardState) {
    state.dismiss_step4_save_error();
}

fn advance_after_next(
    state: &mut WizardState,
    settings_store: &SettingsStore,
    last_saved_step1: &mut Step1State,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    let action = super::app_nav::decide_next_action(state);
    match &action {
        NextAction::Blocked => return,
        NextAction::OpenModlistImport
        | NextAction::ApplySavedLogAndAdvance
        | NextAction::JumpToInstallStep
        | NextAction::Advance => {}
        NextAction::SyncStep3AndAdvance { signature } => {
            super::app_step3_sync_flow::sync_step3_from_step2(state);
            state.set_last_step2_sync_signature(signature.clone());
        }
        NextAction::NeedStep4SaveThenAdvance => {
            if let Err(err) = super::step4_weidu_log_export::auto_save_step4_weidu_logs(state) {
                state.record_step4_save_error(format!("Step 4 save failed: {err}"));
                app_lifecycle::save_settings_best_effort(
                    state,
                    settings_store,
                    last_saved_step1,
                    dev_mode,
                    exe_fingerprint,
                );
                return;
            }
        }
    }
    super::app_nav::apply_next_action(state, &action);
    app_lifecycle::save_settings_best_effort(
        state,
        settings_store,
        last_saved_step1,
        dev_mode,
        exe_fingerprint,
    );
}
