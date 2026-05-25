// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ForkAncestor;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent_deep,
    redesign_border_strong, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

const MAX_WIDTH_PX: f32 = 480.0;

const INDENT_PER_GEN_PX: f32 = 20.0;

const MAX_INDENT_GENERATIONS: usize = 6;

const MAX_INDENT_PX: f32 = INDENT_PER_GEN_PX * 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ForkInfoOutcome {
    #[default]
    Open,
    Closed,
}

pub(crate) struct SelfNode<'a> {
    pub(crate) name: &'a str,
    pub(crate) author: &'a str,
}

pub(crate) fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    id_salt: &str,
    lineage: &[ForkAncestor],
    self_node: &SelfNode<'_>,
) -> ForkInfoOutcome {
    let mut outcome = ForkInfoOutcome::Open;

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(18));

    egui::Window::new("orchestrator_fork_info_popup")
        .id(egui::Id::new(("orchestrator_fork_info_popup", id_salt)))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);

            ui.label(
                egui::RichText::new("Fork lineage")
                    .size(15.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(14.0);

            if lineage.is_empty() {
                ui.label(
                    egui::RichText::new(
                        "This modlist was created from scratch \u{2014} no fork lineage.",
                    )
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
                );
            } else {
                for (i, anc) in lineage.iter().enumerate() {
                    chain_row(ui, palette, i, &anc.name, &anc.author, false, i == 0);
                }
                let cur_idx = lineage.len();
                chain_row(
                    ui,
                    palette,
                    cur_idx,
                    self_node.name,
                    self_node.author,
                    true,
                    false,
                );
            }
            ui.add_space(16.0);

            let footer_h = 30.0;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), footer_h),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if redesign_btn(
                        ui,
                        palette,
                        "Close",
                        BtnOpts {
                            small: true,
                            ..Default::default()
                        },
                    )
                    .clicked()
                    {
                        outcome = ForkInfoOutcome::Closed;
                    }
                },
            );
        });

    outcome
}

fn chain_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    generation: usize,
    name: &str,
    author: &str,
    current: bool,
    is_root: bool,
) {
    let generation = u16::try_from(generation.min(MAX_INDENT_GENERATIONS)).unwrap_or(0);
    let indent = (f32::from(generation) * INDENT_PER_GEN_PX).min(MAX_INDENT_PX);

    ui.horizontal(|ui| {
        ui.add_space(indent);
        ui.spacing_mut().item_spacing.x = 8.0;

        if generation > 0 {
            ui.label(
                egui::RichText::new("\u{21B3}")
                    .size(13.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_faint(palette)),
            );
        }

        let name_color = if current {
            redesign_accent_deep(palette)
        } else {
            redesign_text_primary(palette)
        };
        ui.label(
            egui::RichText::new(name)
                .size(14.0)
                .family(egui::FontFamily::Name(if current {
                    "poppins_bold".into()
                } else {
                    "poppins_medium".into()
                }))
                .color(name_color),
        );

        if current {
            current_tag(ui, palette);
        }
    });

    if !author.trim().is_empty() {
        ui.horizontal(|ui| {
            let extra = if generation > 0 { 21.0 } else { 0.0 };
            ui.add_space(indent + extra);
            ui.label(
                egui::RichText::new(format!("by {}", author.trim()))
                    .size(12.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_faint(palette)),
            );
        });
    }

    if !is_root || generation > 0 || current {
        ui.add_space(10.0);
    }
}

fn current_tag(ui: &mut egui::Ui, palette: ThemePalette) {
    let pad_x = 7.0;
    let pad_y = 1.0;
    let font = egui::FontId::new(9.0, egui::FontFamily::Name("poppins_medium".into()));
    let color = redesign_text_muted(palette);
    let label = "THIS MODLIST";
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), color);

    let fork_w = 9.0;
    let gap = 4.0;
    let content_w = fork_w + gap + galley.size().x;
    let content_h = galley.size().y.max(fork_w);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let fork_center = egui::pos2(rect.left() + pad_x + fork_w / 2.0, rect.center().y);
        paint_fork_glyph(painter, fork_center, color);
        painter.text(
            egui::pos2(rect.left() + pad_x + fork_w + gap, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            color,
        );
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_f32_near {
        ($actual:expr, $expected:expr) => {
            assert!(($actual - $expected).abs() <= f32::EPSILON);
        };
    }

    #[test]
    fn indent_caps_for_deep_lineages() {
        let at_cap = (6.0_f32 * INDENT_PER_GEN_PX).min(MAX_INDENT_PX);
        let way_past = (40.0_f32 * INDENT_PER_GEN_PX).min(MAX_INDENT_PX);
        assert_f32_near!(at_cap, MAX_INDENT_PX);
        assert_f32_near!(way_past, MAX_INDENT_PX);
        const {
            assert!(MAX_INDENT_PX < MAX_WIDTH_PX);
        }
    }

    #[test]
    fn self_node_author_absence_is_representable() {
        let s = SelfNode {
            name: "X",
            author: "",
        };
        assert!(s.author.trim().is_empty());
    }
}
