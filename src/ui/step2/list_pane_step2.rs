// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::service_list_ops_step2::mod_matches_filter;
use crate::ui::step2::state_step2::active_mods_mut;
use crate::ui::step2::tree_render_step2::{
    ModTreeRenderContext, ModTreeRenderResult, render_mod_tree,
};

const SCROLLBAR_RESERVE: f32 = 14.0;

pub(crate) fn render_list_pane(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    left_rect: egui::Rect,
    details_open: &mut bool,
    palette: ThemePalette,
) {
    let selection_before = selection_snapshot(state);
    ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
        ui.set_clip_rect(left_rect);
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin {
                left: 12,
                right: 0,
                top: 10,
                bottom: 0,
            })
            .show(ui, |ui| {
                let border_w = ui.visuals().widgets.noninteractive.bg_stroke.width;
                let borders = border_w * 2.0;
                let content_w = (left_rect.width() - 12.0 - borders).max(0.0);
                let content_h = (left_rect.height() - 10.0 - borders).max(0.0);
                ui.set_min_size(egui::vec2(content_w, content_h));
                ui.scope(|ui| {
                    configure_scroll_style(ui);
                    ui.add_enabled_ui(
                        !state.step1.installs_exactly_from_weidu_logs() && !state.step2.is_scanning,
                        |ui| render_mods_scroll(ui, state, action, details_open, palette),
                    );
                });
            });
    });
    if selection_before != selection_snapshot(state)
        && let Some(err) = crate::ui::step2::service_compat_rules_step2::apply_compat_rules(
            &state.step1,
            &mut state.step2.bgee_mods,
            &mut state.step2.bg2ee_mods,
        )
    {
        state.step2.scan_status = format!("Compat rules load failed: {err}");
    }
}

fn configure_scroll_style(ui: &mut egui::Ui) {
    let mut scroll = egui::style::ScrollStyle::floating();
    scroll.bar_width = 12.0;
    scroll.bar_inner_margin = 0.0;
    scroll.bar_outer_margin = 2.0;
    ui.style_mut().spacing.scroll = scroll;
}

fn render_mods_scroll(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    details_open: &mut bool,
    palette: ThemePalette,
) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_mods_scroll_content(ui, state, action, details_open, palette);
        });
}

fn render_mods_scroll_content(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    details_open: &mut bool,
    palette: ThemePalette,
) {
    let reserved_w = (ui.available_width() - SCROLLBAR_RESERVE).max(0.0);
    ui.set_max_width(reserved_w);

    let filter = state.step2.search_query.trim().to_lowercase();
    let active_tab = state.step2.active_game_tab.clone();
    let collapse_epoch = state.step2.collapse_epoch;
    let collapse_default_open = state.step2.collapse_default_open;
    let mut selected = state.step2.selected.clone();
    let mut next_selection_order = state.step2.next_selection_order;
    let mut jump_to_selected_requested = state.step2.jump_to_selected_requested;
    let mut prompt_popup: Option<(String, String)> = None;
    let prompt_eval = build_prompt_eval_context(state);
    let mods = active_mods_mut(&mut state.step2);
    if mods.is_empty() {
        ui.label("No mods scanned yet.");
    } else {
        let mut ctx = ActiveModsRenderContext {
            filter: &filter,
            active_tab: &active_tab,
            selected: &mut selected,
            next_selection_order: &mut next_selection_order,
            prompt_eval: &prompt_eval,
            collapse_epoch,
            collapse_default_open,
            jump_to_selected_requested: &mut jump_to_selected_requested,
            prompt_popup: &mut prompt_popup,
            details_open,
            palette,
        };
        render_active_mods(ui, mods, &mut ctx, action);
    }
    state.step2.selected = selected;
    state.step2.next_selection_order = next_selection_order;
    state.step2.jump_to_selected_requested = jump_to_selected_requested;
    if let Some((title, text)) = prompt_popup {
        crate::ui::step2::prompt_popup_step2::open_text_prompt_popup(state, title, text);
    }
}

struct ActiveModsRenderContext<'a> {
    filter: &'a str,
    active_tab: &'a str,
    selected: &'a mut Option<crate::app::state::Step2Selection>,
    next_selection_order: &'a mut usize,
    prompt_eval: &'a crate::parser::prompt_eval_expr::PromptEvalContext,
    collapse_epoch: u64,
    collapse_default_open: bool,
    jump_to_selected_requested: &'a mut bool,
    prompt_popup: &'a mut Option<(String, String)>,
    details_open: &'a mut bool,
    palette: ThemePalette,
}

fn render_active_mods(
    ui: &mut egui::Ui,
    mods: &mut [crate::app::state::Step2ModState],
    ctx: &mut ActiveModsRenderContext<'_>,
    action: &mut Option<Step2Action>,
) {
    let mut rendered_any = false;
    for mod_state in mods {
        if !mod_matches_filter(mod_state, ctx.filter) {
            continue;
        }
        rendered_any = true;
        render_one_mod(ui, mod_state, ctx, action);
        ui.add_space(6.0);
    }
    if !rendered_any {
        ui.label("No mods/components match your search.");
    }
}

fn render_one_mod(
    ui: &mut egui::Ui,
    mod_state: &mut crate::app::state::Step2ModState,
    ctx: &mut ActiveModsRenderContext<'_>,
    action: &mut Option<Step2Action>,
) {
    let mut render_ctx = ModTreeRenderContext {
        filter: ctx.filter,
        active_tab: ctx.active_tab,
        selected: ctx.selected,
        next_selection_order: ctx.next_selection_order,
        prompt_eval: ctx.prompt_eval,
        collapse_epoch: ctx.collapse_epoch,
        collapse_default_open: ctx.collapse_default_open,
        jump_to_selected_requested: ctx.jump_to_selected_requested,
        palette: ctx.palette,
    };
    if let Some(result) = render_mod_tree(ui, &mut render_ctx, mod_state) {
        apply_mod_tree_result(ctx, action, result);
    }
}

fn apply_mod_tree_result(
    ctx: &mut ActiveModsRenderContext<'_>,
    action: &mut Option<Step2Action>,
    result: ModTreeRenderResult,
) {
    let ModTreeRenderResult {
        selected,
        open_compat_for_component,
        open_prompt_popup,
        open_details,
    } = result;
    *ctx.selected = Some(selected);
    if open_details {
        *ctx.details_open = true;
    }
    if let Some((tp_file, component_id, component_key)) = open_compat_for_component {
        *action = Some(Step2Action::OpenCompatForComponent {
            game_tab: ctx.active_tab.to_string(),
            tp_file,
            component_id,
            component_key,
        });
    }
    if let Some((title, text)) = open_prompt_popup {
        *ctx.prompt_popup = Some((title, text));
    }
}

fn selection_snapshot(state: &WizardState) -> Vec<String> {
    let mut out = Vec::<String>::new();
    collect_tab_snapshot("BGEE", &state.step2.bgee_mods, &mut out);
    collect_tab_snapshot("BG2EE", &state.step2.bg2ee_mods, &mut out);
    out.sort();
    out
}

fn collect_tab_snapshot(
    tag: &str,
    mods: &[crate::app::state::Step2ModState],
    out: &mut Vec<String>,
) {
    for mod_state in mods {
        for component in &mod_state.components {
            if component.checked {
                out.push(format!(
                    "{tag}|{}|{}|{}",
                    mod_state.tp_file,
                    component.component_id,
                    component.selected_order.unwrap_or(usize::MAX)
                ));
            }
        }
    }
}
