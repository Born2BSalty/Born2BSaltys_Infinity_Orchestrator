// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_home` — the Home destination's top-level renderer.
//
// Mirrors `wireframe-preview/screens.jsx::HomeScreen` (line 228-427):
//   <div className="sk-page">
//     <ScreenTitle title="Welcome back, adventurer" sub={subSummary} />
//     <div grid 2fr 1fr gap:20 marginBottom:20>
//       <Box padding:16>                 // left: chips + card list
//         <div flex gap:8 marginBottom:12 wrap> …chips… </div>
//         <div flex column gap:10>       … cards | empty-filter line … </div>
//       </Box>
//       <Box label="add a modlist" padding:16> …CTAs + game-installs… </Box>
//     </div>
//   </div>
//
// Branch (SPEC §3.4 / §3.1):
//   - **Empty registry** (no installed AND no in-progress): the left Box's
//     contents are replaced with `first_launch_setup_card`. The right Box
//     still renders normally (a user with a share code can paste it without
//     visiting Settings — SPEC §3.4).
//   - **Non-empty**: filter chips + the filtered card list (SPEC §3.1).
//
// Subtitle (SPEC §3.1): `<N> modlists installed` · `<P> in progress` (if
// P > 0) · `last played <game> <relative>` — empty segments omitted, joined
// by ` · `.
//
// Run 1 scope: chips, card list, first-launch CTA, right column, navigation
// wiring. Kebab items are inert (Run 2). Toast / delete / reinstall surfaces
// are NOT rendered this run.
//
// SPEC: §3.1, §3.2, §3.3, §3.4.

use eframe::egui;

use crate::registry::model::{ModlistEntry, ModlistState};
use crate::registry::operations;
use crate::ui::home::add_a_modlist::{self, AddAModlistAction};
use crate::ui::home::confirm_delete;
use crate::ui::home::state_home::{empty_filter_message, HomeFilter, ToastMessage};
use crate::ui::home::{filter_chip, first_launch_setup_card, modlist_card, toast};
use crate::ui::home::modlist_card::ModlistCardActions;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::settings::state_settings::SettingsTab;
use crate::ui::shared::format_relative::relative_time;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

/// Gap between the two columns + bottom margin (wireframe `gap:20`,
/// `marginBottom:20`).
const COLUMN_GAP_PX: f32 = 20.0;
/// Wireframe grid is `2fr 1fr` → the left column gets 2/3 of the row width.
const LEFT_COLUMN_FRACTION: f32 = 2.0 / 3.0;

/// A navigation request bubbled up from the page body, applied after all
/// immutable borrows of `orchestrator` end (avoids a borrow conflict between
/// the read-only render and the `&mut nav` write).
enum NavRequest {
    Settings { tab: SettingsTab },
    Install,
    Create,
    Workspace { modlist_id: String },
}

/// A non-navigation card intent (Kebab / button) carrying the target modlist
/// id, collected during the read-only card render and applied after the
/// immutable borrows end (same deferral pattern as `NavRequest`).
enum CardIntent {
    /// Copy the entry's import code to the clipboard + toast (P5.T2 / §3.2).
    CopyImportCode(String),
    /// Open the install folder in the OS file manager (P5.T17 / §3.2).
    OpenInstallFolder(String),
    /// Arm the Delete confirm dialog (P5.T7 / §3.1).
    RequestDelete(String),
    /// Arm the Reinstall confirm dialog (P5.T18 / §3.1).
    RequestReinstall(String),
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    // ── Snapshot the registry into owned, render-order lists ──
    // installed-first, in-progress-after (SPEC §3.1 ordering for `All`).
    let installed: Vec<ModlistEntry> = orchestrator
        .registry
        .entries
        .iter()
        .filter(|e| e.state == ModlistState::Installed)
        .cloned()
        .collect();
    let in_progress: Vec<ModlistEntry> = orchestrator
        .registry
        .entries
        .iter()
        .filter(|e| e.state == ModlistState::InProgress)
        .cloned()
        .collect();
    let installed_count = installed.len();
    let in_progress_count = in_progress.len();
    let is_empty = installed_count == 0 && in_progress_count == 0;

    let subtitle = build_subtitle(&installed, in_progress_count);

    // ── Title ──
    render_screen_title(
        ui,
        palette,
        "Welcome back, adventurer",
        Some(&subtitle),
    );

    let mut nav_request: Option<NavRequest> = None;
    let mut card_intent: Option<CardIntent> = None;
    // The effective filter for this frame (explicit choice, else SPEC §3.1
    // default derived from live counts). A chip click overrides it.
    let mut effective_filter = orchestrator
        .home_screen_state
        .effective_filter(installed_count, in_progress_count);
    let mut new_filter: Option<HomeFilter> = None;

