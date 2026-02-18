// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use crate::ui::state::WizardState;

pub(super) fn build_base_text(
    state: &WizardState,
    copied_source_logs: &[PathBuf],
    active_order: &[String],
    console_excerpt: &str,
) -> String {
    let mut text = String::new();
    text.push_str("BIO Diagnostics\n");
    text.push_str("====================\n\n");
    text.push_str("[Step1]\n");
    text.push_str(&format!("game_install={}\n", state.step1.game_install));
    text.push_str(&format!("mods_folder={}\n", state.step1.mods_folder));
    text.push_str(&format!("weidu_binary={}\n", state.step1.weidu_binary));
    text.push_str(&format!(
        "mod_installer_binary={}\n",
        state.step1.mod_installer_binary
    ));
    text.push_str(&format!("have_weidu_logs={}\n", state.step1.have_weidu_logs));
    text.push_str(&format!("skip_installed={}\n", state.step1.skip_installed));
    text.push_str(&format!("strict_matching={}\n", state.step1.strict_matching));
    text.push_str(&format!(
        "check_last_installed={}\n",
        state.step1.check_last_installed
    ));
    text.push_str("\n[Step2]\n");
    text.push_str(&format!("selected_count={}\n", state.step2.selected_count));
    text.push_str(&format!("total_count={}\n", state.step2.total_count));
    text.push_str(&format!("active_tab={}\n", state.step2.active_game_tab));
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
