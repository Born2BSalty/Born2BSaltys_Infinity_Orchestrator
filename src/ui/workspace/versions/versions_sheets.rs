// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::app_step2_router::NEW_MOD_CARD_KEY;
use crate::app::github_forks_list::ForksStatus;
use crate::app::github_release_list::ReleaseListState;
use crate::app::mod_downloads::{self, ModDownloadSource};
use crate::app::mod_source_history;
use crate::app::source_form::{self, SourceForm, SourceFormIdentity, SourceKind};
use crate::app::state::{Step2DiscoveredFork, Step2State, VersionsSheet};
use crate::app::step2_action::{ModSourceEditDestination, Step2Action};
use crate::app::versions_view::{KnownExtras, VersionCard};
use crate::ui::orchestrator::widgets::icon_button::{self, ButtonIcon};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, redesign_btn_height};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_soft,
    redesign_border_strong, redesign_error, redesign_input_bg, redesign_shell_bg,
    redesign_text_faint, redesign_text_muted, redesign_text_primary,
};
use crate::ui::shared::redesign_visuals::redesign_overlay_shadow;

use super::{versions_form, versions_icons};

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

#[derive(Debug, Clone, Default)]
pub(crate) struct NoteSeed {
    pub(crate) mod_name: String,
    pub(crate) rule_words: String,
    pub(crate) location: String,
    pub(crate) note_text: String,
    pub(crate) signature: String,
}

#[derive(Debug, Clone, Default)]
struct NoteSheetState {
    tp2: String,
    mod_name: String,
    rule_words: String,
    location: String,
    signature: String,
    who: String,
    text: String,
}

fn note_state_id() -> egui::Id {
    egui::Id::new("versions_note_sheet_state")
}

pub(crate) fn seed_note_sheet(ctx: &egui::Context, tp2: &str, seed: NoteSeed, who: String) {
    let state = NoteSheetState {
        tp2: tp2.to_string(),
        mod_name: seed.mod_name,
        rule_words: seed.rule_words,
        location: seed.location,
        signature: seed.signature,
        who,
        text: seed.note_text,
    };
    ctx.data_mut(|d| d.insert_temp(note_state_id(), state));
}

fn panel_rect(drawer_rect: egui::Rect) -> egui::Rect {
    let w = SHEET_W.min(drawer_rect.width());
    egui::Rect::from_min_max(
        egui::pos2(drawer_rect.right() - w, drawer_rect.top()),
        drawer_rect.max,
    )
}

