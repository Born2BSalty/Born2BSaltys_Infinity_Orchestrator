// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::{ModlistEntry, ModlistState};
use crate::registry::operations::{self, remove_entry_and_save, spawn_delete_folder_worker};
use crate::registry::operations_rename;
use crate::ui::home::add_a_modlist::{self, AddAModlistAction};
use crate::ui::home::confirm_delete;
use crate::ui::home::modlist_card::ModlistCardActions;
use crate::ui::home::reinstall_route_wire;
use crate::ui::home::state_home::{HomeFilter, empty_filter_message};
use crate::ui::home::{filter_chip, first_launch_setup_card, modlist_card};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::orchestrator_app::PendingFolderDelete;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::settings::state_settings::SettingsTab;
use crate::ui::shared::format_relative::relative_time;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

const COLUMN_GAP_PX: f32 = 20.0;
const LEFT_COLUMN_FRACTION: f32 = 2.0 / 3.0;

enum NavRequest {
    Settings { tab: SettingsTab },
    Install,
    Create,
    Workspace { modlist_id: String },
}

enum CardIntent {
    CopyImportCode(String),
    OpenInstallFolder(String),
    RequestDelete(String),
    RequestReinstall(String),
    RequestRename(String),
    SaveRename(String),
    CancelRename,
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    let (installed, in_progress) = split_home_entries(&orchestrator.registry.entries);
    let installed_count = installed.len();
    let in_progress_count = in_progress.len();
    let is_empty = installed_count == 0 && in_progress_count == 0;

    let subtitle = build_subtitle(&installed, in_progress_count);

    render_screen_title(ui, palette, "Welcome back, adventurer", Some(&subtitle));

    let mut nav_request: Option<NavRequest> = None;
    let mut card_intent: Option<CardIntent> = None;
    let mut effective_filter = orchestrator
        .home_screen_state
        .effective_filter(installed_count, in_progress_count);
    let mut new_filter: Option<HomeFilter> = None;

    let row_width = ui.available_width();
    let left_w = ((row_width - COLUMN_GAP_PX) * LEFT_COLUMN_FRACTION).max(0.0);
    let right_w = (row_width - COLUMN_GAP_PX - left_w).max(0.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                redesign_box(ui, palette, None, |ui| {
                    if is_empty {
                        if first_launch_setup_card::render(ui, palette) {
                            nav_request = Some(NavRequest::Settings {
                                tab: SettingsTab::Paths,
                            });
                        }
                    } else {
                        if let Some(picked) = render_filter_chips(
                            ui,
                            palette,
                            effective_filter,
                            installed_count,
                            in_progress_count,
                        ) {
                            new_filter = Some(picked);
                            effective_filter = picked;
                        }

                        ui.add_space(12.0);

                        let (nav, intent) = render_card_list(
                            ui,
                            palette,
                            effective_filter,
                            &installed,
                            &in_progress,
                            orchestrator.home_screen_state.rename_target.as_ref(),
                            &mut orchestrator.home_screen_state.rename_temp,
                        );
                        if let Some(act) = nav {
                            nav_request = Some(act);
                        }
                        if let Some(i) = intent {
                            card_intent = Some(i);
                        }
                    }
                });
            },
        );

        ui.add_space(COLUMN_GAP_PX);

        ui.allocate_ui_with_layout(
            egui::vec2(right_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(right_w);
                match add_a_modlist::render(ui, orchestrator) {
                    AddAModlistAction::PasteImportCode => {
                        nav_request = Some(NavRequest::Install);
                    }
                    AddAModlistAction::CreateYourOwn => {
                        nav_request = Some(NavRequest::Create);
                    }
                    AddAModlistAction::None => {}
                }
            },
        );
    });

    ui.add_space(COLUMN_GAP_PX);

    if let Some(f) = new_filter {
        orchestrator.home_screen_state.filter = Some(f);
    }
    if let Some(req) = nav_request {
        apply_nav_request(orchestrator, req);
    }

    if let Some(intent) = card_intent {
        apply_card_intent(orchestrator, ctx, intent);
    }

    render_delete_confirm(orchestrator, ctx);
    render_reinstall_confirm(orchestrator, ctx);
}

fn apply_nav_request(orchestrator: &mut OrchestratorApp, req: NavRequest) {
    match req {
        NavRequest::Settings { tab } => {
            orchestrator.settings_screen_state.active_tab = tab;
            orchestrator.nav = NavDestination::Settings;
        }
        NavRequest::Install => orchestrator.nav = NavDestination::Install,
        NavRequest::Create => orchestrator.nav = NavDestination::Create,
        NavRequest::Workspace { modlist_id } => {
            orchestrator.nav = NavDestination::Workspace {
                modlist_id: Some(modlist_id),
            };
        }
    }
}

