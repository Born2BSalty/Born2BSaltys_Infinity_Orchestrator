// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::workspace_model::ModsSource;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, InputOpts, redesign_btn, redesign_text_input};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step2::step2_rescan_reconcile;

const SEARCH_INPUT_H: f32 = 30.0;
const ROW_GAP: f32 = 10.0;
const SEARCH_INPUT_TEXT_PAD: i8 = 8;
const DROPDOWN_MIN_W: f32 = 160.0;
const SOURCE_SELECTOR_PAD_X: f32 = 10.0;
const SOURCE_SELECTOR_PAD_Y: f32 = 4.0;
const SOURCE_SELECTOR_FONT_SIZE: f32 = 12.0;
const CARET_GAP: f32 = 7.0;
const CARET_W: f32 = 9.0;
const CARET_H: f32 = 5.0;
const MODS_SOURCE_LABEL: &str = "Mods Source:";

const RESCAN_DISABLED_TIP: &str = "Available after install prep (Phase 7) \u{2014} \
     the mods folder is extracted per-install at prep time (SPEC \u{00A7}13.12a). \
     Use \u{201C}Global mods folder\u{201D} to scan your configured mods folder in the meantime.";

const GLOBAL_MODS_DISABLED_TIP: &str =
    "No mods folder configured. Set it in Settings > Paths > Mods folder.";

const FORK_GLOBAL_DISABLED_TIP: &str = "Only available when creating from extracted mods.";

struct RowParams {
    is_fork: bool,
    current_source: ModsSource,
    global_mods_folder: String,
    global_non_empty: bool,
    scratch_enabled: bool,
    modlist_id: String,
}

impl RowParams {
    fn from_orchestrator(orchestrator: &OrchestratorApp) -> Self {
        let modlist_id = orchestrator.workspace_view.modlist_id.trim().to_string();
        let current_source = orchestrator
            .workspace_state
            .get(modlist_id.as_str())
            .map_or_else(ModsSource::default, |w| w.mods_source);
        let global_mods_folder = orchestrator
            .settings_store
            .load()
            .ok()
            .map(|s| s.step1.mods_folder)
            .unwrap_or_default();
        let global_non_empty = !global_mods_folder.trim().is_empty();
        Self {
            is_fork: orchestrator.workspace_view.fork_meta.is_some(),
            current_source,
            global_mods_folder,
            global_non_empty,
            scratch_enabled: scratch_scan_enabled(orchestrator),
            modlist_id,
        }
    }
}

pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    rect: egui::Rect,
) -> Option<Step2Action> {
    let is_scanning = orchestrator.wizard_state.step2.is_scanning;
    let params = RowParams::from_orchestrator(orchestrator);
    render_row(ui, orchestrator, palette, rect, is_scanning, &params)
}

fn render_row(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    rect: egui::Rect,
    is_scanning: bool,
    params: &RowParams,
) -> Option<Step2Action> {
    let mut action: Option<Step2Action> = None;

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;

            let btn_label = if is_scanning {
                "Cancel Scan"
            } else {
                "Rescan Mods"
            };
            let btn_w = small_btn_width(ui, btn_label);
            let sel_w = source_selector_width(ui, params.current_source);
            let label_w = mods_source_label_width(ui);
            let search_w =
                (rect.width() - label_w - ROW_GAP - sel_w - ROW_GAP - btn_w - ROW_GAP).max(80.0);

            let search_margin = egui::Margin::symmetric(SEARCH_INPUT_TEXT_PAD, 4);
            let _resp = redesign_text_input(
                ui,
                palette,
                InputOpts {
                    edit: egui::TextEdit::singleline(
                        &mut orchestrator.wizard_state.step2.search_query,
                    )
                    .hint_text("Search mods or components...")
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .margin(search_margin)
                    .font(egui::FontId::new(
                        14.0,
                        egui::FontFamily::Name("poppins_medium".into()),
                    )),
                    margin: search_margin,
                    size: egui::vec2(search_w, SEARCH_INPUT_H),
                    border: None,
                },
            );

            render_mods_source_label(ui, palette);
            let trigger = source_selector_trigger(ui, palette, params.current_source);
            let popup_id = ui.make_persistent_id("step2_mods_source_selector");
            if trigger.clicked() {
                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
            }
            if let Some(new_source) = source_selector_dropdown(
                ui,
                palette,
                popup_id,
                &trigger,
                params.is_fork,
                params.global_non_empty,
            ) {
                if let Some(ws) = orchestrator
                    .workspace_state
                    .get_mut(params.modlist_id.as_str())
                {
                    ws.mods_source = new_source;
                }
                orchestrator.wizard_state.step1.mods_folder =
                    source_folder(new_source, &params.global_mods_folder, orchestrator);
                orchestrator.mark_workspace_dirty();
            }

            if let Some(a) = render_scan_btn(ui, orchestrator, palette, is_scanning, params) {
                action = Some(a);
            }
        });
    });

    action
}

