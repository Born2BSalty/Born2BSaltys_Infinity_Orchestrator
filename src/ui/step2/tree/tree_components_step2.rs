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
    selected: Option<&Step2Selection>,
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

#[derive(Default)]
struct ComponentRowBuffers {
    selection: Option<Step2Selection>,
    compat_popup: CompatPopupTarget,
    prompt_popup: PromptPopupTarget,
    open_details: bool,
    enforce_single_select_for: Vec<usize>,
    enforce_collapsible_group_for: Vec<usize>,
    enforce_meta_for: Vec<usize>,
}

impl ComponentRowBuffers {
    const fn render_state(&mut self) -> ComponentRenderState<'_> {
        ComponentRenderState {
            selection: &mut self.selection,
            compat_popup: &mut self.compat_popup,
            prompt_popup: &mut self.prompt_popup,
            open_details: &mut self.open_details,
            enforce_single_select_for: &mut self.enforce_single_select_for,
            enforce_collapsible_group_for: &mut self.enforce_collapsible_group_for,
            enforce_meta_for: &mut self.enforce_meta_for,
        }
    }

    fn into_result(self) -> ComponentRowsResult {
        ComponentRowsResult {
            selection: self.selection,
            compat_popup: self.compat_popup,
            prompt_popup: self.prompt_popup,
            open_details: self.open_details,
        }
    }
}

#[derive(Clone, Copy)]
struct ComponentGroupRender<'a> {
    id_prefix: &'static str,
    header: &'a str,
    start_idx: usize,
    end_idx: usize,
    group_matches: bool,
    indent: f32,
}

pub(crate) fn render_component_rows(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
) -> ComponentRowsResult {
    let mod_name_match = ctx.filter.is_empty() || ctx.mod_name.to_lowercase().contains(ctx.filter);
    let mut buffers = ComponentRowBuffers::default();

    let mut component_idx = 0usize;
    while component_idx < mod_state.components.len() {
        if let Some(group_end) = render_weidu_group(
            ui,
            ctx,
            mod_state,
            &mut buffers,
            component_idx,
            mod_name_match,
        ) {
            component_idx = group_end;
        } else {
            component_idx =
                render_ungrouped_weidu_components(ui, ctx, mod_state, &mut buffers, component_idx);
        }
    }

    enforce_queued_component_rules(mod_state, &buffers);
    buffers.into_result()
}

fn render_weidu_group(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    buffers: &mut ComponentRowBuffers,
    component_idx: usize,
    mod_name_match: bool,
) -> Option<usize> {
    let header = mod_state.components[component_idx].weidu_group.clone()?;
    let group_end = matching_weidu_group_end(&mod_state.components, component_idx, &header);
    let group_matches = !ctx.filter.is_empty() && header.to_lowercase().contains(ctx.filter);
    let group = ComponentGroupRender {
        id_prefix: "step2_weidu_group",
        header: header.as_str(),
        start_idx: component_idx,
        end_idx: group_end,
        group_matches,
        indent: 0.0,
    };
    if !component_range_has_visible_row(ctx, mod_state, mod_name_match, &group) {
        return Some(group_end);
    }
    let header_id = egui::Id::new((
        group.id_prefix,
        ctx.collapse_epoch,
        ctx.tp_file,
        group.start_idx,
        group.header,
    ));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        header_id,
        ctx.collapse_default_open,
    );
    if *ctx.jump_to_selected_requested
        && selected_component_targets_range(
            ctx.selected.as_ref(),
            ctx.active_tab,
            ctx.tp_file,
            &mod_state.components[group.start_idx..group.end_idx],
        )
    {
        state.set_open(true);
    }
    state
        .show_header(ui, |ui| {
            ui.label(group.header);
        })
        .body(|ui| {
            let mut render_state = buffers.render_state();
            render_component_rows_range(
                ui,
                ctx,
                mod_state,
                group.start_idx,
                group.end_idx,
                &mut render_state,
            );
        });
    Some(group_end)
}

fn render_ungrouped_weidu_components(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    buffers: &mut ComponentRowBuffers,
    component_idx: usize,
) -> usize {
    let mut group_end = component_idx + 1;
    while group_end < mod_state.components.len()
        && mod_state.components[group_end].weidu_group.is_none()
    {
        group_end += 1;
    }
    let mut render_state = buffers.render_state();
    render_component_rows_range(
        ui,
        ctx,
        mod_state,
        component_idx,
        group_end,
        &mut render_state,
    );
    group_end
}

