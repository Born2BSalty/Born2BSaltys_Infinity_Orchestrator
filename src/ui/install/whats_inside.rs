// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::preview_counts;
use crate::ui::orchestrator::widgets::redesign_section_header;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_accent_hover, redesign_border_strong, redesign_shell_bg, redesign_text_faint,
    redesign_text_muted, redesign_text_primary, redesign_with_alpha,
};

const HEADING: &str = "What's inside";
const SUB: &str = "Everything this modlist will download and install, in order.";
const ALSO_INCLUDED: &str = "Also included:";
const REFS_LINK: &str = "installed refs & pins";
const SOURCES_LINK: &str = "download sources";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InsideCounts {
    pub(crate) mods: usize,
    pub(crate) components: usize,
    pub(crate) per_game: Vec<(String, usize)>,
    pub(crate) refs: bool,
    pub(crate) sources: bool,
    pub(crate) configs: usize,
}

impl InsideCounts {
    pub(crate) fn from_preview(preview: &ModlistSharePreview) -> Self {
        let mods =
            preview_counts::distinct_mod_count(&preview.bgee_log_text, &preview.bg2ee_log_text);
        let components = preview.bgee_entries + preview.bg2ee_entries;
        let per_game = if preview.game_install.eq_ignore_ascii_case("EET") {
            vec![
                ("BGEE".to_string(), preview.bgee_entries),
                ("BG2EE".to_string(), preview.bg2ee_entries),
            ]
        } else if preview.game_install.eq_ignore_ascii_case("BG2EE") {
            vec![(preview.game_install.clone(), preview.bg2ee_entries)]
        } else {
            vec![(preview.game_install.clone(), preview.bgee_entries)]
        };
        Self {
            mods,
            components,
            per_game,
            refs: preview.has_installed_refs,
            sources: preview.has_source_overrides,
            configs: preview.mod_config_count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum InsideClick {
    #[default]
    None,
    IncludedMods,
    WeiduLogs,
    InstalledRefs,
    DownloadSources,
    ConfigFiles,
}

#[must_use]
pub(crate) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else if word == "entry" {
        format!("{n} entries")
    } else {
        format!("{n} {word}s")
    }
}

#[must_use]
pub(crate) fn mods_sub(counts: &InsideCounts) -> String {
    format!(
        "{} \u{00B7} {}",
        plural(counts.mods, "mod"),
        plural(counts.components, "component")
    )
}

#[must_use]
pub(crate) fn logs_sub(counts: &InsideCounts) -> String {
    counts
        .per_game
        .iter()
        .map(|(game, n)| format!("{game} {n}"))
        .collect::<Vec<_>>()
        .join(" \u{00B7} ")
}

pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    counts: &InsideCounts,
) -> InsideClick {
    let mut click = InsideClick::None;

    redesign_section_header(ui, palette, HEADING, None);
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(SUB)
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(14.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
        if labelled_button(ui, palette, "Included mods", &mods_sub(counts), true).clicked() {
            click = InsideClick::IncludedMods;
        }
        if labelled_button(ui, palette, "WeiDU logs", &logs_sub(counts), false).clicked() {
            click = InsideClick::WeiduLogs;
        }
    });

    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        muted_label(ui, palette, ALSO_INCLUDED);
        if link(ui, palette, REFS_LINK).clicked() {
            click = InsideClick::InstalledRefs;
        }
        separator(ui, palette);
        if link(ui, palette, SOURCES_LINK).clicked() {
            click = InsideClick::DownloadSources;
        }
        separator(ui, palette);
        if link(ui, palette, &format!("config files ({})", counts.configs)).clicked() {
            click = InsideClick::ConfigFiles;
        }
    });

    click
}

fn muted_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
}

fn separator(ui: &mut egui::Ui, palette: ThemePalette) {
    muted_label(ui, palette, "\u{00B7}");
}

