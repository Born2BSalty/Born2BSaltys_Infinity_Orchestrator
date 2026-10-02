// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::game_version::{InstallBlock, install_block_cached};
use crate::app::state::Step1State;
use crate::gallery_feed::index::FeedEntry;
use crate::registry::model::Game;
use crate::ui::install::gallery::card_art;
use crate::ui::install::gallery::catalog;
use crate::ui::install::gallery::filter::{CARD_GAP_PX, GalleryFilter, column_count};
use crate::ui::install::state_install::InstallScreenState;
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, PillTone, redesign_box, redesign_btn, redesign_btn_height,
    redesign_text_input, render_pill, render_screen_title,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, ThemePalette, redesign_input_bg, redesign_pill_text,
    redesign_pill_warn, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GalleryOutcome {
    #[default]
    Stay,
    OpenPaste,
    OpenFile,
    OpenDetails(String),
}

const GAME_OPTIONS: [Game; 4] = [Game::BGEE, Game::BG2EE, Game::IWDEE, Game::EET];
const TITLE_ROW_BUTTONS_W_PX: f32 = 380.0;
const GAME_COMBO_W_PX: f32 = 170.0;
const STARTER_CHECK_W_PX: f32 = 170.0;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
    step1: &Step1State,
) -> GalleryOutcome {
    let mut outcome = GalleryOutcome::Stay;

    ui.horizontal_top(|ui| {
        let title_w = (ui.available_width() - TITLE_ROW_BUTTONS_W_PX).max(200.0);
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
            if redesign_btn(ui, palette, "Install from file", BtnOpts::default()).clicked() {
                outcome = GalleryOutcome::OpenFile;
            }
            if redesign_btn(ui, palette, "Paste share code", BtnOpts::default()).clicked() {
                outcome = GalleryOutcome::OpenPaste;
            }
        });
    });

    filter_row(ui, palette, &mut state.gallery.filter);
    ui.add_space(12.0);

    let visible: Vec<&FeedEntry> = catalog::entries()
        .iter()
        .filter(|entry| state.gallery.filter.matches(entry))
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
            if let Some(id) = card_grid(ui, palette, &visible, step1) {
                outcome = GalleryOutcome::OpenDetails(id);
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
                    &mut filter.featured_only,
                    egui::RichText::new("Featured only")
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
    visible: &[&FeedEntry],
    step1: &Step1State,
) -> Option<String> {
    let mut opened = None;

    let available = ui.available_width();
    let columns = column_count(available);
    let gaps = CARD_GAP_PX * (columns_minus_one(columns));
    let card_w = ((available - gaps) / columns_as_f32(columns)).floor();
    let metrics = card_metrics(ui, card_w);

    for row in visible.chunks(columns) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = CARD_GAP_PX;
            for entry in row {
                ui.allocate_ui_with_layout(
                    egui::vec2(metrics.card_w, metrics.content_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(metrics.card_w);
                        let clicked = ui
                            .push_id(&entry.id, |ui| card(ui, palette, entry, &metrics, step1))
                            .inner;
                        if clicked {
                            opened = Some(entry.id.clone());
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

#[must_use]
pub(crate) fn card_install_block(entry: &FeedEntry, step1: &Step1State) -> Option<InstallBlock> {
    let tag = entry.game_version.as_deref()?;
    install_block_cached(step1, entry.game.to_legacy_string(), tag)
}

fn paint_block_badge(
    ui: &egui::Ui,
    palette: ThemePalette,
    art_rect: egui::Rect,
    block: InstallBlock,
) {
    let mut painter = ui.painter().clone();
    painter.set_opacity(1.0);

    let pad_x = 8.0;
    let pad_y = 2.0;
    let text_color = redesign_pill_text(palette);
    let font = egui::FontId::new(11.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = painter.layout_no_wrap(block.label().to_string(), font.clone(), text_color);
    let size = egui::vec2(
        f32::mul_add(pad_x, 2.0, galley.size().x),
        f32::mul_add(pad_y, 2.0, galley.size().y),
    );
    let rect = egui::Rect::from_min_size(
        egui::pos2(art_rect.right() - 8.0 - size.x, art_rect.top() + 8.0),
        size,
    );

    painter.rect_filled(
        rect,
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
        redesign_pill_warn(palette),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        block.label(),
        font,
        text_color,
    );
}

fn card(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &FeedEntry,
    metrics: &CardMetrics,
    step1: &Step1State,
) -> bool {
    let mut open_details = false;
    let block = card_install_block(entry, step1);

    redesign_box(ui, palette, None, |ui| {
        ui.set_min_height(metrics.content_h);
        let inner_w = ui.available_width();

        let (art_rect, _) =
            ui.allocate_exact_size(egui::vec2(inner_w, metrics.art_h), egui::Sense::hover());
        let mut art_ui = ui.new_child(egui::UiBuilder::new().max_rect(art_rect));
        if block.is_some() {
            art_ui.multiply_opacity(0.45);
        }
        if let Some(png) = entry.cover_png.as_deref() {
            card_art::paint_cover(&art_ui, palette, entry.game, &entry.id, png, art_rect);
        } else {
            card_art::paint(&art_ui, palette, entry.game, art_rect);
        }
        if let Some(block) = block {
            paint_block_badge(ui, palette, art_rect, block);
        }

        ui.add_space(10.0);
        clipped_line(ui, inner_w, metrics.title_h, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(entry.name.as_str())
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
        clipped_description(ui, palette, &entry.description, inner_w, metrics.desc_h);

        ui.add_space(8.0);
        clipped_line(ui, inner_w, metrics.pills_h, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                render_pill(ui, palette, entry.game.to_legacy_string(), PillTone::Info);
                if let Some(tag) = entry.game_version.as_deref() {
                    render_pill(ui, palette, tag, PillTone::Info);
                }
                for tag in &entry.tags {
                    render_pill(ui, palette, tag, PillTone::Neutral);
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
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::app::compat_dlc_source::refresh_source_check;

    struct TempFixture {
        path: std::path::PathBuf,
    }

    impl TempFixture {
        fn new(name: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_stage_gallery_test_{name}_{}_{id}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn write_key(&self, bytes: &[u8]) {
            std::fs::write(self.path.join("chitin.key"), bytes).unwrap();
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn entry(game: Game, game_version: Option<&str>) -> FeedEntry {
        FeedEntry {
            id: "entry-under-test".to_string(),
            name: "Entry Under Test".to_string(),
            author: "BIO Team".to_string(),
            description: String::new(),
            tags: Vec::new(),
            game,
            featured: false,
            version: "1.0.0".to_string(),
            requirements: String::new(),
            code: String::new(),
            cover_png: None,
            game_version: game_version.map(str::to_string),
            order: 0,
        }
    }

    #[test]
    fn card_install_block_is_none_for_untagged_lists() {
        let untagged = entry(Game::BG2EE, None);
        assert_eq!(card_install_block(&untagged, &Step1State::default()), None);
    }

    #[test]
    fn card_install_block_follows_the_probe() {
        let matching = TempFixture::new("matching");
        matching.write_key(b"data/PATCH26.BIF");
        let mismatched = TempFixture::new("mismatched");
        mismatched.write_key(b"data/PATCH27.BIF");

        let tagged = entry(Game::BG2EE, Some("2.6"));

        let mut step1_matching = Step1State {
            bg2ee_game_folder: matching.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        refresh_source_check(&mut step1_matching);
        assert_eq!(card_install_block(&tagged, &step1_matching), None);

        let mut step1_mismatched = Step1State {
            bg2ee_game_folder: mismatched.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        refresh_source_check(&mut step1_mismatched);
        assert_eq!(
            card_install_block(&tagged, &step1_mismatched),
            Some(InstallBlock::VersionMismatch)
        );
    }

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