fn enforce_queued_component_rules(mod_state: &mut Step2ModState, buffers: &ComponentRowBuffers) {
    for idx in &buffers.enforce_single_select_for {
        enforce_subcomponent_single_select(mod_state, *idx);
    }
    for idx in &buffers.enforce_collapsible_group_for {
        enforce_collapsible_group_umbrella_rules(mod_state, *idx);
    }
    for idx in &buffers.enforce_meta_for {
        enforce_tp2_same_mod_exclusive_on_select(mod_state, *idx);
    }
    for idx in &buffers.enforce_meta_for {
        enforce_meta_mode_exclusive(mod_state, *idx);
    }
}

fn render_component_group(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    render_state: &mut ComponentRenderState<'_>,
    mod_name_match: bool,
    group: ComponentGroupRender<'_>,
) {
    if !component_range_has_visible_row(ctx, mod_state, mod_name_match, &group) {
        return;
    }
    let header_id = egui::Id::new((
        group.id_prefix,
        ctx.collapse_epoch,
        ctx.tp_file,
        group.start_idx,
        group.header,
    ));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        header_id,
        ctx.collapse_default_open,
    );
    if *ctx.jump_to_selected_requested
        && selected_component_targets_range(
            ctx.selected.as_ref(),
            ctx.active_tab,
            ctx.tp_file,
            &mod_state.components[group.start_idx..group.end_idx],
        )
    {
        state.set_open(true);
    }
    state
        .show_header(ui, |ui| {
            ui.label(group.header);
        })
        .body(|ui| {
            render_visible_component_rows(ui, ctx, mod_state, render_state, mod_name_match, &group);
        });
}

fn component_range_has_visible_row(
    ctx: &ComponentRowsContext<'_>,
    mod_state: &Step2ModState,
    mod_name_match: bool,
    group: &ComponentGroupRender<'_>,
) -> bool {
    (group.start_idx..group.end_idx).any(|idx| {
        component_matches_filter(
            mod_name_match,
            group.group_matches,
            &mod_state.components[idx],
            ctx.filter,
        )
    })
}

fn render_visible_component_rows(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    render_state: &mut ComponentRenderState<'_>,
    mod_name_match: bool,
    group: &ComponentGroupRender<'_>,
) {
    for idx in group.start_idx..group.end_idx {
        if component_matches_filter(
            mod_name_match,
            group.group_matches,
            &mod_state.components[idx],
            ctx.filter,
        ) {
            render_component_at(ui, ctx, mod_state, render_state, idx, group.indent);
        }
    }
}

fn matching_weidu_group_end(
    components: &[Step2ComponentState],
    component_idx: usize,
    header: &str,
) -> usize {
    let mut group_end = component_idx + 1;
    while group_end < components.len()
        && components[group_end]
            .weidu_group
            .as_deref()
            .is_some_and(|next| next.eq_ignore_ascii_case(header))
    {
        group_end += 1;
    }
    group_end
}

fn component_matches_filter(
    mod_name_match: bool,
    group_matches: bool,
    component: &Step2ComponentState,
    filter: &str,
) -> bool {
    mod_name_match || group_matches || component.label.to_lowercase().contains(filter)
}

fn render_component_at(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    render_state: &mut ComponentRenderState<'_>,
    component_idx: usize,
    indent: f32,
) {
    let mut ui_state = reborrow_render_state(render_state);
    render_component_row(
        ui,
        ctx,
        &mut ui_state,
        component_idx,
        &mut mod_state.components[component_idx],
        None,
        indent,
    );
}

fn render_collapsible_component_group(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    end_idx: usize,
    render_state: &mut ComponentRenderState<'_>,
    component_idx: usize,
    mod_name_match: bool,
) -> Option<usize> {
    let header = mod_state.components[component_idx]
        .collapsible_group
        .clone()?;
    let group_end = matching_collapsible_group_end(
        &mod_state.components,
        component_idx,
        end_idx,
        header.as_str(),
    );
    let group_matches = !ctx.filter.is_empty() && header.to_lowercase().contains(ctx.filter);
    render_component_group(
        ui,
        ctx,
        mod_state,
        render_state,
        mod_name_match,
        ComponentGroupRender {
            id_prefix: "step2_collapsible_group",
            header: header.as_str(),
            start_idx: component_idx,
            end_idx: group_end,
            group_matches,
            indent: 0.0,
        },
    );
    Some(group_end)
}

fn render_subcomponent_group(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    end_idx: usize,
    render_state: &mut ComponentRenderState<'_>,
    component_idx: usize,
    mod_name_match: bool,
) -> Option<usize> {
    let current_label = mod_state.components[component_idx].label.clone();
    let (header, _) = split_subcomponent_label(&current_label)?;
    let group_end = matching_subcomponent_group_end(mod_state, component_idx, end_idx, &header);
    if group_end - component_idx <= 1 {
        return None;
    }
    let group_matches = !ctx.filter.is_empty() && header.to_lowercase().contains(ctx.filter);
    render_component_group(
        ui,
        ctx,
        mod_state,
        render_state,
        mod_name_match,
        ComponentGroupRender {
            id_prefix: "step2_subgroup",
            header: header.as_str(),
            start_idx: component_idx,
            end_idx: group_end,
            group_matches,
            indent: 0.0,
        },
    );
    Some(group_end)
}

