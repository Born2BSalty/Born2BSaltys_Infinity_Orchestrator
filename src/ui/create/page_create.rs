// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_create` — the Create destination's top-level renderer (SPEC §5).
// Dispatches on `CreateScreenState::stage`.
//
// **Run 3 scope (the Create *entry path* only).** This run ships:
//   - `CreateStage::Choose` → `stage_choose::render` (P6.T7): the setup Box
//     (name + game ComboBox + destination FolderInput + conditional reused
//     `DestinationNotEmptyWarning`) and the two starting-point cards.
//   - The Load Draft dialog (P6.T9): a non-blocking `egui::Window` listing
//     in-progress builds (the reused Phase-5 `modlist_card`), opened by the
//     `load draft` button.
//   - The `start →` / `resume` → Workspace routing (P6.T14): both set
//     `orchestrator.nav = NavDestination::Workspace { Some(id) }`. The
//     workspace loader/route is the **already-shipped** P6.T12
//     `page_router::render_workspace` — this file only sets the nav.
//   - `CreateStage::ForkPaste|ForkPreview|ForkDownload` → a **deferred
//     placeholder** ("Import-and-modify lands in Run 4 (P6.T8)"). The fork
//     sub-flow renderers + the `operations_create` lineage append are
//     **Run 4 (P6.T8)**, explicitly NOT this run.
//
// **Deferred-intent pattern** (mirrors `home/page_home.rs` +
// `install/page_install.rs`): each stage / dialog renderer is state-only and
// returns an outcome enum; this dispatcher applies the app-level effects
// (create a registry entry, persist `modlists.json` atomically, switch
// `orchestrator.nav`) **after** the render borrow of `orchestrator` ends.
//
// **`start →` create + atomic registry persist (SPEC §13.14).**
// `operations_create::create_modlist` does the in-memory insert + writes the
// empty `workspace.json`; SPEC §13.14 requires registry *adds* to be atomic
// and non-queued, so this dispatcher then calls
// `orchestrator.registry_store.save(&orchestrator.registry)` immediately —
// the exact `operations::delete_modlist`-caller precedent (Home Delete
// persists straight through `RegistryStore::save`). The persistence-cycle's
// debounced diff is a no-op afterward (idempotent — its snapshot is bumped
// via `mark_registry_dirty` so it does not re-write). A persist failure
// surfaces via the bottom-of-screen error path; the in-memory entry stays
// (the workspace.json is already on disk, so the build is still recoverable).
//
// SPEC: §5 (Create), §5.1 (choose mode), §5.2 (Load Draft), §5.3 (fork —
//       Run 4), §13.14 (atomic registry add).

// rationale: the doc-paragraph-length lint is subjective style (Cat 3).
#![allow(clippy::too_long_first_doc_paragraph)]

use std::time::{Duration, Instant};

use eframe::egui;
use tracing::warn;

use crate::registry::operations;
use crate::registry::operations_create::create_modlist;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::create::destination_default::default_destination;
use crate::ui::create::load_draft_dialog::{self, LoadDraftOutcome};
use crate::ui::create::stage_choose::{self, ChooseOutcome};
use crate::ui::create::state_create::CreateStage;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

/// How long the in-dialog `✓ Copied import code` confirmation shows
/// (SPEC §5.2 — transient; the wireframe `setTimeout(…, 1600)`).
const COPY_CONFIRM_MS: u64 = 1600;

/// A deferred app-level transition bubbled up from the choose stage or the
/// Load Draft dialog, applied after the render borrow of `orchestrator` ends
/// (same pattern as `page_install`'s `InstallRequest`). Exactly one intent
/// per frame.
enum CreateRequest {
    /// `start →` — create a from-scratch modlist, persist the registry
    /// atomically, route into its workspace.
    StartScratch,
    /// Enter the fork sub-flow (`CreateStage::ForkPaste` — Run 4).
    GoForkPaste,
    /// Open the non-blocking Load Draft dialog.
    OpenLoadDraft,
    /// Close the Load Draft dialog (Cancel / done).
    CloseLoadDraft,
    /// A `resume` (Load Draft dialog) — open that build's workspace at
    /// Step 2 (P6.T14). Carries the modlist id.
    ResumeWorkspace(String),
    /// Load Draft Kebab `Copy import code` — copy + show the in-dialog
    /// confirmation. Carries the modlist id.
    CopyImportCode(String),
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    // Expire the in-dialog copy confirmation before render (so a stale toast
    // doesn't linger). egui repaints continuously while a window is open, so
    // no explicit repaint request is needed for the ~1.6s expiry.
    if let Some(deadline) = orchestrator.create_screen_state.load_draft_copied_until
        && Instant::now() >= deadline
    {
        orchestrator.create_screen_state.load_draft_copied_name = None;
        orchestrator.create_screen_state.load_draft_copied_until = None;
    }

    let mut request: Option<CreateRequest> = None;

