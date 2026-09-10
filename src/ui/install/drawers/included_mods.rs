// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::app::controller::util::open_in_shell;
use crate::app::mod_downloads::SourceTier;
use crate::ui::install::highlight::highlight_label;
use crate::ui::install::inside_model::{
    ComponentRow, GameSection, InsideModel, ModGroup, ResolvedSource, filter_section, match_count,
};
use crate::ui::install::state_install::DrawerState;
use crate::ui::install::sub_flow_footer::{GlyphSide, glyph_btn};
use crate::ui::orchestrator::widgets::drawer::{self, DrawerSpec, DrawerWidth};
use crate::ui::orchestrator::widgets::input::{InputOpts, redesign_text_input};
use crate::ui::orchestrator::widgets::tab_strip::{self, TabItem};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_accent_hover, redesign_accent_numbers, redesign_border_soft, redesign_chrome_bg,
    redesign_input_bg, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary, redesign_with_alpha,
};

const SEARCH_HINT: &str = "Search by mod, tp2, or component\u{2026}";
const NO_SOURCE_TITLE: &str = "No download source";
const NO_SOURCE_BODY: &str = "BIO could not resolve a source for this mod from the modlist, your download sources, or the BIO defaults. It is treated as user-provided: place it in your mods folder before installing.";
const NO_MATCHES_ANYWHERE: &str = "No mods or components match.";
const NO_COMPONENTS_AT_REST: &str = "(none in this share code)";

pub(crate) enum IncludedModsOutcome {
    Stay,
    Close,
    Install,
}

pub(crate) fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    drawer: &mut DrawerState,
    name: &str,
    inside: &InsideModel,
    offer_install: bool,
) -> IncludedModsOutcome {
    let max_tab = inside.sections.len().saturating_sub(1);
    if drawer.mods_tab > max_tab {
        drawer.mods_tab = max_tab;
    }

    let subtitle = format!(
        "{name} \u{00B7} {} mods, {} components, in install order",
        inside.mod_count, inside.component_count
    );

    let active_game = inside
        .sections
        .get(drawer.mods_tab)
        .map(|section| section.game.clone())
        .unwrap_or_default();
    let footer_line = footer_text(&active_game);

    let spec = DrawerSpec {
        id_salt: "included_mods",
        title: "Included mods",
        subtitle: &subtitle,
        width: DrawerWidth::Content,
    };

    let response = drawer::render(
        ctx,
        palette,
        &spec,
        |ui| render_body(ui, palette, drawer, inside),
        |ui| render_footer(ui, palette, &footer_line, offer_install),
    );

    if response.footer {
        IncludedModsOutcome::Install
    } else if response.close_requested {
        IncludedModsOutcome::Close
    } else {
        IncludedModsOutcome::Stay
    }
}

fn render_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    drawer: &mut DrawerState,
    inside: &InsideModel,
) {
    let query_lower = drawer.mods_query.to_ascii_lowercase();
    let query_active = !query_lower.is_empty();

    render_toolbar(ui, palette, drawer, inside, &query_lower, query_active);
    ui.add_space(12.0);

    if inside.sections.len() > 1 {
        let tab_items: Vec<(String, String)> = inside
            .sections
            .iter()
            .map(|section| {
                let total = section_component_count(section);
                let hits = section_match_count(section, &query_lower);
                (section.game.clone(), tab_sub(total, hits, query_active))
            })
            .collect();
        let tabs: Vec<TabItem<'_>> = tab_items
            .iter()
            .map(|(label, sub)| TabItem {
                label,
                sub: Some(sub.as_str()),
            })
            .collect();

        let active_tab_rect = tab_strip::render_tab_strip(ui, palette, &tabs, &mut drawer.mods_tab);
        let item_gap = ui.spacing().item_spacing.y;
        ui.add_space(-item_gap);
        let panel_top_y = ui.cursor().top();

        render_list_frame(ui, palette, drawer, inside, &query_lower);

        if let Some(tab_rect) = active_tab_rect {
            tab_strip::paint_tab_seam_cover(ui.painter(), palette, tab_rect, panel_top_y);
        }
    } else {
        render_list_frame(ui, palette, drawer, inside, &query_lower);
    }
}

