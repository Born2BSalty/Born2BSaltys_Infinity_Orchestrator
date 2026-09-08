// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::Game;
use crate::ui::install::gallery::card_art;
use crate::ui::install::gallery::catalog::{self, GalleryEntry};
use crate::ui::install::gallery::filter::{CARD_GAP_PX, GalleryFilter, column_count};
use crate::ui::install::state_install::InstallScreenState;
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, PillTone, redesign_box, redesign_btn, redesign_btn_height,
    redesign_text_input, render_pill, render_screen_title,
};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_input_bg, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GalleryOutcome {
    #[default]
    Stay,
    OpenPaste,
    OpenDetails(usize),
}

const GAME_OPTIONS: [Game; 4] = [Game::BGEE, Game::BG2EE, Game::IWDEE, Game::EET];
const PASTE_BTN_W_PX: f32 = 190.0;
const GAME_COMBO_W_PX: f32 = 170.0;
const STARTER_CHECK_W_PX: f32 = 170.0;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
) -> GalleryOutcome {
    let mut outcome = GalleryOutcome::Stay;

    ui.horizontal_top(|ui| {
        let title_w = (ui.available_width() - PASTE_BTN_W_PX).max(200.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                render_screen_title(
                    ui,
                    palette,
                    "Install a Modlist",
                    Some("Find your next adventure."),
                );
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if redesign_btn(ui, palette, "Paste share code", BtnOpts::default()).clicked() {
                outcome = GalleryOutcome::OpenPaste;
            }
        });
    });

    filter_row(ui, palette, &mut state.gallery.filter);
    ui.add_space(12.0);

    let visible: Vec<(usize, &'static GalleryEntry)> = catalog::entries()
        .iter()
        .enumerate()
        .filter(|(_, entry)| state.gallery.filter.matches(entry))
        .collect();

    ui.label(
        egui::RichText::new(count_label(visible.len()))
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(10.0);

    if visible.is_empty() {
        if empty_state(ui, palette) {
            state.gallery.filter = GalleryFilter::default();
        }
        return outcome;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(index) = card_grid(ui, palette, &visible) {
                outcome = GalleryOutcome::OpenDetails(index);
            }
        });

    outcome
}

fn filter_row(ui: &mut egui::Ui, palette: ThemePalette, filter: &mut GalleryFilter) {
    let box_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            14.0,
            egui::FontFamily::Name("poppins_light".into()),
        ))
    }) + 16.0;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        let reserved = GAME_COMBO_W_PX + STARTER_CHECK_W_PX + 24.0;
        let search_w = (ui.available_width() - reserved).max(160.0);
        redesign_text_input(
            ui,
            palette,
            InputOpts {
                edit: egui::TextEdit::singleline(&mut filter.search)
                    .font(egui::FontId::new(
                        12.0,
                        egui::FontFamily::Name("poppins_light".into()),
                    ))
                    .hint_text(
                        egui::RichText::new("Search modlists or creators\u{2026}")
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_faint(palette)),
                    )
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .vertical_align(egui::Align::Center)
                    .margin(egui::Margin::symmetric(12, 8)),
                margin: egui::Margin::symmetric(12, 8),
                size: egui::vec2(search_w, box_h),
                border: None,
            },
        );

        ui.allocate_ui_with_layout(
            egui::vec2(GAME_COMBO_W_PX, box_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| game_combo(ui, palette, &mut filter.game, box_h),
        );

        ui.allocate_ui_with_layout(
            egui::vec2(STARTER_CHECK_W_PX, box_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.checkbox(
                    &mut filter.starter_only,
                    egui::RichText::new("Starter lists only")
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_muted(palette)),
                );
            },
        );
    });
}

fn count_label(count: usize) -> String {
    if count == 1 {
        "1 modlist".to_string()
    } else {
        format!("{count} modlists")
    }
}

fn game_label(game: Option<Game>) -> &'static str {
    game.map_or("All games", Game::to_legacy_string)
}