fn split_home_entries(entries: &[ModlistEntry]) -> (Vec<ModlistEntry>, Vec<ModlistEntry>) {
    let installed = entries
        .iter()
        .filter(|e| e.state == ModlistState::Installed)
        .cloned()
        .collect();
    let in_progress = entries
        .iter()
        .filter(|e| e.state == ModlistState::InProgress)
        .cloned()
        .collect();
    (installed, in_progress)
}

fn apply_card_intent(orchestrator: &mut OrchestratorApp, ctx: &egui::Context, intent: CardIntent) {
    match intent {
        CardIntent::CopyImportCode(id) => {
            if let Some(code) = operations::share_code_for(&id, &orchestrator.registry) {
                ctx.copy_text(code);
                let name = modlist_name(orchestrator, &id);
                orchestrator
                    .notification_manager
                    .success(format!("Copied import code for \"{name}\""));
            } else {
                let name = modlist_name(orchestrator, &id);
                orchestrator
                    .notification_manager
                    .error(format!("No import code yet for \"{name}\""));
            }
        }
        CardIntent::OpenInstallFolder(id) => open_install_folder_for(orchestrator, &id),
        CardIntent::RequestDelete(id) => {
            orchestrator.home_screen_state.delete_target = Some(id);
        }
        CardIntent::RequestReinstall(id) => {
            orchestrator.home_screen_state.reinstall_target = Some(id);
        }
        CardIntent::RequestRename(id) => {
            let name = modlist_name(orchestrator, &id);
            orchestrator.home_screen_state.rename_temp = name;
            orchestrator.home_screen_state.rename_target = Some(id.clone());
            let focus_marker = egui::Id::new(("home_card_rename_edit",))
                .with(&id)
                .with("focused_once");
            ctx.memory_mut(|m| m.data.remove::<bool>(focus_marker));
        }
        CardIntent::SaveRename(id) => {
            let new_name = orchestrator
                .home_screen_state
                .rename_temp
                .trim()
                .to_string();
            orchestrator.home_screen_state.rename_target = None;
            orchestrator.home_screen_state.rename_temp.clear();
            if new_name.is_empty() {
                return;
            }
            match operations_rename::rename_modlist(&id, &new_name, &mut orchestrator.registry) {
                Ok(()) => {
                    orchestrator
                        .persistence_cycle
                        .mark_registry_dirty(std::time::Instant::now());
                }
                Err(err) => {
                    orchestrator
                        .notification_manager
                        .error(format!("Couldn't rename to \"{new_name}\": {err}"));
                }
            }
        }
        CardIntent::CancelRename => {
            orchestrator.home_screen_state.rename_target = None;
            orchestrator.home_screen_state.rename_temp.clear();
        }
    }
}

fn open_install_folder_for(orchestrator: &mut OrchestratorApp, id: &str) {
    let Some(entry) = orchestrator.registry.find(id).cloned() else {
        return;
    };
    if let Err(msg) = operations::open_install_folder(&entry) {
        orchestrator.notification_manager.error(msg);
    }
}

fn modlist_name(orchestrator: &OrchestratorApp, id: &str) -> String {
    orchestrator
        .registry
        .find(id)
        .map_or_else(|| "modlist".to_string(), |e| e.name.clone())
}

fn render_delete_confirm(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let Some(id) = orchestrator.home_screen_state.delete_target.clone() else {
        return;
    };
    let Some(entry) = orchestrator.registry.find(&id).cloned() else {
        orchestrator.home_screen_state.delete_target = None;
        return;
    };

    let (title, body) = confirm_delete::delete_dialog_text(&entry);
    let dialog = confirm_delete::delete_confirm(&id, &title, &body);
    let outcome = confirm_dialog::render(ctx, orchestrator.theme_palette, &dialog);

    match outcome {
        ConfirmOutcome::Confirmed => {
            orchestrator.home_screen_state.delete_target = None;
            let name = entry.name;
            match remove_entry_and_save(
                &id,
                &orchestrator.registry_store,
                &mut orchestrator.registry,
            ) {
                Ok(Some(target)) => {
                    orchestrator.persistence_cycle.last_saved_registry =
                        orchestrator.registry.clone();
                    orchestrator
                        .notification_manager
                        .info(format!("Deleting \"{}\"\u{2026}", target.name));
                    let rx = spawn_delete_folder_worker(target.dest);
                    orchestrator
                        .pending_folder_deletes
                        .push(PendingFolderDelete {
                            modlist_name: target.name,
                            rx,
                        });
                }
                Ok(None) => {
                    orchestrator.persistence_cycle.last_saved_registry =
                        orchestrator.registry.clone();
                    orchestrator
                        .notification_manager
                        .success(format!("Deleted \"{name}\""));
                }
                Err(err) => {
                    orchestrator
                        .notification_manager
                        .error(format!("Couldn't delete \"{name}\": {err}"));
                }
            }
        }
        ConfirmOutcome::Cancelled => {
            orchestrator.home_screen_state.delete_target = None;
        }
        ConfirmOutcome::Pending => {}
    }
}

