// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{ResumeTargets, Step1State, WizardState};
use crate::ui::terminal::EmbeddedTerminal;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PromptGroup {
    pub label: String,
    pub items: Vec<(String, crate::ui::step5::prompt_memory::PromptAnswerEntry)>,
}

pub fn group_prompt_entries(
    entries: Vec<(String, crate::ui::step5::prompt_memory::PromptAnswerEntry)>,
) -> Vec<PromptGroup> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<
        String,
        Vec<(String, crate::ui::step5::prompt_memory::PromptAnswerEntry)>,
    > = BTreeMap::new();
    for (key, entry) in entries {
        let group_label = prompt_component_label(&entry);
        grouped.entry(group_label).or_default().push((key, entry));
    }
    grouped
        .into_iter()
        .map(|(label, items)| PromptGroup { label, items })
        .collect()
}

pub fn prompt_component_label(
    entry: &crate::ui::step5::prompt_memory::PromptAnswerEntry,
) -> String {
    if !entry.component_name.trim().is_empty() {
        entry.component_name.clone()
    } else if !entry.tp2_file.trim().is_empty() && !entry.component_id.trim().is_empty() {
        format!("{} #{}", entry.tp2_file, entry.component_id)
    } else if !entry.component_key.trim().is_empty() {
        entry.component_key.clone()
    } else if !entry.tp2_file.trim().is_empty() {
        entry.tp2_file.clone()
    } else {
        "(unknown component)".to_string()
    }
}

pub fn play_prompt_required_sound_once() {
    use std::io::{IsTerminal, Write};

    if std::io::stdout().is_terminal() {
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(b"\x07");
        let _ = stdout.flush();
        return;
    }

    if play_platform_beep() {
        return;
    }

    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(b"\x07");
    let _ = stdout.flush();
}

#[cfg(target_os = "windows")]
fn play_platform_beep() -> bool {
    unsafe {
        if MessageBeep(0x0000_0040) == 0 {
            let _ = MessageBeep(0xFFFF_FFFF);
        }
    }
    true
}

#[cfg(target_os = "macos")]
fn play_platform_beep() -> bool {
    unsafe {
        NSBeep();
    }
    true
}

