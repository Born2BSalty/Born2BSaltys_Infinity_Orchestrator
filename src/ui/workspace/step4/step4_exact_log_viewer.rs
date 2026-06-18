// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fmt::Write as _;

use eframe::egui;

use crate::app::state::Step3ItemState;
use crate::app::step4_action::Step4Action;
use crate::app::step5::log_files::{SourceLogInfo, source_log_infos};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};
use crate::ui::step4::service_step4::read_source_log_lines;
use crate::ui::workspace::widgets::weidu_line;

const BOX_PADDING: f32 = 12.0;

pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) -> Option<Step4Action> {
    let infos = source_log_infos(&orchestrator.wizard_state.step1);
    let active_tag = active_log_tag(orchestrator);
    let info = infos.into_iter().find(|i| i.tag == active_tag);

    let action = render_check_button(ui, orchestrator, palette);
    ui.add_space(8.0);

    render_source_info(ui, palette, info.as_ref());
    ui.add_space(8.0);

    let box_rect = render_log_frame(ui, palette);
    let inner = box_rect.shrink(BOX_PADDING);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    render_log_contents(&mut child, palette, active_tag, info.as_ref());

    ui.allocate_rect(box_rect, egui::Sense::hover());

    action
}

fn active_log_tag(orchestrator: &OrchestratorApp) -> &'static str {
    match orchestrator.wizard_state.step1.game_install.as_str() {
        "BG2EE" => "bg2ee",
        "EET" if orchestrator.wizard_state.step3.active_game_tab == "BG2EE" => "bg2ee",
        _ => "bgee",
    }
}

const fn step4_busy(orchestrator: &OrchestratorApp) -> bool {
    orchestrator.wizard_state.step2.is_scanning
        || orchestrator
            .wizard_state
            .step2
            .update_selected_check_running
        || orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        || orchestrator
            .wizard_state
            .step2
            .update_selected_extract_running
}

fn render_check_button(
    ui: &mut egui::Ui,
    orchestrator: &OrchestratorApp,
    palette: ThemePalette,
) -> Option<Step4Action> {
    let step4_busy = step4_busy(orchestrator);
    let mut action = None;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if redesign_btn(
                ui,
                palette,
                "Check Mod List",
                BtnOpts {
                    primary: true,
                    disabled: step4_busy,
                    ..Default::default()
                },
            )
            .clicked()
                && !step4_busy
            {
                action = Some(Step4Action::CheckMissingMods);
            }
        });
    });
    action
}

fn render_source_info(ui: &mut egui::Ui, palette: ThemePalette, info: Option<&SourceLogInfo>) {
    match info {
        Some(info) => {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Source")
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_muted(palette)),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(info.path.to_string_lossy().to_string())
                        .size(12.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_text_primary(palette)),
                );
            });
            let status = if info.exists { "Found" } else { "Missing" };
            let mut status_line = format!("Status: {status}");
            if let Some(sz) = info.size_bytes {
                let _ = write!(status_line, " \u{00B7} {sz} bytes");
            }
            ui.label(
                egui::RichText::new(status_line)
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_faint(palette)),
            );
        }
        None => {
            ui.label(
                egui::RichText::new("No source WeiDU log configured for this tab.")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_faint(palette)),
            );
        }
    }
}

fn render_log_frame(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Rect {
    let avail = ui.available_size();
    let (box_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
    if ui.is_rect_visible(box_rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(box_rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            box_rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    }
    box_rect
}

fn render_log_contents(
    child: &mut egui::Ui,
    palette: ThemePalette,
    active_tag: &str,
    info: Option<&SourceLogInfo>,
) {
    match info {
        Some(info) if info.exists => match read_source_log_lines(&info.path) {
            Ok(lines) if lines.is_empty() => {
                child.label(faint(palette, "Selected source log is empty."));
            }
            Ok(lines) => {
                let max_digits = lines.len().to_string().len();
                let lineno_w = weidu_line::lineno_column_width(max_digits);
                egui::ScrollArea::vertical()
                    .id_salt(("workspace_step4_exactlog_scroll", active_tag))
                    .auto_shrink([false, false])
                    .show(child, |ui| {
                        for (i, line) in lines.iter().enumerate() {
                            let item = line_as_item(line);
                            weidu_line::render_weidu_line(
                                ui,
                                palette,
                                &item,
                                Some(i + 1),
                                lineno_w,
                            );
                        }
                    });
            }
            Err(err) => {
                child.label(faint(palette, &format!("Failed to read file: {err}")));
            }
        },
        Some(_) => {
            child.label(faint(palette, "Source WeiDU log not found on disk."));
        }
        None => {
            child.label(faint(
                palette,
                "No source WeiDU log configured for this tab.",
            ));
        }
    }
}

fn line_as_item(raw: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: String::new(),
        component_id: String::new(),
        mod_name: String::new(),
        component_label: String::new(),
        raw_line: raw.to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 0,
        block_id: String::new(),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn faint(palette: ThemePalette, text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(13.0)
        .family(egui::FontFamily::Name("poppins_medium".into()))
        .color(redesign_text_faint(palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_tag_matches_bio_resolution() {
        let resolve = |game: &str, tab: &str| -> &'static str {
            match game {
                "BG2EE" => "bg2ee",
                "EET" if tab == "BG2EE" => "bg2ee",
                _ => "bgee",
            }
        };
        assert_eq!(resolve("BG2EE", "BGEE"), "bg2ee");
        assert_eq!(resolve("EET", "BG2EE"), "bg2ee");
        assert_eq!(resolve("EET", "BGEE"), "bgee");
        assert_eq!(resolve("BGEE", "BGEE"), "bgee");
        assert_eq!(resolve("IWDEE", "BGEE"), "bgee");
    }

    #[test]
    fn comment_line_wraps_into_raw_line_item() {
        let it = line_as_item("// Log of Currently Installed WeiDU Mods");
        assert_eq!(it.raw_line, "// Log of Currently Installed WeiDU Mods");
        assert!(!it.is_parent);
        let rendered = crate::app::step5::diagnostics::format_step4_item(&it);
        assert_eq!(rendered, "// Log of Currently Installed WeiDU Mods");
    }
}