fn render_sheet_shell(
    ctx: &egui::Context,
    palette: ThemePalette,
    id_salt: &'static str,
    drawer_rect: egui::Rect,
    body: impl FnOnce(&mut egui::Ui, &mut SheetOutcome),
) -> SheetOutcome {
    let mut outcome = SheetOutcome::default();
    let panel = panel_rect(drawer_rect);
    egui::Area::new(egui::Id::new(id_salt))
        .order(egui::Order::Tooltip)
        .fixed_pos(drawer_rect.min)
        .interactable(true)
        .show(ctx, |ui| {
            let backdrop = ui.allocate_rect(drawer_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(drawer_rect, 0.0, egui::Color32::from_black_alpha(89));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(panel), |ui| {
                egui::Frame::default()
                    .fill(redesign_shell_bg(palette))
                    .shadow(redesign_overlay_shadow(palette))
                    .inner_margin(egui::Margin::ZERO)
                    .show(ui, |ui| {
                        ui.set_min_size(panel.size());
                        ui.set_max_size(panel.size());
                        body(ui, &mut outcome);
                    });
            });
            let clicked_outside = backdrop.clicked()
                && ctx
                    .pointer_interact_pos()
                    .is_some_and(|pos| !panel.contains(pos));
            if clicked_outside {
                outcome.close = true;
            }
        });
    outcome
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

pub(crate) fn resolve_current_source(
    tiers: &mod_downloads::SourceTiers,
    card: &VersionCard,
) -> Option<ModDownloadSource> {
    let source_id = card.source_id.as_deref()?;
    let sources = tiers.find_sources(&card.tp2);
    let key = mod_downloads::normalize_source_id(source_id);
    sources
        .iter()
        .find(|source| mod_downloads::normalize_source_id(&source.source_id) == key)
        .cloned()
        .or_else(|| sources.into_iter().next())
}

pub(crate) fn seed_source_form_for_card(
    tiers: &mod_downloads::SourceTiers,
    card: &VersionCard,
    known: Option<&KnownExtras>,
    this_modlist_name: &str,
) -> SourceForm {
    let Some(source) = resolve_current_source(tiers, card) else {
        return blank_source_form(card);
    };
    let destination = card_destination(card);
    let mut form = source_form::from_source(
        &source,
        SourceFormIdentity::default(),
        destination,
        &card.tp2,
    );
    if let Some(known) = known {
        let signature = mod_source_history::rule_signature(&source);
        let key = mod_source_history::note_key(&card.tp2, &signature);
        if let Some(note) = known.store.notes.get(&key) {
            form.note.clone_from(&note.text);
        }
    }
    form.note_seed.clone_from(&form.note);
    form.note_who = match destination {
        ModSourceEditDestination::ThisModlist => this_modlist_name.to_string(),
        ModSourceEditDestination::GlobalDefault => "My default".to_string(),
    };
    form
}

fn blank_source_form(card: &VersionCard) -> SourceForm {
    let seed = ModDownloadSource {
        tp2: card.tp2.clone(),
        name: card.name.clone(),
        source_id: "new-source".to_string(),
        github: Some(String::new()),
        ..ModDownloadSource::default()
    };
    let mut form = source_form::from_source(
        &seed,
        SourceFormIdentity {
            may_change_id: true,
            is_new_mod: true,
        },
        ModSourceEditDestination::GlobalDefault,
        &card.tp2,
    );
    form.note_who = "My default".to_string();
    form
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
    source_fork: &Step2DiscoveredFork,
) -> SourceForm {
    let seed = ModDownloadSource {
        tp2: tp2.to_string(),
        name: name.to_string(),
        source_id: source_fork.owner_login.clone(),
        source_label: source_fork.owner_login.clone(),
        github: Some(source_fork.full_name.clone()),
        branch: Some(source_fork.default_branch.clone()),
        ..ModDownloadSource::default()
    };
    let mut seeded = source_form::from_source(
        &seed,
        SourceFormIdentity {
            may_change_id: true,
            is_new_mod: false,
        },
        ModSourceEditDestination::GlobalDefault,
        tp2,
    );
    seeded.note_who = "My default".to_string();
    seeded
}

pub(crate) fn seed_new_mod_form(this_modlist_name: &str) -> SourceForm {
    let seed = ModDownloadSource {
        tp2: String::new(),
        name: String::new(),
        source_id: "primary".to_string(),
        source_label: "Primary".to_string(),
        github: Some(String::new()),
        ..ModDownloadSource::default()
    };
    let mut form = source_form::from_source(
        &seed,
        SourceFormIdentity {
            may_change_id: true,
            is_new_mod: true,
        },
        ModSourceEditDestination::ThisModlist,
        NEW_MOD_CARD_KEY,
    );
    form.note_who = this_modlist_name.to_string();
    form
}

fn sheet_mod_name(form: &SourceForm) -> String {
    if !form.name.trim().is_empty() {
        form.name.clone()
    } else if !form.tp2.trim().is_empty() {
        form.tp2.clone()
    } else {
        "New mod".to_string()
    }
}

pub(crate) fn render_edit_source_unready(
    ctx: &egui::Context,
    palette: ThemePalette,
    drawer_rect: egui::Rect,
    step2: &mut Step2State,
    escape_active: bool,
) -> SheetOutcome {
    let escape_close = escape_active && ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mut outcome = render_sheet_shell(
        ctx,
        palette,
        "versions_sheet_edit",
        drawer_rect,
        |ui, outcome| {
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(22, 18))
                .show(ui, |ui| {
                    render_header(ui, palette, "Download source", "", outcome);
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
        },
    );
    if escape_close {
        outcome.close = true;
    }
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
    let popover_open = step2
        .versions_ui
        .source_form
        .as_ref()
        .is_some_and(|form| versions_form::any_popover_open(ctx, &form.card_key));
    let escape_close =
        escape_active && !popover_open && ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mod_name = step2
        .versions_ui
        .source_form
        .as_ref()
        .map_or_else(String::new, sheet_mod_name);
    let mut outcome = render_sheet_shell(
        ctx,
        palette,
        "versions_sheet_edit",
        drawer_rect,
        |ui, outcome| {
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(22, 18))
                .show(ui, |ui| {
                    render_header(ui, palette, "Download source", &mod_name, outcome);
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
                        render_edit_source_footer(ui, palette, step2, env.busy, outcome);
                    });
                });
        },
    );
    if escape_close {
        outcome.close = true;
    }
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
        } else if let Some(message) = source_form::config_files_error(form) {
            form.error = Some(message);
        } else if outcome.action.is_none() {
            form.error = None;
            outcome.action = Some(Step2Action::SaveSourceForm);
        }
    }
    if let Some(error) = form.error.as_deref() {
        render_footer_error(ui, palette, error);
    }
}

