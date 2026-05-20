// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::create::state_create::CreateScreenState;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_text_faint, redesign_text_primary,
};

const CODE_PLACEHOLDER: &str =
    "BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...\n\nPaste the full code here.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForkPasteOutcome {
    #[default]
    Stay,
    Back,
    Preview,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
) -> ForkPasteOutcome {
    render_screen_title(
        ui,
        palette,
        "Import and modify another modlist",
        Some("paste a share code \u{2014} preview, then BIO downloads + preselects"),
    );
    ui.add_space(8.0);

    let box_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(160.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), box_h),
        egui::Layout::top_down(egui::Align::Min),
        |ui| import_code_box(ui, palette, &mut state.fork_code),
    );

    let code_empty = state.fork_code.trim().is_empty();
    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Back" }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some(if code_empty {
            "paste a BIO share code to preview"
        } else {
            "no download starts until preview is accepted"
        }),
        PrimaryBtn {
            label: "Preview",
            disabled: code_empty,
        },
    );

    if footer.back_clicked {
        ForkPasteOutcome::Back
    } else if footer.primary_clicked {
        ForkPasteOutcome::Preview
    } else {
        ForkPasteOutcome::Stay
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
