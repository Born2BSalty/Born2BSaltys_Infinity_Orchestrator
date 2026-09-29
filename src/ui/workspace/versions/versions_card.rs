// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::step2_action::Step2Action;
use crate::app::versions_view::{CardDot, FetchPhase, SourceRowKind, VersionCard};
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
const FETCH_BAR_H: f32 = 3.0;
const FETCH_BAR_GAP: f32 = 3.0;
const FETCH_ROW_EXTRA_H: f32 = 10.0;
const SELECTOR_TEXT_GAP: f32 = 8.0;
const FETCH_BAR_RADIUS: u8 = 2;
const NOTE_SLOT_W: f32 = 16.0;
const NAME_FONT_SIZE: f32 = 14.0;
const STATUS_FONT_SIZE: f32 = 12.0;
const STACK_GAP: f32 = 1.0;

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
    let allocated_h = if card.fetching.is_some() {
        CARD_ROW_H + FETCH_ROW_EXTRA_H
    } else {
        CARD_ROW_H
    };
    let (allocated_rect, _) =
        ui.allocate_exact_size(egui::vec2(row_width, allocated_h), egui::Sense::hover());
    let row_rect = egui::Rect::from_min_size(allocated_rect.min, egui::vec2(row_width, CARD_ROW_H));

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
    let note = current_note(card);
    let note_w = if note.is_some() { NOTE_SLOT_W } else { 0.0 };
    let name_left = dot_rect.right() + COLUMN_GAP;
    let name_right = selector_rect.left() - COLUMN_GAP - note_w;
    let name_rect = egui::Rect::from_min_size(
        egui::pos2(name_left, row_rect.top()),
        egui::vec2((name_right - name_left).max(0.0), row_rect.height()),
    );

    render_dot(ui, palette, dot_rect, card.dot);
    let anchor_rect = render_main(ui, palette, name_rect, card);
    if let Some(phase) = card.fetching {
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(name_rect.left(), anchor_rect.bottom() + FETCH_BAR_GAP),
            egui::vec2(name_rect.width(), FETCH_BAR_H),
        );
        paint_fetch_bar(ui, palette, bar_rect, phase);
    }
    if let Some((text, who)) = note {
        let slot_rect = egui::Rect::from_min_size(
            egui::pos2(selector_rect.left() - NOTE_SLOT_W, row_rect.top()),
            egui::vec2(NOTE_SLOT_W, row_rect.height()),
        );
        render_note_slot(ui, palette, slot_rect, card, text, who);
    }
    let selector_response = render_selector(ui, palette, selector_rect, card);

    let (action, kebab_response) = render_icons(
        ui,
        palette,
        card,
        busy,
        [fetch_rect, lock_rect, open_rect, kebab_rect],
    );

    CardRow {
        action,
        selector_rect,
        selector_response,
        kebab_rect,
        kebab_response,
    }
}

fn current_note(card: &VersionCard) -> Option<(&str, &str)> {
    card.sources
        .iter()
        .find(|option| option.kind == SourceRowKind::Current)
        .and_then(|option| option.note.as_ref())
        .map(|(text, who)| (text.as_str(), who.as_str()))
}

fn render_note_slot(
    ui: &egui::Ui,
    palette: ThemePalette,
    slot_rect: egui::Rect,
    card: &VersionCard,
    text: &str,
    who: &str,
) {
    if ui.is_rect_visible(slot_rect) {
        versions_icons::paint_note(
            ui.painter(),
            slot_rect.center(),
            redesign_text_muted(palette),
        );
    }
    ui.interact(
        slot_rect,
        ui.id().with(("versions_card_note", &card.tp2)),
        egui::Sense::hover(),
    )
    .on_hover_text(format!("{text}\n\u{2014} {who}"));
}

fn render_icons(
    ui: &egui::Ui,
    palette: ThemePalette,
    card: &VersionCard,
    busy: bool,
    [fetch_rect, lock_rect, open_rect, kebab_rect]: [egui::Rect; 4],
) -> (Option<Step2Action>, egui::Response) {
    let mut action = None;

    if card.can_fetch && card.fetching.is_none() {
        let response = render_fetch_icon(ui, palette, fetch_rect, card);
        if response.clicked() {
            action = Some(Step2Action::DownloadUpdateFor {
                tp2: card.tp2.clone(),
            });
        }
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

    (action, kebab_response)
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

fn render_main(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    rect: egui::Rect,
    card: &VersionCard,
) -> egui::Rect {
    let name_font = egui::FontId::new(
        NAME_FONT_SIZE,
        egui::FontFamily::Name("poppins_medium".into()),
    );
    let status_font = egui::FontId::new(
        STATUS_FONT_SIZE,
        egui::FontFamily::Name("poppins_light".into()),
    );
    let status = status_text(card);
    let name_h = text_height(ui, &card.name, &name_font);
    let stack_h = if status.is_empty() {
        name_h
    } else {
        name_h + STACK_GAP + text_height(ui, &status, &status_font)
    };
    let offset = (rect.height() - stack_h) / 2.0;
    let stack_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(0.0, offset),
        egui::vec2(rect.width(), stack_h),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(stack_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.y = STACK_GAP;
    let name_rect = child
        .add(
            egui::Label::new(
                egui::RichText::new(&card.name)
                    .font(name_font)
                    .color(redesign_text_primary(palette)),
            )
            .truncate(),
        )
        .rect;
    if status.is_empty() {
        return name_rect;
    }
    child
        .add(
            egui::Label::new(
                egui::RichText::new(status)
                    .font(status_font)
                    .color(redesign_text_muted(palette)),
            )
            .truncate(),
        )
        .rect
}

fn text_height(ui: &egui::Ui, text: &str, font: &egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_string(), font.clone(), egui::Color32::WHITE)
        .size()
        .y
}

fn status_text(card: &VersionCard) -> String {
    match card.fetching {
        Some(FetchPhase::Downloading(Some(fraction))) => {
            format!("Fetching\u{2026} {}%", percent_floor(fraction))
        }
        Some(FetchPhase::Downloading(None)) => "Fetching\u{2026}".to_string(),
        Some(FetchPhase::ExtractQueued) => "Queued to extract".to_string(),
        Some(FetchPhase::Extracting) => "Extracting\u{2026}".to_string(),
        Some(FetchPhase::Extracted) => "Extracted".to_string(),
        Some(FetchPhase::Rescanning) => "Rescanning\u{2026}".to_string(),
        None if card.queued => "Queued for fetch".to_string(),
        None => card.status_line.clone(),
    }
}

fn percent_floor(fraction: f32) -> u8 {
    (0_u8..=100)
        .rev()
        .find(|pct| f32::from(*pct) <= fraction.clamp(0.0, 1.0) * 100.0)
        .unwrap_or(0)
}

fn paint_fetch_bar(ui: &egui::Ui, palette: ThemePalette, rect: egui::Rect, phase: FetchPhase) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let radius = egui::CornerRadius::same(FETCH_BAR_RADIUS);
    ui.painter()
        .rect_filled(rect, radius, redesign_border_soft(palette));
    if let Some(fraction) = fetch_bar_fraction(phase) {
        let fill = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
        );
        ui.painter()
            .rect_filled(fill, radius, redesign_accent(palette));
    }
}

