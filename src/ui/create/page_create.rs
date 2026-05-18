// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_create` — the Create destination's top-level renderer (SPEC §5).
// Dispatches on `CreateScreenState::stage`.
//
// **Scope.** Run 3 shipped the Create *entry path*: `CreateStage::Choose`
// → `stage_choose::render` (P6.T7); the Load Draft dialog (P6.T9); the
// `start →` / `resume` → Workspace routing (P6.T14 — sets
// `orchestrator.nav = Workspace { Some(id) }`; the workspace loader/route is
// the already-shipped P6.T12 `page_router::render_workspace`). **Run 4
// (P6.T8) replaces the `Fork*` deferred placeholder with the real fork
// sub-flow:**
//   - `ForkPaste`    → `stage_fork_paste::render` (the import-code Box; the
//                       reused Phase-5 `sub_flow_footer`).
//   - `ForkPreview`  → `stage_fork_preview::render` (parsed parent
//                       name/author + the reused Phase-5 `ForkInfoPopup` /
//                       `preview_tabs`; `Begin Import →`, no
//                       `allow_auto_install` gate — SPEC §13.3).
//   - `ForkDownload` → `stage_fork_download::render` (the reused Phase-5
//                       `stage_downloading` chassis; live fetch is Phase 7
//                       P7.T17 — SPEC §13.12a).
// On fork-download `Import`: `create_forked_modlist` (the lineage append —
// SPEC §13.3 / §5.3) + the **caller-anchored** empty `workspace.json` write
// + the atomic `modlists.json` persist (SPEC §13.14) + the route into the
// forked Workspace — the exact `start_scratch` precedent (this caller is
// the only party with `OrchestratorApp` access + the post-mint id). The
// share-code parse is one-shot on `ForkPaste → ForkPreview` (the
// `page_install::run_preview_parse` precedent — cheap to keep, expensive
// per-frame, the pasted code can't change on Preview).
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
// SPEC: §5 (Create), §5.1 (choose mode), §5.2 (Load Draft), §5.3 (fork
//       sub-flow), §13.3 (Provenance / lineage append), §13.12a (live fork
//       fetch is Phase 7), §13.14 (atomic registry add — caller-anchored).

// rationale: the doc-paragraph-length lint is subjective style (Cat 3).
#![allow(clippy::too_long_first_doc_paragraph)]

use std::time::{Duration, Instant};

