// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{Step2ComponentState, Step2ModState, Step2Selection};
use crate::ui::step2::tree_component_row_step2::render_component_row;
use crate::ui::step2::tree_component_types_step2::{
    CompatPopupTarget, ComponentRenderState, ComponentRowsContext, ComponentRowsResult,
    PromptPopupTarget, reborrow_render_state,
};
use crate::ui::step2::tree_selection_rules_step2::{
    enforce_collapsible_group_umbrella_rules, enforce_meta_mode_exclusive,
    enforce_subcomponent_single_select, enforce_tp2_same_mod_exclusive_on_select,
    split_subcomponent_label,
};

fn selected_component_targets_range(
    selected: &Option<Step2Selection>,
    active_tab: &str,
    tp_file: &str,
    components: &[Step2ComponentState],
) -> bool {
    let Some(Step2Selection::Component {
        game_tab,
        tp_file: selected_tp,
        component_id,
        ..
    }) = selected
    else {
        return false;
    };
    if game_tab != active_tab || selected_tp != tp_file {
        return false;
    }
    components
        .iter()
        .any(|component| component.component_id == *component_id)
}

pub(crate) fn render_component_rows(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
) -> ComponentRowsResult {
    let mod_name_match = ctx.filter.is_empty() || ctx.mod_name.to_lowercase().contains(ctx.filter);
    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: CompatPopupTarget = None;
    let mut open_prompt_popup: PromptPopupTarget = None;
    let mut enforce_single_select_for = Vec::<usize>::new();
    let mut enforce_collapsible_group_for = Vec::<usize>::new();
    let mut enforce_meta_for = Vec::<usize>::new();

    let mut component_idx = 0usize;
    while component_idx < mod_state.components.len() {
        let current_weidu_group = mod_state.components[component_idx].weidu_group.clone();
        if let Some(header) = current_weidu_group {
            let mut group_end = component_idx + 1;
            while group_end < mod_state.components.len()
                && mod_state.components[group_end]
                    .weidu_group
                    .as_deref()
                    .is_some_and(|next| next.eq_ignore_ascii_case(&header))
            {
                group_end += 1;
            }

            let group_matches =
                !ctx.filter.is_empty() && header.to_lowercase().contains(ctx.filter);
            let any_child_visible = (component_idx..group_end).any(|idx| {
                mod_name_match
                    || group_matches
                    || mod_state.components[idx]
                        .label
                        .to_lowercase()
                        .contains(ctx.filter)
            });
            if any_child_visible {
                let header_id = egui::Id::new((
                    "step2_weidu_group",
                    ctx.collapse_epoch,
                    ctx.tp_file,
                    component_idx,
                    header.as_str(),
                ));
                let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    header_id,
                    ctx.collapse_default_open,
                );
                if *ctx.jump_to_selected_requested
                    && selected_component_targets_range(
                        ctx.selected,
                        ctx.active_tab,
                        ctx.tp_file,
                        &mod_state.components[component_idx..group_end],
                    )
                {
                    state.set_open(true);
                }
                state
                    .show_header(ui, |ui| {
                        ui.label(header.as_str());
                    })
                    .body(|ui| {
                        let mut render_state = ComponentRenderState {
                            selection: &mut new_selection,
                            compat_popup: &mut open_compat_for_component,
                            prompt_popup: &mut open_prompt_popup,
                            enforce_single_select_for: &mut enforce_single_select_for,
                            enforce_collapsible_group_for: &mut enforce_collapsible_group_for,
                            enforce_meta_for: &mut enforce_meta_for,
                        };
                        render_component_rows_range(
                            ui,
                            ctx,
                            mod_state,
                            component_idx,
                            group_end,
                            &mut render_state,
                        );
                    });
            }
            component_idx = group_end;
            continue;
        }

        let mut group_end = component_idx + 1;
        while group_end < mod_state.components.len()
            && mod_state.components[group_end].weidu_group.is_none()
        {
            group_end += 1;
        }
        let mut render_state = ComponentRenderState {
            selection: &mut new_selection,
            compat_popup: &mut open_compat_for_component,
            prompt_popup: &mut open_prompt_popup,
            enforce_single_select_for: &mut enforce_single_select_for,
            enforce_collapsible_group_for: &mut enforce_collapsible_group_for,
            enforce_meta_for: &mut enforce_meta_for,
        };
        render_component_rows_range(
            ui,
            ctx,
            mod_state,
            component_idx,
            group_end,
            &mut render_state,
        );
        component_idx = group_end;
    }

    for idx in enforce_single_select_for {
        enforce_subcomponent_single_select(mod_state, idx);
    }
    for idx in enforce_collapsible_group_for {
        enforce_collapsible_group_umbrella_rules(mod_state, idx);
    }
    for idx in &enforce_meta_for {
        enforce_tp2_same_mod_exclusive_on_select(mod_state, *idx);
    }
    for idx in enforce_meta_for {
        enforce_meta_mode_exclusive(mod_state, idx);
    }

    ComponentRowsResult {
        selection: new_selection,
        compat_popup: open_compat_for_component,
        prompt_popup: open_prompt_popup,
    }
}

