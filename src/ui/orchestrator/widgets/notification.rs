// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use eframe::egui;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};

use crate::ui::orchestrator::widgets::icon_button::paint_close_icon;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX,
    ThemePalette, redesign_border_strong, redesign_error, redesign_error_emphasis, redesign_info,
    redesign_shell_bg, redesign_success, redesign_text_muted, redesign_text_primary,
    redesign_warning,
};
use crate::ui::shared::redesign_visuals::redesign_overlay_shadow;

const HISTORY_CAP: usize = 5;
const TOAST_OFFSET_RIGHT: f32 = 16.0;
const TOAST_OFFSET_BOTTOM: f32 = 44.0;
const TOAST_WIDTH: f32 = 320.0;
const TOAST_PADDING_H: i8 = 14;
const TOAST_PADDING_V: i8 = 10;
const ACCENT_STRIPE_LEFT: i8 = 17;
const TOAST_BODY_WIDTH: f32 = TOAST_WIDTH - 3.0 - 28.0;
const ACCENT_STRIPE_WIDTH: f32 = 3.0;
const ICON_SIZE: f32 = 13.0;
const CLOSE_BTN_SIZE: egui::Vec2 = egui::vec2(18.0, 18.0);

const TTL_SUCCESS: f64 = 3.0;
const TTL_INFO: f64 = 3.0;
const TTL_WARNING: f64 = 4.0;

/// Custom toast kind for install-progress feedback (defined but not yet
/// wired to any push site).
pub const KIND_PROGRESS: u32 = 0;
/// Custom toast kind reserved for future use.
pub const KIND_CUSTOM: u32 = 1;

/// A single entry in the read-only notification history.
#[derive(Clone)]
pub struct NotificationRecord {
    /// Severity of the original notification.
    pub kind: ToastKind,
    /// Message text.
    pub text: String,
    /// Wall-clock time the notification was added.
    pub added_at: Instant,
}

/// App-wide notification manager.
///
/// One long-lived instance lives on `OrchestratorApp`.  Call the add-site
/// methods (`success`, `info`, `warn`, `error`) to enqueue a toast, then call
/// `show` once per frame to flush the queue into the egui-toast queue.
#[derive(Default)]
pub struct NotificationManager {
    pending: Vec<(ToastKind, String)>,
    history: VecDeque<NotificationRecord>,
    /// Whether the history popup is open.
    pub history_open: bool,
}

impl NotificationManager {
    /// Creates an empty manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues a success notification.
    pub fn success(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Success, text.into());
    }

    /// Enqueues an informational notification.
    pub fn info(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Info, text.into());
    }

    /// Enqueues a warning notification.
    pub fn warn(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Warning, text.into());
    }

    /// Enqueues an error notification (persists until manually dismissed).
    pub fn error(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Error, text.into());
    }

    fn push(&mut self, kind: ToastKind, text: String) {
        self.pending.push((kind, text.clone()));
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(NotificationRecord {
            kind,
            text,
            added_at: Instant::now(),
        });
    }

    /// Returns the capped last-5 notification history (oldest-first).
    #[must_use]
    pub const fn history(&self) -> &VecDeque<NotificationRecord> {
        &self.history
    }

    /// Flushes pending toasts into egui-toast and renders the live queue.
    ///
    /// Call exactly once per frame, after the shell render closure, so toasts
    /// overlay every screen.
    pub fn show(&mut self, ctx: &egui::Context, palette: ThemePalette) {
        let mut toasts = Toasts::new()
            .anchor(
                egui::Align2::RIGHT_BOTTOM,
                egui::pos2(-TOAST_OFFSET_RIGHT, -TOAST_OFFSET_BOTTOM),
            )
            .direction(egui::Direction::BottomUp)
            .custom_contents(ToastKind::Success, move |ui, toast| {
                render_custom_toast(ui, toast, palette)
            })
            .custom_contents(ToastKind::Info, move |ui, toast| {
                render_custom_toast(ui, toast, palette)
            })
            .custom_contents(ToastKind::Warning, move |ui, toast| {
                render_custom_toast(ui, toast, palette)
            })
            .custom_contents(ToastKind::Error, move |ui, toast| {
                render_custom_toast(ui, toast, palette)
            })
            .custom_contents(KIND_PROGRESS, move |ui, toast| {
                render_custom_toast(ui, toast, palette)
            })
            .custom_contents(KIND_CUSTOM, move |ui, toast| {
                render_custom_toast(ui, toast, palette)
            });

        for (kind, text) in self.pending.drain(..) {
            let options = toast_options_for(kind);
            toasts.add(Toast::new().kind(kind).text(text).options(options));
        }

        toasts.show(ctx);
    }

    /// Returns whether there are any entries in the history.
    #[must_use]
    pub fn has_history(&self) -> bool {
        !self.history.is_empty()
    }

    /// Renders the read-only history popup anchored above the statusbar right edge.
    ///
    /// `allow_outside_close` must be `false` on the same frame the bell was
    /// clicked (so the click-outside check doesn't self-close the popup
    /// before it renders).  Pass `true` on all subsequent frames.
    pub fn render_history_popup(
        &mut self,
        ctx: &egui::Context,
        palette: ThemePalette,
        allow_outside_close: bool,
    ) {
        if !self.history_open {
            return;
        }

        let popup_w = 340.0_f32;
        let popup_id = egui::Id::new("notification_history_popup");
        let response = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .anchor(
                egui::Align2::RIGHT_BOTTOM,
                egui::vec2(-TOAST_OFFSET_RIGHT, -(REDESIGN_STATUSBAR_HEIGHT_PX + 8.0)),
            )
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(redesign_shell_bg(palette))
                    .stroke(egui::Stroke::new(
                        REDESIGN_BORDER_WIDTH_PX,
                        redesign_border_strong(palette),
                    ))
                    .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
                    .shadow(redesign_overlay_shadow(palette))
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 10,
                        bottom: 10,
                    })
                    .show(ui, |ui| {
                        ui.set_width(popup_w - 24.0);
                        render_history_popup_body(
                            ui,
                            palette,
                            &self.history,
                            &mut self.history_open,
                        );
                    })
                    .response
            })
            .response;

        if allow_outside_close && response.clicked_elsewhere() {
            self.history_open = false;
        }
    }
}

