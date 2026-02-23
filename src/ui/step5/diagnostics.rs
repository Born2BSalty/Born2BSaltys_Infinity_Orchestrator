// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod compat_snapshot;
mod format;
mod text;

use std::fs;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use chrono::Local;

use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};
use crate::platform_defaults::app_config_dir;
use crate::ui::state::WizardState;
use crate::ui::step4::format::build_weidu_export_lines;
use crate::ui::step5::command::build_install_invocation;
use crate::ui::step5::log_files::copy_source_weidu_logs;
use crate::ui::terminal::EmbeddedTerminal;

#[derive(Debug, Clone)]
pub struct DiagnosticsContext {
    pub dev_mode: bool,
    pub exe_fingerprint: String,
}

#[derive(Debug, Default, Clone)]
pub struct AppDataCopySummary {
    pub copied: Vec<PathBuf>,
    pub missing: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct WriteCheckSummary {
    pub lines: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Tp2LayoutSummary {
    pub lines: Vec<String>,
}

pub fn export_diagnostics(
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    ctx: &DiagnosticsContext,
) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let root_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&root_dir)?;
    let run_stamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let run_dir = root_dir.join(format!("run_{run_stamp}"));
    fs::create_dir_all(&run_dir)?;
    let out_path = run_dir.join("bio_diag.txt");

    let source_logs_dir = run_dir.join("source_logs");
    let copied_source_logs = copy_source_weidu_logs(&state.step1, &source_logs_dir, "source");
    let appdata_summary = copy_appdata_configs(&run_dir);
    let write_check_summary = run_write_checks(state, ts);
    let tp2_layout_summary = build_tp2_layout_summary(state);
    let active_order = if state.step3.active_game_tab == "BG2EE" {
        build_weidu_export_lines(&state.step3.bg2ee_items)
    } else {
        build_weidu_export_lines(&state.step3.bgee_items)
    };
    let console_excerpt = terminal
        .map(|t| t.console_excerpt(40_000))
        .unwrap_or_else(|| fallback_console_excerpt(state, 40_000));
    let (installer_program, installer_args) = build_install_invocation(&state.step1);

    let mut text = text::build_base_text(
        state,
        &copied_source_logs,
        &active_order,
        &console_excerpt,
        ts,
        ctx,
        &installer_program,
        &installer_args,
        &appdata_summary,
        &write_check_summary,
        &tp2_layout_summary,
    );
    if ctx.dev_mode {
        compat_snapshot::append_dev_compat_snapshots(state, &mut text);
    }

    fs::write(&out_path, text)?;
    Ok(out_path)
}

fn run_write_checks(state: &WizardState, ts: u64) -> WriteCheckSummary {
    let mut summary = WriteCheckSummary::default();
    let s = &state.step1;
    let mut seen: HashSet<String> = HashSet::new();

    push_dir_check("Mods Folder", &s.mods_folder, ts, &mut seen, &mut summary);
    push_dir_check(
        "Generate Directory",
        &s.generate_directory,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "WeiDU log folder",
        &s.weidu_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "BGEE game folder",
        &s.bgee_game_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "BG2EE game folder",
        &s.bg2ee_game_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET BGEE game folder",
        &s.eet_bgee_game_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET BG2EE game folder",
        &s.eet_bg2ee_game_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "BGEE log folder",
        &s.bgee_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "BG2EE log folder",
        &s.bg2ee_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET BGEE log folder",
        &s.eet_bgee_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET BG2EE log folder",
        &s.eet_bg2ee_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET pre directory",
        &s.eet_pre_dir,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET new directory",
        &s.eet_new_dir,
        ts,
        &mut seen,
        &mut summary,
    );

    let weidu_binary = resolve_weidu_binary(&s.weidu_binary);
    push_parent_check("WeiDU binary", &weidu_binary, ts, &mut seen, &mut summary);
    let mod_installer_binary = resolve_mod_installer_binary(&s.mod_installer_binary);
    push_parent_check(
        "mod_installer binary",
        &mod_installer_binary,
        ts,
        &mut seen,
        &mut summary,
    );
    push_parent_check("WeiDU log file", &s.log_file, ts, &mut seen, &mut summary);
    push_parent_check(
        "BGEE WeiDU log file",
        &s.bgee_log_file,
        ts,
        &mut seen,
        &mut summary,
    );
    push_parent_check(
        "BG2EE WeiDU log file",
        &s.bg2ee_log_file,
        ts,
        &mut seen,
        &mut summary,
    );

    if summary.lines.is_empty() {
        summary
            .lines
            .push("INFO | write checks | no configured paths to test".to_string());
    }
    summary
}

