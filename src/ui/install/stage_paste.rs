// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::install::state_install::{InstallScreenState, InstallStage};
use crate::ui::install::sub_flow_footer::{self, BackBtn, FooterClick, PrimaryBtn};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_error, redesign_input_bg, redesign_text_faint, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PasteOutcome {
    #[default]
    Stay,
    Advance(InstallStage),
}

const CODE_PLACEHOLDER: &str =
    "BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...\n\nPaste the full code here.";

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
) -> PasteOutcome {
    let body_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                render_screen_title(
                    ui,
                    palette,
                    "Paste a share code",
                    Some("paste a BIO share code to review what it installs"),
                );

                import_code_box(ui, palette, &mut state.import_code);

                if let Some(err) = state.preview_parse_error.as_deref() {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(err)
                            .size(13.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_error(palette)),
                    );
                }
            });
    });

    let outcome = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn {
            label: "All modlists",
        }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some("no install starts until preview is accepted"),
        None,
        PrimaryBtn {
            label: "Review",
            disabled: state.import_code.trim().is_empty(),
        },
    );

    match outcome {
        FooterClick::Back => PasteOutcome::Advance(InstallStage::Gallery),
        FooterClick::Primary => PasteOutcome::Advance(InstallStage::Review),
        FooterClick::None | FooterClick::Secondary | FooterClick::LeftAction => PasteOutcome::Stay,
    }
}

fn import_code_box(ui: &mut egui::Ui, palette: ThemePalette, code: &mut String) {
    redesign_box(ui, palette, Some("import code"), |ui| {
        ui.label(
            egui::RichText::new("BIO-MODLIST-V1 share code")
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(8.0);

        let frame = egui::Frame::default()
            .fill(redesign_input_bg(palette))
            .stroke(egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                redesign_border_strong(palette),
            ))
            .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
            .inner_margin(egui::Margin::same(12));
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(code)
                            .desired_width(f32::INFINITY)
                            .desired_rows(8)
                            .font(egui::FontId::new(
                                12.0,
                                egui::FontFamily::Name("firacode_nerd".into()),
                            ))
                            .frame(false)
                            .hint_text(
                                egui::RichText::new(CODE_PLACEHOLDER)
                                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                                    .color(redesign_text_faint(palette)),
                            )
                            .text_color(redesign_text_primary(palette))
                            .background_color(redesign_input_bg(palette)),
                    );
                });
        });
    });
}
