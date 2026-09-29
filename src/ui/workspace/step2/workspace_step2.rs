// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::app_step2_log::{resolve_bg2_weidu_log_path, resolve_bgee_weidu_log_path};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHELL_BORDER_WIDTH_PX,
    ThemePalette, WORKSPACE_CONTENT_TEXT_INSET, redesign_accent, redesign_border_strong,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::shared::tab_open_seam::paint_active_tab_seam_cover;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::state_workspace::WeiduLogImportForm;
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::step2::step2_log_import_dialog::{self, ImportOutcome};
use crate::ui::workspace::step2::{
    step2_global_mods_confirm, step2_rescan_reconcile, step2_search, step2_tab_row,
};

const TITLE_H: f32 = 24.0;
const TITLE_GAP: f32 = 8.0;
const SEARCH_H: f32 = 30.0;
const SEARCH_GAP: f32 = 10.0;
const TAB_ROW_H: f32 = 30.0;
const TAB_TO_GRID_OVERLAP: f32 = 1.5;
const GRID_GAP: f32 = 12.0;
const LEFT_MIN_W: f32 = 420.0;
const DETAILS_W: f32 = 560.0;
const DETAILS_MIN_W: f32 = 420.0;
const CONTENT_MIN_H: f32 = 160.0;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> Option<Step2Action> {
    let palette = orchestrator.theme_palette;

    crate::ui::step2::state_step2::normalize_active_tab(&mut orchestrator.wizard_state);

    let content_rect = ui.available_rect_before_wrap();
    let rects = Step2LayoutRects::from_root(content_rect);
    let mut action: Option<Step2Action> = None;

    render_title(ui, palette, rects.title);

    if let Some(a) = step2_search::render(ui, orchestrator, palette, rects.search, content_rect) {
        action = Some(a);
    }

    let (tab_action, active_tab_rect) =
        step2_tab_row::render(ui, orchestrator, palette, rects.tab_row);
    if let Some(a) = tab_action {
        action = Some(a);
    }

    let details_open = orchestrator.workspace_view.step2.details_open;
    let preferred_details_w = orchestrator
        .redesign_settings
        .step2_details_width
        .map(f32::from);
    let panes = Step2PaneRects::from_content(rects.content, details_open, preferred_details_w);

    ui.painter().rect_filled(
        panes.left,
        egui::CornerRadius::ZERO,
        redesign_shell_bg(palette),
    );

    clipped_pane(ui, panes.left, |ui| {
        ui.visuals_mut().widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
        ui.add_space(8.0);
        crate::ui::step2::list_pane_step2::render_list_pane(
            ui,
            &mut orchestrator.wizard_state,
            &mut action,
            panes.left,
            &mut orchestrator.workspace_view.step2.details_open,
            palette,
        );
    });

    if let Some(tab_rect) = active_tab_rect {
        paint_active_tab_seam_cover(ui.painter(), palette, tab_rect, rects.content.top());
    }

    if let Some(right_rect) = panes.right {
        clipped_pane(ui, right_rect, |ui| {
            crate::ui::step2::details_pane_step2::render_pane(
                ui,
                &mut orchestrator.wizard_state,
                &mut action,
                right_rect,
                palette,
                &mut orchestrator.workspace_view.step2.details_open,
            );
        });
        paint_details_panel_border(ui, palette, right_rect);
    }

    if let Some(gap) = panes.splitter {
        render_details_splitter(ui, orchestrator, palette, rects.content, gap);
    }

    sync_details_selection(orchestrator);

    let ctx = ui.ctx().clone();
    render_popups(ui, orchestrator, &ctx, &mut action, palette);
    if let Some(a) = render_weidu_log_import_form(orchestrator, &ctx) {
        action = Some(a);
    }
    if let Some(a) = render_global_mods_scan_confirm(orchestrator, &ctx) {
        action = Some(a);
    }

    crate::ui::step2::service_list_ops_step2::recompute_selection_counts(
        &mut orchestrator.wizard_state,
    );

    action
}

#[derive(Clone, Copy)]
struct Step2LayoutRects {
    title: egui::Rect,
    search: egui::Rect,
    tab_row: egui::Rect,
    content: egui::Rect,
}

impl Step2LayoutRects {
    fn from_root(root: egui::Rect) -> Self {
        let x = root.left();
        let w = root.width();
        let mut y = root.top();

        let title = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TITLE_H));
        y += TITLE_H + TITLE_GAP;

        let search = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, SEARCH_H));
        y += SEARCH_H + SEARCH_GAP;

        let tab_row = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TAB_ROW_H));
        y += TAB_ROW_H - TAB_TO_GRID_OVERLAP;

        let content_h = (root.bottom() - y).max(CONTENT_MIN_H);
        let content = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, content_h));

        Self {
            title,
            search,
            tab_row,
            content,
        }
    }
}