fn build_tp2_layout_summary(state: &WizardState) -> Tp2LayoutSummary {
    let mut summary = Tp2LayoutSummary::default();
    let mods_root_raw = state.step1.mods_folder.trim();
    let mods_root = if mods_root_raw.is_empty() {
        None
    } else {
        Some(PathBuf::from(mods_root_raw))
    };

    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut selected_tp2_paths: Vec<PathBuf> = Vec::new();
    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        let selected = mod_state.checked || mod_state.components.iter().any(|c| c.checked);
        if !selected {
            continue;
        }
        let tp2 = mod_state.tp2_path.trim();
        if tp2.is_empty() {
            continue;
        }
        let key = tp2.to_ascii_lowercase();
        if !seen_paths.insert(key) {
            continue;
        }
        selected_tp2_paths.push(PathBuf::from(tp2));
    }

    if selected_tp2_paths.is_empty() {
        summary
            .lines
            .push("no selected TP2 paths from Step2".to_string());
        return summary;
    }

    selected_tp2_paths.sort();
    let limit = selected_tp2_paths.len().min(120);
    for tp2_path in selected_tp2_paths.into_iter().take(limit) {
        append_tp2_layout_lines(&mut summary.lines, &tp2_path, mods_root.as_deref());
    }
    if limit >= 120 {
        summary.lines.push("... truncated after 120 selected TP2 entries".to_string());
    }
    summary
}

fn append_tp2_layout_lines(lines: &mut Vec<String>, tp2_path: &std::path::Path, mods_root: Option<&std::path::Path>) {
    lines.push(format!("tp2={}", tp2_path.display()));
    lines.push(format!("tp2_exists={}", tp2_path.is_file()));

    if let Some(root) = mods_root {
        match tp2_path.strip_prefix(root) {
            Ok(rel) => {
                lines.push(format!("relative_from_mods={}", rel.display()));
                lines.push(format!("relative_depth={}", rel.components().count()));
            }
            Err(_) => {
                lines.push("relative_from_mods=<outside mods folder>".to_string());
            }
        }
    } else {
        lines.push("relative_from_mods=<mods folder not set>".to_string());
    }

    let Some(parent) = tp2_path.parent() else {
        lines.push("parent=<none>".to_string());
        lines.push(String::new());
        return;
    };
    lines.push(format!("parent={}", parent.display()));
    lines.push(format!("parent_exists={}", parent.is_dir()));

    let parent_entries = list_parent_entries(parent, 24);
    lines.push(format!(
        "parent_entries={}",
        if parent_entries.is_empty() {
            "<none>".to_string()
        } else {
            parent_entries.join(", ")
        }
    ));

    let hint_dirs = collect_hint_dirs(parent, 2, 30);
    lines.push(format!(
        "nearby_hint_dirs={}",
        if hint_dirs.is_empty() {
            "<none>".to_string()
        } else {
            hint_dirs.join(", ")
        }
    ));

    let tra_count = count_ext_files_limited(parent, "tra", 3, 5000);
    lines.push(format!("nearby_tra_file_count={tra_count}"));
    lines.push(String::new());
}

fn list_parent_entries(parent: &std::path::Path, max_items: usize) -> Vec<String> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let file_type = e.file_type().ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            if file_type.is_dir() {
                Some(format!("[D]{name}"))
            } else if file_type.is_file() {
                Some(format!("[F]{name}"))
            } else {
                None
            }
        })
        .collect();
    names.sort_by_key(|v| v.to_ascii_lowercase());
    names.into_iter().take(max_items).collect()
}