fn render_scan_btn(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    is_scanning: bool,
    params: &RowParams,
) -> Option<Step2Action> {
    if is_scanning {
        return redesign_btn(
            ui,
            palette,
            "Cancel Scan",
            BtnOpts {
                small: true,
                ..Default::default()
            },
        )
        .on_hover_text("Stop the running scan and return to idle.")
        .clicked()
        .then_some(Step2Action::CancelScan);
    }

    let enabled = rescan_btn_enabled(
        params.current_source,
        params.scratch_enabled,
        params.is_fork,
        params.global_non_empty,
    );
    let resp = redesign_btn(
        ui,
        palette,
        "Rescan Mods",
        BtnOpts {
            small: true,
            disabled: !enabled,
            ..Default::default()
        },
    );
    let resp = if enabled {
        resp
    } else {
        resp.on_hover_text(rescan_disabled_tip(params.current_source, params.is_fork))
    };

    if !resp.clicked() {
        return None;
    }

    let last_rescanned = orchestrator
        .workspace_state
        .get(params.modlist_id.as_str())
        .map_or_else(ModsSource::default, |w| w.last_rescanned_mods_source);
    if needs_source_change_warning(params.current_source, last_rescanned) {
        orchestrator.workspace_view.step2.pending_global_mods_scan = Some(());
        return None;
    }

    orchestrator.wizard_state.step1.mods_folder = source_folder(
        params.current_source,
        &params.global_mods_folder,
        orchestrator,
    );
    step2_rescan_reconcile::snapshot_current_selection(orchestrator);
    if let Some(ws) = orchestrator
        .workspace_state
        .get_mut(params.modlist_id.as_str())
    {
        ws.last_rescanned_mods_source = params.current_source;
    }
    orchestrator.mark_workspace_dirty();
    Some(Step2Action::StartScan)
}

const fn rescan_btn_enabled(
    source: ModsSource,
    scratch_enabled: bool,
    is_fork: bool,
    global_non_empty: bool,
) -> bool {
    match source {
        ModsSource::InstallationFolder => scratch_enabled,
        ModsSource::GlobalModsFolder => !global_item_disabled(is_fork, global_non_empty),
    }
}

const fn rescan_disabled_tip(source: ModsSource, is_fork: bool) -> &'static str {
    match source {
        ModsSource::InstallationFolder => RESCAN_DISABLED_TIP,
        ModsSource::GlobalModsFolder if is_fork => FORK_GLOBAL_DISABLED_TIP,
        ModsSource::GlobalModsFolder => GLOBAL_MODS_DISABLED_TIP,
    }
}

fn source_folder(
    source: ModsSource,
    global_mods_folder: &str,
    orchestrator: &OrchestratorApp,
) -> String {
    match source {
        ModsSource::GlobalModsFolder => global_mods_folder.to_string(),
        ModsSource::InstallationFolder => orchestrator
            .workspace_state
            .get(orchestrator.workspace_view.modlist_id.trim())
            .and_then(|w| w.scratch_mods_folder.clone())
            .unwrap_or_default(),
    }
}

const fn source_label(source: ModsSource) -> &'static str {
    match source {
        ModsSource::InstallationFolder => "Installation Folder",
        ModsSource::GlobalModsFolder => "Global Mods Folder",
    }
}