    // ── Two-column row (2fr | gap | 1fr) ──
    let row_width = ui.available_width();
    let left_w = ((row_width - COLUMN_GAP_PX) * LEFT_COLUMN_FRACTION).max(0.0);
    let right_w = (row_width - COLUMN_GAP_PX - left_w).max(0.0);

    ui.horizontal_top(|ui| {
        // ---- Left column ----
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                redesign_box(ui, palette, None, |ui| {
                    if is_empty {
                        // SPEC §3.4 — first-launch CTA replaces the chips +
                        // list inside the same Box.
                        if first_launch_setup_card::render(ui, palette) {
                            nav_request = Some(NavRequest::Settings {
                                tab: SettingsTab::Paths,
                            });
                        }
                    } else {
                        // SPEC §3.1 — filter chips + filtered card list.
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

        // ---- Right column (always renders, incl. empty registry) ----
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

    // ── Apply deferred state mutations (after immutable borrows end) ──
    if let Some(f) = new_filter {
        orchestrator.home_screen_state.filter = Some(f);
    }
    if let Some(req) = nav_request {
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

    // ── Card intents (Kebab / button) — applied after render borrows end ──
    if let Some(intent) = card_intent {
        apply_card_intent(orchestrator, ctx, intent);
    }

    // ── Confirm dialogs (P5.T7 / P5.T18) + toast (P5.T16) ──
    // Rendered after the page body so they float above the destination
    // content. Non-blocking per SPEC §10.1.
    render_delete_confirm(orchestrator, ctx);
    render_reinstall_confirm(orchestrator, ctx);
    drive_toast(orchestrator, ctx);
}

/// Apply a non-navigation card intent. Copy / open run immediately;
/// delete / reinstall arm the corresponding confirm dialog.
fn apply_card_intent(
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
    intent: CardIntent,
) {
    match intent {
        CardIntent::CopyImportCode(id) => {
            // SPEC §3.2: writes the build's BIO-MODLIST-V1 code to the
            // clipboard, shows the toast. The registry helper only resolves
            // the code; the clipboard write is egui-built-in (no crate).
            match operations::share_code_for(&id, &orchestrator.registry) {
                Some(code) => {
                    ctx.copy_text(code);
                    let name = modlist_name(orchestrator, &id);
                    orchestrator.home_screen_state.toast = Some(ToastMessage::success(
                        format!("Copied import code for \"{name}\""),
                    ));
                }
                None => {
                    // No code yet (pre-Phase-7 in-progress build). Surface it
                    // rather than silently doing nothing.
                    let name = modlist_name(orchestrator, &id);
                    orchestrator.home_screen_state.toast = Some(ToastMessage::error(
                        format!("No import code yet for \"{name}\""),
                    ));
                }
            }
        }
        CardIntent::OpenInstallFolder(id) => open_install_folder_for(orchestrator, &id),
        CardIntent::RequestDelete(id) => {
            orchestrator.home_screen_state.delete_target = Some(id);
        }
        CardIntent::RequestReinstall(id) => {
            orchestrator.home_screen_state.reinstall_target = Some(id);
        }
    }
}

/// P5.T17 — open the entry's install folder via the OS file manager. On
/// failure (unset / missing folder) surface the error in the bottom-of-screen
/// toast in its error tone (SPEC §3.2: "do not attempt to open or recreate
/// the folder").
fn open_install_folder_for(orchestrator: &mut OrchestratorApp, id: &str) {
    let Some(entry) = orchestrator.registry.find(id).cloned() else {
        return;
    };
    if let Err(msg) = operations::open_install_folder(&entry) {
        orchestrator.home_screen_state.toast = Some(ToastMessage::error(msg));
    }
}

/// The display name for an id, or a graceful fallback if the entry vanished.
fn modlist_name(orchestrator: &OrchestratorApp, id: &str) -> String {
    orchestrator
        .registry
        .find(id)
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "modlist".to_string())
}

/// Render the Delete confirm if `delete_target` is armed. On confirm: call
/// `operations::delete_modlist` (registry entry + guarded on-disk folder),
/// then toast `Deleted "<name>"`. On cancel: clear the target, no change.
fn render_delete_confirm(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let Some(id) = orchestrator.home_screen_state.delete_target.clone() else {
        return;
    };
    let Some(entry) = orchestrator.registry.find(&id).cloned() else {
        // Entry disappeared (e.g. deleted via another path) — disarm.
        orchestrator.home_screen_state.delete_target = None;
        return;
    };

    let (title, body) = confirm_delete::delete_dialog_text(&entry);
    let dialog = confirm_delete::delete_confirm(&id, &title, &body);
    let outcome = confirm_dialog::render(ctx, orchestrator.theme_palette, &dialog);

    match outcome {
        ConfirmOutcome::Confirmed => {
            orchestrator.home_screen_state.delete_target = None;
            let name = entry.name.clone();
            match operations::delete_modlist(
                &id,
                &orchestrator.registry_store,
                &mut orchestrator.registry,
            ) {
                Ok(_) => {
                    // Keep the persistence cycle's snapshot consistent with
                    // the just-written-through registry so the debounced
                    // tick doesn't re-detect a phantom diff.
                    orchestrator
                        .persistence_cycle
                        .last_saved_registry = orchestrator.registry.clone();
                    orchestrator.home_screen_state.toast =
                        Some(ToastMessage::success(format!("Deleted \"{name}\"")));
                }
                Err(err) => {
                    orchestrator.home_screen_state.toast = Some(ToastMessage::error(
                        format!("Couldn't delete \"{name}\": {err}"),
                    ));
                }
            }
        }
        ConfirmOutcome::Cancelled => {
            orchestrator.home_screen_state.delete_target = None;
        }
        ConfirmOutcome::Pending => {}
    }
}

/// Render the Reinstall confirm if `reinstall_target` is armed. Phase 5 is
/// preview-only (P5.T18): on confirm, show the placeholder toast. No
/// reinstall / no Install-preview route — that is Phase 7.
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
            let msg = operations::queue_reinstall_stub(&id, &orchestrator.registry);
            orchestrator.home_screen_state.toast = Some(ToastMessage::success(msg));
        }
        ConfirmOutcome::Cancelled => {
            orchestrator.home_screen_state.reinstall_target = None;
        }
        ConfirmOutcome::Pending => {}
    }
}

/// Render the toast (if any) and drive its ~1.8s auto-dismiss. egui paints
/// lazily, so request a repaint while a toast is live so it actually expires
/// without waiting for the next user event.
fn drive_toast(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let live = toast::render(
        ctx,
        orchestrator.theme_palette,
        orchestrator.home_screen_state.toast.as_ref(),
    );
    if !live {
        return;
    }
    if let Some(t) = orchestrator.home_screen_state.toast.as_ref() {
        if t.is_expired() {
            orchestrator.home_screen_state.toast = None;
        } else {
            // Wake up exactly when the toast's TTL elapses.
            let remaining = toast::TOAST_TTL.saturating_sub(t.shown_at.elapsed());
            ctx.request_repaint_after(remaining);
        }
    }
}

/// SPEC §3.1 subtitle: `<N> modlists installed` · `<P> in progress` (if
/// P > 0) · `last played <game> <relative>` — empty segments omitted.
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

    // "last played <game> <relative>" — the installed entry with the most
    // recent `last_played_date`. Omitted entirely when nothing's been played.
    if let Some(last) = installed
        .iter()
        .filter(|e| e.last_played_date.is_some())
        .max_by_key(|e| e.last_played_date)
    {
        if let Some(when) = last.last_played_date {
            segments.push(format!(
                "last played {} {}",
                last.game.to_legacy_string(),
                relative_time(when)
            ));
        }
    }

    segments.join(" \u{00B7} ")
}

/// Render the chip row (wireframe `flex gap:8 marginBottom:12 wrap`). Returns
/// `Some(filter)` when a chip was clicked. `In progress` is only rendered
/// when `in_progress_count > 0` (SPEC §3.1); `Installed` + `All` always
/// render.
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

/// Render the filtered card list (wireframe `flex column gap:10`). Returns
/// `(nav, intent)` — at most one of each fires per frame (a single click is a
/// single action). An empty filtered list shows the faint per-chip message
/// (SPEC §3.1 "Empty states").
fn render_card_list(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    filter: HomeFilter,
    installed: &[ModlistEntry],
    in_progress: &[ModlistEntry],
) -> (Option<NavRequest>, Option<CardIntent>) {
    // installed-first then in-progress-after for `All` (SPEC §3.1).
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
            match modlist_card::render(ui, palette, entry) {
                ModlistCardActions::Resume => {
                    // SPEC §3.2: `resume` opens the workspace at Step 2,
                    // pre-populated. Phase 6 wires the workspace loader; the
                    // route + modlist id are set here.
                    nav = Some(NavRequest::Workspace {
                        modlist_id: entry.id.clone(),
                    });
                }
                // Installed card's `open` button == Kebab's `Open install
                // folder` (SPEC §3.2 / HANDOFF M6 — v1 alpha opens the folder).
                ModlistCardActions::Open
                | ModlistCardActions::OpenInstallFolder => {
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
                ModlistCardActions::None => {}
            }
        }
    });
    (nav, intent)
}