fn render_component_rows_range(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    start_idx: usize,
    end_idx: usize,
    render_state: &mut ComponentRenderState<'_>,
) {
    let mod_name_match = ctx.filter.is_empty() || ctx.mod_name.to_lowercase().contains(ctx.filter);
    let mut component_idx = start_idx;
    while component_idx < end_idx {
        let current_component = &mod_state.components[component_idx];
        let current_label = current_component.label.clone();
        let collapsible_group = current_component.collapsible_group.clone();
        let subgroup = split_subcomponent_label(&current_label);

        if let Some(header) = collapsible_group {
            let mut group_end = component_idx + 1;
            while group_end < end_idx
                && mod_state.components[group_end]
                    .collapsible_group
                    .as_deref()
                    .is_some_and(|next| next.eq_ignore_ascii_case(&header))
            {
                group_end += 1;
            }

            let group_matches =
                !ctx.filter.is_empty() && header.to_lowercase().contains(ctx.filter);
            let any_child_visible = (component_idx..group_end).any(|idx| {
                mod_name_match
                    || group_matches
                    || mod_state.components[idx]
                        .label
                        .to_lowercase()
                        .contains(ctx.filter)
            });
            if any_child_visible {
                let header_id = egui::Id::new((
                    "step2_collapsible_group",
                    ctx.collapse_epoch,
                    ctx.tp_file,
                    component_idx,
                    header.as_str(),
                ));
                let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    header_id,
                    ctx.collapse_default_open,
                );
                if *ctx.jump_to_selected_requested
                    && selected_component_targets_range(
                        ctx.selected,
                        ctx.active_tab,
                        ctx.tp_file,
                        &mod_state.components[component_idx..group_end],
                    )
                {
                    state.set_open(true);
                }
                state
                    .show_header(ui, |ui| {
                        ui.label(header.as_str());
                    })
                    .body(|ui| {
                        for idx in component_idx..group_end {
                            let row_visible = mod_name_match
                                || group_matches
                                || mod_state.components[idx]
                                    .label
                                    .to_lowercase()
                                    .contains(ctx.filter);
                            if row_visible {
                                let mut ui_state = reborrow_render_state(render_state);
                                render_component_row(
                                    ui,
                                    ctx,
                                    &mut ui_state,
                                    idx,
                                    &mut mod_state.components[idx],
                                    None,
                                    0.0,
                                );
                            }
                        }
                    });
            }
            component_idx = group_end;
            continue;
        }

        let mut group_end = component_idx + 1;
        if let Some((header, _)) = subgroup.as_ref() {
            while group_end < end_idx {
                let next_label = mod_state.components[group_end].label.clone();
                let Some((next_header, _)) = split_subcomponent_label(&next_label) else {
                    break;
                };
                if !next_header.eq_ignore_ascii_case(header) {
                    break;
                }
                group_end += 1;
            }
        }

        let is_subgroup = subgroup.is_some() && group_end - component_idx > 1;
        if is_subgroup {
            let (header, _) = subgroup.unwrap_or_default();
            let subgroup_matches =
                !ctx.filter.is_empty() && header.to_lowercase().contains(ctx.filter);
            let any_child_visible = (component_idx..group_end).any(|idx| {
                mod_name_match
                    || subgroup_matches
                    || mod_state.components[idx]
                        .label
                        .to_lowercase()
                        .contains(ctx.filter)
            });
            if any_child_visible {
                let subgroup_indent = 0.0;
                let header_id = egui::Id::new((
                    "step2_subgroup",
                    ctx.collapse_epoch,
                    ctx.tp_file,
                    component_idx,
                    header.as_str(),
                ));
                let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    header_id,
                    ctx.collapse_default_open,
                );
                if *ctx.jump_to_selected_requested
                    && selected_component_targets_range(
                        ctx.selected,
                        ctx.active_tab,
                        ctx.tp_file,
                        &mod_state.components[component_idx..group_end],
                    )
                {
                    state.set_open(true);
                }
                state
                    .show_header(ui, |ui| {
                        ui.label(header.as_str());
                    })
                    .body(|ui| {
                        for idx in component_idx..group_end {
                            let row_visible = mod_name_match
                                || subgroup_matches
                                || mod_state.components[idx]
                                    .label
                                    .to_lowercase()
                                    .contains(ctx.filter);
                            if row_visible {
                                let mut ui_state = reborrow_render_state(render_state);
                                render_component_row(
                                    ui,
                                    ctx,
                                    &mut ui_state,
                                    idx,
                                    &mut mod_state.components[idx],
                                    None,
                                    subgroup_indent,
                                );
                            }
                        }
                    });
            }
            component_idx = group_end;
            continue;
        }

        let label = mod_state.components[component_idx].label.clone();
        let row_visible = mod_name_match || label.to_lowercase().contains(ctx.filter);
        if row_visible {
            let mut ui_state = reborrow_render_state(render_state);
            render_component_row(
                ui,
                ctx,
                &mut ui_state,
                component_idx,
                &mut mod_state.components[component_idx],
                None,
                0.0,
            );
        }
        component_idx += 1;
    }
}
