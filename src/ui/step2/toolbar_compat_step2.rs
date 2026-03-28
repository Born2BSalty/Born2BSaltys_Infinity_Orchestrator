// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step2ModState;
use crate::ui::step2::tree_compat_display_step2::compat_colors;

#[derive(Clone)]
pub(crate) struct Step2ToolbarCompatSummary {
    pub(crate) total_count: usize,
    pub(crate) dominant_filter: &'static str,
    pub(crate) dominant_count: usize,
}

#[derive(Clone)]
pub(crate) struct Step2ToolbarIssueTarget {
    pub(crate) tp_file: String,
    pub(crate) component_id: String,
    pub(crate) component_key: String,
}

fn issue_filter_bucket(kind: &str) -> &'static str {
    match kind.trim().to_ascii_lowercase().as_str() {
        "conflict" | "not_compatible" => "Conflict",
        "order_block" => "Order",
        "mismatch" | "game_mismatch" => "Mismatch",
        "missing_dep" => "Missing",
        "included" | "not_needed" => "Included",
        "path_requirement" => "Path",
        "conditional" => "Conditional",
        "deprecated" => "Deprecated",
        "warning" => "Warning",
        _ => "Other",
    }
}

pub(crate) fn active_tab_compat_summary(mods: &[Step2ModState]) -> Step2ToolbarCompatSummary {
    let mut count = 0usize;
    let mut bucket_counts = std::collections::BTreeMap::<&'static str, usize>::new();
    for mod_state in mods {
        for component in &mod_state.components {
            let Some(kind) = component.compat_kind.as_deref() else {
                continue;
            };
            if kind.trim().is_empty() {
                continue;
            }
            count += 1;
            *bucket_counts.entry(issue_filter_bucket(kind)).or_default() += 1;
        }
    }
    let dominant_filter = [
        "Conflict",
        "Mismatch",
        "Missing",
        "Order",
        "Path",
        "Conditional",
        "Deprecated",
        "Warning",
        "Included",
        "Other",
    ]
    .into_iter()
    .max_by_key(|bucket| bucket_counts.get(bucket).copied().unwrap_or(0))
    .unwrap_or("All");
    Step2ToolbarCompatSummary {
        total_count: count,
        dominant_filter,
        dominant_count: bucket_counts.get(dominant_filter).copied().unwrap_or(0),
    }
}

pub(crate) fn first_active_tab_issue_target(
    mods: &[Step2ModState],
    filter: &str,
) -> Option<Step2ToolbarIssueTarget> {
    let mut first_any = None::<Step2ToolbarIssueTarget>;
    for mod_state in mods {
        for component in &mod_state.components {
            let Some(kind) = component.compat_kind.as_deref() else {
                continue;
            };
            if kind.trim().is_empty() {
                continue;
            }
            let target = Step2ToolbarIssueTarget {
                tp_file: mod_state.tp_file.clone(),
                component_id: component.component_id.clone(),
                component_key: component.raw_line.clone(),
            };
            if issue_filter_bucket(kind).eq_ignore_ascii_case(filter) {
                return Some(target);
            }
            if first_any.is_none() {
                first_any = Some(target);
            }
        }
    }
    first_any
}

pub(crate) fn draw_active_tab_issue_badge(
    ui: &mut egui::Ui,
    game_tab: &str,
    summary: &Step2ToolbarCompatSummary,
    active_filter: &str,
) -> bool {
    if summary.total_count == 0 {
        return false;
    }

    let display_filter = if active_filter.eq_ignore_ascii_case("All") {
        summary.dominant_filter
    } else {
        active_filter
    };
    let display_count = if active_filter.eq_ignore_ascii_case("All") {
        summary.dominant_count
    } else {
        summary.total_count
    };

    let kind_key = match display_filter {
        "Conflict" => Some("conflict"),
        "Order" => Some("order_block"),
        "Mismatch" => Some("mismatch"),
        "Missing" => Some("missing_dep"),
        "Included" => Some("included"),
        "Path" => Some("path_requirement"),
        "Conditional" => Some("conditional"),
        "Deprecated" => Some("deprecated"),
        "Warning" => Some("warning"),
        _ => None,
    };
    let (text_color, fill_color) = compat_colors(kind_key)
        .map(|(text_color, fill_color, _)| (text_color, fill_color))
        .unwrap_or((
            crate::ui::shared::theme_global::text_muted(),
            ui.visuals().widgets.inactive.bg_fill,
        ));

    let badge = egui::Button::new(
        crate::ui::shared::typography_global::strong(format!(
            "{game_tab} {} {}",
            display_filter, display_count
        ))
        .color(text_color)
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT),
    )
    .fill(fill_color)
    .stroke(egui::Stroke::new(
        crate::ui::shared::layout_tokens_global::BORDER_THIN,
        fill_color,
    ))
    .corner_radius(egui::CornerRadius::same(7))
    .min_size(egui::vec2(0.0, 18.0));
    let issue_label = if summary.total_count == 1 {
        "issue"
    } else {
        "issues"
    };

    ui.add(badge)
        .on_hover_text(format!(
            "{} compatibility {} in the {} Step 2 tab. Active badge category: {} ({}). Dominant category: {} ({}).",
            summary.total_count,
            issue_label,
            game_tab,
            display_filter,
            display_count,
            summary.dominant_filter,
            summary.dominant_count
        ))
        .clicked()
}
