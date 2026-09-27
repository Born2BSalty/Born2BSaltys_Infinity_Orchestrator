// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::github_release_list::ReleaseListState;
use crate::app::mod_downloads::{self, ModDownloadSource};
use crate::app::source_form::{self, SourceForm, SourceFormIdentity, SourceKind};
use crate::app::state::{Step2DiscoveredFork, Step2State, VersionsSheet};
use crate::app::step2_action::{ModSourceEditDestination, Step2Action};
use crate::app::versions_view::VersionCard;
use crate::ui::orchestrator::widgets::icon_button::{self, ButtonIcon};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, redesign_btn_height};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_soft, redesign_error,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::shared::redesign_visuals::redesign_overlay_shadow;

use super::versions_form;

const SHEET_W: f32 = 600.0;

pub(crate) struct EditSourceEnv {
    pub(crate) busy: bool,
    pub(crate) logged_in: bool,
    pub(crate) release_list: ReleaseListState,
}

#[derive(Default)]
pub(crate) struct SheetOutcome {
    pub(crate) action: Option<Step2Action>,
    pub(crate) open_sheet: Option<VersionsSheet>,
    pub(crate) close: bool,
    pub(crate) seed_form: Option<SourceForm>,
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
    step2.versions_ui.source_form = None;
}

pub(crate) fn editor_ready_for(step2: &Step2State) -> bool {
    step2
        .versions_ui
        .source_form
        .as_ref()
        .is_some_and(|form| step2.versions_ui.sheet_tp2.as_deref() == Some(form.card_key.as_str()))
}

pub(crate) fn seed_source_form_for_card(
    tiers: &mod_downloads::SourceTiers,
    card: &VersionCard,
) -> SourceForm {
    let Some(source_id) = card.source_id.as_deref() else {
        return blank_source_form(card);
    };
    let sources = tiers.find_sources(&card.tp2);
    let key = mod_downloads::normalize_source_id(source_id);
    let resolved = sources
        .iter()
        .find(|source| mod_downloads::normalize_source_id(&source.source_id) == key)
        .cloned()
        .or_else(|| sources.into_iter().next());
    resolved.map_or_else(
        || blank_source_form(card),
        |source| {
            source_form::from_source(
                &source,
                SourceFormIdentity::default(),
                card_destination(card),
                &card.tp2,
            )
        },
    )
}

fn blank_source_form(card: &VersionCard) -> SourceForm {
    let seed = ModDownloadSource {
        tp2: card.tp2.clone(),
        name: card.name.clone(),
        source_id: "new-source".to_string(),
        github: Some(String::new()),
        ..ModDownloadSource::default()
    };
    source_form::from_source(
        &seed,
        SourceFormIdentity {
            may_change_id: true,
            is_new_mod: true,
        },
        ModSourceEditDestination::GlobalDefault,
        &card.tp2,
    )
}

fn card_destination(card: &VersionCard) -> ModSourceEditDestination {
    if card.layer == "This modlist" {
        ModSourceEditDestination::ThisModlist
    } else {
        ModSourceEditDestination::GlobalDefault
    }
}

