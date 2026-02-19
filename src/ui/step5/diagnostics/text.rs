// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use crate::ui::state::WizardState;
use crate::ui::step5::diagnostics::{AppDataCopySummary, DiagnosticsContext};

pub(super) fn build_base_text(
    state: &WizardState,
    copied_source_logs: &[PathBuf],
    active_order: &[String],
    console_excerpt: &str,
    timestamp_unix_secs: u64,
    diag_ctx: &DiagnosticsContext,
    installer_program: &str,
    installer_args: &[String],
    appdata_summary: &AppDataCopySummary,
) -> String {
    let mut text = String::new();
    text.push_str("BIO Diagnostics\n");
    text.push_str("====================\n\n");
    append_run_metadata(&mut text, timestamp_unix_secs, diag_ctx);
    append_validation_summary(&mut text, state);
    append_step1_snapshot(&mut text, state);
    append_effective_installer_args(&mut text, installer_program, installer_args);
    append_step2_summary(&mut text, state);
    append_step2_selected_components(&mut text, state);
    append_wlb_inputs_map(&mut text, state);
    append_appdata_copies(&mut text, appdata_summary);
    text.push_str("\n[Step3 Install Order]\n");
    for line in active_order {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("\n[Step5 Status]\n");
    text.push_str(&format!("install_running={}\n", state.step5.install_running));
    text.push_str(&format!("last_status={}\n", state.step5.last_status_text));
    text.push_str(&format!("last_exit_code={:?}\n", state.step5.last_exit_code));
    text.push_str("\n[Copied Source WeiDU Logs]\n");
    if copied_source_logs.is_empty() {
        text.push_str("none\n");
    } else {
        for p in copied_source_logs {
            text.push_str(&p.display().to_string());
            text.push('\n');
        }
    }
    text.push_str("\n[Console Excerpt]\n");
    text.push_str(console_excerpt);
    text.push('\n');
    text
}

fn append_appdata_copies(out: &mut String, summary: &AppDataCopySummary) {
    out.push_str("[Copied AppData Config Files]\n");
    if summary.copied.is_empty() {
        out.push_str("none\n");
    } else {
        for p in &summary.copied {
            out.push_str(&format!("{}\n", p.display()));
        }
    }
    out.push('\n');

    out.push_str("[Missing/Notes AppData Config]\n");
    if summary.missing.is_empty() {
        out.push_str("none\n");
    } else {
        for note in &summary.missing {
            out.push_str(&format!("{note}\n"));
        }
    }
    out.push('\n');
}

fn append_run_metadata(out: &mut String, timestamp_unix_secs: u64, diag_ctx: &DiagnosticsContext) {
    out.push_str("[Run Metadata]\n");
    out.push_str(&format!("app_name={}\n", env!("CARGO_PKG_NAME")));
    out.push_str(&format!("app_version={}\n", env!("CARGO_PKG_VERSION")));
    out.push_str(&format!("timestamp_unix={timestamp_unix_secs}\n"));
    out.push_str(&format!("os={}\n", std::env::consts::OS));
    out.push_str(&format!("arch={}\n", std::env::consts::ARCH));
    out.push_str(&format!("dev_mode={}\n", diag_ctx.dev_mode));
    out.push_str(&format!("exe_fingerprint={}\n\n", diag_ctx.exe_fingerprint));
}

fn append_validation_summary(out: &mut String, state: &WizardState) {
    out.push_str("[Validation Summary]\n");
    match &state.step1_path_check {
        Some((ok, msg)) => {
            out.push_str(&format!("step1_path_check_ok={ok}\n"));
            out.push_str(&format!("step1_path_check_msg={msg}\n"));
        }
        None => {
            out.push_str("step1_path_check_ok=<not_run>\n");
            out.push_str("step1_path_check_msg=<not_run>\n");
        }
    }
    out.push_str(&format!(
        "step4_save_error_open={}\n",
        state.step4_save_error_open
    ));
    out.push_str(&format!(
        "step4_save_error_text={}\n\n",
        state.step4_save_error_text
    ));
}

fn append_step1_snapshot(out: &mut String, state: &WizardState) {
    let s = &state.step1;
    out.push_str("[Step1 Full Snapshot]\n");
    out.push_str(&format!("game_install={}\n", s.game_install));
    out.push_str(&format!("have_weidu_logs={}\n", s.have_weidu_logs));
    out.push_str(&format!("rust_log_debug={}\n", s.rust_log_debug));
    out.push_str(&format!("rust_log_trace={}\n", s.rust_log_trace));
    out.push_str(&format!("custom_scan_depth={}\n", s.custom_scan_depth));
    out.push_str(&format!(
        "timeout_per_mod_enabled={}\n",
        s.timeout_per_mod_enabled
    ));
    out.push_str(&format!(
        "auto_answer_initial_delay_enabled={}\n",
        s.auto_answer_initial_delay_enabled
    ));
    out.push_str(&format!(
        "auto_answer_post_send_delay_enabled={}\n",
        s.auto_answer_post_send_delay_enabled
    ));
    out.push_str(&format!("lookback_enabled={}\n", s.lookback_enabled));
    out.push_str(&format!("bio_full_debug={}\n", s.bio_full_debug));
    out.push_str(&format!("tick_dev_enabled={}\n", s.tick_dev_enabled));
    out.push_str(&format!("log_raw_output_dev={}\n", s.log_raw_output_dev));
    out.push_str(&format!(
        "weidu_log_mode_enabled={}\n",
        s.weidu_log_mode_enabled
    ));
    out.push_str(&format!(
        "new_pre_eet_dir_enabled={}\n",
        s.new_pre_eet_dir_enabled
    ));
    out.push_str(&format!("new_eet_dir_enabled={}\n", s.new_eet_dir_enabled));
    out.push_str(&format!(
        "generate_directory_enabled={}\n",
        s.generate_directory_enabled
    ));
    out.push_str(&format!("weidu_log_autolog={}\n", s.weidu_log_autolog));
    out.push_str(&format!("weidu_log_logapp={}\n", s.weidu_log_logapp));
    out.push_str(&format!("weidu_log_logextern={}\n", s.weidu_log_logextern));
    out.push_str(&format!(
        "weidu_log_log_component={}\n",
        s.weidu_log_log_component
    ));
    out.push_str(&format!("weidu_log_folder={}\n", s.weidu_log_folder));
    out.push_str(&format!("mod_installer_binary={}\n", s.mod_installer_binary));
    out.push_str(&format!("bgee_game_folder={}\n", s.bgee_game_folder));
    out.push_str(&format!("bgee_log_folder={}\n", s.bgee_log_folder));
    out.push_str(&format!("bgee_log_file={}\n", s.bgee_log_file));
    out.push_str(&format!("bg2ee_game_folder={}\n", s.bg2ee_game_folder));
    out.push_str(&format!("bg2ee_log_folder={}\n", s.bg2ee_log_folder));
    out.push_str(&format!("bg2ee_log_file={}\n", s.bg2ee_log_file));
    out.push_str(&format!(
        "eet_bgee_game_folder={}\n",
        s.eet_bgee_game_folder
    ));
    out.push_str(&format!(
        "eet_bgee_log_folder={}\n",
        s.eet_bgee_log_folder
    ));
    out.push_str(&format!(
        "eet_bg2ee_game_folder={}\n",
        s.eet_bg2ee_game_folder
    ));
    out.push_str(&format!(
        "eet_bg2ee_log_folder={}\n",
        s.eet_bg2ee_log_folder
    ));
    out.push_str(&format!("eet_pre_dir={}\n", s.eet_pre_dir));
    out.push_str(&format!("eet_new_dir={}\n", s.eet_new_dir));
    out.push_str(&format!("game={}\n", s.game));
    out.push_str(&format!("log_file={}\n", s.log_file));
    out.push_str(&format!("generate_directory={}\n", s.generate_directory));
    out.push_str(&format!("mods_folder={}\n", s.mods_folder));
    out.push_str(&format!("weidu_binary={}\n", s.weidu_binary));
    out.push_str(&format!("language={}\n", s.language));
    out.push_str(&format!("depth={}\n", s.depth));
    out.push_str(&format!("skip_installed={}\n", s.skip_installed));
    out.push_str(&format!("abort_on_warnings={}\n", s.abort_on_warnings));
    out.push_str(&format!("timeout={}\n", s.timeout));
    out.push_str(&format!(
        "auto_answer_initial_delay_ms={}\n",
        s.auto_answer_initial_delay_ms
    ));
    out.push_str(&format!(
        "auto_answer_post_send_delay_ms={}\n",
        s.auto_answer_post_send_delay_ms
    ));
    out.push_str(&format!("weidu_log_mode={}\n", s.weidu_log_mode));
    out.push_str(&format!("strict_matching={}\n", s.strict_matching));
    out.push_str(&format!("download={}\n", s.download));
    out.push_str(&format!("overwrite={}\n", s.overwrite));
    out.push_str(&format!("check_last_installed={}\n", s.check_last_installed));
    out.push_str(&format!("tick={}\n", s.tick));
    out.push_str(&format!("lookback={}\n", s.lookback));
    out.push_str(&format!("casefold={}\n", s.casefold));
    out.push_str(&format!(
        "backup_targets_before_eet_copy={}\n\n",
        s.backup_targets_before_eet_copy
    ));
}

fn append_effective_installer_args(out: &mut String, installer_program: &str, installer_args: &[String]) {
    out.push_str("[Effective Installer Args]\n");
    out.push_str(&format!("program={installer_program}\n"));
    if installer_args.is_empty() {
        out.push_str("args=<none>\n\n");
        return;
    }
    for (idx, arg) in installer_args.iter().enumerate() {
        out.push_str(&format!("arg[{idx}]={arg}\n"));
    }
    out.push('\n');
}

fn append_step2_summary(out: &mut String, state: &WizardState) {
    out.push_str("[Step2]\n");
    out.push_str(&format!("selected_count={}\n", state.step2.selected_count));
    out.push_str(&format!("total_count={}\n", state.step2.total_count));
    out.push_str(&format!("active_tab={}\n\n", state.step2.active_game_tab));
}

fn append_step2_selected_components(out: &mut String, state: &WizardState) {
    out.push_str("[Step2 Selected Components]\n");
    let mut listed = 0usize;
    for (tab, mods) in [("BGEE", &state.step2.bgee_mods), ("BG2EE", &state.step2.bg2ee_mods)] {
        for mod_state in mods {
            for component in &mod_state.components {
                if !component.checked {
                    continue;
                }
                listed = listed.saturating_add(1);
                out.push_str(&format!(
                    "{tab} | {} #{} | {}\n",
                    mod_state.tp_file, component.component_id, component.label
                ));
            }
        }
    }
    if listed == 0 {
        out.push_str("none\n");
    }
    out.push('\n');
}

fn append_wlb_inputs_map(out: &mut String, state: &WizardState) {
    out.push_str("[@wlb-inputs Map]\n");
    let mut listed = 0usize;
    for (tab, items) in [("BGEE", &state.step3.bgee_items), ("BG2EE", &state.step3.bg2ee_items)] {
        for item in items {
            if item.is_parent {
                continue;
            }
            let Some(inputs) = extract_wlb_inputs(&item.raw_line) else {
                continue;
            };
            listed = listed.saturating_add(1);
            out.push_str(&format!(
                "{tab} | {} #{} | {inputs}\n",
                item.tp_file, item.component_id
            ));
        }
    }
    if listed == 0 {
        out.push_str("none\n");
    }
    out.push('\n');
}

fn extract_wlb_inputs(raw_line: &str) -> Option<String> {
    let marker = "@wlb-inputs:";
    let lower = raw_line.to_ascii_lowercase();
    let start = lower.find(marker)?;
    let tail = raw_line[start + marker.len()..].trim();
    if tail.is_empty() {
        None
    } else {
        Some(tail.to_string())
    }
}
