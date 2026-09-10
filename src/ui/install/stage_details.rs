// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::app::controller::util::open_in_shell;
use crate::app::modlist_share::ModlistSharePreview;
use crate::app::state::Step1State;
use crate::ui::install::gallery::card_art;
use crate::ui::install::gallery::catalog::GalleryEntry;
use crate::ui::install::state_install::DrawerKind;
use crate::ui::install::sub_flow_footer::{self, FooterClick, LeftActionBtn, PrimaryBtn};
use crate::ui::install::whats_inside::{self, InsideCounts};
use crate::ui::orchestrator::widgets::{
    BtnOpts, PillTone, redesign_box, redesign_btn_glyph, redesign_section_header, render_pill,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_soft, redesign_success,
    redesign_text_muted, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DetailsOutcome {
    Stay,
    Back,
    OpenDrawer(DrawerKind),
}

const ART_W_PX: f32 = 300.0;
const COLUMN_GAP_PX: f32 = 24.0;
const FACTS_W_PX: f32 = 300.0;

const BEFORE_HEADING: &str = "Before you start";
const MAKE_IT_YOURS_HEADING: &str = "Make it your own";
const MAKE_IT_YOURS_BODY: &str = "Install the list as provided, or review and modify its component selection before installation.";
const BIO_DISCORD_URL: &str = "https://discord.gg/mJFs3639tS";

pub(crate) struct FactRow {
    pub(crate) label: String,
    pub(crate) lines: Vec<FactLine>,
}

pub(crate) struct FactLine {
    pub(crate) left: String,
    pub(crate) right: Option<String>,
    pub(crate) tone: FactTone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FactTone {
    Plain,
    Ok,
    Muted,
}

pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &GalleryEntry,
    preview: &ModlistSharePreview,
    counts: &InsideCounts,
) -> DetailsOutcome {
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
                if let Some(click) = body_columns(ui, palette, entry, counts, preview) {
                    outcome = click;
                }
            });
    });

    let hint = format!(
        "{} \u{00B7} {}",
        whats_inside::plural(counts.mods, "mod"),
        whats_inside::plural(counts.components, "component")
    );
    let footer = sub_flow_footer::render(
        ui,
        palette,
        None::<sub_flow_footer::BackBtn<'_>>,
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some(&hint),
        Some(LeftActionBtn {
            label: "BIO Discord",
        }),
        PrimaryBtn {
            label: "Install",
            disabled: false,
        },
    );
    match footer {
        FooterClick::LeftAction => {
            if let Err(err) = open_in_shell(BIO_DISCORD_URL) {
                warn!(
                    target = "orchestrator",
                    "Details: could not open the BIO Discord link: {err}"
                );
            }
        }
        FooterClick::Primary => outcome = DetailsOutcome::OpenDrawer(DrawerKind::Install),
        FooterClick::None | FooterClick::Back | FooterClick::Secondary => {}
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

fn body_columns(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &GalleryEntry,
    counts: &InsideCounts,
    preview: &ModlistSharePreview,
) -> Option<DetailsOutcome> {
    let mut result = None;
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
                    &before_you_start_body(entry.requirements),
                );
                ui.add_space(16.0);
                prose_section(ui, palette, MAKE_IT_YOURS_HEADING, MAKE_IT_YOURS_BODY);
                ui.add_space(26.0);
                divider(ui, palette);
                ui.add_space(18.0);
                let click = whats_inside::render(ui, palette, counts);
                if let Some(kind) = DrawerKind::from_click(click) {
                    result = Some(DetailsOutcome::OpenDrawer(kind));
                }
            },
        );

        ui.allocate_ui_with_layout(
            egui::vec2(FACTS_W_PX, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(FACTS_W_PX);
                facts_column(ui, palette, entry, counts, preview);
            },
        );
    });
    result
}

fn divider(ui: &mut egui::Ui, palette: ThemePalette) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), REDESIGN_BORDER_WIDTH_PX),
        egui::Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, 0.0, redesign_border_soft(palette));
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

#[must_use]
fn before_you_start_body(requirements: &str) -> String {
    format!(
        "You will need {requirements}. Look through what's inside, then choose a destination when you install."
    )
}

#[must_use]
pub(crate) fn fact_rows(
    entry: &GalleryEntry,
    counts: &InsideCounts,
    preview: &ModlistSharePreview,
) -> Vec<FactRow> {
    let mut contents_lines = vec![FactLine {
        left: format!(
            "{}, {}",
            whats_inside::plural(counts.mods, "mod"),
            whats_inside::plural(counts.components, "component")
        ),
        right: None,
        tone: FactTone::Plain,
    }];
    for (game, n) in &counts.per_game {
        contents_lines.push(FactLine {
            left: format!("{game} log"),
            right: Some(whats_inside::plural(*n, "entry")),
            tone: FactTone::Plain,
        });
    }

    let extras_lines = vec![
        extra_line("Installed refs", counts.refs, None),
        extra_line("Download sources", counts.sources, None),
        extra_line(
            "Config files",
            counts.configs > 0,
            Some(counts.configs.to_string()),
        ),
    ];

    vec![
        FactRow {
            label: "Requires".to_string(),
            lines: vec![FactLine {
                left: entry.requirements.to_string(),
                right: None,
                tone: FactTone::Plain,
            }],
        },
        FactRow {
            label: "Contents".to_string(),
            lines: contents_lines,
        },
        FactRow {
            label: "Extras".to_string(),
            lines: extras_lines,
        },
        FactRow {
            label: "Install mode".to_string(),
            lines: vec![FactLine {
                left: Step1State::install_mode_label(&preview.install_mode).to_string(),
                right: None,
                tone: FactTone::Plain,
            }],
        },
        FactRow {
            label: "Built with BIO".to_string(),
            lines: vec![FactLine {
                left: preview.bio_version.clone(),
                right: None,
                tone: FactTone::Plain,
            }],
        },
    ]
}

