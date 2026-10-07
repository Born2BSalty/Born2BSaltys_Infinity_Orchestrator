// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::{ModlistEntry, ModlistState};
use crate::registry::operations;
use crate::registry::operations_rename;
use crate::ui::home::add_a_modlist::{self, AddAModlistAction};
use crate::ui::home::confirm_delete;
use crate::ui::home::delete_modlist;
use crate::ui::home::edit_modlist_dialog::{self, EditModlistDialog, EditOutcome};
use crate::ui::home::modlist_card::{CardMenu, ModlistCardActions};
use crate::ui::home::reinstall_route_wire;
use crate::ui::home::state_home::{HomeFilter, empty_filter_message};
use crate::ui::home::{filter_chip, first_launch_setup_card, modlist_card};
use crate::ui::install::state_install::install_stage_is_idle;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};
use crate::ui::orchestrator::widgets::dialogs::share_modlist_dialog::{
    self, ShareModlistDialog, ShareOutcome,
};
use crate::ui::orchestrator::widgets::share_actions;
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
    RequestShare(String),
    OpenGameFolder(String),
    RequestDelete(String),
    RequestReinstall(String),
    RequestEdit(String),
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
                    AddAModlistAction::InstallAModlist => {
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
    render_edit_dialog(orchestrator, ctx);
    render_share_dialog(orchestrator, ctx);
}

fn apply_nav_request(orchestrator: &mut OrchestratorApp, req: NavRequest) {
    match req {
        NavRequest::Settings { tab } => {
            orchestrator.settings_screen_state.active_tab = tab;
            orchestrator.nav = NavDestination::Settings;
        }
        NavRequest::Install => {
            if install_stage_is_idle(orchestrator.install_screen_state.stage) {
                orchestrator.install_screen_state.reset_to_gallery();
                orchestrator.pending_reinstall_id = None;
            }
            orchestrator.nav = NavDestination::Install;
        }
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
        CardIntent::RequestShare(id) => {
            orchestrator.home_screen_state.share_target = Some(id);
        }
        CardIntent::OpenGameFolder(id) => open_game_folder_for(orchestrator, &id),
        CardIntent::RequestDelete(id) => {
            orchestrator.home_screen_state.delete_target = Some(id);
        }
        CardIntent::RequestReinstall(id) => {
            orchestrator.home_screen_state.reinstall_target = Some(id);
        }
        CardIntent::RequestEdit(id) => {
            let Some(entry) = orchestrator.registry.find(&id) else {
                return;
            };
            orchestrator.home_screen_state.edit_name = entry.name.clone();
            orchestrator.home_screen_state.edit_description =
                entry.description.clone().unwrap_or_default();
            orchestrator.home_screen_state.edit_target = Some(id.clone());
            let focus_marker = edit_dialog_id(&id).with("focused_once");
            ctx.memory_mut(|m| m.data.remove::<bool>(focus_marker));
        }
    }
}

fn edit_dialog_id(id: &str) -> egui::Id {
    egui::Id::new(("orchestrator_edit_modlist_dialog", id))
}

fn close_edit_dialog(orchestrator: &mut OrchestratorApp) {
    orchestrator.home_screen_state.edit_target = None;
    orchestrator.home_screen_state.edit_name.clear();
    orchestrator.home_screen_state.edit_description.clear();
}

fn apply_edit_save(orchestrator: &mut OrchestratorApp, id: &str) -> bool {
    let name = orchestrator.home_screen_state.edit_name.clone();
    let description = orchestrator.home_screen_state.edit_description.clone();
    match operations_rename::edit_modlist(id, &name, &description, &mut orchestrator.registry) {
        Ok(()) => {
            orchestrator
                .persistence_cycle
                .mark_registry_dirty(std::time::Instant::now());
            close_edit_dialog(orchestrator);
            true
        }
        Err(err) => {
            orchestrator
                .notification_manager
                .error(format!("Couldn't save \"{name}\": {err}"));
            false
        }
    }
}

fn render_edit_dialog(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let Some(id) = orchestrator.home_screen_state.edit_target.clone() else {
        return;
    };
    if orchestrator.registry.find(&id).is_none() {
        close_edit_dialog(orchestrator);
        return;
    }

    let mut name = orchestrator.home_screen_state.edit_name.clone();
    let mut description = orchestrator.home_screen_state.edit_description.clone();

    let outcome = edit_modlist_dialog::render(
        ctx,
        orchestrator.theme_palette,
        &mut EditModlistDialog {
            id_salt: &id,
            name: &mut name,
            description: &mut description,
        },
    );

    orchestrator.home_screen_state.edit_name = name;
    orchestrator.home_screen_state.edit_description = description;

    match outcome {
        EditOutcome::Saved => {
            apply_edit_save(orchestrator, &id);
        }
        EditOutcome::Cancelled => {
            close_edit_dialog(orchestrator);
        }
        EditOutcome::Pending => {}
    }
}

