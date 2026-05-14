// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_accent_path, redesign_error, redesign_status_idle,
    redesign_status_preparing, redesign_status_running, redesign_text_muted, redesign_warning,
};

pub(crate) struct PhaseInfo {
    pub label: &'static str,
    pub color: egui::Color32,
}

pub(crate) fn compute_phase(
    state: &WizardState,
    waiting_for_input: bool,
    palette: ThemePalette,
) -> PhaseInfo {
    if state.step5.install_running {
        if state.step5.cancel_pending {
            return PhaseInfo {
                label: "Cancelling",
                color: redesign_warning(palette),
            };
        }
        if waiting_for_input {
            return PhaseInfo {
                label: "Waiting Input",
                color: redesign_accent_path(palette),
            };
        }
        return PhaseInfo {
            label: "Running",
            color: redesign_status_running(palette),
        };
    }
    if state.step5.last_status_text.starts_with("Preflight")
        || state.step5.last_status_text.starts_with("Target prep")
        || state.step5.last_status_text.starts_with("Backup target")
    {
        return PhaseInfo {
            label: "Preparing",
            color: redesign_status_preparing(palette),
        };
    }
    if state.step5.has_run_once {
        return PhaseInfo {
            label: "Finished",
            color: redesign_text_muted(palette),
        };
    }
    PhaseInfo {
        label: "Idle",
        color: redesign_status_idle(palette),
    }
}

pub(crate) fn render_phase(
    ui: &mut egui::Ui,
    state: &WizardState,
    phase: &PhaseInfo,
    palette: ThemePalette,
) {
    let phase_state = if state.step5.cancel_pending {
        "Pending".to_string()
    } else {
        phase.label.to_string()
    };
    let status_tooltip = if !state.step5.last_status_text.is_empty()
        && state.step5.last_status_text != phase.label
        && state.step5.last_status_text != "Running"
        && state.step5.last_status_text != "Idle"
    {
        Some(state.step5.last_status_text.clone())
    } else {
        None
    };
    let phase_text = format!("Phase: {phase_state}");
    let phase_resp = ui.add(
        egui::Label::new(
            crate::ui::shared::typography_global::strong(phase_text).color(phase.color),
        )
        .wrap_mode(egui::TextWrapMode::Extend),
    );
    if let Some(tip) = status_tooltip.as_deref() {
        phase_resp.on_hover_text(tip);
    }
    if let Some(status_text) = status_tooltip.as_deref() {
        ui.label(crate::ui::shared::typography_global::mono_weak("|"));
        let status_color = if status_text.starts_with("Install start failed:")
            || status_text.contains("os error")
        {
            redesign_error(palette)
        } else {
            redesign_text_muted(palette)
        };
        ui.label(
            crate::ui::shared::typography_global::weak(status_text.to_string()).color(status_color),
        );
    }
}
