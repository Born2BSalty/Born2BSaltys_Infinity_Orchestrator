// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, KebabItem, redesign_btn, render_kebab};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_accent_deep, redesign_pill_danger, redesign_pill_text,
    redesign_pill_warn,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::prompt_popup_step2::collect_step2_prompt_toolbar_entries;
use crate::ui::step2::state_step2::{non_scan_controls_locked, review_edit_scan_complete};
use crate::ui::step2::toolbar_actions_step2;
use crate::ui::step2::toolbar_compat_step2::{
    active_tab_compat_summary, first_active_tab_issue_target,
};
use crate::ui::workspace::widgets::game_tab::game_tab;

const TAB_GAP: f32 = 4.0;
const ITEM_GAP: f32 = 8.0;
const ACTION_LEFT_PAD: f32 = 12.0;

fn active_mods(state: &crate::app::state::WizardState) -> &[crate::app::state::Step2ModState] {
    if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    }
}

pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    rect: egui::Rect,
) -> Option<Step2Action> {
    let mut action: Option<Step2Action> = None;
    let row = Step2TabRowState::from_orchestrator(orchestrator);

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            render_game_tabs(ui, orchestrator, palette, &row);
            render_log_buttons(ui, orchestrator, palette, &row);

            if render_updates_button(ui, palette, &row).clicked() && row.updates.enabled {
                action = Some(Step2Action::OpenUpdatePopup);
            }

            if let Some(issue) = render_issue_pill(ui, orchestrator, palette, &row)
                && issue.clicked()
            {
                toolbar_actions_step2::open_active_tab_issue(
                    &mut orchestrator.wizard_state,
                    &row.issue_summary,
                    row.issue_target.clone(),
                );
            }

            if render_prompt_pill(ui, palette, row.prompt_count).is_some_and(|r| r.clicked()) {
                toolbar_actions_step2::open_prompt_toolbar(&mut orchestrator.wizard_state);
            }

            render_right_actions(ui, orchestrator, palette, &row);
        });
    });

    action
}

struct Step2TabRowState {
    tabs: GameTabVisibility,
    scans: ScanStatus,
    modes: ModeFlags,
    active_tab: ActiveTabState,
    issue_summary: crate::ui::step2::toolbar_compat_step2::Step2ToolbarCompatSummary,
    issue_target: Option<crate::ui::step2::toolbar_compat_step2::Step2ToolbarIssueTarget>,
    prompt_count: usize,
    selected_count: usize,
    total_count: usize,
    is_fork: bool,
    updates: UpdatesState,
}

impl Step2TabRowState {
    fn from_orchestrator(orchestrator: &OrchestratorApp) -> Self {
        let game = orchestrator.wizard_state.step1.game_install.as_str();
        let bgee_scanned = !orchestrator.wizard_state.step2.bgee_mods.is_empty();
        let bg2_scanned = !orchestrator.wizard_state.step2.bg2ee_mods.is_empty();
        let reviewed_scan = review_edit_scan_complete(&orchestrator.wizard_state);
        let has_completed_scan = bgee_scanned || bg2_scanned || reviewed_scan;
        let exact_log_mode = orchestrator
            .wizard_state
            .step1
            .installs_exactly_from_weidu_logs();
        let can_bootstrap_from_log =
            can_bootstrap_from_log(orchestrator, exact_log_mode, has_completed_scan);
        let issue_summary = active_tab_compat_summary(active_mods(&orchestrator.wizard_state));
        let target_filter = issue_target_filter(orchestrator, &issue_summary);
        let active_tab = orchestrator.wizard_state.step2.active_game_tab.clone();
        let build_from_scanned = !orchestrator.wizard_state.step1.uses_source_weidu_logs();
        let is_scanning = orchestrator.wizard_state.step2.is_scanning;
        let updates = updates_state(&UpdatesInput {
            mode: UpdateMode {
                build_from_scanned,
                exact_log: exact_log_mode,
            },
            scan: UpdateScan {
                reviewed: reviewed_scan,
                completed: has_completed_scan,
                is_scanning,
            },
        });

        Self {
            tabs: GameTabVisibility {
                show_first_game: matches!(game, "BGEE" | "EET"),
                show_second_game: matches!(game, "BG2EE" | "EET"),
            },
            scans: ScanStatus {
                bgee_scanned,
                bg2_scanned,
                has_completed_scan,
            },
            modes: ModeFlags {
                ui_locked: non_scan_controls_locked(&orchestrator.wizard_state) || exact_log_mode,
                exact_log: exact_log_mode,
                can_bootstrap_from_log,
            },
            active_tab: ActiveTabState {
                name: active_tab.clone(),
                is_bgee: active_tab == "BGEE",
                is_bg2: active_tab == "BG2EE",
            },
            issue_target: first_active_tab_issue_target(
                active_mods(&orchestrator.wizard_state),
                &target_filter,
            ),
            issue_summary,
            prompt_count: collect_step2_prompt_toolbar_entries(&orchestrator.wizard_state)
                .iter()
                .map(|e| e.component_ids.len())
                .sum(),
            selected_count: orchestrator.wizard_state.step2.selected_count,
            total_count: orchestrator.wizard_state.step2.total_count,
            is_fork: orchestrator.workspace_view.fork_meta.is_some(),
            updates,
        }
    }
}

