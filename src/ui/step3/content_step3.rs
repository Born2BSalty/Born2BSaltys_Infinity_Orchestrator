// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::typography_global as typo;
use crate::ui::state::{Step2Selection, Step3ItemState, WizardState};
use crate::ui::compat_step3_rules::Step3CompatMarker;
use crate::ui::step2::compat_types_step2::{CompatIssueDisplay, CompatIssueStatusTone};
use crate::ui::step2::prompt_popup_step2::{
    collect_step3_prompt_toolbar_entries_from_items, draw_prompt_toolbar_badge,
    open_toolbar_prompt_popup,
};
use crate::ui::step3::list_step3;
use crate::ui::step3::state_step3;
use crate::ui::step5::service_diagnostics_support_step5::export_diagnostics;

pub fn draw_tab(ui: &mut egui::Ui, active: &mut String, value: &str) {
    let is_active = active == value;
    let fill = if is_active {
        ui.visuals().widgets.active.bg_fill
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    let stroke = if is_active {
        ui.visuals().widgets.active.bg_stroke
    } else {
        ui.visuals().widgets.inactive.bg_stroke
    };
    let text_color = if is_active {
        ui.visuals().widgets.active.fg_stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    let button = egui::Button::new(crate::ui::shared::typography_global::plain(value).color(text_color))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(
            crate::ui::shared::layout_tokens_global::RADIUS_SM as u8,
        ));
    if ui.add_sized([58.0, 24.0], button).clicked() {
        *active = value.to_string();
    }
}

fn draw_tab_issue_badge(
    ui: &mut egui::Ui,
    active: &mut String,
    value: &str,
    issue_count: usize,
    has_blocking: bool,
) -> bool {
    if issue_count == 0 {
        return false;
    }

    let (text_color, fill_color) = if has_blocking {
        (
            crate::ui::shared::theme_global::conflict(),
            crate::ui::shared::theme_global::conflict_fill(),
        )
    } else {
        (
            crate::ui::shared::theme_global::warning(),
            crate::ui::shared::theme_global::warning_fill(),
        )
    };

    let badge_text = crate::ui::shared::typography_global::strong(format!("{value} {issue_count}"))
        .color(text_color)
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    let issue_label = if issue_count == 1 { "issue" } else { "issues" };
    let badge = egui::Button::new(badge_text)
        .fill(fill_color)
        .stroke(egui::Stroke::new(
            crate::ui::shared::layout_tokens_global::BORDER_THIN,
            fill_color,
        ))
        .corner_radius(egui::CornerRadius::same(7))
        .min_size(egui::vec2(0.0, 18.0));
    if ui
        .add(badge)
        .on_hover_text(format!("{issue_count} compatibility {issue_label} in the {value} Step 3 tab."))
        .clicked()
    {
        *active = value.to_string();
        return true;
    }
    false
}

fn tab_compat_summary(markers: &std::collections::HashMap<String, Step3CompatMarker>) -> (usize, bool) {
    let has_blocking = markers
        .values()
        .any(|marker| {
            crate::ui::compat_step3_rules::marker_issue(marker).status_tone
                == CompatIssueStatusTone::Blocking
        });
    (markers.len(), has_blocking)
}

fn tab_has_conflict(markers: &std::collections::HashMap<String, Step3CompatMarker>) -> bool {
    markers
        .values()
        .any(|marker| marker.kind.eq_ignore_ascii_case("conflict"))
}

#[derive(Clone)]
struct Step3ToolbarIssueTarget {
    tab_id: String,
    tp_file: String,
    component_id: String,
    component_key: String,
    issue: CompatIssueDisplay,
}

struct Step3ToolbarSummary {
    show_bgee: bool,
    show_bg2ee: bool,
    bgee_summary: (usize, bool),
    bg2ee_summary: (usize, bool),
    bgee_prompt_count: usize,
    bg2ee_prompt_count: usize,
    bgee_target: Option<Step3ToolbarIssueTarget>,
    bg2ee_target: Option<Step3ToolbarIssueTarget>,
}

fn first_tab_issue_target(
    tab_id: &str,
    items: &[Step3ItemState],
    markers: &std::collections::HashMap<String, Step3CompatMarker>,
) -> Option<Step3ToolbarIssueTarget> {
    let mut first_warning: Option<Step3ToolbarIssueTarget> = None;

    for item in items.iter().filter(|item| !item.is_parent) {
        let key = crate::ui::compat_step3_rules::marker_key(item);
        let Some(marker) = markers.get(&key) else {
            continue;
        };
        let issue = crate::ui::compat_step3_rules::marker_issue(marker);
        let target = Step3ToolbarIssueTarget {
            tab_id: tab_id.to_string(),
            tp_file: item.tp_file.clone(),
            component_id: item.component_id.clone(),
            component_key: item.raw_line.clone(),
            issue: issue.clone(),
        };
        if issue.status_tone == CompatIssueStatusTone::Blocking {
            return Some(target);
        }
        if first_warning.is_none() {
            first_warning = Some(target);
        }
    }

    first_warning
}

fn open_toolbar_issue_popup(state: &mut WizardState, target: &Step3ToolbarIssueTarget) {
    state.step3.active_game_tab = target.tab_id.clone();
    state.step2.selected = Some(Step2Selection::Component {
        game_tab: target.tab_id.clone(),
        tp_file: target.tp_file.clone(),
        component_id: target.component_id.clone(),
        component_key: target.component_key.clone(),
    });
    state.step2.compat_popup_issue_override = Some(target.issue.clone());
    state.step2.compat_popup_open = true;
}

pub fn weidu_colored_widget_text(ui: &egui::Ui, text: &str) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let default_color = ui.visuals().text_color();
    let path_color = crate::ui::shared::theme_global::accent_path();
    let nums_color = crate::ui::shared::theme_global::accent_numbers();
    let comment_color = crate::ui::shared::theme_global::success();

    if let Some(path_start) = text.find('~')
        && let Some(path_end_rel) = text[path_start + 1..].find('~')
    {
        let path_end = path_start + path_end_rel + 2;
        let comment_start = text[path_end..].find("//").map(|idx| path_end + idx);

        append_text(&mut job, &text[..path_start], &mono, default_color);
        append_text(&mut job, &text[path_start..path_end], &mono, path_color);

        if let Some(comment_start) = comment_start {
            append_text(&mut job, &text[path_end..comment_start], &mono, nums_color);
            append_text(&mut job, &text[comment_start..], &mono, comment_color);
        } else {
            append_text(&mut job, &text[path_end..], &mono, nums_color);
        }

        return egui::WidgetText::from(job);
    }

    append_text(&mut job, text, &mono, default_color);
    egui::WidgetText::from(job)
}

