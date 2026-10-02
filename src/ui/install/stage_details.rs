// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::app::compat_dlc_source::SourceNotice;
use crate::app::controller::util::open_in_shell;
use crate::app::modlist_share::{ForkAncestor, ModlistSharePreview};
use crate::gallery_feed::index::FeedEntry;
use crate::registry::model::Game;
use crate::ui::install::fork_info_button;
use crate::ui::install::gallery::card_art;
use crate::ui::install::gallery::catalog::requirements_for;
use crate::ui::install::stage_review::{self, ModifyAvailability};
use crate::ui::install::state_install::{DrawerKind, ReviewOrigin};
use crate::ui::install::sub_flow_footer::{self, FooterClick, LeftActionBtn, PrimaryBtn};
use crate::ui::install::whats_inside::{self, InsideCounts};
use crate::ui::orchestrator::widgets::dialogs::fork_info_popup::{self, SelfNode};
use crate::ui::orchestrator::widgets::{
    BtnOpts, PillTone, redesign_box, redesign_btn_glyph, redesign_section_header, render_pill,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_soft, redesign_text_muted,
    redesign_text_primary,
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
const MAKE_IT_YOURS_BOTH: &str = "Install the list as provided, or review and modify its component selection before installation.";
const MAKE_IT_YOURS_REINSTALL: &str =
    "Reinstall keeps this modlist as it is; to change it, open it from Home.";
const MAKE_IT_YOURS_MODIFY_ONLY: &str =
    "This share code was exported mid-install, so it can only be reviewed and modified.";
const BIO_DISCORD_URL: &str = "https://discord.gg/mJFs3639tS";
const DASH: &str = "\u{2014}";

pub(crate) struct FactRow {
    pub(crate) label: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetailsCover {
    pub(crate) entry_id: String,
    pub(crate) png: Vec<u8>,
}

pub(crate) struct DetailsHeader {
    pub(crate) cover: Option<DetailsCover>,
    pub(crate) source_compat_issue: Option<SourceNotice>,
    pub(crate) source_residue_issue: Option<SourceNotice>,
    pub(crate) unresolved_sources_issue: Option<SourceNotice>,
    pub(crate) unresolved_mod_count: usize,
    pub(crate) name: String,
    pub(crate) author: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) game: Game,
    pub(crate) game_version: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) fork_note: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) requirements: String,
    pub(crate) built_with: Option<String>,
    pub(crate) lineage: Vec<ForkAncestor>,
    pub(crate) back_label: &'static str,
}

impl DetailsHeader {
    #[must_use]
    pub(crate) fn from_gallery_entry(entry: &FeedEntry, preview: &ModlistSharePreview) -> Self {
        Self {
            cover: entry.cover_png.clone().map(|png| DetailsCover {
                entry_id: entry.id.clone(),
                png,
            }),
            name: entry.name.clone(),
            author: Some(entry.author.clone()),
            version: Some(entry.version.clone()),
            game: entry.game,
            game_version: preview.game_version.clone(),
            description: Some(entry.description.clone()),
            fork_note: None,
            tags: entry.tags.clone(),
            requirements: entry.requirements.clone(),
            built_with: non_empty(&preview.bio_version),
            lineage: preview.forked_from.clone(),
            back_label: "All modlists",
            source_compat_issue: None,
            source_residue_issue: None,
            unresolved_sources_issue: None,
            unresolved_mod_count: preview.unresolved_mods.len(),
        }
    }

