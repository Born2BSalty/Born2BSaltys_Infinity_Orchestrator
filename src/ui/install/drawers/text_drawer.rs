// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::inside_model;
use crate::ui::install::stage_review;
use crate::ui::install::state_install::InstallScreenState;
use crate::ui::install::whats_inside::{self, InsideCounts};
use crate::ui::orchestrator::widgets::clipboard;
use crate::ui::orchestrator::widgets::drawer::{self, DrawerSpec, DrawerWidth};
use crate::ui::orchestrator::widgets::tab_strip::{self, TabItem};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_soft,
    redesign_input_bg, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextDrawer {
    WeiduLogs,
    InstalledRefs,
    DownloadSources,
    ConfigFiles,
}

#[must_use]
pub(crate) const fn title(kind: TextDrawer) -> &'static str {
    match kind {
        TextDrawer::WeiduLogs => "WeiDU logs",
        TextDrawer::InstalledRefs => "Installed refs / pins",
        TextDrawer::DownloadSources => "Download sources",
        TextDrawer::ConfigFiles => "Mod config files",
    }
}

#[must_use]
pub(crate) fn footer_text(kind: TextDrawer, tab_count: usize) -> String {
    match kind {
        TextDrawer::WeiduLogs => {
            let subject = if tab_count > 1 {
                "these logs"
            } else {
                "this log"
            };
            format!("Read-only. BIO writes {subject} into the destination before installing.")
        }
        TextDrawer::InstalledRefs => {
            "Version pins and the source each mod resolved from.".to_string()
        }
        TextDrawer::DownloadSources => "Where each mod is fetched from. Overrides your global download sources for this modlist only.".to_string(),
        TextDrawer::ConfigFiles => {
            "Configuration files stored with the modlist and applied before install.".to_string()
        }
    }
}

#[must_use]
pub(crate) fn copy_label(kind: TextDrawer, game: &str) -> String {
    match kind {
        TextDrawer::WeiduLogs => format!("Copy {game} log"),
        TextDrawer::InstalledRefs | TextDrawer::DownloadSources | TextDrawer::ConfigFiles => {
            "Copy".to_string()
        }
    }
}

#[must_use]
pub(crate) fn copy_toast(kind: TextDrawer, game: &str) -> String {
    match kind {
        TextDrawer::WeiduLogs => format!("Copied the {game} WeiDU log"),
        TextDrawer::InstalledRefs | TextDrawer::DownloadSources | TextDrawer::ConfigFiles => {
            "Copied to clipboard".to_string()
        }
    }
}

fn text_sections(kind: TextDrawer, preview: &ModlistSharePreview) -> Vec<(String, String)> {
    match kind {
        TextDrawer::WeiduLogs => inside_model::section_texts(preview)
            .into_iter()
            .map(|(game, text)| (game, text.to_string()))
            .collect(),
        TextDrawer::InstalledRefs => vec![(String::new(), preview.installed_refs_text.clone())],
        TextDrawer::DownloadSources => vec![(String::new(), preview.source_overrides_text.clone())],
        TextDrawer::ConfigFiles => vec![(String::new(), preview.mod_configs_text.clone())],
    }
}

fn entries_for(counts: &InsideCounts, game: &str) -> usize {
    counts
        .per_game
        .iter()
        .find(|(g, _)| g == game)
        .map_or(0, |(_, n)| *n)
}

fn subtitle_for(
    kind: TextDrawer,
    name: &str,
    sections: &[(String, String)],
    counts: &InsideCounts,
) -> String {
    match kind {
        TextDrawer::WeiduLogs => {
            let parts: Vec<String> = sections
                .iter()
                .map(|(game, _)| {
                    format!(
                        "{game} {}",
                        whats_inside::plural(entries_for(counts, game), "entry")
                    )
                })
                .collect();
            format!(
                "{name} \u{00B7} {}, in install order",
                parts.join(" \u{00B7} ")
            )
        }
        TextDrawer::InstalledRefs | TextDrawer::DownloadSources | TextDrawer::ConfigFiles => {
            name.to_string()
        }
    }
}

fn body_frame(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    let frame = egui::Frame::default()
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_soft(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(12));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        if text.trim().is_empty() {
            ui.label(
                egui::RichText::new("(none in this share code)")
                    .size(12.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_faint(palette)),
            );
        } else {
            ui.label(
                egui::RichText::new(text)
                    .size(12.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_primary(palette)),
            );
        }
    });
}