fn source_selector_dropdown(
    ui: &egui::Ui,
    palette: ThemePalette,
    popup_id: egui::Id,
    trigger: &egui::Response,
    is_fork: bool,
    global_settings_non_empty: bool,
) -> Option<ModsSource> {
    if !ui.memory(|mem| mem.is_popup_open(popup_id)) {
        return None;
    }

    let mut pos = trigger.rect.right_bottom();
    if let Some(to_global) = ui.ctx().layer_transform_to_global(ui.layer_id()) {
        pos = to_global * pos;
    }

    let frame = egui::Frame::popup(ui.style());
    let area_resp = egui::Area::new(popup_id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .pivot(egui::Align2::RIGHT_TOP)
        .show(ui.ctx(), |inner_ui| {
            frame
                .show(inner_ui, |inner_ui| {
                    inner_ui.set_max_width(DROPDOWN_MIN_W);
                    let chassis = egui::Frame::default()
                        .fill(redesign_shell_bg(palette))
                        .stroke(egui::Stroke::new(
                            REDESIGN_BORDER_WIDTH_PX,
                            redesign_border_strong(palette),
                        ))
                        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
                        .inner_margin(egui::Margin::same(4));
                    chassis
                        .show(inner_ui, |inner_ui| {
                            inner_ui.spacing_mut().item_spacing.y = 0.0;
                            build_source_menu_items(
                                inner_ui,
                                palette,
                                is_fork,
                                global_settings_non_empty,
                            )
                        })
                        .inner
                })
                .inner
        });

    let should_close = trigger.clicked_elsewhere() && area_resp.response.clicked_elsewhere();
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
        ui.memory_mut(egui::Memory::close_popup);
    }

    let chosen = area_resp.inner;
    if chosen.is_some() {
        ui.memory_mut(egui::Memory::close_popup);
    }
    chosen
}

fn build_source_menu_items(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    is_fork: bool,
    global_settings_non_empty: bool,
) -> Option<ModsSource> {
    let global_enabled = !global_item_disabled(is_fork, global_settings_non_empty);
    let global_tip = if is_fork {
        Some(FORK_GLOBAL_DISABLED_TIP)
    } else if !global_settings_non_empty {
        Some(GLOBAL_MODS_DISABLED_TIP)
    } else {
        None
    };

    let inst_clicked = dropdown_item(
        ui,
        palette,
        source_label(ModsSource::InstallationFolder),
        true,
        None,
    );
    let global_clicked = dropdown_item(
        ui,
        palette,
        source_label(ModsSource::GlobalModsFolder),
        global_enabled,
        global_tip,
    );

    if inst_clicked {
        Some(ModsSource::InstallationFolder)
    } else if global_clicked {
        Some(ModsSource::GlobalModsFolder)
    } else {
        None
    }
}

fn dropdown_item(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    enabled: bool,
    tooltip: Option<&str>,
) -> bool {
    use crate::ui::shared::redesign_tokens::{redesign_hover_overlay, redesign_text_faint};

    let text_color = if enabled {
        redesign_text_primary(palette)
    } else {
        redesign_text_faint(palette)
    };
    let font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into()));

    let pad_x = 10.0;
    let pad_y = 6.0;
    let row_width = ui.available_width().max(DROPDOWN_MIN_W - 8.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let row_height = galley.size().y + pad_y * 2.0;
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(row_width, row_height), sense);

    if ui.is_rect_visible(rect) {
        if response.hovered() && enabled {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + pad_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            text_color,
        );
    }

    if let Some(tip) = tooltip {
        response.on_hover_text(tip).clicked()
    } else {
        response.clicked()
    }
}

fn scratch_scan_enabled(orchestrator: &OrchestratorApp) -> bool {
    let id = orchestrator.workspace_view.modlist_id.trim();
    if id.is_empty()
        || orchestrator
            .wizard_state
            .step1
            .mods_folder
            .trim()
            .is_empty()
    {
        return false;
    }
    orchestrator
        .workspace_state
        .get(id)
        .and_then(|workspace| workspace.scratch_mods_folder.as_deref())
        .is_some_and(|folder| !folder.trim().is_empty())
}

fn small_btn_width(ui: &egui::Ui, label: &str) -> f32 {
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, egui::Color32::WHITE);
    10.0_f32.mul_add(2.0, galley.size().x)
}

