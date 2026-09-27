// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::VersionsSheet;
use crate::app::step2_action::{ModSourceEditDestination, Step2Action};
use crate::app::versions_view::{CardSourceOption, VersionCard};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_hover_overlay, redesign_shell_bg, redesign_text_faint,
    redesign_text_primary,
};

use super::versions_icons;

const SOURCES_MENU_W: f32 = 620.0;
const KEBAB_MENU_W: f32 = 220.0;
const ROW_H: f32 = 30.0;

#[derive(Default)]
pub(crate) struct MenuOutcome {
    pub(crate) action: Option<Step2Action>,
    pub(crate) open_sheet: Option<VersionsSheet>,
    pub(crate) close: bool,
}

fn popup_frame(palette: ThemePalette) -> egui::Frame {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(6))
}

fn edit_source_action(card: &VersionCard) -> Step2Action {
    Step2Action::OpenModDownloadSourceEditor {
        tp2: card.tp2.clone(),
        label: card.name.clone(),
        source_id: card
            .source_id
            .clone()
            .unwrap_or_else(|| "new-source".to_string()),
        allow_source_id_change: card.source_id.is_none(),
        destination: card_destination(card),
    }
}

fn card_destination(card: &VersionCard) -> ModSourceEditDestination {
    if card.layer == "This modlist" {
        ModSourceEditDestination::ThisModlist
    } else {
        ModSourceEditDestination::GlobalDefault
    }
}

fn find_forks_action(card: &VersionCard) -> Step2Action {
    Step2Action::DiscoverModDownloadForks {
        tp2: card.tp2.clone(),
        label: card.name.clone(),
        repo: card.repo.clone().unwrap_or_default(),
    }
}

pub(crate) fn render_sources_menu(
    ctx: &egui::Context,
    palette: ThemePalette,
    anchor_rect: egui::Rect,
    anchor_response: &egui::Response,
    card: &VersionCard,
    busy: bool,
) -> MenuOutcome {
    let mut outcome = MenuOutcome::default();
    let pos = egui::pos2(anchor_rect.left(), anchor_rect.bottom() + 4.0);
    let response = egui::Area::new(egui::Id::new("versions_sources_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            popup_frame(palette).show(ui, |ui| {
                ui.set_width(SOURCES_MENU_W);
                render_known_sources_body(ui, palette, card, busy, &mut outcome);
            });
        });
    let should_close = anchor_response.clicked_elsewhere() && response.response.clicked_elsewhere();
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
        outcome.close = true;
    }
    outcome
}

fn render_known_sources_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    card: &VersionCard,
    busy: bool,
    outcome: &mut MenuOutcome,
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.label(
        egui::RichText::new("KNOWN SOURCES")
            .size(11.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_faint(palette)),
    );
    ui.add_space(4.0);
    for option in &card.sources {
        if render_source_row(ui, palette, option, busy) {
            outcome.action = Some(Step2Action::SetModDownloadSource {
                tp2: card.tp2.clone(),
                source_id: option.source_id.clone(),
            });
            outcome.close = true;
        }
    }
    ui.add_space(4.0);
    ui.separator();
    ui.horizontal(|ui| {
        if redesign_btn(
            ui,
            palette,
            "Edit source\u{2026}",
            BtnOpts {
                small: true,
                disabled: busy,
                ..Default::default()
            },
        )
        .clicked()
            && !busy
        {
            outcome.action = Some(edit_source_action(card));
            outcome.open_sheet = Some(VersionsSheet::EditSource);
            outcome.close = true;
        }
        if card.repo.is_some()
            && redesign_btn(
                ui,
                palette,
                "Find forks\u{2026}",
                BtnOpts {
                    small: true,
                    disabled: busy,
                    ..Default::default()
                },
            )
            .clicked()
            && !busy
        {
            outcome.action = Some(find_forks_action(card));
            outcome.open_sheet = Some(VersionsSheet::Forks);
            outcome.close = true;
        }
    });
}