fn render_history_popup_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    history: &VecDeque<NotificationRecord>,
    history_open: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Recent notifications")
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (close_rect, close_resp) =
                ui.allocate_exact_size(CLOSE_BTN_SIZE, egui::Sense::click());
            if close_resp.clicked() {
                *history_open = false;
            }
            let close_color = if close_resp.hovered() {
                redesign_text_primary(palette)
            } else {
                redesign_text_muted(palette)
            };
            paint_close_icon(ui.painter(), close_rect, close_color);
        });
    });

    ui.add_space(6.0);
    if history.is_empty() {
        ui.label(
            egui::RichText::new("No notifications yet.")
                .size(11.0)
                .color(redesign_text_muted(palette)),
        );
    } else {
        for record in history.iter().rev() {
            render_history_row(ui, record, palette);
            ui.add_space(4.0);
        }
    }
}

fn toast_options_for(kind: ToastKind) -> ToastOptions {
    let base = ToastOptions::default().show_icon(false);
    match kind {
        // show_progress drives per-frame repaints so hover-pause is sampled
        // every frame rather than only on the scheduled ttl tick.
        ToastKind::Success => base.show_progress(true).duration_in_seconds(TTL_SUCCESS),
        ToastKind::Info => base.show_progress(true).duration_in_seconds(TTL_INFO),
        ToastKind::Warning => base.show_progress(true).duration_in_seconds(TTL_WARNING),
        // Persistent toasts have no TTL; show_progress(false) avoids an
        // infinite repaint loop.
        ToastKind::Error | ToastKind::Custom(_) => {
            base.show_progress(false).duration(None::<Duration>)
        }
    }
}

const fn accent_color_for(kind: ToastKind, palette: ThemePalette) -> egui::Color32 {
    match kind {
        ToastKind::Success => redesign_success(palette),
        ToastKind::Info => redesign_info(palette),
        ToastKind::Warning => redesign_warning(palette),
        ToastKind::Error | ToastKind::Custom(_) => redesign_error_emphasis(palette),
    }
}

const fn text_color_for(kind: ToastKind, palette: ThemePalette) -> egui::Color32 {
    match kind {
        ToastKind::Success => redesign_success(palette),
        ToastKind::Info => redesign_info(palette),
        ToastKind::Warning => redesign_warning(palette),
        ToastKind::Error | ToastKind::Custom(_) => redesign_error(palette),
    }
}