pub(crate) fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    kind: TextDrawer,
    state: &mut InstallScreenState,
    preview: &ModlistSharePreview,
    counts: &InsideCounts,
) -> bool {
    let name = stage_review::display_name(&state.review.name, preview);
    let sections = text_sections(kind, preview);
    let subtitle = subtitle_for(kind, &name, &sections, counts);

    let initial_tab = if state.drawer.logs_tab < sections.len() {
        state.drawer.logs_tab
    } else {
        0
    };
    let active_tab_cell = std::cell::Cell::new(initial_tab);

    let spec = DrawerSpec {
        id_salt: "text_drawer",
        title: title(kind),
        subtitle: &subtitle,
        width: DrawerWidth::Content,
    };

    let mut copy_requested = false;
    let response = drawer::render(
        ctx,
        palette,
        &spec,
        |ui| {
            let subs: Vec<String> = sections
                .iter()
                .map(|(game, _)| whats_inside::plural(entries_for(counts, game), "entry"))
                .collect();
            let mut active_tab_rect = None;
            if sections.len() > 1 {
                let tab_items: Vec<TabItem<'_>> = sections
                    .iter()
                    .zip(subs.iter())
                    .map(|((game, _), sub)| TabItem {
                        label: game.as_str(),
                        sub: Some(sub.as_str()),
                    })
                    .collect();
                let mut current = active_tab_cell.get();
                active_tab_rect =
                    tab_strip::render_tab_strip(ui, palette, &tab_items, &mut current);
                active_tab_cell.set(current);
                let item_gap = ui.spacing().item_spacing.y;
                ui.add_space(-item_gap);
            }
            let panel_top_y = ui.cursor().top();
            let text = sections
                .get(active_tab_cell.get())
                .map_or("", |(_, t)| t.as_str());
            body_frame(ui, palette, text);
            if let Some(rect) = active_tab_rect {
                tab_strip::paint_tab_seam_cover(ui.painter(), palette, rect, panel_top_y);
            }
        },
        |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let game = sections
                    .get(active_tab_cell.get())
                    .map_or("", |(g, _)| g.as_str());
                if redesign_btn(
                    ui,
                    palette,
                    &copy_label(kind, game),
                    BtnOpts {
                        primary: true,
                        small: true,
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    copy_requested = true;
                }
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(footer_text(kind, sections.len()))
                                .size(12.0)
                                .family(egui::FontFamily::Name("poppins_light".into()))
                                .color(redesign_text_muted(palette)),
                        )
                        .wrap(),
                    );
                });
            });
        },
    );

    if sections.len() > 1 {
        state.drawer.logs_tab = active_tab_cell.get();
    }

    if copy_requested {
        let (game, text) = sections
            .get(active_tab_cell.get())
            .cloned()
            .unwrap_or_else(|| (String::new(), String::new()));
        clipboard::copy_with_message(ctx, text, copy_toast(kind, &game));
    }

    response.close_requested
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_footers_are_verbatim() {
        assert_eq!(title(TextDrawer::WeiduLogs), "WeiDU logs");
        assert_eq!(title(TextDrawer::InstalledRefs), "Installed refs / pins");
        assert_eq!(title(TextDrawer::DownloadSources), "Download sources");
        assert_eq!(title(TextDrawer::ConfigFiles), "Mod config files");

        assert_eq!(
            footer_text(TextDrawer::WeiduLogs, 1),
            "Read-only. BIO writes this log into the destination before installing."
        );
        assert_eq!(
            footer_text(TextDrawer::WeiduLogs, 2),
            "Read-only. BIO writes these logs into the destination before installing."
        );
        assert_eq!(
            footer_text(TextDrawer::InstalledRefs, 1),
            "Version pins and the source each mod resolved from."
        );
        assert_eq!(
            footer_text(TextDrawer::DownloadSources, 1),
            "Where each mod is fetched from. Overrides your global download sources for this modlist only."
        );
        assert_eq!(
            footer_text(TextDrawer::ConfigFiles, 1),
            "Configuration files stored with the modlist and applied before install."
        );
    }

    #[test]
    fn logs_copy_label_and_toast_name_the_game() {
        assert_eq!(copy_label(TextDrawer::WeiduLogs, "BGEE"), "Copy BGEE log");
        assert_eq!(
            copy_toast(TextDrawer::WeiduLogs, "BGEE"),
            "Copied the BGEE WeiDU log"
        );
        assert_eq!(copy_label(TextDrawer::InstalledRefs, ""), "Copy");
        assert_eq!(
            copy_toast(TextDrawer::ConfigFiles, ""),
            "Copied to clipboard"
        );
    }
}
