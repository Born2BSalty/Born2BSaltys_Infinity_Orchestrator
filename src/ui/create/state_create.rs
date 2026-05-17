// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `CreateScreenState` — per-screen UI state for the Create destination
// (SPEC §5). Lives on `OrchestratorApp::create_screen_state`. Persists across
// screen visits within a session; not written to disk (the typed name /
// chosen game / destination are transient until `start →` creates a registry
// entry — at which point the data lives in `modlists.json` +
// `modlists/<id>/workspace.json`).
//
// Mirrors `wireframe-preview/screens.jsx::CreateScreen` (line 3767-3911):
//   const [mode, setMode]   = useState("choose");          // CreateStage
//   const [modlistName, …]  = useState("");
//   const [game, setGame]   = useState("EET");             // EET default
//   const [dest, setDest]   = useState("");
//   const [destChoice, …]   = useState(null);              // DestChoice
//   const [loadDraftOpen,…] = useState(false);
//
// ## Run 3 scope (this run)
//
// The stage machine is declared **whole** (`Choose | ForkPaste | ForkPreview
// | ForkDownload`) so `page_create`'s dispatch + the fork back-navigation are
// total. Run 3 implements only the `Choose` stage (the setup Box + the two
// starting-point cards — P6.T7) and the Load Draft dialog (P6.T9). The three
// `Fork*` stages render a **deferred placeholder** ("Import-and-modify lands
// in Run 4 (P6.T8)") — the established §4.3-chassis deferral pattern. The
// fork sub-flow renderers + the `operations_create` lineage append are
// **Run 4 (P6.T8)**, NOT this run.
//
// `fork_code` / `fork_preview` are the Run-4 fork-paste/fork-preview fields
// (declared now, inert this run, mirroring the Phase-5 `InstallScreenState`
// staged-field pattern — `parsed_preview` was likewise declared a run early).
//
// `resumed_build_id` records which in-progress build a `resume` (Home card or
// Load Draft dialog) targeted. In the wireframe, `CreateScreen` short-circuits
// to `<WorkspaceView>` when `resumedBuild` is set; in the orchestrator
// architecture the equivalent is `orchestrator.nav =
// NavDestination::Workspace { Some(id) }` (the workspace shell + loader live
// at the router layer — `page_router::render_workspace`, already shipped in
// Run 1 / P6.T12). So this field is the orchestrator-side record of "Create
// kicked off a resume"; the actual workspace open is the nav switch, not a
// `CreateStage`. The same applies to `start →`: it sets the nav directly, so
// there is no `WorkspaceOpen` stage (the plan's earlier inventory listed one;
// the realized architecture routes via nav, never via a Create-local stage —
// keeping a dead stage variant would be a no-op the router never reaches).
//
// SPEC: §5.1 (choose mode), §5.2 (Load Draft), §5.3 (fork sub-flow — Run 4),
//       §13.3 (provenance — Run 4).

// rationale: per-screen UI state struct + trivial query/ctor helpers —
// `Self`/`const fn`/`#[must_use]` churn adds noise without behavior value;
// the doc-paragraph-length lint is subjective style — all Cat 3.
#![allow(
    clippy::use_self,
    clippy::missing_const_for_fn,
    clippy::must_use_candidate,
    clippy::too_long_first_doc_paragraph
)]

use crate::app::modlist_share::ModlistSharePreview;
use crate::registry::model::Game;
use crate::ui::install::state_install::DestChoice;

/// The Create screen's stages (SPEC §5: choose → one of the fork sub-stages).
/// The machine is whole so `page_create`'s dispatch + the fork
/// back-navigation are total. **Run 3 implements `Choose` only;** the three
/// `Fork*` stages render the Run-4 deferred placeholder (the §4.3-chassis
/// deferral pattern). The fork sub-flow renderers are Run 4 (P6.T8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreateStage {
    /// Initial `choose` mode (SPEC §5.1): the setup Box (name + game +
    /// destination + conditional `DestinationNotEmptyWarning`) and the two
    /// starting-point cards. Fully implemented this run (P6.T7).
    #[default]
    Choose,
    /// Fork sub-flow stage 1 — paste a parent share code (SPEC §5.3). Run 4
    /// (P6.T8). This run it renders the deferred placeholder.
    ForkPaste,
    /// Fork sub-flow stage 2 — parsed parent preview (SPEC §5.3). Run 4
    /// (P6.T8). Placeholder this run.
    ForkPreview,
    /// Fork sub-flow stage 3 — the fork-download chassis (SPEC §5.3). Run 4
    /// (P6.T8) wires the navigation + the registry/lineage append; the live
    /// fetch is Phase 7 P7.T17 (§13.12a). Placeholder this run.
    ForkDownload,
}