fn render_custom_toast(
    ui: &mut egui::Ui,
    toast: &mut Toast,
    palette: ThemePalette,
) -> egui::Response {
    let accent = accent_color_for(toast.kind, palette);
    let text_color = text_color_for(toast.kind, palette);

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .shadow(redesign_overlay_shadow(palette))
        .inner_margin(egui::Margin {
            left: ACCENT_STRIPE_LEFT,
            right: TOAST_PADDING_H,
            top: TOAST_PADDING_V,
            bottom: TOAST_PADDING_V,
        });

    let outer = frame
        .show(ui, |ui| {
            ui.set_width(TOAST_BODY_WIDTH);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                render_toast_icon(ui, toast.kind, accent);
                // Reserve the close column and force the label to fill the rest so
                // the ✕ pins to the far-right edge instead of trailing the text;
                // the label wraps within its column.
                let gap = ui.spacing().item_spacing.x;
                let label_w = (ui.available_width() - CLOSE_BTN_SIZE.x - gap - gap).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(label_w, CLOSE_BTN_SIZE.y),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_width(label_w);
                        ui.label(
                            egui::RichText::new(toast.text.text())
                                .size(12.0)
                                .family(egui::FontFamily::Name("poppins_medium".into()))
                                .color(text_color),
                        );
                    },
                );
                let (close_rect, close_resp) =
                    ui.allocate_exact_size(CLOSE_BTN_SIZE, egui::Sense::click());
                if close_resp.clicked() {
                    toast.close();
                }
                let close_color = if close_resp.hovered() {
                    redesign_text_primary(palette)
                } else {
                    redesign_text_muted(palette)
                };
                paint_close_icon(ui.painter(), close_rect, close_color);
            });
        })
        .response;

    let stripe_rect = egui::Rect::from_min_size(
        outer.rect.min,
        egui::vec2(ACCENT_STRIPE_WIDTH, outer.rect.height()),
    );
    let corner = egui::CornerRadius {
        nw: REDESIGN_BORDER_RADIUS_U8,
        sw: REDESIGN_BORDER_RADIUS_U8,
        ne: 0,
        se: 0,
    };
    ui.painter()
        .with_clip_rect(outer.rect)
        .rect_filled(stripe_rect, corner, accent);

    outer
}

fn render_toast_icon(ui: &mut egui::Ui, kind: ToastKind, accent: egui::Color32) {
    match kind {
        ToastKind::Success => {
            ui.label(
                egui::RichText::new("\u{2713}")
                    .size(ICON_SIZE)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(accent),
            );
        }
        ToastKind::Warning => {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            paint_warning_icon(ui.painter(), rect.center(), accent);
        }
        ToastKind::Info => {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            paint_info_icon(ui.painter(), rect.center(), accent);
        }
        ToastKind::Error | ToastKind::Custom(_) => {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            paint_error_icon(ui.painter(), rect.center(), accent);
        }
    }
}

fn render_history_row(ui: &mut egui::Ui, record: &NotificationRecord, palette: ThemePalette) {
    let accent = accent_color_for(record.kind, palette);
    let now = chrono::Utc::now();
    let elapsed = record.added_at.elapsed();
    let secs = i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX);
    let ts = crate::ui::shared::format_relative::relative_time_from(
        now - chrono::Duration::try_seconds(secs).unwrap_or_default(),
        now,
    );

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        render_history_icon(ui, record.kind, accent);
        // Reserve a fixed timestamp column on the right; the message fills the
        // rest and wraps instead of running under the timestamp.
        let gap = ui.spacing().item_spacing.x;
        let ts_w = 52.0_f32;
        let msg_w = (ui.available_width() - ts_w - gap - gap).max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(msg_w, ICON_SIZE),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_min_width(msg_w);
                ui.label(
                    egui::RichText::new(&record.text)
                        .size(11.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                );
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(ts_w, ICON_SIZE),
            egui::Layout::top_down(egui::Align::RIGHT),
            |ui| {
                ui.label(
                    egui::RichText::new(&ts)
                        .size(10.0)
                        .color(redesign_text_muted(palette)),
                );
            },
        );
    });
}

