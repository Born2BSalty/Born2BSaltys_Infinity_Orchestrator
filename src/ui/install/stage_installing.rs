// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::operations;
use crate::ui::install::state_install::InstallStage;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_primary,
};
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::workspace::step5::state_workspace_step5::PostInstallAction;
use crate::ui::workspace::step5::{post_install_actions, success_banner};

const FALLBACK_NAME: &str = "Shared modlist";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StageInstallingOutcome {
    #[default]
    Stay,
    Back(InstallStage),
    Nav(NavDestination),
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> StageInstallingOutcome {
    let palette = orchestrator.theme_palette;

    let name = orchestrator
        .install_screen_state
        .parsed_preview
        .as_ref()
        .and_then(|p| p.name.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_NAME)
        .to_string();

    let back_target = if orchestrator.install_screen_state.preview_cached {
        InstallStage::Preview
    } else {
        InstallStage::Paste
    };

    let mut outcome = StageInstallingOutcome::Stay;
    let sub = format!("{name} \u{00B7} live install console");
    ui.horizontal_top(|ui| {
        let back_btn_w = 130.0;
        let title_w = (ui.available_width() - back_btn_w).max(160.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                render_screen_title(ui, palette, "Installing modlist", Some(&sub));
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.add_space(0.0);
            if back_to_import_btn(ui, palette).clicked() {
                outcome = StageInstallingOutcome::Back(back_target);
            }
        });
    });
    ui.add_space(10.0);

    let dest = orchestrator
        .install_screen_state
        .destination
        .trim()
        .to_string();
    let entry = orchestrator
        .registry
        .entries
        .iter()
        .find(|e| e.destination_folder.trim() == dest && !dest.is_empty())
        .cloned();
    sync_share_provenance(orchestrator, entry.as_ref());

    let entry_for_row = entry.clone().unwrap_or_default();

    success_banner::render(ui, palette, &orchestrator.wizard_state, &entry_for_row);

    let post_install_action: Option<PostInstallAction> =
        post_install_actions::render(ui, palette, &orchestrator.wizard_state, &entry_for_row);

    let exe_fingerprint = orchestrator.exe_fingerprint.clone();
    let panel_rect = ui.available_rect_before_wrap();
    let mut action: Option<Step5Action> = None;
    clipped_pane(ui, panel_rect, |ui| {
        action = crate::ui::step5::page_step5::render(
            ui,
            &mut orchestrator.wizard_state,
            &mut orchestrator.step5_console_view,
            orchestrator.step5_terminal.as_mut(),
            orchestrator.step5_terminal_error.as_deref(),
            orchestrator.dev_mode,
            &exe_fingerprint,
        );
    });

    if action == Some(Step5Action::StartInstall) {
        let s5 = &orchestrator.wizard_state.step5;
        let already_in_flight = s5.start_install_requested || s5.install_running || s5.prep_running;
        if already_in_flight {
            tracing::debug!(
                target = "orchestrator",
                "stage_installing: ignoring Step5Action::StartInstall — \
                 install already in flight"
            );
        } else {
            orchestrator.wizard_state.step5.start_install_requested = true;
        }
    }

    match post_install_action {
        Some(PostInstallAction::ReturnToHome) => {
            outcome = StageInstallingOutcome::Nav(NavDestination::Home);
        }
        Some(PostInstallAction::OpenInstallFolder) => {
            let target = entry.unwrap_or_else(|| crate::registry::model::ModlistEntry {
                name: name.clone(),
                destination_folder: dest.clone(),
                ..Default::default()
            });
            if let Err(msg) = operations::open_install_folder(&target) {
                orchestrator.home_screen_state.toast =
                    Some(crate::ui::home::state_home::ToastMessage::error(msg));
            }
        }
        None => {}
    }

    outcome
}

fn sync_share_provenance(
    orchestrator: &mut OrchestratorApp,
    entry: Option<&crate::registry::model::ModlistEntry>,
) {
    if let Some(entry) = entry {
        orchestrator.wizard_state.set_modlist_share_provenance(
            Some(entry.name.clone()),
            entry.author.clone(),
            entry.forked_from.clone(),
        );
        return;
    }

    if let Some(preview) = orchestrator.install_screen_state.parsed_preview.as_ref() {
        orchestrator.wizard_state.set_modlist_share_provenance(
            preview.name.clone(),
            preview.author.clone(),
            preview.forked_from.clone(),
        );
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

fn back_to_import_btn(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font_size = 12.0;
    let gap = 5.0;

    let fill = redesign_shell_bg(palette);
    let text_color = redesign_text_primary(palette);
    let border = redesign_border_strong(palette);

    let glyph_font = egui::FontId::new(font_size, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));

    let glyph_galley =
        ui.painter()
            .layout_no_wrap("\u{2190}".to_string(), glyph_font.clone(), text_color);
    let prose_galley =
        ui.painter()
            .layout_no_wrap("back to import".to_string(), prose_font.clone(), text_color);

    let content_w = glyph_galley.size().x + gap + prose_galley.size().x;
    let content_h = glyph_galley.size().y.max(prose_galley.size().y);
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
        painter.rect_filled(rect, radius, fill);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border),
            egui::StrokeKind::Inside,
        );
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            "\u{2190}",
            glyph_font,
            text_color,
        );
        painter.text(
            egui::pos2(start_x + glyph_galley.size().x + gap, cy),
            egui::Align2::LEFT_CENTER,
            "back to import",
            prose_font,
            text_color,
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_name_is_the_spec_authoritative_string() {
        assert_eq!(FALLBACK_NAME, "Shared modlist");
    }

    #[test]
    fn outcome_default_is_stay() {
        assert_eq!(
            StageInstallingOutcome::default(),
            StageInstallingOutcome::Stay
        );
    }

    #[test]
    fn back_target_is_preview_when_cached_else_paste() {
        use crate::ui::install::state_install::InstallScreenState;
        let mut st = InstallScreenState::default();
        assert!(!st.preview_cached);
        let t = if st.preview_cached {
            InstallStage::Preview
        } else {
            InstallStage::Paste
        };
        assert_eq!(t, InstallStage::Paste, "no cached preview ⇒ Back to Paste");
        st.preview_cached = true;
        let t = if st.preview_cached {
            InstallStage::Preview
        } else {
            InstallStage::Paste
        };
        assert_eq!(t, InstallStage::Preview, "cached preview ⇒ Back to Preview");
    }
}