fn close_share_dialog(orchestrator: &mut OrchestratorApp) {
    orchestrator.home_screen_state.share_target = None;
    orchestrator.share_name_buffer.clear();
}

fn share_target_own_key(id: &str) -> egui::Id {
    egui::Id::new(("home_share_target_is_own", id))
}

fn share_target_is_own(orchestrator: &OrchestratorApp, ctx: &egui::Context, id: &str) -> bool {
    let key = share_target_own_key(id);
    if let Some(own) = ctx.data(|d| d.get_temp::<bool>(key)) {
        return own;
    }
    let own = orchestrator.list_is_own(id);
    ctx.data_mut(|d| d.insert_temp(key, own));
    own
}

fn render_share_dialog(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let Some(id) = orchestrator.home_screen_state.share_target.clone() else {
        return;
    };
    let Some((name, has_code)) = orchestrator.registry.find(&id).map(|entry| {
        let has_code = entry
            .latest_share_code
            .as_deref()
            .is_some_and(|c| !c.trim().is_empty());
        (entry.name.clone(), has_code)
    }) else {
        close_share_dialog(orchestrator);
        return;
    };
    let needed = orchestrator.redesign_settings.user_name.trim().is_empty()
        && share_target_is_own(orchestrator, ctx, &id);

    let outcome = share_modlist_dialog::render(
        ctx,
        orchestrator.theme_palette,
        &mut ShareModlistDialog {
            id_salt: &id,
            modlist_name: &name,
            has_code,
            name_prompt: needed.then_some(&mut orchestrator.share_name_buffer),
        },
    );

    if outcome != ShareOutcome::Pending {
        ctx.data_mut(|d| d.remove::<bool>(share_target_own_key(&id)));
    }

    match outcome {
        ShareOutcome::ExportFile => {
            if let Some(code) = orchestrator.code_for_share(&id) {
                let unresolved = share_actions::unresolved_mods_for_code(&code);
                share_actions::export_modlist_file(
                    &name,
                    &code,
                    &unresolved,
                    &mut orchestrator.notification_manager,
                );
            }
            close_share_dialog(orchestrator);
        }
        ShareOutcome::CopyCode => {
            if let Some(code) = orchestrator.code_for_share(&id) {
                let unresolved = share_actions::unresolved_mods_for_code(&code);
                share_actions::copy_share_code(ctx, &name, &code, &unresolved);
            }
            close_share_dialog(orchestrator);
        }
        ShareOutcome::Closed => {
            close_share_dialog(orchestrator);
        }
        ShareOutcome::Pending => {}
    }
}