/// Per-screen Create UI state.
#[derive(Debug, Clone, Default)]
pub struct CreateScreenState {
    /// Active stage. Defaults to `Choose` so a fresh visit from the rail
    /// lands on the setup screen (wireframe `useState("choose")`).
    pub stage: CreateStage,
    /// Typed modlist name (wireframe `modlistName`). Placeholder
    /// "e.g. Tactical EET 2026". Trimmed when passed to `create_modlist`.
    pub modlist_name: String,
    /// Chosen game family. **Defaults to `EET`** (SPEC §5.1: "EET is the
    /// default selection"; wireframe `useState("EET")`). `Game::default()`
    /// is `BGEE`, so this is explicitly initialized to `EET` in
    /// `OrchestratorApp::new` / via `CreateScreenState::new`.
    pub game: Game,
    /// Destination install folder (wireframe `dest`). Set by the `browse`
    /// picker or typed.
    pub destination: String,
    /// The `DestinationNotEmptyWarning` choice (wireframe `destChoice`).
    /// `None` until the user picks one; reset to `None` whenever the
    /// destination changes (a new folder invalidates the prior answer).
    /// Continue-partial is disallowed in Create (SPEC §5.1) — only `Clear` /
    /// `Backup` are offered (the `allow_partial = false` arg to the reused
    /// Phase-5 `destination_not_empty::render`).
    pub destination_choice: Option<DestChoice>,
    /// Whether the Load Draft dialog is open (wireframe `loadDraftOpen`).
    /// Non-blocking `egui::Window` per SPEC §10 — this is the persistent
    /// open-bool the dialog re-renders from each frame.
    pub load_draft_open: bool,

    // ── Run-4 fork fields (declared now, inert this run) ──
    /// Pasted parent share code for the fork sub-flow (Run 4 — P6.T8).
    pub fork_code: String,
    /// Parsed parent preview for the fork sub-flow (Run 4 — P6.T8).
    ///
    /// `pub(crate)` (the rest of the struct is `pub`): `ModlistSharePreview`
    /// is BIO's `pub(crate)` type (carve-out #5 keeps it at its existing
    /// field visibility — not a redesign decision), so this field cannot be
    /// more public than the type it holds (`private_interfaces`). Every
    /// reader is in-crate. Exactly the `InstallScreenState::parsed_preview`
    /// precedent.
    ///
    /// `#[allow(dead_code)]`: genuinely unread in Run 3 — the fork-preview
    /// stage that populates + reads it is Run 4 (P6.T8). Field-scoped, with
    /// the Run-4 reason inline, exactly the established staged-field pattern
    /// (the Run-1 `step2_update_extract_rx` inert-behind-allow precedent —
    /// HANDOFF "Lessons"). Removed when Run 4 wires the reader.
    #[allow(dead_code)]
    pub(crate) fork_preview: Option<ModlistSharePreview>,

    /// The in-progress build id a `resume` targeted (Home card or Load Draft
    /// dialog). Recorded for traceability; the actual workspace open is the
    /// `orchestrator.nav` switch to `Workspace { Some(id) }` (the router owns
    /// the loader — P6.T12, already shipped), not a `CreateStage`.
    pub resumed_build_id: Option<String>,

    /// In-dialog `✓ Copied import code for "<name>"` confirmation text
    /// (SPEC §5.2). `Some(name)` while the transient confirmation is showing
    /// inside the Load Draft dialog; cleared by `page_create` once
    /// `load_draft_copied_until` elapses (the `home` toast-lifetime pattern,
    /// kept here so the dialog stays a pure renderer). `Option<_>` so the
    /// struct's derived `Default` stays valid.
    pub load_draft_copied_name: Option<String>,
    /// Deadline for the in-dialog copy confirmation. `Option<Instant>`
    /// defaults to `None` (derive-`Default`-safe; `Instant` itself is not
    /// `Default`, the `Option` is).
    pub load_draft_copied_until: Option<std::time::Instant>,
}

impl CreateScreenState {
    /// A fresh Create state with the SPEC §5.1 default game (`EET`). Used by
    /// `OrchestratorApp::new` so the game ComboBox shows `EET` on first open
    /// (the wireframe's `useState("EET")`) rather than `Game::default()`
    /// (`BGEE`).
    pub fn new() -> Self {
        Self {
            game: Game::EET,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_to_eet_and_choose_stage() {
        // SPEC §5.1: EET is the default game; wireframe lands on `choose`.
        let s = CreateScreenState::new();
        assert_eq!(s.game, Game::EET);
        assert_eq!(s.stage, CreateStage::Choose);
        assert!(s.modlist_name.is_empty());
        assert!(s.destination.is_empty());
        assert_eq!(s.destination_choice, None);
        assert!(!s.load_draft_open);
        assert_eq!(s.resumed_build_id, None);
    }

    #[test]
    fn derive_default_is_bgee_so_new_is_required_for_eet() {
        // Guard: `Game::default()` is `BGEE` (NOT `EET`). The Create state
        // MUST be built via `new()` (which forces `EET`) — never the bare
        // `Default` — so the game ComboBox shows the SPEC §5.1 default.
        assert_eq!(CreateScreenState::default().game, Game::BGEE);
        assert_eq!(CreateScreenState::new().game, Game::EET);
    }

    #[test]
    fn stage_default_is_choose() {
        assert_eq!(CreateStage::default(), CreateStage::Choose);
    }
}