fn render_toolbar(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    drawer: &mut DrawerState,
    inside: &InsideModel,
    query_lower: &str,
    query_active: bool,
) {
    let hits: usize = inside
        .sections
        .iter()
        .map(|section| section_match_count(section, query_lower))
        .sum();
    let count_text = count_label(inside.component_count, hits, query_active);
    let count_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()));
    let count_color = redesign_text_muted(palette);

    ui.horizontal(|ui| {
        let count_galley =
            ui.painter()
                .layout_no_wrap(count_text.clone(), count_font.clone(), count_color);
        let count_w = count_galley.size().x;

        let margin = egui::Margin {
            left: 12,
            right: 12,
            top: 8,
            bottom: 8,
        };
        let field_font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()));
        let box_h = ui.fonts(|f| f.row_height(&field_font))
            + f32::from(margin.top)
            + f32::from(margin.bottom);
        let width = (ui.available_width() - count_w - 12.0).max(120.0);

        redesign_text_input(
            ui,
            palette,
            InputOpts {
                edit: egui::TextEdit::singleline(&mut drawer.mods_query)
                    .font(field_font)
                    .hint_text(
                        egui::RichText::new(SEARCH_HINT)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_faint(palette)),
                    )
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .vertical_align(egui::Align::Center)
                    .margin(margin),
                margin,
                size: egui::vec2(width, box_h),
                border: None,
            },
        );

        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(count_text)
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(count_color),
        );
    });
}

fn render_list_frame(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    drawer: &mut DrawerState,
    inside: &InsideModel,
    query_lower: &str,
) {
    let active_index = drawer.mods_tab.min(inside.sections.len().saturating_sub(1));

    egui::Frame::default()
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_soft(palette),
        ))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let Some(section) = inside.sections.get(active_index) else {
                return;
            };
            let filtered = filter_section(section, query_lower);
            if filtered.is_empty() {
                render_empty(ui, palette, drawer, inside, active_index, query_lower);
                return;
            }
            for (group_index, rows) in filtered {
                render_group(
                    ui,
                    palette,
                    group_index,
                    &section.mods[group_index],
                    &rows,
                    query_lower,
                );
            }
        });
}

fn render_empty(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    drawer: &mut DrawerState,
    inside: &InsideModel,
    active_index: usize,
    query_lower: &str,
) {
    if query_lower.is_empty() {
        muted_label(ui, palette, NO_COMPONENTS_AT_REST);
        return;
    }
    let others = other_sections_with_hits(inside, active_index, query_lower);
    let current = inside
        .sections
        .get(active_index)
        .map_or("", |section| section.game.as_str());
    let message = empty_message(current, &others);

    if message.links.is_empty() {
        muted_label(ui, palette, &message.lead);
        return;
    }

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        muted_label(ui, palette, &format!("{} ", message.lead));
        for (index, (hint, game)) in message.links.iter().enumerate() {
            if index > 0 {
                muted_label(ui, palette, ", ");
            }
            if game_link(ui, palette, hint).clicked()
                && let Some(target) = inside
                    .sections
                    .iter()
                    .position(|section| &section.game == game)
            {
                drawer.mods_tab = target;
            }
        }
        muted_label(ui, palette, ".");
    });
}

fn render_group(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    group_index: usize,
    group: &ModGroup,
    rows: &[usize],
    query_lower: &str,
) {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_soft(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            render_group_head(ui, palette, group_index, group, query_lower);
            for (row_position, &row_index) in rows.iter().enumerate() {
                render_component_row(
                    ui,
                    palette,
                    &group.components[row_index],
                    query_lower,
                    row_position == 0,
                );
            }
        });
    ui.add_space(8.0);
}

fn render_group_head(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    group_index: usize,
    group: &ModGroup,
    query_lower: &str,
) {
    egui::Frame::default()
        .fill(redesign_chrome_bg(palette))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let order_font =
                    egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into()));
                ui.add_sized(
                    egui::vec2(26.0, ui.fonts(|f| f.row_height(&order_font))),
                    egui::Label::new(
                        egui::RichText::new(format!("{:02}", group_index + 1))
                            .font(order_font)
                            .color(redesign_accent(palette)),
                    ),
                );
                highlight_label(
                    ui,
                    &group.folder,
                    query_lower,
                    egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
                    redesign_text_primary(palette),
                    palette,
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    render_source_button(ui, palette, &group.source, &group.folder);
                    ui.add_space(8.0);
                    render_version_tp_line(ui, palette, group, query_lower);
                });
            });
        });
}