const fn fetch_bar_fraction(phase: FetchPhase) -> Option<f32> {
    match phase {
        FetchPhase::Downloading(fraction) => fraction,
        FetchPhase::ExtractQueued | FetchPhase::Extracting | FetchPhase::Extracted => Some(1.0),
        FetchPhase::Rescanning => None,
    }
}

fn render_selector(
    ui: &egui::Ui,
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
        paint_selector_texts(ui, palette, text_rect, card);
    }
    if card.source_id.is_some() {
        response.on_hover_text(card.selector_hover.clone())
    } else {
        response
    }
}

fn paint_selector_texts(
    ui: &egui::Ui,
    palette: ThemePalette,
    text_rect: egui::Rect,
    card: &VersionCard,
) {
    let light = egui::FontFamily::Name("poppins_light".into());
    let texts = if card.source_id.is_some() {
        vec![
            egui::RichText::new(card.layer)
                .size(10.0)
                .family(light.clone())
                .color(redesign_text_faint(palette)),
            egui::RichText::new(&card.rule_words)
                .size(13.0)
                .family(light)
                .color(redesign_text_primary(palette)),
        ]
    } else {
        vec![
            egui::RichText::new("Click to add a source")
                .size(13.0)
                .family(light)
                .color(redesign_text_muted(palette)),
        ]
    };
    let mut x = text_rect.left();
    for text in texts {
        let galley = egui::WidgetText::from(text).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (text_rect.right() - x).max(0.0),
            egui::FontSelection::Default,
        );
        let size = galley.size();
        let pos = egui::pos2(x, size.y.mul_add(-0.5, text_rect.center().y));
        ui.painter()
            .galley(pos, galley, redesign_text_primary(palette));
        x += size.x + SELECTOR_TEXT_GAP;
    }
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
            versions_icons::paint_glyph(
                painter,
                center,
                if locked {
                    versions_icons::GLYPH_LOCK
                } else {
                    versions_icons::GLYPH_UNLOCK
                },
                resolved_color,
            );
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
        |painter, center, color| {
            versions_icons::paint_glyph(
                painter,
                center,
                versions_icons::GLYPH_EXTERNAL_LINK,
                color,
            );
        },
        None,
    )
    .on_hover_text("Open source in browser")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::versions_view::CardStatus;

    fn card_in(phase: Option<FetchPhase>) -> VersionCard {
        VersionCard {
            tp2: "a".to_string(),
            name: "A".to_string(),
            status: CardStatus::Fetch,
            dot: CardDot::Update,
            status_line: "1.0 \u{2192} 2.0".to_string(),
            target: Some("2.0".to_string()),
            locked: false,
            can_fetch: true,
            layer: "BIO default",
            rule_words: String::new(),
            selector_hover: String::new(),
            open_url: None,
            repo: None,
            source_id: None,
            sources: Vec::new(),
            fetching: phase,
            queued: false,
        }
    }

    #[test]
    fn status_text_names_the_three_unpack_states() {
        assert_eq!(
            status_text(&card_in(Some(FetchPhase::ExtractQueued))),
            "Queued to extract"
        );
        assert_eq!(
            status_text(&card_in(Some(FetchPhase::Extracting))),
            "Extracting\u{2026}"
        );
        assert_eq!(
            status_text(&card_in(Some(FetchPhase::Extracted))),
            "Extracted"
        );
        assert_eq!(status_text(&card_in(None)), "1.0 \u{2192} 2.0");
    }

    #[test]
    fn fetch_bar_fill_is_full_for_every_unpack_state() {
        for phase in [
            FetchPhase::ExtractQueued,
            FetchPhase::Extracting,
            FetchPhase::Extracted,
        ] {
            assert_eq!(fetch_bar_fraction(phase), Some(1.0));
        }
        assert_eq!(
            fetch_bar_fraction(FetchPhase::Downloading(Some(0.25))),
            Some(0.25)
        );
        assert_eq!(fetch_bar_fraction(FetchPhase::Downloading(None)), None);
        assert_eq!(fetch_bar_fraction(FetchPhase::Rescanning), None);
    }
}
