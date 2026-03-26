// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::WizardApp;

mod dispatch {
use eframe::egui;

use crate::ui::{step1, step2, step3, step4, step5};

use super::super::WizardApp;

pub(super) fn render_current_step(app: &mut WizardApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| match app.state.current_step {
        0 => step1::page_step1::render(ui, &mut app.state, app.dev_mode, app.exe_fingerprint.as_str()),
        1 => {
            if let Some(action) =
                step2::page_step2::render(ui, &mut app.state, app.dev_mode, app.exe_fingerprint.as_str())
            {
                app.handle_step2_action(action);
            }
            app.revalidate_compat_step2_checked_order();
        }
        2 => {
            if let Some(action) =
                step3::page_step3::render(ui, &mut app.state, app.dev_mode, app.exe_fingerprint.as_str())
            {
                handle_step3_action(app, action);
            }
        }
        3 => {
            if let Some(action) =
                step4::page_step4::render(ui, &mut app.state, app.dev_mode, app.exe_fingerprint.as_str())
            {
                app.handle_step4_action(ctx, action);
            }
        }
        4 => {
            if let Some(action) = step5::page_step5::render(
                ui,
                &mut app.state,
                app.step5_terminal.as_mut(),
                app.step5_terminal_error.as_deref(),
                app.dev_mode,
                app.exe_fingerprint.as_str(),
            ) {
                handle_step5_action(app, action);
            }
        }
        _ => {}
    });
}

fn handle_step3_action(app: &mut WizardApp, action: crate::ui::step3::action_step3::Step3Action) {
    match action {
        crate::ui::step3::action_step3::Step3Action::Revalidate => {
            app.revalidate_compat();
        }
    }
}

fn handle_step5_action(app: &mut WizardApp, action: step5::action_step5::Step5Action) {
    match action {
        step5::action_step5::Step5Action::CheckCompatBeforeInstall => {
            if app.check_compat_before_install() {
                app.state.compat.show_pre_install_modal = false;
                app.state.step5.start_install_requested = true;
            } else {
                app.state.step5.last_status_text =
                    "Blocking compatibility errors found. Fix them on Step 3.".to_string();
            }
        }
    }
}
}
mod repaint {
use eframe::egui;
use std::time::Duration;

use super::super::WizardApp;

pub(super) fn request_if_needed(app: &WizardApp, ctx: &egui::Context) {
    if app.step2_scan_rx.is_some()
        || !app.step2_progress_queue.is_empty()
        || app
            .step5_terminal
            .as_ref()
            .map(|t| t.has_new_data())
            .unwrap_or(false)
        || app.state.step5.install_running
    {
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
}
mod terminal {
use eframe::egui;
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::WizardApp;

pub(super) fn poll_step5_terminal(app: &mut WizardApp, ctx: &egui::Context) {
    app.ensure_step5_terminal(ctx);
    if let Some(term) = app.step5_terminal.as_mut() {
        term.poll_output();
        if term.has_new_data() {
            ctx.request_repaint();
        }
        process_graceful_cancel(&mut app.state.step5, term);
        process_exit_event(&mut app.state.step5, term);
    }
}

fn process_graceful_cancel(
    step5: &mut crate::ui::state::Step5State,
    term: &mut crate::ui::terminal::EmbeddedTerminal,
) {
    if !(step5.install_running && step5.cancel_pending) {
        return;
    }

    if let Some(start) = step5.cancel_pending_output_len
        && start > term.output_len()
    {
        // Output buffer rotated; reset anchor so graceful cancel does not stall forever.
        step5.cancel_pending_output_len = Some(0);
    }
    let boundary_counter = term.boundary_event_count();
    let last_seen = step5.cancel_pending_boundary_count.unwrap_or(boundary_counter);
    let boundary = boundary_counter > last_seen;
    if boundary {
        step5.cancel_pending_boundary_count = Some(boundary_counter);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let can_retry = step5
            .cancel_signal_sent_unix_secs
            .map(|last| now.saturating_sub(last) >= 1)
            .unwrap_or(true);
        if can_retry {
            term.graceful_terminate();
            step5.cancel_was_graceful = true;
            step5.last_cancel_mode = "graceful".to_string();
            step5.cancel_signal_sent_unix_secs = Some(now);
            step5.cancel_attempt_count = step5.cancel_attempt_count.saturating_add(1);
            step5.cancel_pending_output_len = Some(term.output_len());
            step5.cancel_pending_boundary_count = Some(boundary_counter);
            step5.last_status_text = format!(
                "Graceful cancel signal sent at SUCCESSFULLY INSTALLED (attempt #{})",
                step5.cancel_attempt_count
            );
        }
    } else {
        step5.last_status_text = if step5.cancel_attempt_count == 0 {
            "Graceful pending: waiting for SUCCESSFULLY INSTALLED boundary".to_string()
        } else {
            format!(
                "Graceful pending: waiting next boundary (attempts={})",
                step5.cancel_attempt_count
            )
        };
    }
}

fn process_exit_event(
    step5: &mut crate::ui::state::Step5State,
    term: &mut crate::ui::terminal::EmbeddedTerminal,
) {
    if !term.take_exit_event() {
        return;
    }

    let finished_exit = term.take_exit_code();
    if let Some(start) = step5.install_started_unix_secs.take() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        step5.last_runtime_secs = Some(now.saturating_sub(start));
    }
    step5.last_install_failed = term.likely_failure_visible();
    step5.last_exit_code = finished_exit;
    step5.install_running = false;
    step5.cancel_requested = false;
    step5.cancel_pending = false;
    step5.cancel_pending_started_unix_secs = None;
    step5.cancel_pending_output_len = None;
    step5.cancel_pending_boundary_count = None;
    step5.cancel_signal_sent_unix_secs = None;
    step5.cancel_attempt_count = 0;
    step5.resume_available = step5.cancel_was_graceful;
    if !step5.resume_available {
        step5.resume_targets = crate::ui::state::ResumeTargets::default();
    }
    step5.cancel_was_graceful = false;
    step5.last_scripted_skip_signature = None;
    step5.prompt_ready_signature = None;
    step5.prompt_ready_seen_count = 0;
    step5.prompt_ready_first_seen_unix_ms = None;
    step5.prompt_required_sound_latched = false;
    if let Some(run_id) = step5.active_run_id.take() {
        let suffix = match finished_exit {
            Some(code) => format!(" (exit {code})"),
            None => String::new(),
        };
        term.append_marker(&format!("Run #{run_id} finished{suffix}"));
    }
    step5.last_status_text = if step5.last_install_failed {
        match step5.last_exit_code {
            Some(code) => format!("Install failed (exit {code})"),
            None => "Install failed".to_string(),
        }
    } else {
        match step5.last_exit_code {
            Some(0) => "Install finished (exit 0)".to_string(),
            Some(code) => format!("Install finished (exit {code})"),
            None => "Install finished".to_string(),
        }
    };
}
}

pub(super) fn run(app: &mut WizardApp, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    if ctx.input(|i| i.viewport().close_requested()) {
        app.shutdown_and_exit();
    }

    app.poll_step2_scan_events();
    terminal::poll_step5_terminal(app, ctx);
    dispatch::render_current_step(app, ctx);
    super::nav_ui::render_nav_buttons(app, ctx);

    if app.state.step1 != app.last_saved_step1 {
        app.save_settings_best_effort();
    }
    repaint::request_if_needed(app, ctx);
}