fn game_combo(ui: &mut egui::Ui, palette: ThemePalette, game: &mut Option<Game>, box_h: f32) {
    let mut selected = *game;

    let combo_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let content_h = ui
        .fonts(|f| f.row_height(&combo_font))
        .max(ui.spacing().icon_width);
    let pad_y = ((box_h - content_h) / 2.0).max(0.0);

    let fill = redesign_input_bg(palette);
    let visuals = ui.visuals_mut();
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
    }
    ui.spacing_mut().button_padding = egui::vec2(10.0, pad_y);

    egui::ComboBox::from_id_salt("install_gallery_game_filter")
        .width(GAME_COMBO_W_PX)
        .selected_text(
            egui::RichText::new(game_label(selected))
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut selected,
                None,
                egui::RichText::new(game_label(None))
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            for option in GAME_OPTIONS {
                ui.selectable_value(
                    &mut selected,
                    Some(option),
                    egui::RichText::new(option.to_legacy_string())
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                );
            }
        });

    if selected != *game {
        *game = selected;
    }
}

fn empty_state(ui: &mut egui::Ui, palette: ThemePalette) -> bool {
    let mut cleared = false;
    redesign_box(ui, palette, None, |ui| {
        ui.label(
            egui::RichText::new("No modlists match these filters.")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.add_space(10.0);
        if redesign_btn(
            ui,
            palette,
            "Clear filters",
            BtnOpts {
                small: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            cleared = true;
        }
    });
    cleared
}

struct CardMetrics {
    card_w: f32,
    content_h: f32,
    art_h: f32,
    title_h: f32,
    author_h: f32,
    desc_h: f32,
    pills_h: f32,
    btn_h: f32,
}

fn card_metrics(ui: &egui::Ui, card_w: f32) -> CardMetrics {
    let inner_w = (card_w - 24.0).max(80.0);
    let art_h = card_art::height_for_width(inner_w);
    let title_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            16.0,
            egui::FontFamily::Name("poppins_medium".into()),
        ))
    });
    let body_line = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            13.0,
            egui::FontFamily::Name("poppins_light".into()),
        ))
    });
    let author_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            12.0,
            egui::FontFamily::Name("poppins_light".into()),
        ))
    });
    let pill_row = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            11.0,
            egui::FontFamily::Name("poppins_medium".into()),
        ))
    }) + 4.0;
    let pills_h = pill_row.mul_add(2.0, 4.0);
    let desc_h = (body_line * 3.0).ceil();
    let btn_h = redesign_btn_height(ui, true);

    let content_h =
        art_h + 10.0 + title_h + 2.0 + author_h + 8.0 + desc_h + 8.0 + pills_h + 10.0 + btn_h;

    CardMetrics {
        card_w,
        content_h,
        art_h,
        title_h,
        author_h,
        desc_h,
        pills_h,
        btn_h,
    }
}

fn card_grid(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    visible: &[(usize, &'static GalleryEntry)],
) -> Option<usize> {
    let mut opened = None;

    let available = ui.available_width();
    let columns = column_count(available);
    let gaps = CARD_GAP_PX * (columns_minus_one(columns));
    let card_w = ((available - gaps) / columns_as_f32(columns)).floor();
    let metrics = card_metrics(ui, card_w);

    for row in visible.chunks(columns) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = CARD_GAP_PX;
            for (index, entry) in row {
                ui.allocate_ui_with_layout(
                    egui::vec2(metrics.card_w, metrics.content_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(metrics.card_w);
                        let clicked = ui
                            .push_id(entry.id, |ui| card(ui, palette, entry, &metrics))
                            .inner;
                        if clicked {
                            opened = Some(*index);
                        }
                    },
                );
            }
        });
        ui.add_space(CARD_GAP_PX);
    }

    opened
}

const fn columns_minus_one(columns: usize) -> f32 {
    match columns {
        0 | 1 => 0.0,
        2 => 1.0,
        3 => 2.0,
        _ => 3.0,
    }
}