pub fn format_step3_item(item: &Step3ItemState) -> String {
    if !item.raw_line.trim().is_empty() {
        normalize_weidu_like_line(&item.raw_line)
    } else {
        let folder = item.mod_name.replace('/', "\\");
        format!(
            "~{}\\{}~ #0 #{} // {}",
            folder, item.tp_file, item.component_id, item.component_label
        )
    }
}

fn render_toolbar(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
    summary: &Step3ToolbarSummary,
) {
    ui.horizontal(|ui| {
        let (bgee_issue_count, bgee_has_blocking) = summary.bgee_summary;
        let (bg2ee_issue_count, bg2ee_has_blocking) = summary.bg2ee_summary;
        if summary.show_bgee && summary.show_bg2ee {
            draw_tab(ui, &mut state.step3.active_game_tab, "BGEE");
            draw_tab(ui, &mut state.step3.active_game_tab, "BG2EE");
            ui.add_space(8.0);
            if draw_tab_issue_badge(
                ui,
                &mut state.step3.active_game_tab,
                "BGEE",
                bgee_issue_count,
                bgee_has_blocking,
            ) && let Some(target) = summary.bgee_target.as_ref()
            {
                open_toolbar_issue_popup(state, target);
            }
            if draw_tab_issue_badge(
                ui,
                &mut state.step3.active_game_tab,
                "BG2EE",
                bg2ee_issue_count,
                bg2ee_has_blocking,
            ) && let Some(target) = summary.bg2ee_target.as_ref()
            {
                open_toolbar_issue_popup(state, target);
            }
            let active_prompt_count = if state.step3.active_game_tab == "BGEE" {
                summary.bgee_prompt_count
            } else {
                summary.bg2ee_prompt_count
            };
            if draw_prompt_toolbar_badge(ui, active_prompt_count) {
                open_toolbar_prompt_popup(
                    state,
                    &format!("Prompt Components ({})", state.step3.active_game_tab),
                );
            }
        } else if summary.show_bgee {
            ui.label(typo::monospace("BGEE"));
            if draw_tab_issue_badge(
                ui,
                &mut state.step3.active_game_tab,
                "BGEE",
                bgee_issue_count,
                bgee_has_blocking,
            ) && let Some(target) = summary.bgee_target.as_ref()
            {
                open_toolbar_issue_popup(state, target);
            }
            if draw_prompt_toolbar_badge(ui, summary.bgee_prompt_count) {
                open_toolbar_prompt_popup(state, "Prompt Components (BGEE)");
            }
        } else if summary.show_bg2ee {
            ui.label(typo::monospace("BG2EE"));
            if draw_tab_issue_badge(
                ui,
                &mut state.step3.active_game_tab,
                "BG2EE",
                bg2ee_issue_count,
                bg2ee_has_blocking,
            ) && let Some(target) = summary.bg2ee_target.as_ref()
            {
                open_toolbar_issue_popup(state, target);
            }
            if draw_prompt_toolbar_badge(ui, summary.bg2ee_prompt_count) {
                open_toolbar_prompt_popup(state, "Prompt Components (BG2EE)");
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if dev_mode
                && ui
                    .button("Export diagnostics")
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP3_EXPORT_DIAGNOSTICS)
                    .clicked()
            {
                match export_diagnostics(state, None, dev_mode, exe_fingerprint) {
                    Ok(path) => {
                        state.step5.last_status_text =
                            format!("Diagnostics exported: {}", path.display());
                    }
                    Err(err) => {
                        state.step5.last_status_text =
                            format!("Diagnostics export failed: {err}");
                    }
                }
            }
            let (
                items,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
                collapsed_blocks,
                _,
                _,
                undo_stack,
                redo_stack,
            ) = state_step3::active_list_mut(state);
            if ui
                .button("Expand All")
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_EXPAND_ALL)
                .clicked()
            {
                collapsed_blocks.clear();
            }
            if ui
                .button("Collapse All")
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_COLLAPSE_ALL)
                .clicked()
            {
                collapsed_blocks.clear();
                for item in items.iter().filter(|item| item.is_parent) {
                    if !collapsed_blocks.contains(&item.block_id) {
                        collapsed_blocks.push(item.block_id.clone());
                    }
                }
            }
            if ui
                .add_enabled(!redo_stack.is_empty(), egui::Button::new("Redo"))
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_REDO)
                .clicked()
                && let Some(next) = redo_stack.pop()
            {
                undo_stack.push(items.clone());
                *items = next;
            }
            if ui
                .add_enabled(!undo_stack.is_empty(), egui::Button::new("Undo"))
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_UNDO)
                .clicked()
                && let Some(previous) = undo_stack.pop()
            {
                redo_stack.push(items.clone());
                *items = previous;
            }
        });
    });
}