fn selector_font() -> egui::FontId {
    egui::FontId::new(
        SOURCE_SELECTOR_FONT_SIZE,
        egui::FontFamily::Name("poppins_medium".into()),
    )
}

fn source_selector_width(ui: &egui::Ui, source: ModsSource) -> f32 {
    let label = source_label(source);
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), selector_font(), egui::Color32::WHITE);
    SOURCE_SELECTOR_PAD_X.mul_add(2.0, galley.size().x) + CARET_GAP + CARET_W
}

fn mods_source_label_width(ui: &egui::Ui) -> f32 {
    ui.painter()
        .layout_no_wrap(
            MODS_SOURCE_LABEL.to_string(),
            selector_font(),
            egui::Color32::WHITE,
        )
        .size()
        .x
}

fn render_mods_source_label(ui: &mut egui::Ui, palette: ThemePalette) {
    let font = selector_font();
    let color = redesign_text_muted(palette);
    let galley = ui
        .painter()
        .layout_no_wrap(MODS_SOURCE_LABEL.to_string(), font.clone(), color);
    let size = egui::vec2(
        galley.size().x,
        SOURCE_SELECTOR_PAD_Y.mul_add(2.0, galley.size().y),
    );
    let (rect, _resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            egui::pos2(rect.left(), rect.center().y),
            egui::Align2::LEFT_CENTER,
            MODS_SOURCE_LABEL,
            font,
            color,
        );
    }
}

fn source_selector_trigger(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    source: ModsSource,
) -> egui::Response {
    let label = source_label(source);
    let text_color = redesign_text_primary(palette);
    let font = selector_font();
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let content_w = galley.size().x + CARET_GAP + CARET_W;
    let size = egui::vec2(
        SOURCE_SELECTOR_PAD_X.mul_add(2.0, content_w),
        SOURCE_SELECTOR_PAD_Y.mul_add(2.0, galley.size().y),
    );
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let rect = if response.is_pointer_button_down_on() {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            egui::pos2(rect.left() + SOURCE_SELECTOR_PAD_X, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            text_color,
        );
        let caret_cx =
            rect.left() + SOURCE_SELECTOR_PAD_X + galley.size().x + CARET_GAP + CARET_W / 2.0;
        paint_down_caret(painter, egui::pos2(caret_cx, rect.center().y), text_color);
    }

    response
}

fn paint_down_caret(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let hw = CARET_W / 2.0;
    let hh = CARET_H / 2.0;
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(center.x - hw, center.y - hh),
            egui::pos2(center.x + hw, center.y - hh),
            egui::pos2(center.x, center.y + hh),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

#[must_use]
pub(crate) const fn needs_source_change_warning(
    current: ModsSource,
    last_rescanned: ModsSource,
) -> bool {
    !matches!(
        (current, last_rescanned),
        (
            ModsSource::InstallationFolder,
            ModsSource::InstallationFolder
        ) | (ModsSource::GlobalModsFolder, ModsSource::GlobalModsFolder)
    )
}

#[must_use]
pub(crate) const fn global_item_disabled(is_fork: bool, global_settings_non_empty: bool) -> bool {
    is_fork || !global_settings_non_empty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_source_does_not_need_warning() {
        assert!(!needs_source_change_warning(
            ModsSource::InstallationFolder,
            ModsSource::InstallationFolder
        ));
        assert!(!needs_source_change_warning(
            ModsSource::GlobalModsFolder,
            ModsSource::GlobalModsFolder
        ));
    }

    #[test]
    fn different_source_needs_warning() {
        assert!(needs_source_change_warning(
            ModsSource::GlobalModsFolder,
            ModsSource::InstallationFolder
        ));
        assert!(needs_source_change_warning(
            ModsSource::InstallationFolder,
            ModsSource::GlobalModsFolder
        ));
    }

    #[test]
    fn fork_disables_global_item() {
        assert!(global_item_disabled(true, true));
        assert!(global_item_disabled(true, false));
    }

    #[test]
    fn non_fork_empty_settings_disables_global_item() {
        assert!(global_item_disabled(false, false));
    }

    #[test]
    fn non_fork_with_settings_enables_global_item() {
        assert!(!global_item_disabled(false, true));
    }
}
