// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{Step2ComponentState, Step2ModState};
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

fn actionable_issue_bucket(component: &Step2ComponentState) -> Option<&'static str> {
    if !component.checked {
        return None;
    }
    let kind = component.compat_kind.as_deref()?.trim();
    if kind.is_empty() {
        return None;
    }
    let bucket = issue_filter_bucket(kind);
    (!matches!(bucket, "Mismatch" | "Included")).then_some(bucket)
}

pub(crate) fn active_tab_compat_summary(mods: &[Step2ModState]) -> Step2ToolbarCompatSummary {
    let mut count = 0usize;
    let mut bucket_counts = std::collections::BTreeMap::<&'static str, usize>::new();
    for mod_state in mods {
        for component in &mod_state.components {
            let Some(bucket) = actionable_issue_bucket(component) else {
                continue;
            };
            count += 1;
            *bucket_counts.entry(bucket).or_default() += 1;
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
    .rev()
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
            let Some(bucket) = actionable_issue_bucket(component) else {
                continue;
            };
            let target = Step2ToolbarIssueTarget {
                tp_file: mod_state.tp_file.clone(),
                component_id: component.component_id.clone(),
                component_key: component.raw_line.clone(),
            };
            if bucket.eq_ignore_ascii_case(filter) {
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
    let (text_color, fill_color) = compat_colors(kind_key).map_or_else(
        || {
            (
                crate::ui::shared::theme_global::text_muted(),
                ui.visuals().widgets.inactive.bg_fill,
            )
        },
        |(text_color, fill_color, _)| (text_color, fill_color),
    );
    let badge_text =
        crate::ui::shared::typography_global::strong(format!("{display_filter} {display_count}"))
            .color(text_color)
            .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    let badge = egui::Button::new(badge_text)
        .fill(fill_color)
        .stroke(egui::Stroke::new(
            crate::ui::shared::layout_tokens_global::BORDER_THIN,
            text_color,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn component(id: &str, checked: bool, disabled: bool, kind: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: id.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: format!("~MOD.TP2~ #0 #{id}"),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled,
            compat_kind: Some(kind.to_string()),
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked,
            selected_order: None,
        }
    }

    fn mod_with(components: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: "Mod".to_string(),
            tp_file: "mod.tp2".to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components,
        }
    }

    #[test]
    fn badge_counts_only_ticked_actionable_components() {
        let mods = vec![mod_with(vec![
            component("1", true, false, "conflict"),
            component("2", false, false, "conflict"),
            component("3", false, true, "not_compatible"),
            component("4", true, false, "warning"),
            component("5", false, false, "warning"),
        ])];
        let summary = active_tab_compat_summary(&mods);
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.dominant_filter, "Conflict");
        assert_eq!(summary.dominant_count, 1);
    }

    #[test]
    fn badge_ignores_informational_kinds_even_when_ticked() {
        let mods = vec![mod_with(vec![
            component("1", true, false, "mismatch"),
            component("2", true, false, "game_mismatch"),
            component("3", true, false, "included"),
            component("4", true, false, "not_needed"),
            component("5", true, true, "mismatch"),
        ])];
        let summary = active_tab_compat_summary(&mods);
        assert_eq!(summary.total_count, 0);
        assert!(first_active_tab_issue_target(&mods, "Mismatch").is_none());
        assert!(first_active_tab_issue_target(&mods, "All").is_none());
    }

    #[test]
    fn badge_tie_breaks_toward_the_more_severe_bucket() {
        let mods = vec![mod_with(vec![
            component("1", true, false, "warning"),
            component("2", true, false, "conflict"),
        ])];
        let summary = active_tab_compat_summary(&mods);
        assert_eq!(summary.dominant_filter, "Conflict");
        let target = first_active_tab_issue_target(&mods, summary.dominant_filter).expect("target");
        assert_eq!(target.component_id, "2");
    }

    #[test]
    fn badge_hidden_when_no_ticked_component_has_an_issue() {
        let mods = vec![mod_with(vec![
            component("1", false, false, "conflict"),
            component("2", false, false, "warning"),
        ])];
        assert_eq!(active_tab_compat_summary(&mods).total_count, 0);
    }

    #[test]
    fn jump_target_skips_unticked_components() {
        let mods = vec![mod_with(vec![
            component("1", false, false, "conflict"),
            component("2", true, false, "conflict"),
        ])];
        let target = first_active_tab_issue_target(&mods, "Conflict").expect("target");
        assert_eq!(target.component_id, "2");
        let none = first_active_tab_issue_target(
            &[mod_with(vec![component("1", false, false, "conflict")])],
            "Conflict",
        );
        assert!(none.is_none());
    }
}
