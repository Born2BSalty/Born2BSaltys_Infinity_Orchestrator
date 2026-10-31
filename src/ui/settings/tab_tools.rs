// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::tool_validation::{ToolKind, ToolPlatform, guidance, release_url};
use crate::ui::settings::validate_debounce;
use crate::ui::settings::validate_now;
use crate::ui::settings::widgets::path_row::{self, PathRow, PathRowHelp, PathRowMode};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    bin_row_for_field(
        ui,
        orchestrator,
        ToolKind::WeiDu,
        "WeiDU binary",
        validate_now::FIELD_WEIDU_BINARY,
    );
    bin_row_for_field(
        ui,
        orchestrator,
        ToolKind::ModInstaller,
        "Mod Installer",
        validate_now::FIELD_MOD_INSTALLER_BINARY,
    );
}

fn bin_row_for_field(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    kind: ToolKind,
    label: &str,
    field: &'static str,
) {
    let Some(value) = field_mut(&mut orchestrator.wizard_state.step1, field) else {
        return;
    };
    let requested = value.clone();
    let status =
        orchestrator
            .settings_screen_state
            .tool_validation
            .observe(kind, &requested, ui.ctx());
    let hint = status.hint(kind);
    let link_label = format!("Download {}", kind.label());
    let mut changed = false;
    if let Some(value) = field_mut(&mut orchestrator.wizard_state.step1, field) {
        path_row::render_with_help(
            ui,
            orchestrator.theme_palette,
            PathRow {
                label,
                mono_value: value,
                hint: Some(&hint),
                tone: status.tone(),
                mode: PathRowMode::File,
            },
            PathRowHelp {
                guidance: guidance(kind, ToolPlatform::current()),
                link_label: &link_label,
                url: release_url(kind),
            },
            || changed = true,
        );
    }
    if changed {
        if let Some(value) = field_mut(&mut orchestrator.wizard_state.step1, field) {
            let value = value.clone();
            orchestrator
                .settings_screen_state
                .tool_validation
                .observe(kind, &value, ui.ctx());
        }
        validate_debounce::mark_dirty(orchestrator, field);
    }
}

fn field_mut<'a>(
    step1: &'a mut crate::app::state::Step1State,
    field: &'static str,
) -> Option<&'a mut String> {
    match field {
        f if f == validate_now::FIELD_WEIDU_BINARY => Some(&mut step1.weidu_binary),
        f if f == validate_now::FIELD_MOD_INSTALLER_BINARY => Some(&mut step1.mod_installer_binary),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tab_tools_tests.rs"]
mod tests;
