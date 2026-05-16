# Phase 6 — Create screen + Workspace shell (Steps 2-4)

## Summary

Build the Create destination (choose mode + setup Box + starting-point cards + Load Draft dialog + fork-paste/fork-preview/fork-download sub-flow) and the `WorkspaceView` shell that hosts Steps 2-4 inside the orchestrator. The Workspace shell renders the title row with rename + fork badge + save-draft / share-import-code buttons, the 4-step progress bar, the per-step hint line, the active step's content area, and the back/next nav bar.

**Step 2 and Step 3 call BIO's existing public `pub fn render` directly**, passing the orchestrator's own `WizardState` instance. **Step 4's body is replaced with an orchestrator-side renderer (per C4)** that reuses BIO's public save action (`bio::app::step4_weidu_log_export::auto_save_step4_weidu_logs`) and the Step 3 order data (`state.step3.bgee_items` / `bg2ee_items`). No `WizardApp` is hosted; no `WizardApp`-internal handlers are reached. Step 5 inside the workspace is stubbed in Phase 6; Phase 7 wires the install runtime.

The previous plan's "orchestrator bridge" (which translated between a per-modlist registry and a foreign `WizardApp.state`) is replaced with a simpler **workspace state loader** that populates the orchestrator's owned `WizardState` from `modlists/<id>/workspace.json` on workspace open and extracts back on workspace close / nav-away / debounced save.

## What ships after this phase

- `cargo build --bin infinity_orchestrator --release` succeeds.
- Clicking Create in the left rail opens the real Create screen:
  - Title row with `Create / edit modlist` + `load draft` button.
  - Setup Box: modlist-name input + game ComboBox (default EET) + destination FolderInput + `DestinationNotEmptyWarning` when applicable.
  - Two starting-point cards: `New modlist from downloaded mods` and `Import and modify another modlist`.
  - Clicking `start →` on the first card creates a new registry entry in `in-progress` state and routes into the Workspace.
  - Clicking `paste share code →` on the second card opens fork-paste → fork-preview → fork-download → Workspace (the `ImportDownloadScreen` **chassis** is reused from Phase 5). Phase 6 ships the fork navigation + the registry entry + the lineage append; the **live** fork download/extract (and therefore a scan-populated forked workspace) depends on Phase 7 P7.T17 / SPEC §13.12a — the same import → auto-build pipeline + content-addressed staging that all share-code consumption uses.
  - `load draft` button opens the Load Draft dialog listing in-progress builds; clicking `resume` opens the Workspace with that build pre-loaded.
- Inside the Workspace:
  - Header `Editing <name>` + ✎ rename inline edit (registry write only, not folder rename).
  - 4-step progress bar (Step 2 / Step 3 / Step 4 / Step 5) with completion checkmarks.
  - Per-step hint line.
  - Save-draft button (Steps 2-4) that persists current state to `modlists/<id>/workspace.json` and flashes `✓ saved!`.
  - Share import code button (Step 5, disabled until post-install — wired in Phase 7).
  - Nav bar with `← Previous` / `Next →`.
  - Step 2: BIO's `bio::ui::step2::page_step2::render` renders directly (signature returns `Option<Step2Action>`, dispatched via the orchestrator's step-action dispatch layer).
  - Step 3: BIO's `bio::ui::step3::page_step3::render` renders directly (signature returns `()` per H2 — Step 3 has no action enum, handles its own intents internally).
  - Step 4: **orchestrator-side `workspace_step4::render` replaces BIO's `page_step4::render` body** (per C4). The new renderer composes a Save button + count row, the game tab strip, and a line-numbered monospace review list reading from `wizard_state.step3.bgee_items` / `bg2ee_items` (per `src/core/app/state/state_step3.rs:23-24`). The Save button calls `bio::app::step4_weidu_log_export::auto_save_step4_weidu_logs(&mut state)` directly. Exact-log mode shows a read-only viewer + `Check Mod List` button per SPEC §8.2 / A.7.
- Each modlist's workspace state persists per SPEC §13.14:
  - Debounced writes on every change.
  - Flush-on-nav-away from Workspace.
  - Flush-on-exit via `eframe::App::on_exit` (primary, per H4) + `Drop for OrchestratorApp` fallback.

## What's still missing

- Step 5 install runtime — Phase 7.
- `WorkspaceNavBar` ← Previous disable behavior after Install click — Phase 7.
- Share import code button enabled state after successful install — Phase 7.
- Theme-token reskins of the BIO Step 2 tree / details / popups — Phase 8 (Step 2 inside the workspace works correctly in Phase 6; visual polish lands later).
- `modlist-import-code.txt` auto-write on install start — Phase 7.

## Dependencies

- Phase 2 (`OrchestratorApp` owns the `WizardState` that Step pages read/write).
- Phase 3 (registry + workspace state files).
- Phase 4 (Settings — Create reads the default destination from Settings → Paths).
- Phase 5 (`ImportDownloadScreen` **chassis** shared with Phase 6 fork-download; its live download data is bound in Phase 7 P7.T17 / SPEC §13.12a, not Phase 6).

## File inventory

### New files

