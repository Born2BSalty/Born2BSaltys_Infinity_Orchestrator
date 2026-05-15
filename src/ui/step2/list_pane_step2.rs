// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BIO_SCROLL_BAR_WIDTH_PX, REDESIGN_BIO_SCROLL_INNER_MARGIN_PX,
    REDESIGN_BIO_SCROLL_OUTER_MARGIN_PX, ThemePalette, redesign_text_primary,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::service_list_ops_step2::mod_matches_filter;
use crate::ui::step2::state_step2::active_mods_mut;
use crate::ui::step2::tree_render_step2::{
    ModTreeRenderContext, ModTreeRenderResult, render_mod_tree,
};

pub(crate) fn render_list_pane(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    left_rect: egui::Rect,
    palette: ThemePalette,
) {
    let selection_before = selection_snapshot(state);
    ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
        ui.set_clip_rect(left_rect);
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin {
                left: 0,
                right: 6,
                top: 0,
                bottom: 0,
            })
            .show(ui, |ui| {
                ui.set_min_size(left_rect.size());
                render_list_scroll(ui, state, action, palette);
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

fn render_list_scroll(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    palette: ThemePalette,
) {
    ui.scope(|ui| {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = REDESIGN_BIO_SCROLL_BAR_WIDTH_PX;
        scroll.bar_inner_margin = REDESIGN_BIO_SCROLL_INNER_MARGIN_PX;
        scroll.bar_outer_margin = REDESIGN_BIO_SCROLL_OUTER_MARGIN_PX;
        ui.style_mut().spacing.scroll = scroll;
        ui.add_enabled_ui(
            !state.step1.installs_exactly_from_weidu_logs() && !state.step2.is_scanning,
            |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| render_list_content(ui, state, action, palette));
            },
        );
    });
}

fn render_list_content(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    palette: ThemePalette,
) {
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
        render_empty_list_label(ui, palette, "No mods scanned yet.");
    } else {
        let rendered_any = render_matching_mods(
            ui,
            action,
            ModListRenderState {
                mods,
                filter: &filter,
                active_tab: &active_tab,
                selected: &mut selected,
                next_selection_order: &mut next_selection_order,
                prompt_eval: &prompt_eval,
                collapse_epoch,
                collapse_default_open,
                jump_to_selected_requested: &mut jump_to_selected_requested,
                prompt_popup: &mut prompt_popup,
                palette,
            },
        );
        if !rendered_any {
            render_empty_list_label(ui, palette, "No mods/components match your search.");
        }
    }
    state.step2.selected = selected;
    state.step2.next_selection_order = next_selection_order;
    state.step2.jump_to_selected_requested = jump_to_selected_requested;
    if let Some((title, text)) = prompt_popup {
        crate::ui::step2::prompt_popup_step2::open_text_prompt_popup(state, title, text);
    }
}

struct ModListRenderState<'a> {
    mods: &'a mut [crate::app::state::Step2ModState],
    filter: &'a str,
    active_tab: &'a str,
    selected: &'a mut Option<crate::app::state::Step2Selection>,
    next_selection_order: &'a mut usize,
    prompt_eval: &'a crate::parser::PromptEvalContext,
    collapse_epoch: u64,
    collapse_default_open: bool,
    jump_to_selected_requested: &'a mut bool,
    prompt_popup: &'a mut Option<(String, String)>,
    palette: ThemePalette,
}

fn render_matching_mods(
    ui: &mut egui::Ui,
    action: &mut Option<Step2Action>,
    state: ModListRenderState<'_>,
) -> bool {
    let mut rendered_any = false;
    for mod_state in state.mods {
        if !mod_matches_filter(mod_state, state.filter) {
            continue;
        }
        rendered_any = true;
        let mut render_ctx = ModTreeRenderContext {
            filter: state.filter,
            active_tab: state.active_tab,
            selected: &*state.selected,
            next_selection_order: &mut *state.next_selection_order,
            prompt_eval: state.prompt_eval,
            collapse_epoch: state.collapse_epoch,
            collapse_default_open: state.collapse_default_open,
            jump_to_selected_requested: &mut *state.jump_to_selected_requested,
            palette: state.palette,
        };
        let maybe_selected = render_mod_tree(ui, &mut render_ctx, mod_state);
        if let Some(result) = maybe_selected {
            apply_mod_tree_result(
                action,
                state.active_tab,
                state.selected,
                state.prompt_popup,
                result,
            );
        }
        ui.add_space(6.0);
    }
    rendered_any
}

fn apply_mod_tree_result(
    action: &mut Option<Step2Action>,
    active_tab: &str,
    selected: &mut Option<crate::app::state::Step2Selection>,
    prompt_popup: &mut Option<(String, String)>,
    result: ModTreeRenderResult,
) {
    *selected = Some(result.selected);
    if let Some((tp_file, component_id, component_key)) = result.open_compat_for_component {
        *action = Some(Step2Action::OpenCompatForComponent {
            game_tab: active_tab.to_string(),
            tp_file,
            component_id,
            component_key,
        });
    }
    if let Some((title, text)) = result.open_prompt_popup {
        *prompt_popup = Some((title, text));
    }
}

fn render_empty_list_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        crate::ui::shared::typography_global::plain(text).color(redesign_text_primary(palette)),
    );
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
