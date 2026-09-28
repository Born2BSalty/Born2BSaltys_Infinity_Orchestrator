// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{Duration, Instant};

use eframe::egui;

use crate::registry::operations_rename;
use crate::registry::share_export::{self, ShareMeta};
use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::fork_info_popup::{self, SelfNode};
use crate::ui::orchestrator::widgets::help_button::{self, HelpPage};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_accent_deep, redesign_border_strong, redesign_input_bg, redesign_shell_bg,
    redesign_text_muted, redesign_text_primary,
};
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::workspace_state_loader;
use tracing::warn;

const SAVE_FLASH_MS: u64 = 1600;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    ui.horizontal_top(|ui| {
        render_title_row(ui, orchestrator, palette);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            render_save_or_share_button(ui, orchestrator, palette);
            if orchestrator.workspace_view.fork_meta.is_some()
                && fork_details_button(ui, palette).clicked()
            {
                orchestrator.workspace_view.fork_info_open = true;
            }
            ui.add_space(8.0);
            let created_from_mods = orchestrator.workspace_view.fork_meta.is_none();
            let page =
                help_page_for_step(orchestrator.workspace_view.current_step, created_from_mods);
            help_button::render(ui, palette, page);
        });
    });
    render_fork_subline(ui, orchestrator, palette);

    if orchestrator.workspace_view.fork_info_open {
        render_fork_info_popup(orchestrator, palette, ctx);
    }
}

const fn help_page_for_step(step: WorkspaceStep, created_from_mods: bool) -> HelpPage {
    match step {
        WorkspaceStep::Step2 => HelpPage::Step2 { created_from_mods },
        WorkspaceStep::Step3 => HelpPage::Step3,
        WorkspaceStep::Step4 => HelpPage::Step4,
        WorkspaceStep::Step5 => HelpPage::Step5,
    }
}

fn render_title_row(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, palette: ThemePalette) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if orchestrator.workspace_view.renaming {
            render_rename_editor(ui, orchestrator, palette);
        } else {
            let name = orchestrator.workspace_view.modlist_name.clone();
            let title = if name.trim().is_empty() {
                "Editing modlist".to_string()
            } else {
                format!("Editing {name}")
            };
            ui.label(
                egui::RichText::new(title)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            if pencil_button(ui, palette).clicked() {
                orchestrator
                    .workspace_view
                    .rename_temp
                    .clone_from(&orchestrator.workspace_view.modlist_name);
                orchestrator.workspace_view.renaming = true;
                let m = egui::Id::new(("workspace_header_rename_edit",)).with("focused_once");
                ui.memory_mut(|mem| mem.data.remove::<bool>(m));
            }
        }

        if orchestrator.workspace_view.fork_meta.is_some() {
            fork_badge(ui, palette);
        }
    });
}

fn render_rename_editor(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) {
    ui.label(
        egui::RichText::new("Editing")
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );

    let edit_id = egui::Id::new(("workspace_header_rename_edit",));
    let response = ui.add_sized(
        egui::vec2(240.0, 28.0),
        egui::TextEdit::singleline(&mut orchestrator.workspace_view.rename_temp)
            .id(edit_id)
            .font(egui::FontId::new(
                13.0,
                egui::FontFamily::Name("poppins_medium".into()),
            ))
            .text_color(redesign_text_primary(palette))
            .background_color(redesign_input_bg(palette))
            .margin(egui::Margin::symmetric(8, 4)),
    );

    let focus_marker = edit_id.with("focused_once");
    let already_focused = ui
        .memory(|m| m.data.get_temp::<bool>(focus_marker))
        .unwrap_or(false);
    if !already_focused {
        response.request_focus();
        ui.memory_mut(|m| m.data.insert_temp(focus_marker, true));
    }

    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));

    let mut do_save = enter;
    let mut do_cancel = escape;

    if redesign_btn(
        ui,
        palette,
        "save",
        BtnOpts {
            primary: true,
            small: true,
            ..Default::default()
        },
    )
    .clicked()
    {
        do_save = true;
    }
    if redesign_btn(
        ui,
        palette,
        "cancel",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .clicked()
    {
        do_cancel = true;
    }

    if do_save {
        commit_rename(orchestrator);
    } else if do_cancel {
        orchestrator.workspace_view.renaming = false;
        orchestrator.workspace_view.rename_temp.clear();
    }
}

