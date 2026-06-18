// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::install::state_install::DestChoice;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette,
};

const WARN_BORDER: egui::Color32 = egui::Color32::from_rgb(0xed, 0xc5, 0x47);
const WARN_INK: egui::Color32 = egui::Color32::from_rgb(0xff, 0xff, 0xff);
fn warn_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(0xED, 0xC5, 0x47, 46)
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    choice: Option<DestChoice>,
    allow_partial: bool,
) -> Option<DestChoice> {
    ui.add_space(12.0);

    let mut picked: Option<DestChoice> = None;

    let frame = egui::Frame::default()
        .fill(warn_fill())
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, WARN_BORDER))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        });

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let (icon_rect, _) =
                ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
            paint_warning_triangle(ui.painter(), icon_rect.center(), WARN_INK);
            ui.label(
                egui::RichText::new("Target directory not empty")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(WARN_INK),
            );
        });

        ui.add_space(4.0);

        ui.label(
            egui::RichText::new("How would you like to proceed?")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(egui::Color32::from_rgba_unmultiplied(
                    0xff, 0xff, 0xff, 0xCC,
                )),
        );

        ui.add_space(10.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            for (opt, label) in option_set(allow_partial) {
                let is_active = choice == Some(opt);
                if redesign_btn(
                    ui,
                    palette,
                    label,
                    BtnOpts {
                        small: true,
                        primary: is_active,
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    picked = Some(opt);
                }
            }
        });
    });

    picked
}

fn paint_warning_triangle(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let hw = 6.5;
    let top = center.y - 6.0;
    let base_y = center.y + 5.0;

    painter.add(egui::Shape::closed_line(
        vec![
            egui::pos2(center.x, top),
            egui::pos2(center.x + hw, base_y),
            egui::pos2(center.x - hw, base_y),
        ],
        stroke,
    ));

    painter.line_segment(
        [
            egui::pos2(center.x, center.y - 2.0),
            egui::pos2(center.x, center.y + 1.5),
        ],
        stroke,
    );
    painter.circle_filled(egui::pos2(center.x, center.y + 3.6), 0.95, color);
}

fn option_set(allow_partial: bool) -> Vec<(DestChoice, &'static str)> {
    let mut opts = vec![
        (DestChoice::Clear, "Clear contents"),
        (DestChoice::Backup, "Backup contents then proceed"),
    ];
    if allow_partial {
        opts.push((DestChoice::Continue, "Continue partial installation"));
    }
    opts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_labels_are_wireframe_verbatim() {
        let opts = option_set(true);
        assert_eq!(
            opts,
            vec![
                (DestChoice::Clear, "Clear contents"),
                (DestChoice::Backup, "Backup contents then proceed"),
                (DestChoice::Continue, "Continue partial installation"),
            ]
        );
    }

    #[test]
    fn continue_option_hidden_when_partial_disallowed() {
        let opts = option_set(false);
        assert_eq!(
            opts,
            vec![
                (DestChoice::Clear, "Clear contents"),
                (DestChoice::Backup, "Backup contents then proceed"),
            ]
        );
        assert!(!opts.iter().any(|(c, _)| *c == DestChoice::Continue));
    }
}
