// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::{ModlistRegistry, ModlistState};
use crate::ui::home::modlist_card::{self, ModlistCardActions};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_success, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LoadDraftOutcome {
    #[default]
    Pending,
    Cancelled,
    Resume(String),
    CopyImportCode(String),
    Delete(String),
}

const MAX_WIDTH_PX: f32 = 620.0;

#[must_use]
pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    registry: &ModlistRegistry,
    copied_name: Option<&str>,
) -> LoadDraftOutcome {
    let mut outcome = LoadDraftOutcome::Pending;
    let frame = dialog_frame(palette);
    egui::Window::new("Resume in-progress build")
        .id(egui::Id::new("create_load_draft_dialog"))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            render_body(ui, palette, registry, copied_name, &mut outcome);
        });
    outcome
}

fn dialog_frame(palette: ThemePalette) -> egui::Frame {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(22))
}

fn render_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    registry: &ModlistRegistry,
    copied_name: Option<&str>,
    outcome: &mut LoadDraftOutcome,
) {
    ui.set_max_width(MAX_WIDTH_PX);
    ui.set_width(MAX_WIDTH_PX.min(ui.available_width()));
    render_header(ui, palette);

    let in_progress: Vec<&crate::registry::model::ModlistEntry> = registry
        .entries
        .iter()
        .filter(|e| e.state == ModlistState::InProgress)
        .collect();
    render_draft_list(ui, palette, &in_progress, outcome);
    render_footer(ui, palette, outcome);
    render_copied_toast(ui, palette, copied_name);
}

fn render_header(ui: &mut egui::Ui, palette: ThemePalette) {
    ui.label(
        egui::RichText::new("Resume in-progress build")
            .size(18.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Pick a build to resume. BIO restores its order, selection, and settings and drops you back where you left off.",
        )
        .size(13.0)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(redesign_text_muted(palette)),
    );
    ui.add_space(16.0);
}

fn render_draft_list(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    in_progress: &[&crate::registry::model::ModlistEntry],
    outcome: &mut LoadDraftOutcome,
) {
    if in_progress.is_empty() {
        render_empty_state(ui, palette);
    } else {
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                for entry in in_progress {
                    apply_card_action(outcome, modlist_card::render(ui, palette, entry), entry);
                }
            });
    }
}

fn render_empty_state(ui: &mut egui::Ui, palette: ThemePalette) {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 20,
            right: 20,
            top: 16,
            bottom: 16,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new("No in-progress builds. Start a new modlist from Create.")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
            );
        });
}

fn apply_card_action(
    outcome: &mut LoadDraftOutcome,
    action: ModlistCardActions,
    entry: &crate::registry::model::ModlistEntry,
) {
    match action {
        ModlistCardActions::Resume => {
            *outcome = LoadDraftOutcome::Resume(entry.id.clone());
        }
        ModlistCardActions::CopyImportCode => {
            *outcome = LoadDraftOutcome::CopyImportCode(entry.id.clone());
        }
        ModlistCardActions::Delete => {
            *outcome = LoadDraftOutcome::Delete(entry.id.clone());
        }
        _ => {}
    }
}

fn render_footer(ui: &mut egui::Ui, palette: ThemePalette, outcome: &mut LoadDraftOutcome) {
    ui.add_space(14.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 30.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            if crate::ui::orchestrator::widgets::redesign_btn(
                ui,
                palette,
                "Cancel",
                crate::ui::orchestrator::widgets::BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                *outcome = LoadDraftOutcome::Cancelled;
            }
        },
    );
}

fn render_copied_toast(ui: &mut egui::Ui, palette: ThemePalette, copied_name: Option<&str>) {
    let Some(name) = copied_name else {
        return;
    };
    ui.add_space(10.0);
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 12,
            right: 12,
            top: 6,
            bottom: 6,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    egui::RichText::new("\u{2713}")
                        .size(13.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_success(palette)),
                );
                ui.label(
                    egui::RichText::new(format!("Copied import code for \"{name}\""))
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_success(palette)),
                );
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_default_is_pending() {
        assert_eq!(LoadDraftOutcome::default(), LoadDraftOutcome::Pending);
    }

    #[test]
    fn resume_and_copy_carry_the_id() {
        assert_eq!(
            LoadDraftOutcome::Resume("ABC".to_string()),
            LoadDraftOutcome::Resume("ABC".to_string())
        );
        assert_ne!(
            LoadDraftOutcome::Resume("ABC".to_string()),
            LoadDraftOutcome::CopyImportCode("ABC".to_string())
        );
    }
}