pub(crate) fn seed_source_form_from_fork(
    tp2: &str,
    name: &str,
    fork: &Step2DiscoveredFork,
) -> SourceForm {
    let seed = ModDownloadSource {
        tp2: tp2.to_string(),
        name: name.to_string(),
        source_id: fork.owner_login.clone(),
        source_label: fork.owner_login.clone(),
        github: Some(fork.full_name.clone()),
        branch: Some(fork.default_branch.clone()),
        ..ModDownloadSource::default()
    };
    source_form::from_source(
        &seed,
        SourceFormIdentity {
            may_change_id: true,
            is_new_mod: false,
        },
        ModSourceEditDestination::GlobalDefault,
        tp2,
    )
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
                            render_header(ui, palette, "Download source", "", &mut outcome);
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new("Could not open this source")
                                    .color(redesign_error(palette)),
                            );
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
    env: &EditSourceEnv,
    escape_active: bool,
) -> SheetOutcome {
    let mut outcome = SheetOutcome::default();
    if render_scrim(ctx, "edit", drawer_rect) {
        outcome.close = true;
    }
    let popover_open = step2
        .versions_ui
        .source_form
        .as_ref()
        .is_some_and(|form| versions_form::any_popover_open(ctx, &form.card_key));
    if escape_active && !popover_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome.close = true;
    }
    let panel = panel_rect(drawer_rect);
    let mod_name = step2
        .versions_ui
        .source_form
        .as_ref()
        .map_or_else(String::new, |form| {
            if form.name.trim().is_empty() {
                form.tp2.clone()
            } else {
                form.name.clone()
            }
        });
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
                            render_header(ui, palette, "Download source", &mod_name, &mut outcome);
                            ui.add_space(12.0);
                            let footer_reserve = ui
                                .spacing()
                                .item_spacing
                                .y
                                .mul_add(2.0, redesign_btn_height(ui, true) + 12.0);
                            let body_h = (ui.available_height() - footer_reserve).max(0.0);
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .max_height(body_h)
                                .show(ui, |ui| {
                                    if let Some(action) =
                                        render_edit_source_body(ctx, ui, palette, step2, env)
                                        && outcome.action.is_none()
                                    {
                                        outcome.action = Some(action);
                                    }
                                });
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                render_edit_source_footer(
                                    ui,
                                    palette,
                                    step2,
                                    env.busy,
                                    &mut outcome,
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

fn render_edit_source_body(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    step2: &mut Step2State,
    env: &EditSourceEnv,
) -> Option<Step2Action> {
    let has_modlist_destination = mod_downloads::active_modlist_downloads_path().is_some();
    let form = step2.versions_ui.source_form.as_mut()?;
    if let Some(error) = form.error.clone() {
        ui.label(egui::RichText::new(error).color(redesign_error(palette)));
        ui.add_space(8.0);
    }
    let form_env = versions_form::FormEnv {
        has_modlist_destination,
        logged_in: env.logged_in,
        release_list: &env.release_list,
    };
    let outcome = versions_form::render(ctx, ui, palette, form, &form_env);
    outcome
        .request_release_list
        .map(|repo| Step2Action::RequestReleaseList { repo })
}

fn missing_field_message(form: &SourceForm) -> Option<String> {
    match form.kind {
        SourceKind::GitHub => form
            .repo
            .trim()
            .is_empty()
            .then(|| "Add a repository first".to_string()),
        SourceKind::WeaselMods | SourceKind::MorpheusMart | SourceKind::DirectLink => form
            .link
            .trim()
            .is_empty()
            .then(|| "Paste a link first".to_string()),
    }
}

fn render_edit_source_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    step2: &mut Step2State,
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
    let Some(form) = step2.versions_ui.source_form.as_mut() else {
        return;
    };
    let save_label = match form.save_to {
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
        if let Some(message) = missing_field_message(form) {
            form.error = Some(message);
        } else if outcome.action.is_none() {
            form.error = None;
            outcome.action = Some(Step2Action::SaveSourceForm);
        }
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
                outcome.seed_form = Some(seed_source_form_from_fork(tp2, label, fork));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_and_recheck_use_the_card_key_for_mixed_case_tp2() {
        let source = ModDownloadSource {
            tp2: "BG1NPC".to_string(),
            name: "BG1 NPC".to_string(),
            source_id: "primary".to_string(),
            github: Some("owner/repo".to_string()),
            ..ModDownloadSource::default()
        };
        let form = source_form::from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::ThisModlist,
            "bg1npcmusic",
        );

        let mut step2 = Step2State::default();
        step2.versions_ui.sheet_tp2 = Some("bg1npcmusic".to_string());
        step2.versions_ui.source_form = Some(form);

        assert!(editor_ready_for(&step2));
    }
}
