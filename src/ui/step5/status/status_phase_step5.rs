// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;

pub(crate) struct PhaseInfo {
    pub label: &'static str,
    pub color: egui::Color32,
}

pub(crate) fn compute_phase(state: &WizardState, waiting_for_input: bool) -> PhaseInfo {
    if state.step5.install_running {
        if state.step5.cancel_pending {
            return PhaseInfo {
                label: "Cancelling",
                color: crate::ui::shared::theme_global::warning(),
            };
        }
        if waiting_for_input {
            return PhaseInfo {
                label: "Waiting Input",
                color: crate::ui::shared::theme_global::accent_path(),
            };
        }
        return PhaseInfo {
            label: "Running",
            color: crate::ui::shared::theme_global::status_running(),
        };
    }
    if state.step5.last_status_text.starts_with("Preflight")
        || state.step5.last_status_text.starts_with("Target prep")
        || state.step5.last_status_text.starts_with("Backup target")
    {
        return PhaseInfo {
            label: "Preparing",
            color: crate::ui::shared::theme_global::status_preparing(),
        };
    }
    if state.step5.has_run_once {
        return PhaseInfo {
            label: "Finished",
            color: crate::ui::shared::theme_global::text_muted(),
        };
    }
    PhaseInfo {
        label: "Idle",
        color: crate::ui::shared::theme_global::status_idle(),
    }
}

pub(crate) fn render_phase(ui: &mut egui::Ui, state: &WizardState, phase: &PhaseInfo) {
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
            crate::ui::shared::theme_global::error()
        } else {
            crate::ui::shared::theme_global::text_muted()
        };
        ui.label(
            crate::ui::shared::typography_global::weak(status_text.to_string()).color(status_color),
        );
    }
}
