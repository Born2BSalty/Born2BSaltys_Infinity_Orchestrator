// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::registry::model::ModlistEntry;
use crate::ui::shared::format_relative::{format_install_duration, relative_time};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_success,
    redesign_text_faint, redesign_text_primary,
};

#[must_use]
pub fn clean_exit(state: &WizardState) -> bool {
    !state.step5.install_running
        && state.step5.last_exit_code == Some(0)
        && !state.step5.last_install_failed
}

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &WizardState, entry: &ModlistEntry) {
    if !clean_exit(state) {
        return;
    }

    let pad_x: i8 = 14;
    let pad_y: i8 = 10;
    let gap = 12.0;
    let margin_bottom = 10.0;

    let success = redesign_success(palette);

    egui::Frame::default()
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, success))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: pad_x,
            right: pad_x,
            top: pad_y,
            bottom: pad_y,
        })
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 0.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = gap;

                    installed_pill(ui, success);

                    ui.label(
                        egui::RichText::new(format!(
                            "{} mods \u{00B7} {} components \u{00B7} no errors",
                            entry.mod_count, entry.component_count
                        ))
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_primary(palette)),
                    );

                    let ran_finished = duration_line(entry);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(ran_finished)
                                .size(13.0)
                                .family(egui::FontFamily::Name("firacode_nerd".into()))
                                .color(redesign_text_faint(palette)),
                        );
                    });
                },
            );
        });

    ui.add_space(margin_bottom);
}

fn installed_pill(ui: &mut egui::Ui, success: egui::Color32) {
    let pad_x = 8.0;
    let pad_y = 2.0;
    let text_color = egui::Color32::from_rgb(0x0B, 0x11, 0x16);
    let font = egui::FontId::new(11.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap("Installed".to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            success,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Installed",
            font,
            text_color,
        );
    }
}

fn duration_line(entry: &ModlistEntry) -> String {
    let ran = match (entry.install_started_at, entry.install_date) {
        (Some(start), Some(end)) => {
            let secs = end.signed_duration_since(start).num_seconds();
            if secs < 0 {
                "\u{2014}:\u{2014}".to_string()
            } else {
                format_install_duration(std::time::Duration::from_secs(
                    u64::try_from(secs).unwrap_or(0),
                ))
            }
        }
        _ => "\u{2014}:\u{2014}".to_string(),
    };
    let finished = entry
        .install_date
        .map_or_else(|| "just now".to_string(), relative_time);
    format!("ran {ran} \u{00B7} finished {finished}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry};
    use chrono::{Duration as ChronoDuration, Utc};

    #[test]
    fn fresh_state_is_not_clean_exit() {
        let s = WizardState::default();
        assert!(!clean_exit(&s), "no install has run ⇒ the banner is hidden");
    }

    #[test]
    fn running_install_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = true;
        s.step5.last_exit_code = Some(0);
        assert!(
            !clean_exit(&s),
            "an install still running is not a clean exit"
        );
    }

    #[test]
    fn nonzero_exit_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(1);
        assert!(!clean_exit(&s), "a nonzero exit code is not clean");
    }

    #[test]
    fn flagged_failure_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(0);
        s.step5.last_install_failed = true;
        assert!(
            !clean_exit(&s),
            "a BIO-flagged likely-failure is not a clean exit even at \
             exit code 0"
        );
    }

    #[test]
    fn clean_triple_holds() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(0);
        s.step5.last_install_failed = false;
        assert!(
            clean_exit(&s),
            "not running + exit 0 + not flagged ⇒ clean exit (the C3 \
             triple the banner/post-install/registry-flip gate on)"
        );
    }

    #[test]
    fn duration_line_formats_mm_ss_and_relative() {
        let start = Utc::now() - ChronoDuration::seconds(4 * 60 + 12);
        let end = start + ChronoDuration::seconds(4 * 60 + 12);
        let entry = ModlistEntry {
            id: "B".to_string(),
            name: "n".to_string(),
            game: Game::EET,
            install_started_at: Some(start),
            install_date: Some(end),
            ..Default::default()
        };
        let line = duration_line(&entry);
        assert!(
            line.starts_with("ran 4:12 \u{00B7} finished "),
            "expected `ran 4:12 · finished …`, got: {line}"
        );
    }

    #[test]
    fn duration_line_handles_missing_start_and_skew() {
        let end = Utc::now();
        let entry = ModlistEntry {
            install_started_at: None,
            install_date: Some(end),
            ..Default::default()
        };
        let line = duration_line(&entry);
        assert!(line.starts_with("ran \u{2014}:\u{2014} \u{00B7} finished "));

        let now = Utc::now();
        let skew = ModlistEntry {
            install_started_at: Some(now),
            install_date: Some(now - ChronoDuration::seconds(30)),
            ..Default::default()
        };
        assert!(duration_line(&skew).starts_with("ran \u{2014}:\u{2014}"));
    }
}
