// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::ThemePalette;

pub(crate) const fn selected_row_fill(palette: ThemePalette) -> Option<egui::Color32> {
    match palette {
        ThemePalette::Dark => Some(egui::Color32::from_rgb(16, 46, 44)),
        ThemePalette::Light => None,
    }
}

pub(crate) fn selectable_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    selected: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    ui.scope(|ui| {
        if selected && let Some(fill) = selected_row_fill(palette) {
            ui.visuals_mut().selection.bg_fill = fill;
        }
        ui.selectable_label(selected, text)
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_row_fill_is_dark_teal_on_dark_and_default_on_light() {
        assert_eq!(
            selected_row_fill(ThemePalette::Dark),
            Some(egui::Color32::from_rgb(16, 46, 44))
        );
        assert_eq!(selected_row_fill(ThemePalette::Light), None);
    }

    fn painted_rect_fills(palette: ThemePalette, selected: bool) -> (Vec<egui::Color32>, bool) {
        let ctx = egui::Context::default();
        let mut fill_leaked = false;
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let before = ui.visuals().selection.bg_fill;
                let _ = selectable_row(ui, palette, selected, "row");
                fill_leaked = ui.visuals().selection.bg_fill != before;
            });
        });
        let fills = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect) => Some(rect.fill),
                _ => None,
            })
            .collect();
        (fills, fill_leaked)
    }

    #[test]
    fn selectable_row_paints_dark_teal_band_when_selected_on_dark() {
        let (fills, leaked) = painted_rect_fills(ThemePalette::Dark, true);
        assert!(fills.contains(&egui::Color32::from_rgb(16, 46, 44)));
        assert!(!leaked);
    }

    #[test]
    fn selectable_row_keeps_shared_band_on_light() {
        let (fills, leaked) = painted_rect_fills(ThemePalette::Light, true);
        assert!(!fills.contains(&egui::Color32::from_rgb(16, 46, 44)));
        assert!(!leaked);
    }

    #[test]
    fn selectable_row_paints_no_band_when_unselected() {
        let (fills, leaked) = painted_rect_fills(ThemePalette::Dark, false);
        assert!(!fills.contains(&egui::Color32::from_rgb(16, 46, 44)));
        assert!(!leaked);
    }
}