fn render_version_tp_line(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    group: &ModGroup,
    query_lower: &str,
) {
    ui.horizontal(|ui| {
        if !group.version.is_empty() {
            ui.label(
                egui::RichText::new(&group.version)
                    .size(11.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_muted(palette)),
            );
            ui.label(
                egui::RichText::new("\u{00B7}")
                    .size(11.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_muted(palette)),
            );
        }
        highlight_label(
            ui,
            &group.tp_file,
            query_lower,
            egui::FontId::new(11.0, egui::FontFamily::Name("firacode_nerd".into())),
            redesign_text_muted(palette),
            palette,
        );
    });
}

fn render_source_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    source: &ResolvedSource,
    folder: &str,
) {
    match source {
        ResolvedSource::Link { url, label, tier } => {
            let response = glyph_square_button(
                ui,
                palette,
                "\u{2197}",
                "firacode_nerd",
                redesign_text_primary(palette),
            )
            .on_hover_text(source_tooltip(label, *tier));
            if response.clicked()
                && let Err(err) = open_in_shell(url)
            {
                warn!(
                    target = "orchestrator",
                    "Included mods: could not open the download source for {folder}: {err}"
                );
            }
        }
        ResolvedSource::None => {
            let popup_id = ui.make_persistent_id(("included_mods_no_source", folder));
            let response = glyph_square_button(
                ui,
                palette,
                "?",
                "poppins_light",
                redesign_with_alpha(redesign_text_primary(palette), 4, 10),
            );
            if response.clicked() {
                ui.memory_mut(|memory| memory.toggle_popup(popup_id));
            }
            egui::popup_below_widget(
                ui,
                popup_id,
                &response,
                egui::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.set_max_width(280.0);
                    ui.label(
                        egui::RichText::new(NO_SOURCE_TITLE)
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(redesign_text_primary(palette)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(NO_SOURCE_BODY)
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_muted(palette)),
                    );
                },
            );
        }
    }
}

fn glyph_square_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    glyph: &str,
    family: &str,
    color: egui::Color32,
) -> egui::Response {
    let size = egui::vec2(22.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::new(12.0, egui::FontFamily::Name(family.into())),
            color,
        );
    }

    response
}

fn render_component_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    row: &ComponentRow,
    query_lower: &str,
    is_first: bool,
) {
    if !is_first {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), REDESIGN_BORDER_WIDTH_PX),
            egui::Sense::hover(),
        );
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            ],
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette)),
        );
    }

    egui::Frame::default()
        .inner_margin(egui::Margin {
            left: 38,
            right: 12,
            top: 6,
            bottom: 6,
        })
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    highlight_label(
                        ui,
                        &format!("#{}", row.id),
                        query_lower,
                        egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into())),
                        redesign_accent_numbers(palette),
                        palette,
                    );
                    highlight_label(
                        ui,
                        &row.label,
                        query_lower,
                        egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into())),
                        redesign_text_primary(palette),
                        palette,
                    );
                });
                if let Some(inputs) = &row.wlb_inputs {
                    ui.label(
                        egui::RichText::new(format!("@wlb-inputs: {inputs}"))
                            .size(11.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_faint(palette)),
                    );
                }
            });
        });
}

fn render_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    footer_line: &str,
    offer_install: bool,
) -> bool {
    let button_w = if offer_install {
        install_button_width(ui)
    } else {
        0.0
    };
    let label_w = (ui.available_width() - button_w - 12.0).max(120.0);
    ui.allocate_ui_with_layout(
        egui::vec2(label_w, ui.available_height()),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(label_w);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(footer_line)
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_muted(palette)),
                )
                .wrap(),
            );
        },
    );

    let mut install_clicked = false;
    if offer_install {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if glyph_btn(
                ui,
                palette,
                GlyphSide::Trailing("\u{2192}"),
                "Install",
                true,
                false,
            )
            .clicked()
            {
                install_clicked = true;
            }
        });
    }
    install_clicked
}

fn install_button_width(ui: &egui::Ui) -> f32 {
    let glyph_font = egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let glyph_w = ui
        .painter()
        .layout_no_wrap("\u{2192}".to_string(), glyph_font, egui::Color32::WHITE)
        .size()
        .x;
    let prose_w = ui
        .painter()
        .layout_no_wrap("Install".to_string(), prose_font, egui::Color32::WHITE)
        .size()
        .x;
    glyph_w + prose_w + 5.0 + 20.0
}

fn muted_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
}

fn game_link(ui: &mut egui::Ui, palette: ThemePalette, text: &str) -> egui::Response {
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
    }

    response
}