    #[must_use]
    pub(crate) fn from_preview(
        preview: &ModlistSharePreview,
        typed_name: &str,
        origin: ReviewOrigin,
    ) -> Self {
        let game = Game::from_legacy_string(&preview.game_install);
        let has_lineage = !preview.forked_from.is_empty();
        Self {
            cover: None,
            name: stage_review::display_name(typed_name, preview),
            author: preview
                .author
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            version: None,
            game,
            game_version: preview.game_version.clone(),
            description: preview
                .description
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            fork_note: preview
                .forked_from
                .last()
                .filter(|parent| !parent.name.trim().is_empty())
                .map(|parent| {
                    let author = parent.author.trim();
                    if author.is_empty() {
                        format!("Forked from {}.", parent.name.trim())
                    } else {
                        format!("Forked from {}, by {}.", parent.name.trim(), author)
                    }
                }),
            tags: if has_lineage {
                vec!["Fork".to_string()]
            } else {
                Vec::new()
            },
            requirements: requirements_for(game).to_string(),
            built_with: non_empty(&preview.bio_version),
            source_compat_issue: None,
            source_residue_issue: None,
            unresolved_sources_issue: None,
            unresolved_mod_count: preview.unresolved_mods.len(),
            lineage: preview.forked_from.clone(),
            back_label: if matches!(origin, ReviewOrigin::Paste) {
                "Back"
            } else {
                "All modlists"
            },
        }
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[must_use]
pub(crate) const fn make_it_yours_body(availability: ModifyAvailability) -> &'static str {
    match availability {
        ModifyAvailability::Both => MAKE_IT_YOURS_BOTH,
        ModifyAvailability::OnlyInstall => MAKE_IT_YOURS_REINSTALL,
        ModifyAvailability::OnlyModify => MAKE_IT_YOURS_MODIFY_ONLY,
    }
}

pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    ctx: &egui::Context,
    header: &DetailsHeader,
    counts: &InsideCounts,
    availability: ModifyAvailability,
    fork_info_open: &mut bool,
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
                    &format!(" {}", header.back_label),
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

                header_row(ui, palette, header, fork_info_open);
                ui.add_space(20.0);
                if let Some(click) = body_columns(ui, palette, header, counts, availability) {
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

    render_fork_popup(ctx, palette, header, fork_info_open);

    outcome
}

fn render_fork_popup(
    ctx: &egui::Context,
    palette: ThemePalette,
    header: &DetailsHeader,
    fork_info_open: &mut bool,
) {
    if !*fork_info_open {
        return;
    }
    let result = fork_info_popup::render(
        ctx,
        palette,
        "install_details",
        &header.lineage,
        &SelfNode {
            name: &header.name,
            author: header.author.as_deref().unwrap_or(""),
        },
    );
    if result == fork_info_popup::ForkInfoOutcome::Closed {
        *fork_info_open = false;
    }
}

fn header_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    header: &DetailsHeader,
    fork_info_open: &mut bool,
) {
    let art_h = card_art::height_for_width(ART_W_PX);
    let has_lineage = !header.lineage.is_empty();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = COLUMN_GAP_PX;

        let (art_rect, _) =
            ui.allocate_exact_size(egui::vec2(ART_W_PX, art_h), egui::Sense::hover());
        if let Some(cover) = &header.cover {
            card_art::paint_cover(
                ui,
                palette,
                header.game,
                &cover.entry_id,
                &cover.png,
                art_rect,
            );
        } else {
            card_art::paint(ui, palette, header.game, art_rect);
        }

        let text_w = ui.available_width().max(200.0);
        ui.allocate_ui_with_layout(
            egui::vec2(text_w, art_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(text_w);
                ui.horizontal_top(|ui| {
                    let button_w = if has_lineage {
                        fork_info_button::WIDTH_PX
                    } else {
                        0.0
                    };
                    let title_w = (ui.available_width() - button_w).max(120.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(title_w, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.label(
                                egui::RichText::new(&header.name)
                                    .size(24.0)
                                    .family(egui::FontFamily::Name("poppins_medium".into()))
                                    .color(redesign_text_primary(palette)),
                            );
                        },
                    );
                    if has_lineage {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if fork_info_button::render(ui, palette).clicked() {
                                *fork_info_open = true;
                            }
                        });
                    }
                });
                if let Some(description) = &header.description {
                    ui.add_space(12.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(description)
                                .size(14.0)
                                .family(egui::FontFamily::Name("poppins_light".into()))
                                .color(redesign_text_muted(palette)),
                        )
                        .wrap(),
                    );
                }
                if let Some(fork_note) = &header.fork_note {
                    ui.add_space(6.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(fork_note)
                                .size(14.0)
                                .family(egui::FontFamily::Name("poppins_light".into()))
                                .color(redesign_text_muted(palette)),
                        )
                        .wrap(),
                    );
                }
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                    render_pill(ui, palette, header.game.to_legacy_string(), PillTone::Info);
                    for tag in &header.tags {
                        render_pill(ui, palette, tag, PillTone::Neutral);
                    }
                });
            },
        );
    });
}

