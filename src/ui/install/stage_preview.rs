// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::preview_counts;
use crate::ui::install::preview_tabs;
use crate::ui::install::state_install::InstallScreenState;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn, SecondaryBtn};
use crate::ui::orchestrator::widgets::dialogs::fork_info_popup::{self, SelfNode};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_pill_danger, redesign_shadow, redesign_shell_bg, redesign_text_faint,
    redesign_text_muted, redesign_text_primary,
};

const FALLBACK_TITLE: &str = "Shared modlist";
const SHADOW_OFFSET_PX: i8 = 2;

const DRAFT_BANNER: &str = "Draft modlist code \u{2014} this is not from a verified install. \
Review and customize the components in Create \u{2192} Import and modify before installing.";

const DISABLED_IMPORT_TIP: &str =
    "Auto-install disabled for draft codes \u{2014} open in Create to review";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PreviewOutcome {
    #[default]
    Stay,
    Back,
    OpenInCreate,
    Advance,
}

pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    ctx: &egui::Context,
    state: &mut InstallScreenState,
) -> PreviewOutcome {
    if let Some(err) = state.preview_parse_error.clone() {
        return render_parse_error(ui, palette, &err);
    }

    let Some(preview) = state.parsed_preview.clone() else {
        return render_parse_error(
            ui,
            palette,
            "No preview is available. Go back and paste a share code.",
        );
    };

    let title = preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_TITLE);
    let subline = build_subline(preview.author.as_deref());
    let has_lineage = !preview.forked_from.is_empty();

    ui.horizontal_top(|ui| {
        let fork_btn_w = if has_lineage { 110.0 } else { 0.0 };
        let title_w = (ui.available_width() - fork_btn_w).max(120.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                render_screen_title(ui, palette, title, Some(&subline));
            },
        );
        if has_lineage {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.add_space(0.0);
                if fork_info_button(ui, palette).clicked() {
                    state.fork_info_open = true;
                }
            });
        }
    });

    let auto_install = preview.allow_auto_install;
    if !auto_install {
        draft_banner(ui, palette);
        ui.add_space(12.0);
    }

    render_preview_body(ui, palette, state, &preview);
    let outcome = render_preview_footer(ui, palette, auto_install);
    render_fork_popup(ctx, palette, state, &preview);

    outcome
}

fn build_subline(author: Option<&str>) -> String {
    let tail = "review what will be installed before BIO downloads anything";
    author
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| tail.to_string(), |a| format!("by {a} \u{00B7} {tail}"))
}

fn render_preview_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
    preview: &ModlistSharePreview,
) {
    overview_box(ui, palette, preview);
    ui.add_space(12.0);

    preview_tabs::render_tab_strip(ui, palette, &mut state.active_preview_tab);

    let content_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(80.0);
    let content_frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(14));
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), content_h),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            content_frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(content_h - 28.0);
                preview_tabs::render_tab_body(ui, palette, state.active_preview_tab, preview);
            });
        },
    );
}

fn render_preview_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    auto_install: bool,
) -> PreviewOutcome {
    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Back" }),
        if auto_install {
            None
        } else {
            Some(SecondaryBtn {
                label: "Open in Create",
            })
        },
        Some(if auto_install {
            "downloads, extracts, then runs install \u{2014} no review step"
        } else {
            DISABLED_IMPORT_TIP
        }),
        PrimaryBtn {
            label: "Import Modlist",
            disabled: !auto_install,
        },
    );

    if footer.back_clicked {
        PreviewOutcome::Back
    } else if footer.secondary_clicked {
        PreviewOutcome::OpenInCreate
    } else if footer.primary_clicked {
        PreviewOutcome::Advance
    } else {
        PreviewOutcome::Stay
    }
}

fn render_fork_popup(
    ctx: &egui::Context,
    palette: ThemePalette,
    state: &mut InstallScreenState,
    preview: &ModlistSharePreview,
) {
    if !state.fork_info_open {
        return;
    }

    let self_author = preview.author.as_deref().unwrap_or("").trim();
    let self_name = preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_TITLE);
    let result = fork_info_popup::render(
        ctx,
        palette,
        "install_preview",
        &preview.forked_from,
        &SelfNode {
            name: self_name,
            author: self_author,
        },
    );
    if result == fork_info_popup::ForkInfoOutcome::Closed {
        state.fork_info_open = false;
    }
}

