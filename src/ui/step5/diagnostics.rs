// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod compat_snapshot;
mod format;
mod text;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

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

pub fn export_diagnostics(
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    ctx: &DiagnosticsContext,
) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let out_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("bio_diag_{ts}.txt"));

    let copied_source_logs = copy_source_weidu_logs(&state.step1, &out_dir, format!("source_{ts}").as_str());
    let appdata_summary = copy_appdata_configs(&out_dir, ts);
    let active_order = if state.step3.active_game_tab == "BG2EE" {
        build_weidu_export_lines(&state.step3.bg2ee_items)
    } else {
        build_weidu_export_lines(&state.step3.bgee_items)
    };
    let console_excerpt = terminal
        .map(|t| t.console_excerpt(40_000))
        .unwrap_or_else(|| "Terminal not available".to_string());
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
    );
    if ctx.dev_mode {
        compat_snapshot::append_dev_compat_snapshots(state, &mut text);
    }

    fs::write(&out_path, text)?;
    Ok(out_path)
}

fn copy_appdata_configs(out_dir: &std::path::Path, ts: u64) -> AppDataCopySummary {
    let mut summary = AppDataCopySummary::default();
    let Some(bio_dir) = app_config_dir() else {
        summary
            .missing
            .push("BIO app-data directory could not be resolved".to_string());
        return summary;
    };

    copy_named_appdata_dir(
        &bio_dir,
        "bio",
        out_dir,
        ts,
        &mut summary.copied,
        &mut summary.missing,
    );

    if let Some(parent) = bio_dir.parent() {
        let mod_installer_dir = parent.join("mod_installer");
        copy_named_appdata_dir(
            &mod_installer_dir,
            "mod_installer",
            out_dir,
            ts,
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
    ts: u64,
    copied: &mut Vec<PathBuf>,
    missing: &mut Vec<String>,
) {
    if !source_dir.is_dir() {
        missing.push(format!("{label}: not found at {}", source_dir.display()));
        return;
    }

    let dest_dir = out_dir.join(format!("appdata_{label}_{ts}"));
    if fs::create_dir_all(&dest_dir).is_err() {
        missing.push(format!(
            "{label}: failed to create destination {}",
            dest_dir.display()
        ));
        return;
    }

    let Ok(entries) = fs::read_dir(source_dir) else {
        missing.push(format!("{label}: failed to read {}", source_dir.display()));
        return;
    };

    let mut copied_any = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !matches!(ext.as_str(), "json" | "toml" | "yaml" | "yml" | "log" | "txt") {
            continue;
        }
        let dest = dest_dir.join(name);
        if fs::copy(&path, &dest).is_ok() {
            copied.push(dest);
            copied_any = true;
        }
    }

    if !copied_any {
        missing.push(format!(
            "{label}: no copyable config files found in {}",
            source_dir.display()
        ));
    }
}