fn body_columns(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    header: &DetailsHeader,
    counts: &InsideCounts,
    availability: ModifyAvailability,
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
                if let Some(notice) = &header.source_compat_issue {
                    stage_review::render_source_notice(
                        ui,
                        palette,
                        notice,
                        stage_review::source_warning_action(
                            availability == ModifyAvailability::OnlyInstall,
                            availability == ModifyAvailability::OnlyModify,
                        ),
                    );
                    ui.add_space(16.0);
                }
                if let Some(notice) = &header.source_residue_issue {
                    stage_review::render_source_notice(
                        ui,
                        palette,
                        notice,
                        stage_review::source_warning_action(
                            availability == ModifyAvailability::OnlyInstall,
                            availability == ModifyAvailability::OnlyModify,
                        ),
                    );
                    ui.add_space(16.0);
                }
                if let Some(notice) = &header.unresolved_sources_issue {
                    stage_review::render_source_notice(
                        ui,
                        palette,
                        notice,
                        stage_review::source_warning_action(
                            availability == ModifyAvailability::OnlyInstall,
                            availability == ModifyAvailability::OnlyModify,
                        ),
                    );
                    ui.add_space(16.0);
                }
                prose_section(
                    ui,
                    palette,
                    BEFORE_HEADING,
                    &before_you_start_body(&header.requirements),
                );
                ui.add_space(16.0);
                prose_section(
                    ui,
                    palette,
                    MAKE_IT_YOURS_HEADING,
                    make_it_yours_body(availability),
                );
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
                facts_column(ui, palette, header);
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
pub(crate) fn fact_rows(header: &DetailsHeader) -> Vec<FactRow> {
    vec![
        FactRow {
            label: "Author".to_string(),
            value: header.author.clone().unwrap_or_else(|| DASH.to_string()),
        },
        FactRow {
            label: "Version".to_string(),
            value: header.version.clone().unwrap_or_else(|| DASH.to_string()),
        },
        FactRow {
            label: "Game".to_string(),
            value: header.game.to_legacy_string().to_string(),
        },
        FactRow {
            label: "Game version".to_string(),
            value: header
                .game_version
                .clone()
                .unwrap_or_else(|| "any".to_string()),
        },
        FactRow {
            label: "Requires".to_string(),
            value: header.requirements.clone(),
        },
        FactRow {
            label: "Built with BIO".to_string(),
            value: header
                .built_with
                .clone()
                .unwrap_or_else(|| DASH.to_string()),
        },
        FactRow {
            label: "Sources".to_string(),
            value: sources_fact_value(header.unresolved_mod_count),
        },
    ]
}

#[must_use]
fn sources_fact_value(unresolved: usize) -> String {
    if unresolved == 0 {
        "all resolved".to_string()
    } else if unresolved == 1 {
        "1 without a source".to_string()
    } else {
        format!("{unresolved} without a source")
    }
}

fn facts_column(ui: &mut egui::Ui, palette: ThemePalette, header: &DetailsHeader) {
    redesign_box(ui, palette, None, |ui| {
        for row in fact_rows(header) {
            fact_label(ui, palette, &row.label);
            fact_value(ui, palette, &row.value);
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

fn fact_value(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        )
        .wrap(),
    );
    ui.add_space(6.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eet_preview(bio_version: &str) -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: bio_version.to_string(),
            game_install: "EET".to_string(),
            game_version: None,
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
            description: None,
            forked_from: Vec::new(),
            unresolved_mods: Vec::new(),
        }
    }

    #[test]
    fn gallery_header_carries_the_entry_and_the_codes_bio_version() {
        let entry = &crate::ui::install::gallery::catalog::entries()[1];
        let preview = eet_preview("0.1.0-test");
        let header = DetailsHeader::from_gallery_entry(entry, &preview);

        assert_eq!(header.name, entry.name);
        assert_eq!(header.author.as_deref(), Some(entry.author.as_str()));
        assert_eq!(header.version.as_deref(), Some(entry.version.as_str()));
        assert_eq!(
            header.description.as_deref(),
            Some(entry.description.as_str())
        );
        assert_eq!(header.tags, entry.tags);
        assert_eq!(header.built_with.as_deref(), Some("0.1.0-test"));
        assert_eq!(header.back_label, "All modlists");
    }

    #[test]
    fn code_header_uses_the_display_name_dashes_and_the_derived_requirements() {
        let mut preview = eet_preview("");
        preview.name = None;
        preview.author = None;
        preview.game_install = "EET".to_string();
        let header = DetailsHeader::from_preview(&preview, "", ReviewOrigin::Paste);

        assert_eq!(header.name, "Shared modlist");
        assert_eq!(header.author, None);
        assert_eq!(header.version, None);
        assert_eq!(header.description, None);
        assert_eq!(header.fork_note, None);
        assert!(header.tags.is_empty());
        assert_eq!(header.requirements, requirements_for(Game::EET));
        assert_eq!(header.built_with, None);
        assert_eq!(header.back_label, "Back");

        let rows = fact_rows(&header);
        assert_eq!(rows[0].value, "\u{2014}");
        assert_eq!(rows[1].value, "\u{2014}");
        assert_eq!(rows[5].value, "\u{2014}");
    }

    #[test]
    fn code_header_with_lineage_gets_the_fork_pill_and_the_parent_sentence() {
        let mut preview = eet_preview("0.2.0");
        preview.forked_from = vec![
            ForkAncestor {
                name: "Original".to_string(),
                author: "@root".to_string(),
            },
            ForkAncestor {
                name: "Parent".to_string(),
                author: "@parent".to_string(),
            },
        ];
        let header = DetailsHeader::from_preview(&preview, "", ReviewOrigin::Paste);

        assert_eq!(header.description, None);
        assert_eq!(
            header.fork_note.as_deref(),
            Some("Forked from Parent, by @parent.")
        );
        assert_eq!(header.tags, vec!["Fork".to_string()]);
        assert_eq!(header.lineage.len(), 2);
    }

    #[test]
    fn code_header_carries_a_trimmed_description_and_the_fork_note_stays_separate() {
        let mut preview = eet_preview("0.2.0");
        preview.description = Some("  BG2EE with the fixpack  ".to_string());
        preview.forked_from = vec![ForkAncestor {
            name: "Root build".to_string(),
            author: "@root".to_string(),
        }];
        let header = DetailsHeader::from_preview(&preview, "", ReviewOrigin::Paste);

        assert_eq!(
            header.description.as_deref(),
            Some("BG2EE with the fixpack")
        );
        assert_eq!(
            header.fork_note.as_deref(),
            Some("Forked from Root build, by @root.")
        );
    }

    #[test]
    fn reinstall_header_keeps_the_typed_name_and_the_all_modlists_back() {
        let preview = eet_preview("0.1.0");
        let header = DetailsHeader::from_preview(&preview, "My EET", ReviewOrigin::Reinstall);

        assert_eq!(header.name, "My EET");
        assert_eq!(header.back_label, "All modlists");
    }

    #[test]
    fn make_it_yours_body_states_what_the_drawer_offers() {
        assert_eq!(
            make_it_yours_body(ModifyAvailability::Both),
            "Install the list as provided, or review and modify its component selection before installation."
        );
        assert_eq!(
            make_it_yours_body(ModifyAvailability::OnlyInstall),
            "Reinstall keeps this modlist as it is; to change it, open it from Home."
        );
        assert_eq!(
            make_it_yours_body(ModifyAvailability::OnlyModify),
            "This share code was exported mid-install, so it can only be reviewed and modified."
        );
    }

    #[test]
    fn fact_rows_list_author_version_game_requires_and_bio_version() {
        let entry = &crate::ui::install::gallery::catalog::entries()[0];
        let preview = eet_preview("0.1.0-test");
        let header = DetailsHeader::from_gallery_entry(entry, &preview);
        let rows = fact_rows(&header);

        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "Author",
                "Version",
                "Game",
                "Game version",
                "Requires",
                "Built with BIO",
                "Sources"
            ]
        );

        assert_eq!(rows[0].value, entry.author);
        assert_eq!(rows[1].value, entry.version);
        assert_eq!(rows[2].value, entry.game.to_legacy_string());
        assert_eq!(rows[3].value, "any");
        assert_eq!(rows[4].value, entry.requirements);
        assert_eq!(rows[5].value, "0.1.0-test");
        assert_eq!(rows[6].value, "all resolved");
    }

    #[test]
    fn fact_rows_show_the_game_version_or_any() {
        let entry = &crate::ui::install::gallery::catalog::entries()[0];
        let mut preview = eet_preview("0.1.0-test");
        preview.game_version = Some("2.6".to_string());
        let header = DetailsHeader::from_gallery_entry(entry, &preview);
        let rows = fact_rows(&header);
        assert_eq!(rows[2].label, "Game");
        assert_eq!(rows[3].label, "Game version");
        assert_eq!(rows[3].value, "2.6");

        preview.game_version = None;
        let header = DetailsHeader::from_gallery_entry(entry, &preview);
        let rows = fact_rows(&header);
        assert_eq!(rows[3].value, "any");
    }

    #[test]
    fn sources_fact_value_reports_a_partial_count_or_all_resolved() {
        assert_eq!(sources_fact_value(0), "all resolved");
        assert_eq!(sources_fact_value(1), "1 without a source");
        assert_eq!(sources_fact_value(2), "2 without a source");
    }

    fn cover_png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::new(width, height);
        let mut buffer = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, image::ImageFormat::Png)
            .expect("encode png");
        buffer.into_inner()
    }

    fn entry_with_cover(cover: Option<Vec<u8>>) -> FeedEntry {
        let mut entry = crate::ui::install::gallery::catalog::entries()[0].clone();
        entry.id = "details-cover-under-test".to_string();
        entry.cover_png = cover;
        entry
    }

    fn cover_cached_after_one_frame(header: &DetailsHeader, cache_id: egui::Id) -> bool {
        let ctx = egui::Context::default();
        crate::ui::shared::redesign_fonts::install_redesign_fonts(&ctx);
        let mut fork_info_open = false;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                header_row(ui, ThemePalette::Dark, header, &mut fork_info_open);
            });
        });
        ctx.memory(|m| m.data.get_temp::<egui::TextureHandle>(cache_id).is_some())
    }

    #[test]
    fn gallery_header_carries_the_cover() {
        let bytes = cover_png(4, 2);
        let entry = entry_with_cover(Some(bytes.clone()));
        let preview = eet_preview("0.1.0-test");
        let header = DetailsHeader::from_gallery_entry(&entry, &preview);
        assert_eq!(
            header.cover,
            Some(DetailsCover {
                entry_id: entry.id,
                png: bytes,
            })
        );

        let bare = entry_with_cover(None);
        let header = DetailsHeader::from_gallery_entry(&bare, &preview);
        assert!(header.cover.is_none());
    }

    #[test]
    fn code_header_has_no_cover() {
        let preview = eet_preview("0.1.0-test");
        let header = DetailsHeader::from_preview(&preview, "", ReviewOrigin::Paste);
        assert!(header.cover.is_none());
    }

    #[test]
    fn header_row_fills_the_shared_cover_cache() {
        let png = cover_png(460, 215);
        let entry = entry_with_cover(Some(png.clone()));
        let preview = eet_preview("0.1.0-test");
        let cache_id = egui::Id::new(("gallery_cover", entry.id.as_str(), png.len()));

        let header = DetailsHeader::from_gallery_entry(&entry, &preview);
        assert!(
            cover_cached_after_one_frame(&header, cache_id),
            "the Details frame must paint through the card's cover cache"
        );

        let mut bare = DetailsHeader::from_gallery_entry(&entry, &preview);
        bare.cover = None;
        assert!(
            !cover_cached_after_one_frame(&bare, cache_id),
            "a header without a cover must keep the placeholder"
        );
    }

    #[test]
    fn before_you_start_copy_is_verbatim() {
        assert_eq!(
            before_you_start_body("Baldur's Gate: Enhanced Edition"),
            "You will need Baldur's Gate: Enhanced Edition. Look through what's inside, then choose a destination when you install."
        );
    }
}