#[derive(Clone, Copy)]
struct Step2PaneRects {
    left: egui::Rect,
    right: Option<egui::Rect>,
    splitter: Option<egui::Rect>,
}

impl Step2PaneRects {
    fn from_content(
        content: egui::Rect,
        details_open: bool,
        preferred_details_w: Option<f32>,
    ) -> Self {
        if !details_open {
            return Self {
                left: content,
                right: None,
                splitter: None,
            };
        }
        let usable_w = (content.width() - GRID_GAP).max(0.0);
        let (right_w, splitter_allowed) = if usable_w >= LEFT_MIN_W + DETAILS_MIN_W {
            let right_w = preferred_details_w.map_or_else(
                || DETAILS_W.min(usable_w - LEFT_MIN_W).max(DETAILS_MIN_W),
                |w| clamp_details_width(w, usable_w),
            );
            (right_w, true)
        } else {
            let max_right_w = usable_w.min(DETAILS_W);
            let min_right_w = DETAILS_MIN_W.min(max_right_w);
            ((usable_w * 0.56).clamp(min_right_w, max_right_w), false)
        };
        let left_w = (content.width() - GRID_GAP - right_w).max(0.0);
        let left = egui::Rect::from_min_size(content.min, egui::vec2(left_w, content.height()));
        let right = egui::Rect::from_min_size(
            egui::pos2(left.right() + GRID_GAP, content.top()),
            egui::vec2(right_w, content.height()),
        );
        let splitter = splitter_allowed.then(|| {
            egui::Rect::from_min_max(
                egui::pos2(left.right(), content.top()),
                egui::pos2(right.left(), content.bottom()),
            )
        });
        Self {
            left,
            right: Some(right),
            splitter,
        }
    }
}

fn clamp_details_width(preferred: f32, usable_w: f32) -> f32 {
    preferred.clamp(DETAILS_MIN_W, usable_w - LEFT_MIN_W)
}

fn render_details_splitter(
    ui: &egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    content: egui::Rect,
    gap: egui::Rect,
) {
    let resp = ui.interact(
        gap,
        ui.id().with("workspace_step2_details_splitter"),
        egui::Sense::click_and_drag(),
    );

    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    if resp.double_clicked() {
        orchestrator.redesign_settings.step2_details_width = None;
    } else if resp.dragged()
        && let Some(pointer) = ui.ctx().pointer_latest_pos()
    {
        let usable_w = (content.width() - GRID_GAP).max(0.0);
        let new_w = clamp_details_width(content.right() - pointer.x - GRID_GAP / 2.0, usable_w);
        if let Some(width) = crate::ui::shared::numeric::u16_from_f32(new_w) {
            orchestrator.redesign_settings.step2_details_width = Some(width);
        }
    }

    let grip_color = if resp.hovered() || resp.dragged() {
        redesign_accent(palette)
    } else {
        redesign_border_strong(palette)
    };
    let grip_rect = egui::Rect::from_center_size(gap.center(), egui::vec2(2.0, 32.0));
    ui.painter()
        .rect_filled(grip_rect, egui::CornerRadius::same(1), grip_color);
}

const HELP_TITLE: &str = "Adding mods to this modlist";
const HELP_STEP_1: &str = "1. Download any mods you want to add.";
const HELP_STEP_2: &str = "2. Extract them to this modlist's \"mods\" folder: ";
const HELP_STEP_2_GLOBAL: &str = "2. Extract them to your global Mods folder: ";
const HELP_STEP_3: &str = "3. Click Rescan Mods to pick it up.";
const HELP_OPEN_BUTTON: &str = "Open Mods folder";

