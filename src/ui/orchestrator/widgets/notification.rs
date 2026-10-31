// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use eframe::egui;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};

use crate::ui::orchestrator::widgets::icon_button::paint_close_icon;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, redesign_window_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX,
    REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_border_soft, redesign_border_strong,
    redesign_error, redesign_error_emphasis, redesign_info, redesign_shell_bg, redesign_success,
    redesign_text_muted, redesign_text_primary, redesign_warning,
};
use crate::ui::shared::redesign_visuals::redesign_overlay_shadow;

const HISTORY_CAP: usize = 5;
const KIND_LANGUAGE_RECOVERY: u32 = 2;
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

const TOAST_PREVIEW_LINES: usize = 6;
const HISTORY_PREVIEW_LINES: usize = 3;
const PREVIEW_LINE_CHARS: usize = 160;
const TOAST_MAX_ROWS: usize = 8;
const HISTORY_MAX_ROWS: usize = 4;

pub const KIND_PROGRESS: u32 = 0;
pub const KIND_CUSTOM: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageRecoveryRow {
    pub tab: String,
    pub original: crate::registry::workspace_model::ComponentRef,
    pub available_language: Option<u8>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageRecoveryNotice {
    pub owner: String,
    pub generation: u64,
    pub rows: Vec<LanguageRecoveryRow>,
}

impl LanguageRecoveryNotice {
    fn text(&self) -> String {
        let mut lines = Vec::new();
        if self.rows.iter().any(|row| row.available_language.is_some()) {
            lines.push("Some saved selections use a different language".to_string());
            lines.push("An older BIO version could save the wrong language for selected components. These components were found, but their saved language differs from the current scan.".to_string());
            for row in self
                .rows
                .iter()
                .filter(|row| row.available_language.is_some())
            {
                if let Some(language) = row.available_language {
                    lines.push(format!(
                        "{} / {} #{}: saved language {}, available language {}",
                        row.tab, row.original.tp2, row.original.id, row.original.language, language
                    ));
                }
            }
        }
        if self.rows.iter().any(|row| row.available_language.is_none()) {
            lines.push("Other unresolved selections".to_string());
            for row in self
                .rows
                .iter()
                .filter(|row| row.available_language.is_none())
            {
                lines.push(format!(
                    "{} / {} #{}: {}",
                    row.tab, row.original.tp2, row.original.id, row.reason
                ));
            }
        }
        lines.push(
            "Unresolved selections stay saved. You can keep editing and return after another scan."
                .to_string(),
        );
        if self
            .rows
            .iter()
            .any(|row| row.original.wlb_inputs.is_some())
        {
            lines.push("Previous installer answers are kept, but must be reviewed or entered again in Step 3 after changing language.".to_string());
        }
        lines.join("\n")
    }
}

#[derive(Clone)]
pub struct NotificationRecord {
    pub kind: ToastKind,
    pub text: String,
    pub added_at: Instant,
    recovery: Option<LanguageRecoveryNotice>,
}

#[derive(Clone)]
struct ViewAllRequest {
    kind: ToastKind,
    text: String,
    recovery: Option<LanguageRecoveryNotice>,
}

pub(crate) fn toasts_id() -> egui::Id {
    egui::Id::new("bio_toasts")
}

fn view_all_request_id() -> egui::Id {
    egui::Id::new("notification_view_all_request")
}

struct TextPreview {
    text: String,
    hidden_lines: usize,
    truncated: bool,
}

fn preview_text(full: &str, max_lines: usize, max_chars: usize) -> TextPreview {
    let full = full.trim_end_matches('\n');
    let all_lines: Vec<&str> = full.split('\n').collect();
    let total_lines = all_lines.len();
    let mut cut_any_line = false;
    let mut kept_lines: Vec<String> = Vec::new();
    for line in all_lines.into_iter().take(max_lines) {
        let char_count = line.chars().count();
        if char_count > max_chars {
            cut_any_line = true;
            let cut: String = line.chars().take(max_chars.saturating_sub(1)).collect();
            kept_lines.push(format!("{cut}\u{2026}"));
        } else {
            kept_lines.push(line.to_string());
        }
    }
    let hidden_lines = total_lines.saturating_sub(kept_lines.len());
    let truncated = hidden_lines > 0 || cut_any_line;
    TextPreview {
        text: kept_lines.join("\n"),
        hidden_lines,
        truncated,
    }
}

fn preview_galley(
    ui: &egui::Ui,
    text: &str,
    max_rows: usize,
    width: f32,
    size: f32,
    color: egui::Color32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(
        text.to_string(),
        egui::FontId::new(size, egui::FontFamily::Name("poppins_medium".into())),
        color,
        width,
    );
    job.wrap.overflow_character = Some('\u{2026}');
    job.wrap.max_rows = max_rows;
    ui.fonts(|f| f.layout_job(job))
}

fn lines_hidden(preview: &TextPreview, galley: &egui::Galley) -> usize {
    if !galley.elided {
        return preview.hidden_lines;
    }
    let row_breaks = galley
        .rows
        .iter()
        .rev()
        .skip(1)
        .filter(|row| row.ends_with_newline)
        .count();
    hidden_after_row_breaks(preview, row_breaks)
}

fn hidden_after_row_breaks(preview: &TextPreview, row_breaks: usize) -> usize {
    let kept_lines = preview.text.split('\n').count();
    preview.hidden_lines + kept_lines.saturating_sub(row_breaks + 1)
}

#[derive(Default)]
pub struct NotificationManager {
    pending: Vec<(ToastKind, String, bool)>,
    closing: Vec<String>,
    slots: HashMap<&'static str, String>,
    history: VecDeque<NotificationRecord>,
    pub history_open: bool,
    view_all: Option<ViewAllRequest>,
    recovery: Option<LanguageRecoveryNotice>,
    recovery_replaced: bool,
    recovery_enabled: bool,
    recovery_action: Option<LanguageRecoveryNotice>,
}

impl NotificationManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn success(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Success, text.into(), false);
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Info, text.into(), false);
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Warning, text.into(), false);
    }

    pub fn warn_persistent(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Warning, text.into(), true);
    }

    pub(crate) fn warn_language_recovery(&mut self, notice: LanguageRecoveryNotice) {
        self.pending
            .retain(|(kind, _, _)| *kind != ToastKind::Custom(KIND_LANGUAGE_RECOVERY));
        self.recovery_replaced = true;
        self.push(
            ToastKind::Custom(KIND_LANGUAGE_RECOVERY),
            notice.text(),
            true,
        );
        if let Some(record) = self.history.back_mut() {
            record.recovery = Some(notice.clone());
        }
        self.recovery = Some(notice);
    }

    pub(crate) fn set_language_recovery_enabled(
        &mut self,
        owner: &str,
        generation: u64,
        scanning: bool,
    ) {
        self.recovery_enabled = !scanning
            && self
                .view_all
                .as_ref()
                .and_then(|request| request.recovery.as_ref())
                .is_some_and(|notice| notice.owner == owner && notice.generation == generation);
    }

    pub(crate) const fn take_language_recovery_action(&mut self) -> Option<LanguageRecoveryNotice> {
        self.recovery_action.take()
    }

    pub fn replace_persistent_warning(&mut self, slot: &'static str, text: Option<String>) {
        if let Some(previous) = self.slots.remove(slot) {
            self.pending.retain(|(_, pending, _)| *pending != previous);
            self.closing.push(previous);
        }
        if let Some(text) = text {
            self.slots.insert(slot, text.clone());
            self.warn_persistent(text);
        }
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Error, text.into(), false);
    }

    fn push(&mut self, kind: ToastKind, text: String, persistent: bool) {
        self.pending.push((kind, text.clone(), persistent));
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(NotificationRecord {
            kind,
            text,
            added_at: Instant::now(),
            recovery: None,
        });
    }

    #[must_use]
    pub const fn history(&self) -> &VecDeque<NotificationRecord> {
        &self.history
    }

    pub fn show(&mut self, ctx: &egui::Context, palette: ThemePalette) {
        self.close_replaced(ctx);
        if std::mem::take(&mut self.recovery_replaced) {
            ctx.data_mut(|d| {
                for toast in d.get_temp_mut_or_default::<Vec<Toast>>(toasts_id()) {
                    if toast.kind == ToastKind::Custom(KIND_LANGUAGE_RECOVERY) {
                        toast.close();
                    }
                }
            });
        }
        let recovery = self.recovery.clone();
        let toasts = Toasts::with_id(toasts_id())
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

        let mut toasts = toasts.custom_contents(KIND_LANGUAGE_RECOVERY, move |ui, toast| {
            render_toast_with_recovery(ui, toast, palette, recovery.as_ref())
        });
        for (kind, text, persistent) in self.pending.drain(..) {
            let options = if persistent {
                ToastOptions::default()
                    .show_icon(false)
                    .show_progress(false)
                    .duration(None::<Duration>)
            } else {
                toast_options_for(kind)
            };
            toasts.add(Toast::new().kind(kind).text(text).options(options));
        }

        toasts.show(ctx);

        let id = view_all_request_id();
        let request = ctx.data_mut(|d| {
            let value = d.get_temp::<ViewAllRequest>(id);
            if value.is_some() {
                d.remove::<ViewAllRequest>(id);
            }
            value
        });
        if let Some(request) = request {
            self.view_all = Some(request);
        }
        self.render_view_all_window(ctx, palette);
    }

    fn close_replaced(&mut self, ctx: &egui::Context) {
        if self.closing.is_empty() {
            return;
        }
        let closing = std::mem::take(&mut self.closing);
        ctx.data_mut(|d| {
            for toast in d.get_temp_mut_or_default::<Vec<Toast>>(toasts_id()) {
                if closing.iter().any(|text| toast.text.text() == text) {
                    toast.close();
                }
            }
        });
    }

    #[must_use]
    pub fn has_history(&self) -> bool {
        !self.history.is_empty()
    }

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
                            &mut self.view_all,
                        );
                    })
                    .response
            })
            .response;

        if allow_outside_close && response.clicked_elsewhere() {
            self.history_open = false;
        }
    }

    pub(crate) fn guard_language_recovery_escape(&mut self, ctx: &egui::Context) {
        if self
            .view_all
            .as_ref()
            .is_some_and(|request| request.recovery.is_some())
            && !ctx.memory(egui::Memory::any_popup_open)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.view_all = None;
        }
    }

    fn render_view_all_window(&mut self, ctx: &egui::Context, palette: ThemePalette) {
        let Some(request) = self.view_all.as_ref() else {
            return;
        };

        if let Some(notice) = request.recovery.as_ref() {
            self.history_open = false;
            let (restore, close) =
                render_recovery_drawer(ctx, palette, notice, self.recovery_enabled, &request.text);
            if restore {
                self.recovery_action = Some(notice.clone());
            }
            if restore || close {
                self.view_all = None;
            }
            return;
        }

        let title = match request.kind {
            ToastKind::Success => "Success",
            ToastKind::Info => "Info",
            ToastKind::Warning | ToastKind::Custom(KIND_LANGUAGE_RECOVERY) => "Warning",
            ToastKind::Error | ToastKind::Custom(_) => "Error",
        };
        let text = request.text.as_str();
        let recovery = request.recovery.clone();
        let mut restore_clicked = false;

        let mut open = true;
        let mut close_clicked = false;
        let max_h = (ctx.screen_rect().height() * 0.6).max(120.0);
        egui::Window::new(redesign_window_title(palette, title))
            .id(egui::Id::new("notification_view_all_window"))
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(560.0)
            .order(egui::Order::Foreground)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(max_h)
                    .min_scrolled_height(max_h)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(text)
                                    .size(12.0)
                                    .family(egui::FontFamily::Name("poppins_medium".into()))
                                    .color(redesign_text_primary(palette)),
                            )
                            .selectable(true)
                            .wrap(),
                        );
                    });
                if let Some(notice) = recovery.as_ref() {
                    (restore_clicked, close_clicked) =
                        render_recovery_footer(ui, palette, notice, self.recovery_enabled, text);
                } else {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if redesign_btn(
                                ui,
                                palette,
                                "Close",
                                BtnOpts {
                                    primary: true,
                                    small: true,
                                    ..Default::default()
                                },
                            )
                            .clicked()
                            {
                                close_clicked = true;
                            }
                            if redesign_btn(
                                ui,
                                palette,
                                "Copy",
                                BtnOpts {
                                    small: true,
                                    ..Default::default()
                                },
                            )
                            .clicked()
                            {
                                ui.ctx().copy_text(text.to_string());
                            }
                        });
                    });
                }
            });

        if restore_clicked {
            self.recovery_action = recovery;
        }
        if !open || close_clicked || restore_clicked {
            self.view_all = None;
        }
    }
}

