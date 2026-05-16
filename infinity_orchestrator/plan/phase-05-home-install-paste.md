# Phase 5 — Home + Install Modlist (paste / preview / download stages)

## Summary

Build the Home screen (filter chips, modlist cards, Add-a-modlist Box, game-installs-detected block, first-launch empty state, delete confirm dialog) and the Install Modlist top-level destination's first three stages: paste + preview fully, **downloading as a forward-compatible chassis only** (Run 5). The Install Modlist fourth stage (the actual install runtime) is stubbed and rolled in during Phase 7. Reuse the existing BIO share-code parser (`bio::app::modlist_share::preview_modlist_share_code`) for the preview stage. **The BIO download / extract engines (`bio::app::app_step2_update_*`) are NOT consumed in Phase 5** — driving the import → auto-build → download/extract pipeline + per-install directory derivation + content-addressed archive staging is **Phase 7 P7.T17** (SPEC §13.12a), since that pipeline terminates in the install runtime. Wire the registry from Phase 3 to the modlist cards.

## What ships after this phase

- `cargo build --bin infinity_orchestrator --release` succeeds.
- Clicking Home in the left rail opens the real Home screen:
  - Title "Welcome back, adventurer" + sub line "<N> modlists installed · <P> in progress · last played <game> <relative>".
  - 2-col grid: left = filter chips Box + cards; right = `add a modlist` Box + `game installs detected` block.
  - Filter chips: `Installed (N)`, `In progress (P)` (only when P > 0), `All (N+P)`. Default selection = whichever category has content.
  - Cards show modlist name + meta line + action cluster (Resume/Play + Kebab).
  - First-launch empty-registry state: setup CTA card replaces left Box, suggesting `Open Settings`.
  - Empty filter state: faint message per chip.
  - Delete confirm dialog removes the modlist registry entry **and** the install folder.
- Clicking `paste import code` (or the Install rail item) opens Install Modlist:
  - Stage 1 (paste): destination folder + `DestinationNotEmptyWarning` (with `clear` / `backup` / `continue partial`) + import-code textarea + footer with `Preview →`.
  - Stage 2 (preview): parsed share-code preview — Overview Box (Game/Mods/Components/log entries) + tabbed Content Box (Summary / BGEE WeiDU / BG2EE WeiDU / User Downloads / Installed Refs / Mod Configs).
  - Stage 3 (downloading): the §4.3 **chassis** — overall-progress Box + per-mod 4-col grid + Cancel/auto-advance, grid empty (live download/extract data + content-addressed staging = Phase 7 P7.T17 / SPEC §13.12a).
  - Stage 4: stub showing "Install runtime — Phase 7" + a `← Back to preview` button.
- The `game installs detected` block on Home reflects path validation events from Phase 4's `validate_now`.
- Toast notifications appear bottom-center for "Copied import code", "Deleted <name>", etc.
- **Open button (renamed from "play" per M6):** Installed cards show an `open` button (not `play`) in v1 alpha — no game-launcher integration exists in today's BIO, so the label reflects what the button actually does: opens the install folder in the OS file manager. Same behavior as the Kebab's `Open install folder`. The wireframe's `play` label is the long-term goal; when a game launcher ships in a future release the label flips back to `play` and the action becomes "launch the game" (SPEC §3.2 note added).

## What's still missing

- Stage 4 of Install Modlist (actual install runtime) — Phase 7.
- Reinstall flow that routes through Install Modlist's preview stage with overwrite-install forced — Phase 7.
- Workspace view (Create + Resume) — Phase 6.
- Step 5 share-code generation post-install — Phase 7.
- `modlist-import-code.txt` auto-write on install start — Phase 7.

## Dependencies

- Phase 2 (orchestrator app, page_router, redesign widgets).
- Phase 3 (modlist registry data layer).
- Phase 4 (Settings persists path validation results — Home reads them).

## File inventory

### New files