use eframe::egui;
use tracing::warn;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::registry::operations;
use crate::registry::operations_create::{create_forked_modlist, create_modlist};
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::create::destination_default::default_destination;
use crate::ui::create::load_draft_dialog::{self, LoadDraftOutcome};
use crate::ui::create::stage_choose::{self, ChooseOutcome};
use crate::ui::create::stage_fork_download::{self, ForkDownloadOutcome};
use crate::ui::create::stage_fork_paste::{self, ForkPasteOutcome};
use crate::ui::create::stage_fork_preview::{self, ForkPreviewOutcome};
use crate::ui::create::state_create::CreateStage;
use crate::ui::home::confirm_delete;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};

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
    /// `paste share code →` — enter the fork sub-flow at `ForkPaste`.
    GoForkPaste,
    /// Fork-paste `Back` — return to the `choose` stage.
    ForkPasteBack,
    /// Fork-paste `Preview →` — run the share-code parse (one-shot) and
    /// advance to `ForkPreview`.
    ForkPastePreview,
    /// Fork-preview `← Back` — return to `ForkPaste` (clears the cached
    /// preview; the pasted code may change).
    ForkPreviewBack,
    /// Fork-preview `Begin Import →` — advance to the fork-download chassis
    /// (`ForkDownload`).
    ForkBeginImport,
    /// Fork-download `← Cancel` — return to `ForkPreview` (drops the grid).
    ForkDownloadCancel,
    /// Fork-download completion — `create_forked_modlist` (the SPEC §13.3
    /// lineage append) + the caller-anchored `workspace.json` write + the
    /// atomic `modlists.json` persist + route into the forked Workspace.
    ForkImport,
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
    /// Load Draft Kebab `Delete` (SPEC §5.2 — user-directed deviation) —
    /// arm the danger `ConfirmDialog` over the dialog. Carries the id.
    ArmDeleteDraft(String),
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
        // Fork sub-flow (P6.T8 / SPEC §5.3). Each stage is a pure renderer
        // returning an outcome; the deferred `CreateRequest` below applies
        // the transition / app-level effect after the render borrow ends
        // (the established Run-3 pattern).
        CreateStage::ForkPaste => {
            match stage_fork_paste::render(ui, palette, &mut orchestrator.create_screen_state) {
                ForkPasteOutcome::Back => request = Some(CreateRequest::ForkPasteBack),
                ForkPasteOutcome::Preview => request = Some(CreateRequest::ForkPastePreview),
                ForkPasteOutcome::Stay => {}
            }
        }
        CreateStage::ForkPreview => {
            match stage_fork_preview::render(
                ui,
                palette,
                ctx,
                &mut orchestrator.create_screen_state,
            ) {
                ForkPreviewOutcome::Back => request = Some(CreateRequest::ForkPreviewBack),
                ForkPreviewOutcome::BeginImport => {
                    request = Some(CreateRequest::ForkBeginImport);
                }
                ForkPreviewOutcome::Stay => {}
            }
        }
        CreateStage::ForkDownload => {
            match stage_fork_download::render(
                ui,
                palette,
                &orchestrator.create_screen_state.fork_download_progress,
            ) {
                ForkDownloadOutcome::Cancel => {
                    request = Some(CreateRequest::ForkDownloadCancel);
                }
                ForkDownloadOutcome::Import => request = Some(CreateRequest::ForkImport),
                ForkDownloadOutcome::Stay => {}
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
            LoadDraftOutcome::Delete(id) => {
                request = Some(CreateRequest::ArmDeleteDraft(id));
            }
            LoadDraftOutcome::Pending => {}
        }

        // ── Load Draft `Delete` confirm (SPEC §5.2 — user-directed
        //    deviation). Rendered AFTER the dialog so the danger confirm
        //    floats above it. Uses the EXACT Home delete machinery
        //    (`confirm_delete` text/descriptor + the shared
        //    `confirm_dialog` + `operations::delete_modlist`) — reused, not
        //    reimplemented (the `home::page_home::render_delete_confirm`
        //    path). ──
        render_load_draft_delete_confirm(orchestrator, ctx);
    }

    // ── Apply the deferred app-level effect. ──
    if let Some(req) = request {
        match req {
            CreateRequest::StartScratch => start_scratch(orchestrator),
            CreateRequest::GoForkPaste => {
                // Entering the fork sub-flow fresh: start from a clean
                // fork-paste (a stale code/preview from a prior abandoned
                // fork must not leak in).
                orchestrator.create_screen_state.fork_code.clear();
                orchestrator.create_screen_state.clear_fork_preview();
                orchestrator.create_screen_state.stage = CreateStage::ForkPaste;
            }
            CreateRequest::ForkPasteBack => {
                orchestrator.create_screen_state.clear_fork_preview();
                orchestrator.create_screen_state.stage = CreateStage::Choose;
            }
            CreateRequest::ForkPastePreview => {
                // Parse-on-transition (one-shot — the `page_install`
                // precedent). The result / error is cached on
                // `create_screen_state`; the fork-preview stage renders the
                // error honestly when the parse failed.
                run_fork_preview_parse(&mut orchestrator.create_screen_state);
                orchestrator.create_screen_state.stage = CreateStage::ForkPreview;
            }
            CreateRequest::ForkPreviewBack => {
                // Going back to fork-paste invalidates the cached preview
                // (the pasted code may change before the next preview).
                orchestrator.create_screen_state.clear_fork_preview();
                orchestrator.create_screen_state.stage = CreateStage::ForkPaste;
            }
            CreateRequest::ForkBeginImport => {
                // Entering the fork-download chassis: drop any accumulated
                // grid so a re-entry can't inherit a stale list (the
                // `page_install` Downloading-Cancel precedent).
                orchestrator.create_screen_state.fork_download_progress =
                    crate::ui::install::stage_downloading::DownloadProgress::default();
                orchestrator.create_screen_state.stage = CreateStage::ForkDownload;
            }
            CreateRequest::ForkDownloadCancel => {
                // SPEC §5.3 / §4.3: `Cancel` (← back) returns to
                // fork-preview. Drop the grid (no stale list on re-entry).
                orchestrator.create_screen_state.fork_download_progress =
                    crate::ui::install::stage_downloading::DownloadProgress::default();
                orchestrator.create_screen_state.stage = CreateStage::ForkPreview;
            }
            CreateRequest::ForkImport => fork_import(orchestrator),
            CreateRequest::OpenLoadDraft => {
                orchestrator.create_screen_state.load_draft_open = true;
            }
            CreateRequest::CloseLoadDraft => {
                orchestrator.create_screen_state.load_draft_open = false;
                orchestrator.create_screen_state.load_draft_copied_name = None;
                orchestrator.create_screen_state.load_draft_copied_until = None;
                // A pending delete-confirm must not survive a dialog close.
                orchestrator.create_screen_state.load_draft_delete_target = None;
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
            CreateRequest::ArmDeleteDraft(id) => {
                // SPEC §5.2 (user-directed deviation): arm the danger
                // confirm; the actual delete runs on Confirm in
                // `render_load_draft_delete_confirm` (the Home pattern).
                orchestrator.create_screen_state.load_draft_delete_target = Some(id);
            }
        }
    }
}