fn render_source_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    option: &CardSourceOption,
    disabled: bool,
) -> bool {
    let sense = if disabled {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        let mut x = rect.left() + 8.0;
        if option.current {
            let check_color = if disabled {
                redesign_accent(palette).gamma_multiply(0.5)
            } else {
                redesign_accent(palette)
            };
            versions_icons::paint_check(painter, egui::pos2(x + 6.0, rect.center().y), check_color);
        }
        x += 18.0;
        let layer_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()));
        let faint = if disabled {
            redesign_text_faint(palette).gamma_multiply(0.6)
        } else {
            redesign_text_faint(palette)
        };
        painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            option.layer,
            layer_font,
            faint,
        );
        x += 118.0;
        let rule_font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()));
        let rule_text = versions_icons::elide(painter, &option.rule_words, &rule_font, 240.0);
        let primary = if disabled {
            redesign_text_primary(palette).gamma_multiply(0.6)
        } else {
            redesign_text_primary(palette)
        };
        painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            rule_text,
            rule_font,
            primary,
        );
        x += 250.0;
        let loc_font = egui::FontId::new(12.0, egui::FontFamily::Monospace);
        let max_w = (rect.right() - 8.0 - x).max(20.0);
        let loc_text = versions_icons::elide(painter, &option.location, &loc_font, max_w);
        painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            loc_text,
            loc_font,
            faint,
        );
    }
    !disabled && response.clicked()
}

pub(crate) fn render_kebab_menu(
    ctx: &egui::Context,
    palette: ThemePalette,
    anchor_rect: egui::Rect,
    anchor_response: &egui::Response,
    card: &VersionCard,
    busy: bool,
) -> MenuOutcome {
    let mut outcome = MenuOutcome::default();
    let pos = egui::pos2(
        anchor_rect.right() - KEBAB_MENU_W,
        anchor_rect.bottom() + 4.0,
    );
    let response = egui::Area::new(egui::Id::new("versions_kebab_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            popup_frame(palette).show(ui, |ui| {
                ui.set_width(KEBAB_MENU_W);
                ui.spacing_mut().item_spacing.y = 0.0;
                if card.can_fetch {
                    let label = card
                        .target
                        .as_deref()
                        .map_or_else(|| "Fetch".to_string(), |t| format!("Fetch {t}"));
                    if menu_item(ui, palette, &label, busy) {
                        outcome.action = Some(Step2Action::DownloadUpdateFor {
                            tp2: card.tp2.clone(),
                        });
                        outcome.close = true;
                    }
                    ui.separator();
                }
                let edit_label = if card.source_id.is_some() {
                    "Edit source\u{2026}"
                } else {
                    "Add source\u{2026}"
                };
                if menu_item(ui, palette, edit_label, busy) {
                    outcome.action = Some(edit_source_action(card));
                    outcome.open_sheet = Some(VersionsSheet::EditSource);
                    outcome.close = true;
                }
                if card.repo.is_some() && menu_item(ui, palette, "Find forks\u{2026}", busy) {
                    outcome.action = Some(find_forks_action(card));
                    outcome.open_sheet = Some(VersionsSheet::Forks);
                    outcome.close = true;
                }
            });
        });
    let should_close = anchor_response.clicked_elsewhere() && response.response.clicked_elsewhere();
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
        outcome.close = true;
    }
    outcome
}

fn menu_item(ui: &mut egui::Ui, palette: ThemePalette, label: &str, disabled: bool) -> bool {
    let font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into()));
    let text_color = if disabled {
        redesign_text_primary(palette).gamma_multiply(0.5)
    } else {
        redesign_text_primary(palette)
    };
    let pad_x = 10.0;
    let pad_y = 6.0;
    let row_width = ui.available_width();
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let row_height = galley.size().y + pad_y * 2.0;
    let sense = if disabled {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(row_width, row_height), sense);
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + pad_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            text_color,
        );
    }
    !disabled && response.clicked()
}