    match orchestrator.create_screen_state.stage {
        CreateStage::Choose => {
            match stage_choose::render(ui, palette, &mut orchestrator.create_screen_state) {
                ChooseOutcome::StartScratch => request = Some(CreateRequest::StartScratch),
                ChooseOutcome::GoForkPaste => request = Some(CreateRequest::GoForkPaste),
                ChooseOutcome::OpenLoadDraft => request = Some(CreateRequest::OpenLoadDraft),
                ChooseOutcome::Stay => {}
            }
        }
        // Run-4 deferred placeholders — the §4.3-chassis deferral pattern.
        // No fork paste/preview/download is implemented this run; the screen
        // renders a clear "lands in Run 4" message and a `← Back` to the
        // choose stage (so it is never a dead end / blank).
        CreateStage::ForkPaste | CreateStage::ForkPreview | CreateStage::ForkDownload => {
            if render_fork_deferred(ui, palette) {
                orchestrator.create_screen_state.stage = CreateStage::Choose;
            }
        }
    }

    // ── Load Draft dialog (non-blocking; rendered above the page body). ──
    if orchestrator.create_screen_state.load_draft_open {
        let copied = orchestrator
            .create_screen_state
            .load_draft_copied_name
            .clone();
        match load_draft_dialog::render(ctx, palette, &orchestrator.registry, copied.as_deref()) {
            LoadDraftOutcome::Cancelled => request = Some(CreateRequest::CloseLoadDraft),
            LoadDraftOutcome::Resume(id) => request = Some(CreateRequest::ResumeWorkspace(id)),
            LoadDraftOutcome::CopyImportCode(id) => {
                request = Some(CreateRequest::CopyImportCode(id));
            }
            LoadDraftOutcome::Pending => {}
        }
    }

    // ── Apply the deferred app-level effect. ──
    if let Some(req) = request {
        match req {
            CreateRequest::StartScratch => start_scratch(orchestrator),
            CreateRequest::GoForkPaste => {
                orchestrator.create_screen_state.stage = CreateStage::ForkPaste;
            }
            CreateRequest::OpenLoadDraft => {
                orchestrator.create_screen_state.load_draft_open = true;
            }
            CreateRequest::CloseLoadDraft => {
                orchestrator.create_screen_state.load_draft_open = false;
                orchestrator.create_screen_state.load_draft_copied_name = None;
                orchestrator.create_screen_state.load_draft_copied_until = None;
            }
            CreateRequest::ResumeWorkspace(id) => {
                // P6.T14 / SPEC §5.2 — close the dialog, open the workspace
                // at Step 2. The router (P6.T12 `render_workspace`, already
                // shipped) does the lookup + loader; we only set the nav.
                orchestrator.create_screen_state.load_draft_open = false;
                orchestrator.create_screen_state.load_draft_copied_name = None;
                orchestrator.create_screen_state.load_draft_copied_until = None;
                orchestrator.create_screen_state.resumed_build_id = Some(id.clone());
                orchestrator.nav = NavDestination::Workspace {
                    modlist_id: Some(id),
                };
            }
            CreateRequest::CopyImportCode(id) => copy_import_code(orchestrator, ctx, &id),
        }
    }
}

