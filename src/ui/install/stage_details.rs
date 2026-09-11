// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::app::controller::util::open_in_shell;
use crate::app::modlist_share::{ForkAncestor, ModlistSharePreview};
use crate::registry::model::Game;
use crate::ui::install::fork_info_button;
use crate::ui::install::gallery::card_art;
use crate::ui::install::gallery::catalog::{GalleryEntry, requirements_for};
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

pub(crate) struct DetailsHeader {
    pub(crate) source_compat_issue: Option<&'static str>,
    pub(crate) name: String,
    pub(crate) author: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) game: Game,
    pub(crate) description: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) sample: bool,
    pub(crate) requirements: String,
    pub(crate) built_with: Option<String>,
    pub(crate) lineage: Vec<ForkAncestor>,
    pub(crate) back_label: &'static str,
}

impl DetailsHeader {
    #[must_use]
    pub(crate) fn from_gallery_entry(entry: &GalleryEntry, preview: &ModlistSharePreview) -> Self {
        Self {
            name: entry.name.to_string(),
            author: Some(entry.author.to_string()),
            version: Some(entry.version.to_string()),
            game: entry.game,
            description: Some(entry.description.to_string()),
            tags: entry.tags.iter().map(|tag| (*tag).to_string()).collect(),
            sample: entry.sample,
            requirements: entry.requirements.to_string(),
            built_with: non_empty(&preview.bio_version),
            lineage: preview.forked_from.clone(),
            back_label: "All modlists",
            source_compat_issue: None,
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
            name: stage_review::display_name(typed_name, preview),
            author: preview
                .author
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            version: None,
            game,
            description: preview
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
            sample: false,
            requirements: requirements_for(game).to_string(),
            built_with: non_empty(&preview.bio_version),
            source_compat_issue: None,
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
                if let Some(issue) = header.source_compat_issue {
                    crate::ui::install::stage_review::render_source_warning(
                        ui,
                        palette,
                        issue,
                        availability == ModifyAvailability::OnlyInstall,
                    );
                    ui.add_space(16.0);
                }
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
        card_art::paint(ui, palette, header.game, art_rect);

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
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                    render_pill(ui, palette, header.game.to_legacy_string(), PillTone::Info);
                    for tag in &header.tags {
                        render_pill(ui, palette, tag, PillTone::Neutral);
                    }
                    if header.sample {
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
    ]
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
    fn gallery_header_carries_the_entry_and_the_codes_bio_version() {
        let entry = &crate::ui::install::gallery::catalog::entries()[1];
        let preview = eet_preview("0.1.0-test");
        let header = DetailsHeader::from_gallery_entry(entry, &preview);

        assert_eq!(header.name, "EET + Fixes");
        assert_eq!(header.author.as_deref(), Some("BIO Team"));
        assert_eq!(header.version.as_deref(), Some("1.0.0"));
        assert_eq!(header.description.as_deref(), Some(entry.description));
        assert_eq!(
            header.tags,
            entry
                .tags
                .iter()
                .map(|tag| (*tag).to_string())
                .collect::<Vec<_>>()
        );
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
        assert!(header.tags.is_empty());
        assert_eq!(header.requirements, requirements_for(Game::EET));
        assert_eq!(header.built_with, None);
        assert_eq!(header.back_label, "Back");

        let rows = fact_rows(&header);
        assert_eq!(rows[0].value, "\u{2014}");
        assert_eq!(rows[1].value, "\u{2014}");
        assert_eq!(rows[4].value, "\u{2014}");
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

        assert_eq!(
            header.description.as_deref(),
            Some("Forked from Parent, by @parent.")
        );
        assert_eq!(header.tags, vec!["Fork".to_string()]);
        assert_eq!(header.lineage.len(), 2);
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
            vec!["Author", "Version", "Game", "Requires", "Built with BIO"]
        );

        assert_eq!(rows[0].value, entry.author);
        assert_eq!(rows[1].value, entry.version);
        assert_eq!(rows[2].value, entry.game.to_legacy_string());
        assert_eq!(rows[3].value, entry.requirements);
        assert_eq!(rows[4].value, "0.1.0-test");
    }

    #[test]
    fn before_you_start_copy_is_verbatim() {
        assert_eq!(
            before_you_start_body("Baldur's Gate: Enhanced Edition"),
            "You will need Baldur's Gate: Enhanced Edition. Look through what's inside, then choose a destination when you install."
        );
    }
}