fn commit_rename(orchestrator: &mut OrchestratorApp) {
    let new_name = orchestrator.workspace_view.rename_temp.trim().to_string();
    orchestrator.workspace_view.renaming = false;

    if new_name.is_empty() {
        orchestrator.workspace_view.rename_temp.clear();
        return;
    }

    let id = orchestrator.workspace_view.modlist_id.clone();
    match operations_rename::rename_modlist(&id, &new_name, &mut orchestrator.registry) {
        Ok(()) => {
            orchestrator.workspace_view.modlist_name = new_name;
            orchestrator
                .persistence_cycle
                .mark_registry_dirty(Instant::now());
        }
        Err(err) => {
            warn!(target = "orchestrator", "rename_modlist failed: {err}");
            orchestrator
                .notification_manager
                .error(format!("Couldn't rename to \"{new_name}\": {err}"));
        }
    }
    orchestrator.workspace_view.rename_temp.clear();
}

fn render_fork_subline(ui: &mut egui::Ui, orchestrator: &OrchestratorApp, palette: ThemePalette) {
    let Some(meta) = orchestrator.workspace_view.fork_meta.as_ref() else {
        return;
    };
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        paint_inline_fork(ui, redesign_accent_deep(palette));
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Forked from ")
                .size(16.0)
                .family(egui::FontFamily::Name("poppins_bold".into()))
                .color(redesign_accent_deep(palette)),
        );
        ui.label(
            egui::RichText::new(format!("\"{}\"", meta.parent_name))
                .size(16.0)
                .family(egui::FontFamily::Name("poppins_bold".into()))
                .color(redesign_text_primary(palette)),
        );
        if !meta.parent_author.trim().is_empty() {
            ui.label(
                egui::RichText::new(format!(" by {}", meta.parent_author.trim()))
                    .size(16.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_muted(palette)),
            );
        }
        ui.label(
            egui::RichText::new(format!(
                " \u{00B7} {} mods \u{00B7} {} components preselected",
                meta.mods, meta.components
            ))
            .size(16.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
        );
    });
}