fn render_recovery_drawer(
    ctx: &egui::Context,
    palette: ThemePalette,
    notice: &LanguageRecoveryNotice,
    enabled: bool,
    text: &str,
) -> (bool, bool) {
    let screen = ctx.screen_rect();
    let panel = egui::Rect::from_min_max(
        screen.min + egui::vec2(0.0, REDESIGN_TITLEBAR_HEIGHT_PX),
        screen.max,
    );
    let width = super::drawer::drawer_width(screen.width(), super::drawer::DrawerWidth::Content)
        .min(screen.width());
    let popup_open = ctx.memory(egui::Memory::any_popup_open);
    let scrim_id = egui::Id::new("language_recovery_scrim");
    let scrim_layer = egui::LayerId::new(egui::Order::Foreground, scrim_id);
    ctx.move_to_top(scrim_layer);
    let scrim_clicked = egui::Area::new(scrim_id)
        .order(egui::Order::Foreground)
        .fixed_pos(panel.min)
        .show(ctx, |ui| {
            let response = ui.allocate_rect(panel, egui::Sense::click());
            ui.painter()
                .rect_filled(panel, 0.0, egui::Color32::from_black_alpha(148));
            response.clicked()
        })
        .inner;
    let drawer_id = egui::Id::new("language_recovery_drawer");
    ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, drawer_id));
    let mut restore = false;
    let mut close = false;
    egui::Area::new(drawer_id)
        .constrain(false)
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(panel.right() - width, panel.top()))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .shadow(redesign_overlay_shadow(palette))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(width, panel.height()));
                    ui.set_max_size(egui::vec2(width, panel.height()));
                    let bounds = egui::Rect::from_min_size(
                        ui.min_rect().min,
                        egui::vec2(width, panel.height()),
                    );
                    ui.painter().line_segment(
                        [bounds.left_top(), bounds.left_bottom()],
                        egui::Stroke::new(1.0_f32, redesign_border_strong(palette)),
                    );
                    let mut header = ui.new_child(egui::UiBuilder::new().max_rect(bounds));
                    close = render_recovery_header(&mut header, palette, notice);
                    let header_bottom = header.min_rect().bottom();
                    let content_width = (width - 44.0).max(1.0);
                    let footer_height = recovery_footer_height(ui, content_width);
                    let footer_top = bounds.bottom() - footer_height;
                    let list_rect = egui::Rect::from_min_max(
                        egui::pos2(bounds.left() + 22.0, header_bottom + 18.0),
                        egui::pos2(
                            bounds.right() - 22.0,
                            (footer_top - 18.0).max(header_bottom + 18.0),
                        ),
                    );
                    let mut list = ui.new_child(egui::UiBuilder::new().max_rect(list_rect));
                    list.set_clip_rect(list_rect);
                    egui::ScrollArea::vertical()
                        .id_salt("language_recovery_rows")
                        .auto_shrink([false, false])
                        .max_height(list_rect.height())
                        .show(&mut list, |ui| render_recovery_rows(ui, palette, notice));
                    for y in [header_bottom, footer_top] {
                        ui.painter().hline(
                            bounds.x_range(),
                            y,
                            egui::Stroke::new(1.0_f32, redesign_border_soft(palette)),
                        );
                    }
                    let footer_rect = egui::Rect::from_min_max(
                        egui::pos2(bounds.left() + 22.0, footer_top + 12.0),
                        bounds.right_bottom() - egui::vec2(22.0, 12.0),
                    );
                    let mut footer = ui.new_child(egui::UiBuilder::new().max_rect(footer_rect));
                    let (clicked, dismissed) =
                        render_recovery_footer(&mut footer, palette, notice, enabled, text);
                    restore = clicked;
                    close |= dismissed;
                });
        });
    close |= scrim_clicked
        || (!popup_open
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)));
    (restore, close)
}