fn append_text(
    job: &mut egui::text::LayoutJob,
    text: &str,
    font_id: &egui::FontId,
    color: egui::Color32,
) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font_id.clone(),
            color,
            ..Default::default()
        },
    );
}

fn normalize_weidu_like_line(raw: &str) -> String {
    crate::platform_defaults::normalize_weidu_like_line(raw)
}

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    state_step3::normalize_active_tab(state);
    let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    let bgee_markers = if show_bgee {
        crate::ui::compat_step3_rules::collect_step3_compat_markers(
            &state.step1,
            "BGEE",
            &state.step2.bgee_mods,
            &state.step3.bgee_items,
        )
    } else {
        std::collections::HashMap::new()
    };
    let bg2ee_markers = if show_bg2ee {
        crate::ui::compat_step3_rules::collect_step3_compat_markers(
            &state.step1,
            "BG2EE",
            &state.step2.bg2ee_mods,
            &state.step3.bg2ee_items,
        )
    } else {
        std::collections::HashMap::new()
    };
    let prompt_eval = crate::ui::step2::state_step2::build_prompt_eval_context(state);
    let bgee_prompt_count = if show_bgee {
        collect_step3_prompt_toolbar_entries_from_items(&state.step3.bgee_items, &prompt_eval)
            .into_iter()
            .map(|entry| entry.component_ids.len())
            .sum()
    } else {
        0
    };
    let bg2ee_prompt_count = if show_bg2ee {
        collect_step3_prompt_toolbar_entries_from_items(&state.step3.bg2ee_items, &prompt_eval)
            .into_iter()
            .map(|entry| entry.component_ids.len())
            .sum()
    } else {
        0
    };
    let active_markers = if state.step3.active_game_tab == "BGEE" {
        &bgee_markers
    } else {
        &bg2ee_markers
    };
    let toolbar_summary = Step3ToolbarSummary {
        show_bgee,
        show_bg2ee,
        bgee_summary: tab_compat_summary(&bgee_markers),
        bg2ee_summary: tab_compat_summary(&bg2ee_markers),
        bgee_prompt_count,
        bg2ee_prompt_count,
        bgee_target: first_tab_issue_target("BGEE", &state.step3.bgee_items, &bgee_markers),
        bg2ee_target: first_tab_issue_target("BG2EE", &state.step3.bg2ee_items, &bg2ee_markers),
    };
    state.step3.bgee_has_conflict = show_bgee && tab_has_conflict(&bgee_markers);
    state.step3.bg2ee_has_conflict = show_bg2ee && tab_has_conflict(&bg2ee_markers);

    ui.heading("Step 3: Reorder and Resolve");
    ui.label("Review and adjust install order. Drag and drop components to reorder them.");
    ui.label(
        crate::ui::shared::typography_global::weak(
            "Right-click a component for more actions, including uncheck and prompt tools.",
        ),
    );
    ui.add_space(8.0);

    render_toolbar(
        ui,
        state,
        dev_mode,
        exe_fingerprint,
        &toolbar_summary,
    );

    ui.add_space(6.0);
    let mut jump_to_selected_requested = state.step3.jump_to_selected_requested;
    state.step3.jump_to_selected_requested = false;
    list_step3::render(ui, state, &mut jump_to_selected_requested, active_markers);
    crate::ui::step2::content_step2::render_compat_popup(ui, state);
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, state);
    state.step3.jump_to_selected_requested =
        state.step3.jump_to_selected_requested || jump_to_selected_requested;
}
