// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::details::selected_details;
use crate::ui::state::{CompatIssueDisplay, WizardState};

use super::issue_text;
use super::selection;

pub(super) fn render_details(ui: &mut egui::Ui, state: &WizardState) {
    let details = selected_details(state);
    let title = details
        .component_label
        .as_deref()
        .or(details.mod_name.as_deref())
        .unwrap_or("No component selected");
    ui.label(egui::RichText::new(title).strong());
    ui.add_space(6.0);

    if details.compat_kind.is_none() && details.disabled_reason.is_none() {
        ui.label("No compatibility issue data for this item.");
        return;
    }

    let issue = selection::current_issue_for_selection(state)
        .or_else(|| synth_issue_from_details(&details));
    let kind = details.compat_kind.as_deref().unwrap_or("unknown");
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Kind").strong());
        ui.label(issue_text::human_kind(kind));
        if let Some(issue) = issue.as_ref() {
            let (badge_text, badge_color) = if issue.is_blocking {
                ("Blocks install", egui::Color32::from_rgb(220, 96, 96))
            } else {
                ("Warning only", egui::Color32::from_rgb(220, 180, 100))
            };
            ui.label(egui::RichText::new(badge_text).color(badge_color).strong());
        }
    });

    if let Some(role) = details.compat_role.as_deref() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Role").strong());
            ui.label(role);
        });
    }
    if let Some(issue) = issue.as_ref()
        && let Some(verdict) = issue_text::issue_verdict(issue)
    {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Verdict").strong());
            ui.label(verdict);
        });
    }
    if let Some(issue) = issue.as_ref() {
        ui.add_space(2.0);
        ui.label(egui::RichText::new("Why this appears").strong());
        ui.label(issue_text::issue_why_this_appears(issue));

        ui.add_space(2.0);
        ui.label(egui::RichText::new("What to do").strong());
        ui.label(issue_text::issue_what_to_do(issue));
    }
    if let Some(reason) = details.disabled_reason.as_deref() {
        ui.add_space(2.0);
        ui.label(egui::RichText::new("Reason").strong());
        if let Some(issue) = issue.as_ref() {
            ui.label(issue_text::issue_reason(issue, reason));
        } else {
            ui.label(reason);
        }
    }
    if let Some(source) = details.compat_source.as_deref() {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Source").strong());
            ui.monospace(source);
        });
    }
    if let Some(related) = details.compat_related_target.as_deref() {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Related target").strong());
            ui.label(related);
        });
    }
    if let Some(graph) = details.compat_graph.as_deref() {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Graph").strong());
            ui.monospace(graph);
        });
    }
    if let Some(evidence) = details.compat_evidence.as_deref() {
        ui.add_space(2.0);
        egui::CollapsingHeader::new("Rule detail")
            .default_open(false)
            .show(ui, |ui| {
                ui.monospace(evidence);
            });
    }
}

fn synth_issue_from_details(details: &crate::ui::step2::details::Step2Details) -> Option<CompatIssueDisplay> {
    let kind = details.compat_kind.as_deref()?.to_ascii_lowercase();
    let code = if kind == "game_mismatch" {
        "GAME_MISMATCH"
    } else if kind == "missing_dep" {
        "REQ_MISSING"
    } else if kind == "conflict" || kind == "not_compatible" {
        "FORBID_HIT"
    } else if kind == "conditional" {
        "CONDITIONAL"
    } else {
        "RULE_HIT"
    };
    let is_blocking = !matches!(code, "CONDITIONAL" | "ORDER_WARN");
    let related = details
        .compat_related_target
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    Some(CompatIssueDisplay {
        issue_id: "synthetic".to_string(),
        code: code.to_string(),
        severity: if is_blocking {
            "Error".to_string()
        } else {
            "Warning".to_string()
        },
        is_blocking,
        affected_mod: details.tp_file.clone().unwrap_or_else(|| "unknown".to_string()),
        affected_component: details.component_id.as_deref().and_then(|v| v.parse::<u32>().ok()),
        related_mod: related,
        related_component: None,
        reason: details.disabled_reason.clone().unwrap_or_default(),
        source: details.compat_source.clone().unwrap_or_default(),
        raw_evidence: details.compat_evidence.clone(),
    })
}