fn open_game_folder_for(orchestrator: &mut OrchestratorApp, id: &str) {
    let Some(entry) = orchestrator.registry.find(id).cloned() else {
        return;
    };
    if let Err(msg) = operations::open_game_folder(&entry) {
        orchestrator.notification_manager.error(msg);
    }
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
            delete_modlist::delete_confirmed_modlist(orchestrator, &entry);
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
            match modlist_card::render(ui, palette, entry, CardMenu::Full) {
                ModlistCardActions::Resume => {
                    nav = Some(NavRequest::Workspace {
                        modlist_id: entry.id.clone(),
                    });
                }
                ModlistCardActions::Open | ModlistCardActions::OpenGameFolder => {
                    intent = Some(CardIntent::OpenGameFolder(entry.id.clone()));
                }
                ModlistCardActions::ShareModlist => {
                    intent = Some(CardIntent::RequestShare(entry.id.clone()));
                }
                ModlistCardActions::Reinstall => {
                    intent = Some(CardIntent::RequestReinstall(entry.id.clone()));
                }
                ModlistCardActions::Delete => {
                    intent = Some(CardIntent::RequestDelete(entry.id.clone()));
                }
                ModlistCardActions::EditModlist => {
                    intent = Some(CardIntent::RequestEdit(entry.id.clone()));
                }
                ModlistCardActions::None => {}
            }
        }
    });
    (nav, intent)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::ui::install::state_install::InstallStage;

    fn orch_for_home_test() -> OrchestratorApp {
        OrchestratorApp::new_isolated_for_test("hometest")
    }

    #[test]
    fn install_a_modlist_cta_navigates_to_the_install_screen() {
        let mut app = orch_for_home_test();
        assert_eq!(app.nav, NavDestination::Home);

        apply_nav_request(&mut app, NavRequest::Install);

        assert_eq!(app.nav, NavDestination::Install);
        assert_eq!(
            app.install_screen_state.stage,
            InstallStage::Gallery,
            "the install screen opens on the gallery"
        );
    }

    #[test]
    fn install_cta_resets_a_stale_review_to_the_gallery() {
        let mut app = orch_for_home_test();
        app.install_screen_state.stage = InstallStage::Details;
        app.pending_reinstall_id = Some("REINSTALL0001".to_string());

        apply_nav_request(&mut app, NavRequest::Install);

        assert_eq!(app.install_screen_state.stage, InstallStage::Gallery);
        assert_eq!(app.nav, NavDestination::Install);
        assert!(app.pending_reinstall_id.is_none());
    }

    #[test]
    fn install_cta_leaves_a_live_download_alone() {
        let mut app = orch_for_home_test();
        app.install_screen_state.stage = InstallStage::Downloading;
        app.install_screen_state.destination = "D:\\eet install".to_string();

        apply_nav_request(&mut app, NavRequest::Install);

        assert_eq!(app.install_screen_state.stage, InstallStage::Downloading);
        assert_eq!(app.install_screen_state.destination, "D:\\eet install");
        assert_eq!(app.nav, NavDestination::Install);
    }

    #[test]
    fn create_your_own_cta_navigates_to_the_create_screen() {
        let mut app = orch_for_home_test();

        apply_nav_request(&mut app, NavRequest::Create);

        assert_eq!(app.nav, NavDestination::Create);
    }

    fn orch_with_entry(id: &str, name: &str) -> OrchestratorApp {
        let mut app = orch_for_home_test();
        app.registry.entries.push(ModlistEntry {
            id: id.to_string(),
            name: name.to_string(),
            game: crate::registry::model::Game::EET,
            state: ModlistState::InProgress,
            ..Default::default()
        });
        app
    }

    fn minimal_share_code(name: &str) -> String {
        let json = format!(
            r#"{{
                "format_version": 1,
                "game_install": "BGEE",
                "install_mode": "normal",
                "weidu_logs": {{ "bgee": "~SETUP-X.TP2~ #0 #0 // X" }},
                "name": "{name}"
            }}"#
        );
        crate::app::modlist_share::encode_share_payload_text(&json).expect("mint code")
    }

    #[test]
    fn request_edit_seeds_the_dialog_from_the_entry() {
        let mut app = orch_with_entry("EDIT00000001", "Old Name");
        app.registry.find_mut("EDIT00000001").unwrap().description =
            Some("existing description".to_string());
        let ctx = egui::Context::default();

        apply_card_intent(
            &mut app,
            &ctx,
            CardIntent::RequestEdit("EDIT00000001".to_string()),
        );

        assert_eq!(
            app.home_screen_state.edit_target.as_deref(),
            Some("EDIT00000001")
        );
        assert_eq!(app.home_screen_state.edit_name, "Old Name");
        assert_eq!(
            app.home_screen_state.edit_description,
            "existing description"
        );
    }

    #[test]
    fn saving_the_edit_dialog_updates_the_entry_and_its_code() {
        let mut app = orch_with_entry("EDIT00000002", "Old Name");
        let code = minimal_share_code("Old Name");
        app.registry
            .find_mut("EDIT00000002")
            .unwrap()
            .latest_share_code = Some(code);
        app.home_screen_state.edit_target = Some("EDIT00000002".to_string());
        app.home_screen_state.edit_name = "New Name".to_string();
        app.home_screen_state.edit_description = "BG2EE with the fixpack".to_string();

        let saved = apply_edit_save(&mut app, "EDIT00000002");

        assert!(saved);
        let entry = app.registry.find("EDIT00000002").unwrap();
        assert_eq!(entry.name, "New Name");
        assert_eq!(entry.description.as_deref(), Some("BG2EE with the fixpack"));
        let preview = crate::app::modlist_share::preview_modlist_share_code(
            entry.latest_share_code.as_deref().unwrap(),
        )
        .expect("preview");
        assert_eq!(
            preview.description.as_deref(),
            Some("BG2EE with the fixpack")
        );
        assert!(app.home_screen_state.edit_target.is_none());
    }

    #[test]
    fn cancelling_the_edit_dialog_clears_the_fields() {
        let mut app = orch_with_entry("EDIT00000003", "Old Name");
        app.home_screen_state.edit_target = Some("EDIT00000003".to_string());
        app.home_screen_state.edit_name = "Something Typed".to_string();
        app.home_screen_state.edit_description = "a draft description".to_string();

        close_edit_dialog(&mut app);

        assert!(app.home_screen_state.edit_target.is_none());
        assert_eq!(app.home_screen_state.edit_name.len(), 0);
        assert_eq!(app.home_screen_state.edit_description.len(), 0);
    }
}
