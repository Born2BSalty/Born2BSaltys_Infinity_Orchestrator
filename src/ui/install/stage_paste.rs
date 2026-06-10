// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::install::destination_not_empty;
use crate::ui::install::state_install::{InstallScreenState, InstallStage};
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::widgets::{
    InputOpts, redesign_box, redesign_text_input, render_screen_title,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PasteOutcome {
    #[default]
    Stay,
    Advance(InstallStage),
}

const CODE_PLACEHOLDER: &str =
    "BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...\n\nPaste the full code here.";
const BROWSE_W_PX: f32 = 96.0;

const FORM_INPUT_MARGIN: egui::Margin = egui::Margin {
    left: 12,
    right: 12,
    top: 8,
    bottom: 8,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
) -> PasteOutcome {
    let is_partial = state.is_partial();

    render_screen_title(
        ui,
        palette,
        "Install shared modlist",
        Some(if is_partial {
            "destination has existing modlist \u{2014} share code skipped"
        } else {
            "set destination + mods paths, paste a BIO share code, then preview before importing"
        }),
    );

    redesign_box(ui, palette, None, |ui| {
        let dest_changed = folder_input(
            ui,
            palette,
            "destination folder",
            "D:\\BG2EE_install_test",
            &mut state.destination,
        );
        if dest_changed {
            state.destination_choice = None;
        }

        if destination_is_non_empty(&state.destination)
            && let Some(picked) =
                destination_not_empty::render(ui, palette, state.destination_choice, true)
        {
            state.destination_choice = Some(picked);
        }
    });

    ui.add_space(14.0);

    if is_partial {
        partial_info_box(ui, palette, &state.destination);
        let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
        if spacer > 0.0 {
            ui.add_space(spacer);
        }
    } else {
        let box_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(160.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), box_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| import_code_box(ui, palette, &mut state.import_code),
        );
    }

    let dest_valid = {
        let t = state.destination.trim();
        !t.is_empty() && std::path::Path::new(t).is_dir()
    };
    let code_empty = state.import_code.trim().is_empty();
    let primary_disabled = !dest_valid || (!is_partial && code_empty);
    let hint: &str = if !dest_valid {
        "set a valid destination folder (browse to a real folder) to continue"
    } else if is_partial {
        "no share code needed"
    } else {
        "no install starts until preview is accepted"
    };
    let outcome = sub_flow_footer::render(
        ui,
        palette,
        None::<BackBtn<'_>>,
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some(hint),
        PrimaryBtn {
            label: if is_partial {
                "Continue Install"
            } else {
                "Preview"
            },
            disabled: primary_disabled,
        },
    );

    if outcome.primary_clicked {
        return PasteOutcome::Advance(if is_partial {
            InstallStage::InstallingStub
        } else {
            InstallStage::Preview
        });
    }

    PasteOutcome::Stay
}

fn destination_is_non_empty(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    std::fs::read_dir(trimmed).is_ok_and(|mut entries| entries.next().is_some())
}

fn folder_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    placeholder: &str,
    value: &mut String,
) -> bool {
    let mut changed = false;

    ui.label(
        egui::RichText::new(label)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);

    let box_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            14.0,
            egui::FontFamily::Name("poppins_light".into()),
        ))
    }) + f32::from(FORM_INPUT_MARGIN.top)
        + f32::from(FORM_INPUT_MARGIN.bottom);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        let reserved = BROWSE_W_PX + 8.0;
        let edit_width = (ui.available_width() - reserved).max(120.0);

        let pre = value.clone();
        let response = redesign_text_input(
            ui,
            palette,
            InputOpts {
                edit: egui::TextEdit::singleline(value)
                    .font(egui::FontId::new(
                        12.0,
                        egui::FontFamily::Name("firacode_nerd".into()),
                    ))
                    .hint_text(
                        egui::RichText::new(placeholder)
                            .family(egui::FontFamily::Name("firacode_nerd".into()))
                            .color(redesign_text_faint(palette)),
                    )
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .vertical_align(egui::Align::Center)
                    .margin(FORM_INPUT_MARGIN),
                margin: FORM_INPUT_MARGIN,
                size: egui::vec2(edit_width, box_h),
                border: None,
            },
        );
        if response.changed() || *value != pre {
            changed = true;
        }

        if ui
            .add_sized(
                egui::vec2(BROWSE_W_PX, box_h),
                egui::Button::new(
                    egui::RichText::new("browse\u{2026}")
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                )
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                )),
            )
            .clicked()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            let s = path.to_string_lossy().to_string();
            if s != *value {
                *value = s;
                changed = true;
            }
        }
    });

    changed
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

fn partial_info_box(ui: &mut egui::Ui, palette: ThemePalette, dest: &str) {
    redesign_box(ui, palette, None, |ui| {
        ui.label(
            egui::RichText::new("Continue partial installation")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!(
                "Existing mod files detected at {dest}. Share-code entry is skipped \u{2014} BIO will pick up where the previous install left off."
            ))
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
        );
    });
}
