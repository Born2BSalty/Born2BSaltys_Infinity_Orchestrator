// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::step2_action::Step2Action;
use crate::app::versions_view::{CardDot, VersionCard};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_soft, redesign_border_strong, redesign_chrome_bg, redesign_error,
    redesign_hover_overlay, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary, redesign_warning,
};

use super::versions_icons;

const CARD_RADIUS: u8 = 4;
const CARD_ROW_H: f32 = 32.0;
const ICON_SIZE: f32 = 30.0;
const ICON_GAP: f32 = 2.0;
const COLUMN_GAP: f32 = 16.0;
const DOT_SIZE: f32 = 8.0;

pub(crate) struct CardEvent {
    pub(crate) rect: egui::Rect,
    pub(crate) action: Option<Step2Action>,
    pub(crate) selector_rect: egui::Rect,
    pub(crate) selector_response: egui::Response,
    pub(crate) kebab_rect: egui::Rect,
    pub(crate) kebab_response: egui::Response,
}

struct CardRow {
    action: Option<Step2Action>,
    selector_rect: egui::Rect,
    selector_response: egui::Response,
    kebab_rect: egui::Rect,
    kebab_response: egui::Response,
}

pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    card: &VersionCard,
    selector_width: f32,
    busy: bool,
    focused: bool,
) -> CardEvent {
    let stroke_color = if focused {
        redesign_accent(palette)
    } else {
        redesign_border_soft(palette)
    };
    let frame = egui::Frame::default()
        .fill(redesign_chrome_bg(palette))
        .stroke(egui::Stroke::new(1.0_f32, stroke_color))
        .corner_radius(egui::CornerRadius::same(CARD_RADIUS))
        .inner_margin(egui::Margin::symmetric(16, 9))
        .show(ui, |ui| render_row(ui, palette, card, selector_width, busy));
    let row = frame.inner;

    CardEvent {
        rect: frame.response.rect,
        action: row.action,
        selector_rect: row.selector_rect,
        selector_response: row.selector_response,
        kebab_rect: row.kebab_rect,
        kebab_response: row.kebab_response,
    }
}

fn render_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    card: &VersionCard,
    selector_width: f32,
    busy: bool,
) -> CardRow {
    let row_width = ui.available_width();
    let (row_rect, _) =
        ui.allocate_exact_size(egui::vec2(row_width, CARD_ROW_H), egui::Sense::hover());

    let icons_w = 4.0_f32.mul_add(ICON_SIZE, 3.0 * ICON_GAP);
    let icons_left = row_rect.right() - icons_w;
    let fetch_rect = icon_slot_rect(icons_left, row_rect, 0);
    let lock_rect = icon_slot_rect(icons_left, row_rect, 1);
    let open_rect = icon_slot_rect(icons_left, row_rect, 2);
    let kebab_rect = icon_slot_rect(icons_left, row_rect, 3);

    let selector_right = icons_left - COLUMN_GAP;
    let selector_rect = egui::Rect::from_min_size(
        egui::pos2(selector_right - selector_width, row_rect.top()),
        egui::vec2(selector_width, CARD_ROW_H),
    );

    let dot_rect =
        egui::Rect::from_min_size(row_rect.left_top(), egui::vec2(DOT_SIZE, row_rect.height()));
    let name_left = dot_rect.right() + COLUMN_GAP;
    let name_right = selector_rect.left() - COLUMN_GAP;
    let name_rect = egui::Rect::from_min_size(
        egui::pos2(name_left, row_rect.top()),
        egui::vec2((name_right - name_left).max(0.0), row_rect.height()),
    );

    render_dot(ui, palette, dot_rect, card.dot);
    render_main(ui, palette, name_rect, card);
    let selector_response = render_selector(ui, palette, selector_rect, card);

    let mut action = None;

    if card.can_fetch && !busy {
        let response = render_fetch_icon(ui, palette, fetch_rect, card);
        if response.clicked() {
            action = Some(Step2Action::DownloadUpdateFor {
                tp2: card.tp2.clone(),
            });
        }
    } else if card.can_fetch {
        render_disabled_fetch_icon(ui, palette, fetch_rect);
    }

    let lock_response =
        render_lock_icon(ui, palette, lock_rect, card.tp2.as_str(), card.locked, busy);
    if lock_response.clicked() && !busy {
        action = Some(Step2Action::SetModUpdateLocked {
            tp2: card.tp2.clone(),
            locked: !card.locked,
        });
    }

    if let Some(url) = card.open_url.as_ref() {
        let response = render_open_icon(ui, palette, open_rect, card.tp2.as_str());
        if response.clicked() {
            action = Some(Step2Action::OpenSelectedWeb(url.clone()));
        }
    }

    let kebab_response = icon_button_at(
        ui,
        palette,
        kebab_rect,
        card.tp2.as_str(),
        "kebab",
        versions_icons::paint_kebab,
        None,
    );

    CardRow {
        action,
        selector_rect,
        selector_response,
        kebab_rect,
        kebab_response,
    }
}