struct GameTabVisibility {
    show_first_game: bool,
    show_second_game: bool,
}

struct ScanStatus {
    bgee_scanned: bool,
    bg2_scanned: bool,
    has_completed_scan: bool,
}

struct ModeFlags {
    ui_locked: bool,
    exact_log: bool,
    can_bootstrap_from_log: bool,
}

struct ActiveTabState {
    name: String,
    is_bgee: bool,
    is_bg2: bool,
}

struct UpdatesState {
    label: &'static str,
    enabled: bool,
}

fn can_bootstrap_from_log(
    orchestrator: &OrchestratorApp,
    exact_log_mode: bool,
    has_completed_scan: bool,
) -> bool {
    if exact_log_mode {
        false
    } else if scratch_workspace_has_mods_folder(orchestrator) {
        true
    } else if orchestrator.wizard_state.step1.bootstraps_from_weidu_logs() {
        review_edit_scan_complete(&orchestrator.wizard_state)
    } else {
        has_completed_scan
    }
}

fn scratch_workspace_has_mods_folder(orchestrator: &OrchestratorApp) -> bool {
    let id = orchestrator.workspace_view.modlist_id.trim();
    if id.is_empty() {
        return false;
    }
    orchestrator
        .workspace_state
        .get(id)
        .and_then(|workspace| workspace.scratch_mods_folder.as_deref())
        .is_some_and(|folder| !folder.trim().is_empty())
}

struct UpdatesInput {
    mode: UpdateMode,
    scan: UpdateScan,
}

struct UpdateMode {
    build_from_scanned: bool,
    exact_log: bool,
}

struct UpdateScan {
    reviewed: bool,
    completed: bool,
    is_scanning: bool,
}

const fn updates_state(input: &UpdatesInput) -> UpdatesState {
    if input.mode.build_from_scanned {
        UpdatesState {
            label: "Updates...",
            enabled: input.scan.completed && !input.scan.is_scanning,
        }
    } else if input.mode.exact_log {
        UpdatesState {
            label: "Mod List...",
            enabled: !input.scan.is_scanning,
        }
    } else {
        UpdatesState {
            label: "Updates...",
            enabled: input.scan.reviewed && !input.scan.is_scanning,
        }
    }
}

fn issue_target_filter(
    orchestrator: &OrchestratorApp,
    summary: &crate::ui::step2::toolbar_compat_step2::Step2ToolbarCompatSummary,
) -> String {
    if orchestrator
        .wizard_state
        .step2
        .compat_popup_filter
        .eq_ignore_ascii_case("All")
    {
        summary.dominant_filter.to_owned()
    } else {
        orchestrator.wizard_state.step2.compat_popup_filter.clone()
    }
}

