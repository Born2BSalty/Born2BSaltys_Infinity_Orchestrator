// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::state::{Step2DiscoveredFork, Step2State, VersionsSheet};
use crate::app::step2_action::{ModSourceEditDestination, Step2Action};
use crate::ui::orchestrator::widgets::icon_button::{self, ButtonIcon};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_soft, redesign_error,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::shared::redesign_visuals::redesign_overlay_shadow;

const SHEET_W: f32 = 600.0;

#[derive(Default)]
pub(crate) struct SheetOutcome {
    pub(crate) action: Option<Step2Action>,
    pub(crate) open_sheet: Option<VersionsSheet>,
    pub(crate) close: bool,
}

fn panel_rect(drawer_rect: egui::Rect) -> egui::Rect {
    let w = SHEET_W.min(drawer_rect.width());
    egui::Rect::from_min_max(
        egui::pos2(drawer_rect.right() - w, drawer_rect.top()),
        drawer_rect.max,
    )
}

fn render_scrim(ctx: &egui::Context, id_salt: &str, drawer_rect: egui::Rect) -> bool {
    egui::Area::new(egui::Id::new(("versions_sheet_scrim", id_salt)))
        .order(egui::Order::Foreground)
        .fixed_pos(drawer_rect.min)
        .show(ctx, |ui| {
            let response = ui.allocate_rect(drawer_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(drawer_rect, 0.0, egui::Color32::from_black_alpha(89));
            response.clicked()
        })
        .inner
}

fn render_header(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    title: &str,
    mod_name: &str,
    outcome: &mut SheetOutcome,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .size(17.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.label(
            egui::RichText::new(mod_name)
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button::render(ui, palette, ButtonIcon::Close, "Close", true).clicked() {
                outcome.close = true;
            }
        });
    });
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        egui::Stroke::new(1.0_f32, redesign_border_soft(palette)),
    );
}

pub(crate) fn clear_editor_state(step2: &mut Step2State) {
    step2.mod_download_source_editor_open = false;
    step2.mod_download_source_editor_text.clear();
    step2.mod_download_source_editor_error = None;
}

pub(crate) fn editor_ready_for(step2: &Step2State) -> bool {
    step2.mod_download_source_editor_open
        && step2.versions_ui.sheet_tp2.as_deref()
            == Some(step2.mod_download_source_editor_tp2.as_str())
}

pub(crate) fn render_edit_source_unready(
    ctx: &egui::Context,
    palette: ThemePalette,
    drawer_rect: egui::Rect,
    step2: &mut Step2State,
    escape_active: bool,
) -> SheetOutcome {
    let mut outcome = SheetOutcome::default();
    if render_scrim(ctx, "edit", drawer_rect) {
        outcome.close = true;
    }
    if escape_active && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome.close = true;
    }
    let panel = panel_rect(drawer_rect);
    egui::Area::new(egui::Id::new("versions_sheet_panel_edit"))
        .order(egui::Order::Foreground)
        .fixed_pos(panel.min)
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .shadow(redesign_overlay_shadow(palette))
                .inner_margin(egui::Margin::ZERO)
                .show(ui, |ui| {
                    ui.set_min_size(panel.size());
                    ui.set_max_size(panel.size());
                    egui::Frame::default()
                        .inner_margin(egui::Margin::symmetric(22, 18))
                        .show(ui, |ui| {
                            render_header(
                                ui,
                                palette,
                                "Download source",
                                &step2.mod_download_source_editor_display_name,
                                &mut outcome,
                            );
                            ui.add_space(12.0);
                            let message = step2
                                .mod_download_source_editor_error
                                .clone()
                                .unwrap_or_else(|| "Could not open this source".to_string());
                            ui.label(egui::RichText::new(message).color(redesign_error(palette)));
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                if redesign_btn(
                                    ui,
                                    palette,
                                    "Cancel",
                                    BtnOpts {
                                        small: true,
                                        ..Default::default()
                                    },
                                )
                                .clicked()
                                {
                                    outcome.close = true;
                                }
                                redesign_btn(
                                    ui,
                                    palette,
                                    "Save",
                                    BtnOpts {
                                        primary: true,
                                        small: true,
                                        disabled: true,
                                        ..Default::default()
                                    },
                                );
                            });
                        });
                });
        });
    if outcome.close {
        clear_editor_state(step2);
    }
    outcome
}

pub(crate) fn render_edit_source(
    ctx: &egui::Context,
    palette: ThemePalette,
    drawer_rect: egui::Rect,
    step2: &mut Step2State,
    busy: bool,
    escape_active: bool,
) -> SheetOutcome {
    let mut outcome = SheetOutcome::default();
    if render_scrim(ctx, "edit", drawer_rect) {
        outcome.close = true;
    }
    if escape_active && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome.close = true;
    }
    let panel = panel_rect(drawer_rect);
    egui::Area::new(egui::Id::new("versions_sheet_panel_edit"))
        .order(egui::Order::Foreground)
        .fixed_pos(panel.min)
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .shadow(redesign_overlay_shadow(palette))
                .inner_margin(egui::Margin::ZERO)
                .show(ui, |ui| {
                    ui.set_min_size(panel.size());
                    ui.set_max_size(panel.size());
                    egui::Frame::default()
                        .inner_margin(egui::Margin::symmetric(22, 18))
                        .show(ui, |ui| {
                            render_header(
                                ui,
                                palette,
                                "Download source",
                                &step2.mod_download_source_editor_display_name,
                                &mut outcome,
                            );
                            ui.add_space(12.0);
                            render_edit_source_body(ui, palette, step2, busy);
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                render_edit_source_footer(ui, palette, step2, busy, &mut outcome);
                            });
                        });
                });
        });
    if outcome.close {
        clear_editor_state(step2);
    }
    outcome
}

