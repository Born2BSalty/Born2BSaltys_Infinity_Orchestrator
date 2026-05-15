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
use crate::ui::home::add_a_modlist::{self, AddAModlistAction};
use crate::ui::home::state_home::{empty_filter_message, HomeFilter};
use crate::ui::home::{filter_chip, first_launch_setup_card, modlist_card};
use crate::ui::home::modlist_card::ModlistCardActions;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
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

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, _ctx: &egui::Context) {
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
                    ui.set_width(ui.available_width());
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

                        if let Some(act) = render_card_list(
                            ui,
                            palette,
                            effective_filter,
                            &installed,
                            &in_progress,
                        ) {
                            nav_request = Some(act);
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

/// Render the filtered card list (wireframe `flex column gap:10`). Returns a
/// `NavRequest` if a card's primary action fired this frame. An empty
/// filtered list shows the faint per-chip message (SPEC §3.1 "Empty states").
fn render_card_list(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    filter: HomeFilter,
    installed: &[ModlistEntry],
    in_progress: &[ModlistEntry],
) -> Option<NavRequest> {
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
        return None;
    }

    let mut nav: Option<NavRequest> = None;
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
                ModlistCardActions::Open => {
                    // `open` opens the install folder (P5.T17, Run 2). Run 1
                    // intentionally takes no action — the button renders and
                    // is clickable but the folder-open is deferred.
                }
                ModlistCardActions::None => {}
            }
        }
    });
    nav
}
