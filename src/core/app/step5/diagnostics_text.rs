// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fmt::Write as _;

use crate::app::state::WizardState;

use super::super::log_files::DiagnosticLogGroup;
use super::{AppDataCopySummary, DiagnosticsContext, Tp2LayoutSummary, WriteCheckSummary};

#[path = "diagnostics_text_step2.rs"]
mod step2;

use super::undefined_detect;

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

pub(super) fn build_base_text(input: &BuildBaseTextInput<'_>) -> String {
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
    let _ = writeln!(
        text,
        "install_running={}",
        input.state.step5.install_running
    );
    let _ = writeln!(text, "last_status={}", input.state.step5.last_status_text);
    let _ = writeln!(
        text,
        "last_exit_code={:?}",
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
    let _ = writeln!(
        out,
        "console_buffer_chars={}",
        state.step5.console_output.chars().count()
    );
    let _ = writeln!(
        out,
        "console_excerpt_chars={}",
        console_excerpt.chars().count()
    );
    let _ = writeln!(out, "bio_full_debug_enabled={}", state.step1.bio_full_debug);
    let _ = writeln!(
        out,
        "raw_output_log_enabled={}",
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
    let _ = writeln!(
        out,
        "resume_available_after_cancel={}",
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
    let _ = writeln!(out, "prep_ran={}", state.step5.prep_ran);
    let _ = writeln!(out, "prep_used_backup={}", state.step5.prep_used_backup);
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
            let _ = writeln!(out, "{}", p.display());
        }
    }
    out.push('\n');

    out.push_str("[Missing/Notes AppData Config]\n");
    if summary.missing.is_empty() {
        out.push_str("none\n");
    } else {
        for note in &summary.missing {
            let _ = writeln!(out, "{note}");
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
    let _ = writeln!(out, "app_name={}", env!("CARGO_PKG_NAME"));
    let _ = writeln!(out, "app_version={}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(out, "timestamp_unix={timestamp_unix_secs}");
    let _ = writeln!(out, "diagnostics_run_id={diagnostics_run_id}");
    let _ = writeln!(out, "os={}", std::env::consts::OS);
    let _ = writeln!(out, "arch={}", std::env::consts::ARCH);
    let _ = writeln!(out, "dev_mode={}", diag_ctx.dev_mode);
    let _ = writeln!(out, "exe_fingerprint={}\n", diag_ctx.exe_fingerprint);
}

fn append_validation_summary(out: &mut String, state: &WizardState) {
    out.push_str("[Validation Summary]\n");
    if let Some((ok, msg)) = &state.step1_path_check {
        let _ = writeln!(out, "step1_path_check_ok={ok}");
        let _ = writeln!(out, "step1_path_check_msg={msg}");
    } else {
        out.push_str("step1_path_check_ok=<not_run>\n");
        out.push_str("step1_path_check_msg=<not_run>\n");
    }
    let _ = writeln!(out, "step4_save_error_open={}", state.step4_save_error_open);
    let _ = writeln!(
        out,
        "step4_save_error_text={}\n",
        state.step4_save_error_text
    );
}

fn append_step1_snapshot(out: &mut String, state: &WizardState) {
    let s = &state.step1;
    out.push_str("[Step1 Full Snapshot]\n");
    let _ = writeln!(out, "game_install={}", s.game_install);
    let _ = writeln!(out, "have_weidu_logs={}", s.have_weidu_logs);
    let _ = writeln!(out, "rust_log_debug={}", s.rust_log_debug);
    let _ = writeln!(out, "rust_log_trace={}", s.rust_log_trace);
    let _ = writeln!(out, "custom_scan_depth={}", s.custom_scan_depth);
    let _ = writeln!(out, "timeout_per_mod_enabled={}", s.timeout_per_mod_enabled);
    let _ = writeln!(
        out,
        "auto_answer_initial_delay_enabled={}",
        s.auto_answer_initial_delay_enabled
    );
    let _ = writeln!(
        out,
        "auto_answer_post_send_delay_enabled={}",
        s.auto_answer_post_send_delay_enabled
    );
    let _ = writeln!(
        out,
        "prompt_required_sound_enabled={}",
        s.prompt_required_sound_enabled
    );
    let _ = writeln!(out, "lookback_enabled={}", s.lookback_enabled);
    let _ = writeln!(out, "bio_full_debug={}", s.bio_full_debug);
    let _ = writeln!(out, "tick_dev_enabled={}", s.tick_dev_enabled);
    let _ = writeln!(out, "log_raw_output_dev={}", s.log_raw_output_dev);
    let _ = writeln!(out, "weidu_log_mode_enabled={}", s.weidu_log_mode_enabled);
    let _ = writeln!(out, "new_pre_eet_dir_enabled={}", s.new_pre_eet_dir_enabled);
    let _ = writeln!(out, "new_eet_dir_enabled={}", s.new_eet_dir_enabled);
    let _ = writeln!(
        out,
        "generate_directory_enabled={}",
        s.generate_directory_enabled
    );
    let _ = writeln!(
        out,
        "prepare_target_dirs_before_install={}",
        s.prepare_target_dirs_before_install
    );
    let _ = writeln!(out, "weidu_log_autolog={}", s.weidu_log_autolog);
    let _ = writeln!(out, "weidu_log_logapp={}", s.weidu_log_logapp);
    let _ = writeln!(out, "weidu_log_logextern={}", s.weidu_log_logextern);
    let _ = writeln!(out, "weidu_log_log_component={}", s.weidu_log_log_component);
    let _ = writeln!(out, "weidu_log_folder={}", s.weidu_log_folder);
    let _ = writeln!(out, "mod_installer_binary={}", s.mod_installer_binary);
    let _ = writeln!(out, "bgee_game_folder={}", s.bgee_game_folder);
    let _ = writeln!(out, "bgee_log_folder={}", s.bgee_log_folder);
    let _ = writeln!(out, "bgee_log_file={}", s.bgee_log_file);
    let _ = writeln!(out, "bg2ee_game_folder={}", s.bg2ee_game_folder);
    let _ = writeln!(out, "bg2ee_log_folder={}", s.bg2ee_log_folder);
    let _ = writeln!(out, "bg2ee_log_file={}", s.bg2ee_log_file);
    let _ = writeln!(out, "eet_bgee_game_folder={}", s.eet_bgee_game_folder);
    let _ = writeln!(out, "eet_bgee_log_folder={}", s.eet_bgee_log_folder);
    let _ = writeln!(out, "eet_bg2ee_game_folder={}", s.eet_bg2ee_game_folder);
    let _ = writeln!(out, "eet_bg2ee_log_folder={}", s.eet_bg2ee_log_folder);
    let _ = writeln!(out, "eet_pre_dir={}", s.eet_pre_dir);
    let _ = writeln!(out, "eet_new_dir={}", s.eet_new_dir);
    let _ = writeln!(out, "game={}", s.game);
    let _ = writeln!(out, "log_file={}", s.log_file);
    let _ = writeln!(out, "generate_directory={}", s.generate_directory);
    let _ = writeln!(out, "mods_folder={}", s.mods_folder);
    let _ = writeln!(out, "weidu_binary={}", s.weidu_binary);
    let _ = writeln!(out, "language={}", s.language);
    let _ = writeln!(out, "depth={}", s.depth);
    let _ = writeln!(out, "skip_installed={}", s.skip_installed);
    let _ = writeln!(out, "abort_on_warnings={}", s.abort_on_warnings);
    let _ = writeln!(out, "timeout={}", s.timeout);
    let _ = writeln!(
        out,
        "auto_answer_initial_delay_ms={}",
        s.auto_answer_initial_delay_ms
    );
    let _ = writeln!(
        out,
        "auto_answer_post_send_delay_ms={}",
        s.auto_answer_post_send_delay_ms
    );
    let _ = writeln!(out, "weidu_log_mode={}", s.weidu_log_mode);
    let _ = writeln!(out, "strict_matching={}", s.strict_matching);
    let _ = writeln!(out, "download={}", s.download);
    let _ = writeln!(out, "download_archive={}", s.download_archive);
    let _ = writeln!(out, "mods_archive_folder={}", s.mods_archive_folder);
    let _ = writeln!(out, "mods_backup_folder={}", s.mods_backup_folder);
    let _ = writeln!(out, "overwrite={}", s.overwrite);
    let _ = writeln!(out, "check_last_installed={}", s.check_last_installed);
    let _ = writeln!(out, "tick={}", s.tick);
    let _ = writeln!(out, "lookback={}", s.lookback);
    let _ = writeln!(out, "casefold={}", s.casefold);
    let _ = writeln!(
        out,
        "backup_targets_before_eet_copy={}\n",
        s.backup_targets_before_eet_copy
    );
}

fn append_effective_installer_args(
    out: &mut String,
    installer_program: &str,
    installer_args: &[String],
) {
    out.push_str("[Effective Installer Args]\n");
    let _ = writeln!(out, "program={installer_program}");
    if installer_args.is_empty() {
        out.push_str("args=<none>\n\n");
        return;
    }
    for (idx, arg) in installer_args.iter().enumerate() {
        let _ = writeln!(out, "arg[{idx}]={arg}");
    }
    out.push('\n');
}

fn append_installer_invocation_context(out: &mut String) {
    out.push_str("[Installer Invocation Context]\n");
    match std::env::current_dir() {
        Ok(cwd) => {
            let _ = writeln!(out, "cwd={}", cwd.display());
        }
        Err(err) => {
            let _ = writeln!(out, "cwd=<unavailable: {err}>");
        }
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
        let _ = writeln!(out, "env[{key}]={value}");
    }
    out.push('\n');
}