const fn columns_as_f32(columns: usize) -> f32 {
    match columns {
        0 | 1 => 1.0,
        2 => 2.0,
        3 => 3.0,
        _ => 4.0,
    }
}

fn card(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &GalleryEntry,
    metrics: &CardMetrics,
) -> bool {
    let mut open_details = false;

    redesign_box(ui, palette, None, |ui| {
        ui.set_min_height(metrics.content_h);
        let inner_w = ui.available_width();

        let (art_rect, _) =
            ui.allocate_exact_size(egui::vec2(inner_w, metrics.art_h), egui::Sense::hover());
        card_art::paint(ui, palette, entry.game, art_rect);

        ui.add_space(10.0);
        clipped_line(ui, inner_w, metrics.title_h, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(entry.name)
                        .size(16.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                )
                .truncate(),
            );
        });

        ui.add_space(2.0);
        clipped_line(ui, inner_w, metrics.author_h, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("by {}", entry.author))
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_muted(palette)),
                )
                .truncate(),
            );
        });

        ui.add_space(8.0);
        clipped_description(ui, palette, entry.description, inner_w, metrics.desc_h);

        ui.add_space(8.0);
        clipped_line(ui, inner_w, metrics.pills_h, |ui| {
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
        });

        let used = ui.min_rect().height();
        ui.add_space((metrics.content_h - used - metrics.btn_h).max(0.0));

        if redesign_btn(
            ui,
            palette,
            "Details",
            BtnOpts {
                primary: true,
                small: true,
                block: true,
                no_shadow: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            open_details = true;
        }
    });

    open_details
}

fn clipped_description(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    text: &str,
    width: f32,
    height: f32,
) {
    let font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()));
    let color = redesign_text_muted(palette);
    let galley = ui
        .painter()
        .layout(text.to_string(), font.clone(), color, width);
    let overflows = galley.size().y > height + 0.5;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let clip = rect.intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip);
    painter.galley(rect.min, galley, color);

    if overflows {
        let fade_w = 16.0;
        let last_line_top = height.mul_add(-1.0 / 3.0, rect.bottom());
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(rect.right() - fade_w, last_line_top),
                rect.right_bottom(),
            ),
            egui::CornerRadius::ZERO,
            redesign_shell_bg(palette),
        );
        painter.text(
            rect.right_bottom(),
            egui::Align2::RIGHT_BOTTOM,
            "\u{2026}",
            font,
            color,
        );
    }
}

fn clipped_line(ui: &mut egui::Ui, width: f32, height: f32, body: impl FnOnce(&mut egui::Ui)) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let clip = rect.intersect(ui.clip_rect());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.set_clip_rect(clip);
    body(&mut child);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_label_pluralises_on_everything_but_one() {
        assert_eq!(count_label(0), "0 modlists");
        assert_eq!(count_label(1), "1 modlist");
        assert_eq!(count_label(2), "2 modlists");
        assert_eq!(count_label(4), "4 modlists");
    }

    #[test]
    fn all_games_is_the_unfiltered_label() {
        assert_eq!(game_label(None), "All games");
        assert_eq!(game_label(Some(Game::IWDEE)), "IWDEE");
    }

    #[test]
    fn gap_and_divisor_track_the_column_count() {
        for (columns, gaps, divisor) in [
            (1_usize, 0.0_f32, 1.0_f32),
            (2, 1.0, 2.0),
            (3, 2.0, 3.0),
            (4, 3.0, 4.0),
        ] {
            assert!((columns_minus_one(columns) - gaps).abs() < f32::EPSILON);
            assert!((columns_as_f32(columns) - divisor).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn a_zero_column_count_never_divides_by_zero() {
        assert!((columns_as_f32(0) - 1.0).abs() < f32::EPSILON);
        assert!((columns_minus_one(0) - 0.0).abs() < f32::EPSILON);
    }
}