fn section_component_count(section: &GameSection) -> usize {
    section
        .mods
        .iter()
        .map(|group| group.components.len())
        .sum()
}

fn section_match_count(section: &GameSection, query_lower: &str) -> usize {
    match_count(section, query_lower)
}

fn other_sections_with_hits(
    inside: &InsideModel,
    active_index: usize,
    query_lower: &str,
) -> Vec<(String, usize)> {
    inside
        .sections
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != active_index)
        .map(|(_, section)| {
            (
                section.game.clone(),
                section_match_count(section, query_lower),
            )
        })
        .collect()
}

#[must_use]
pub(crate) fn count_label(all: usize, hits: usize, query_active: bool) -> String {
    if query_active {
        format!("{hits} of {all} components match")
    } else {
        format!("{all} components")
    }
}

#[must_use]
pub(crate) fn tab_sub(total: usize, hits: usize, query_active: bool) -> String {
    if query_active {
        format!("{hits} of {total}")
    } else {
        format!("{total} components")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EmptyMessage {
    pub(crate) lead: String,
    pub(crate) links: Vec<(String, String)>,
}

#[must_use]
pub(crate) fn empty_message(current: &str, others: &[(String, usize)]) -> EmptyMessage {
    let links: Vec<(String, String)> = others
        .iter()
        .filter(|(_, n)| *n > 0)
        .map(|(game, n)| (other_hint(game, *n), game.clone()))
        .collect();
    let lead = if links.is_empty() {
        NO_MATCHES_ANYWHERE.to_string()
    } else {
        format!("No matches on {current}.")
    };
    EmptyMessage { lead, links }
}

fn other_hint(game: &str, n: usize) -> String {
    format!("{n} on {game}")
}

#[must_use]
pub(crate) fn source_tooltip(label: &str, tier: SourceTier) -> String {
    format!("Download source: {label} (from {})", tier_origin(tier))
}

const fn tier_origin(tier: SourceTier) -> &'static str {
    match tier {
        SourceTier::Modlist => "this modlist",
        SourceTier::User => "your download sources",
        SourceTier::Default => "BIO default sources",
    }
}

#[must_use]
pub(crate) fn footer_text(game: &str) -> String {
    format!("Install order for {game}. Reorder on Step 3 via \"review and modify\".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_label_reads_all_components_at_rest_and_n_of_m_match_under_a_query() {
        assert_eq!(count_label(7, 0, false), "7 components");
        assert_eq!(count_label(7, 4, true), "4 of 7 components match");
    }

    #[test]
    fn tab_sub_reads_components_at_rest_and_n_of_m_under_a_query() {
        assert_eq!(tab_sub(3, 0, false), "3 components");
        assert_eq!(tab_sub(3, 2, true), "2 of 3");
    }

    #[test]
    fn empty_message_points_at_the_other_games_with_hits() {
        let none = empty_message("BGEE", &[]);
        assert_eq!(none.lead, "No mods or components match.");
        assert!(none.links.is_empty());
        let zero_elsewhere = empty_message("BGEE", &[("BG2EE".to_string(), 0)]);
        assert_eq!(zero_elsewhere.lead, "No mods or components match.");
        assert!(zero_elsewhere.links.is_empty());
        let hit_elsewhere = empty_message("BGEE", &[("BG2EE".to_string(), 1)]);
        assert_eq!(hit_elsewhere.lead, "No matches on BGEE.");
        assert_eq!(
            hit_elsewhere.links,
            vec![("1 on BG2EE".to_string(), "BG2EE".to_string())]
        );
    }

    #[test]
    fn source_tooltip_names_the_label_and_the_tier() {
        assert_eq!(
            source_tooltip("github.com/Owner/Repo", SourceTier::Default),
            "Download source: github.com/Owner/Repo (from BIO default sources)"
        );
        assert_eq!(
            source_tooltip("example.com/mod", SourceTier::User),
            "Download source: example.com/mod (from your download sources)"
        );
        assert_eq!(
            source_tooltip("example.com/mod", SourceTier::Modlist),
            "Download source: example.com/mod (from this modlist)"
        );
    }

    #[test]
    fn search_hint_and_footer_copy_are_verbatim() {
        assert_eq!(SEARCH_HINT, "Search by mod, tp2, or component\u{2026}");
        assert_eq!(
            footer_text("BGEE"),
            "Install order for BGEE. Reorder on Step 3 via \"review and modify\"."
        );
    }
}
