// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fmt::Write as _;

use crate::app::state::{Step1State, WizardState};

use super::super::log_files::DiagnosticLogGroup;
use super::{AppDataCopySummary, DiagnosticsContext, Tp2LayoutSummary, WriteCheckSummary};

#[path = "diagnostics_text_step2.rs"]
mod step2;

use super::undefined_detect;

macro_rules! push_fmt {
    ($dst:expr, $($arg:tt)*) => {
        write!($dst, $($arg)*).expect("writing to String should not fail")
    };
}

#[derive(Clone, Copy)]
pub(super) struct BuildBaseTextInput<'a> {
    pub state: &'a WizardState,
    pub diagnostics_run_id: &'a str,
    pub log_groups: &'a [DiagnosticLogGroup],
    pub active_order: &'a [String],
    pub console_excerpt: &'a str,
    pub timestamp_unix_secs: u64,
    pub diag_ctx: &'a DiagnosticsContext,
    pub installer_program: &'a str,
    pub installer_args: &'a [String],
    pub appdata_summary: &'a AppDataCopySummary,
    pub write_check_summary: &'a WriteCheckSummary,
    pub tp2_layout_summary: &'a Tp2LayoutSummary,
}

pub(super) fn build_base_text(input: BuildBaseTextInput<'_>) -> String {
    let mut text = String::new();
    text.push_str("BIO Diagnostics\n");
    text.push_str("====================\n\n");
    append_run_metadata(
        &mut text,
        input.diagnostics_run_id,
        input.timestamp_unix_secs,
        input.diag_ctx,
    );
    append_validation_summary(&mut text, input.state);
    append_step1_snapshot(&mut text, input.state);
    append_effective_installer_args(&mut text, input.installer_program, input.installer_args);
    append_installer_invocation_context(&mut text);
    step2::append_step2_sections(&mut text, input.state);
    append_tp2_layout_snapshot(&mut text, input.tp2_layout_summary);
    append_write_checks(&mut text, input.write_check_summary);
    append_appdata_copies(&mut text, input.appdata_summary);
    text.push_str("\n[Step3 Install Order]\n");
    for line in input.active_order {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("\n[Step5 Status]\n");
    push_fmt!(
        text,
        "install_running={}\n",
        input.state.step5.install_running
    );
    push_fmt!(text, "last_status={}\n", input.state.step5.last_status_text);
    push_fmt!(
        text,
        "last_exit_code={:?}\n",
        input.state.step5.last_exit_code
    );
    append_step5_runtime_summary(&mut text, input.state);
    append_weidu_log_groups(&mut text, input.log_groups);
    text.push_str("\n[Console Excerpt]\n");
    text.push_str(input.console_excerpt);
    text.push('\n');
    append_runtime_snapshot(&mut text, input.state, input.console_excerpt);
    append_undefined_string_signals(&mut text, input.console_excerpt);
    text
}

fn append_tp2_layout_snapshot(out: &mut String, summary: &Tp2LayoutSummary) {
    out.push_str("[TP2 Layout Snapshot]\n");
    if summary.lines.is_empty() {
        out.push_str("none\n\n");
        return;
    }
    for line in &summary.lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
}

fn append_weidu_log_groups(out: &mut String, log_groups: &[DiagnosticLogGroup]) {
    out.push_str("\n[WeiDU Logs By Origin]\n");
    if log_groups.is_empty() {
        out.push_str("none\n");
        return;
    }
    for group in log_groups {
        out.push_str(&group.label);
        out.push('\n');
        if group.copied_paths.is_empty() {
            out.push_str("none\n");
        } else {
            for path in &group.copied_paths {
                out.push_str(&path.display().to_string());
                out.push('\n');
            }
        }
        out.push('\n');
    }
}

fn append_runtime_snapshot(out: &mut String, state: &WizardState, console_excerpt: &str) {
    out.push_str("\n[Startup/Runtime Snapshot]\n");
    out.push_str("source=state.step5.console_output\n");
    push_fmt!(
        out,
        "console_buffer_chars={}\n",
        state.step5.console_output.chars().count()
    );
    push_fmt!(
        out,
        "console_excerpt_chars={}\n",
        console_excerpt.chars().count()
    );
    push_fmt!(
        out,
        "bio_full_debug_enabled={}\n",
        state.step1.bio_full_debug
    );
    push_fmt!(
        out,
        "raw_output_log_enabled={}\n",
        state.step1.log_raw_output_dev
    );
    out.push_str(
        "note=This snapshot contains UI/runtime output captured before or during install.\n",
    );
}

fn append_step5_runtime_summary(out: &mut String, state: &WizardState) {
    out.push_str("start_mode=");
    out.push_str(&state.step5.last_start_mode);
    out.push('\n');
    push_fmt!(
        out,
        "resume_available_after_cancel={}\n",
        state.step5.resume_available
    );
    out.push_str("cancel_mode=");
    out.push_str(&state.step5.last_cancel_mode);
    out.push('\n');
    out.push_str("resolved_bg1_game_dir=");
    out.push_str(&state.step5.resolved_bg1_game_dir);
    out.push('\n');
    out.push_str("resolved_bg2_game_dir=");
    out.push_str(&state.step5.resolved_bg2_game_dir);
    out.push('\n');
    out.push_str("resolved_game_dir=");
    out.push_str(&state.step5.resolved_game_dir);
    out.push('\n');
    push_fmt!(out, "prep_ran={}\n", state.step5.prep_ran);
    push_fmt!(out, "prep_used_backup={}\n", state.step5.prep_used_backup);
    out.push_str("prep_backup_paths=");
    if state.step5.prep_backup_paths.is_empty() {
        out.push_str("<none>\n");
    } else {
        out.push_str(&state.step5.prep_backup_paths.join(" | "));
        out.push('\n');
    }
    out.push_str("prep_cleaned_paths=");
    if state.step5.prep_cleaned_paths.is_empty() {
        out.push_str("<none>\n");
    } else {
        out.push_str(&state.step5.prep_cleaned_paths.join(" | "));
        out.push('\n');
    }
}

fn append_write_checks(out: &mut String, summary: &WriteCheckSummary) {
    out.push_str("[Write/Permission Checks]\n");
    for line in &summary.lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
}

fn append_undefined_string_signals(out: &mut String, console_excerpt: &str) {
    out.push_str("\n[Undefined String Signals]\n");
    let mut hits: Vec<&str> = console_excerpt
        .lines()
        .filter(|line| undefined_detect::looks_like_undefined_signal(line))
        .collect();
    if hits.is_empty() {
        out.push_str("none_detected\n");
        return;
    }
    if hits.len() > 120 {
        let keep_from = hits.len().saturating_sub(120);
        hits = hits.split_off(keep_from);
    }
    for line in hits {
        out.push_str(line);
        out.push('\n');
    }
}

fn append_appdata_copies(out: &mut String, summary: &AppDataCopySummary) {
    out.push_str("[Copied AppData Config Files]\n");
    if summary.copied.is_empty() {
        out.push_str("none\n");
    } else {
        for p in &summary.copied {
            push_fmt!(out, "{}\n", p.display());
        }
    }
    out.push('\n');

    out.push_str("[Missing/Notes AppData Config]\n");
    if summary.missing.is_empty() {
        out.push_str("none\n");
    } else {
        for note in &summary.missing {
            push_fmt!(out, "{note}\n");
        }
    }
    out.push('\n');
}

fn append_run_metadata(
    out: &mut String,
    diagnostics_run_id: &str,
    timestamp_unix_secs: u64,
    diag_ctx: &DiagnosticsContext,
) {
    out.push_str("[Run Metadata]\n");
    push_fmt!(out, "app_name={}\n", env!("CARGO_PKG_NAME"));
    push_fmt!(out, "app_version={}\n", env!("CARGO_PKG_VERSION"));
    push_fmt!(out, "timestamp_unix={timestamp_unix_secs}\n");
    push_fmt!(out, "diagnostics_run_id={diagnostics_run_id}\n");
    push_fmt!(out, "os={}\n", std::env::consts::OS);
    push_fmt!(out, "arch={}\n", std::env::consts::ARCH);
    push_fmt!(out, "dev_mode={}\n", diag_ctx.dev_mode);
    push_fmt!(out, "exe_fingerprint={}\n\n", diag_ctx.exe_fingerprint);
}

fn append_validation_summary(out: &mut String, state: &WizardState) {
    out.push_str("[Validation Summary]\n");
    if let Some((ok, msg)) = &state.step1_path_check {
        push_fmt!(out, "step1_path_check_ok={ok}\n");
        push_fmt!(out, "step1_path_check_msg={msg}\n");
    } else {
        out.push_str("step1_path_check_ok=<not_run>\n");
        out.push_str("step1_path_check_msg=<not_run>\n");
    }
    push_fmt!(
        out,
        "step4_save_error_open={}\n",
        state.step4_save_error_open
    );
    push_fmt!(
        out,
        "step4_save_error_text={}\n\n",
        state.step4_save_error_text
    );
}

fn append_step1_snapshot(out: &mut String, state: &WizardState) {
    let s = &state.step1;
    out.push_str("[Step1 Full Snapshot]\n");
    append_step1_general_options(out, s);
    append_step1_paths(out, s);
    append_step1_install_options(out, s);
}

fn append_step1_general_options(out: &mut String, s: &Step1State) {
    push_fmt!(out, "game_install={}\n", s.game_install);
    push_fmt!(out, "have_weidu_logs={}\n", s.have_weidu_logs);
    push_fmt!(out, "rust_log_debug={}\n", s.rust_log_debug);
    push_fmt!(out, "rust_log_trace={}\n", s.rust_log_trace);
    push_fmt!(out, "custom_scan_depth={}\n", s.custom_scan_depth);
    push_fmt!(
        out,
        "timeout_per_mod_enabled={}\n",
        s.timeout_per_mod_enabled
    );
    push_fmt!(
        out,
        "auto_answer_initial_delay_enabled={}\n",
        s.auto_answer_initial_delay_enabled
    );
    push_fmt!(
        out,
        "auto_answer_post_send_delay_enabled={}\n",
        s.auto_answer_post_send_delay_enabled
    );
    push_fmt!(
        out,
        "prompt_required_sound_enabled={}\n",
        s.prompt_required_sound_enabled
    );
    push_fmt!(out, "lookback_enabled={}\n", s.lookback_enabled);
    push_fmt!(out, "bio_full_debug={}\n", s.bio_full_debug);
    push_fmt!(out, "tick_dev_enabled={}\n", s.tick_dev_enabled);
    push_fmt!(out, "log_raw_output_dev={}\n", s.log_raw_output_dev);
    push_fmt!(out, "weidu_log_mode_enabled={}\n", s.weidu_log_mode_enabled);
    push_fmt!(
        out,
        "new_pre_eet_dir_enabled={}\n",
        s.new_pre_eet_dir_enabled
    );
    push_fmt!(out, "new_eet_dir_enabled={}\n", s.new_eet_dir_enabled);
    push_fmt!(
        out,
        "generate_directory_enabled={}\n",
        s.generate_directory_enabled
    );
    push_fmt!(
        out,
        "prepare_target_dirs_before_install={}\n",
        s.prepare_target_dirs_before_install
    );
    push_fmt!(out, "weidu_log_autolog={}\n", s.weidu_log_autolog);
    push_fmt!(out, "weidu_log_logapp={}\n", s.weidu_log_logapp);
    push_fmt!(out, "weidu_log_logextern={}\n", s.weidu_log_logextern);
    push_fmt!(
        out,
        "weidu_log_log_component={}\n",
        s.weidu_log_log_component
    );
}

fn append_step1_paths(out: &mut String, s: &Step1State) {
    push_fmt!(out, "weidu_log_folder={}\n", s.weidu_log_folder);
    push_fmt!(out, "mod_installer_binary={}\n", s.mod_installer_binary);
    push_fmt!(out, "bgee_game_folder={}\n", s.bgee_game_folder);
    push_fmt!(out, "bgee_log_folder={}\n", s.bgee_log_folder);
    push_fmt!(out, "bgee_log_file={}\n", s.bgee_log_file);
    push_fmt!(out, "bg2ee_game_folder={}\n", s.bg2ee_game_folder);
    push_fmt!(out, "bg2ee_log_folder={}\n", s.bg2ee_log_folder);
    push_fmt!(out, "bg2ee_log_file={}\n", s.bg2ee_log_file);
    push_fmt!(out, "eet_bgee_game_folder={}\n", s.eet_bgee_game_folder);
    push_fmt!(out, "eet_bgee_log_folder={}\n", s.eet_bgee_log_folder);
    push_fmt!(out, "eet_bg2ee_game_folder={}\n", s.eet_bg2ee_game_folder);
    push_fmt!(out, "eet_bg2ee_log_folder={}\n", s.eet_bg2ee_log_folder);
    push_fmt!(out, "eet_pre_dir={}\n", s.eet_pre_dir);
    push_fmt!(out, "eet_new_dir={}\n", s.eet_new_dir);
    push_fmt!(out, "game={}\n", s.game);
    push_fmt!(out, "log_file={}\n", s.log_file);
    push_fmt!(out, "generate_directory={}\n", s.generate_directory);
    push_fmt!(out, "mods_folder={}\n", s.mods_folder);
    push_fmt!(out, "weidu_binary={}\n", s.weidu_binary);
}

fn append_step1_install_options(out: &mut String, s: &Step1State) {
    push_fmt!(out, "language={}\n", s.language);
    push_fmt!(out, "depth={}\n", s.depth);
    push_fmt!(out, "skip_installed={}\n", s.skip_installed);
    push_fmt!(out, "abort_on_warnings={}\n", s.abort_on_warnings);
    push_fmt!(out, "timeout={}\n", s.timeout);
    push_fmt!(
        out,
        "auto_answer_initial_delay_ms={}\n",
        s.auto_answer_initial_delay_ms
    );
    push_fmt!(
        out,
        "auto_answer_post_send_delay_ms={}\n",
        s.auto_answer_post_send_delay_ms
    );
    push_fmt!(out, "weidu_log_mode={}\n", s.weidu_log_mode);
    push_fmt!(out, "strict_matching={}\n", s.strict_matching);
    push_fmt!(out, "download={}\n", s.download);
    push_fmt!(out, "download_archive={}\n", s.download_archive);
    push_fmt!(out, "mods_archive_folder={}\n", s.mods_archive_folder);
    push_fmt!(out, "mods_backup_folder={}\n", s.mods_backup_folder);
    push_fmt!(out, "overwrite={}\n", s.overwrite);
    push_fmt!(out, "check_last_installed={}\n", s.check_last_installed);
    push_fmt!(out, "tick={}\n", s.tick);
    push_fmt!(out, "lookback={}\n", s.lookback);
    push_fmt!(out, "casefold={}\n", s.casefold);
    push_fmt!(
        out,
        "backup_targets_before_eet_copy={}\n\n",
        s.backup_targets_before_eet_copy
    );
}

fn append_effective_installer_args(
    out: &mut String,
    installer_program: &str,
    installer_args: &[String],
) {
    out.push_str("[Effective Installer Args]\n");
    push_fmt!(out, "program={installer_program}\n");
    if installer_args.is_empty() {
        out.push_str("args=<none>\n\n");
        return;
    }
    for (idx, arg) in installer_args.iter().enumerate() {
        push_fmt!(out, "arg[{idx}]={arg}\n");
    }
    out.push('\n');
}

fn append_installer_invocation_context(out: &mut String) {
    out.push_str("[Installer Invocation Context]\n");
    match std::env::current_dir() {
        Ok(cwd) => push_fmt!(out, "cwd={}\n", cwd.display()),
        Err(err) => push_fmt!(out, "cwd=<unavailable: {err}>\n"),
    }
    for key in [
        "APPDATA",
        "LOCALAPPDATA",
        "USERPROFILE",
        "HOME",
        "XDG_CONFIG_HOME",
        "LANG",
    ] {
        let value = std::env::var(key).unwrap_or_else(|_| "<unset>".to_string());
        push_fmt!(out, "env[{key}]={value}\n");
    }
    out.push('\n');
}
