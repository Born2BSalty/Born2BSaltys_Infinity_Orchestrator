// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::typography_global as typo;
use crate::ui::state::{Step3ItemState, WizardState};
use crate::ui::step3::action_step3::Step3Action;
use crate::ui::step3::compat_modal_step3::compat_modal;
use crate::ui::step3::compat_modal_step3::compat_model;
use crate::ui::step3::list_step3::list;
use crate::ui::step3::state_step3;
use crate::ui::step5::service_step5::export_diagnostics;

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

pub fn render_toolbar(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    ui.horizontal(|ui| {
        let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
        let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
        if show_bgee && show_bg2ee {
            draw_tab(ui, &mut state.step3.active_game_tab, "BGEE");
            draw_tab(ui, &mut state.step3.active_game_tab, "BG2EE");
        } else if show_bgee {
            ui.label(typo::monospace("BGEE"));
    } else if show_bg2ee {
            ui.label(typo::monospace("BG2EE"));
        }

        if state.compat.error_count > 0
            && ui
                .button(
                    crate::ui::shared::typography_global::strong(format!("{} errors", state.compat.error_count))
                        .color(crate::ui::shared::theme_global::error()),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_OPEN_COMPAT_ISSUES)
                .clicked()
        {
            state.step3.compat_modal_open = true;
        }
        if state.compat.warning_count > 0
            && ui
                .button(
                    crate::ui::shared::typography_global::strong(format!("{} warnings", state.compat.warning_count))
                        .color(crate::ui::shared::theme_global::warning_soft()),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_OPEN_COMPAT_ISSUES)
                .clicked()
        {
            state.step3.compat_modal_open = true;
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
                for item in items.iter().filter(|i| i.is_parent) {
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


// --- migrated from frame_step3.rs ---

// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step3Action> {
    let mut action: Option<Step3Action> = None;
    state_step3::normalize_active_tab(state);

    ui.heading("Step 3: Reorder and Resolve");
    ui.label("Review and adjust install order. Drag and drop components to reorder them.");
    ui.label(
        crate::ui::shared::typography_global::weak(
            "Right-click a component for more actions, including uncheck and prompt tools.",
        ),
    );
    ui.add_space(8.0);

    crate::ui::step3::content_step3::render_toolbar(
        ui,
        state,
        dev_mode,
        exe_fingerprint,
    );

    ui.add_space(6.0);
    let mut jump_to_selected_requested = state.step3.jump_to_selected_requested;
    list::render(ui, state, &mut action, &mut jump_to_selected_requested);

    if state.step3.compat_modal_open {
        render_compat_modal(ui, state, &mut jump_to_selected_requested);
    }
    crate::ui::step2::content_step2::render_prompt_popup(ui, state);
    state.step3.jump_to_selected_requested = jump_to_selected_requested;

    action
}

fn render_compat_modal(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
) {
    let jump_request = compat_modal::render(ui, state);
    if let Some(jump_request) = jump_request
        && let issue_id = match &jump_request {
            compat_model::CompatJumpAction::Auto(id)
            | compat_model::CompatJumpAction::Affected(id)
            | compat_model::CompatJumpAction::Related(id) => id.as_str(),
        }
        && let Some(issue) = state
            .compat
            .issues
            .iter()
            .find(|i| i.issue_id == issue_id)
            .cloned()
        && match jump_request {
            compat_model::CompatJumpAction::Auto(_) => crate::ui::step3::service_step3::jump_to_compat_issue(state, &issue),
            compat_model::CompatJumpAction::Affected(_) => crate::ui::step3::service_step3::jump_to_affected_issue(state, &issue),
            compat_model::CompatJumpAction::Related(_) => crate::ui::step3::service_step3::jump_to_related_issue(state, &issue),
        }
    {
        *jump_to_selected_requested = true;
        state.step3.compat_modal_open = false;
    }
}