fn recovery_footer_height(ui: &egui::Ui, width: f32) -> f32 {
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let mut row_width = 0.0;
    let mut height = 0.0;
    let mut row_height: f32 = 0.0;
    for label in ["Restore selections", "Copy", "Keep unresolved for now"] {
        let size = ui
            .painter()
            .layout_no_wrap(label.to_string(), font.clone(), egui::Color32::WHITE)
            .size()
            + egui::vec2(20.0, 8.0);
        if row_width > 0.0 && row_width + 8.0 + size.x > width {
            height += row_height + 8.0;
            row_width = 0.0;
            row_height = 0.0;
        }
        row_width += if row_width > 0.0 {
            8.0 + size.x
        } else {
            size.x
        };
        row_height = row_height.max(size.y);
    }
    height + row_height + 24.0
}

fn recovery_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str, size: f32, muted: bool) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(size)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(if muted {
                    redesign_text_muted(palette)
                } else {
                    redesign_text_primary(palette)
                }),
        )
        .wrap()
        .selectable(true),
    );
}

fn render_recovery_header(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    notice: &LanguageRecoveryNotice,
) -> bool {
    let eligible = notice
        .rows
        .iter()
        .any(|row| row.available_language.is_some());
    let mut close = false;
    egui::Frame::default().inner_margin(egui::Margin { left: 22, right: 22, top: 18, bottom: 12 }).show(ui, |ui| {
        ui.horizontal_top(|ui| {
            let title_width = (ui.available_width() - CLOSE_BTN_SIZE.x - 8.0).max(1.0);
            ui.allocate_ui_with_layout(egui::vec2(title_width, 18.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_min_width(title_width);
                recovery_label(ui, palette, if eligible { "Some saved selections use a different language" } else { "Other unresolved selections" }, 17.0, false);
            });
            let (rect, response) = ui.allocate_exact_size(CLOSE_BTN_SIZE, egui::Sense::click());
            paint_close_icon(ui.painter(), rect, redesign_text_muted(palette));
            close = response.on_hover_text("Close").clicked();
        });
        if eligible {
            recovery_label(ui, palette, "An older BIO version could save the wrong language for selected components. These components were found, but their saved language differs from the current scan.", 12.0, true);
        }
        recovery_label(ui, palette, "Unresolved selections stay saved. You can keep editing and return after another scan.", 12.0, true);
        if notice.rows.iter().any(|row| row.original.wlb_inputs.is_some()) {
            recovery_label(ui, palette, "Previous installer answers are kept, but must be reviewed or entered again in Step 3 after changing language.", 12.0, true);
        }
    });
    close
}

