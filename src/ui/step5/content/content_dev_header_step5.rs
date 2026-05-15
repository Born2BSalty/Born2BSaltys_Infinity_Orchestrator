// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_accent_path, redesign_success};

const STEP5_TITLE: &str = "Step 5: Install, Logs, Diagnostics";

pub(crate) fn render_dev_header(
    ui: &mut egui::Ui,
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    dev_mode: bool,
    palette: ThemePalette,
) {
    ui.heading(step5_title(state, terminal));
    ui.label("Final execution view.");
    if dev_mode {
        let has_rust_log = state.step1.rust_log_debug || state.step1.rust_log_trace;
        let level = if state.step1.rust_log_trace {
            "TRACE"
        } else if state.step1.rust_log_debug {
            "DEBUG"
        } else {
            "OFF"
        };
        let color = if has_rust_log {
            redesign_success(palette)
        } else {
            redesign_accent_path(palette)
        };
        let msg = if has_rust_log {
            format!("Dev Mode: RUST_LOG={level} selected.")
        } else {
            "Dev Mode: open Diagnostics and choose RUST_LOG=DEBUG or TRACE before Install."
                .to_string()
        };
        ui.label(crate::ui::shared::typography_global::strong(msg).color(color));
    }
    ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_LG);
}

fn step5_title(state: &WizardState, terminal: Option<&EmbeddedTerminal>) -> String {
    if !state.step5.install_running {
        return STEP5_TITLE.to_string();
    }
    let Some(current_tp2) =
        terminal.and_then(crate::app::terminal::EmbeddedTerminal::current_scripted_component_tp2)
    else {
        return STEP5_TITLE.to_string();
    };
    let current_tp2 = crate::platform_defaults::normalize_tp2_filename(&current_tp2);
    if current_tp2.trim().is_empty() {
        return STEP5_TITLE.to_string();
    }

    let primary_progress = mod_progress_for_items(&current_tp2, &state.step3.bgee_items);
    let secondary_progress = mod_progress_for_items(&current_tp2, &state.step3.bg2ee_items);
    match (primary_progress, secondary_progress) {
        (Some((index, total)), None) => {
            format!("{STEP5_TITLE} — Installing BGEE mod {index}/{total}")
        }
        (None, Some((index, total))) => {
            format!("{STEP5_TITLE} — Installing BG2EE mod {index}/{total}")
        }
        _ => STEP5_TITLE.to_string(),
    }
}

fn mod_progress_for_items(
    current_tp2: &str,
    items: &[crate::app::state::Step3ItemState],
) -> Option<(usize, usize)> {
    let mut ordered_tp2s = Vec::<String>::new();
    for item in items.iter().filter(|item| !item.is_parent) {
        let tp2 = crate::platform_defaults::normalize_tp2_filename(&item.tp_file);
        if !tp2.trim().is_empty()
            && !ordered_tp2s
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&tp2))
        {
            ordered_tp2s.push(tp2);
        }
    }

    ordered_tp2s
        .iter()
        .position(|tp2| tp2.eq_ignore_ascii_case(current_tp2))
        .map(|index| (index + 1, ordered_tp2s.len()))
}