#[cfg(all(unix, not(target_os = "macos")))]
fn play_platform_beep() -> bool {
    try_command("canberra-gtk-play", &["-i", "bell"])
        || try_command("paplay", &["/usr/share/sounds/freedesktop/stereo/bell.oga"])
        || try_command("aplay", &["/usr/share/sounds/alsa/Front_Center.wav"])
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn play_platform_beep() -> bool {
    false
}

#[cfg(all(unix, not(target_os = "macos")))]
fn try_command(program: &str, args: &[&str]) -> bool {
    std::process::Command::new(program)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    fn NSBeep();
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBeep(u_type: u32) -> i32;
}

pub fn apply_dev_defaults(state: &mut WizardState, dev_mode: bool) {
    if dev_mode {
        state.step1.bio_full_debug = true;
        state.step1.log_raw_output_dev = true;
    } else {
        state.step1.bio_full_debug = false;
        state.step1.log_raw_output_dev = false;
    }
}

pub fn export_diagnostics(
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> anyhow::Result<PathBuf> {
    if !dev_mode {
        anyhow::bail!("Diagnostics export is only available in -d mode");
    }
    let ctx = crate::ui::step5::diagnostics::DiagnosticsContext {
        dev_mode,
        exe_fingerprint: exe_fingerprint.to_string(),
    };
    crate::ui::step5::diagnostics::export_diagnostics(state, terminal, &ctx)
}

pub fn copy_source_weidu_logs(step1: &Step1State, out_dir: &Path, suffix: &str) -> Vec<PathBuf> {
    crate::ui::step5::log_files::copy_source_weidu_logs(step1, out_dir, suffix)
}

pub fn copy_saved_weidu_logs(step1: &Step1State, out_dir: &Path, suffix: &str) -> Vec<PathBuf> {
    crate::ui::step5::log_files::copy_saved_weidu_logs(step1, out_dir, suffix)
}

pub fn source_log_infos(step1: &Step1State) -> Vec<crate::ui::step5::log_files::SourceLogInfo> {
    crate::ui::step5::log_files::source_log_infos(step1)
}

pub fn save_console_log(state: &WizardState, console_text: &str) -> anyhow::Result<PathBuf> {
    Ok(crate::ui::step5::log_files::save_console_log(
        &state.step5,
        console_text,
    )?)
}

pub fn open_console_logs_folder() -> anyhow::Result<()> {
    Ok(crate::ui::step5::log_files::open_console_logs_folder()?)
}

pub fn open_last_log_file(step1: &Step1State) -> anyhow::Result<()> {
    Ok(crate::ui::step5::log_files::open_last_log_file(step1)?)
}

pub fn build_install_invocation(step1: &Step1State) -> (String, Vec<String>) {
    step5_command::build_install_invocation(step1)
}

pub fn capture_resume_targets(step1: &Step1State) -> ResumeTargets {
    step5_command::capture_resume_targets(step1)
}

pub fn build_resume_invocation(
    step1: &Step1State,
    resume_targets: &ResumeTargets,
) -> (String, Vec<String>) {
    step5_command::build_resume_invocation(step1, resume_targets)
}

pub fn build_install_command(step1: &Step1State) -> String {
    step5_command::build_install_command(step1)
}

pub fn build_command_preview_lines(step1: &Step1State) -> Vec<String> {
    step5_command::build_command_preview_lines(step1)
}

pub fn wrap_display_line(line: &str, max_chars: usize) -> Vec<String> {
    step5_command::wrap_display_line(line, max_chars)
}

pub fn prepare_target_dirs_before_install(
    step1: &Step1State,
) -> anyhow::Result<crate::ui::step5::log_files::TargetPrepResult> {
    Ok(crate::ui::step5::log_files::prepare_target_dirs_before_install(step1)?)
}

pub fn validate_resume_paths(
    step1: &Step1State,
    resume_targets: &ResumeTargets,
) -> anyhow::Result<()> {
    crate::ui::step5::log_files::validate_resume_paths(step1, resume_targets)
        .map_err(anyhow::Error::msg)
}

pub fn validate_runtime_prep_paths(step1: &Step1State) -> anyhow::Result<()> {
    crate::ui::step5::log_files::validate_runtime_prep_paths(step1).map_err(anyhow::Error::msg)
}

pub fn verify_targets_prepared(step1: &Step1State) -> anyhow::Result<()> {
    crate::ui::step5::log_files::verify_targets_prepared(step1).map_err(anyhow::Error::msg)
}

pub fn load_scripted_inputs(step1: &Step1State) -> HashMap<String, Vec<String>> {
    crate::ui::step5::scripted_inputs::load_from_step1(step1)
}

pub fn begin_diagnostics_run(state: &mut WizardState) -> String {
    crate::ui::step5::service_diagnostics_run_step5::begin_new_run(&mut state.step5)
}

pub fn copy_weidu_logs_for_diagnostics(step1: &Step1State, run_id: &str) {
    if !step1.bio_full_debug && !step1.log_raw_output_dev {
        return;
    }
    let run_dir = crate::ui::step5::service_diagnostics_run_step5::run_dir_from_id(run_id);
    let source_logs_dir = run_dir.join("source_logs");
    let _ = copy_source_weidu_logs(step1, &source_logs_dir, "original");
    let saved_logs_dir = run_dir.join("saved_logs");
    let _ = copy_saved_weidu_logs(step1, &saved_logs_dir, "original");
}

mod step5_command {
    pub(crate) use crate::ui::step5::service_step5_command_step5::*;
}

pub(crate) mod auto_answer {
    pub(crate) use crate::ui::step5::service_auto_answer_step5::*;
}

pub(crate) mod json_fallback {
    pub(crate) use crate::ui::step5::service_json_fallback_step5::*;
}

pub(crate) mod readiness {
    pub(crate) use crate::ui::step5::service_readiness_step5::*;
}

pub(crate) mod scripted {
    pub(crate) use crate::ui::step5::service_scripted_step5::*;
}

pub(crate) mod process_line {
    pub(crate) use crate::ui::step5::service_process_line_step5::*;
}

pub(crate) mod install_flow {
    pub(crate) use crate::ui::step5::service_install_flow_step5::*;
}