fn render_edit_source_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    step2: &mut Step2State,
    busy: bool,
) {
    let has_modlist_destination = mod_downloads::active_modlist_downloads_path().is_some();
    ui.horizontal(|ui| {
        if has_modlist_destination
            && destination_chip(
                ui,
                palette,
                "This modlist",
                step2.mod_download_source_editor_destination
                    == ModSourceEditDestination::ThisModlist,
                busy,
            )
        {
            step2.mod_download_source_editor_destination = ModSourceEditDestination::ThisModlist;
        }
        if destination_chip(
            ui,
            palette,
            "My default",
            step2.mod_download_source_editor_destination == ModSourceEditDestination::GlobalDefault,
            busy,
        ) {
            step2.mod_download_source_editor_destination = ModSourceEditDestination::GlobalDefault;
        }
    });
    ui.add_space(8.0);
    if let Some(err) = step2.mod_download_source_editor_error.clone() {
        ui.label(egui::RichText::new(err).color(redesign_error(palette)));
        ui.add_space(4.0);
    }
    ui.add_enabled(
        !busy,
        egui::TextEdit::multiline(&mut step2.mod_download_source_editor_text)
            .desired_rows(16)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace),
    );
}

fn destination_chip(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    selected: bool,
    busy: bool,
) -> bool {
    redesign_btn(
        ui,
        palette,
        label,
        BtnOpts {
            small: true,
            primary: selected,
            disabled: busy,
            ..Default::default()
        },
    )
    .clicked()
        && !selected
        && !busy
}

fn render_edit_source_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    step2: &Step2State,
    busy: bool,
    outcome: &mut SheetOutcome,
) {
    if redesign_btn(
        ui,
        palette,
        "Cancel",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .clicked()
    {
        outcome.close = true;
    }
    let save_label = match step2.mod_download_source_editor_destination {
        ModSourceEditDestination::ThisModlist => "Save to this modlist",
        ModSourceEditDestination::GlobalDefault => "Save to My default",
    };
    if redesign_btn(
        ui,
        palette,
        save_label,
        BtnOpts {
            primary: true,
            small: true,
            disabled: busy,
            ..Default::default()
        },
    )
    .clicked()
        && !busy
    {
        outcome.action = Some(Step2Action::SaveModDownloadSourceEditor);
    }
}

pub(crate) fn render_forks(
    ctx: &egui::Context,
    palette: ThemePalette,
    drawer_rect: egui::Rect,
    step2: &Step2State,
    escape_active: bool,
) -> SheetOutcome {
    let mut outcome = SheetOutcome::default();
    if render_scrim(ctx, "forks", drawer_rect) {
        outcome.close = true;
    }
    if escape_active && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome.close = true;
    }
    let panel = panel_rect(drawer_rect);
    egui::Area::new(egui::Id::new("versions_sheet_panel_forks"))
        .order(egui::Order::Foreground)
        .fixed_pos(panel.min)
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .shadow(redesign_overlay_shadow(palette))
                .inner_margin(egui::Margin::ZERO)
                .show(ui, |ui| {
                    ui.set_min_size(panel.size());
                    ui.set_max_size(panel.size());
                    egui::Frame::default()
                        .inner_margin(egui::Margin::symmetric(22, 18))
                        .show(ui, |ui| {
                            render_header(
                                ui,
                                palette,
                                "Forks",
                                &step2.mod_download_forks_popup_label,
                                &mut outcome,
                            );
                            ui.add_space(12.0);
                            if let Some(err) = step2.mod_download_forks_popup_error.as_ref() {
                                ui.label(egui::RichText::new(err).color(redesign_error(palette)));
                                ui.add_space(8.0);
                            }
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for fork in &step2.mod_download_forks {
                                        render_fork_row(
                                            ui,
                                            palette,
                                            fork,
                                            &step2.mod_download_forks_popup_tp2,
                                            &step2.mod_download_forks_popup_label,
                                            &mut outcome,
                                        );
                                    }
                                });
                        });
                });
        });
    outcome
}

fn render_fork_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    fork: &Step2DiscoveredFork,
    tp2: &str,
    label: &str,
    outcome: &mut SheetOutcome,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(&fork.full_name)
                .font(egui::FontId::monospace(13.0))
                .color(redesign_text_primary(palette)),
        );
        ui.label(
            egui::RichText::new(&fork.default_branch)
                .size(12.0)
                .color(redesign_text_muted(palette)),
        );
        let updated_date = fork
            .updated_at
            .split('T')
            .next()
            .unwrap_or(&fork.updated_at);
        ui.label(
            egui::RichText::new(updated_date)
                .size(12.0)
                .color(redesign_text_muted(palette)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if redesign_btn(
                ui,
                palette,
                "Use",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                outcome.action = Some(Step2Action::AddDiscoveredModDownloadFork {
                    tp2: tp2.to_string(),
                    label: label.to_string(),
                    full_name: fork.full_name.clone(),
                    owner_login: fork.owner_login.clone(),
                    default_branch: fork.default_branch.clone(),
                });
                outcome.open_sheet = Some(VersionsSheet::EditSource);
            }
            if redesign_btn(
                ui,
                palette,
                "Open",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                outcome.action = Some(Step2Action::OpenSelectedWeb(fork.html_url.clone()));
            }
        });
    });
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette)),
    );
    ui.add_space(6.0);
}