fn render_reinstall_confirm(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let Some(id) = orchestrator.home_screen_state.reinstall_target.clone() else {
        return;
    };
    let Some(entry) = orchestrator.registry.find(&id).cloned() else {
        orchestrator.home_screen_state.reinstall_target = None;
        return;
    };

    let (title, body) = confirm_delete::reinstall_dialog_text(&entry);
    let dialog = confirm_delete::reinstall_confirm(&id, &title, &body);
    let outcome = confirm_dialog::render(ctx, orchestrator.theme_palette, &dialog);

    match outcome {
        ConfirmOutcome::Confirmed => {
            orchestrator.home_screen_state.reinstall_target = None;
            reinstall_route_wire::confirm_reinstall(orchestrator, &id);
        }
        ConfirmOutcome::Cancelled => {
            orchestrator.home_screen_state.reinstall_target = None;
        }
        ConfirmOutcome::Pending => {}
    }
}

fn build_subtitle(installed: &[ModlistEntry], in_progress_count: usize) -> String {
    let mut segments: Vec<String> = Vec::new();

    segments.push(format!(
        "{} modlist{} installed",
        installed.len(),
        if installed.len() == 1 { "" } else { "s" }
    ));

    if in_progress_count > 0 {
        segments.push(format!("{in_progress_count} in progress"));
    }

    if let Some(last) = installed
        .iter()
        .filter(|e| e.last_played_date.is_some())
        .max_by_key(|e| e.last_played_date)
        && let Some(when) = last.last_played_date
    {
        segments.push(format!(
            "last played {} {}",
            last.game.to_legacy_string(),
            relative_time(when)
        ));
    }

    segments.join(" \u{00B7} ")
}

fn render_filter_chips(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    active: HomeFilter,
    installed_count: usize,
    in_progress_count: usize,
) -> Option<HomeFilter> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if filter_chip::render(
            ui,
            palette,
            "Installed",
            installed_count,
            active == HomeFilter::Installed,
        )
        .clicked()
        {
            picked = Some(HomeFilter::Installed);
        }

        if in_progress_count > 0
            && filter_chip::render(
                ui,
                palette,
                "In progress",
                in_progress_count,
                active == HomeFilter::InProgress,
            )
            .clicked()
        {
            picked = Some(HomeFilter::InProgress);
        }

        if filter_chip::render(
            ui,
            palette,
            "All",
            installed_count + in_progress_count,
            active == HomeFilter::All,
        )
        .clicked()
        {
            picked = Some(HomeFilter::All);
        }
    });
    picked
}

fn render_card_list(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    filter: HomeFilter,
    installed: &[ModlistEntry],
    in_progress: &[ModlistEntry],
    rename_target: Option<&String>,
    rename_temp: &mut String,
) -> (Option<NavRequest>, Option<CardIntent>) {
    let visible: Vec<&ModlistEntry> = match filter {
        HomeFilter::Installed => installed.iter().collect(),
        HomeFilter::InProgress => in_progress.iter().collect(),
        HomeFilter::All => installed.iter().chain(in_progress.iter()).collect(),
    };

    if visible.is_empty() {
        ui.label(
            egui::RichText::new(empty_filter_message(filter))
                .size(13.0)
                .family(egui::FontFamily::Proportional)
                .color(redesign_text_faint(palette)),
        );
        return (None, None);
    }

    let mut nav: Option<NavRequest> = None;
    let mut intent: Option<CardIntent> = None;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        for entry in visible {
            let is_renaming = rename_target.is_some_and(|id| id.as_str() == entry.id.as_str());
            let buf = if is_renaming {
                Some(rename_temp as &mut String)
            } else {
                None
            };
            match modlist_card::render(ui, palette, entry, buf) {
                ModlistCardActions::Resume => {
                    nav = Some(NavRequest::Workspace {
                        modlist_id: entry.id.clone(),
                    });
                }
                ModlistCardActions::Open | ModlistCardActions::OpenInstallFolder => {
                    intent = Some(CardIntent::OpenInstallFolder(entry.id.clone()));
                }
                ModlistCardActions::CopyImportCode => {
                    intent = Some(CardIntent::CopyImportCode(entry.id.clone()));
                }
                ModlistCardActions::Reinstall => {
                    intent = Some(CardIntent::RequestReinstall(entry.id.clone()));
                }
                ModlistCardActions::Delete => {
                    intent = Some(CardIntent::RequestDelete(entry.id.clone()));
                }
                ModlistCardActions::Rename => {
                    intent = Some(CardIntent::RequestRename(entry.id.clone()));
                }
                ModlistCardActions::SaveRename => {
                    intent = Some(CardIntent::SaveRename(entry.id.clone()));
                }
                ModlistCardActions::CancelRename => {
                    intent = Some(CardIntent::CancelRename);
                }
                ModlistCardActions::None => {}
            }
        }
    });
    (nav, intent)
}