/// `start →` — create a from-scratch modlist (P6.T7), write the canonical
/// empty `workspace.json`, persist `modlists.json` atomically (SPEC §13.14),
/// and route into the new workspace at Step 2 (the already-shipped P6.T12
/// `render_workspace`).
///
/// `create_modlist` does only the registry-side work + mints the id (see its
/// module header for the PLAN-GAP-resolution rationale). The empty
/// `workspace.json` write is here because only this caller can name the
/// canonical id-bound path (`WorkspaceStore::new_for_id(&entry.id)`) — and it
/// MUST be written before the workspace opens, because
/// `page_router::render_workspace` calls `WorkspaceStore::load`, which errors
/// on a missing file.
fn start_scratch(orchestrator: &mut OrchestratorApp) {
    let name = orchestrator
        .create_screen_state
        .modlist_name
        .trim()
        .to_string();
    if name.is_empty() {
        // The card has no disabled state (wireframe parity — the Box is the
        // click target with no guard), so guard here: an empty name can't
        // create a registry entry. Surface it rather than silently no-op.
        warn!(
            target = "orchestrator",
            "Create: `start \u{2192}` with an empty modlist name — ignored (name is required)"
        );
        return;
    }
    let game = orchestrator.create_screen_state.game;
    // Honor the user's destination; fall back to the computed default
    // (`<config_dir>/modlists/installs/<slug>`) when left blank. The
    // wireframe starts the FolderInput empty + expects a `browse`, but a
    // blank submit should still produce a usable per-modlist install dir
    // rather than an empty `destination_folder` (affordance-forward).
    let dest = {
        let d = orchestrator.create_screen_state.destination.trim();
        if d.is_empty() {
            default_destination(&name)
        } else {
            d.to_string()
        }
    };

    // 1. Registry-side: mint the id + insert the in-memory `in_progress`
    //    entry (no IO — see `operations_create`'s module header).
    let entry = match create_modlist(&name, game, &dest, &mut orchestrator.registry) {
        Ok(e) => e,
        Err(err) => {
            warn!(
                target = "orchestrator",
                "Create: create_modlist failed: {err}"
            );
            return;
        }
    };

    // 2. Write the empty `workspace.json` at the CANONICAL per-modlist path
    //    the router's loader reads, and register the store + state in the
    //    orchestrator maps so the first `render_workspace` finds a loadable
    //    file (it calls `WorkspaceStore::load`, which `Err`s on a missing
    //    file — `page_router::render_workspace`).
    let canonical_store = WorkspaceStore::new_for_id(&entry.id);
    let empty = ModlistWorkspaceState::default();
    if let Err(err) = canonical_store.save(&empty) {
        warn!(
            target = "orchestrator",
            "Create: writing canonical workspace.json for {} failed: {err} \
             (the router degrades to an empty workspace)",
            entry.id
        );
    }
    orchestrator.workspace_state.insert(entry.id.clone(), empty);
    orchestrator
        .workspace_stores
        .insert(entry.id.clone(), canonical_store);

    // 3. SPEC §13.14 — registry adds are atomic + non-queued. Persist
    //    `modlists.json` immediately (the `delete_modlist`-caller
    //    precedent), then anchor the persistence-cycle debounce so its diff
    //    is a no-op afterward (idempotent).
    if let Err(err) = orchestrator.registry_store.save(&orchestrator.registry) {
        warn!(
            target = "orchestrator",
            "Create: atomic registry persist failed: {err} \
             (entry is in memory + workspace.json is on disk; recoverable)"
        );
    }
    orchestrator
        .persistence_cycle
        .mark_registry_dirty(Instant::now());

    // 4. Reset the choose form so returning to Create is a clean slate, and
    //    route into the new workspace at Step 2 (P6.T12 `render_workspace`).
    let new_id = entry.id;
    orchestrator.create_screen_state.modlist_name.clear();
    orchestrator.create_screen_state.destination.clear();
    orchestrator.create_screen_state.destination_choice = None;
    orchestrator.create_screen_state.resumed_build_id = Some(new_id.clone());
    orchestrator.nav = NavDestination::Workspace {
        modlist_id: Some(new_id),
    };
}

/// Load Draft Kebab `Copy import code` (SPEC §5.2). Resolves the build's
/// BIO-MODLIST-V1 code via `operations::share_code_for`, writes it to the
/// clipboard (egui built-in), and arms the in-dialog `✓ Copied …`
/// confirmation. Pre-Phase-7 in-progress builds may have no code yet — that
/// is surfaced as an honest in-dialog message rather than a silent no-op.
fn copy_import_code(orchestrator: &mut OrchestratorApp, ctx: &egui::Context, id: &str) {
    let name = orchestrator
        .registry
        .find(id)
        .map_or_else(|| "modlist".to_string(), |e| e.name.clone());
    if let Some(code) = operations::share_code_for(id, &orchestrator.registry) {
        ctx.copy_text(code);
        orchestrator.create_screen_state.load_draft_copied_name = Some(name);
    } else {
        // No code yet (no successful install → no `latest_share_code`).
        // Honest in-dialog message (mirrors Home's "No import code yet").
        orchestrator.create_screen_state.load_draft_copied_name = Some(format!(
            "{name}\u{201D} \u{2014} no import code yet \u{201C}"
        ));
    }
    orchestrator.create_screen_state.load_draft_copied_until =
        Some(Instant::now() + Duration::from_millis(COPY_CONFIRM_MS));
}

/// The Run-4 deferred placeholder for the fork sub-flow stages (the
/// §4.3-chassis deferral pattern — a clear "lands in Run 4" panel, never a
/// blank screen or a panic). Returns `true` when `← Back to choose` is
/// clicked so the dispatcher returns to `CreateStage::Choose`.
fn render_fork_deferred(ui: &mut egui::Ui, palette: ThemePalette) -> bool {
    render_screen_title(
        ui,
        palette,
        "Import and modify another modlist",
        Some("paste a share code, preview, then BIO downloads + preselects"),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Import-and-modify lands in Run 4 (P6.T8): fork-paste \u{2192} fork-preview \u{2192} fork-download, the registry entry, and the lineage append. The live mod fetch is Phase 7 (SPEC \u{00A7}13.12a).",
        )
        .size(13.0)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(redesign_text_faint(palette)),
    );
    ui.add_space(16.0);
    crate::ui::orchestrator::widgets::redesign_btn(
        ui,
        palette,
        "\u{2190} Back to choose",
        crate::ui::orchestrator::widgets::BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .clicked()
}