fn render_footer_error(ui: &mut egui::Ui, palette: ThemePalette, error: &str) {
    ui.add_space(4.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(error)
                .font(egui::FontId::new(
                    12.0,
                    egui::FontFamily::Name("poppins_light".into()),
                ))
                .color(redesign_error(palette)),
        )
        .wrap_mode(egui::TextWrapMode::Truncate),
    );
}

pub(crate) fn render_forks(
    ctx: &egui::Context,
    palette: ThemePalette,
    drawer_rect: egui::Rect,
    step2: &Step2State,
    escape_active: bool,
) -> SheetOutcome {
    let escape_close = escape_active && ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mut outcome = render_sheet_shell(
        ctx,
        palette,
        "versions_sheet_forks",
        drawer_rect,
        |ui, outcome| {
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(22, 18))
                .show(ui, |ui| {
                    render_header(
                        ui,
                        palette,
                        "Forks",
                        &step2.mod_download_forks_popup_label,
                        outcome,
                    );
                    ui.add_space(12.0);
                    render_forks_list(ui, palette, step2, outcome);
                });
        },
    );
    if escape_close {
        outcome.close = true;
    }
    outcome
}

const FORK_ICON_SLOT: f32 = 30.0;
const FORK_HINT_H: f32 = 18.0;

fn render_fork_open_icon(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    full_name: &str,
) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(FORK_ICON_SLOT, FORK_ICON_SLOT),
        egui::Sense::hover(),
    );
    let response = ui.interact(
        rect,
        ui.id().with(("fork_open", full_name)),
        egui::Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        let color = if response.hovered() {
            redesign_text_primary(palette)
        } else {
            redesign_text_muted(palette)
        };
        versions_icons::paint_glyph(
            ui.painter(),
            rect.center(),
            versions_icons::GLYPH_EXTERNAL_LINK,
            color,
        );
    }
    response.on_hover_text("Open on GitHub")
}