fn render_recovery_rows(ui: &mut egui::Ui, palette: ThemePalette, notice: &LanguageRecoveryNotice) {
    for row in notice
        .rows
        .iter()
        .filter(|row| row.available_language.is_some())
    {
        if let Some(language) = row.available_language {
            recovery_label(
                ui,
                palette,
                &format!(
                    "{} / {} #{}: saved language {}, available language {}",
                    row.tab, row.original.tp2, row.original.id, row.original.language, language
                ),
                12.0,
                false,
            );
        }
    }
    if notice
        .rows
        .iter()
        .any(|row| row.available_language.is_none())
    {
        if notice
            .rows
            .iter()
            .any(|row| row.available_language.is_some())
        {
            ui.add_space(12.0);
            recovery_label(ui, palette, "Other unresolved selections", 12.0, false);
        }
        for row in notice
            .rows
            .iter()
            .filter(|row| row.available_language.is_none())
        {
            recovery_label(
                ui,
                palette,
                &format!(
                    "{} / {} #{}: {}",
                    row.tab, row.original.tp2, row.original.id, row.reason
                ),
                12.0,
                false,
            );
        }
    }
}

fn render_recovery_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    notice: &LanguageRecoveryNotice,
    enabled: bool,
    text: &str,
) -> (bool, bool) {
    let mut restore_clicked = false;
    let mut close_clicked = false;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        ui.horizontal_wrapped(|ui| {
            restore_clicked = render_recovery_button(ui, palette, Some(notice), enabled);
            if redesign_btn(
                ui,
                palette,
                "Copy",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                ui.ctx().copy_text(text.to_string());
            }
            if redesign_btn(
                ui,
                palette,
                "Keep unresolved for now",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                close_clicked = true;
            }
        });
    });
    (restore_clicked, close_clicked)
}

