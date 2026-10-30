// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use std::sync::{Arc, Mutex};

use chrono::Utc;

use crate::app::game_authority;
use crate::app::game_version::read_game_version;
use crate::app::state::{BasicInstallToolVersionLookup, Step1State, Step5State, WizardState};
use crate::app::terminal::EmbeddedTerminal;

const REPORT_SCHEMA_VERSION: u32 = 1;
const SUPPORT_DIRNAME: &str = "bio_support";
const FAILURE_TAIL_CHARS: usize = 12_000;
const TOOL_VERSION_TIMEOUT: Duration = Duration::from_secs(5);

pub fn begin(state: &mut WizardState, run_id: &str) {
    state.step5.basic_install_tool_versions = None;
    state.step5.basic_install_mod_installer_version = "collecting...".to_string();
    state.step5.basic_install_weidu_version = "collecting...".to_string();
    let Some(path) = report_path(&state.step1, run_id) else {
        state.step5.basic_install_report_path = None;
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    state.step5.basic_install_report_path = Some(path);
    state.step5.basic_install_workspace_case_sensitive =
        filesystem_case_sensitivity(state.step5.basic_install_report_path.as_deref()).to_string();
    state.step5.basic_install_report_state = "starting".to_string();
    start_tool_version_lookup(&mut state.step5, state.step1.weidu_binary.clone());
    write(state, "starting", None);
}

pub fn update(state: &mut WizardState, report_state: &str, terminal: Option<&EmbeddedTerminal>) {
    state.step5.basic_install_report_state = report_state.to_string();
    write(state, report_state, terminal);
}

pub fn poll(state: &mut WizardState, terminal: Option<&EmbeddedTerminal>) {
    let Some(results) = state.step5.basic_install_tool_versions.as_ref() else {
        return;
    };
    let Ok(mut results) = results.0.lock() else {
        return;
    };
    let Some((_, weidu)) = results.take() else {
        return;
    };
    drop(results);
    state.step5.basic_install_weidu_version = weidu;
    state.step5.basic_install_tool_versions = None;
    let report_state = state.step5.basic_install_report_state.clone();
    write(state, &report_state, terminal);
}

fn report_path(step1: &Step1State, run_id: &str) -> Option<PathBuf> {
    install_workspace_root(step1).map(|workspace| {
        workspace
            .join(SUPPORT_DIRNAME)
            .join(format!("install_{run_id}.txt"))
    })
}

fn install_workspace_root(step1: &Step1State) -> Option<PathBuf> {
    let target = match step1.game_install.trim() {
        "EET" => step1.eet_pre_dir.trim(),
        "BGEE" | "BG2EE" | "IWDEE" => step1.generate_directory.trim(),
        _ => "",
    };
    Path::new(target)
        .parent()
        .filter(|_| !target.is_empty())
        .map(Path::to_path_buf)
}

fn write(state: &WizardState, report_state: &str, terminal: Option<&EmbeddedTerminal>) {
    let Some(path) = state.step5.basic_install_report_path.as_ref() else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let _ = fs::write(path, render(state, report_state, terminal));
}

fn render(state: &WizardState, report_state: &str, terminal: Option<&EmbeddedTerminal>) -> String {
    let mut lines = header_lines(report_state, &state.step5);
    lines.extend(modlist_lines(state));
    append_game_source_lines(&mut lines, &state.step1);
    lines.extend(path_lines(&state.step1));
    lines.extend(install_target_lines(&state.step1));
    lines.extend(tool_lines(&state.step1, &state.step5));
    lines.extend(visible_setting_lines(&state.step1));
    lines.extend(result_lines(&state.step5));
    append_failure_output(&mut lines, report_state, terminal);
    lines.join("\n") + "\n"
}

fn header_lines(report_state: &str, step5: &Step5State) -> Vec<String> {
    vec![
        "BIO BASIC INSTALL REPORT".to_string(),
        format!("Report schema: {REPORT_SCHEMA_VERSION}"),
        format!("BIO version: {}", env!("CARGO_PKG_VERSION")),
        format!("Updated (UTC): {}", Utc::now().to_rfc3339()),
        format!("State: {report_state}"),
        String::new(),
        "SYSTEM".to_string(),
        format!("Operating system: {}", std::env::consts::OS),
        format!("Architecture: {}", std::env::consts::ARCH),
        format!(
            "Workspace filesystem case-sensitive: {}",
            step5.basic_install_workspace_case_sensitive
        ),
        String::new(),
    ]
}

fn modlist_lines(state: &WizardState) -> Vec<String> {
    let name = state
        .modlist_share_name
        .as_deref()
        .unwrap_or("not available");
    vec![
        "MODLIST".to_string(),
        format!("Name: {name}"),
        format!("Target game: {}", state.step1.game_install.trim()),
        format!("Install source: {}", state.step5.install_source),
        format!("Install choice: {}", state.step5.install_choice),
        format!("Run: {}", state.step5.last_start_mode),
        format!("Language: {}", state.step1.language.trim()),
        String::new(),
    ]
}

fn append_game_source_lines(lines: &mut Vec<String>, step1: &Step1State) {
    lines.push("GAME SOURCES".to_string());
    for (label, folder) in game_authority::source_game_folders(step1, &step1.game_install) {
        let folder = folder.trim();
        let version = read_game_version(Path::new(folder)).map_or("unknown", |value| value.tag());
        lines.push(format!("{label}: {folder}"));
        lines.push(format!("  Version: {version}"));
    }
    lines.push(String::new());
}

fn path_lines(step1: &Step1State) -> Vec<String> {
    let mut lines = vec![
        "PATHS".to_string(),
        format!("Mods folder: {}", step1.mods_folder.trim()),
        format!("Mods archive: {}", step1.mods_archive_folder.trim()),
        format!("Mods backup: {}", step1.mods_backup_folder.trim()),
    ];
    lines.push(String::new());
    lines
}

fn install_target_lines(step1: &Step1State) -> Vec<String> {
    let mut lines = vec!["INSTALL TARGETS".to_string()];
    for (label, folder) in install_target_folders(step1) {
        lines.push(format!("{label}: {folder}"));
    }
    lines.push(String::new());
    lines
}

fn install_target_folders(step1: &Step1State) -> Vec<(&'static str, &str)> {
    match step1.game_install.trim() {
        "EET" => vec![
            ("BGEE install target", step1.eet_pre_dir.trim()),
            ("BG2EE install target", step1.eet_new_dir.trim()),
        ],
        "BGEE" | "BG2EE" | "IWDEE" => {
            vec![("Install target", step1.generate_directory.trim())]
        }
        _ => Vec::new(),
    }
}

fn tool_lines(step1: &Step1State, step5: &Step5State) -> Vec<String> {
    vec![
        "TOOLS".to_string(),
        format!("WeiDU: {}", step1.weidu_binary.trim()),
        format!("  Version: {}", step5.basic_install_weidu_version),
        format!("Mod Installer: {}", step1.mod_installer_binary.trim()),
        format!("  Version: {}", step5.basic_install_mod_installer_version),
        String::new(),
    ]
}

fn visible_setting_lines(step1: &Step1State) -> Vec<String> {
    vec![
        "INSTALL SETTINGS".to_string(),
        format!("Prompt sound cue: {}", step1.prompt_required_sound_enabled),
        format!("Download missing mods: {}", step1.download),
        format!("Keep downloaded mod archives: {}", step1.download_archive),
        format!("Casefold filename matching: {}", step1.casefold),
        format!("Skip installed: {}", step1.skip_installed),
        format!("Check last installed: {}", step1.check_last_installed),
        format!("Abort on warnings: {}", step1.abort_on_warnings),
        format!("Strict matching: {}", step1.strict_matching),
        format!("Overwrite: {}", step1.overwrite),
        format!("Custom scan depth: {}", step1.custom_scan_depth),
        format!(
            "Mod install timeout enabled: {}",
            step1.timeout_per_mod_enabled
        ),
        format!(
            "Auto-answer initial delay enabled: {}",
            step1.auto_answer_initial_delay_enabled
        ),
        format!(
            "Auto-answer post-send delay enabled: {}",
            step1.auto_answer_post_send_delay_enabled
        ),
        format!(
            "Prompt context lookback enabled: {}",
            step1.lookback_enabled
        ),
        format!("Tick (dev) enabled: {}", step1.tick_dev_enabled),
        format!(
            "Prepare target directories: {}",
            step1.prepare_target_dirs_before_install
        ),
        String::new(),
    ]
}

fn result_lines(step5: &Step5State) -> Vec<String> {
    vec![
        "RESULT".to_string(),
        format!("Status: {}", step5.last_status_text),
        format!("Exit code: {}", option_number(step5.last_exit_code)),
        format!(
            "Runtime seconds: {}",
            option_number(step5.last_runtime_secs)
        ),
        format!("Cancel mode: {}", step5.last_cancel_mode),
    ]
}

fn append_failure_output(
    lines: &mut Vec<String>,
    report_state: &str,
    terminal: Option<&EmbeddedTerminal>,
) {
    if !matches!(
        report_state,
        "blocked"
            | "preflight_failed"
            | "preparation_failed"
            | "start_failed"
            | "failed"
            | "stopped"
    ) {
        return;
    }
    let Some(terminal) = terminal else {
        return;
    };
    let error_block = terminal.extract_error_block();
    if !error_block.trim().is_empty() {
        lines.push(String::new());
        lines.push("ERROR BLOCK".to_string());
        lines.push(error_block);
    }
    let tail = terminal.console_excerpt(FAILURE_TAIL_CHARS);
    if !tail.trim().is_empty() {
        lines.push(String::new());
        lines.push("CONSOLE TAIL".to_string());
        lines.push(tail);
    }
}

fn tool_version(path: &str) -> String {
    let path = path.trim();
    if path.is_empty() {
        return "not configured".to_string();
    }
    if !Path::new(path).is_file() {
        return "tool not found".to_string();
    }
    let mut command = Command::new(path);
    command.arg("--version");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command_version(command, TOOL_VERSION_TIMEOUT)
}

fn command_version(command: Command, timeout: Duration) -> String {
    match crate::app::tool_version_probe::query_command(command, timeout) {
        Ok(output) => {
            let text = if output.stdout.trim().is_empty() {
                &output.stderr
            } else {
                &output.stdout
            };
            text.lines()
                .find(|line| !line.trim().is_empty())
                .map_or_else(
                    || "no version output".to_string(),
                    |line| line.trim().to_string(),
                )
        }
        Err(error) => error.to_string(),
    }
}

fn start_tool_version_lookup(step5: &mut Step5State, weidu_path: String) {
    let results = Arc::new(Mutex::new(None));
    let worker_results = Arc::clone(&results);
    let spawn_result = std::thread::Builder::new()
        .name("bio-tool-version-report".to_string())
        .spawn(move || {
            let weidu = tool_version(&weidu_path);
            if let Ok(mut result) = worker_results.lock() {
                *result = Some((String::new(), weidu));
            }
        });
    match spawn_result {
        Ok(_) => step5.basic_install_tool_versions = Some(BasicInstallToolVersionLookup(results)),
        Err(error) => {
            let message = format!("version lookup failed: {error}");
            step5.basic_install_weidu_version = message;
            step5.basic_install_tool_versions = None;
        }
    }
}

fn filesystem_case_sensitivity(report_path: Option<&Path>) -> &'static str {
    let Some(parent) = report_path.and_then(Path::parent) else {
        return "unknown";
    };
    let lower = parent.join(format!("bio_case_probe_{}", std::process::id()));
    let upper = parent.join(format!("BIO_CASE_PROBE_{}", std::process::id()));
    if fs::write(&lower, b"bio").is_err() {
        return "unknown";
    }
    let insensitive = upper.exists();
    let _ = fs::remove_file(lower);
    if insensitive { "false" } else { "true" }
}

fn option_number(value: Option<impl std::fmt::Display>) -> String {
    value.map_or_else(|| "none".to_string(), |value| value.to_string())
}

pub(crate) fn completion_state(step5: &Step5State) -> &'static str {
    if step5.last_cancel_mode != "none" {
        "stopped"
    } else if step5.last_install_failed || step5.last_exit_code.is_some_and(|code| code != 0) {
        "failed"
    } else {
        "finished"
    }
}

#[cfg(test)]
#[path = "basic_install_report_tests.rs"]
mod tests;