fn collect_hint_dirs(dir: &std::path::Path, depth: usize, max_items: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    collect_hint_dirs_recursive(dir, depth, max_items, &mut out);
    out.sort_by_key(|v| v.to_ascii_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    out
}

fn collect_hint_dirs_recursive(
    dir: &std::path::Path,
    depth: usize,
    max_items: usize,
    out: &mut Vec<String>,
) {
    if depth == 0 || out.len() >= max_items {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= max_items {
            break;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "tra" | "lang" | "language" | "languages" | "lib" | "tpa" | "tph"
        ) {
            out.push(entry.path().display().to_string());
        }
        collect_hint_dirs_recursive(&entry.path(), depth.saturating_sub(1), max_items, out);
    }
}

fn count_ext_files_limited(
    dir: &std::path::Path,
    ext: &str,
    depth: usize,
    hard_limit: usize,
) -> usize {
    let mut total = 0usize;
    count_ext_files_recursive(dir, ext, depth, hard_limit, &mut total);
    total
}

fn count_ext_files_recursive(
    dir: &std::path::Path,
    ext: &str,
    depth: usize,
    hard_limit: usize,
    total: &mut usize,
) {
    if depth == 0 || *total >= hard_limit {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if *total >= hard_limit {
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            count_ext_files_recursive(&path, ext, depth.saturating_sub(1), hard_limit, total);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let is_match = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false);
        if is_match {
            *total = total.saturating_add(1);
        }
    }
}

fn push_dir_check(
    label: &str,
    raw: &str,
    ts: u64,
    seen: &mut HashSet<String>,
    summary: &mut WriteCheckSummary,
) {
    let value = raw.trim();
    if value.is_empty() {
        return;
    }
    let key = format!("dir:{value}");
    if !seen.insert(key) {
        return;
    }
    let path = PathBuf::from(value);
    let line = match probe_write_dir(&path, ts) {
        Ok(()) => format!("OK | {label} | {}", path.display()),
        Err(err) => format!("FAIL | {label} | {} | {err}", path.display()),
    };
    summary.lines.push(line);
}

fn push_parent_check(
    label: &str,
    raw: &str,
    ts: u64,
    seen: &mut HashSet<String>,
    summary: &mut WriteCheckSummary,
) {
    let value = raw.trim();
    if value.is_empty() {
        return;
    }
    let path = PathBuf::from(value);
    let Some(parent) = path.parent() else {
        summary
            .lines
            .push(format!("INFO | {label} parent | {value} | no parent directory"));
        return;
    };
    let key = format!("dir:{}", parent.display());
    if !seen.insert(key) {
        return;
    }
    let line = match probe_write_dir(parent, ts) {
        Ok(()) => format!("OK | {label} parent | {}", parent.display()),
        Err(err) => format!("FAIL | {label} parent | {} | {err}", parent.display()),
    };
    summary.lines.push(line);
}

fn probe_write_dir(dir: &std::path::Path, ts: u64) -> Result<(), String> {
    if !dir.exists() {
        return Err("directory does not exist".to_string());
    }
    if !dir.is_dir() {
        return Err("path is not a directory".to_string());
    }
    let probe = dir.join(format!(
        ".bio_diag_write_probe_{}_{}.tmp",
        process::id(),
        ts
    ));
    fs::write(&probe, b"bio_diagnostics_probe")
        .map_err(|e| format!("write failed: {e}"))?;
    if let Err(e) = fs::remove_file(&probe) {
        return Err(format!("cleanup failed: {e}"));
    }
    Ok(())
}

fn fallback_console_excerpt(state: &WizardState, limit_chars: usize) -> String {
    let text = state.step5.console_output.as_str();
    if text.trim().is_empty() {
        return "Terminal not available".to_string();
    }
    let total = text.chars().count();
    if total <= limit_chars {
        return text.to_string();
    }
    let start = total.saturating_sub(limit_chars);
    text.chars().skip(start).collect()
}

fn copy_appdata_configs(run_dir: &std::path::Path) -> AppDataCopySummary {
    let mut summary = AppDataCopySummary::default();
    let appdata_out_dir = run_dir.join("appdata");
    let _ = fs::create_dir_all(&appdata_out_dir);
    let Some(bio_dir) = app_config_dir() else {
        summary
            .missing
            .push("BIO app-data directory could not be resolved".to_string());
        return summary;
    };

    copy_named_appdata_dir(
        &bio_dir,
        "bio",
        &appdata_out_dir,
        &mut summary.copied,
        &mut summary.missing,
    );

    if let Some(parent) = bio_dir.parent() {
        let mod_installer_dir = parent.join("mod_installer");
        copy_named_appdata_dir(
            &mod_installer_dir,
            "mod_installer",
            &appdata_out_dir,
            &mut summary.copied,
            &mut summary.missing,
        );
    } else {
        summary
            .missing
            .push("mod_installer app-data directory parent could not be resolved".to_string());
    }

    summary
}

fn copy_named_appdata_dir(
    source_dir: &std::path::Path,
    label: &str,
    out_dir: &std::path::Path,
    copied: &mut Vec<PathBuf>,
    missing: &mut Vec<String>,
) {
    if !source_dir.is_dir() {
        missing.push(format!("{label}: not found at {}", source_dir.display()));
        return;
    }

    let dest_dir = out_dir.join(label);
    if fs::create_dir_all(&dest_dir).is_err() {
        missing.push(format!(
            "{label}: failed to create destination {}",
            dest_dir.display()
        ));
        return;
    }

    let mut copied_any = false;
    copy_appdata_tree_filtered(source_dir, &dest_dir, copied, &mut copied_any);

    if !copied_any {
        missing.push(format!(
            "{label}: no copyable config files found in {}",
            source_dir.display()
        ));
    }
}

fn copy_appdata_tree_filtered(
    src_dir: &std::path::Path,
    dst_dir: &std::path::Path,
    copied: &mut Vec<PathBuf>,
    copied_any: &mut bool,
) {
    let Ok(entries) = fs::read_dir(src_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let src_path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            let next_dst = dst_dir.join(entry.file_name());
            let _ = fs::create_dir_all(&next_dst);
            copy_appdata_tree_filtered(&src_path, &next_dst, copied, copied_any);
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let ext = src_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !matches!(ext.as_str(), "json" | "toml" | "yaml" | "yml" | "log" | "txt") {
            continue;
        }

        let Some(name) = src_path.file_name() else {
            continue;
        };
        let dest = dst_dir.join(name);
        if fs::copy(&src_path, &dest).is_ok() {
            copied.push(dest);
            *copied_any = true;
        }
    }
}
