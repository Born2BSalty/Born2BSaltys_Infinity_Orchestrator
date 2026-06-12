// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{Duration, Instant};

use eframe::egui;

use crate::registry::operations_rename;
use crate::registry::share_export::{self, ShareMeta};
use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::fork_info_popup::{self, SelfNode};
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
        ui.vertical(|ui| {
            render_title_row(ui, orchestrator, palette);
            render_fork_subline(ui, orchestrator, palette);
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            render_save_or_share_button(ui, orchestrator, palette);
            if orchestrator.workspace_view.fork_meta.is_some()
                && fork_details_button(ui, palette).clicked()
            {
                orchestrator.workspace_view.fork_info_open = true;
            }
        });
    });

    if orchestrator.workspace_view.fork_info_open {
        render_fork_info_popup(orchestrator, palette, ctx);
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
    ui.horizontal(|ui| {
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
            "Share import code",
            BtnOpts {
                small: true,
                primary: installed,
                disabled: !installed,
                ..Default::default()
            },
        )
        .on_hover_text(if installed {
            "View and copy the import code for this modlist"
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
            rebake_share_code_after_save_draft(orchestrator, &id);
        }
        Err(err) => {
            warn!(target = "orchestrator", "save draft for {id} failed: {err}");
        }
    }
}

/// Re-bakes the modlist's share code from the current wizard state and persists
/// the registry so a later "Copy import code" reflects per-modlist source changes.
///
/// Leaves the existing `latest_share_code` unchanged when the export returns an
/// error (e.g. no `WeiDU` entries available yet for a not-yet-scanned modlist).
fn rebake_share_code_after_save_draft(orchestrator: &mut OrchestratorApp, id: &str) {
    let Some(entry) = orchestrator.registry.find(id) else {
        return;
    };
    // A draft share code is review-first: the recipient imports-and-modifies it
    // (Create) rather than one-click auto-installing.
    let meta = ShareMeta::from_entry(entry, false);
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
        egui::Stroke::new(1.0, color),
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
    let stroke = egui::Stroke::new(1.4, color);
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
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
    use crate::registry::store::RegistryStore;
    use egui_toast::ToastKind;

    static HDRTEST_TMP: AtomicU64 = AtomicU64::new(0);

    fn orch_with_entry(name: &str) -> OrchestratorApp {
        let mut app = OrchestratorApp::new(false);
        let tmp = std::env::temp_dir().join(format!(
            "bio_hdrtest_{}_{}.json",
            std::process::id(),
            HDRTEST_TMP.fetch_add(1, Ordering::Relaxed)
        ));
        app.registry_store = RegistryStore::new_with_path(tmp);
        app.registry = ModlistRegistry::default();
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
        // Point workspace_view at a non-existent id so rename_modlist returns Err.
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
}