fn render_title(ui: &mut egui::Ui, palette: ThemePalette, rect: egui::Rect) {
    let title_text_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(WORKSPACE_CONTENT_TEXT_INSET, 0.0),
        rect.max,
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(title_text_rect), |ui| {
        ui.label(
            egui::RichText::new("Mods / Components")
                .size(15.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
    });
}

pub(super) fn render_help_popover(
    ui: &egui::Ui,
    palette: ThemePalette,
    anchor: &egui::Response,
    constrain_to: egui::Rect,
    mods_folder: &str,
    global_source: bool,
) {
    let popup_id = ui.make_persistent_id("workspace_step2_add_mods_help");
    if !ui.memory(|memory| memory.is_popup_open(popup_id)) {
        return;
    }

    let mut anchor_pos = anchor.rect.right_bottom();
    if let Some(to_global) = ui.ctx().layer_transform_to_global(ui.layer_id()) {
        anchor_pos = to_global * anchor_pos;
    }

    let popup_frame = egui::Frame::popup(ui.style());
    let area_response = egui::Area::new(popup_id)
        .kind(egui::UiKind::Popup)
        .order(egui::Order::Foreground)
        .fixed_pos(anchor_pos)
        .pivot(egui::Align2::RIGHT_TOP)
        .constrain_to(constrain_to)
        .show(ui.ctx(), |ui| {
            ui.set_max_width(320.0);
            popup_frame.show(ui, |ui| {
                egui::Frame::default()
                    .fill(redesign_shell_bg(palette))
                    .stroke(egui::Stroke::new(
                        REDESIGN_BORDER_WIDTH_PX,
                        redesign_border_strong(palette),
                    ))
                    .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        let wrap_width = ui.available_width();
                        ui.label(
                            egui::RichText::new(HELP_TITLE)
                                .size(12.0)
                                .family(egui::FontFamily::Name("poppins_medium".into()))
                                .color(redesign_text_primary(palette)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(HELP_STEP_1)
                                .size(12.0)
                                .family(egui::FontFamily::Name("poppins_light".into()))
                                .color(redesign_text_muted(palette)),
                        );
                        ui.label(help_step_2_job(
                            palette,
                            mods_folder,
                            wrap_width,
                            global_source,
                        ));
                        ui.label(
                            egui::RichText::new(HELP_STEP_3)
                                .size(12.0)
                                .family(egui::FontFamily::Name("poppins_light".into()))
                                .color(redesign_text_muted(palette)),
                        );
                        ui.add_space(8.0);
                        let open_response = redesign_btn(
                            ui,
                            palette,
                            HELP_OPEN_BUTTON,
                            BtnOpts {
                                small: true,
                                disabled: mods_folder.is_empty(),
                                ..BtnOpts::default()
                            },
                        );
                        if open_response.clicked()
                            && let Err(err) =
                                crate::app::controller::util::open_in_shell(mods_folder)
                        {
                            tracing::warn!(
                                target = "orchestrator",
                                "Open Mods folder failed: {err}"
                            );
                        }
                    });
            });
        });

    let should_close = anchor.clicked_elsewhere() && area_response.response.clicked_elsewhere();
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
        ui.memory_mut(egui::Memory::close_popup);
    }
}

fn help_step_2_job(
    palette: ThemePalette,
    mods_folder: &str,
    wrap_width: f32,
    global_source: bool,
) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.append(
        if global_source {
            HELP_STEP_2_GLOBAL
        } else {
            HELP_STEP_2
        },
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into())),
            color: redesign_text_muted(palette),
            ..Default::default()
        },
    );
    job.append(
        mods_folder,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(11.0, egui::FontFamily::Monospace),
            color: redesign_text_muted(palette),
            ..Default::default()
        },
    );
    egui::WidgetText::LayoutJob(job)
}

fn sync_details_selection(orchestrator: &mut OrchestratorApp) {
    let live_sel = orchestrator.wizard_state.step2.selected.clone();
    orchestrator.workspace_view.step2.last_selection = live_sel;
}

fn paint_details_panel_border(
    ui: &egui::Ui,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
    rect: egui::Rect,
) {
    ui.painter().rect_stroke(
        rect.shrink(1.0),
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
        egui::Stroke::new(
            REDESIGN_SHELL_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ),
        egui::StrokeKind::Inside,
    );
}

fn render_popups(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
    action: &mut Option<Step2Action>,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
) {
    let outcome =
        crate::ui::step2::compat_window_step2::render(ui, &mut orchestrator.wizard_state, palette);
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, &mut orchestrator.wizard_state);
    crate::ui::workspace::related_jump::apply_related_jump_outcome(orchestrator, outcome);

    if !orchestrator.wizard_state.step2.update_selected_popup_open {
        orchestrator.wizard_state.step2.versions_ui.menu = None;
        orchestrator.wizard_state.step2.versions_ui.sheet = None;
    }

    if orchestrator.wizard_state.step2.update_selected_popup_open {
        crate::ui::workspace::versions::versions_drawer::render(ctx, orchestrator, action, palette);
    }
}

fn clipped_pane(ui: &mut egui::Ui, rect: egui::Rect, add: impl FnOnce(&mut egui::Ui)) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let clip = rect.intersect(ui.clip_rect());
    child.set_clip_rect(clip);
    add(&mut child);
    ui.allocate_rect(rect, egui::Sense::hover());
}