| Path | Purpose | Depends on |
|------|---------|-----------|
| `src/ui/create/mod.rs` | `pub mod page_create; pub mod state_create; pub mod stage_choose; pub mod stage_fork_paste; pub mod stage_fork_preview; pub mod stage_fork_download; pub mod load_draft_dialog; pub mod destination_default;` | — |
| `src/ui/create/page_create.rs` | Top-level Create renderer. Dispatches on `CreateStage`. | stage_* |
| `src/ui/create/state_create.rs` | `pub struct CreateScreenState { stage: CreateStage, modlist_name: String, game: Game, destination: String, destination_choice: Option<DestChoice>, fork_code: String, fork_preview: Option<ModlistSharePreview>, resumed_build_id: Option<String> }`. `CreateStage::{Choose, ForkPaste, ForkPreview, ForkDownload, WorkspaceOpen { id }}`. | — |
| `src/ui/create/stage_choose.rs` | Renders the choose-mode setup Box + 2 starting-point cards. | redesign widgets, settings |
| `src/ui/create/stage_fork_paste.rs` | Fork-paste stage — paste textarea + footer. Reuses paste textarea component. | shared paste textarea |
| `src/ui/create/stage_fork_preview.rs` | Fork-preview stage — same chassis as Install preview: packed `name`/`author` title/subline + `⑂ fork info` (reuses the Phase-5 `ForkInfoPopup`, showing the incoming parent's lineage), `Begin Import →` CTA, no `allow_auto_install` gate. | `preview_tabs` + `fork_info_popup` from Phase 5 |
| `src/ui/create/stage_fork_download.rs` | Fork-download stage — drives the Phase-5 `stage_downloading` **chassis** (via `DownloadScreenCopy`: title "Downloading fork" + continueLabel "continue to Step 2 →"). Live download/extract data is bound in Phase 7 P7.T17 (SPEC §13.12a); Phase 6 ships the navigation + the post-completion registry/route. | `stage_downloading` chassis from Phase 5 |
| `src/ui/create/load_draft_dialog.rs` | Non-blocking `egui::Window` popup (per SPEC §10) listing in-progress builds with `resume` + Kebab per card. Empty state when none. | registry, modlist_card chassis from Phase 5 |
| `src/ui/create/destination_default.rs` | Computes a default destination folder for new modlists: `<config_dir>/modlists/installs/<slug-of-name>` (or honors a user-configured base path from Settings → Paths). | platform_defaults, settings |
| `src/ui/workspace/mod.rs` | `pub mod workspace_view; pub mod state_workspace; pub mod workspace_header; pub mod workspace_progress_bar; pub mod workspace_nav_bar; pub mod workspace_hint_line; pub mod workspace_step_router; pub mod workspace_state_loader; pub mod workspace_step5_stub; pub mod step4; pub mod widgets;` | — |
| `src/ui/workspace/workspace_view.rs` | The top-level workspace renderer. Owns `WorkspaceViewState`, handles tab switching, embeds the active step content. Signature: `pub fn render(ui, orchestrator: &mut OrchestratorApp, modlist_id: &str, ctx)`. | step_router, state_workspace, state_loader |
| `src/ui/workspace/state_workspace.rs` | `pub struct WorkspaceViewState { modlist_id: String, modlist_name: String, fork_meta: Option<ForkMeta>, game: Game, current_step: WorkspaceStep, completed_steps: HashSet<WorkspaceStep>, renaming: bool, rename_temp: String, save_draft_flash_until: Option<Instant>, share_paste_open: bool, install_complete: bool, loaded_workspace_id: Option<String> }`. Note: `loaded_workspace_id` tracks which modlist's data is currently sitting inside the orchestrator's shared `WizardState` so the loader knows when a swap is needed. | registry workspace model |
| `src/ui/workspace/workspace_header.rs` | Top row: `Editing <name>` + ✎ + Fork badge + `⑂ view fork details` (opens the reused Phase-5 `ForkInfoPopup` when `forked_from` non-empty) + right-side `save draft` / `Share import code` button. | redesign widgets, Phase-5 `fork_info_popup` |
| `src/ui/workspace/workspace_progress_bar.rs` | 4-segment horizontal progress bar with current/completed/upcoming styling. Mirrors `screens.jsx::WorkspaceProgressBar` (line 3186-3244). | redesign theme tokens |
| `src/ui/workspace/workspace_hint_line.rs` | Per-step hint line in faint hand style. | redesign theme tokens |
| `src/ui/workspace/workspace_nav_bar.rs` | Bottom row: `← Previous` (disabled on Step 5 once install starts — Phase 7) + step indicator + `Next →`. | redesign widgets |
| `src/ui/workspace/workspace_step_router.rs` | Dispatches the current step. For Step 2 calls BIO's existing `pub fn render` directly with `&mut orchestrator.wizard_state`; dispatches returned `Option<Step2Action>` via `step_action_dispatch::dispatch_step2`. For Step 3 calls BIO's existing `pub fn render` directly — returns `()` per H2, no dispatch arm needed. For Step 4 calls `workspace_step4::render` (the C4 orchestrator-side wrapper) which composes new chrome around no BIO renderer (C4: BIO's `page_step4::render` is **not** called). For Step 5 calls `workspace_step5_stub::render` (Phase 7 replaces the stub). | BIO step renderers (read-only) |
| `src/ui/workspace/workspace_step5_stub.rs` | Phase-6 placeholder for Step 5 — renders "Step 5 — install runtime arrives in Phase 7" + the WeiDU command preview Box + a disabled `Install` button. | redesign widgets |
| `src/ui/workspace/workspace_state_loader.rs` | The replacement for the previous plan's "orchestrator bridge". Three `pub fn` operations: (a) `populate_wizard_state_from_workspace(workspace: &ModlistWorkspaceState, registry_entry: &ModlistEntry, settings: &Step1Settings, wizard_state: &mut WizardState)` — called on workspace open, writes the per-modlist data into the orchestrator's owned `WizardState` (Step 1 fields from the entry + settings; Step 2/3 order, expand state, prompt overrides from the workspace file; Step 5 fields are reset to defaults — they belong to the install runtime and are tracked separately). (b) `extract_workspace_state_from_wizard(wizard_state: &WizardState) -> ModlistWorkspaceState` — called on save / nav-away / debounce, reads the relevant `pub` fields out of `WizardState` to produce a `ModlistWorkspaceState` for the store. (c) `sync_paths_from_settings(settings_store: &SettingsStore, wizard_state: &mut WizardState)` — called **once on workspace open** (from `populate`) as a defensive re-assert of the canonical `Step1Settings` path/tool fields into `wizard_state.step1`. Per-frame is unnecessary (M2 amended — overview 2026-05-16): the orchestrator's Settings → Paths tab edits the *same in-memory* `wizard_state.step1` the workspace renders from, so Settings edits propagate by construction with no close/reopen and no per-frame disk read. Pure copy; no other state touched. Uses BIO's `pub` API only — no field is private, no method is added. Per C5: **the loader is never invoked while an install is running** (the rail-nav lock prevents nav-into-workspace mid-install). | BIO `WizardState` (read-only), registry workspace model |
| `src/ui/workspace/step_action_dispatch.rs` | A thin dispatch layer that maps each `Step2Action` / `Step4Action` returned by the relevant step page renderers (Step 3 returns `()` per H2 and is excluded) to the corresponding BIO public action-handler entry point in `bio::app::*`. Replicates the same dispatch surface that `WizardApp::handle_step{2,4}_action` uses — read those functions for the dispatch map, then call the underlying `bio::app::*` functions directly. **The orchestrator does not call `WizardApp::handle_*` (which is `WizardApp`-internal); it calls the same `bio::app::*` public functions those handlers call, exactly as `bio::ui::app::update_loop::run` (H3-corrected real path) does.** | BIO `bio::app::*` (read-only) |
| `src/ui/workspace/step4/mod.rs` | `pub mod workspace_step4; pub mod step4_review_list; pub mod step4_save_row; pub mod step4_exact_log_viewer;` — Phase 6 C4 module tree. | — |
| `src/ui/workspace/step4/workspace_step4.rs` | **Per C4: the orchestrator-side renderer for Step 4.** `pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp)`. Composes the Save action row at the top, the game tab strip, and either the line-numbered monospace review list (normal install modes) or the exact-log read-only viewer (when `install_mode == install_exactly_from_weidu_logs`). BIO's `bio::ui::step4::page_step4::render` is **not called**. | step4_review_list, step4_save_row, step4_exact_log_viewer |
| `src/ui/workspace/step4/step4_save_row.rs` | `pub fn render(ui, orchestrator: &mut OrchestratorApp)` — the top action row. Left: `Save weidu.log's` button (or `Save weidu.log` for single-game modlists). Right: count text "_N_ components ready to install on _\<tab\>_ · across _M_ mods". The Save button calls `bio::app::step4_weidu_log_export::auto_save_step4_weidu_logs(&mut orchestrator.wizard_state)` directly. The function is `pub(crate) fn` per `src/core/app/step4_weidu_log_export.rs:50` and takes `&mut WizardState` already — no C2 carve-out #4 refactor needed. | redesign widgets, BIO `step4_weidu_log_export` (read-only) |
| `src/ui/workspace/step4/step4_review_list.rs` | `pub fn render(ui, items: &[Step3ItemState], active_tab: &str)` — the line-numbered monospace review list per SPEC §8.1. Each line is rendered via `widgets/weidu_line::render_weidu_line` using the three-color WeiDU-line syntax per SPEC §6.7. | `widgets/weidu_line`, BIO `Step3ItemState` (read-only) |
| `src/ui/workspace/step4/step4_exact_log_viewer.rs` | `pub fn render(ui, orchestrator: &mut OrchestratorApp)` — the read-only viewer for `install_mode == install_exactly_from_weidu_logs` per SPEC §8.2 / A.7. Reads the configured WeiDU log files from `wizard_state.step1.bgee_log_file` / `bg2ee_log_file` and displays them line-numbered. The `Check Mod List` button triggers `bio::app::app_step4_flow::handle_step4_action(&mut state, Step4Action::CheckMissingMods)` — `pub(crate) fn` per `src/core/app/app_step4_flow.rs:8`, takes `&mut WizardState`, reachable same-crate per the Phase 1 split. No C2 carve-out #4 refactor needed. | redesign widgets, BIO `app_step4_flow` (read-only) |
| `src/ui/workspace/widgets/mod.rs` | `pub mod weidu_line;` — reserved for any workspace-local widgets that aren't broadly reusable. | — |
| `src/ui/workspace/widgets/weidu_line.rs` | `pub fn render_weidu_line(ui, item: &Step3ItemState, line_number: Option<usize>)` — three-color line renderer per SPEC §6.7: `<tp2_file>` in `accent-deep`, `<component_id>` in `text-muted`, `<component_label>` in `text-primary`. Optional line-number prefix in `text-faint` for the Step 4 review list. | redesign theme tokens |
| `src/registry/operations_create.rs` | High-level create operation: `create_modlist(name, game, destination, registry, workspace_store) -> Result<ModlistEntry, RegistryError>`. Allocates an ID, inserts the entry, writes the empty workspace state. For forks, also a `create_forked_modlist(...)` variant that sets `author` (from `RedesignSettings.user_name`) and `forked_from` (`parent.forked_from ++ [{parent.name, parent.author}]` — the append rule, parent fields from the parsed `ModlistSharePreview`). `ModlistEntry` (Phase-3 `src/registry/model.rs`) gains `author: Option<String>` + `forked_from: Vec<ForkAncestor>`, both `#[serde(default)]` (additive to a Phase-3 new file — backward-compatible with existing `modlists.json`). | registry store, workspace store, BIO `ForkAncestor` |
| `src/registry/operations_rename.rs` | `rename_modlist(id, new_name, registry) -> Result<(), RegistryError>`. **Registry rename only** — does not touch the install folder on disk (per SPEC §2.2). | registry store |

### BIO files read from / consumed (no modifications)

- `src/ui/step2/page_step2.rs::render` — Called directly with `&mut orchestrator.wizard_state`. Signature `pub fn render(ui, state: &mut WizardState, dev_mode: bool, exe_fingerprint: &str) -> Option<Step2Action>` (verified in source). Used unchanged.
- `src/ui/step3/page_step3.rs::render` — Called directly. Signature `pub fn render(ui, state: &mut WizardState, dev_mode: bool, exe_fingerprint: &str)` returning `()` per H2. Step 3 has no action enum; the page handles its internal intents directly against `state` (drag-reorder, undo/redo via `step3_history`, etc.). The orchestrator's step router calls it and ignores the return value.
- `src/ui/step4/page_step4.rs::render` — **Per C4: NOT called** for v1 alpha. The orchestrator renders Step 4 via its own `workspace_step4::render`. BIO's `page_step4::render` continues to render normally when invoked by the legacy `BIO` binary's `WizardApp::update_loop`; the orchestrator simply doesn't go through it. File is read-only reference for the BIO behavior the orchestrator's new renderer matches.
- `src/core/app/state/state_wizard.rs::WizardState` — `OrchestratorApp` owns one instance; the loader writes to its `pub` fields directly. No modification.
- `src/core/app/state/state_step1.rs::Step1State` — read/written through `wizard_state.step1`. All fields are `pub`.
- `src/core/app/state/state_step2.rs`, `state_step3.rs`, `state_step5.rs` — read/written through `wizard_state.step{2,3,5}`. All public fields used. **`state_step3.rs:23-24`** — `pub bgee_items: Vec<Step3ItemState>` and `pub bg2ee_items: Vec<Step3ItemState>` are the canonical Step 3 order fields. Each `Step3ItemState` has `pub tp_file`, `pub component_id`, `pub mod_name`, `pub component_label`, `pub raw_line`, `pub prompt_summary`, etc. (all `pub` per `state_step3.rs:5-17`).
- `src/core/app/step4_weidu_log_export.rs::auto_save_step4_weidu_logs` — `pub(crate) fn auto_save_step4_weidu_logs(state: &mut WizardState) -> Result<(), String>` (per source line 50). Takes `&mut WizardState` already; same-crate reachable from orchestrator. **Called directly by `step4_save_row.rs`** — no carve-out #4 refactor needed.
- `src/core/app/app_step4_flow.rs::handle_step4_action` — `pub(crate) fn handle_step4_action(state: &mut WizardState, action: Step4Action)` (per source line 8). Same-crate reachable. Used by the orchestrator's Step 4 wrapper for the `Check Mod List` button in exact-log mode (`Step4Action::CheckMissingMods`).
- `src/core/app/app_step2_router.rs::handle_step2_action` — `pub(crate) fn handle_step2_action(state: &mut WizardState, scan_rx: &mut ..., cancel: &mut ..., progress: &mut ..., update_check_rx: &mut ..., update_download_rx: &mut ..., action: Step2Action)` per `src/core/app/app_step2_router.rs:6`. Takes `&mut WizardState` + the orchestrator-owned channel receivers directly. **The orchestrator calls this function directly** from `step_action_dispatch::dispatch_step2` — no `WizardApp`, no carve-out #4 refactor. The orchestrator owns its own copies of all the receivers, populated when the relevant background task starts (per the BIO pattern).
- `src/core/app/modlist_share.rs::preview_modlist_share_code` — Used in fork-preview. The returned `ModlistSharePreview` now carries the carve-out #5 provenance fields (`name`/`author`/`forked_from`); fork-import reads them to compute the child's `forked_from` (append rule). The `ForkAncestor` struct (defined in `modlist_share.rs` per carve-out #5, Phase 5) is **reused** as the element type of `ModlistEntry.forked_from` — no parallel registry-side type, no drift. Its Phase-5 derive set (`Debug, Clone, PartialEq, Serialize, Deserialize` — pinned in P5.T10 / SPEC §1) already satisfies `ModlistEntry`'s own derives + the `modlists.json` round-trip + the registry `assert_eq!` tests, so **Phase 6 adds no BIO-source edit** — only the additive `author`/`forked_from` fields on the Phase-3-owned `registry/model.rs`.
- `src/core/app/step3_history.rs` — Step 3 undo/redo state. Continues to work inside the embedded Step 3 page (the page reads/writes via `WizardState` fields, no orchestrator involvement needed).
- `src/core/app/app_step2_*`, `app_step3_*`, `app_step4_*` — public action handlers / orchestration called from `step_action_dispatch.rs` and (for Step 4) from the orchestrator's own Step 4 wrapper.

### C2 audit table — Phase 6 functions

Per SPEC §1 carve-out #4 (WizardApp → WizardState refactor), audit each `&mut WizardApp`-taking function that Phase 6 needs.

| Function | Location | C2 audit result | Phase 6 plan |
|----------|----------|-----------------|--------------|
| `WizardApp::handle_step2_action` | `src/ui/app_methods.rs:75` | **Stays as `&mut WizardApp`** — body calls `super::step2_router::handle_step2_action(self, action)` (`src/ui/app_methods.rs:79`) which itself takes `&mut WizardApp` because it mutates `app.step2_scan_rx`, `app.step2_cancel`, `app.step2_progress_queue`, `app.step2_update_check_rx`, `app.step2_update_download_rx`. Channel receivers, not `state`. | Orchestrator does not call `handle_step2_action`. Instead, `step_action_dispatch::dispatch_step2(action, orchestrator)` calls `bio::app::app_step2_router::handle_step2_action(&mut orchestrator.wizard_state, &mut orchestrator.step2_scan_rx, ..., action)` directly. The underlying `bio::app::app_step2_router::handle_step2_action` already takes only `&mut WizardState` + per-receiver `&mut` args (no `WizardApp`); the orchestrator passes its own per-receiver fields. No carve-out #4 refactor required. The two `Step2Action` variants `SelectBgeeViaLog` / `SelectBg2eeViaLog` (which BIO's `step2_router.rs:11-15` intercepts before delegating to `bio::app::app_step2_router`) need orchestrator-side equivalents — see the audit row below. |
| `src/ui/app_step2_log.rs::apply_weidu_log_selection(app: &mut WizardApp, bgee: bool)` | `src/ui/app_step2_log.rs:10` | **Stays as `&mut WizardApp`** — body calls `app.save_settings_best_effort()` (`src/ui/app_step2_log.rs:31`), which is a `WizardApp` method. Mutation surface is `state.step1.{bgee,bg2ee}_log_file` **plus** the settings-store save side-effect via the `WizardApp` impl. Outside carve-out #4's scope (state-only refactor). | **Sibling (simple)** per SPEC §1 decision order. Orchestrator builds `src/ui/workspace/step2_log_glue.rs::apply_weidu_log_selection_for_orchestrator(orchestrator: &mut OrchestratorApp, bgee: bool)`. The new function (a) opens the same `rfd::FileDialog` as BIO does, (b) writes `wizard_state.step1.{bgee,bg2ee}_log_file = picked`, (c) triggers the orchestrator's settings-persistence cycle (debounced write of `bio_settings.json`), (d) calls the underlying `bio::app::app_step2_log::apply_weidu_log_selection_from_path(&mut state, bgee, log_path)` — `pub fn` per source — to update the in-memory state. The two `Step2Action::SelectBgeeViaLog` / `Step2Action::SelectBg2eeViaLog` arms in `step_action_dispatch.rs` call this new function instead of going through `app_step2_log::apply_weidu_log_selection`. No carve-out needed. |
| `WizardApp::handle_step4_action` | `src/ui/app_methods.rs:82` | **Refactor authorized but unnecessary.** Body calls `super::step4_flow::handle_step4_action(&mut self.state, action)` (`src/ui/app_methods.rs:87`), which re-exports `bio::app::app_step4_flow::handle_step4_action(state: &mut WizardState, action: Step4Action)`. The underlying `pub(crate) fn` per `src/core/app/app_step4_flow.rs:8` already takes `&mut WizardState`. | Orchestrator calls `bio::app::app_step4_flow::handle_step4_action(&mut state, action)` directly from the Step 4 wrapper (for `Check Mod List`) and from `step_action_dispatch::dispatch_step4` (for any other `Step4Action` variants returned by a step-page renderer). No carve-out #4 refactor needed. |
| `bio::app::step4_weidu_log_export::auto_save_step4_weidu_logs` | `src/core/app/step4_weidu_log_export.rs:50` | **Already `&mut WizardState`.** | Orchestrator calls directly from `step4_save_row.rs`. No refactor. |

Net effect for Phase 6: zero BIO functions are refactored under carve-out #4. The orchestrator routes around each `WizardApp`-coupled function by either (a) calling the underlying `bio::app::*` function the BIO method wraps (which already takes `&mut WizardState`), or (b) building a net-new orchestrator-side sibling that owns the equivalent side effects (e.g., settings-persistence trigger).

### BIO files needing allowed mild refactor

**None.** Phase 6 keeps every existing step-page and step-state file untouched. The workspace state loader works through BIO's existing `pub` and `pub(crate)` API (both reachable same-crate per the Phase 1 split). It constructs `WizardState`, reads/writes its `pub` fields, and calls `bio::ui::step{2,3}::page_step{N}::render` directly. Step 4 is rendered by the new orchestrator-side `workspace_step4::render`, and reuses `bio::app::step4_weidu_log_export::auto_save_step4_weidu_logs` (already takes `&mut WizardState`) for the Save action.

The previous plan's references to `app_update_loop::run` are corrected to `bio::ui::app::update_loop::run` (H3). The orchestrator never calls this function — that function is `WizardApp`'s frame entry, not a public API for outside use. The orchestrator dispatches by calling the same `bio::app::*` functions the BIO update loop calls.

## Implementation tasks

### P6.T1 — `WorkspaceViewState` + workspace state loader

- **What:** Define `WorkspaceViewState` in `state_workspace.rs`. Implement `workspace_state_loader::populate_wizard_state_from_workspace` and `extract_workspace_state_from_wizard`:

  **Canonical source for order.** `order_by_tab` in `WorkspaceViewState` is a **transient render-only cache** populated from `wizard_state.step3.bgee_items` / `bg2ee_items` per frame. The canonical persisted source is `ModlistWorkspaceState` (in `modlists/<id>/workspace.json`). The cache exists only because the workspace UI reads from a small struct; it must be re-derived from `WizardState` on every frame and never written to independently.

  **populate (workspace → WizardState):**
  - `wizard_state.step1.game_install` ← `entry.game` (the per-modlist game choice).
  - `wizard_state.step1.bgee_game_folder`, `bg2ee_game_folder`, etc. ← copied from the loaded `Step1Settings` (`OrchestratorApp::settings_store`).
  - `wizard_state.step1.mods_folder` ← from settings.
  - `wizard_state.step2.*` ← reconstructed from `workspace.order_<tab>` arrays + cached scan results (the scan is per-modlist mods folder; if the scan cache is fresh, reuse it via BIO's existing scan-cache machinery — read `bio::app::step2::scan::cache` to find the public entry).
  - `wizard_state.step3.bgee_items` / `bg2ee_items` ← reconstructed from order arrays + expand state.
  - `wizard_state.step5.*` ← reset to defaults (install state is not per-workspace-load; it's per-active-install, gated by Phase 7's install concurrency policy).

  **extract (WizardState → workspace):**
  - `workspace.order_bgee` ← derived from `wizard_state.step3.bgee_items`.
  - `workspace.order_bg2ee` ← derived from `wizard_state.step3.bg2ee_items`.
  - `workspace.expand_state` ← `wizard_state.step2.expand_state`-equivalent.
  - `workspace.step3_group_collapse` ← derived from `wizard_state.step3.bgee_collapsed_blocks` + `bg2ee_collapsed_blocks` (`pub` per `state_step3.rs:44-45`).
  - `workspace.prompt_overrides` ← per-workspace prompt overrides.
  - `workspace.last_share_code` ← updated post-install by Phase 7.

  **C5 — never invoked during install.** The loader is gated by the rail-nav lock (Phase 7 P7.T9b): when `install_runtime::install_concurrency::install_in_progress(orchestrator)` returns `Some`, the user cannot navigate to a different workspace, so `populate_wizard_state_from_workspace` is never invoked mid-install.

- **Where:** New files `state_workspace.rs` + `workspace_state_loader.rs`.
- **Acceptance:** Opening a workspace populates `WizardState` such that Step 2's tree, Step 3's order, and Step 4's review all show the modlist's persisted data. Switching modlists swaps the data cleanly. **Per the new architecture, `wizard_state.step1.game_install` lives only in the orchestrator's in-memory `WizardState` — setting it does not write to `bio_settings.json`.**
- **SPEC:** §13.1 (per-modlist workspace state), §13.14 (persistence timing), §5.1 (game choice immutable per-workspace).

### P6.T2 — `workspace_step_router::render`

- **What:** Dispatches the current `WorkspaceStep` to the appropriate renderer. **All dispatch happens at the router layer for consistency.** Step wrappers render the chrome but return the action up; the router dispatches every action via `step_action_dispatch::dispatch_stepN(action, orchestrator)`. The body looks like:
  ```rust
  match state.current_step {
      WorkspaceStep::Step2 => {
          let action = bio::ui::step2::page_step2::render(ui, &mut orchestrator.wizard_state, orchestrator.dev_mode, &orchestrator.exe_fingerprint);
          if let Some(a) = action {
              step_action_dispatch::dispatch_step2(a, orchestrator);
          }
      }
      WorkspaceStep::Step3 => {
          bio::ui::step3::page_step3::render(ui, &mut orchestrator.wizard_state, orchestrator.dev_mode, &orchestrator.exe_fingerprint);
          // Per H2: Step 3 returns (); no action dispatch arm. The page handles its own intents via direct state mutation.
      }
      WorkspaceStep::Step4 => {
          let action = bio::ui::workspace::step4::workspace_step4::render(ui, orchestrator); // C4 — orchestrator-side renderer; BIO's page_step4::render is NOT called
          if let Some(a) = action {
              step_action_dispatch::dispatch_step4(a, orchestrator);
          }
      }
      WorkspaceStep::Step5 => bio::ui::workspace::workspace_step5_stub::render(ui, orchestrator),
  }
  ```
  Step 2 calls BIO's existing public `pub fn render` directly with no orchestrator wrapping (the wireframe content of Step 2 is unchanged from today's BIO). Step 3 calls BIO's existing public `pub fn render` and ignores the `()` return (per H2). Step 4 calls the orchestrator-side `workspace_step4::render` (per C4 — see P6.T2b) and its returned `Option<Step4Action>` is dispatched uniformly via the router's `dispatch_step4` arm. **All dispatch happens at the router layer for consistency.** Step wrappers render the chrome but return the action up; the router dispatches.
- **Where:** New file `workspace_step_router.rs`. Plus `step_action_dispatch.rs` for Step 2 action handling.
- **Acceptance:** Step 2 renders the existing BIO tree with full functionality (search, checkbox toggles, drag, pills, etc.). Step 3 renders the drag-reorder list. Step 4 renders via the new C4 wrapper. Each step's returned action is dispatched via the same `bio::app::*` public entry points BIO's `WizardApp` ultimately uses (verified via the C2 audit table).
- **SPEC:** §2.2, §6, §7, §8.

### P6.T2b — Step 4 orchestrator-side renderer (C4 — replaces BIO's `page_step4::render`)

- **What:** Build `src/ui/workspace/step4/workspace_step4.rs` — a complete orchestrator-side Step 4 renderer that replaces BIO's `page_step4::render` for the workspace flow. BIO's `bio::ui::step4::page_step4::render` is **not called** by the workspace step router. (The legacy `BIO` binary's `WizardApp` continues to invoke `page_step4::render` normally — the orchestrator simply doesn't.)

  **Wrapper signature:** `pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> Option<Step4Action>`. The wrapper **returns** any `Step4Action` produced by button clicks (Save, Check Mod List) to the router; the router dispatches uniformly via `step_action_dispatch::dispatch_step4(action, orchestrator)` per M11. (The Save row can additionally surface the save-error popup inline — that's a render-side concern, not a dispatch action.) **All dispatch happens at the router layer for consistency.**

  Per SPEC §8.1, the orchestrator's Step 4 panel renders (top-down):

  1. **Save action row.** New `step4_save_row::render(ui, orchestrator) -> Option<Step4Action>` composes:
     - Left: `Save weidu.log's` button (for dual-game modlists) or `Save weidu.log` (single-game). Button uses `redesign_btn` with primary styling.
     - Right: count text "_N_ components ready to install on _\<active tab\>_ · across _M_ mods", rendered in `redesign_label_hand` style. Counts come from `wizard_state.step3.bgee_items.len()` or `bg2ee_items.len()` (per the active tab from `wizard_state.step3.active_game_tab`) and unique TP2 count.
     - Save button click: **returns `Some(Step4Action::SaveWeiduLog)` to the router**, which dispatches via `dispatch_step4` → `bio::app::app_step4_flow::handle_step4_action(&mut state, Step4Action::SaveWeiduLog)` (the function is `pub(crate) fn` per `src/core/app/app_step4_flow.rs:8` and routes to `auto_save_step4_weidu_logs` internally). On `Err(msg)` from the save call, the orchestrator surfaces the error via the same save-error popup BIO uses today (the popup is a render-side concern; it does not bubble back up as an action).

  2. **Game tab strip** (for EET dual-game modlists only — single-game modlists skip this row). Renders a 2-tab strip (`BGEE` / `BG2EE`) using `bio::ui::settings::widgets::tab_strip::render` (the same file-folder pattern from Phase 4). Active tab is written to `wizard_state.step3.active_game_tab`.

  3. **Body — branches on install mode:**
     - **Normal install modes** (most cases): `step4_review_list::render(ui, items, active_tab)` where `items` is `&wizard_state.step3.bgee_items` or `bg2ee_items` (per the active tab). Each row in the list is one line; the renderer uses `widgets/weidu_line::render_weidu_line(ui, item, line_number)` to lay out the three-color WeiDU line syntax per SPEC §6.7 (`<tp2_file>` in `accent-deep`, `<component_id>` in `text-muted`, `<component_label>` in `text-primary`) with an optional line-number prefix in `text-faint`. The list is wrapped in `egui::ScrollArea::vertical()` for scrollability.
     - **`install_mode == install_exactly_from_weidu_logs`** (per SPEC §8.2 / Appendix A.7): `step4_exact_log_viewer::render(ui, orchestrator)`. The viewer reads the configured WeiDU log files from `wizard_state.step1.bgee_log_file` / `bg2ee_log_file` (or whichever applies per game) and displays them in a read-only line-numbered monospace pane. A `Check Mod List` button below the viewer triggers `bio::app::app_step4_flow::handle_step4_action(&mut orchestrator.wizard_state, Step4Action::CheckMissingMods)` — `pub(crate) fn` per `src/core/app/app_step4_flow.rs:8`, takes `&mut WizardState` directly. No `WizardApp` involvement; no carve-out #4 refactor.

  **Step 4 is no longer BIO-fidelity for the UI.** SPEC §8.1 + §8.2 BIO-fidelity callouts for the **save action** stay (the save logic is reused via `bio::app::step4_weidu_log_export::auto_save_step4_weidu_logs`); the rendering surface (the Save row, the tab strip, the review list, the exact-log viewer) is **net-new** in `src/ui/workspace/step4/`. The Save button, Check Mod List button, and exact-log file reads go through BIO's existing `pub`/`pub(crate)` API.

- **Where:** New files `src/ui/workspace/step4/mod.rs` + `workspace_step4.rs` + `step4_save_row.rs` + `step4_review_list.rs` + `step4_exact_log_viewer.rs` + `widgets/weidu_line.rs`.
- **Acceptance:** Step 4 in the workspace shows the new chrome at the top (Save button + count + game tab strip) and the line-numbered monospace review list (normal modes) or exact-log viewer (`install_exactly_from_weidu_logs` mode). Clicking the Save button persists the WeiDU log via BIO's existing save path — verifiable by `auto_save_step4_weidu_logs` being called with `&mut wizard_state` and the file appearing on disk at the resolved path. For `install_exactly_from_weidu_logs` modlists, the read-only log viewer + Check Mod List button render instead of the editable review. The Check Mod List button triggers the same update-check flow as BIO's exact-log Step 4 today.
- **SPEC:** §8.1, §8.2, §6.7 (WeiDU line three-color syntax), Appendix A.7.

### Step action dispatch surface

This sub-section enumerates the action variants the orchestrator must dispatch from each step page renderer's return value, the BIO function that today's `WizardApp` calls for each variant, and the orchestrator's implementation strategy per SPEC §1's decision order (direct reuse → sibling for simple workflows → carve-out for complex workflows).

**Conventions.**

- "Direct call" means the orchestrator's `step_action_dispatch` calls the named `bio::app::*` function with `&mut orchestrator.wizard_state` plus any orchestrator-owned channel receivers. No new code beyond the dispatch arm.
- "Sibling (simple)" means a small orchestrator-side file (under `src/ui/workspace/` or sibling) re-implements the surface that touches non-`state` fields. The sibling calls the underlying `bio::app::*` public helper where one exists. No carve-out.
- "Complex / already covered" means the workflow is complex but an existing carve-out (or an already-planned mitigation) handles it.

**Note on call-site channel receivers.** The orchestrator owns its own copies of every channel receiver `bio::app::app_step2_router::handle_step2_action` mutates: `step2_scan_rx`, `step2_cancel`, `step2_progress_queue`, `step2_update_check_rx`, `step2_update_download_rx`. They are added to `OrchestratorApp` in Phase 6 (alongside `WizardState`) and are populated by the same `bio::app::app_step2_*` channel-creation functions BIO uses (`pub` per source).

#### Step 2 — `Step2Action` (variants per `src/core/app/state/step2_action.rs:5-53`, re-exported as `crate::ui::step2::action_step2::Step2Action`)

BIO's `WizardApp::handle_step2_action` (`src/ui/app_methods.rs:75-80`) delegates to `super::step2_router::handle_step2_action` (`src/ui/app_step2_router.rs:6-27`), which intercepts the two log-picker variants and forwards everything else to the underlying `bio::app::app_step2_router::handle_step2_action` (`src/core/app/app_step2_router.rs:15-291`).

| Variant | Payload | BIO dispatch target (file:line) | Reachable from orchestrator? | Orchestrator implementation |
|---------|---------|---------------------------------|------------------------------|-----------------------------|
| `StartScan` | — | `bio::app::app_step2_router::handle_step2_action` → `app_step2_scan::start_step2_scan` (`src/core/app/app_step2_router.rs:29-34`) | Yes — `app_step2_router::handle_step2_action` is `pub(crate) fn`, same-crate reachable | **Direct call** to `bio::app::app_step2_router::handle_step2_action(&mut wizard_state, &mut step2_scan_rx, &mut step2_cancel, &mut step2_progress_queue, &mut step2_update_check_rx, &mut step2_update_download_rx, Step2Action::StartScan)`. |
| `CancelScan` | — | Same router → `app_step2_scan::cancel_step2_scan` (`src/core/app/app_step2_router.rs:35`) | Yes | **Direct call** via the same router. |
| `SelectBgeeViaLog` | — | BIO's `app_step2_router` (UI layer) intercepts at `src/ui/app_step2_router.rs:11-13` and calls `super::step2_log::apply_weidu_log_selection(app, true)` (`src/ui/app_step2_log.rs:10-34`). The UI helper body touches `app.save_settings_best_effort()` (line 31) — outside carve-out #4's scope (touches more than `app.state`). | The underlying `bio::app::app_step2_log::apply_weidu_log_selection_from_path(state, bgee, log_path)` is `pub(crate) fn` (`src/core/app/app_step2_log.rs:31`), same-crate reachable. The `WizardApp` wrapper itself is not. | **Sibling (simple)** per SPEC §1 decision order — `src/ui/workspace/step2_log_glue.rs::apply_weidu_log_selection_for_orchestrator(orchestrator, bgee: true)` opens the same `rfd::FileDialog`, writes `wizard_state.step1.bgee_log_file`, triggers the orchestrator's settings-persistence cycle (debounced `bio_settings.json` write), then calls the underlying `bio::app::app_step2_log::apply_weidu_log_selection_from_path` directly. (Already documented in the C2 audit table above.) No carve-out needed. |
| `SelectBg2eeViaLog` | — | Same UI router intercept → `apply_weidu_log_selection(app, false)` | Same | **Sibling (simple)** — same `step2_log_glue.rs` helper with `bgee: false`. |
| `OpenUpdatePopup` | — | `bio::app::app_step2_router::handle_step2_action` → mutates `state.step2.update_selected_*_target_*` flags (`src/core/app/app_step2_router.rs:36-42`) | Yes | **Direct call** via router. |
| `CheckExactLogModList` | — | Router → `app_step2_saved_log_flow::queue_exact_log_update_preview` (`src/core/app/app_step2_router.rs:43-50`) | Yes | **Direct call** via router. |
| `PreviewUpdateSelected` | — | Router → `app_step2_update_preview::preview_update_selected` (`src/core/app/app_step2_router.rs:103-110`) | Yes | **Direct call** via router. |
| `PreviewUpdateSelectedMod` | — | Router → `app_step2_update_preview::preview_update_selected_mod` (`src/core/app/app_step2_router.rs:111-118`) | Yes | **Direct call** via router. |
| `DownloadUpdates` | — | Router → `app_step2_update_download::start_step2_update_download` (`src/core/app/app_step2_router.rs:51-56`) | Yes | **Direct call** via router. |
| `AcceptLatestForExactVersionMisses` | — | Router → `app_step2_update_check::start_step2_update_check` (`src/core/app/app_step2_router.rs:57-102`) | Yes | **Direct call** via router. |
| `OpenSelectedReadme(String)` | path | Router → `open_in_shell` (`src/core/app/app_step2_router.rs:122-130`) | Yes | **Direct call** via router. |
| `OpenSelectedWeb(String)` | url | Same arm | Yes | **Direct call** via router. |
| `OpenSelectedTp2Folder(String)` | path | Same arm | Yes | **Direct call** via router. |
| `OpenSelectedTp2(String)` | path | Same arm | Yes | **Direct call** via router. |
| `OpenSelectedIni(String)` | path | Same arm | Yes | **Direct call** via router. |
| `OpenModDownloadsUserSource` | — | Router → `mod_downloads::ensure_mod_downloads_files` + `open_in_shell` (`src/core/app/app_step2_router.rs:179-188`) | Yes | **Direct call** via router. |
| `ReloadModDownloadSources` | — | Router → `mod_downloads::ensure_mod_downloads_files` + `mod_downloads::load_mod_download_sources` (`src/core/app/app_step2_router.rs:189-201`) | Yes | **Direct call** via router. |
| `OpenModDownloadSourceEditor { tp2, label, source_id, allow_source_id_change }` | named fields | Router → `mod_downloads::load_user_mod_download_source_block` + state writes (`src/core/app/app_step2_router.rs:202-227`) | Yes | **Direct call** via router. |
| `DiscoverModDownloadForks { tp2, label, repo }` | named fields | Router → `app_step2_update_github_forks::fetch_github_forks` (`src/core/app/app_step2_router.rs:131-151`) | Yes | **Direct call** via router. |
| `AddDiscoveredModDownloadFork { tp2, label, full_name, owner_login, default_branch }` | named fields | Router → builds source_block string + state writes (`src/core/app/app_step2_router.rs:152-178`) | Yes | **Direct call** via router. |
| `SaveModDownloadSourceEditor` | — | Router → `mod_downloads::save_user_mod_download_source_block` (`src/core/app/app_step2_router.rs:228-270`) | Yes | **Direct call** via router. |
| `SetModDownloadSource { tp2, source_id }` | named fields | Router → `set_mod_download_source` private helper (`src/core/app/app_step2_router.rs:271-273`) | Yes — the file-private helper is reached through the public router | **Direct call** via router. |
| `SetSelectedModUpdateLocked(bool)` | locked | Router → `set_selected_mod_update_locked` + `mod_update_locks::set_mod_update_lock` (`src/core/app/app_step2_router.rs:119-121`) | Yes | **Direct call** via router. |
| `OpenCompatForComponent { game_tab, tp_file, component_id, component_key }` | named fields | Router → state writes to open compat popup (`src/core/app/app_step2_router.rs:274-288`) | Yes | **Direct call** via router. |

**Summary for Step 2.** 22 of 24 variants dispatch via a single direct call to `bio::app::app_step2_router::handle_step2_action` (direct reuse — SPEC §1 decision-order step 1). The two log-picker variants route to a sibling in `src/ui/workspace/step2_log_glue.rs` because BIO's UI-layer wrapper (`apply_weidu_log_selection`) couples the `rfd::FileDialog` + settings-save to a `&mut WizardApp` (outside carve-out #4's scope). The orchestrator's sibling — a simple file-picker wrapper + state mutation + settings-debounce trigger — calls the underlying `bio::app::app_step2_log::apply_weidu_log_selection_from_path` (`pub(crate) fn`) directly (SPEC §1 decision-order step 2). No carve-out needed.

#### Step 3 — no action enum

`bio::ui::step3::page_step3::render` returns `()` (verified at `src/ui/step3/page_step3.rs:7-9`, per H2). Step 3 has no `Step3Action` enum: the page handles its own intents directly against `&mut WizardState` (drag-reorder via `state_drag_step3`, undo/redo via `step3_history`, block selection via `block_selection_step3`, etc.). The orchestrator's `workspace_step_router::render` calls `bio::ui::step3::page_step3::render(ui, &mut orchestrator.wizard_state, dev_mode, exe_fingerprint)` and ignores the return value. No dispatch arm exists; no `step_action_dispatch::dispatch_step3` function exists. The orchestrator handles Step 3 **by reading `WizardState` mutations directly** — the dirty-bit check in P6.T11's persistence cycle picks up reorder / collapse / undo via a small state fingerprint over `wizard_state.step3.<active_tab>_items` (order vector length + first/last element ids). No action dispatch needed.

#### Step 4 — `Step4Action` (variants per `src/core/app/state/step4_action.rs:5-8`, re-exported as `crate::ui::step4::action_step4::Step4Action`)

BIO's `WizardApp::handle_step4_action` (`src/ui/app_methods.rs:82-88`) delegates to `super::step4_flow::handle_step4_action`, which is a re-export of `bio::app::app_step4_flow::handle_step4_action` (`src/core/app/app_step4_flow.rs:8-24`). The public function already takes `&mut WizardState` directly.

**Important caveat for Step 4.** The orchestrator does **not** call `bio::ui::step4::page_step4::render` (per C4 in the Phase 6 summary — Step 4's body is replaced with an orchestrator-side renderer at `src/ui/workspace/step4/workspace_step4.rs`). So `Step4Action` is never returned from a BIO renderer to the orchestrator. The orchestrator's Step 4 wrapper triggers each variant directly from its own button handlers and routes through `dispatch_step4` (or calls the underlying `bio::app::app_step4_flow::handle_step4_action` directly — both reach the same code). The table below documents the dispatch surface anyway because the orchestrator's Save action row and Check Mod List button feed into it.

| Variant | Payload | BIO dispatch target (file:line) | Reachable from orchestrator? | Orchestrator implementation |
|---------|---------|---------------------------------|------------------------------|-----------------------------|
| `SaveWeiduLog` | — | `bio::app::app_step4_flow::handle_step4_action` → `step4_weidu_log_export::auto_save_step4_weidu_logs` (`src/core/app/app_step4_flow.rs:10-14`); `auto_save_step4_weidu_logs` is `pub(crate) fn` per `src/core/app/step4_weidu_log_export.rs:50` and takes `&mut WizardState` directly. | Yes — same-crate `pub(crate)` reachable for both wrappers. | **Direct call** — `src/ui/workspace/step4/step4_save_row.rs`'s Save button calls `bio::app::step4_weidu_log_export::auto_save_step4_weidu_logs(&mut orchestrator.wizard_state)` directly (skipping the `app_step4_flow::handle_step4_action` wrapper). Alternatively `step_action_dispatch::dispatch_step4(Step4Action::SaveWeiduLog, orchestrator)` calls `bio::app::app_step4_flow::handle_step4_action(&mut state, action)` — both paths produce the same effect. The Save row uses the direct call to surface the `Err(msg)` for the save-error popup; the dispatch route is also available. |
| `CheckMissingMods` | — | `bio::app::app_step4_flow::handle_step4_action` → `app_step2_saved_log_flow::queue_exact_log_update_preview` (`src/core/app/app_step4_flow.rs:15-22`). | Yes — same-crate `pub(crate)` reachable. | **Direct call** — `src/ui/workspace/step4/step4_exact_log_viewer.rs`'s Check Mod List button calls `bio::app::app_step4_flow::handle_step4_action(&mut orchestrator.wizard_state, Step4Action::CheckMissingMods)` directly. No `WizardApp` involvement; no carve-out #4 refactor needed (the function already takes `&mut WizardState`). |

**Summary for Step 4.** Both variants dispatch via direct calls to `bio::app::*` public functions. No sibling is needed; no carve-out is needed beyond the C4 wrapper that's already documented (the C4 wrapper is a sibling renderer, not a dispatch wrapper — it composes the Save row + tab strip + review list around the same direct `bio::app::*` calls that `WizardApp` ultimately runs).

#### Step 5 — `Step5Action` (variants per `src/ui/step5/action_step5.rs:5-7`)

Phase 6's workspace shell stubs Step 5 (`workspace_step5_stub::render`); Phase 7 wraps the real install runtime. The single variant `Step5Action::StartInstall` (verified at `src/ui/step5/action_step5.rs:5-7` — note: Step 5's action enum lives under `ui/`, not `core/app/state/`, unlike Step 2 / Step 4) is dispatched in Phase 7 by setting `state.step5.start_install_requested = true` (same as BIO does — verify against `bio::ui::app::update_loop::run`'s dispatch site, the H3 real path). Documented here for completeness; Phase 7 owns the wiring.

| Variant | Payload | BIO dispatch target | Orchestrator implementation |
|---------|---------|---------------------|-----------------------------|
| `StartInstall` | — | Flips `state.step5.start_install_requested = true`; BIO's `bio::app::app_update_cycle::start_after_render` picks it up the next poll. | **Direct state mutation** (gated by Phase 7's install-start hook + concurrency check). The orchestrator's Step 5 chrome dispatches via `state.step5.start_install_requested = true` after running `install_runtime::start_hooks::on_install_start` (P7.T3). |

#### Dispatch file inventory

`src/ui/workspace/step_action_dispatch.rs` exposes:

```rust
pub fn dispatch_step2(action: Step2Action, orchestrator: &mut OrchestratorApp) { /* match per the Step 2 table; the two SelectVia*Log arms call step2_log_glue::apply_weidu_log_selection_for_orchestrator; everything else calls bio::app::app_step2_router::handle_step2_action */ }
pub fn dispatch_step4(action: Step4Action, orchestrator: &mut OrchestratorApp) { /* call bio::app::app_step4_flow::handle_step4_action(&mut state, action) */ }
// No dispatch_step3 — Step 3 has no action enum (returns ()); the workspace_step_router calls page_step3::render and ignores its return.
// No dispatch_step5 in Phase 6 — Phase 7's page_workspace_step5 inlines the StartInstall dispatch via state mutation + the install-start hook.
```

The `src/ui/workspace/step2_log_glue.rs` sibling owns the `rfd::FileDialog` + settings-persistence trigger for `SelectBgeeViaLog` / `SelectBg2eeViaLog`. It calls `bio::app::app_step2_log::apply_weidu_log_selection_from_path` (`pub(crate)`, same-crate reachable per Phase 1's carve-out #3 split) for the underlying state mutation, plus the orchestrator's own settings-debounce trigger for the `bio_settings.json` write that BIO's `app.save_settings_best_effort()` does inside `WizardApp`.

### P6.T3 — Workspace progress bar

- **What:** 4 segments per SPEC §2.2 / `screens.jsx::WorkspaceProgressBar`. Each segment shows `STEP N` (uppercase Poppins 10px 500 letter-spacing 1.4px) + the step label (14px 700 if current; 13px regular otherwise). Completed steps get a green `✓` on the right. Current step is filled in `accent`; upcoming steps are `chrome_bg` at 55% opacity.
- **Where:** New file.
- **Acceptance:** Matches the wireframe visually. Switching steps updates the highlighting; checking-off a step adds a checkmark.
- **SPEC:** §2.2.

### P6.T4 — Workspace nav bar

- **What:** Bottom row per SPEC §2.2. `← Previous` button (disabled on first step), the step indicator label "on Step N · <label> · step N of 4", and `Next →` primary button (disabled on last step). The `disable_prev` argument lands in Phase 7 to gate post-install navigation.
- **Where:** New file.
- **Acceptance:** Clicking buttons changes the step. Auto-advances `completed_steps` when crossing into a new step.
- **SPEC:** §2.2.

### P6.T5 — Workspace header with inline rename

- **What:** Renders `Editing <name>` + ✎ icon. Clicking ✎ swaps to an inline input with `save` + `cancel` buttons. Pressing Enter saves; Escape cancels. The rename writes to `registry::operations_rename::rename_modlist` (registry only; on-disk folder untouched). Below the title: a fork badge if this is a forked build (`ModlistEntry.forked_from` non-empty), and a fork-source sub-line built from the immediate parent (last `forked_from` entry). The header's **`⑂ view fork details`** button (shown only when `forked_from` is non-empty, per SPEC §2.2) opens the `ForkInfoPopup` — **the same `src/ui/orchestrator/widgets/dialogs/fork_info_popup.rs` widget built in Phase 5 P5.T10**, fed the entry's `forked_from` chain + the entry's own `name`/`author` as the current node. No new popup file.
- **Where:** New file (`workspace_header.rs`); reuses the Phase-5 `ForkInfoPopup`.
- **Acceptance:** Renaming a modlist updates the registry; the Home card reflects the new name on next visit. The install folder on disk is unchanged. For a forked build, `⑂ view fork details` opens the lineage popup with the full chain (oldest→newest) + the current modlist emphasized.
- **SPEC:** §2.2, §10.9, §13.3 (Provenance).

### P6.T6 — Save draft button

- **What:** Steps 2-4 only. Clicking persists the current workspace state immediately: call `workspace_state_loader::extract_workspace_state_from_wizard(&orchestrator.wizard_state)` → write the result via `WorkspaceStore::save`. Flash `✓ saved!` for ~1.6s. After the flash, label reverts to `save draft`. No dialog, no file picker.
- **Where:** Inside `workspace_header.rs`.
- **Acceptance:** Clicking saves. Restarting the app and resuming the build shows the saved state. The on-disk `modlists/<id>/workspace.json` matches the workspace state at save time.
- **SPEC:** §10.1.

### P6.T7 — Create stage_choose

- **What:** Render setup Box (modlist name input + game ComboBox with options `EET (default), BGEE, BG2EE, IWDEE` + destination FolderInput + conditional DestinationNotEmptyWarning, partial-install option disabled per SPEC). Below: two starting-point cards (`start →` and `paste share code →`).
- **Where:** New file.
- **Acceptance:** `start →` calls `registry::operations_create::create_modlist(name, game, destination, ...)` → switches `orchestrator.nav` to `NavDestination::Workspace { modlist_id: Some(new_entry.id) }`. `paste share code →` switches `CreateStage::ForkPaste`.
- **SPEC:** §5.1.

### P6.T8 — Create fork-paste / fork-preview / fork-download

- **What:** Reuse the same paste textarea + `preview_tabs` widget + `ImportDownloadScreen` from Phase 5, with different button labels and continueLabel. **Fork-preview** displays the parsed parent code's packed `name`/`author` as title/subline + the `⑂ fork info` affordance (the Phase-5 `ForkInfoPopup`, showing the *incoming parent's* lineage) — identical to the Install preview (P5.T10), differing only in the `Begin Import →` CTA and no `allow_auto_install` gate (forking is always allowed — SPEC §13.3). On fork-download completion, create the registry entry (the fork's name + game + default destination) and route to the Workspace.

  **Lineage append (the credit guarantee — SPEC §13.3 Provenance / §5.3).** When the registry entry is created, populate, in addition to name/game/destination:
  - `author` ← `RedesignSettings.user_name` (the local user is the author of *this* fork; empty ⇒ `None`).
  - `forked_from` ← `<parent.forked_from> ++ [ ForkAncestor { name: parent.name, author: parent.author } ]`, where the parent's `name`/`author`/`forked_from` are read off the parsed `ModlistSharePreview` (carve-out #5 fields). Append-only — the original creator stays first, no ancestor is ever rewritten.
  This is a registry write only; no share code is generated at fork time (`pack_meta` generation happens later, at install-start / `flip_to_installed` — Phase 7 — and reads these entry fields).

  **Phasing (SPEC §13.12a).** Phase 6 ships fork-paste + fork-preview fully, the registry entry + the lineage append, and the fork-download **chassis** navigation. The **live** fork download/extract — driving BIO's import → auto-build pipeline + the per-install dirs + content-addressed staging — is **Phase 7 P7.T17** (the pipeline terminates in the install runtime). Until P7.T17, fork-download navigates the chassis and the forked Workspace opens, but its Step-2 scan is not populated by real fetched mods; it lights up automatically when P7.T17 binds the live pipeline (same forward-compatible model as P5.T12 / Install).
- **Where:** New files `stage_fork_paste.rs`, `stage_fork_preview.rs`, `stage_fork_download.rs`; the append logic lives in `operations_create.rs`. Reuses the Phase-5 `ForkInfoPopup` + `stage_downloading` chassis.
- **Acceptance (Phase-6 scope):** Pasting a known share code → preview (shows parent name/author + `⑂ fork info` lineage) → begin import → the fork-download chassis renders → the forked Workspace opens with a `⑂ Fork` badge. The new `ModlistEntry` has `author` = the configured user name and `forked_from` = parent chain + parent appended (verify by inspecting `modlists.json`). Live mod download/extract + the resulting populated Step-2 selection/order = Phase 7 P7.T17 acceptance (SPEC §13.12a), not exercised in Phase 6.
- **SPEC:** §5.3, §13.3 (Provenance / append rule), §10.9.

### P6.T9 — Load Draft dialog

- **What:** Non-blocking `egui::Window` popup (per SPEC §10) listing in-progress builds (filtered from the registry) as cards using the same chassis from Phase 5 (`modlist_card`). Each card has a `resume` primary button + Kebab with `Copy import code` / `Delete`. Empty state copy per SPEC §5.2. Footer: `Cancel` only.
- **Where:** New file.
- **Acceptance:** Clicking `resume` closes the dialog and opens the Workspace with that build's state. The dialog is not a file picker (SPEC §5.2 explicitly forbids that).
- **SPEC:** §5.2, §10.2.

### P6.T10 — Game tabs from modlist game

- **What:** `workspace_state_loader::populate_wizard_state_from_workspace` sets `wizard_state.step1.game_install` from `entry.game` before each frame's Step 2/3 render. The existing BIO `state_step2`/`state_step3` reads `Step1State::game_install` to decide single vs dual tabs; the loader ensures the field reflects the active modlist's choice. Because the orchestrator's `WizardState` is per-orchestrator-process (not per-settings-file), this write does not touch `bio_settings.json`. The orchestrator-side Step 4 wrapper (P6.T2b) likewise branches on the modlist's `game_install` for single vs dual tab rendering.
- **Where:** `workspace_state_loader.rs`.
- **Acceptance:** Opening an EET workspace shows both BGEE and BG2EE tabs in Step 2 / Step 4; opening a BGEE workspace shows only one tab. Switching between workspaces of different games swaps the tab set cleanly. The on-disk `bio_settings.json` is untouched.
- **SPEC:** §5.1 ("game choice immutable once the workspace opens"), overview "Architecture" section (per-modlist `WizardState`).

### P6.T11 — Persistence cycle integration (dirty-bit-gated)

- **What:** Extend `RegistryPersistenceCycle` (Phase 3) to also debounce-write the per-modlist workspace state. **Per H1 — use an explicit dirty bit, not per-frame extract+compare.** `OrchestratorApp` carries a `workspace_state_dirty: bool` flag. Mutating call sites set it to `true`:
  - `step_action_dispatch::dispatch_step2(action, orchestrator)` — every `Step2Action` variant that mutates `wizard_state.step2` or `wizard_state.step3` sets the flag.
  - `step_action_dispatch::dispatch_step4(action, orchestrator)` — same for Step 4 mutations.
  - Step 3 drag-reorder, expand-collapse, undo/redo (which mutate `wizard_state.step3` via BIO's internal handlers) — the orchestrator's per-frame logic detects via comparing a single representative field (e.g., `step3.<active_tab>_order.len()` or a generation counter on `state_step3` if BIO has one — verify) and sets the flag. If BIO has no equivalent generation counter, the orchestrator wraps the Step 3 page render in a small detector: capture a cheap state fingerprint (e.g., a u64 hash of the active tab's order vector lengths + the first/last element ids) before render, compare after. This is much cheaper than full `ModlistWorkspaceState` extract+compare every frame.
  - The Step 2 checkbox toggles (which mutate the order via `Step2Action`) and Step 3 group-collapse toggles flow through the action dispatch above and set the flag there. No fingerprint needed for those.
  - Workspace rename (registry mutation, not workspace-state mutation) does NOT set this flag — it sets the registry dirty bit instead.

  On every `OrchestratorApp::update`, the persistence cycle checks: if `workspace_state_dirty == true`, call `extract_workspace_state_from_wizard(&orchestrator.wizard_state)`, compare to the last-extracted snapshot, if different queue a debounced write + clear the flag. If the flag is `false`, the extract+compare is skipped entirely — zero per-frame work for idle workspaces.
- **Where:** Edit `src/registry/persistence_cycle.rs` (Phase 3 file — editable) to add the workspace cadence with the dirty-bit check. The dirty bit lives on `OrchestratorApp::workspace_state_dirty`. Add `mark_workspace_dirty(&mut self)` helper method.
- **Acceptance:** Editing a Step 2 checkbox dirties the workspace state; the file is written ~1s later (debounce). Editing nothing produces no extract+compare overhead (verify by adding a debug counter incremented inside the extract function — should not increment when the workspace is idle). Closing the app flushes pending writes via `on_exit` (per H4).
- **SPEC:** §13.14.

### P6.T12 — `NavDestination::Workspace { modlist_id }` is real

- **What:** `NavDestination::Workspace { modlist_id: Option<String> }` (defined in Phase 2 as nullable) now carries a required id once Create / Resume / Home Resume routes set it. `page_router::render` for `Workspace { Some(id) }`:
  1. Look up `id` in `orchestrator.registry.entries`.
  2. **Per C5 gate:** if `install_runtime::install_concurrency::install_in_progress(orchestrator) == Some(running)` and `running.modlist_id != id`, the rail-nav lock should have prevented this transition. As a safety net, refuse to swap and re-pin nav to `Workspace { Some(running.modlist_id) }`.
  3. If `state_workspace.loaded_workspace_id != Some(id)`: call `workspace_state_loader::populate_wizard_state_from_workspace(...)`, set `loaded_workspace_id = Some(id)`.
  4. **Settings path re-assert (M2 — open-only).** `populate_wizard_state_from_workspace` calls `workspace_state_loader::sync_paths_from_settings(&orchestrator.settings_store, &mut orchestrator.wizard_state)` **once on workspace open / modlist swap**, not per frame. The orchestrator's Settings → Paths tab edits the *same in-memory* `wizard_state.step1` the workspace renders from, so Settings edits made while the user was away propagate **by construction** — there is no second source to reconcile and no per-frame disk read. The open-time call is a defensive re-assert of the last-persisted paths (`bgee_game_folder`, `bg2ee_game_folder`, `iwdee_game_folder`, `mods_folder`, `mods_backup_folder`, tool binaries, …) via `settings_store.load()`; a load error ⇒ silent no-op (the live `step1` already holds current values). (Amended from the original per-frame M2 — overview 2026-05-16.)
  5. Call `workspace_view::render(ui, orchestrator, id, ctx)`.
  For `Workspace { None }` (dev stub from Phase 2): render `workspace_step5_stub`-style placeholder. (Phase 6 keeps the dev path so testing can navigate without a real id.)
- **Where:** Edit `src/ui/orchestrator/nav_destination.rs` (already accepting `Option<String>` from Phase 2) and `src/ui/orchestrator/page_router.rs`.
- **Acceptance:** Switching between modlists by opening different cards from Home properly swaps the workspace's data. During an in-flight install, navigation is blocked at the rail layer (Phase 7); the loader never runs in this state.
- **SPEC:** §2.2, §13.15.

### P6.T13 — Wire Create into `page_router`

- **What:** Replace the Create stub with `bio::ui::create::page_create::render(...)`.
- **Where:** Edit `src/ui/orchestrator/page_router.rs`.
- **Acceptance:** Create rail item opens the real Create screen.
- **SPEC:** §5.

### P6.T14 — Resume button on Home / Load Draft routes to Workspace

- **What:** The Home in-progress card's `resume` button (Phase 5 hooks the callback) and the Load Draft dialog's `resume` button both set `orchestrator.nav = NavDestination::Workspace { modlist_id: Some(card.id) }` and switch.
- **Where:** Edit `src/ui/home/modlist_card.rs` callbacks + `src/ui/create/load_draft_dialog.rs`.
- **Acceptance:** Click resume → Workspace opens with the build's data; the header reads `Editing <name>`; the user lands on Step 2.
- **Workspace open behavior:** The workspace always opens at Step 2 for v1 alpha (per SPEC §3.2 — 'wireframe demo always lands on Step 2'). Remembering the last-active step per build is a future refinement, not in v1.
- **SPEC:** §3.2, §5.2.

### P6.T15 — Nav-away flush

- **What:** When the user navigates from `Workspace` to any other destination, call `workspace_state_loader::extract_workspace_state_from_wizard` and write synchronously via `WorkspaceStore::save` before the screen transitions. Implemented in `page_router::render` by detecting the nav transition. Per H4, this is one of the persistence write paths; the on-exit `flush_all` (from `eframe::App::on_exit`) is the other.
- **Where:** Edit `src/ui/orchestrator/page_router.rs`.
- **Acceptance:** Editing Step 2, immediately clicking Home, then quitting the app, then relaunching, then reopening the workspace: the Step 2 change is still there.
- **SPEC:** §13.14 ("On nav-away from the workspace").

## Open questions / risks

- **Workspace-state-loader correctness.** Phase 6's most delicate task is mapping every relevant `ModlistWorkspaceState` field to its `WizardState` counterpart and back. The canonical Step 3 order fields are `state.step3.bgee_items` and `state.step3.bg2ee_items` (each `Vec<Step3ItemState>`, both `pub` per `src/core/app/state/state_step3.rs:23-24`). The collapsed-blocks state is `state.step3.bgee_collapsed_blocks` / `bg2ee_collapsed_blocks` (`pub` per `state_step3.rs:44-45`). Read the BIO state structs in Phase 6 implementation and adjust the workspace model field names to match. The `ModlistWorkspaceState` struct is new (Phase 3); renaming its fields to align with BIO is free.
- **Step 2 scan worker.** The scan worker is started by BIO's `bio::app::app_step2_scan::start_scan` (verify visibility). The orchestrator owns its own scan-event receiver and starts the scan on workspace entry using the mods folder from `Step1Settings`. On modlist switch, the orchestrator restarts the scan for the new mods folder (in practice the same global mods folder unless the user has changed it; the scan cache handles this).
- **Step 3 history per modlist.** Step 3 undo/redo state is in `state.step3.bgee_undo_stack` / `bg2ee_undo_stack` (`pub` per `state_step3.rs:50-51`) and is currently per-`WizardState` scope. The workspace state loader resets the history on workspace open (acceptable: the user has the saved registry file as "undo of last resort"). Saving + restoring the history per workspace is a later enhancement; the spec does not require it for v1 alpha.
- **`step_action_dispatch.rs` completeness.** The dispatch surface for every `Step2Action` and `Step4Action` variant is enumerated in the "Step action dispatch surface" sub-section above (24 `Step2Action` variants + 2 `Step4Action` variants, each with source-line citations and orchestrator strategy). Step 3 has no action enum (returns `()` per H2). The C2 audit table — and the dispatch tables above — together document the dispatch surface per function.
- **EET-only modlists tabs.** Per SPEC §5.1, the game selection is `EET (default), BGEE, BG2EE, IWDEE`. For IWDEE specifically, BIO's existing scan + install code must already support it (CLI subcommand `bio scan components --game-directory ...` works); confirm the GUI tab path is also wired (the existing Step 2 tree code handles single-tab cases). If IWDEE has gaps, document them; spec doesn't require Phase 6 to add new IWDEE support.
- **C4 fidelity to BIO's review logic.** The orchestrator-side Step 4 renderer reads the same `state.step3.bgee_items` / `bg2ee_items` data BIO's `bio::ui::step4::content_step4::render` reads (`src/ui/step4/content_step4.rs:277,279`). The visual rendering differs (orchestrator uses redesign tokens + the three-color line widget) but the data plumbing is identical. The Save action is identical because both paths call `auto_save_step4_weidu_logs` with the same `&mut WizardState`. If a future BIO change adds new Step 4 logic (e.g., a Step 4 search bar), the orchestrator's renderer would need a parallel implementation — document this divergence risk.

## Verification

1. `cargo build --bin infinity_orchestrator --release` succeeds.
2. From Home (empty registry), click `create your own` → Create opens.
3. Fill name "Test EET", game = EET (default), pick a destination, click `start →`. Workspace opens with `Editing Test EET` in header + 4-step progress bar showing Step 2 active. Confirm `wizard_state.step1.game_install == EET` is in-memory only — `bio_settings.json` on disk has not been written to with this value.
4. In Step 2: scan mods, toggle a checkbox; verify the change persists by clicking save draft (`✓ saved!` flash) and inspecting `modlists/<id>/workspace.json`.
5. Step 3: drag a row; the order array updates.
6. Step 4: shows the orchestrator-side renderer — top row with Save button + count, game tab strip (for EET), line-numbered monospace review list with three-color WeiDU lines. Verify BIO's `bio::ui::step4::page_step4::render` is **not** invoked by inspecting the orchestrator's call stack / grepping the bound code. Click Save: a `weidu.log` file is written to the resolved BGEE / BG2EE game folder (same effect as BIO's Step 4 save today).
7. Step 5: stub renders with "Phase 7" message + disabled Install.
8. Click Home: workspace state flushes. From Home `In progress (1)` chip → the card appears. Click `resume`: workspace reopens with Step 2 selections intact.
9. Click `load draft` from Create: dialog lists the in-progress build. Click `resume`: workspace opens at Step 2.
10. Rename via ✎ icon: header label updates; registry file shows new name; install folder on disk is unchanged.
11. Create a second modlist with game = BGEE. Open it: only one game tab shows in Step 2 / Step 4 (not two). Switch to the first modlist: two tabs return. Confirm `bio_settings.json` is unchanged.
12. `cargo build --bin BIO --release` continues to succeed; the legacy wizard is unaffected. Confirm: launching `BIO` and navigating to Step 4 renders BIO's existing `page_step4` (not the orchestrator's wrapper) — the two binaries' Step 4 renderings differ visually, and that's the intended split.
