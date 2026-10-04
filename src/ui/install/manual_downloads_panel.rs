// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::install::stage_downloading::{check_prose_cell, sized_label};
use crate::ui::install::state_install::{ManualDownloadsState, ManualRowStatus};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_error, redesign_success, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelAction {
    None,
    OpenPage(usize),
    PickFile(usize),
}

#[must_use]
pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &ManualDownloadsState,
    box_frame: egui::Frame,
    last_refusal: Option<&str>,
) -> PanelAction {
    let mut action = PanelAction::None;

    box_frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("manual downloads")
                .size(11.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "Download these by hand and drop them into your Mods archive folder. BIO picks them up automatically.",
            )
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_primary(palette)),
        );
        ui.add_space(10.0);

        action = render_grid(ui, palette, state);

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(watching_line(&state.watched_folder))
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_faint(palette)),
        );

        if !state.unmatched.is_empty() {
            ui.label(
                egui::RichText::new(format!(
                    "landed, matched nothing: {}",
                    capped_unmatched_list(&state.unmatched)
                ))
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_faint(palette)),
            );
        }

        if let Some(reason) = last_refusal {
            ui.label(
                egui::RichText::new(reason)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_error(palette)),
            );
        }
    });

    action
}

fn render_grid(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &ManualDownloadsState,
) -> PanelAction {
    let mut action = PanelAction::None;
    let col_gap = 12.0;
    let from_w = 170.0;
    let status_w = 110.0;
    let actions_w = 200.0;
    let mod_w = f32::mul_add(
        col_gap,
        -3.0,
        ui.available_width() - from_w - status_w - actions_w,
    )
    .max(120.0);

    egui::ScrollArea::vertical()
        .max_height(120.0)
        .min_scrolled_height(120.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new("manual_downloads_grid")
                .num_columns(4)
                .spacing(egui::vec2(col_gap, 6.0))
                .min_col_width(0.0)
                .show(ui, |ui| {
                    grid_header(ui, palette, "mod", mod_w);
                    grid_header(ui, palette, "from", from_w);
                    grid_header(ui, palette, "status", status_w);
                    grid_header(ui, palette, "actions", actions_w);
                    ui.end_row();

                    for (index, row) in state.rows.iter().enumerate() {
                        sized_label(
                            ui,
                            mod_w,
                            &row.label,
                            14.0,
                            "poppins_medium",
                            redesign_text_primary(palette),
                        );
                        sized_label(
                            ui,
                            from_w,
                            &row.from,
                            13.0,
                            "poppins_light",
                            redesign_text_faint(palette),
                        );
                        render_status_cell(ui, palette, status_w, &row.status);

                        let disabled = matches!(
                            row.status,
                            ManualRowStatus::Found | ManualRowStatus::Skipped
                        );
                        let (open_clicked, pick_clicked) =
                            render_actions_cell(ui, palette, actions_w, &row.page_url, disabled);
                        if open_clicked {
                            action = PanelAction::OpenPage(index);
                        } else if pick_clicked {
                            action = PanelAction::PickFile(index);
                        }
                        ui.end_row();
                    }
                });
        });

    action
}

fn watching_line(watched_folder: &str) -> String {
    if watched_folder.is_empty() {
        "set your Mods archive folder in Settings \u{2192} Paths and BIO will watch it for these files".to_string()
    } else {
        format!("watching {watched_folder}")
    }
}

fn capped_unmatched_list(unmatched: &[String]) -> String {
    const CAP: usize = 3;
    if unmatched.len() <= CAP {
        return unmatched.join(", ");
    }
    let shown = unmatched[..CAP].join(", ");
    let more = unmatched.len() - CAP;
    format!("{shown} and {more} more")
}

fn grid_header(ui: &mut egui::Ui, palette: ThemePalette, text: &str, w: f32) {
    sized_label(
        ui,
        w,
        text,
        14.0,
        "poppins_light",
        redesign_text_muted(palette),
    );
}

fn render_status_cell(ui: &mut egui::Ui, palette: ThemePalette, w: f32, status: &ManualRowStatus) {
    match status {
        ManualRowStatus::Waiting => {
            sized_label(
                ui,
                w,
                "waiting",
                14.0,
                "poppins_medium",
                redesign_text_faint(palette),
            );
        }
        ManualRowStatus::Found => {
            check_prose_cell(ui, w, "found", redesign_success(palette));
        }
        ManualRowStatus::Refused(_) => {
            sized_label(
                ui,
                w,
                "refused",
                14.0,
                "poppins_medium",
                redesign_error(palette),
            );
        }
        ManualRowStatus::Skipped => {
            sized_label(
                ui,
                w,
                "skipped",
                14.0,
                "poppins_medium",
                redesign_text_faint(palette),
            );
        }
    }
}

fn render_actions_cell(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    w: f32,
    page_url: &str,
    disabled: bool,
) -> (bool, bool) {
    let mut open_clicked = false;
    let mut pick_clicked = false;
    ui.allocate_ui_with_layout(
        egui::vec2(w, 18.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            if !page_url.is_empty() {
                let opts = BtnOpts {
                    small: true,
                    disabled,
                    ..Default::default()
                };
                if redesign_btn(ui, palette, "Open page", opts).clicked() {
                    open_clicked = true;
                }
                ui.add_space(8.0);
            }
            let opts = BtnOpts {
                small: true,
                disabled,
                ..Default::default()
            };
            if redesign_btn(ui, palette, "Pick file…", opts).clicked() {
                pick_clicked = true;
            }
        },
    );
    (open_clicked, pick_clicked)
}

#[cfg(test)]
mod tests {
    use super::{capped_unmatched_list, watching_line};

    #[test]
    fn watching_line_copy_with_and_without_folder() {
        assert_eq!(
            watching_line("D:\\BIO\\archives"),
            "watching D:\\BIO\\archives"
        );
        assert_eq!(
            watching_line(""),
            "set your Mods archive folder in Settings \u{2192} Paths and BIO will watch it for these files"
        );
    }

    #[test]
    fn capped_unmatched_list_shows_three_then_a_count() {
        let names: Vec<String> = ["a.zip", "b.zip", "c.zip"]
            .into_iter()
            .map(str::to_string)
            .collect();
        assert_eq!(capped_unmatched_list(&names), "a.zip, b.zip, c.zip");

        let seven: Vec<String> = [
            "a.zip", "b.zip", "c.zip", "d.zip", "e.zip", "f.zip", "g.zip",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(
            capped_unmatched_list(&seven),
            "a.zip, b.zip, c.zip and 4 more"
        );
    }
}