fn render_weidu_log_import_form(
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
) -> Option<Step2Action> {
    let mut form = orchestrator
        .workspace_view
        .step2
        .weidu_log_import_form
        .clone()?;
    let step1 = &orchestrator.wizard_state.step1;
    let start_paths = WeiduLogImportForm {
        first: resolve_bgee_weidu_log_path(step1),
        second: resolve_bg2_weidu_log_path(step1),
    };

    let outcome = step2_log_import_dialog::render(
        ctx,
        orchestrator.theme_palette,
        &step1.game_install,
        &mut form,
        &start_paths,
    );

    match outcome {
        ImportOutcome::Import => {
            orchestrator.workspace_view.step2.weidu_log_import_form = Some(form);
            Some(Step2Action::ImportWeiduLogs)
        }
        ImportOutcome::Cancelled => {
            orchestrator.workspace_view.step2.weidu_log_import_form = None;
            None
        }
        ImportOutcome::Pending => {
            orchestrator.workspace_view.step2.weidu_log_import_form = Some(form);
            None
        }
    }
}

fn render_global_mods_scan_confirm(
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
) -> Option<Step2Action> {
    use crate::registry::workspace_model::ModsSource;

    orchestrator.workspace_view.step2.pending_global_mods_scan?;

    let dialog = step2_global_mods_confirm::global_mods_scan_confirm();
    let outcome = confirm_dialog::render(ctx, orchestrator.theme_palette, &dialog);

    match outcome {
        ConfirmOutcome::Confirmed => {
            orchestrator.workspace_view.step2.pending_global_mods_scan = None;
            let modlist_id = orchestrator.workspace_view.modlist_id.trim().to_string();
            let current_source = orchestrator
                .workspace_state
                .get(modlist_id.as_str())
                .map_or_else(ModsSource::default, |w| w.mods_source);
            let folder = match current_source {
                ModsSource::GlobalModsFolder => orchestrator
                    .settings_store
                    .load()
                    .ok()
                    .map(|s| s.step1.effective_global_mods_folder().to_string())
                    .unwrap_or_default(),
                ModsSource::InstallationFolder => orchestrator
                    .workspace_state
                    .get(modlist_id.as_str())
                    .and_then(|w| w.scratch_mods_folder.clone())
                    .unwrap_or_default(),
            };
            orchestrator.wizard_state.step1.mods_folder = folder;
            step2_rescan_reconcile::snapshot_current_selection(orchestrator);
            if let Some(workspace) = orchestrator.workspace_state.get_mut(modlist_id.as_str()) {
                workspace.last_rescanned_mods_source = current_source;
            }
            orchestrator.mark_workspace_dirty();
            step_action_dispatch::dispatch_step2(Step2Action::StartScan, orchestrator);
            None
        }
        ConfirmOutcome::Cancelled => {
            orchestrator.workspace_view.step2.pending_global_mods_scan = None;
            None
        }
        ConfirmOutcome::Pending => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    fn content_rect(width: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width, 400.0))
    }

    #[test]
    fn details_default_width_unchanged_without_preference() {
        let panes = Step2PaneRects::from_content(content_rect(1400.0), true, None);
        let right = panes.right.expect("details open");
        assert!(approx_eq(right.width(), 560.0));
        assert!(panes.splitter.is_some());
    }

    #[test]
    fn details_width_follows_preference() {
        let panes = Step2PaneRects::from_content(content_rect(1400.0), true, Some(700.0));
        let right = panes.right.expect("details open");
        assert!(approx_eq(right.width(), 700.0));
    }

    #[test]
    fn details_width_clamps_to_minimum() {
        let panes = Step2PaneRects::from_content(content_rect(1400.0), true, Some(100.0));
        let right = panes.right.expect("details open");
        assert!(approx_eq(right.width(), DETAILS_MIN_W));
    }

    #[test]
    fn details_width_keeps_tree_minimum() {
        let panes = Step2PaneRects::from_content(content_rect(1400.0), true, Some(5000.0));
        let right = panes.right.expect("details open");
        assert!(approx_eq(right.width(), 1400.0 - GRID_GAP - LEFT_MIN_W));
    }

    #[test]
    fn narrow_content_ignores_preference_and_has_no_splitter() {
        let with_preference = Step2PaneRects::from_content(content_rect(800.0), true, Some(700.0));
        let without_preference = Step2PaneRects::from_content(content_rect(800.0), true, None);
        assert!(approx_eq(
            with_preference.right.expect("details open").width(),
            without_preference.right.expect("details open").width(),
        ));
        assert!(with_preference.splitter.is_none());
    }

    #[test]
    fn closed_details_has_no_splitter() {
        let panes = Step2PaneRects::from_content(content_rect(1400.0), false, Some(700.0));
        assert!(panes.splitter.is_none());
        assert!(panes.right.is_none());
    }
}
