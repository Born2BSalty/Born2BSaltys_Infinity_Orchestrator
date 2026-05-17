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
// ## Run 4 scope (this run)
//
// The stage machine is declared **whole** (`Choose | ForkPaste | ForkPreview
// | ForkDownload`) so `page_create`'s dispatch + the fork back-navigation are
// total. Run 3 shipped the `Choose` stage (P6.T7) + the Load Draft dialog
// (P6.T9). **Run 4 (P6.T8) replaces the `Fork*` deferred placeholder with
// the real fork sub-flow** — fork-paste / fork-preview / fork-download — plus
// the `operations_create::create_forked_modlist` lineage append (SPEC §5.3 /
// §13.3). The fork stages reuse the Phase-5 chassis (the
// `sub_flow_footer` / `preview_tabs` / `stage_downloading` /
// `ForkInfoPopup`); only the labels + the `Begin Import →` CTA differ and
// there is **no `allow_auto_install` gate** (forking is always allowed —
// SPEC §13.3).
//
// `fork_code` carries the pasted parent share code; `fork_preview` carries
// the parsed `ModlistSharePreview` (cached so the parse runs once on
// `ForkPaste → ForkPreview`, not per-frame — the Install `parsed_preview`
// precedent). `fork_preview_parse_error` / `fork_active_preview_tab` /
// `fork_info_open` are the symmetric fork-preview UI fields (mirroring
// `InstallScreenState`'s `preview_parse_error` / `active_preview_tab` /
// `fork_info_open`).
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
// SPEC: §5.1 (choose mode), §5.2 (Load Draft), §5.3 (fork sub-flow),
//       §13.3 (provenance / lineage append), §10.9 (ForkInfoPopup).

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
use crate::ui::install::stage_downloading::DownloadProgress;
use crate::ui::install::state_install::{DestChoice, PreviewTab};

/// The Create screen's stages (SPEC §5: choose → one of the fork sub-stages).
/// The machine is whole so `page_create`'s dispatch + the fork
/// back-navigation are total. Run 3 shipped `Choose`; **Run 4 (P6.T8) ships
/// the real `Fork*` sub-flow** (reusing the Phase-5 chassis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreateStage {
    /// Initial `choose` mode (SPEC §5.1): the setup Box (name + game +
    /// destination + conditional `DestinationNotEmptyWarning`) and the two
    /// starting-point cards (P6.T7 — Run 3).
    #[default]
    Choose,
    /// Fork sub-flow stage 1 — paste a parent share code
    /// (`ForkPasteScreen`, SPEC §5.3). `stage_fork_paste` (P6.T8 / Run 4).
    ForkPaste,
    /// Fork sub-flow stage 2 — parsed parent preview (`ForkPreviewScreen`,
    /// SPEC §5.3): packed parent `name`/`author` + `⑂ fork info` (the
    /// incoming parent's lineage) + `Begin Import →`, no `allow_auto_install`
    /// gate. `stage_fork_preview` (P6.T8 / Run 4).
    ForkPreview,
    /// Fork sub-flow stage 3 — the fork-download chassis (SPEC §5.3).
    /// `stage_fork_download` wires the chassis navigation + the
    /// post-completion `create_forked_modlist` registry/lineage append +
    /// the route into the forked Workspace (P6.T8 / Run 4); the **live**
    /// fork download/extract is Phase 7 P7.T17 (§13.12a) — the same
    /// forward-compatible chassis model as Install's §4.3 Downloading stage.
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

    // ── Fork sub-flow fields (P6.T8 / Run 4) ──
    /// Pasted parent BIO-MODLIST-V1 share code (fork-paste stage). Empty
    /// disables the fork-paste footer primary (SPEC §5.3 — the `Preview →`
    /// gate, same as Install's paste stage).
    pub fork_code: String,
    /// Parsed parent preview (carve-out #5: carries the provenance trio
    /// `name`/`author`/`forked_from` — the parent's lineage). `None` until
    /// the parse runs on `ForkPaste → ForkPreview`; cached so the parse is
    /// one-shot, not per-frame (the `InstallScreenState::parsed_preview`
    /// precedent). Cleared when the user goes back to fork-paste.
    ///
    /// `pub(crate)` (the rest of the struct is `pub`): `ModlistSharePreview`
    /// is BIO's `pub(crate)` type (carve-out #5 keeps it at its existing
    /// field visibility — not a redesign decision), so this field cannot be
    /// more public than the type it holds (`private_interfaces`). Every
    /// reader is in-crate.
    pub(crate) fork_preview: Option<ModlistSharePreview>,
    /// Set when `preview_modlist_share_code` returned `Err` so the
    /// fork-preview stage shows the failure instead of a blank box (mutually
    /// exclusive with `fork_preview` — a parse yields a preview or an error).
    /// The wireframe assumes a valid code; a real paste-stage parse failure
    /// must be surfaced, not silently swallowed (the Install precedent).
    pub fork_preview_parse_error: Option<String>,
    /// The selected Content-Box tab on the fork-preview stage (SPEC §5.3 ⇒
    /// "identical to Install's preview stage" ⇒ the same 6 tabs / §4.2).
    pub fork_active_preview_tab: PreviewTab,
    /// Whether the `ForkInfoPopup` (SPEC §10.9) is open on the fork-preview
    /// stage — showing the *incoming parent's* lineage. Toggled by the
    /// `⑂ fork info` button; the popup is Close-only + non-blocking.
    pub fork_info_open: bool,
    /// The fork-download per-mod progress model (SPEC §5.3 / §13.12a). Drives
    /// the **reused Phase-5 `stage_downloading` chassis**. Empty until
    /// Phase 7 P7.T17 binds the live fork download/extract pipeline (the
    /// same forward-compatible model as `InstallScreenState::
    /// download_progress`); the chassis renders the §4.3 surface with no
    /// rows + never auto-advances until then. Cleared whenever the user
    /// leaves fork-download back to fork-preview (a re-parse must not
    /// inherit a stale grid).
    pub fork_download_progress: DownloadProgress,

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

    /// Drop the cached fork preview + any parse error and close the fork
    /// popup. Called when the user goes back to fork-paste (the pasted code
    /// may change, so a stale preview must not survive) and just before a
    /// fresh parse runs. The mirror of `InstallScreenState::clear_preview`.
    pub fn clear_fork_preview(&mut self) {
        self.fork_preview = None;
        self.fork_preview_parse_error = None;
        self.fork_info_open = false;
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
        // Fork sub-flow defaults: clean.
        assert!(s.fork_code.is_empty());
        assert!(s.fork_preview.is_none());
        assert!(s.fork_preview_parse_error.is_none());
        assert_eq!(s.fork_active_preview_tab, PreviewTab::Summary);
        assert!(!s.fork_info_open);
    }

    #[test]
    fn clear_fork_preview_resets_fork_preview_state() {
        // Mirrors `InstallScreenState::clear_preview` — back-to-fork-paste /
        // pre-reparse must not leave a stale parent preview / open popup.
        let mut s = CreateScreenState::new();
        s.fork_preview_parse_error = Some("bad code".to_string());
        s.fork_info_open = true;
        s.clear_fork_preview();
        assert!(s.fork_preview.is_none());
        assert!(s.fork_preview_parse_error.is_none());
        assert!(!s.fork_info_open);
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