fn render_save_or_share_button(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) {
    if orchestrator.workspace_view.current_step == WorkspaceStep::Step5 {
        let installed =
            crate::ui::workspace::step5::success_banner::clean_exit(&orchestrator.wizard_state);
        let resp = redesign_btn(
            ui,
            palette,
            "Share this modlist",
            BtnOpts {
                small: true,
                primary: installed,
                disabled: !installed,
                ..Default::default()
            },
        )
        .on_hover_text(if installed {
            "Export a modlist file or copy the share code"
        } else {
            "Available after a successful install"
        });
        if installed && resp.clicked() {
            orchestrator.workspace_step5.share_dialog_open = true;
        }
        return;
    }

    let now = Instant::now();
    let flashing = match orchestrator.workspace_view.save_draft_flash_until {
        Some(until) if now < until => true,
        Some(_) => {
            orchestrator.workspace_view.save_draft_flash_until = None;
            false
        }
        None => false,
    };

    if flashing {
        let _ = saved_flash_button(ui, palette);
        ui.ctx().request_repaint_after(Duration::from_millis(120));
    } else if redesign_btn(
        ui,
        palette,
        "save draft",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .on_hover_text("Save this in-progress build so you can resume it from Home")
    .clicked()
    {
        save_draft(orchestrator);
    }
}

fn save_draft(orchestrator: &mut OrchestratorApp) {
    let id = orchestrator.workspace_view.modlist_id.clone();
    if id.is_empty() {
        return;
    }

    workspace_state_loader::sync_step3_from_step2_if_changed(&mut orchestrator.wizard_state);

    let prior = orchestrator
        .workspace_state
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let extracted = workspace_state_loader::extract_workspace_state_from_wizard(
        &orchestrator.wizard_state,
        &prior,
    );

    let store = orchestrator
        .workspace_stores
        .entry(id.clone())
        .or_insert_with(|| WorkspaceStore::new_for_id(&id));

    match store.save(&extracted) {
        Ok(()) => {
            orchestrator
                .workspace_state
                .insert(id.clone(), extracted.clone());
            orchestrator
                .persistence_cycle
                .last_saved_workspaces
                .insert(id.clone(), extracted);
            orchestrator.workspace_view.save_draft_flash_until =
                Some(Instant::now() + Duration::from_millis(SAVE_FLASH_MS));
            write_selection_counts(orchestrator, &id);
            rebake_share_code_after_save_draft(orchestrator, &id);
        }
        Err(err) => {
            warn!(target = "orchestrator", "save draft for {id} failed: {err}");
        }
    }
}

fn write_selection_counts(orchestrator: &mut OrchestratorApp, id: &str) {
    let step2 = &orchestrator.wizard_state.step2;
    if step2.is_scanning || (step2.bgee_mods.is_empty() && step2.bg2ee_mods.is_empty()) {
        return;
    }
    let (mods, components) = crate::install_runtime::registry_transition::count_mods_and_components(
        &orchestrator.wizard_state,
    );
    let Some(entry) = orchestrator.registry.find_mut(id) else {
        return;
    };
    entry.mod_count = mods;
    entry.component_count = components;
    orchestrator
        .persistence_cycle
        .mark_registry_dirty(Instant::now());
}

fn rebake_share_code_after_save_draft(orchestrator: &mut OrchestratorApp, id: &str) {
    let Some(entry) = orchestrator.registry.find(id) else {
        return;
    };
    let meta = ShareMeta::from_entry(entry, false).with_archive_meta(
        share_export::archive_meta_for_draft(&orchestrator.wizard_state),
    );
    match share_export::pack_meta(&orchestrator.wizard_state, &meta) {
        Ok(code) => {
            if let Some(entry_mut) = orchestrator.registry.find_mut(id) {
                entry_mut.latest_share_code = Some(code);
            }
            orchestrator
                .persistence_cycle
                .mark_registry_dirty(Instant::now());
        }
        Err(err) => {
            tracing::debug!(
                target = "orchestrator",
                "save draft: share code re-bake for {id} skipped ({err}); \
                 existing code retained"
            );
        }
    }
}

fn render_fork_info_popup(
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    ctx: &egui::Context,
) {
    let id = orchestrator.workspace_view.modlist_id.clone();
    let (self_name, self_author, lineage) = match orchestrator.registry.find(&id) {
        Some(e) => (
            if e.name.trim().is_empty() {
                orchestrator.workspace_view.modlist_name.clone()
            } else {
                e.name.clone()
            },
            e.author.clone().unwrap_or_default(),
            e.forked_from.clone(),
        ),
        None => (
            orchestrator.workspace_view.modlist_name.clone(),
            String::new(),
            orchestrator
                .workspace_view
                .fork_meta
                .as_ref()
                .map(|m| m.forked_from.clone())
                .unwrap_or_default(),
        ),
    };

    let outcome = fork_info_popup::render(
        ctx,
        palette,
        "workspace_header",
        &lineage,
        &SelfNode {
            name: &self_name,
            author: self_author.trim(),
        },
    );
    if outcome == fork_info_popup::ForkInfoOutcome::Closed {
        orchestrator.workspace_view.fork_info_open = false;
    }
}

fn pencil_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad = 4.0;
    let ink = 13.0;
    let desired = egui::vec2(ink + pad * 2.0, ink + pad * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let color = if response.hovered() {
        redesign_text_primary(palette)
    } else {
        redesign_text_muted(palette)
    };
    if ui.is_rect_visible(rect) {
        let optical_rise = egui::vec2(0.0, 2.0);
        paint_pencil_glyph(ui.painter(), rect.center() - optical_rise, ink, color);
    }
    response.on_hover_text("Rename modlist")
}

fn paint_pencil_glyph(painter: &egui::Painter, center: egui::Pos2, ink: f32, color: egui::Color32) {
    let h = ink / 2.0;
    let tip = egui::pos2(center.x - h, center.y + h);
    let cap = egui::pos2(center.x + h, center.y - h);
    let axis = normalize(cap - tip);
    let nrm = egui::vec2(-axis.y, axis.x);
    let w = ink * 0.16;

    let nib_len = ink * 0.30;
    let body_start = tip + axis * nib_len;
    let cap_end = cap - axis * (ink * 0.06);

    painter.add(egui::Shape::convex_polygon(
        vec![tip, body_start + nrm * w, body_start - nrm * w],
        color,
        egui::Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![
            body_start + nrm * w,
            cap_end + nrm * w,
            cap_end - nrm * w,
            body_start - nrm * w,
        ],
        color,
        egui::Stroke::NONE,
    ));
    let ferrule = cap_end - axis * (ink * 0.22);
    painter.line_segment(
        [ferrule + nrm * w, ferrule - nrm * w],
        egui::Stroke::new(1.0_f32, color),
    );
}

fn normalize(v: egui::Vec2) -> egui::Vec2 {
    let len = v.length();
    if len <= f32::EPSILON {
        egui::Vec2::ZERO
    } else {
        v / len
    }
}

fn fork_badge(ui: &mut egui::Ui, palette: ThemePalette) {
    let pad_x = 12.0;
    let pad_y = 4.0;
    let font = egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into()));
    let ink = egui::Color32::from_rgb(0x1a, 0x26, 0x38);
    let label = "FORK";
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), ink);
    let fork_w = 9.0;
    let gap = 5.0;
    let content_w = fork_w + gap + galley.size().x;
    let desired = egui::vec2(
        content_w + pad_x * 2.0,
        galley.size().y.max(fork_w) + pad_y * 2.0,
    );
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(rect, radius, redesign_accent(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        paint_fork_at(painter, egui::pos2(start_x + fork_w / 2.0, cy), ink);
        painter.text(
            egui::pos2(start_x + fork_w + gap, cy),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            ink,
        );
    }
}

