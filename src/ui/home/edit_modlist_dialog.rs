// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, InputOpts, redesign_btn, redesign_text_input};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

pub struct EditModlistDialog<'a> {
    pub id_salt: &'a str,
    pub name: &'a mut String,
    pub description: &'a mut String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditOutcome {
    #[default]
    Pending,
    Saved,
    Cancelled,
}

struct NameFieldEvents {
    enter_pressed: bool,
}

const MAX_WIDTH_PX: f32 = 460.0;
const MAX_DESCRIPTION_CHARS: usize = 500;

#[must_use]
pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    dialog: &mut EditModlistDialog<'_>,
) -> EditOutcome {
    let mut outcome = EditOutcome::Pending;

    let dialog_id = egui::Id::new(("orchestrator_edit_modlist_dialog", dialog.id_salt));

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(18));

    egui::Window::new("Edit modlist")
        .id(dialog_id)
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);

            ui.label(
                egui::RichText::new("Edit modlist")
                    .size(15.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(12.0);

            let name_events = render_name_field(ui, palette, dialog_id, dialog.name);

            ui.add_space(10.0);

            render_description_field(ui, palette, dialog.description);

            ui.add_space(14.0);

            let save_disabled = dialog.name.trim().is_empty();

            if let Some(footer_outcome) =
                render_footer(ui, palette, save_disabled, name_events.enter_pressed)
            {
                outcome = footer_outcome;
            }
        });

    outcome
}

fn render_name_field(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    dialog_id: egui::Id,
    name: &mut String,
) -> NameFieldEvents {
    ui.label(
        egui::RichText::new("Name")
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);

    let name_id = dialog_id.with("name_field");
    let name_response = redesign_text_input(
        ui,
        palette,
        InputOpts {
            edit: egui::TextEdit::singleline(name)
                .id(name_id)
                .char_limit(80)
                .font(egui::FontId::new(
                    14.0,
                    egui::FontFamily::Name("poppins_light".into()),
                ))
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette))
                .margin(egui::Margin::symmetric(8, 4)),
            margin: egui::Margin::symmetric(8, 4),
            size: egui::vec2(ui.available_width(), 30.0),
            border: None,
        },
    );

    let focus_marker = dialog_id.with("focused_once");
    let already_focused = ui
        .memory(|m| m.data.get_temp::<bool>(focus_marker))
        .unwrap_or(false);
    if !already_focused {
        name_response.request_focus();
        ui.memory_mut(|m| m.data.insert_temp(focus_marker, true));
    }

    let enter_pressed = (name_response.has_focus() || name_response.lost_focus())
        && ui.input(|i| i.key_pressed(egui::Key::Enter));

    NameFieldEvents { enter_pressed }
}

fn render_description_field(ui: &mut egui::Ui, palette: ThemePalette, description: &mut String) {
    ui.label(
        egui::RichText::new("Description")
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);

    let description_frame = egui::Frame::default()
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(12));
    description_frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.add(
            egui::TextEdit::multiline(description)
                .desired_width(f32::INFINITY)
                .desired_rows(4)
                .char_limit(MAX_DESCRIPTION_CHARS)
                .frame(false)
                .font(egui::FontId::new(
                    13.0,
                    egui::FontFamily::Name("poppins_light".into()),
                ))
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette)),
        );
    });

    let char_count = description.chars().count();
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        ui.label(
            egui::RichText::new(format!("{char_count} / {MAX_DESCRIPTION_CHARS}"))
                .size(11.0)
                .color(redesign_text_faint(palette)),
        );
    });
}

fn render_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    save_disabled: bool,
    enter_pressed: bool,
) -> Option<EditOutcome> {
    let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

    let mut outcome = None;
    let footer_h = 30.0;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), footer_h),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            let save_clicked = redesign_btn(
                ui,
                palette,
                "Save",
                BtnOpts {
                    small: true,
                    primary: true,
                    disabled: save_disabled,
                    ..Default::default()
                },
            )
            .clicked();

            let cancel_clicked = redesign_btn(
                ui,
                palette,
                "Cancel",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked();

            if save_clicked || (enter_pressed && !save_disabled) {
                outcome = Some(EditOutcome::Saved);
            } else if cancel_clicked || escape_pressed {
                outcome = Some(EditOutcome::Cancelled);
            }
        },
    );

    outcome
}