fn render_recovery_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    notice: Option<&LanguageRecoveryNotice>,
    enabled: bool,
) -> bool {
    let Some(notice) = notice else {
        return false;
    };
    let eligible = notice
        .rows
        .iter()
        .any(|row| row.available_language.is_some());
    ui.add_enabled_ui(enabled && eligible, |ui| {
        redesign_btn(
            ui,
            palette,
            "Restore selections",
            BtnOpts {
                primary: true,
                small: true,
                ..Default::default()
            },
        )
        .on_hover_text("Restore selections using current scanned language")
        .on_disabled_hover_text("Restore selections using current scanned language")
        .clicked()
    })
    .inner
}

fn render_history_popup_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    history: &VecDeque<NotificationRecord>,
    history_open: &mut bool,
    view_all: &mut Option<ViewAllRequest>,
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
            render_history_row(ui, record, palette, view_all);
            ui.add_space(4.0);
        }
    }
}

fn toast_options_for(kind: ToastKind) -> ToastOptions {
    let base = ToastOptions::default().show_icon(false);
    match kind {
        ToastKind::Success => base.show_progress(true).duration_in_seconds(TTL_SUCCESS),
        ToastKind::Info => base.show_progress(true).duration_in_seconds(TTL_INFO),
        ToastKind::Warning | ToastKind::Custom(KIND_LANGUAGE_RECOVERY) => {
            base.show_progress(true).duration_in_seconds(TTL_WARNING)
        }
        ToastKind::Error | ToastKind::Custom(_) => {
            base.show_progress(false).duration(None::<Duration>)
        }
    }
}