fn labelled_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    sub: &str,
    primary: bool,
) -> egui::Response {
    let pad_x = 14.0_f32;
    let pad_y = 10.0_f32;
    let gap = 9.0_f32;

    let label_font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into()));
    let sub_font = egui::FontId::new(11.0, egui::FontFamily::Name("poppins_light".into()));

    let fill = if primary {
        redesign_accent(palette)
    } else {
        redesign_shell_bg(palette)
    };
    let label_color = if primary {
        egui::Color32::from_rgb(0x1a, 0x26, 0x38)
    } else {
        redesign_text_primary(palette)
    };
    let sub_color = redesign_with_alpha(label_color, 7, 10);

    let label_galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), label_font.clone(), label_color);
    let sub_galley = ui
        .painter()
        .layout_no_wrap(sub.to_string(), sub_font.clone(), sub_color);

    let content_w = label_galley.size().x + gap + sub_galley.size().x;
    let content_h = label_galley.size().y.max(sub_galley.size().y);
    let desired = egui::vec2(pad_x.mul_add(2.0, content_w), pad_y.mul_add(2.0, content_h));

    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(rect, radius, fill);
        if !primary {
            painter.rect_stroke(
                rect,
                radius,
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                egui::StrokeKind::Inside,
            );
        }
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            label,
            label_font,
            label_color,
        );
        painter.text(
            egui::pos2(start_x + label_galley.size().x + gap, cy),
            egui::Align2::LEFT_CENTER,
            sub,
            sub_font,
            sub_color,
        );
    }

    response
}

fn link(ui: &mut egui::Ui, palette: ThemePalette, text: &str) -> egui::Response {
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()));
    let color = redesign_accent(palette);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), color);
    let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painted_color = if response.hovered() {
            redesign_accent_hover(palette)
        } else {
            color
        };
        ui.painter().text(
            rect.left_center(),
            egui::Align2::LEFT_CENTER,
            text,
            font,
            painted_color,
        );
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
            ],
            egui::Stroke::new(1.0_f32, redesign_text_faint(palette)),
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eet_preview() -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "0.1.0-test".to_string(),
            game_install: "EET".to_string(),
            install_mode: "build_from_scanned_mods".to_string(),
            bgee_entries: 3,
            bg2ee_entries: 4,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: "~A/A.TP2~ #0 #0 // A".to_string(),
            bg2ee_log_text: "~B/B.TP2~ #0 #0 // B".to_string(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            forked_from: Vec::new(),
        }
    }

    #[test]
    fn counts_come_from_the_preview_for_eet_and_single_game() {
        let eet = eet_preview();
        let counts = InsideCounts::from_preview(&eet);
        assert_eq!(counts.components, 7);
        assert_eq!(
            counts.per_game,
            vec![("BGEE".to_string(), 3), ("BG2EE".to_string(), 4)]
        );

        let mut single = eet_preview();
        single.game_install = "BGEE".to_string();
        let counts = InsideCounts::from_preview(&single);
        assert_eq!(counts.per_game, vec![("BGEE".to_string(), 3)]);

        let mut bg2ee_only = eet_preview();
        bg2ee_only.game_install = "BG2EE".to_string();
        let counts = InsideCounts::from_preview(&bg2ee_only);
        assert_eq!(counts.per_game, vec![("BG2EE".to_string(), 4)]);
    }

    #[test]
    fn plural_handles_entry_entries() {
        assert_eq!(plural(1, "entry"), "1 entry");
        assert_eq!(plural(2, "entry"), "2 entries");
        assert_eq!(plural(1, "mod"), "1 mod");
        assert_eq!(plural(2, "mod"), "2 mods");
    }

    #[test]
    fn sub_labels_are_verbatim() {
        let counts = InsideCounts::from_preview(&eet_preview());
        assert_eq!(mods_sub(&counts), "2 mods \u{00B7} 7 components");
        assert_eq!(logs_sub(&counts), "BGEE 3 \u{00B7} BG2EE 4");
    }
}