| Path | Purpose | Depends on |
|------|---------|-----------|
| `src/ui/home/mod.rs` | `pub mod page_home; pub mod state_home; pub mod filter_chip; pub mod modlist_card; pub mod add_a_modlist; pub mod game_installs_detected; pub mod confirm_delete; pub mod first_launch_setup_card; pub mod toast;` | — |
| `src/ui/home/page_home.rs` | Top-level screen renderer. Branches on empty-registry first-launch vs normal. | registry, settings |
| `src/ui/home/state_home.rs` | `pub struct HomeScreenState { filter: HomeFilter, toast: Option<ToastMessage>, delete_target: Option<String>, reinstall_target: Option<String> }`. `HomeFilter::{Installed, InProgress, All}`. | — |
| `src/ui/home/filter_chip.rs` | `pub fn render(ui, label, count, active, on_click)` — chip with 14px border-radius, no drop shadow, `accent` fill when active. Mirrors `screens.jsx::Chip` (line 277–296). | redesign theme tokens |
| `src/ui/home/modlist_card.rs` | `pub fn render(ui, entry: &ModlistEntry, actions: ModlistCardActions)` — card chassis with name + meta + Resume/Play + Kebab. Two action sets based on state. | redesign widgets, registry model |
| `src/ui/home/add_a_modlist.rs` | The right-column Box rendering `paste import code` + `create your own` CTAs. | redesign theme tokens |
| `src/ui/home/game_installs_detected.rs` | The detected-games block below the CTAs. Reads from the path validation summary. | path validation |
| `src/ui/home/confirm_delete.rs` | The danger `ConfirmDialog` for Delete (modlist registry entry + on-disk install folder). | redesign widgets |
| `src/ui/home/first_launch_setup_card.rs` | The setup CTA card shown when registry is empty. | redesign widgets |
| `src/ui/home/toast.rs` | Bottom-center transient `Toast` that auto-dismisses after ~1.8s. | redesign theme tokens |
| `src/ui/shared/format_relative.rs` | `pub fn relative_time(ts: DateTime<Utc>) -> String` — "2 hours ago", "yesterday", "last week", "last month". Lives under `src/ui/shared/` (not `src/ui/home/`) because other surfaces (Workspace header, Step 5 success banner) also format relative times. | chrono |
| `src/ui/orchestrator/widgets/kebab.rs` | `pub fn render(ui, items: &[KebabItem])` — three-dot menu with dropdown. Reusable across Home cards, Step 5, etc. | redesign theme tokens |
| `src/ui/orchestrator/widgets/pill.rs` | `pub fn render(ui, label, tone: PillTone)` — generic pill with tone-aware fill. | redesign theme tokens |
| `src/ui/install/mod.rs` | `pub mod page_install; pub mod state_install; pub mod stage_paste; pub mod stage_preview; pub mod stage_downloading; pub mod stage_installing_stub; pub mod sub_flow_footer; pub mod preview_tabs;` | — |
| `src/ui/install/page_install.rs` | Top-level Install Modlist screen renderer. Dispatches on `InstallStage`. | stage_* |
| `src/ui/install/state_install.rs` | `pub struct InstallScreenState { stage: InstallStage, destination: String, destination_choice: Option<DestChoice>, import_code: String, parsed_preview: Option<ModlistSharePreview>, active_preview_tab: PreviewTab, fork_info_open: bool, download_progress: DownloadProgress }`. `parsed_preview` (`ModlistSharePreview`) now carries `allow_auto_install` + the provenance trio `name`/`author`/`forked_from` via carve-out #5. | BIO `ModlistSharePreview` |
| `src/ui/install/stage_paste.rs` | Stage 1 renderer per SPEC §4.1. | redesign widgets |
| `src/ui/install/stage_preview.rs` | Stage 2 renderer per SPEC §4.2. | preview_tabs |
| `src/ui/install/stage_downloading.rs` | Stage 3 **chassis** per SPEC §4.3 (`ImportDownloadScreen`): overall-progress Box + 4-col grid + Cancel/auto-advance, rendered from `InstallScreenState.download_progress` (empty in Phase 5). Phase 7 P7.T17 edits this same file to bind live `Step2UpdateDownloadEvent`/`Step2UpdateExtractEvent` data. Reusable by Phase 6 fork-download via `DownloadScreenCopy`. | redesign tokens (no BIO dep in Phase 5) |
| `src/ui/install/stage_installing_stub.rs` | Stage 4 placeholder — labeled "Install runtime — Phase 7" + a `Back to preview` button. | — |
| `src/ui/install/sub_flow_footer.rs` | `pub fn render(ui, back, hint, primary)` — mirrors `screens.jsx::SubFlowFooter` (line 3494–3510). | redesign widgets |
| `src/ui/install/preview_tabs.rs` | The 6-tab file-folder strip + per-tab rendered content. Tab content reads from the parsed `ModlistSharePreview`. | BIO `modlist_share::preview_modlist_share_code` (read-only) |
| `src/ui/install/destination_not_empty.rs` | The yellow-bordered warning Box with 3 radio buttons. Mirrors `screens.jsx::DestinationNotEmptyWarning` (line 123–154). | redesign widgets |
| `src/ui/orchestrator/widgets/dialogs/confirm_dialog.rs` | Shared `ConfirmDialog` (title + message + Cancel + primary Confirm, optional danger styling). Used by Home delete, Step 2 select-via-weidu-log (existing BIO), etc. Per SPEC §10.1, non-blocking `egui::Window`. | redesign widgets |
| `src/ui/orchestrator/widgets/dialogs/fork_info_popup.rs` | `ForkInfoPopup` (SPEC §10.9) — read-only credit-lineage chain (oldest→newest ancestors + current node emphasized), Close-only, collapse-anchored. Reused by Phase 6's workspace header `⑂ view fork details` + fork-preview. | redesign widgets, BIO `ModlistSharePreview` (provenance fields) |
| `src/registry/operations.rs` | High-level write helpers used by Home: `delete_modlist(id, store, registry, options)` (removes registry entry **and** on-disk install folder, atomic-where-possible), `share_code_for(id, registry) -> Option<String>` (returns the entry's `latest_share_code` for the caller to copy), `rename_modlist(id, new_name, registry)`. **No clipboard crate** — the actual copy is done at the UI layer via egui's built-in `ui.ctx().copy_text(code)` (a ctx-less registry helper can't reach the clipboard; `arboard`/`copypasta` would add an X11/Wayland-linked dependency for zero benefit). | std::fs |

### BIO files read from / consumed (no modifications)

- `src/core/app/modlist_share.rs::preview_modlist_share_code` — Used in Install stage 2 to parse the pasted code without committing it.
- `src/core/app/state/state_step1.rs` (path validation results) — Read by `game_installs_detected.rs`.
- `src/core/app/app_step2_update_download.rs` / `app_step2_update_extract.rs` — **NOT consumed in Phase 5.** Phase 5's Stage 3 is a chassis with no live data (P5.T12). Driving these engines (via the import → auto-build pipeline + the net-new content-addressed staging layer that wraps them, no BIO modification) is **Phase 7 P7.T17** per SPEC §13.12a — the pipeline terminates in the install runtime, which Phase 5 does not build.
- `src/registry/store.rs` / `src/registry/store_workspace.rs` (Phase 3 new files) — Read by Home, written by delete.

### BIO files needing allowed mild refactor

**One file, Run 4 only:** `src/core/app/modlist_share.rs` — carve-out #5 "Modlist-share provenance application" (SPEC §1): the four `#[serde(default)]` fields on `ModlistSharePayload` (`allow_auto_install` + `name`/`author`/`forked_from`), the `default_true()` fn, the `ForkAncestor` struct, the symmetric `ModlistSharePreview` fields, and the four `share_preview()` propagation lines (enumerated exactly in P5.T10). Schema-additive and behavior-neutral — defaults reproduce today's BIO bit-for-bit. **No other BIO source is touched in Phase 5.** Generation (`pack_meta`) is net-new orchestrator code, never a BIO edit (Phase 6/7).

## Implementation tasks

### P5.T1 — Filter chip widget

- **What:** `src/ui/home/filter_chip.rs::render(ui, label, count, active, on_click) -> egui::Response`. Sketchy 1.5px border, 14px border-radius (pill shape), 4px×12px padding, `accent` fill when active. Count is rendered after the label with `text-faint` color in `(N)` parens.
- **Where:** New file.
- **Acceptance:** Clicking a chip returns a clicked response; visual matches `screens.jsx::Chip`.
- **SPEC:** §3.1 ("Filter chips").

### P5.T2 — `modlist_card::render`

- **What:** Card chassis: a horizontal Box, left has the modlist name (bold Poppins 13px) + meta line (hand-style faint Poppins 14px), right has the action cluster. Two card types differ only in the action cluster + meta-line content:
  - In-progress: `<N> mods · <C> components · last touched <rel> · paused at Step <K>` + primary `resume` + Kebab with `Copy import code`, `Rename`, `Delete`. `K` reads `entry.paused_at_step` (the denormalized registry field added for this — `mod_count` / `component_count` are denormalized the same way). If `paused_at_step` is `None`, omit the `· paused at Step <K>` segment gracefully rather than rendering a placeholder. `Copy import code` reads `operations::share_code_for(id, registry)` then calls `ui.ctx().copy_text(code)` at the callback site (egui built-in clipboard — no external crate).
  - Installed: `<N> mods · <size> · installed <rel>` + neutral **`open`** button (renamed from wireframe's `play` for v1 alpha; opens the install folder per M6 / SPEC §3.2) + Kebab with `Copy import code`, `Open install folder`, `Rename`, `Reinstall`, `Delete`.
- **Where:** New file.
- **Acceptance:** Both card types render correctly. Kebab menu items invoke the right callbacks. The `open` button (renamed from `play` per M6) opens the install folder in the OS file manager. No game-launcher attempt; the label honestly reflects the behavior. **`Rename` is the one exception — it ships visible but inert in Phase 5:** the rename mechanism is a registry-only write that lands in Phase 6 alongside the Workspace ✎ inline rename and `operations_rename.rs` (SPEC §2.2). Intentional staged deviation recorded in SPEC §3.2 (both Kebab tables) + HANDOFF, **not** an unmet acceptance — do not re-flag.
- **SPEC:** §3.2.

### P5.T3 — Home `page_home::render`

- **What:** Branch on registry state:
  - **Empty** (no in-progress AND no installed): replace the left Box's contents with `first_launch_setup_card::render` (heading "Welcome to Infinity Orchestrator" + sub "Get set up first — point BIO at your games and tools." + primary `Open Settings` button that navigates to Settings → Paths).
  - **Non-empty**: render filter chips + filtered card list per SPEC §3.1 (Cards in the filtered list).
- **Where:** New file.
- **Acceptance:** First-launch state: clicking `Open Settings` switches `NavDestination` to `Settings` and sets the active tab to `Paths`. Non-empty state: filter chips show counts, switching chips filters the visible cards.
- **SPEC:** §3.1, §3.4.

### P5.T4 — Default filter selection + empty-filter messages

- **What:** Default chip selection logic per SPEC §3.1: if installed count > 0 then `Installed`; else if in-progress count > 0 then `In progress`; else `All`. Empty-filter copy per SPEC §3.1 ("Empty states").
- **Where:** `state_home.rs::HomeScreenState::default()` and inside `page_home::render`.
- **Acceptance:** A fresh-installed user with one installed modlist defaults to the `Installed` chip; a user with only in-progress builds defaults to `In progress`; an empty user defaults to `All`.
- **SPEC:** §3.1.

### P5.T5 — `Add a modlist` right column

- **What:** A `Box label="add a modlist"` containing primary `paste import code` button (navigates to Install) and neutral `create your own` button (navigates to Create). Labels intentionally lowercase. Below: `game installs detected` block.
- **Where:** `src/ui/home/add_a_modlist.rs`.
- **Acceptance:** Clicking each button navigates to the right destination. The right column always renders, even in empty-registry mode.
- **SPEC:** §3.3.

### P5.T6 — `game installs detected` block

- **What:** Render the 3 detection lines (BGEE / BG2EE / IWDEE) using the path validation summary from Phase 4. Found = `✓ <NAME>`; missing = `? <NAME> · not found` in faint type. Auto-refreshes on path-validation events.
- **Where:** `src/ui/home/game_installs_detected.rs`.
- **Acceptance:** Editing a path in Settings → Paths updates Home's block on next visit. Clicking `Validate now` in Settings updates immediately if Home is visible.
- **SPEC:** §3.3 (Refresh semantics).

### P5.T7 — Delete confirm dialog + side effects

- **What:** `ConfirmDialog` opens on Kebab → Delete with the danger style. Title: `Delete "<name>"?`. Body: **wireframe-verbatim from `infinity_orchestrator/wireframe-preview/screens.jsx:388-399`** (which the SPEC §3.1 Delete-semantics paragraph paraphrases). The implementer should read the wireframe directly for the canonical wording — citing exact lines avoids drift between the SPEC paraphrase and the wireframe source. Buttons: `Cancel` + danger-tinted `Delete`. On confirm: call `registry::operations::delete_modlist` which removes both the registry entry and the on-disk install folder (recursive). Show success toast `Deleted "<name>"`.
- **Where:** `src/ui/home/confirm_delete.rs` + `src/registry/operations.rs`.
- **Acceptance:** Deleting a modlist removes the card immediately and reduces the statusbar's modlist count. The on-disk install folder is gone after confirmation.
- **SPEC:** §3.1 (Delete semantics), Appendix B.1.

### P5.T8 — Kebab widget

- **What:** `src/ui/orchestrator/widgets/kebab.rs::render`. Three-dot icon (`···`) button; clicking opens a dropdown of menu items (`KebabItem { label, on_click, danger: bool }`). Danger items render in coral. Clicking outside the dropdown closes it.
- **Where:** New file.
- **Acceptance:** Used by Home cards. Verified by clicking each menu item in a test card and seeing the expected action.
- **SPEC:** §3.2, §6.4 (toolbar Kebab pattern).

### P5.T9 — `Install` stage 1 (paste)

- **What:** Three sections per SPEC §4.1: FolderInput for destination + `DestinationNotEmptyWarning` (rendered when destination is set and non-empty). When `Continue partial installation` is picked, the import-code section disappears. Footer: `SubFlowFooter` with primary `Preview →` (or `Continue Install →` in partial mode).

  **DestChoice radio options** (rendered inside `DestinationNotEmptyWarning` per `infinity_orchestrator/wireframe-preview/screens.jsx:123-154`). Per H10, the labels use the wireframe's verbatim copy:

  - `clear` — **"Clear contents"** (wipes + reinstalls). Maps to `prepare_target_dirs_before_install = true`, backup off (SPEC §13.12 #6).
  - `backup` — Move existing files to a backup folder, then install. Maps to `prepare_target_dirs_before_install = true`, backup on (SPEC §13.12 #6).
  - `continue` — Continue Partial Install (only when `allowPartial=true`). Maps to `prepare_target_dirs_before_install = false`, backup off (SPEC §13.12 #6; also triggers `-s`/`-c` per SPEC §13.12 #1).

  The previous draft used "Replace contents" for the `clear` label. That is incorrect — wireframe `screens.jsx:123-154` shows "Clear contents". Fixed per H10.

- **Where:** `src/ui/install/stage_paste.rs`.
- **Acceptance:** A valid destination is **required** — the footer primary is disabled until `destination` resolves to a real existing directory (typed garbage / empty does not satisfy it; `browse` does), in every mode. With a valid destination: pasting a valid BIO-MODLIST-V1 code and clicking `Preview →` advances to stage 2; empty code keeps the button disabled (non-partial). The `DestinationNotEmptyWarning` Box is legible in **both** Light and Dark themes: an un-premultiplied amber fill (not the old baked-dark constant) **and** white ink for its text (the fill composites to a dark olive in both themes, so light ink — not the §12.2 dark-on-*light*-pill rule) — a recorded deviation from the wireframe's `text-muted`, which is illegible on it.
- **SPEC:** §4.1.

### P5.T10 — `Install` stage 2 (preview): provenance display + `allow_auto_install` gate

- **What:** Calls `bio::app::modlist_share::preview_modlist_share_code(&code)` → `ModlistSharePreview` (now carrying, via carve-out #5, `allow_auto_install` + the provenance trio `name` / `author` / `forked_from`). Render the Overview Box + 6-tab Content Box (P5.T11).

  **Provenance display (SPEC §4.2 + §13.3 Provenance).**
  - **Title** = the packed `name`; if absent, the honest fallback **`Shared modlist`** (never fabricate a name).
  - **Subline** = `by @<author> · review what will be installed before BIO downloads anything`; if `author` absent, drop the `by @<author> · ` segment. Never invent an author.
  - **`⑂ fork info`** button in the title row when `forked_from` is non-empty → opens the `ForkInfoPopup` (SPEC §10.9) showing the lineage chain. Hidden when the chain is empty.

  **`allow_auto_install` gate (per SPEC §4.2 + §13.3).** Reads `allow_auto_install` from the preview (defaulting to `true` if absent). If `false`:
  - Info banner **above the Overview Box**: *"Draft modlist code — this is not from a verified install. Review and customize the components in Create → Import and modify before installing."*
  - **Disable the footer `Import Modlist →`** (greyed, tooltip "Auto-install disabled for draft codes — open in Create to review").
  - Secondary CTA **`Open in Create →`** in the footer between `← Back` and the disabled primary. On click: navigate to `NavDestination::Create` (Phase 6 wires the code pre-load handoff).

  If `allow_auto_install == true` (default for older / post-install codes): unchanged — enabled `Import Modlist →` advances to stage 3.

  **Carve-out #5 — the ONLY BIO-source touch in all of Phase 5 (per SPEC §1 "Modlist-share provenance application").** On `src/core/app/modlist_share.rs`, exactly:
  1. `ModlistSharePayload` (`#[derive(Deserialize)]`): `#[serde(default = "default_true")] allow_auto_install: bool`, `#[serde(default)] name: Option<String>`, `#[serde(default)] author: Option<String>`, `#[serde(default)] forked_from: Vec<ForkAncestor>`.
  2. New `fn default_true() -> bool { true }` (required by the serde attr).
  3. New `struct ForkAncestor { name: String, author: String }` — the `forked_from` element type. Derive exactly **`#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`** (not just `Deserialize`): `Deserialize` for the payload parse; `Serialize` + `PartialEq` + `Clone` + `Debug` because Phase 6 reuses *this same BIO type* as the element of `ModlistEntry.forked_from`, and `registry/model.rs` derives `Debug, Clone, PartialEq, Serialize, Deserialize` and round-trips / `assert_eq!`s the registry. Pinning the full set in this one Phase-5 edit means **no follow-up BIO-source touch** is needed in Phase 6. Still within carve-out #5 — precedent: the sibling `ModlistShareConfigFile` already derives `Deserialize, Serialize` in this same file (`modlist_share.rs:163`). No `Default` needed (`Vec` defaults empty; `ModlistEntry`'s manual `Default` sets `forked_from: Vec::new()`).
  4. Symmetric fields on `ModlistSharePreview` (`pub(crate)`, matching existing field visibility).
  5. Four propagation lines in `share_preview()` (copy each field payload→preview).
  Nothing else. All `#[serde(default)]` ⇒ pre-redesign / third-party codes parse and behave bit-for-bit as today. The implementer must surface **`SPEC CONFLICT`** if anything beyond these five mechanical edits is needed in BIO source.

  **Generation is NOT in this run.** Run 4 is consume-only — the orchestrator never *writes* these fields here. The `pack_meta` generator (SPEC §13.3 Generation mechanism), the registry `ModlistEntry.author` / `forked_from`, and the fork-lineage append land in Phase 6 (Save Draft / fork) + Phase 7 (install-start, `flip_to_installed`). Until then every code lacks the trio, so the preview renders the fallback path in practice — but the display is forward-compatible and lights up automatically when generation ships (same additive model as `allow_auto_install`).
- **Where:** `src/ui/install/stage_preview.rs` + new `src/ui/orchestrator/widgets/dialogs/fork_info_popup.rs` (`ForkInfoPopup`, reused by Phase 6's workspace header + fork-preview). `InstallScreenState` gains `fork_info_open: bool`; provenance + `allow_auto_install` are read off `parsed_preview` (the `ModlistSharePreview`). The cross-screen Create handoff is Phase 6.
- **Acceptance:** A code carrying `name`/`author` shows them as title/subline; absent ⇒ `Shared modlist` / no author segment (no fabrication). A code with non-empty `forked_from` shows `⑂ fork info`; clicking opens `ForkInfoPopup` with the chain oldest→newest + the current node emphasized. `allow_auto_install = false` ⇒ banner + disabled Import + enabled `Open in Create →`; `true` / absent ⇒ normal enabled Import. `cargo build --bin BIO --release` still succeeds (the carve-out is behavior-neutral).
- **SPEC:** §4.2, §13.3 (Provenance + Generation mechanism), §10.9, §1 (carve-out #5 "Modlist-share provenance application").

### P5.T11 — Preview tabs

- **What:** `preview_tabs::render` renders the 6 tabs in the file-folder style (per SPEC §6.4 same pattern). Each tab's body is a monospace pre-wrapped text area showing the relevant section of the preview.
- **Where:** `src/ui/install/preview_tabs.rs`.
- **Acceptance:** Tab switching is instant. Each tab's content matches the wireframe `screens.jsx::PreviewText` (line 512–529).
- **SPEC:** §4.2 (Preview tab contents).

### P5.T12 — `Install` stage 3 (downloading) — **chassis only; live wiring is Phase 7**

- **What:** Ship the SPEC §4.3 `ImportDownloadScreen` as a forward-compatible **chassis**: `Box label="overall progress"` ("N / T mods · P%" + bar) + a 4-column grid (mod / source / status / progress) with per-row status vocabulary `queued` / `downloading <p>%` / `extracting...` / `✓ staged` (tones: faint=queued, normal=active, success-green=done) + footer `Cancel` (← back) and auto-advance to stage 4 when `DownloadProgress` reports all-staged. The grid renders from `InstallScreenState.download_progress`, which is **empty in Phase 5** (navigable, honest empty-state).
- **Why live data is NOT in Phase 5 (corrected — this is not "BIO can't" and not "Install lacks paths"):** the per-mod download list only materializes as a byproduct of BIO's `import_modlist_share_code` → saved-log/auto-build pipeline (scan → apply weidu.log → update-check resolves each mod from the imported source config into an asset → download → extract → **install**). That pipeline *terminates in the install runtime* and depends on the per-install directory architecture + content-addressed archive staging — all of which is **Phase 7** (SPEC §13.12a, plan P7.T17). The global paths it needs are already in Settings → Paths (§11.2) and reach the orchestrator's `WizardState` via the established settings-sync — the Install screen does not collect them. Phase 5 has no install runtime, so the chassis is the correct Phase-5 deliverable; it lights up automatically when P7.T17 binds live progress (same additive model Phase 5 used for provenance generation).
- **Where:** `src/ui/install/stage_downloading.rs` (chassis; reusable — Phase 6 fork-download reuses it via `DownloadScreenCopy`). Phase 7 P7.T17 edits this same file to bind live `Step2UpdateDownloadEvent`/`Step2UpdateExtractEvent` progress.
- **Acceptance (Run 5 = chassis):** From Preview, `Import Modlist →` (enabled code) → Downloading renders the overall-progress Box + 4-col grid (empty/"no mods queued") + Cancel → Preview; auto-advance path to the stage-4 stub exists. `cargo build --bin BIO --release` clean (zero BIO source touched this run). Live per-mod progress + content-addressed archives = P7.T17 acceptance.
- **SPEC:** §4.3 (Phasing note), §13.12a (directory architecture + pipeline-reuse contract); live wiring → plan P7.T17.

### P5.T13 — `Install` stage 4 stub

- **What:** `stage_installing_stub::render` shows `ScreenTitle title="Installing modlist" sub="Install runtime arrives in Phase 7"` + a faint sub-line + a single `← Back to preview` button.
- **Where:** New file.
- **Acceptance:** Stage 4 renders without crashing. Clicking `Back to preview` returns to stage 2 (or `paste` if no preview is cached).
- **SPEC:** §4.4 (full implementation deferred to Phase 7).

### P5.T14 — Wire Install into `page_router`

- **What:** Replace the `NavDestination::Install` arm's stub with `bio::ui::install::page_install::render(...)` (or `crate::ui::install::page_install::render(...)` — both resolve to the same orchestrator-side new file because the orchestrator code lives inside the library crate per the Phase 1 carve-out #3 split; use the `crate::` convention when writing from inside another `src/ui/orchestrator/*` file, and `bio::` when writing from the binary entry).
- **Where:** Edit `src/ui/orchestrator/page_router.rs` (Phase 2 new file).
- **Acceptance:** Install rail item opens the real Install screen.
- **SPEC:** §4.

### P5.T15 — Wire Home into `page_router`

- **What:** Replace the `NavDestination::Home` arm's stub with `bio::ui::home::page_home::render(...)` (or `crate::ui::home::page_home::render(...)` from inside library-crate orchestrator code).
- **Where:** Edit `src/ui/orchestrator/page_router.rs`.
- **Acceptance:** Home rail item opens the real Home screen.
- **SPEC:** §3.

### P5.T16 — Toast notifications

- **What:** `Toast` floats in bottom-center, auto-dismisses after 1.8s. Used by Home `Copy import code`, `Deleted "<name>"`, and any other "this happened" feedback. Rendered as an `egui::Area` with `Order::Tooltip` so it stays above the destination content.
- **Where:** `src/ui/home/toast.rs`.
- **Acceptance:** Copying an import code from a Kebab menu produces the toast; it fades after ~1.8s.
- **SPEC:** §3.1, §10.8.

### P5.T17 — `Open install folder` Kebab action

- **What:** Uses `rfd::FileDialog` or platform-native (`std::process::Command::new("open" / "explorer" / "xdg-open")`) to open the install folder. If the folder no longer exists on disk, render the error in the status / error area at the bottom (per SPEC §3.2 Open install folder semantics).
- **Where:** `src/registry/operations.rs::open_install_folder` + wired into the Kebab callback.
- **Acceptance:** Clicking opens the folder; clicking when the folder is missing shows the error message.
- **SPEC:** §3.2 (Open install folder).

### P5.T18 — `Reinstall` Kebab action (preview-only this phase)

- **What:** Phase 5 wires the Kebab item but only opens a danger confirm dialog with the SPEC-verbatim body (§3.1 Reinstall semantics). On confirm, the action shows a toast `Reinstall queued — install runtime arrives in Phase 7`. Actual reinstall lands in Phase 7.
- **Where:** `src/ui/home/confirm_delete.rs` (sibling Reinstall confirm) and `src/registry/operations.rs::queue_reinstall_stub`.
- **Acceptance:** Confirm dialog renders with the right body. Confirming shows the placeholder toast.
- **SPEC:** §3.1 (Reinstall semantics; final wiring in Phase 7).

## Open questions / risks

- **Game launcher (deferred; button renamed per M6).** The wireframe's `play` button is renamed `open` in v1 alpha — the label honestly reflects the behavior (opens the install folder). When a game launcher ships in a future release the label flips back to `play` and the action launches the game. SPEC §3.2 documents this. No game-launcher implementation is in scope for v1 alpha.
- **Total size computation.** Cards show `<size>` for installed modlists. As noted in Phase 3, computing this requires a recursive `du`. Compute on install completion in Phase 7 and cache in the registry; for Phase 5, render `—` for size when unknown.
- **Live Downloading wiring is Phase 7, not Phase 5 (resolved scope).** Phase 5 ships the §4.3 Downloading **chassis** only (see P5.T12). Driving BIO's import → auto-build → download/extract pipeline, the per-install directory derivation, and the content-addressed archive staging land in Phase 7 P7.T17 (SPEC §13.12a) — because that pipeline terminates in the install runtime (Phase 7) and Phase 5 has no install runtime. The concurrent-download / no-UI-thread-blocking-during-large-extracts concern is therefore verified in Phase 7, not here. This corrects an earlier framing ("BIO can't" / "Install lacks paths") — both were wrong: the pipeline exists and the global paths are in Settings → Paths (§11.2); the spec simply hadn't defined the per-install architecture + reuse contract (now §13.12a).
- **Modlist name / author in the preview — RESOLVED, not an open gap.** The pasted share code now packs `name` / `author` / `forked_from` (SPEC §13.3 Provenance, carve-out #5); the preview reads them for the title/subline + `ForkInfoPopup`. Codes that lack them fall back to `Shared modlist` / author-less copy — intentional, not a defect to re-flag. Generation that *populates* them is Phase 6/7 (`pack_meta`); in Phase 5 the fallback path is what renders in practice (consume-only run).

## Verification

1. `cargo build --bin infinity_orchestrator --release` succeeds.
2. Launch with empty registry: Home shows the first-launch setup CTA.
3. Click `Open Settings` → lands on Settings → Paths.
4. Use the Phase-3 dev `Seed test modlist (dev)` button to create one in-progress + one installed entry. Return to Home: filter chips show `(1)` and `(1)`, cards render with correct meta lines.
5. Click Kebab → Copy import code: toast appears, clipboard contains the code.
6. Click Kebab → Delete: confirm dialog appears with correct body. Confirm: card disappears, statusbar bumps down.
7. Click `paste import code` → Install opens. Paste a known good share code, click `Preview →`. Preview shows tabs with parsed content. Click `Import Modlist →` → the Downloading **chassis** renders (overall-progress Box + 4-col grid, empty in Phase 5) and `Cancel` returns to Preview; the stage-4 stub is reachable via the auto-advance path. (Live per-mod download data + content-addressed archives = Phase 7 P7.T17 / SPEC §13.12a — not exercised in Phase 5.)
8. Provenance: a pre-redesign / third-party code (no packed fields) shows title `Shared modlist`, no `by @…` segment, no `⑂ fork info` button — and `cargo test --lib` confirms the serde-default round-trip is behavior-neutral. (Codes that *carry* the trio only exist once Phase 6/7 generation ships; the display path is verified here, populated later.)
9. Navigate freely between Home / Install / Settings; no state loss.
10. `cargo build --bin BIO --release` continues to succeed; the legacy wizard is unaffected (carve-out #5 is schema-additive + behavior-neutral).
