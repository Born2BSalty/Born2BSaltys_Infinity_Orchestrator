// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::layout_tokens_global::{STEP4_SAVE_BTN_H, STEP4_SAVE_BTN_W};
use crate::ui::shared::tooltip_global as tt;
use crate::ui::shared::typography_global as typo;
use crate::ui::step4::action_step4::Step4Action;
use crate::ui::step4::service_step4::read_source_log_lines;
use crate::ui::step4::state_step4::active_tab_mut;
use crate::ui::step5::diagnostics::format_step4_item;
use crate::ui::step5::service_diagnostics_support_step5::{export_diagnostics, source_log_infos};

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step4Action> {
    let mut action = None;
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let step4_busy = state.step2.is_scanning
        || state.step2.update_selected_check_running
        || state.step2.update_selected_download_running
        || state.step2.update_selected_extract_running;
    ui.horizontal(|ui| {
        ui.heading("Step 4: Review");
        if dev_mode {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Export diagnostics").clicked() {
                    match export_diagnostics(state, None, dev_mode, exe_fingerprint) {
                        Ok(path) => {
                            state.step5.last_status_text =
                                format!("Diagnostics exported: {}", path.display());
                        }
                        Err(err) => {
                            state.step5.last_status_text =
                                format!("Diagnostics export failed: {err}");
                        }
                    }
                }
            });
        }
    });
    if exact_log_mode {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!step4_busy, egui::Button::new("Check Mod List"))
                    .clicked()
                {
                    action = Some(Step4Action::CheckMissingMods);
                }
            });
        });
    }

    if exact_log_mode {
        ui.label("Review the source WeiDU log file(s) that will be used for install.");
        ui.label("Next continues to Step 5 without going through Step 2/3.");
        if step4_busy {
            ui.add_space(6.0);
            ui.label(&state.step2.scan_status);
        }
    } else {
        ui.label("Verify setup and install order before running.");
        ui.label("Next will save weidu.log file(s) and continue to Step 5.");
    }

    ui.add_space(12.0);
    if !exact_log_mode {
        ui.horizontal(|ui| {
            let label = match state.step1.game_install.as_str() {
                "EET" => "Save weidu.log's",
                _ => "Save weidu.log",
            };
            if ui
                .add_sized(
                    [STEP4_SAVE_BTN_W, STEP4_SAVE_BTN_H],
                    egui::Button::new(label),
                )
                .on_hover_text(tt::STEP4_SAVE_WEIDU_LOG)
                .clicked()
            {
                action = Some(Step4Action::SaveWeiduLog);
            }
        });
    }

    ui.add_space(8.0);
    section(
        ui,
        if exact_log_mode {
            "Source WeiDU Logs"
        } else {
            "Install Order"
        },
        |ui| {
            if exact_log_mode {
                render_source_logs(ui, state);
                return;
            }

            let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
            let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                if show_bgee && show_bg2ee {
                    draw_tab(ui, active_tab_mut(state), "BGEE");
                    draw_tab(ui, active_tab_mut(state), "BG2EE");
                } else if show_bgee {
                    ui.label(typo::monospace("BGEE"));
                } else if show_bg2ee {
                    ui.label(typo::monospace("BG2EE"));
                }
            });
            ui.add_space(6.0);
            render_order_list(ui, state);
        },
    );

    action
}

