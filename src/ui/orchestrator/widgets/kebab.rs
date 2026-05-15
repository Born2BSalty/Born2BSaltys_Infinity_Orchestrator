// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `Kebab` primitive — three-dot overflow-menu button with a click-to-open
// dropdown of menu items. Reusable across Home cards, Step 5, etc.
//
// Mirrors `wireframe-preview/screens.jsx::Kebab` (line 1625-1669):
//   - Trigger: a `small` Btn rendering `···` (3×9 padding, 15px, lineHeight 1).
//   - Dropdown: `position:absolute; top:calc(100%+4px); right:0; minWidth:180`,
//     sketchyBorder, shell-bg fill, 3×3 drop shadow, 4px padding, zIndex 5.
//   - Items: 6×10 padding, 13px, 3px radius, hover paints `--hover-overlay`.
//   - Clicking outside the dropdown closes it (wireframe attaches a one-shot
//     document click listener).
//
// egui implementation: the trigger button toggles a per-id popup via
// `egui::Memory::toggle_popup`; `egui::popup::popup_below_widget` with
// `PopupCloseBehavior::CloseOnClickOutside` renders the dropdown in an
// `Order::Foreground` area (floats above the page) and closes it on a click
// landing outside both the trigger and the dropdown — the egui-idiomatic
// equivalent of the wireframe's document listener. The dropdown is
// right-aligned under the trigger to match `right: 0`.
//
// Danger items render in coral (`pill_danger`) per SPEC §3.2.
//
// SPEC: §3.2 (card Kebab menus), §6.4 (toolbar Kebab pattern).

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    redesign_border_strong, redesign_hover_overlay, redesign_pill_danger, redesign_shadow,
    redesign_shell_bg, redesign_text_primary, ThemePalette, REDESIGN_BORDER_RADIUS_PX,
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
};

/// One entry in a Kebab dropdown.
///
/// `on_click` is invoked when the item is clicked (the dropdown closes
/// first). `danger: true` paints the label in coral for destructive actions
/// (Delete / Reinstall).
pub struct KebabItem<'a> {
    pub label: &'a str,
    pub on_click: Box<dyn FnMut() + 'a>,
    pub danger: bool,
}

impl<'a> KebabItem<'a> {
    /// Convenience constructor for a normal (non-danger) item.
    pub fn new(label: &'a str, on_click: impl FnMut() + 'a) -> Self {
        Self {
            label,
            on_click: Box::new(on_click),
            danger: false,
        }
    }

    /// Convenience constructor for a danger (coral) item.
    pub fn danger(label: &'a str, on_click: impl FnMut() + 'a) -> Self {
        Self {
            label,
            on_click: Box::new(on_click),
            danger: true,
        }
    }
}

/// Minimum dropdown width — wireframe `minWidth: 180`.
const DROPDOWN_MIN_WIDTH_PX: f32 = 180.0;

/// Render a Kebab trigger + its dropdown. `id_salt` must be unique per Kebab
/// instance on a frame (e.g. the modlist id) so multiple cards each get an
/// independent open/closed state.
///
/// Returns the trigger's `egui::Response` (callers rarely need it; provided
/// for layout symmetry with the other widgets).
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    id_salt: &str,
    items: &mut [KebabItem<'_>],
) -> egui::Response {
    let popup_id = ui.make_persistent_id(("orchestrator_kebab", id_salt));

    // ── Trigger: a small sketchy `···` button (3×9 padding, 15px). ──
    let trigger = trigger_button(ui, palette);
    if trigger.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    // ── Dropdown ──
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &trigger,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            // Right-align the menu under the trigger (wireframe `right: 0`)
            // and apply the sketchy chassis + shadow ourselves so the popup
            // matches the redesign rather than egui's default popup frame.
            ui.set_min_width(DROPDOWN_MIN_WIDTH_PX);

            let chassis = egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
                .inner_margin(egui::Margin::same(4))
                .shadow(egui::epaint::Shadow {
                    offset: [
                        REDESIGN_SHADOW_OFFSET_BTN_PX as i8 + 1,
                        REDESIGN_SHADOW_OFFSET_BTN_PX as i8 + 1,
                    ],
                    blur: 0,
                    spread: 0,
                    color: redesign_shadow(palette),
                });

            chassis.show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for item in items.iter_mut() {
                    if menu_item(ui, palette, item.label, item.danger) {
                        (item.on_click)();
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            });
        },
    );

    trigger
}

/// The `···` trigger button — `small` sketchy Btn with tightened 3×9 padding
/// and a 15px glyph (wireframe `padding: "3px 9px", fontSize: 15`).
fn trigger_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 9.0;
    let pad_y = 3.0;
    let label = "\u{00B7}\u{00B7}\u{00B7}"; // ···
    let text_color = redesign_text_primary(palette);
    let font = egui::FontId::new(15.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }

    response.on_hover_text("More actions")
}

/// One dropdown row. Returns `true` when clicked. Hover paints the
/// `hover_overlay`; danger items render coral.
fn menu_item(ui: &mut egui::Ui, palette: ThemePalette, label: &str, danger: bool) -> bool {
    let text_color = if danger {
        redesign_pill_danger(palette)
    } else {
        redesign_text_primary(palette)
    };
    let font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into()));

    // Full-width clickable row with 6×10 inner padding.
    let pad_x = 10.0;
    let pad_y = 6.0;
    let row_width = ui.available_width().max(DROPDOWN_MIN_WIDTH_PX - 8.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let row_height = galley.size().y + pad_y * 2.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(row_width, row_height), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
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

    response.clicked()
}
