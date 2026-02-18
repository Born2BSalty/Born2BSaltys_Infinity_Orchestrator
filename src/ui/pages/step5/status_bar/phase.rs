// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

pub(super) struct PhaseInfo {
    pub label: &'static str,
    pub color: egui::Color32,
}

pub(super) fn compute_phase(state: &WizardState, waiting_for_input: bool) -> PhaseInfo {
    if state.step5.install_running {
        if state.step5.cancel_pending {
            return PhaseInfo {
                label: "Cancelling",
                color: egui::Color32::from_rgb(214, 168, 96),
            };
        }
        if waiting_for_input {
            return PhaseInfo {
                label: "Waiting Input",
                color: egui::Color32::from_rgb(224, 196, 156),
            };
        }
        return PhaseInfo {
            label: "Running",
            color: egui::Color32::from_rgb(168, 204, 98),
        };
    }
    if state.step5.last_status_text.starts_with("Preflight")
        || state.step5.last_status_text.starts_with("Target prep")
        || state.step5.last_status_text.starts_with("Backup target")
    {
        return PhaseInfo {
            label: "Preparing",
            color: egui::Color32::from_rgb(180, 170, 220),
        };
    }
    if state.step5.has_run_once {
        return PhaseInfo {
            label: "Finished",
            color: egui::Color32::from_gray(190),
        };
    }
    PhaseInfo {
        label: "Idle",
        color: egui::Color32::from_gray(170),
    }
}

pub(super) fn render_phase(ui: &mut egui::Ui, state: &WizardState, phase: &PhaseInfo) {
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
        egui::Label::new(egui::RichText::new(phase_text).strong().color(phase.color))
            .wrap_mode(egui::TextWrapMode::Extend),
    );
    if let Some(tip) = status_tooltip.as_deref() {
        phase_resp.on_hover_text(tip);
    }
}