fn extra_line(label: &str, present: bool, count_text: Option<String>) -> FactLine {
    let (right, tone) = if present {
        (
            count_text.unwrap_or_else(|| "\u{2713} included".to_string()),
            FactTone::Ok,
        )
    } else {
        ("none".to_string(), FactTone::Muted)
    };
    FactLine {
        left: label.to_string(),
        right: Some(right),
        tone,
    }
}

fn facts_column(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &GalleryEntry,
    counts: &InsideCounts,
    preview: &ModlistSharePreview,
) {
    redesign_box(ui, palette, None, |ui| {
        for row in fact_rows(entry, counts, preview) {
            fact_label(ui, palette, &row.label);
            for line in &row.lines {
                fact_line(ui, palette, line);
            }
            ui.add_space(10.0);
        }
    });
}

fn fact_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(2.0);
}

fn fact_line(ui: &mut egui::Ui, palette: ThemePalette, line: &FactLine) {
    let Some(right) = &line.right else {
        ui.add(
            egui::Label::new(
                egui::RichText::new(&line.left)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            )
            .wrap(),
        );
        ui.add_space(6.0);
        return;
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(&line.left)
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| match line.tone {
                FactTone::Ok => {
                    ui.label(
                        egui::RichText::new(right)
                            .size(12.0)
                            .family(egui::FontFamily::Name("firacode_nerd".into()))
                            .color(redesign_success(palette)),
                    );
                }
                FactTone::Muted => {
                    ui.label(
                        egui::RichText::new(right)
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_muted(palette)),
                    );
                }
                FactTone::Plain => {
                    ui.label(
                        egui::RichText::new(right)
                            .size(13.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(redesign_text_primary(palette)),
                    );
                }
            },
        );
    });
    ui.add_space(6.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> &'static GalleryEntry {
        &crate::ui::install::gallery::catalog::entries()[0]
    }

    fn eet_preview(refs: bool, sources: bool, configs: usize) -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "0.1.0-test".to_string(),
            game_install: "EET".to_string(),
            install_mode: "build_from_scanned_mods".to_string(),
            bgee_entries: 3,
            bg2ee_entries: 4,
            has_source_overrides: sources,
            has_installed_refs: refs,
            bgee_log_text: "~A/A.TP2~ #0 #0 // A".to_string(),
            bg2ee_log_text: "~B/B.TP2~ #0 #0 // B".to_string(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: configs,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            forked_from: Vec::new(),
        }
    }

    #[test]
    fn fact_rows_list_requires_contents_extras_install_mode_and_bio_version() {
        let preview = eet_preview(true, false, 2);
        let counts = InsideCounts::from_preview(&preview);
        let e = entry();
        let rows = fact_rows(e, &counts, &preview);

        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "Requires",
                "Contents",
                "Extras",
                "Install mode",
                "Built with BIO"
            ]
        );

        let contents = &rows[1];
        assert_eq!(contents.lines[0].left, "2 mods, 7 components");
        assert_eq!(contents.lines[1].left, "BGEE log");
        assert_eq!(contents.lines[1].right, Some("3 entries".to_string()));
        assert_eq!(contents.lines[2].left, "BG2EE log");
        assert_eq!(contents.lines[2].right, Some("4 entries".to_string()));

        let extras = &rows[2];
        assert_eq!(extras.lines[0].right, Some("\u{2713} included".to_string()));
        assert_eq!(extras.lines[0].tone, FactTone::Ok);
        assert_eq!(extras.lines[1].right, Some("none".to_string()));
        assert_eq!(extras.lines[1].tone, FactTone::Muted);
        assert_eq!(extras.lines[2].right, Some("2".to_string()));
        assert_eq!(extras.lines[2].tone, FactTone::Ok);

        assert_eq!(rows[3].lines[0].left, "Build from scanned mods");
        assert_eq!(rows[4].lines[0].left, "0.1.0-test");
    }

    #[test]
    fn fact_rows_say_none_for_absent_extras() {
        let preview = eet_preview(false, false, 0);
        let counts = InsideCounts::from_preview(&preview);
        let e = entry();
        let rows = fact_rows(e, &counts, &preview);
        let extras = &rows[2];
        for line in &extras.lines {
            assert_eq!(line.right, Some("none".to_string()));
            assert_eq!(line.tone, FactTone::Muted);
        }
    }

    #[test]
    fn fact_rows_pluralise_a_single_mod_component_and_entry() {
        let preview = ModlistSharePreview {
            game_install: "IWDEE".to_string(),
            bgee_entries: 1,
            bg2ee_entries: 0,
            bgee_log_text: "~A/A.TP2~ #0 #0 // A".to_string(),
            bg2ee_log_text: String::new(),
            ..eet_preview(false, false, 0)
        };
        let counts = InsideCounts::from_preview(&preview);
        let rows = fact_rows(entry(), &counts, &preview);
        let contents = &rows[1];
        assert_eq!(contents.lines[0].left, "1 mod, 1 component");
        assert_eq!(contents.lines.len(), 2);
        assert_eq!(contents.lines[1].left, "IWDEE log");
        assert_eq!(contents.lines[1].right, Some("1 entry".to_string()));
    }

    #[test]
    fn before_you_start_copy_is_verbatim() {
        assert_eq!(
            before_you_start_body("Baldur's Gate: Enhanced Edition"),
            "You will need Baldur's Gate: Enhanced Edition. Look through what's inside, then choose a destination when you install."
        );
    }
}