pub fn draw_tab(ui: &mut egui::Ui, active: &mut String, value: &str) {
    let is_active = active == value;
    let fill = if is_active {
        ui.visuals().widgets.active.bg_fill
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    let stroke = if is_active {
        ui.visuals().widgets.active.bg_stroke
    } else {
        ui.visuals().widgets.inactive.bg_stroke
    };
    let text_color = if is_active {
        ui.visuals().widgets.active.fg_stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    let button = egui::Button::new(typo::plain(value).color(text_color))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(
            crate::ui::shared::layout_tokens_global::RADIUS_SM as u8,
        ));
    if ui.add_sized([58.0, 24.0], button).clicked() {
        *active = value.to_string();
    }
}

pub fn render_weidu_colored_line(ui: &mut egui::Ui, text: &str) {
    let mut job = egui::text::LayoutJob::default();
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let default_color = ui.visuals().text_color();
    let path_color = crate::ui::shared::theme_global::accent_path();
    let nums_color = crate::ui::shared::theme_global::accent_numbers();
    let comment_color = crate::ui::shared::theme_global::success();

    if text.trim_start().starts_with("//") {
        append_text(&mut job, text, &mono, comment_color);
        ui.label(egui::WidgetText::from(job));
        return;
    }

    if let Some(path_start) = text.find('~')
        && let Some(path_end_rel) = text[path_start + 1..].find('~')
    {
        let path_end = path_start + path_end_rel + 2;
        let comment_start = text[path_end..].find("//").map(|idx| path_end + idx);

        append_text(&mut job, &text[..path_start], &mono, default_color);
        append_text(&mut job, &text[path_start..path_end], &mono, path_color);
        if let Some(comment_start) = comment_start {
            append_text(&mut job, &text[path_end..comment_start], &mono, nums_color);
            append_text(&mut job, &text[comment_start..], &mono, comment_color);
        } else {
            append_text(&mut job, &text[path_end..], &mono, nums_color);
        }
    } else {
        append_text(&mut job, text, &mono, default_color);
    }

    ui.label(egui::WidgetText::from(job));
}

pub fn render_source_logs(ui: &mut egui::Ui, state: &mut WizardState) {
    let infos = source_log_infos(&state.step1);
    if infos.is_empty() {
        ui.label("No source WeiDU logs configured.");
        return;
    }

    let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    ui.horizontal(|ui| {
        if show_bgee && show_bg2ee {
            draw_tab(ui, &mut state.step3.active_game_tab, "BGEE");
            draw_tab(ui, &mut state.step3.active_game_tab, "BG2EE");
        } else if show_bgee {
            ui.label(typo::monospace("BGEE"));
        } else if show_bg2ee {
            ui.label(typo::monospace("BG2EE"));
        }
    });
    ui.add_space(6.0);

    let active_tag = match state.step1.game_install.as_str() {
        "BG2EE" => "bg2ee",
        "EET" if state.step3.active_game_tab == "BG2EE" => "bg2ee",
        _ => "bgee",
    };
    let Some(info) = infos.into_iter().find(|i| i.tag == active_tag) else {
        ui.label("No source WeiDU log configured for this tab.");
        return;
    };

    ui.horizontal(|ui| {
        ui.label(typo::strong("Source"));
        ui.label(typo::monospace(info.path.to_string_lossy().to_string()));
    });
    let status = if info.exists { "Found" } else { "Missing" };
    ui.horizontal(|ui| {
        ui.label(format!("Status: {status}"));
        if let Some(sz) = info.size_bytes {
            ui.label(format!("Size: {sz} bytes"));
        }
    });
    ui.add_space(4.0);

    if !info.exists {
        return;
    }

    let lines = match read_source_log_lines(&info.path) {
        Ok(v) => v,
        Err(err) => {
            ui.label(format!("Failed to read file: {err}"));
            return;
        }
    };
    let nav_clearance = 26.0;
    let list_height = (ui.available_height() - nav_clearance).max(180.0);
    let viewport_w = ui.available_width();
    ui.scope(|ui| {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 12.0;
        scroll.bar_inner_margin = 0.0;
        scroll.bar_outer_margin = 2.0;
        ui.style_mut().spacing.scroll = scroll;
        egui::ScrollArea::vertical()
            .id_salt(("step4_source_logs_scroll", active_tag))
            .auto_shrink([false, false])
            .max_height(list_height)
            .show(ui, |ui| {
                ui.set_min_width(viewport_w);
                if lines.is_empty() {
                    ui.label("Selected source log is empty.");
                    return;
                }
                for line in &lines {
                    render_weidu_colored_line(ui, line);
                }
            });
    });
}

pub fn render_order_list(ui: &mut egui::Ui, state: &WizardState) {
    let items = if state.step3.active_game_tab == "BG2EE" {
        &state.step3.bg2ee_items
    } else {
        &state.step3.bgee_items
    };
    let export_items: Vec<_> = items.iter().filter(|i| !i.is_parent).collect();
    let nav_clearance = 26.0;
    let list_height = (ui.available_height() - nav_clearance).max(180.0);
    let viewport_w = ui.available_width();
    ui.scope(|ui| {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 12.0;
        scroll.bar_inner_margin = 0.0;
        scroll.bar_outer_margin = 2.0;
        ui.style_mut().spacing.scroll = scroll;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(list_height)
            .show(ui, |ui| {
                ui.set_min_width(viewport_w);
                if export_items.is_empty() {
                    ui.label("No ordered components found.");
                } else {
                    for item in export_items {
                        render_weidu_colored_line(ui, &format_step4_item(item));
                    }
                }
            });
    });
}

fn append_text(
    job: &mut egui::text::LayoutJob,
    text: &str,
    font_id: &egui::FontId,
    color: egui::Color32,
) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font_id.clone(),
            color,
            ..Default::default()
        },
    );
}

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(crate::ui::shared::typography_global::section_title(title));
        });
        ui.add_space(6.0);
        body(ui);
    });
}