fn matching_collapsible_group_end(
    components: &[Step2ComponentState],
    component_idx: usize,
    end_idx: usize,
    header: &str,
) -> usize {
    let mut group_end = component_idx + 1;
    while group_end < end_idx
        && components[group_end]
            .collapsible_group
            .as_deref()
            .is_some_and(|next| next.eq_ignore_ascii_case(header))
    {
        group_end += 1;
    }
    group_end
}

fn matching_subcomponent_group_end(
    mod_state: &Step2ModState,
    component_idx: usize,
    end_idx: usize,
    header: &str,
) -> usize {
    let mut group_end = component_idx + 1;
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
    group_end
}

fn render_single_component(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
    render_state: &mut ComponentRenderState<'_>,
    component_idx: usize,
    mod_name_match: bool,
) {
    if component_matches_filter(
        mod_name_match,
        false,
        &mod_state.components[component_idx],
        ctx.filter,
    ) {
        render_component_at(ui, ctx, mod_state, render_state, component_idx, 0.0);
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
        if let Some(group_end) = render_collapsible_component_group(
            ui,
            ctx,
            mod_state,
            end_idx,
            render_state,
            component_idx,
            mod_name_match,
        ) {
            component_idx = group_end;
            continue;
        }
        if let Some(group_end) = render_subcomponent_group(
            ui,
            ctx,
            mod_state,
            end_idx,
            render_state,
            component_idx,
            mod_name_match,
        ) {
            component_idx = group_end;
            continue;
        }
        render_single_component(
            ui,
            ctx,
            mod_state,
            render_state,
            component_idx,
            mod_name_match,
        );
        component_idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::prompt_eval_expr::PromptEvalContext;
    use crate::ui::shared::redesign_tokens::ThemePalette;

    fn make_component(
        component_id: &str,
        label: &str,
        group: &str,
        family: &str,
    ) -> Step2ComponentState {
        Step2ComponentState {
            component_id: component_id.to_string(),
            label: label.to_string(),
            weidu_group: Some(group.to_string()),
            collapsible_group: Some(family.to_string()),
            collapsible_group_is_umbrella: false,
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked: false,
            selected_order: None,
        }
    }

    #[test]
    fn weidu_group_body_reaches_nested_collapsible_umbrella() {
        let ctx = egui::Context::default();
        ctx.memory_mut(|mem| mem.set_everything_is_visible(true));

        let collapse_epoch = 0u64;
        let tp_file = "setup-test.tp2";
        let weidu_header = "Engine";
        let family_header = "Family";

        let mut mod_state = Step2ModState {
            name: "TestMod".to_string(),
            tp_file: tp_file.to_string(),
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
            components: vec![
                make_component("1", "Sub A", weidu_header, family_header),
                make_component("2", "Sub B", weidu_header, family_header),
            ],
        };

        let prompt_eval = PromptEvalContext::default();
        let selected: Option<Step2Selection> = None;
        let mut next_selection_order = 1usize;
        let mut jump_requested = false;

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut rows_ctx = ComponentRowsContext {
                    filter: "",
                    active_tab: "BGEE",
                    selected: &selected,
                    next_selection_order: &mut next_selection_order,
                    prompt_eval: &prompt_eval,
                    collapse_epoch,
                    collapse_default_open: true,
                    jump_to_selected_requested: &mut jump_requested,
                    tp_file,
                    mod_name: "TestMod",
                    palette: ThemePalette::Dark,
                };
                let _ = render_component_rows(ui, &mut rows_ctx, &mut mod_state);
            });
        });

        let weidu_id = egui::Id::new((
            "step2_weidu_group",
            collapse_epoch,
            tp_file,
            0usize,
            weidu_header,
        ));
        assert!(
            egui::collapsing_header::CollapsingState::load(&ctx, weidu_id).is_some(),
            "WeiDU-group collapsing header was not stored — render_weidu_group did not run its body",
        );

        let nested_id = egui::Id::new((
            "step2_collapsible_group",
            collapse_epoch,
            tp_file,
            0usize,
            family_header,
        ));
        assert!(
            egui::collapsing_header::CollapsingState::load(&ctx, nested_id).is_some(),
            "Nested collapsible-group umbrella was not rendered inside the WeiDU group body — \
             umbrella detection regression (render_weidu_group must call render_component_rows_range)",
        );
    }
}