fn render_history_icon(ui: &mut egui::Ui, kind: ToastKind, accent: egui::Color32) {
    match kind {
        ToastKind::Success => {
            ui.label(
                egui::RichText::new("\u{2713}")
                    .size(11.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(accent),
            );
        }
        ToastKind::Warning => {
            let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            paint_warning_icon(ui.painter(), r.center(), accent);
        }
        ToastKind::Info => {
            let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            paint_info_icon(ui.painter(), r.center(), accent);
        }
        ToastKind::Error | ToastKind::Custom(_) => {
            let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            paint_error_icon(ui.painter(), r.center(), accent);
        }
    }
}

/// Vector-paint a warning triangle (outline + stem dot) in the given color.
fn paint_warning_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5, color);
    let hw = 5.0_f32;
    let top = center.y - 5.0;
    let base_y = center.y + 4.0;
    painter.add(egui::Shape::closed_line(
        vec![
            egui::pos2(center.x, top),
            egui::pos2(center.x + hw, base_y),
            egui::pos2(center.x - hw, base_y),
        ],
        stroke,
    ));
    painter.line_segment(
        [
            egui::pos2(center.x, center.y - 1.5),
            egui::pos2(center.x, center.y + 1.0),
        ],
        stroke,
    );
    painter.circle_filled(egui::pos2(center.x, center.y + 2.5), 0.8, color);
}

/// Vector-paint a filled-circle + vertical bar info icon.
fn paint_info_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    painter.circle_filled(center, 5.5, color);
    let ink = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 210);
    let stroke = egui::Stroke::new(1.4, ink);
    painter.circle_filled(egui::pos2(center.x, center.y - 2.5), 0.9, ink);
    painter.line_segment(
        [
            egui::pos2(center.x, center.y - 0.5),
            egui::pos2(center.x, center.y + 3.0),
        ],
        stroke,
    );
}

/// Vector-paint a filled-circle + exclamation-bar error icon.
fn paint_error_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    painter.circle_filled(center, 5.5, color);
    let ink = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 210);
    let stroke = egui::Stroke::new(1.4, ink);
    painter.line_segment(
        [
            egui::pos2(center.x, center.y - 3.0),
            egui::pos2(center.x, center.y + 1.0),
        ],
        stroke,
    );
    painter.circle_filled(egui::pos2(center.x, center.y + 2.5), 0.9, ink);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_color_differs_from_error_across_palettes() {
        for palette in [ThemePalette::Dark, ThemePalette::Light] {
            let s = accent_color_for(ToastKind::Success, palette);
            let e = accent_color_for(ToastKind::Error, palette);
            assert_ne!(s, e, "success and error must differ for {palette:?}");
        }
    }

    #[test]
    fn info_color_differs_from_warning_across_palettes() {
        for palette in [ThemePalette::Dark, ThemePalette::Light] {
            let i = accent_color_for(ToastKind::Info, palette);
            let w = accent_color_for(ToastKind::Warning, palette);
            assert_ne!(i, w, "info and warning must differ for {palette:?}");
        }
    }

    #[test]
    fn all_four_severities_distinct_dark() {
        let p = ThemePalette::Dark;
        let colors = [
            accent_color_for(ToastKind::Success, p),
            accent_color_for(ToastKind::Info, p),
            accent_color_for(ToastKind::Warning, p),
            accent_color_for(ToastKind::Error, p),
        ];
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i], colors[j],
                    "severity colors[{i}] and [{j}] must differ"
                );
            }
        }
    }

    #[test]
    fn all_four_severities_distinct_light() {
        let p = ThemePalette::Light;
        let colors = [
            accent_color_for(ToastKind::Success, p),
            accent_color_for(ToastKind::Info, p),
            accent_color_for(ToastKind::Warning, p),
            accent_color_for(ToastKind::Error, p),
        ];
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i], colors[j],
                    "severity colors[{i}] and [{j}] must differ"
                );
            }
        }
    }

    #[test]
    fn history_caps_at_five_with_fifo_eviction() {
        let mut mgr = NotificationManager::new();
        for i in 0..7_usize {
            mgr.success(format!("msg{i}"));
        }
        assert_eq!(mgr.history().len(), HISTORY_CAP);
        assert_eq!(mgr.history().front().unwrap().text, "msg2");
        assert_eq!(mgr.history().back().unwrap().text, "msg6");
    }

    #[test]
    fn error_ttl_infinite_progress_is_zero() {
        let opts = toast_options_for(ToastKind::Error);
        assert!(
            (opts.progress() - 0.0_f64).abs() < f64::EPSILON,
            "error toast must have zero progress (infinite TTL)"
        );
    }

    #[test]
    fn timed_severities_have_nonzero_progress() {
        for kind in [ToastKind::Success, ToastKind::Info, ToastKind::Warning] {
            let opts = toast_options_for(kind);
            assert!(
                opts.progress() > 0.0,
                "{kind:?} must have non-zero initial progress (finite TTL)"
            );
        }
    }
}