fn overview_box(ui: &mut egui::Ui, palette: ThemePalette, p: &ModlistSharePreview) {
    redesign_box(ui, palette, None, |ui| {
        let total_w = ui.available_width();
        let col_gap = 16.0;
        let col_w = ((total_w - col_gap * 3.0) / 4.0).max(60.0);

        let components = p.bgee_entries + p.bg2ee_entries;
        let mods = preview_counts::distinct_mod_count(&p.bgee_log_text, &p.bg2ee_log_text);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = col_gap;
            overview_cell(ui, palette, col_w, "Game", &p.game_install);
            overview_cell(ui, palette, col_w, "Mods", &mods.to_string());
            overview_cell(ui, palette, col_w, "Components", &components.to_string());
            overview_cell(
                ui,
                palette,
                col_w,
                "BGEE/BG2EE entries",
                &format!("{}/{}", p.bgee_entries, p.bg2ee_entries),
            );
        });
    });
}

fn overview_cell(ui: &mut egui::Ui, palette: ThemePalette, width: f32, label: &str, value: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 22.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(
                egui::RichText::new(format!("{label}:"))
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            );
            ui.label(
                egui::RichText::new(value)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_bold".into()))
                    .color(redesign_text_primary(palette)),
            );
        },
    );
}

fn draft_banner(ui: &mut egui::Ui, palette: ThemePalette) {
    let fill = egui::Color32::from_rgba_premultiplied(0x1B, 0x12, 0x11, 31);
    let frame = egui::Frame::default()
        .fill(fill)
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_pill_danger(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
        .shadow(egui::epaint::Shadow {
            offset: [SHADOW_OFFSET_PX, SHADOW_OFFSET_PX],
            blur: 0,
            spread: 0,
            color: redesign_shadow(palette),
        });
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new(DRAFT_BANNER)
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_pill_danger(palette)),
        );
    });
}

fn fork_info_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let color = redesign_text_primary(palette);
    let label = "fork info";
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), color);

    let fork_w = 9.0;
    let gap = 5.0;
    let content_w = fork_w + gap + galley.size().x;
    let content_h = galley.size().y.max(fork_w);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    let pressed = response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let total_w = fork_w + gap + galley.size().x;
        let start_x = rect.center().x - total_w / 2.0;
        let cy = rect.center().y;
        paint_fork_glyph(painter, egui::pos2(start_x + fork_w / 2.0, cy), color);
        painter.text(
            egui::pos2(start_x + fork_w + gap, cy),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            color,
        );
    }

    response
}

fn paint_fork_glyph(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let half_h = 4.5;
    let split_y = center.y - 0.5;
    let tine_dx = 3.0;
    painter.line_segment(
        [
            egui::pos2(center.x, center.y + half_h),
            egui::pos2(center.x, split_y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x - tine_dx, center.y - half_h),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x + tine_dx, center.y - half_h),
        ],
        stroke,
    );
}

fn render_parse_error(ui: &mut egui::Ui, palette: ThemePalette, err: &str) -> PreviewOutcome {
    render_screen_title(
        ui,
        palette,
        "Preview",
        Some("the pasted share code could not be read"),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(err)
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_faint(palette)),
    );

    let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    if spacer > 0.0 {
        ui.add_space(spacer);
    }

    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Back" }),
        None::<SecondaryBtn<'_>>,
        Some("fix the code on the paste screen, then preview again"),
        PrimaryBtn {
            label: "Import Modlist",
            disabled: true,
        },
    );
    if footer.back_clicked {
        PreviewOutcome::Back
    } else {
        PreviewOutcome::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subline_includes_author_when_present() {
        assert_eq!(
            build_subline(Some("@b2bs")),
            "by @b2bs \u{00B7} review what will be installed before BIO downloads anything"
        );
    }

    #[test]
    fn subline_drops_author_segment_when_absent_or_blank() {
        let tail = "review what will be installed before BIO downloads anything";
        assert_eq!(build_subline(None), tail);
        assert_eq!(build_subline(Some("")), tail);
        assert_eq!(build_subline(Some("   ")), tail);
    }

    #[test]
    fn fallback_title_is_the_spec_authoritative_string() {
        assert_eq!(FALLBACK_TITLE, "Shared modlist");
    }

    #[test]
    fn draft_banner_copy_is_spec_verbatim() {
        assert!(DRAFT_BANNER.starts_with("Draft modlist code"));
        assert!(DRAFT_BANNER.contains("not from a verified install"));
        assert!(DRAFT_BANNER.contains("Create \u{2192} Import and modify"));
    }
}