/// Render the Load Draft `Delete` confirm if `load_draft_delete_target` is
/// armed (SPEC §5.2 — user-directed deviation). This is the **exact
/// `home::page_home::render_delete_confirm` flow, reused**: the same
/// `confirm_delete::delete_dialog_text` / `delete_confirm` descriptor, the
/// same shared `confirm_dialog::render`, and the same guarded
/// `operations::delete_modlist` (registry entry + safe on-disk folder
/// removal) + the persistence-cycle snapshot anchor so the debounced tick
/// doesn't re-detect a phantom diff. On Confirm the entry is gone from the
/// registry AND disk; the Load Draft list (re-derived from the registry each
/// frame) reflects it immediately. On Cancel: clear the target, no change.
fn render_load_draft_delete_confirm(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let Some(id) = orchestrator
        .create_screen_state
        .load_draft_delete_target
        .clone()
    else {
        return;
    };
    let Some(entry) = orchestrator.registry.find(&id).cloned() else {
        // Entry disappeared (deleted via another path) — disarm.
        orchestrator.create_screen_state.load_draft_delete_target = None;
        return;
    };

    let (title, body) = confirm_delete::delete_dialog_text(&entry);
    // Distinct id-salt from Home's so the two confirm windows never collide
    // if both code paths are ever live (`confirm_dialog` keys on id_salt).
    let dialog = confirm_delete::delete_confirm("load_draft", &title, &body);
    let outcome = confirm_dialog::render(ctx, orchestrator.theme_palette, &dialog);

    match outcome {
        ConfirmOutcome::Confirmed => {
            orchestrator.create_screen_state.load_draft_delete_target = None;
            match operations::delete_modlist(
                &id,
                &orchestrator.registry_store,
                &mut orchestrator.registry,
            ) {
                Ok(_) => {
                    // Keep the persistence cycle's snapshot consistent with
                    // the just-written-through registry so the debounced
                    // tick doesn't re-detect a phantom diff (the exact Home
                    // `render_delete_confirm` post-step).
                    orchestrator.persistence_cycle.last_saved_registry =
                        orchestrator.registry.clone();
                }
                Err(err) => {
                    warn!(
                        target = "orchestrator",
                        "Create Load Draft: delete_modlist failed for {id}: {err}"
                    );
                }
            }
        }
        ConfirmOutcome::Cancelled => {
            orchestrator.create_screen_state.load_draft_delete_target = None;
        }
        ConfirmOutcome::Pending => {}
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

/// SPEC §4.2-authoritative honest fallback for a parent code with no packed
/// `name` (the same string `stage_fork_preview` / Install's preview use —
/// never fabricate a parent name).
const PARENT_FALLBACK_NAME: &str = "Shared modlist";

/// Run the parent share-code parse for the just-pasted fork code and cache
/// the result on `CreateScreenState`. One-shot on `ForkPaste → ForkPreview`
/// (the `page_install::run_preview_parse` precedent — the pasted code can't
/// change while on the preview). On success: `fork_preview = Some`, error
/// cleared, the active tab reset. On failure: `fork_preview_parse_error =
/// Some(msg)` (the fork-preview stage renders the error honestly).
fn run_fork_preview_parse(state: &mut crate::ui::create::state_create::CreateScreenState) {
    state.clear_fork_preview();
    match preview_modlist_share_code(state.fork_code.trim()) {
        Ok(preview) => {
            state.fork_preview = Some(preview);
            state.fork_active_preview_tab =
                crate::ui::install::state_install::PreviewTab::default();
        }
        Err(msg) => {
            state.fork_preview_parse_error = Some(msg);
        }
    }
}

/// Fork-download completion — create the **forked** modlist registry entry
/// with the SPEC §13.3 / §5.3 lineage append, write the canonical empty
/// `workspace.json`, persist `modlists.json` atomically (SPEC §13.14), and
/// route into the forked workspace at Step 2 (the already-shipped P6.T12
/// `render_workspace`, which builds the `⑂ Fork` badge from
/// `entry.forked_from` via `fork_meta_from_entry`).
///
/// **The exact `start_scratch` shape** — `create_forked_modlist` does only
/// the registry-side work + mints the id (zero IO); the empty
/// `workspace.json` write + the atomic persist are caller-side here because
/// only this caller can name the canonical id-bound path AND it must be
/// written before the workspace opens (`page_router::render_workspace`
/// `WorkspaceStore::load` errors on a missing file). Beyond that it differs
/// only in: the fork name (`<parent name> (fork)` — SPEC §5.3 default), the
/// game (the parent code's `game_install`), and the lineage args read off
/// the parsed parent `ModlistSharePreview` (carve-out #5). **No share code
/// is generated** (`pack_meta` is Phase 7).
///
/// Pre-P7.T17 (SPEC §13.12a) the live fork download/extract hasn't run, so
/// the opened Workspace's Step-2 scan isn't populated by real fetched mods —
/// forward-compatible, not a bug (the same model as Install's §4.3 chassis).
fn fork_import(orchestrator: &mut OrchestratorApp) {
    // The parsed parent preview is the source of the lineage + game. It is
    // always `Some` here (the dispatcher only reaches `ForkDownload` after a
    // successful `ForkPaste → ForkPreview` parse); stay total if it isn't.
    let Some(preview) = orchestrator.create_screen_state.fork_preview.clone() else {
        warn!(
            target = "orchestrator",
            "Create fork: import requested with no parsed parent preview \u{2014} ignored"
        );
        return;
    };

    // SPEC §5.3 — the parent's packed name; absent ⇒ the honest fallback
    // (never fabricate). The fork's default name is `<parent name> (fork)`.
    let parent_name = preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(PARENT_FALLBACK_NAME)
        .to_string();
    let parent_author = preview.author.as_deref().unwrap_or("").trim().to_string();
    let fork_name = format!("{parent_name} (fork)");

    // The fork's game = the parent code's `game_install` (the modlist's
    // game travels in the share code; the fork installs the same game).
    let game = crate::registry::model::Game::from_legacy_string(&preview.game_install);

    // The fork flow has no destination input (it is choose → paste → preview
    // → download; the wireframe collects no folder on the fork path). Use
    // the computed per-modlist default (`<config>/modlists/installs/<slug>`)
    // — the same affordance-forward fallback `start_scratch` uses for a
    // blank destination.
    let dest = default_destination(&fork_name);

    // `author` ← the local user's handle (SPEC §13.3; trimmed-empty ⇒ None
    // is handled inside `create_forked_modlist`).
    let user_name = orchestrator.redesign_settings.user_name.clone();

    // 1. Registry-side: mint the id + insert the in-memory `in_progress`
    //    forked entry with the appended lineage (no IO — see
    //    `operations_create`'s module header / `create_forked_modlist`).
    let entry = match create_forked_modlist(
        &fork_name,
        game,
        &dest,
        &user_name,
        &parent_name,
        &parent_author,
        &preview.forked_from,
        &mut orchestrator.registry,
    ) {
        Ok(e) => e,
        Err(err) => {
            warn!(
                target = "orchestrator",
                "Create fork: create_forked_modlist failed: {err}"
            );
            return;
        }
    };

    // 2. Write the empty `workspace.json` at the CANONICAL per-modlist path
    //    the router's loader reads (it `Err`s on a missing file), and
    //    register the store + state in the orchestrator maps.
    let canonical_store = WorkspaceStore::new_for_id(&entry.id);
    let empty = ModlistWorkspaceState::default();
    if let Err(err) = canonical_store.save(&empty) {
        warn!(
            target = "orchestrator",
            "Create fork: writing canonical workspace.json for {} failed: {err} \
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
            "Create fork: atomic registry persist failed: {err} \
             (entry is in memory + workspace.json is on disk; recoverable)"
        );
    }
    orchestrator
        .persistence_cycle
        .mark_registry_dirty(Instant::now());

    // 4. Reset the fork sub-flow form so returning to Create is a clean
    //    slate, and route into the forked workspace at Step 2 (P6.T12
    //    `render_workspace` builds the `⑂ Fork` badge from
    //    `entry.forked_from`).
    let new_id = entry.id;
    orchestrator.create_screen_state.fork_code.clear();
    orchestrator.create_screen_state.clear_fork_preview();
    orchestrator.create_screen_state.fork_download_progress =
        crate::ui::install::stage_downloading::DownloadProgress::default();
    orchestrator.create_screen_state.stage = CreateStage::Choose;
    orchestrator.create_screen_state.resumed_build_id = Some(new_id.clone());
    orchestrator.nav = NavDestination::Workspace {
        modlist_id: Some(new_id),
    };
}