fn fork_details_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let color = redesign_text_primary(palette);
    let label = "view fork details";
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), color);
    let fork_w = 9.0;
    let gap = 5.0;
    let content_w = fork_w + gap + galley.size().x;
    let content_h = galley.size().y.max(fork_w);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let pressed = response.is_pointer_button_down_on();
    let rect = if pressed {
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
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        paint_fork_at(painter, egui::pos2(start_x + fork_w / 2.0, cy), color);
        painter.text(
            egui::pos2(start_x + fork_w + gap, cy),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            color,
        );
    }
    response
}

fn saved_flash_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let glyph_font = egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let color = redesign_text_primary(palette);
    let glyph = "\u{2713}";
    let prose = " saved!";
    let g = ui
        .painter()
        .layout_no_wrap(glyph.to_string(), glyph_font.clone(), color);
    let p = ui
        .painter()
        .layout_no_wrap(prose.to_string(), prose_font.clone(), color);
    let content_w = g.size().x + p.size().x;
    let content_h = g.size().y.max(p.size().y);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());
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
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            glyph,
            glyph_font,
            color,
        );
        painter.text(
            egui::pos2(start_x + g.size().x, cy),
            egui::Align2::LEFT_CENTER,
            prose,
            prose_font,
            color,
        );
    }
    response
}

fn paint_fork_at(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4_f32, color);
    let half_h = 4.5;
    let split_y = center.y - 0.5;
    let tine_dx = 3.0;
    painter.line_segment(
        [
            egui::pos2(center.x, center.y + half_h),
            egui::pos2(center.x, split_y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x - tine_dx, center.y - half_h),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x + tine_dx, center.y - half_h),
        ],
        stroke,
    );
}

