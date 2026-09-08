// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::install::gallery::card_art;
use crate::ui::install::gallery::catalog::GalleryEntry;
use crate::ui::install::sub_flow_footer::{self, PrimaryBtn};
use crate::ui::orchestrator::widgets::{
    BtnOpts, PillTone, redesign_box, redesign_btn_glyph, redesign_section_header, render_pill,
};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_muted, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailsOutcome {
    #[default]
    Stay,
    Back,
    ReviewInstallation,
}

const ART_W_PX: f32 = 300.0;
const COLUMN_GAP_PX: f32 = 24.0;
const FACTS_W_PX: f32 = 300.0;

const BEFORE_HEADING: &str = "Before you start";
const MAKE_IT_YOURS_HEADING: &str = "Make it your own";
const MAKE_IT_YOURS_BODY: &str = "Install the list as provided, or review and modify its component selection before installation.";

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, entry: &GalleryEntry) -> DetailsOutcome {
    let mut outcome = DetailsOutcome::Stay;

    let body_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if redesign_btn_glyph(
                    ui,
                    palette,
                    "\u{2190}",
                    " All modlists",
                    BtnOpts {
                        small: true,
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    outcome = DetailsOutcome::Back;
                }
                ui.add_space(16.0);

                header_row(ui, palette, entry);
                ui.add_space(20.0);
                body_columns(ui, palette, entry);
            });
    });

    let footer = sub_flow_footer::render(
        ui,
        palette,
        None::<sub_flow_footer::BackBtn<'_>>,
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some(&format!("{} mods in this collection", entry.mods.len())),
        PrimaryBtn {
            label: "Review Installation",
            disabled: false,
        },
    );
    if footer.primary_clicked {
        outcome = DetailsOutcome::ReviewInstallation;
    }

    outcome
}

fn header_row(ui: &mut egui::Ui, palette: ThemePalette, entry: &GalleryEntry) {
    let art_h = card_art::height_for_width(ART_W_PX);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = COLUMN_GAP_PX;

        let (art_rect, _) =
            ui.allocate_exact_size(egui::vec2(ART_W_PX, art_h), egui::Sense::hover());
        card_art::paint(ui, palette, entry.game, art_rect);

        let text_w = ui.available_width().max(200.0);
        ui.allocate_ui_with_layout(
            egui::vec2(text_w, art_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(text_w);
                ui.label(
                    egui::RichText::new(entry.name)
                        .size(24.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!(
                        "{} \u{00B7} Modlist {} \u{00B7} {}",
                        entry.author,
                        entry.version,
                        entry.game.to_legacy_string()
                    ))
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
                );
                ui.add_space(12.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(entry.description)
                            .size(14.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_muted(palette)),
                    )
                    .wrap(),
                );
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                    render_pill(ui, palette, entry.game.to_legacy_string(), PillTone::Info);
                    for tag in entry.tags {
                        render_pill(ui, palette, tag, PillTone::Neutral);
                    }
                    if entry.sample {
                        render_pill(ui, palette, "Sample", PillTone::Warn);
                    }
                });
            },
        );
    });
}

fn body_columns(ui: &mut egui::Ui, palette: ThemePalette, entry: &GalleryEntry) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = COLUMN_GAP_PX;

        let prose_w = (ui.available_width() - FACTS_W_PX - COLUMN_GAP_PX).max(240.0);
        ui.allocate_ui_with_layout(
            egui::vec2(prose_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(prose_w);
                prose_section(
                    ui,
                    palette,
                    BEFORE_HEADING,
                    &format!(
                        "You will need {}. Review the included components and choose your installation destination before continuing.",
                        entry.requirements
                    ),
                );
                ui.add_space(16.0);
                prose_section(ui, palette, MAKE_IT_YOURS_HEADING, MAKE_IT_YOURS_BODY);
            },
        );

        ui.allocate_ui_with_layout(
            egui::vec2(FACTS_W_PX, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(FACTS_W_PX);
                facts_column(ui, palette, entry);
            },
        );
    });
}

fn prose_section(ui: &mut egui::Ui, palette: ThemePalette, heading: &str, body: &str) {
    redesign_section_header(ui, palette, heading, None);
    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(body)
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        )
        .wrap(),
    );
}

fn facts_column(ui: &mut egui::Ui, palette: ThemePalette, entry: &GalleryEntry) {
    redesign_box(ui, palette, None, |ui| {
        fact_row(ui, palette, "Creator", entry.author);
        fact_row(ui, palette, "Modlist version", entry.version);
        fact_row(ui, palette, "Created with BIO", env!("CARGO_PKG_VERSION"));
        fact_row(ui, palette, "Game target", entry.game.to_legacy_string());
        fact_row(ui, palette, "Required games", entry.requirements);
    });
}

fn fact_row(ui: &mut egui::Ui, palette: ThemePalette, label: &str, value: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(2.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(value)
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        )
        .wrap(),
    );
    ui.add_space(12.0);
}