fn render_game_tabs(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    row: &Step2TabRowState,
) {
    ui.spacing_mut().item_spacing.x = TAB_GAP;
    if row.tabs.show_first_game {
        game_tab(
            ui,
            palette,
            "BGEE",
            &mut orchestrator.wizard_state.step2.active_game_tab,
        );
    }
    if row.tabs.show_second_game {
        game_tab(
            ui,
            palette,
            "BG2EE",
            &mut orchestrator.wizard_state.step2.active_game_tab,
        );
    }
    ui.add_space(ACTION_LEFT_PAD - TAB_GAP);
    ui.spacing_mut().item_spacing.x = ITEM_GAP;
}

fn render_log_buttons(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    row: &Step2TabRowState,
) {
    if row.is_fork || row.modes.exact_log {
        return;
    }
    if row.active_tab.is_bgee {
        render_log_button(
            ui,
            orchestrator,
            palette,
            "Select BGEE via WeiDU Log",
            true,
            row,
        );
    } else if row.active_tab.is_bg2 {
        render_log_button(
            ui,
            orchestrator,
            palette,
            "Select BG2EE via WeiDU Log",
            false,
            row,
        );
    }
}

fn render_log_button(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    label: &str,
    bgee: bool,
    row: &Step2TabRowState,
) {
    let enabled = if bgee {
        row.scans.bgee_scanned
    } else {
        row.scans.bg2_scanned
    } || row.modes.can_bootstrap_from_log;
    let tooltip = if bgee {
        crate::ui::shared::tooltip_global::STEP2_SELECT_BGEE_LOG
    } else {
        crate::ui::shared::tooltip_global::STEP2_SELECT_BG2EE_LOG
    };
    if redesign_btn(
        ui,
        palette,
        label,
        BtnOpts {
            small: true,
            disabled: !enabled,
            ..Default::default()
        },
    )
    .on_hover_text(tooltip)
    .clicked()
        && enabled
    {
        orchestrator.workspace_view.step2.pending_weidu_log_confirm = Some(bgee);
    }
}

fn render_updates_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    row: &Step2TabRowState,
) -> egui::Response {
    redesign_btn(
        ui,
        palette,
        row.updates.label,
        BtnOpts {
            small: true,
            disabled: !row.updates.enabled,
            ..Default::default()
        },
    )
    .on_hover_text("Open the updates popup.")
}

fn render_issue_pill(
    ui: &mut egui::Ui,
    orchestrator: &OrchestratorApp,
    palette: ThemePalette,
    row: &Step2TabRowState,
) -> Option<egui::Response> {
    if row.issue_summary.total_count == 0 {
        return None;
    }
    let display_filter = display_filter(orchestrator, row);
    let display_count = display_count(orchestrator, row);
    let issue_word = if row.issue_summary.total_count == 1 {
        "issue"
    } else {
        "issues"
    };
    let tip = format!(
        "{} compatibility {} in the {} Step 2 tab. Active badge category: {} ({}). Dominant category: {} ({}).",
        row.issue_summary.total_count,
        issue_word,
        row.active_tab.name,
        display_filter,
        display_count,
        row.issue_summary.dominant_filter,
        row.issue_summary.dominant_count
    );
    Some(clickable_pill(
        ui,
        palette,
        &format!(
            "{} {} {}",
            row.active_tab.name, display_filter, display_count
        ),
        redesign_pill_danger(palette),
        &tip,
    ))
}

fn display_filter<'a>(orchestrator: &'a OrchestratorApp, row: &'a Step2TabRowState) -> &'a str {
    if orchestrator
        .wizard_state
        .step2
        .compat_popup_filter
        .eq_ignore_ascii_case("All")
    {
        row.issue_summary.dominant_filter
    } else {
        orchestrator.wizard_state.step2.compat_popup_filter.as_str()
    }
}

fn display_count(orchestrator: &OrchestratorApp, row: &Step2TabRowState) -> usize {
    if orchestrator
        .wizard_state
        .step2
        .compat_popup_filter
        .eq_ignore_ascii_case("All")
    {
        row.issue_summary.dominant_count
    } else {
        row.issue_summary.total_count
    }
}

