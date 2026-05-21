// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_popup_text::{build_mod_prompt_popup_text, mod_has_any_prompt};
use crate::app::state::{Step2ModState, Step2Selection};
use crate::parser::prompt_eval_expr::PromptEvalContext;
use crate::ui::orchestrator::widgets::{ButtonIcon, render_icon_button};
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step2::tree_compat_display_step2::{parent_compat_summary, parent_compat_target};
use crate::ui::step2::tree_selection_rules_step2::{
    enforce_collapsible_group_umbrella_after_bulk, enforce_meta_mode_after_bulk,
    enforce_subcomponent_single_select_keep_first, enforce_tp2_same_mod_exclusive_after_bulk,
    set_component_checked_state,
};

#[derive(Default)]
pub(crate) struct ParentRowResult {
    pub selection: Option<Step2Selection>,
    pub open_compat_for_component: Option<(String, String, String)>,
    pub open_prompt_popup: Option<(String, String)>,
    pub open_details: bool,
}

#[derive(Clone, Copy)]
struct ParentSelectionCounts {
    mod_visible_count: usize,
    selected_visible_count: usize,
    enabled_count: usize,
    all_selected: bool,
    any_selected: bool,
}

struct ParentRenderContext<'a> {
    active_tab: &'a str,
    selected: Option<&'a Step2Selection>,
    prompt_eval: &'a PromptEvalContext,
    jump_to_selected_requested: &'a mut bool,
    palette: ThemePalette,
}

pub(crate) struct ParentRowInput<'a> {
    pub active_tab: &'a str,
    pub selected: Option<&'a Step2Selection>,
    pub next_selection_order: &'a mut usize,
    pub prompt_eval: &'a PromptEvalContext,
    pub jump_to_selected_requested: &'a mut bool,
    pub palette: ThemePalette,
}

pub(crate) fn render_parent_row(
    ui: &mut egui::Ui,
    mod_state: &mut Step2ModState,
    input: ParentRowInput<'_>,
) -> ParentRowResult {
    let ParentRowInput {
        active_tab,
        selected,
        next_selection_order,
        prompt_eval,
        jump_to_selected_requested,
        palette,
    } = input;
    let mod_name = mod_state.name.clone();
    let counts = parent_selection_counts(mod_state);
    let mod_header_label = format!(
        "{mod_name} ({}/{})",
        counts.selected_visible_count, counts.mod_visible_count
    );
    let parent_summary = parent_compat_summary(mod_state);
    let mut result = ParentRowResult::default();
    let mut row_ctx = ParentRenderContext {
        active_tab,
        selected,
        prompt_eval,
        jump_to_selected_requested,
        palette,
    };

    ui.horizontal(|ui| {
        render_parent_checkbox(ui, mod_state, next_selection_order, counts);
        render_parent_label_area(
            ui,
            mod_state,
            &mut row_ctx,
            &mut result,
            mod_header_label.as_str(),
            parent_summary.as_ref(),
        );
    });
    result
}

fn parent_selection_counts(mod_state: &Step2ModState) -> ParentSelectionCounts {
    let mod_visible_count = mod_state.components.len();
    let selected_visible_count = mod_state
        .components
        .iter()
        .filter(|component| component.checked)
        .count();
    let enabled_count = mod_state
        .components
        .iter()
        .filter(|component| !component.disabled)
        .count();
    let all_selected = enabled_count > 0
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
    let any_selected = mod_state
        .components
        .iter()
        .filter(|component| !component.disabled)
        .any(|component| component.checked);
    ParentSelectionCounts {
        mod_visible_count,
        selected_visible_count,
        enabled_count,
        all_selected,
        any_selected,
    }
}

fn render_parent_checkbox(
    ui: &mut egui::Ui,
    mod_state: &mut Step2ModState,
    next_selection_order: &mut usize,
    counts: ParentSelectionCounts,
) {
    let mut parent_checked = counts.all_selected;
    let mut checkbox = egui::Checkbox::new(&mut parent_checked, "");
    if counts.any_selected && !counts.all_selected {
        checkbox = checkbox.indeterminate(true);
    }
    let parent_clicked = ui
        .push_id(
            (
                "mod_parent_checkbox",
                &mod_state.tp_file,
                &mod_state.name,
                &mod_state.tp2_path,
            ),
            |ui| {
                ui.add_enabled_ui(counts.enabled_count > 0, |ui| ui.add(checkbox).clicked())
                    .inner
            },
        )
        .inner;
    if parent_clicked {
        apply_parent_checkbox_change(
            mod_state,
            next_selection_order,
            counts.enabled_count,
            !counts.any_selected,
        );
    }
}

fn apply_parent_checkbox_change(
    mod_state: &mut Step2ModState,
    next_selection_order: &mut usize,
    enabled_count: usize,
    set_value: bool,
) {
    for component in &mut mod_state.components {
        if component.disabled {
            continue;
        }
        component.checked = set_value;
        set_component_checked_state(component, next_selection_order);
    }
    if set_value {
        enforce_subcomponent_single_select_keep_first(mod_state);
        enforce_collapsible_group_umbrella_after_bulk(mod_state);
        enforce_tp2_same_mod_exclusive_after_bulk(mod_state);
    }
    enforce_meta_mode_after_bulk(mod_state);
    mod_state.checked = enabled_count > 0
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
}