fn icon_slot_rect(icons_left: f32, row_rect: egui::Rect, index: u8) -> egui::Rect {
    let offset = f32::from(index).mul_add(ICON_SIZE + ICON_GAP, icons_left);
    egui::Rect::from_min_size(
        egui::pos2(offset, row_rect.top()),
        egui::vec2(ICON_SIZE, row_rect.height()),
    )
}

fn render_dot(ui: &egui::Ui, palette: ThemePalette, rect: egui::Rect, dot: CardDot) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let color = match dot {
        CardDot::Update => redesign_accent(palette),
        CardDot::Warn => redesign_warning(palette),
        CardDot::Bad => redesign_error(palette),
        CardDot::Neutral => redesign_text_faint(palette),
    };
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

fn render_main(ui: &mut egui::Ui, palette: ThemePalette, rect: egui::Rect, card: &VersionCard) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.y = 1.0;
    child.add(
        egui::Label::new(
            egui::RichText::new(&card.name)
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        )
        .truncate(),
    );
    child.add(
        egui::Label::new(
            egui::RichText::new(&card.status_line)
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        )
        .truncate(),
    );
}

fn render_selector(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    rect: egui::Rect,
    card: &VersionCard,
) -> egui::Response {
    let id = ui.id().with((card.tp2.as_str(), "versions_selector"));
    let response = ui.interact(rect, id, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let border_color = if response.hovered() {
            redesign_accent(palette)
        } else {
            redesign_border_strong(palette)
        };
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            redesign_shell_bg(palette),
        );
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border_color),
            egui::StrokeKind::Inside,
        );
        let caret_center = egui::pos2(rect.right() - 14.0, rect.center().y);
        versions_icons::paint_caret_down(ui.painter(), caret_center, redesign_text_faint(palette));

        let text_rect = egui::Rect::from_min_max(
            rect.min + egui::vec2(10.0, 0.0),
            egui::pos2(caret_center.x - 8.0, rect.max.y),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(text_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        if card.source_id.is_some() {
            child.spacing_mut().item_spacing.x = 8.0;
            child.add(
                egui::Label::new(
                    egui::RichText::new(card.layer)
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_faint(palette)),
                )
                .truncate(),
            );
            child.add(
                egui::Label::new(
                    egui::RichText::new(&card.rule_words)
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_primary(palette)),
                )
                .truncate(),
            );
        } else {
            child.add(
                egui::Label::new(
                    egui::RichText::new("Click to add a source")
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_muted(palette)),
                )
                .truncate(),
            );
        }
    }
    response
}

fn icon_button_at(
    ui: &egui::Ui,
    palette: ThemePalette,
    rect: egui::Rect,
    tp2: &str,
    salt: &str,
    paint: impl FnOnce(&egui::Painter, egui::Pos2, egui::Color32),
    active_color: Option<egui::Color32>,
) -> egui::Response {
    let id = ui.id().with((tp2, salt));
    let response = ui.interact(rect, id, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        let color = active_color.unwrap_or_else(|| redesign_text_muted(palette));
        paint(painter, rect.center(), color);
    }
    response
}

fn render_fetch_icon(
    ui: &egui::Ui,
    palette: ThemePalette,
    rect: egui::Rect,
    card: &VersionCard,
) -> egui::Response {
    let tip = card
        .target
        .as_ref()
        .map_or_else(|| "Fetch".to_string(), |target| format!("Fetch {target}"));
    icon_button_at(
        ui,
        palette,
        rect,
        card.tp2.as_str(),
        "fetch",
        versions_icons::paint_down_arrow,
        Some(redesign_accent(palette)),
    )
    .on_hover_text(tip)
}

fn render_disabled_fetch_icon(ui: &egui::Ui, palette: ThemePalette, rect: egui::Rect) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let color = redesign_accent(palette).gamma_multiply(0.35);
    versions_icons::paint_down_arrow(ui.painter(), rect.center(), color);
}

fn render_lock_icon(
    ui: &egui::Ui,
    palette: ThemePalette,
    rect: egui::Rect,
    tp2: &str,
    locked: bool,
    busy: bool,
) -> egui::Response {
    let tip = if locked {
        "Locked \u{00B7} Fetch skips this mod"
    } else {
        "Lock this version"
    };
    let color = if busy {
        Some(redesign_text_faint(palette).gamma_multiply(0.6))
    } else {
        locked.then(|| redesign_accent(palette))
    };
    icon_button_at(
        ui,
        palette,
        rect,
        tp2,
        "lock",
        move |painter, center, resolved_color| {
            versions_icons::paint_lock(painter, center, resolved_color, locked);
        },
        color,
    )
    .on_hover_text(tip)
}

fn render_open_icon(
    ui: &egui::Ui,
    palette: ThemePalette,
    rect: egui::Rect,
    tp2: &str,
) -> egui::Response {
    icon_button_at(
        ui,
        palette,
        rect,
        tp2,
        "open",
        versions_icons::paint_external_link,
        None,
    )
    .on_hover_text("Open source in browser")
}