fn forks_note(ui: &mut egui::Ui, palette: ThemePalette, text: &str, size: f32) {
    ui.label(
        egui::RichText::new(text)
            .size(size)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
}

fn render_forks_list(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    step2: &Step2State,
    outcome: &mut SheetOutcome,
) {
    let loading = step2.forks_list.status == ForksStatus::Loading;
    if loading {
        forks_note(ui, palette, "Looking for forks\u{2026}", 13.0);
        ui.add_space(8.0);
    } else if let Some(err) = step2.mod_download_forks_popup_error.as_ref() {
        ui.label(egui::RichText::new(err).color(redesign_error(palette)));
        ui.add_space(8.0);
    } else if step2.mod_download_forks.is_empty() {
        forks_note(ui, palette, "No forks found.", 13.0);
        ui.add_space(8.0);
    }
    let hint_reserve = ui.spacing().item_spacing.y.mul_add(2.0, FORK_HINT_H + 8.0);
    let list_h = (ui.available_height() - hint_reserve).max(0.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(list_h)
        .show(ui, |ui| {
            for fork in &step2.mod_download_forks {
                render_fork_row(
                    ui,
                    palette,
                    fork,
                    &step2.mod_download_forks_popup_tp2,
                    &step2.mod_download_forks_popup_label,
                    outcome,
                );
            }
        });
    ui.add_space(8.0);
    forks_note(
        ui,
        palette,
        "GitHub lists direct forks only. Add a fork it misses as a new source.",
        12.0,
    );
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
                "Use as new source",
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
            if render_fork_open_icon(ui, palette, &fork.full_name).clicked() {
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

pub(crate) struct NoteEnv<'a> {
    pub(crate) sheet_error: Option<&'a str>,
}

pub(crate) fn clear_note_sheet(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove_temp::<NoteSheetState>(note_state_id()));
}

pub(crate) fn render_note(
    ctx: &egui::Context,
    palette: ThemePalette,
    drawer_rect: egui::Rect,
    escape_active: bool,
    env: &NoteEnv<'_>,
) -> SheetOutcome {
    let mut state = ctx
        .data(|d| d.get_temp::<NoteSheetState>(note_state_id()))
        .unwrap_or_default();
    let escape_close = escape_active && ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mut outcome = render_sheet_shell(
        ctx,
        palette,
        "versions_sheet_note",
        drawer_rect,
        |ui, outcome| {
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(22, 18))
                .show(ui, |ui| {
                    render_header(ui, palette, "Note", &state.mod_name, outcome);
                    ui.add_space(12.0);
                    render_note_subtitle(ui, palette, &state);
                    ui.add_space(12.0);
                    render_note_text_area(ui, palette, &mut state);
                    ui.add_space(8.0);
                    if let Some(error) = env.sheet_error {
                        ui.label(egui::RichText::new(error).color(redesign_error(palette)));
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "Shown when you hover this source in the Known sources menu.",
                            )
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_faint(palette)),
                        );
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        render_note_footer(ui, palette, &state, outcome);
                    });
                });
        },
    );
    if escape_close {
        outcome.close = true;
    }
    ctx.data_mut(|d| d.insert_temp(note_state_id(), state));
    outcome
}

fn render_note_subtitle(ui: &mut egui::Ui, palette: ThemePalette, state: &NoteSheetState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} \u{b7}", state.rule_words))
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.label(
            egui::RichText::new(&state.location)
                .font(egui::FontId::monospace(13.0))
                .color(redesign_text_muted(palette)),
        );
    });
}

const NOTE_TEXT_AREA_H: f32 = 120.0;

fn render_note_text_area(ui: &mut egui::Ui, palette: ThemePalette, state: &mut NoteSheetState) {
    let margin = egui::Margin::symmetric(10, 8);
    egui::Frame::default()
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::ZERO)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("versions_note_text_scroll")
                .max_height(NOTE_TEXT_AREA_H)
                .min_scrolled_height(NOTE_TEXT_AREA_H)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_sized(
                        egui::vec2(ui.available_width(), NOTE_TEXT_AREA_H),
                        egui::TextEdit::multiline(&mut state.text)
                            .frame(false)
                            .hint_text(
                                egui::RichText::new("Why this version?")
                                    .color(redesign_text_faint(palette)),
                            )
                            .text_color(redesign_text_primary(palette))
                            .margin(margin)
                            .font(egui::FontId::new(
                                13.0,
                                egui::FontFamily::Name("poppins_light".into()),
                            )),
                    );
                });
        });
}

fn render_note_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &NoteSheetState,
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
    if redesign_btn(
        ui,
        palette,
        "Save note",
        BtnOpts {
            primary: true,
            small: true,
            ..Default::default()
        },
    )
    .clicked()
    {
        outcome.action = Some(Step2Action::SaveSourceNote {
            tp2: state.tp2.clone(),
            signature: state.signature.clone(),
            text: state.text.clone(),
            who: state.who.clone(),
        });
    }
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

    #[test]
    fn new_mod_form_opens_under_its_card_key_as_a_new_mod() {
        let form = seed_new_mod_form("Speedrun EET");
        assert!(form.identity.is_new_mod);
        assert!(form.identity.may_change_id);
        assert!(form.tp2.is_empty());
        assert_eq!(form.source_id, "primary");
        assert_eq!(sheet_mod_name(&form), "New mod");
        assert_eq!(form.save_to, ModSourceEditDestination::ThisModlist);
        assert_eq!(form.note_who, "Speedrun EET");

        let mut step2 = Step2State::default();
        step2
            .versions_ui
            .open_sheet(VersionsSheet::EditSource, NEW_MOD_CARD_KEY.to_string());
        step2.versions_ui.source_form = Some(form);
        assert!(editor_ready_for(&step2));
    }
}