fn render_parent_label_area(
    ui: &mut egui::Ui,
    mod_state: &Step2ModState,
    ctx: &mut ParentRenderContext<'_>,
    result: &mut ParentRowResult,
    mod_header_label: &str,
    parent_summary: Option<&(egui::Color32, egui::Color32, String)>,
) {
    let row_w = ui.available_width().max(0.0);
    let row_h = ui.spacing().interact_size.y;
    let (row_rect, row_response) =
        ui.allocate_exact_size(egui::vec2(row_w, row_h), egui::Sense::hover());
    let row_hovered = row_response.hovered() || ui.rect_contains_pointer(row_rect);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(row_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_max_width(row_w);
            let is_selected = parent_is_selected(mod_state, ctx);
            render_parent_selection_label(
                ui,
                mod_state,
                ctx,
                result,
                mod_header_label,
                is_selected,
            );
            crate::ui::step2::tree_header_marker_step2::render(ui, mod_state);
            render_parent_compat_pill(ui, mod_state, result, parent_summary);
            render_parent_prompt_pill(ui, mod_state, ctx, result);
            render_parent_details_action(ui, mod_state, ctx, result, row_hovered || is_selected);
        },
    );
}

fn parent_is_selected(mod_state: &Step2ModState, ctx: &ParentRenderContext<'_>) -> bool {
    matches!(
        ctx.selected,
        Some(Step2Selection::Mod { game_tab, tp_file })
            if game_tab == ctx.active_tab && tp_file == &mod_state.tp_file
    )
}

fn render_parent_selection_label(
    ui: &mut egui::Ui,
    mod_state: &Step2ModState,
    ctx: &mut ParentRenderContext<'_>,
    result: &mut ParentRowResult,
    mod_header_label: &str,
    is_selected: bool,
) {
    let row = ui.selectable_label(is_selected, mod_header_label);
    if *ctx.jump_to_selected_requested && is_selected {
        ui.scroll_to_rect(row.rect, Some(egui::Align::Center));
        *ctx.jump_to_selected_requested = false;
    }
    if row.clicked() {
        select_parent(result, ctx.active_tab, &mod_state.tp_file);
    }
}

fn render_parent_details_action(
    ui: &mut egui::Ui,
    mod_state: &Step2ModState,
    ctx: &ParentRenderContext<'_>,
    result: &mut ParentRowResult,
    visible: bool,
) {
    let spare = (ui.available_width() - 24.0).max(0.0);
    ui.add_space(spare);
    if render_icon_button(
        ui,
        ctx.palette,
        ButtonIcon::Details,
        "Show details",
        visible,
    )
    .clicked()
    {
        select_parent(result, ctx.active_tab, &mod_state.tp_file);
        result.open_details = true;
    }
}

fn select_parent(result: &mut ParentRowResult, active_tab: &str, tp_file: &str) {
    result.selection = Some(Step2Selection::Mod {
        game_tab: active_tab.to_string(),
        tp_file: tp_file.to_string(),
    });
}

fn render_parent_compat_pill(
    ui: &mut egui::Ui,
    mod_state: &Step2ModState,
    result: &mut ParentRowResult,
    parent_summary: Option<&(egui::Color32, egui::Color32, String)>,
) {
    let Some((text_color, bg, label)) = parent_summary else {
        return;
    };
    ui.add_space(6.0);
    let resp = ui.add(
        egui::Button::new(
            crate::ui::shared::typography_global::strong(label)
                .color(*text_color)
                .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT),
        )
        .fill(*bg)
        .stroke(egui::Stroke::new(
            crate::ui::shared::layout_tokens_global::BORDER_THIN,
            *bg,
        ))
        .corner_radius(egui::CornerRadius::same(7))
        .min_size(egui::vec2(0.0, 18.0)),
    );
    if resp.clicked()
        && let Some(target_compat) = parent_compat_target(mod_state)
    {
        result.open_compat_for_component = Some((
            mod_state.tp_file.clone(),
            target_compat.component_id.clone(),
            target_compat.raw_line.clone(),
        ));
    }
}

fn render_parent_prompt_pill(
    ui: &mut egui::Ui,
    mod_state: &Step2ModState,
    ctx: &ParentRenderContext<'_>,
    result: &mut ParentRowResult,
) {
    if !mod_has_any_prompt(mod_state, ctx.prompt_eval) {
        return;
    }
    ui.add_space(6.0);
    let prompt_resp = ui.add(
        egui::Button::new(
            crate::ui::shared::typography_global::strong("PROMPT")
                .color(crate::ui::shared::theme_global::prompt_text())
                .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT),
        )
        .fill(crate::ui::shared::theme_global::prompt_fill())
        .stroke(egui::Stroke::new(
            crate::ui::shared::layout_tokens_global::BORDER_THIN,
            crate::ui::shared::theme_global::prompt_stroke(),
        ))
        .corner_radius(egui::CornerRadius::same(7))
        .min_size(egui::vec2(0.0, 18.0)),
    );
    let prompt_resp =
        prompt_resp.on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
    if prompt_resp.clicked() {
        select_parent(result, ctx.active_tab, &mod_state.tp_file);
        if let Some(text) = build_mod_prompt_popup_text(mod_state, ctx.prompt_eval) {
            result.open_prompt_popup = Some((mod_state.tp_file.clone(), text));
        }
    }
}
