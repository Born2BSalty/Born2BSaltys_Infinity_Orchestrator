// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

pub struct ShareModlistDialog<'a> {
    pub id_salt: &'a str,
    pub modlist_name: &'a str,
    pub has_code: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShareOutcome {
    #[default]
    Pending,
    ExportFile,
    CopyCode,
    Closed,
}

const MAX_WIDTH_PX: f32 = 460.0;

#[must_use]
pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    dialog: &ShareModlistDialog<'_>,
) -> ShareOutcome {
    let mut outcome = ShareOutcome::Pending;

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(18));

    egui::Window::new("Share this modlist")
        .id(egui::Id::new((
            "orchestrator_share_modlist_dialog",
            dialog.id_salt,
        )))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);

            render_header(ui, palette, dialog.modlist_name);

            if let Some(choice) = render_export_choice(ui, palette, dialog.has_code) {
                outcome = choice;
            }

            ui.add_space(12.0);

            if let Some(choice) = render_copy_choice(ui, palette, dialog.has_code) {
                outcome = choice;
            }

            if !dialog.has_code {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("No share code yet for this modlist.")
                        .size(12.0)
                        .color(redesign_text_faint(palette)),
                );
            }

            ui.add_space(14.0);

            if render_footer(ui, palette) {
                outcome = ShareOutcome::Closed;
            }
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome = ShareOutcome::Closed;
    }

    outcome
}

fn render_header(ui: &mut egui::Ui, palette: ThemePalette, modlist_name: &str) {
    ui.label(
        egui::RichText::new("Share this modlist")
            .size(15.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(modlist_name)
            .size(13.0)
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(14.0);
}

fn render_export_choice(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    has_code: bool,
) -> Option<ShareOutcome> {
    let clicked = redesign_btn(
        ui,
        palette,
        "Export modlist file",
        BtnOpts {
            primary: true,
            block: true,
            disabled: !has_code,
            ..Default::default()
        },
    )
    .clicked();

    ui.label(
        egui::RichText::new(
            "Saves a .biolist file with everything needed to reinstall this list. \
             Send it, upload it, or submit it to the gallery.",
        )
        .size(12.0)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(redesign_text_muted(palette)),
    );

    clicked.then_some(ShareOutcome::ExportFile)
}

fn render_copy_choice(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    has_code: bool,
) -> Option<ShareOutcome> {
    let clicked = redesign_btn(
        ui,
        palette,
        "Copy share code",
        BtnOpts {
            block: true,
            disabled: !has_code,
            ..Default::default()
        },
    )
    .clicked();

    ui.label(
        egui::RichText::new(
            "A text code you can paste into a chat. Long lists make long codes; \
             the file is easier to hand around.",
        )
        .size(12.0)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(redesign_text_muted(palette)),
    );

    clicked.then_some(ShareOutcome::CopyCode)
}

fn render_footer(ui: &mut egui::Ui, palette: ThemePalette) -> bool {
    let mut close_clicked = false;
    let footer_h = 30.0;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), footer_h),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            close_clicked = redesign_btn(
                ui,
                palette,
                "Close",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked();
        },
    );
    close_clicked
}