fn render_prompt_pill(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    prompt_count: usize,
) -> Option<egui::Response> {
    (prompt_count > 0).then(|| {
        clickable_pill(
            ui,
            palette,
            &format!("PROMPT {prompt_count}"),
            redesign_pill_warn(palette),
            crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS,
        )
    })
}

fn render_right_actions(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    row: &Step2TabRowState,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let details_open = orchestrator.workspace_view.step2.details_open;
        let picked: std::cell::Cell<Option<KebabIntent>> = std::cell::Cell::new(None);
        {
            let mut items = kebab_items(details_open, &picked);
            render_kebab(ui, palette, "step2_tab_row_kebab", &mut items);
        }
        if let Some(intent) = picked.get() {
            apply_kebab_intent(
                intent,
                orchestrator,
                row.modes.ui_locked,
                row.scans.has_completed_scan,
            );
        }

        ui.add_space(ITEM_GAP);
        ui.label(
            egui::RichText::new(format!(
                "{} / {} on {}",
                row.selected_count, row.total_count, row.active_tab.name
            ))
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_accent_deep(palette)),
        );
    });
}

fn kebab_items(
    details_open: bool,
    picked: &std::cell::Cell<Option<KebabIntent>>,
) -> [KebabItem<'_>; 6] {
    [
        KebabItem::new(details_label(details_open), || {
            picked.set(Some(KebabIntent::ToggleDetails));
        }),
        KebabItem::new("Clear All", || {
            picked.set(Some(KebabIntent::ClearAll));
        }),
        KebabItem::new("Select Visible", || {
            picked.set(Some(KebabIntent::SelectVisible));
        }),
        KebabItem::new("Collapse All", || {
            picked.set(Some(KebabIntent::CollapseAll));
        }),
        KebabItem::new("Expand All", || {
            picked.set(Some(KebabIntent::ExpandAll));
        }),
        KebabItem::new("Jump to Selected", || {
            picked.set(Some(KebabIntent::JumpToSelected));
        }),
    ]
}

#[derive(Clone, Copy)]
enum KebabIntent {
    ToggleDetails,
    ClearAll,
    SelectVisible,
    CollapseAll,
    ExpandAll,
    JumpToSelected,
}

fn apply_kebab_intent(
    intent: KebabIntent,
    orchestrator: &mut OrchestratorApp,
    ui_locked: bool,
    has_completed_scan: bool,
) {
    let state = &mut orchestrator.wizard_state;
    match intent {
        KebabIntent::ToggleDetails => {
            orchestrator.workspace_view.step2.details_open =
                !orchestrator.workspace_view.step2.details_open;
        }
        KebabIntent::ClearAll => {
            let has_any_checked = active_mods(state)
                .iter()
                .any(|m| m.checked || m.components.iter().any(|c| c.checked));
            if has_completed_scan && !ui_locked && has_any_checked {
                toolbar_actions_step2::clear_all_and_refresh_compat(state);
            }
        }
        KebabIntent::SelectVisible => {
            if has_completed_scan && !ui_locked {
                toolbar_actions_step2::select_visible_and_refresh_compat(state);
            }
        }
        KebabIntent::CollapseAll => {
            if has_completed_scan && !ui_locked {
                toolbar_actions_step2::collapse_all(state);
            }
        }
        KebabIntent::ExpandAll => {
            if has_completed_scan && !ui_locked {
                toolbar_actions_step2::expand_all(state);
            }
        }
        KebabIntent::JumpToSelected => {
            if state.step2.selected.is_some() && !ui_locked {
                state.step2.jump_to_selected_requested = true;
            }
        }
    }
}

const fn details_label(open: bool) -> &'static str {
    if open {
        "Hide Details panel"
    } else {
        "Show Details panel"
    }
}

fn clickable_pill(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    fill: egui::Color32,
    tooltip: &str,
) -> egui::Response {
    let pad_x = 6.0;
    let pad_y = 1.0;
    let text_color = redesign_pill_text(palette);
    let font = egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let size = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::same(7), fill);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }
    response.on_hover_text(tooltip.to_owned())
}