fn paint_inline_fork(ui: &mut egui::Ui, color: egui::Color32) {
    let w = 9.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 16.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_fork_at(ui.painter(), rect.center(), color);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistState};
    use egui_toast::ToastKind;

    fn orch_with_entry(name: &str) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test("hdrtest");
        app.registry.entries.push(ModlistEntry {
            id: "HDRTEST00000".to_string(),
            name: name.to_string(),
            game: Game::EET,
            state: ModlistState::InProgress,
            ..Default::default()
        });
        app.workspace_view.modlist_id = "HDRTEST00000".to_string();
        app.workspace_view.modlist_name = name.to_string();
        app
    }

    fn orch_with_entry_and_code(name: &str, code: &str) -> OrchestratorApp {
        let mut app = orch_with_entry(name);
        app.registry
            .find_mut("HDRTEST00000")
            .unwrap()
            .latest_share_code = Some(code.to_string());
        app
    }

    #[test]
    fn commit_rename_updates_registry_and_header_only() {
        let mut app = orch_with_entry("Old Name");
        app.workspace_view.renaming = true;
        app.workspace_view.rename_temp = "Brand New Name".to_string();

        commit_rename(&mut app);

        assert!(!app.workspace_view.renaming);
        assert_eq!(app.workspace_view.modlist_name, "Brand New Name");
        assert_eq!(
            app.registry.find("HDRTEST00000").unwrap().name,
            "Brand New Name"
        );
        assert!(
            !app.workspace_state_dirty,
            "rename must not mark workspace_state_dirty"
        );
    }

    #[test]
    fn empty_rename_is_noop_cancel() {
        let mut app = orch_with_entry("Keep Me");
        app.workspace_view.renaming = true;
        app.workspace_view.rename_temp = "   ".to_string();

        commit_rename(&mut app);

        assert!(!app.workspace_view.renaming);
        assert_eq!(app.workspace_view.modlist_name, "Keep Me");
        assert_eq!(app.registry.find("HDRTEST00000").unwrap().name, "Keep Me");
    }

    #[test]
    fn rename_failure_pushes_error_toast() {
        let mut app = orch_with_entry("Original");
        app.workspace_view.modlist_id = "DOES_NOT_EXIST".to_string();
        app.workspace_view.renaming = true;
        app.workspace_view.rename_temp = "New Name".to_string();

        commit_rename(&mut app);

        let history = app.notification_manager.history();
        assert_eq!(
            history.len(),
            1,
            "exactly one notification must be enqueued"
        );
        let record = history.back().unwrap();
        assert_eq!(
            record.kind,
            ToastKind::Error,
            "rename failure must be an error toast"
        );
        assert!(
            record.text.contains("Couldn't rename to"),
            "toast must mention the failure: {}",
            record.text
        );
        assert!(
            record.text.contains("\"New Name\""),
            "toast must include the attempted name: {}",
            record.text
        );
    }

    #[test]
    fn rebake_leaves_code_unchanged_when_no_weidu_entries() {
        let sentinel = "BIO-MODLIST-V1:SENTINEL-UNCHANGED";
        let mut app = orch_with_entry_and_code("My Modlist", sentinel);

        rebake_share_code_after_save_draft(&mut app, "HDRTEST00000");

        assert_eq!(
            app.registry
                .find("HDRTEST00000")
                .unwrap()
                .latest_share_code
                .as_deref(),
            Some(sentinel),
            "a not-yet-scanned modlist (no WeiDU entries) must not have its \
             share code overwritten by save draft"
        );
    }

    #[test]
    fn rebake_is_noop_for_missing_entry() {
        let mut app = orch_with_entry_and_code("X", "BIO-MODLIST-V1:NOTOUCH");

        rebake_share_code_after_save_draft(&mut app, "DOES-NOT-EXIST");

        assert_eq!(
            app.registry
                .find("HDRTEST00000")
                .unwrap()
                .latest_share_code
                .as_deref(),
            Some("BIO-MODLIST-V1:NOTOUCH"),
            "a rebake for a missing id must not affect other entries"
        );
    }

    fn counted_mod(tp_file: &str) -> crate::app::state::Step2ModState {
        crate::app::state::Step2ModState {
            name: tp_file.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: format!("{tp_file}/{tp_file}.tp2"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: true,
            hidden_components: Vec::new(),
            components: vec![crate::app::state::Step2ComponentState {
                component_id: "0".to_string(),
                label: "0".to_string(),
                weidu_group: None,
                collapsible_group: None,
                collapsible_group_is_umbrella: false,
                collapsible_group_combinable: false,
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
                checked: true,
                selected_order: Some(1),
            }],
        }
    }

    struct AmbientRestore(Option<std::path::PathBuf>);

    impl Drop for AmbientRestore {
        fn drop(&mut self) {
            crate::app::mod_downloads::set_active_modlist_dir(self.0.take());
        }
    }

    #[test]
    fn save_draft_rebake_carries_the_recorded_archive_hashes() {
        use crate::app::app_step2_update_source_refs::{
            InstalledArchiveRecord, ModSourceRefsFile, installed_source_refs_path,
        };
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient = AmbientRestore(crate::app::mod_downloads::active_modlist_dir());
        let mut app = orch_with_entry("hashes");
        crate::install_runtime::active_modlist_source_path::set_ambient_for_modlist("HDRTEST00000");
        app.wizard_state.step1.game_install = "BGEE".to_string();
        app.wizard_state.step2.bgee_mods = vec![counted_mod("alpha")];
        app.wizard_state.step3.bgee_items = vec![crate::app::state::Step3ItemState {
            tp_file: "ALPHA/ALPHA.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "alpha".to_string(),
            component_label: "0".to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }];
        let record = InstalledArchiveRecord {
            name: "alpha__primary__v19.zip".to_string(),
            size: 1234,
            hash: "0123456789abcdef0123456789abcdef".to_string(),
        };
        let mut refs_file = ModSourceRefsFile::default();
        refs_file
            .archives
            .insert("alpha".to_string(), record.clone());
        let refs_path = installed_source_refs_path();
        assert!(
            refs_path.starts_with(app.isolated_test_config_root.as_ref().unwrap()),
            "the refs file must sit under the isolated config root: {}",
            refs_path.display()
        );
        std::fs::create_dir_all(refs_path.parent().unwrap()).unwrap();
        std::fs::write(&refs_path, toml::to_string_pretty(&refs_file).unwrap()).unwrap();

        rebake_share_code_after_save_draft(&mut app, "HDRTEST00000");

        let code = app
            .registry
            .find("HDRTEST00000")
            .unwrap()
            .latest_share_code
            .clone()
            .expect("the rebake must mint a share code");
        let metas = share_export::decode_archive_meta(&code).unwrap();
        assert_eq!(
            metas,
            vec![share_export::ArchiveMeta {
                name: record.name,
                size: record.size,
                hash: record.hash,
            }]
        );
    }

    fn counts_of(app: &OrchestratorApp) -> (u32, u32) {
        let entry = app.registry.find("HDRTEST00000").unwrap();
        (entry.mod_count, entry.component_count)
    }

    #[test]
    fn save_draft_writes_the_selection_counts() {
        let mut app = orch_with_entry("counts");
        app.wizard_state.step1.game_install = "BGEE".to_string();
        app.wizard_state.step2.bgee_mods = vec![counted_mod("alpha"), counted_mod("beta")];

        save_draft(&mut app);

        let store_path = app.workspace_stores["HDRTEST00000"].path().to_path_buf();
        assert!(
            store_path.starts_with(std::env::temp_dir()),
            "the workspace store must live under the isolated temp root: {}",
            store_path.display()
        );
        assert!(
            store_path.is_file(),
            "save draft must write the workspace file"
        );
        assert_eq!(counts_of(&app), (2, 2));

        app.wizard_state.step2.bgee_mods[1].components[0].checked = false;
        save_draft(&mut app);

        assert_eq!(counts_of(&app), (1, 1));
    }

    #[test]
    fn save_draft_keeps_the_counts_while_the_tree_is_not_loaded() {
        let mut app = orch_with_entry("counts_hold");
        app.wizard_state.step1.game_install = "BGEE".to_string();
        app.wizard_state.step2.bgee_mods = vec![counted_mod("alpha"), counted_mod("beta")];
        save_draft(&mut app);
        assert_eq!(counts_of(&app), (2, 2));

        app.wizard_state.step2.is_scanning = true;
        app.wizard_state.step3.bgee_items.clear();
        save_draft(&mut app);
        assert_eq!(counts_of(&app), (2, 2));

        app.wizard_state.step2.is_scanning = false;
        app.wizard_state.step2.bgee_mods.clear();
        save_draft(&mut app);
        assert_eq!(counts_of(&app), (2, 2));
    }
}
