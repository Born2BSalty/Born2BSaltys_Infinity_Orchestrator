// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{Duration, Instant};

use eframe::egui;

use crate::registry::model::ModlistEntry;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_I8, ThemePalette,
    redesign_border_strong, redesign_input_bg, redesign_shadow, redesign_shell_bg,
    redesign_success, redesign_text_muted, redesign_text_primary,
};
use crate::ui::workspace::step5::state_workspace_step5::WorkspaceStep5State;

const MAX_WIDTH_PX: f32 = 600.0;
const CODE_BOX_MAX_HEIGHT_PX: f32 = 180.0;
const COPIED_FLASH: Duration = Duration::from_millis(1500);

pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    state: &mut WorkspaceStep5State,
    entry: &ModlistEntry,
) {
    if !state.share_dialog_open {
        return;
    }

    let code = entry.latest_share_code.as_deref().filter(|c| !c.is_empty());
    let flashing = copied_flash_active(state);

    let mut close_clicked = false;
    let mut copy_clicked = false;

    egui::Window::new("Share import code")
        .id(egui::Id::new("orchestrator_share_paste_code_dialog"))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(dialog_frame(palette))
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);
            render_header(ui, palette);
            render_code_box(ui, palette, code);
            render_footer(
                ui,
                palette,
                code.is_some(),
                flashing,
                &mut copy_clicked,
                &mut close_clicked,
            );
        });

    apply_dialog_outcome(ctx, state, code, copy_clicked, close_clicked);
}

fn copied_flash_active(state: &mut WorkspaceStep5State) -> bool {
    match state.copied_flash_until {
        Some(until) if Instant::now() < until => true,
        Some(_) => {
            state.copied_flash_until = None;
            false
        }
        None => false,
    }
}

fn dialog_frame(palette: ThemePalette) -> egui::Frame {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(20))
        .shadow(egui::epaint::Shadow {
            offset: [REDESIGN_SHADOW_OFFSET_I8 - 1, REDESIGN_SHADOW_OFFSET_I8 - 1],
            blur: 0,
            spread: 0,
            color: redesign_shadow(palette),
        })
}

fn render_header(ui: &mut egui::Ui, palette: ThemePalette) {
    ui.label(
        egui::RichText::new("Share import code")
            .size(18.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Anyone can paste this into BIO \u{2192} Install to get the same modlist.",
        )
        .size(13.0)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(redesign_text_muted(palette)),
    );
    ui.add_space(14.0);
}

fn render_code_box(ui: &mut egui::Ui, palette: ThemePalette, code: Option<&str>) {
    egui::Frame::default()
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(CODE_BOX_MAX_HEIGHT_PX)
                .auto_shrink([false, true])
                .show(ui, |ui| render_code_text(ui, palette, code));
        });
    ui.add_space(14.0);
}

fn render_code_text(ui: &mut egui::Ui, palette: ThemePalette, code: Option<&str>) {
    match code {
        Some(c) => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(c)
                        .size(12.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_text_primary(palette)),
                )
                .wrap(),
            );
        }
        None => {
            ui.label(
                egui::RichText::new("No import code available yet for this modlist.")
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            );
        }
    }
}

fn render_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy_enabled: bool,
    flashing: bool,
    copy_clicked: &mut bool,
    close_clicked: &mut bool,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 30.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            *copy_clicked = copy_button_clicked(ui, palette, copy_enabled);
            *close_clicked = close_button_clicked(ui, palette);
            if flashing {
                render_copied_status(ui, palette);
            }
        },
    );
}

fn copy_button_clicked(ui: &mut egui::Ui, palette: ThemePalette, enabled: bool) -> bool {
    enabled
        && redesign_btn(
            ui,
            palette,
            "Copy",
            BtnOpts {
                small: true,
                primary: true,
                disabled: !enabled,
                ..Default::default()
            },
        )
        .clicked()
}

fn close_button_clicked(ui: &mut egui::Ui, palette: ThemePalette) -> bool {
    redesign_btn(
        ui,
        palette,
        "Close",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .clicked()
}

fn render_copied_status(ui: &mut egui::Ui, palette: ThemePalette) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.label(
            egui::RichText::new("\u{2713}")
                .size(14.0)
                .family(egui::FontFamily::Name("firacode_nerd".into()))
                .color(redesign_success(palette)),
        );
        ui.label(
            egui::RichText::new(" copied to clipboard")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_success(palette)),
        );
    });
}

fn apply_dialog_outcome(
    ctx: &egui::Context,
    state: &mut WorkspaceStep5State,
    code: Option<&str>,
    copy_clicked: bool,
    close_clicked: bool,
) {
    if copy_clicked && let Some(c) = code {
        ctx.copy_text(c.to_string());
        state.copied_flash_until = Some(Instant::now() + COPIED_FLASH);
    }
    if close_clicked {
        state.share_dialog_open = false;
        state.copied_flash_until = None;
    }

    if state.copied_flash_until.is_some() {
        ctx.request_repaint_after(Duration::from_millis(120));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry};

    #[test]
    fn closed_dialog_is_a_noop_state_unchanged() {
        let s = WorkspaceStep5State::default();
        assert!(!s.share_dialog_open);
        assert!(s.copied_flash_until.is_none());
    }

    #[test]
    fn code_source_is_the_registry_entry_snapshot() {
        let with_code = ModlistEntry {
            id: "S".to_string(),
            name: "n".to_string(),
            game: Game::EET,
            latest_share_code: Some("BIO-MODLIST-V1:ABC".to_string()),
            ..Default::default()
        };
        assert_eq!(
            with_code
                .latest_share_code
                .as_deref()
                .filter(|c| !c.is_empty()),
            Some("BIO-MODLIST-V1:ABC")
        );

        let no_code = ModlistEntry {
            latest_share_code: Some(String::new()),
            ..with_code.clone()
        };
        assert_eq!(
            no_code
                .latest_share_code
                .as_deref()
                .filter(|c| !c.is_empty()),
            None
        );
        let none_code = ModlistEntry {
            latest_share_code: None,
            ..with_code
        };
        assert_eq!(none_code.latest_share_code.as_deref(), None);
    }
}
