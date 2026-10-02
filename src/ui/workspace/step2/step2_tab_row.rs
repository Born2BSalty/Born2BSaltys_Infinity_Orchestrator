// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::game_authority::{self, GameSlot};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, KebabItem, redesign_btn, render_kebab};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_accent_deep, redesign_pill_danger, redesign_pill_text,
    redesign_pill_warn,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::prompt_popup_step2::collect_step2_prompt_toolbar_entries;
use crate::ui::step2::state_step2::{
    applied_weidu_log_has_pending_downloads, non_scan_controls_locked, review_edit_scan_complete,
};
use crate::ui::step2::toolbar_actions_step2;
use crate::ui::step2::toolbar_compat_step2::{
    active_tab_compat_summary, first_active_tab_issue_target,
};
use crate::ui::workspace::state_workspace::WeiduLogImportForm;
use crate::ui::workspace::widgets::game_tab::game_tab;

const TAB_GAP: f32 = 4.0;
const ITEM_GAP: f32 = 8.0;
const ACTION_LEFT_PAD: f32 = 12.0;

fn active_mods(state: &crate::app::state::WizardState) -> &[crate::app::state::Step2ModState] {
    match game_authority::slot_for_tab(&state.step2.active_game_tab) {
        GameSlot::First => &state.step2.bgee_mods,
        GameSlot::Second => &state.step2.bg2ee_mods,
    }
}

pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    rect: egui::Rect,
) -> (Option<Step2Action>, Option<egui::Rect>) {
    let mut action: Option<Step2Action> = None;
    let row = Step2TabRowState::from_orchestrator(orchestrator);
    let mut active_tab_rect: Option<egui::Rect> = None;

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            active_tab_rect = render_game_tabs(ui, orchestrator, palette, &row);
            render_import_logs_button(ui, orchestrator, palette, &row);

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

    (action, active_tab_rect)
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
        let has_pending_log_downloads =
            applied_weidu_log_has_pending_downloads(&orchestrator.wizard_state);
        let updates = updates_state(&UpdatesInput {
            mode: UpdateMode {
                build_from_scanned,
                exact_log: exact_log_mode,
            },
            scan: UpdateScan {
                reviewed: reviewed_scan,
                completed: has_completed_scan,
                has_pending_log_downloads,
            },
            activity: ScanActivity::from_is_scanning(is_scanning),
        });

        Self {
            tabs: GameTabVisibility {
                tabs: game_authority::tabs_for_install(game),
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
            active_tab: ActiveTabState { name: active_tab },
            issue_target: first_active_tab_issue_target(
                active_mods(&orchestrator.wizard_state),
                &target_filter,
            ),
            issue_summary,
            prompt_count: crate::app::prompt_popup_text::prompt_toolbar_count(
                &collect_step2_prompt_toolbar_entries(&orchestrator.wizard_state),
            ),
            selected_count: orchestrator.wizard_state.step2.selected_count,
            total_count: orchestrator.wizard_state.step2.total_count,
            is_fork: orchestrator.workspace_view.fork_meta.is_some(),
            updates,
        }
    }
}

struct GameTabVisibility {
    tabs: &'static [&'static str],
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
    activity: ScanActivity,
}

struct UpdateMode {
    build_from_scanned: bool,
    exact_log: bool,
}

struct UpdateScan {
    reviewed: bool,
    completed: bool,
    has_pending_log_downloads: bool,
}

#[derive(Clone, Copy)]
enum ScanActivity {
    Idle,
    Scanning,
}

impl ScanActivity {
    const fn from_is_scanning(is_scanning: bool) -> Self {
        if is_scanning {
            Self::Scanning
        } else {
            Self::Idle
        }
    }

    const fn allows_updates(self) -> bool {
        matches!(self, Self::Idle)
    }
}