const fn accent_color_for(kind: ToastKind, palette: ThemePalette) -> egui::Color32 {
    match kind {
        ToastKind::Success => redesign_success(palette),
        ToastKind::Info => redesign_info(palette),
        ToastKind::Warning | ToastKind::Custom(KIND_LANGUAGE_RECOVERY) => redesign_warning(palette),
        ToastKind::Error | ToastKind::Custom(_) => redesign_error_emphasis(palette),
    }
}

const fn text_color_for(kind: ToastKind, palette: ThemePalette) -> egui::Color32 {
    match kind {
        ToastKind::Success => redesign_success(palette),
        ToastKind::Info => redesign_info(palette),
        ToastKind::Warning | ToastKind::Custom(KIND_LANGUAGE_RECOVERY) => redesign_warning(palette),
        ToastKind::Error | ToastKind::Custom(_) => redesign_error(palette),
    }
}

fn render_custom_toast(
    ui: &mut egui::Ui,
    toast: &mut Toast,
    palette: ThemePalette,
) -> egui::Response {
    render_toast_with_recovery(ui, toast, palette, None)
}

fn render_toast_with_recovery(
    ui: &mut egui::Ui,
    toast: &mut Toast,
    palette: ThemePalette,
    recovery: Option<&LanguageRecoveryNotice>,
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
                let gap = ui.spacing().item_spacing.x;
                let label_w = (ui.available_width() - CLOSE_BTN_SIZE.x - gap - gap).max(0.0);
                let kind = toast.kind;
                let toast_ctx = ui.ctx().clone();
                let preview =
                    preview_text(toast.text.text(), TOAST_PREVIEW_LINES, PREVIEW_LINE_CHARS);
                ui.allocate_ui_with_layout(
                    egui::vec2(label_w, CLOSE_BTN_SIZE.y),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_width(label_w);
                        let galley = preview_galley(
                            ui,
                            &preview.text,
                            TOAST_MAX_ROWS,
                            label_w,
                            12.0,
                            text_color,
                        );
                        let elided = galley.elided;
                        let hidden = lines_hidden(&preview, &galley);
                        ui.label(galley);
                        if recovery.is_some() || preview.truncated || elided {
                            render_preview_footer(ui, palette, 11.0, text_color, hidden, || {
                                let id = view_all_request_id();
                                let request = ViewAllRequest {
                                    kind,
                                    text: toast.text.text().to_string(),
                                    recovery: recovery.cloned(),
                                };
                                toast_ctx.data_mut(|d| d.insert_temp(id, request));
                            });
                        }
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

fn render_preview_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    size: f32,
    view_all_color: egui::Color32,
    hidden_lines: usize,
    mut on_view_all: impl FnMut(),
) {
    ui.horizontal(|ui| {
        if hidden_lines > 0 {
            ui.label(
                egui::RichText::new(format!("…and {hidden_lines} more"))
                    .size(size)
                    .color(redesign_text_muted(palette)),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let resp = ui.add(
                egui::Label::new(
                    egui::RichText::new("View all")
                        .size(size)
                        .color(view_all_color),
                )
                .sense(egui::Sense::click()),
            );
            if resp.hovered() {
                let y = resp.rect.bottom() - 1.0;
                ui.painter().line_segment(
                    [
                        egui::pos2(resp.rect.left(), y),
                        egui::pos2(resp.rect.right(), y),
                    ],
                    egui::Stroke::new(1.0_f32, view_all_color),
                );
            }
            if resp.clicked() {
                on_view_all();
            }
        });
    });
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
        ToastKind::Warning | ToastKind::Custom(KIND_LANGUAGE_RECOVERY) => {
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

fn render_history_row(
    ui: &mut egui::Ui,
    record: &NotificationRecord,
    palette: ThemePalette,
    view_all: &mut Option<ViewAllRequest>,
) {
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
        let gap = ui.spacing().item_spacing.x;
        let ts_w = 52.0_f32;
        let msg_w = (ui.available_width() - ts_w - gap - gap).max(0.0);
        let preview = preview_text(&record.text, HISTORY_PREVIEW_LINES, PREVIEW_LINE_CHARS);
        ui.allocate_ui_with_layout(
            egui::vec2(msg_w, ICON_SIZE),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_min_width(msg_w);
                let galley = preview_galley(
                    ui,
                    &preview.text,
                    HISTORY_MAX_ROWS,
                    msg_w,
                    11.0,
                    redesign_text_primary(palette),
                );
                let elided = galley.elided;
                let hidden = lines_hidden(&preview, &galley);
                ui.label(galley);
                if record.recovery.is_some() || preview.truncated || elided {
                    render_preview_footer(
                        ui,
                        palette,
                        10.0,
                        redesign_text_primary(palette),
                        hidden,
                        || {
                            *view_all = Some(ViewAllRequest {
                                kind: record.kind,
                                text: record.text.clone(),
                                recovery: record.recovery.clone(),
                            });
                        },
                    );
                }
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
        ToastKind::Warning | ToastKind::Custom(KIND_LANGUAGE_RECOVERY) => {
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

fn paint_warning_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
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

fn paint_info_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    painter.circle_filled(center, 5.5, color);
    let ink = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 210);
    let stroke = egui::Stroke::new(1.4_f32, ink);
    painter.circle_filled(egui::pos2(center.x, center.y - 2.5), 0.9, ink);
    painter.line_segment(
        [
            egui::pos2(center.x, center.y - 0.5),
            egui::pos2(center.x, center.y + 3.0),
        ],
        stroke,
    );
}

fn paint_error_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    painter.circle_filled(center, 5.5, color);
    let ink = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 210);
    let stroke = egui::Stroke::new(1.4_f32, ink);
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
    fn warn_persistent_queues_warning_kind_with_infinite_ttl() {
        let mut mgr = NotificationManager::new();
        mgr.warn_persistent("test persistent warning");
        let (kind, _, persistent) = &mgr.pending[0];
        assert_eq!(*kind, ToastKind::Warning);
        assert!(*persistent, "warn_persistent must set persistent flag");
        assert_eq!(mgr.history().back().unwrap().kind, ToastKind::Warning);
    }

    fn shown_toasts(ctx: &egui::Context) -> Vec<String> {
        ctx.data_mut(|d| d.get_temp::<Vec<Toast>>(toasts_id()))
            .unwrap_or_default()
            .iter()
            .map(|toast| toast.text.text().to_string())
            .collect()
    }

    fn show_a_frame(ctx: &egui::Context, mgr: &mut NotificationManager) {
        crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            mgr.show(ctx, ThemePalette::Dark);
        });
    }

    #[test]
    fn a_replaced_warning_closes_the_one_on_screen() {
        let ctx = egui::Context::default();
        let mut mgr = NotificationManager::new();
        mgr.warn_persistent("other");
        mgr.replace_persistent_warning("log", Some("first".to_string()));
        show_a_frame(&ctx, &mut mgr);
        assert_eq!(shown_toasts(&ctx), vec!["other", "first"]);

        mgr.replace_persistent_warning("log", Some("second".to_string()));
        show_a_frame(&ctx, &mut mgr);
        assert_eq!(shown_toasts(&ctx), vec!["other", "second"]);

        mgr.replace_persistent_warning("log", None);
        show_a_frame(&ctx, &mut mgr);
        assert_eq!(shown_toasts(&ctx), vec!["other"]);
    }

    #[test]
    fn a_warning_replaced_before_it_shows_never_shows() {
        let ctx = egui::Context::default();
        let mut mgr = NotificationManager::new();
        mgr.replace_persistent_warning("log", Some("first".to_string()));
        mgr.replace_persistent_warning("log", Some("second".to_string()));
        show_a_frame(&ctx, &mut mgr);
        assert_eq!(shown_toasts(&ctx), vec!["second"]);
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

    #[test]
    fn preview_keeps_short_text_whole() {
        let full = "line one\nline two\nline three";
        let preview = preview_text(full, 6, 160);
        assert_eq!(preview.text, full);
        assert_eq!(preview.hidden_lines, 0);
        assert!(!preview.truncated);
    }

    #[test]
    fn preview_hides_lines_past_the_cap() {
        let full = (0..45)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let preview = preview_text(&full, 6, 160);
        assert_eq!(preview.text.split('\n').count(), 6);
        assert_eq!(preview.hidden_lines, 39);
        assert!(preview.truncated);
    }

    #[test]
    fn hidden_count_includes_preview_lines_the_row_cap_cut() {
        let full = (0..45)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let preview = preview_text(&full, 6, 160);
        assert_eq!(hidden_after_row_breaks(&preview, 2), 42);
        assert_eq!(hidden_after_row_breaks(&preview, 5), 39);
    }

    #[test]
    fn preview_cuts_a_long_line_with_an_ellipsis() {
        let full = "a".repeat(400);
        let preview = preview_text(&full, 6, 160);
        assert_eq!(preview.text.chars().count(), 160);
        assert!(preview.text.ends_with('\u{2026}'));
        assert_eq!(preview.hidden_lines, 0);
        assert!(preview.truncated);
    }

    #[test]
    fn preview_counts_chars_not_bytes() {
        let full = "é".repeat(200);
        let preview = preview_text(&full, 6, 160);
        let chars: Vec<char> = preview.text.chars().collect();
        assert_eq!(chars.len(), 160);
        assert!(chars[..159].iter().all(|&c| c == 'é'));
        assert_eq!(chars[159], '\u{2026}');
    }

    #[test]
    fn preview_ignores_a_trailing_newline() {
        let full = "one\ntwo\nthree\nfour\nfive\nsix\n";
        let preview = preview_text(full, 6, 160);
        assert_eq!(preview.hidden_lines, 0);
        assert!(!preview.truncated);
    }
}

#[cfg(test)]
#[path = "notification_language_recovery_tests.rs"]
mod language_recovery_tests;