const fn updates_state(input: &UpdatesInput) -> UpdatesState {
    let log_update_context = input.scan.reviewed || input.scan.has_pending_log_downloads;
    if input.mode.build_from_scanned {
        UpdatesState {
            label: "Versions...",
            enabled: (input.scan.completed || input.scan.has_pending_log_downloads)
                && input.activity.allows_updates(),
        }
    } else if input.mode.exact_log {
        UpdatesState {
            label: "Mod List...",
            enabled: input.activity.allows_updates(),
        }
    } else {
        UpdatesState {
            label: "Versions...",
            enabled: log_update_context && input.activity.allows_updates(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ScanActivity, UpdateMode, UpdateScan, UpdatesInput, import_logs_button_enabled,
        updates_state,
    };

    fn review_edit_input(reviewed: bool, has_pending_log_downloads: bool) -> UpdatesInput {
        UpdatesInput {
            mode: UpdateMode {
                build_from_scanned: false,
                exact_log: false,
            },
            scan: UpdateScan {
                reviewed,
                completed: reviewed,
                has_pending_log_downloads,
            },
            activity: ScanActivity::Idle,
        }
    }

    fn scanned_input(completed: bool, has_pending_log_downloads: bool) -> UpdatesInput {
        UpdatesInput {
            mode: UpdateMode {
                build_from_scanned: true,
                exact_log: false,
            },
            scan: UpdateScan {
                reviewed: false,
                completed,
                has_pending_log_downloads,
            },
            activity: ScanActivity::Idle,
        }
    }

    #[test]
    fn review_edit_pending_log_downloads_enable_updates() {
        let state = updates_state(&review_edit_input(false, true));

        assert_eq!(state.label, "Versions...");
        assert!(state.enabled);
    }

    #[test]
    fn review_edit_without_scan_or_pending_downloads_disables_updates() {
        let state = updates_state(&review_edit_input(false, false));

        assert!(!state.enabled);
    }

    #[test]
    fn scanning_still_disables_pending_log_updates() {
        let mut input = review_edit_input(false, true);
        input.activity = ScanActivity::Scanning;

        let state = updates_state(&input);

        assert!(!state.enabled);
    }

    #[test]
    fn scanned_mode_pending_log_downloads_enable_updates() {
        let state = updates_state(&scanned_input(false, true));

        assert_eq!(state.label, "Versions...");
        assert!(state.enabled);
    }

    #[test]
    fn import_button_enabled_when_any_tab_is_scanned_or_bootstrappable() {
        assert!(!import_logs_button_enabled(false, false, false));
        assert!(import_logs_button_enabled(true, false, false));
        assert!(import_logs_button_enabled(false, true, false));
        assert!(import_logs_button_enabled(false, false, true));
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
) -> Option<egui::Rect> {
    ui.spacing_mut().item_spacing.x = TAB_GAP;
    let mut active_rect = None;
    for tab in row.tabs.tabs {
        if let Some(rect) = game_tab(
            ui,
            palette,
            tab,
            &mut orchestrator.wizard_state.step2.active_game_tab,
        ) {
            active_rect = Some(rect);
        }
    }
    ui.add_space(ACTION_LEFT_PAD - TAB_GAP);
    ui.spacing_mut().item_spacing.x = ITEM_GAP;
    active_rect
}

#[must_use]
pub(crate) const fn import_logs_button_enabled(
    bgee_scanned: bool,
    bg2_scanned: bool,
    can_bootstrap: bool,
) -> bool {
    bgee_scanned || bg2_scanned || can_bootstrap
}

fn render_import_logs_button(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    row: &Step2TabRowState,
) {
    if row.is_fork || row.modes.exact_log {
        return;
    }
    let enabled = import_logs_button_enabled(
        row.scans.bgee_scanned,
        row.scans.bg2_scanned,
        row.modes.can_bootstrap_from_log,
    );
    if redesign_btn(
        ui,
        palette,
        "Import from WeiDU Logs\u{2026}",
        BtnOpts {
            small: true,
            disabled: !enabled,
            ..Default::default()
        },
    )
    .on_hover_text(crate::ui::shared::tooltip_global::STEP2_IMPORT_WEIDU_LOGS)
    .clicked()
        && enabled
    {
        orchestrator.workspace_view.step2.weidu_log_import_form =
            Some(WeiduLogImportForm::default());
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
    .on_hover_text("Compare or fetch mod versions.")
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

const RIGHT_ACTIONS_BOTTOM_PAD: f32 = 4.0;

const STEP2_KEBAB_HEIGHT: f32 = 22.0;

fn render_right_actions(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    row: &Step2TabRowState,
) {
    let avail_w = ui.available_width();
    ui.vertical(|ui| {
        ui.set_width(avail_w);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let details_open = orchestrator.workspace_view.step2.details_open;
            let picked: std::cell::Cell<Option<KebabIntent>> = std::cell::Cell::new(None);
            {
                let mut items = kebab_items(details_open, &picked);
                render_kebab(
                    ui,
                    palette,
                    "step2_tab_row_kebab",
                    &mut items,
                    STEP2_KEBAB_HEIGHT,
                );
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
        ui.add_space(RIGHT_ACTIONS_BOTTOM_PAD);
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
    let size = egui::vec2(
        f32::mul_add(pad_x, 2.0, galley.size().x),
        f32::mul_add(pad_y, 2.0, galley.size().y),
    );
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
