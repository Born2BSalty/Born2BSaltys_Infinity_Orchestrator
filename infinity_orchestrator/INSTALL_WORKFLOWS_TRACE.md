# Install Workflows — End-to-End Trace

**Branch:** `xgatt/create-then-modify-fix` @ `7386195`
**Generated:** 2026-05-22

This document maps the six install/registry workflows end-to-end across the orchestrator codebase. It is **read-only investigation** — no source code is modified. Citations are absolute `file:line` so the parent agent can grep-verify every claim.

---

## Scope + notation

- **Citations** are `relative-path:line` from repo root, suitable for IDE click-through.
- **Call chains** use `→` (so: `caller (file.rs:NNN) → callee (file.rs:MMM)`).
- **State mutations** list `field` → `value` → `setter@file:line`.
- **File writes** list `path-expression` → `trigger function`.
- Identifiers trail in italics/parens (Option-B format); plain-language headlines are bolded.

---

## ⚠️ Risk Map

Read these first. Each row's "Where" links to the inline callouts below where the risk surfaces. Severity legend (3 levels only):

- 🔴 **Data loss** — file destruction, registry corruption, install failure.
- 🟠 **Cleanup / state gap** — orphaned files, unwired UI, missing teardown.
- 🟡 **Fragility / tech debt** — works today by coincidence, brittle gate logic, naming overload.

| # | Risk (one-line) | Severity | Where (sections) | Status |
|---|---|---|---|---|
| R1 | Fork install runs `prepare_destination` twice on Clear/Backup — wipes or double-archives the freshly-extracted Mods/ at Workspace Step-5 click | 🔴 Data loss | Shared/`destination_prep`, §3.1 code path, §3.2, §3.3 | **Closed (PR #18, 2026-05-27)** |
| R2 | `delete_modlist` orphans `%APPDATA%\bio\modlists\<id>\workspace.json` + the in-memory `workspace_state` / `workspace_stores` HashMaps | 🟠 Cleanup gap | §6 | Open |
| R3 | Reinstall variant is computed as `Install` (not `Reinstall`) at install-start because `pending_reinstall_id` clears at `PreviewOutcome::Advance` before `register_and_write_install_start_artifacts` runs | 🟡 Fragility | §4 code path | Open — works today by coincidence |
| R4 | Fork auto-build suppression depends on Run 4 + Run 5 patches clearing `modlist_auto_build_*` at the right moment to beat `advance_pending_saved_log_flow`'s race to `start_auto_build_install` | 🟡 Fragility | Shared/auto-build, §3 auto-build flag flow | Wired via Runs 4 and 5 |
| R5 | `from_workspace` (a single boolean derived from `loaded_workspace_id.is_some()`) gates five distinct decisions in `maybe_flip_to_installed_on_clean_exit`: id selection, share-code-override, saved-log pre-flip, `active_install_modlist_id` clear, post-install reset-gate arming | 🟡 Fragility | Shared/lifecycle, §1 lifecycle, §4 lifecycle | Open |
| R6 | Continue partial install (workflow 5) is NOT wired — `stage_paste` jumps `Paste → InstallingStub` directly, skipping `arm_pipeline_once` and `register_and_write_install_start_artifacts`; no registry entry, no flag application, no per-install dir derivation, no `active_install_modlist_id` | 🟠 Cleanup gap (dead UI) | §5 | Open — Spec-only |
| R7 | `delete_modlist` safety guard does not check `path.is_symlink()` — on Linux `fs::remove_dir_all` follows symlinks and recursively deletes the target | 🔴 Data loss | §6 safety-guard analysis | Open |
| R8 | Fork extract pipeline could hang on Downloading if every asset fails both hash-skip AND download (worst-case network outage); `archives_observed == 0 && update_selected_update_assets != []` never satisfies `fork_extract_complete` | 🟡 Fragility | §3 code path, Open Q#9 | Open |
| R9 | `flush_workspace_on_nav_away` skips persistence when a rescan or cold-resume is in flight (`restore_pending(&step2)`) — a mid-scan nav-away discards the user's pre-rescan local edits | 🟠 Cleanup gap | Cross-workflow concerns, Open Q#6 | Open |
| R10 | `arm_auto_build` deliberately leaves `pending_saved_log_download = false` — silent contract that the orchestrator owns the parallel `stream_downloader`; any backout to BIO's serial downloader would not re-arm and would silently produce zero downloads | 🟡 Fragility | Shared/auto-build, Open Q#8 | Open |
| R11 | Workspace from-scratch install picks `FreshCreate` workflow regardless of whether the user's `step3` selection references download-requiring mods — `apply_flags(FreshCreate)` leaves `download` following Settings → Advanced; when Settings has download off and the user has unstaged mods, the install fails silently | 🟡 Fragility | §2.1 code path, Open Q#10 | Open |
| R12 | `reset_install_screen_to_paste` clears `install_screen_state.parsed_preview` at fork-route-to-workspace — works today only because the Workspace sources fork lineage from `registry.find(id).forked_from`; any future code that reads workspace lineage from `install_screen_state.parsed_preview` would silently get `None` | 🟡 Fragility | §3 code path, Open Q#5 | Open |
| R13 | Fork install rewrites `modlist-import-code.txt` with `wizard_state`-packed code at install-start (and at clean-exit), NOT the originally-pasted parent code — easy to misread as a bug but intentional per the `forked_from` lineage append rule | 🟡 Fragility (read-misleading) | §3.1 code path | Open — by design |

**Recommended next fix** (full ranking in "Recommended fix order" at end): ~~**R1 (Data loss)**~~ closed by PR #18 — `mint_and_arm` now writes `pending_destination_prep: None` unconditionally; regression test pins the single-fire invariant. **R7 (Linux-only symlink data loss in delete)** is next, then **R6 (Continue partial install)**. R7 is now the only open Data-loss-tier risk.

---

## Shared subsystems (referenced by multiple workflows)

### Pipeline arming + the five `_once` pipeline functions

The Install-paste, Reinstall, and Create→Import-and-Modify workflows all converge on the same arming sequence inside the `Downloading` stage. The Install-paste and Reinstall variants run inside `page_install::render` → `stage_downloading::render_live`; the Fork-Import variant runs inside `page_create::render` → `stage_fork_download::render_live`. Both call sites invoke the same five `_once` functions in the same order, so the sequence is the single source of truth for "download/extract/ingest".

Order, per frame:

1. **`arm_pipeline_once`** *(`src/ui/install/stage_downloading.rs:565-630`)* — fires exactly once per stage entry (latched by `install_screen_state.pipeline_flags.armed()`). Calls **`destination_prep::prepare_destination`** *(`src/install_runtime/destination_prep.rs:29`)* → **`auto_build_driver::prepare_install_dirs_and_maybe_import`** *(`src/install_runtime/auto_build_driver.rs:19`)* → **`auto_build_driver::arm_download_archive_policy`** *(`src/install_runtime/auto_build_driver.rs:57`)* → **`install_modlist_registration::register_and_write_install_start_artifacts`** *(`src/install_runtime/install_modlist_registration.rs:84`)*.
2. **`stage_and_kick_archive_skip_once`** *(`src/ui/install/stage_downloading.rs:632`)* — latched by `archives_staged()`. Calls `archive_store::stage_known_archives` *(`src/install_runtime/archive_store.rs:213`)* + spawns the async checksum-then-skip worker (`archive_skip_async::start_async_archive_skip`).
3. **`kick_streaming_downloader_once`** *(`src/ui/install/stage_downloading.rs:717`)* — latched by `download_phase_started()`; gated on `download_gate_open(wizard_state)` *(`auto_build_driver.rs:82`)* (which requires the saved-log apply + update-preview to be done). Calls `stream_downloader::start_stream_download` *(`src/install_runtime/stream_downloader.rs`)*. Sets `wizard_state.modlist_auto_build_waiting_for_install = true`.
4. **`verify_downloaded_archives_once`** *(`src/ui/install/stage_downloading.rs:765`)* — latched by `archives_verified()`. Calls `archive_skip::verify_downloaded_archives` (hash-checks every downloaded archive; deletes any whose hash ≠ expected; never installs corrupt bytes).
5. **`ingest_downloaded_archives_once`** *(`src/ui/install/stage_downloading.rs:821`)* — latched by `archives_ingested()`. Calls `archive_store::ingest_downloaded_archives` *(`src/install_runtime/archive_store.rs:258`)* — hashes each archive, writes a content-addressed copy under `<mods-archive>/{name}__{hash}.{ext}`, hardlinks (or copies on cross-volume) so the deterministic path AND the hash-named path both exist.

The frame-loop drain functions (`drain_stream_download`, `drain_extract_parallel`, `drain_archive_skip_events`) live on `OrchestratorApp` *(`src/ui/orchestrator/orchestrator_app.rs:527-877`)* and feed events back into `wizard_state.step2.update_selected_*` + `install_screen_state.{hashed_indices, download_progress}`. **`drain_stream_download`'s `Finished` arm** *(`orchestrator_app.rs:583-606`)* calls `start_parallel_extract` to kick the extract phase; **`drain_extract_parallel`'s `Finished` arm** *(`orchestrator_app.rs:682-757`)* — when `extracted > 0` — re-arms `pending_saved_log_apply = true` and starts a `start_step2_scan` so the imported WeiDU log's selection re-applies onto the now-populated step2.

**Pipeline-completion advancement:**
- **Install-paste / Reinstall** — `auto_build_driver::pipeline_reached_install` *(`auto_build_driver.rs:101`)* returns true when `pipeline_finished && (start_install_requested || install_running || current_step == 4)`. `render_live` returns `DownloadingOutcome::Advance` → page_install flips stage to `InstallingStub` *(`page_install.rs:76-80`)*.
- **Create-Fork** — `fork_extract_complete` *(`stage_fork_download.rs:122`)* checks armed + skip_completed + download_phase_started + archives_verified + archives_ingested + nothing running + `archives_observed > 0`. Returns `ForkDownloadOutcome::Import` → `fork_extract_complete_route_to_workspace` *(`page_create.rs:383`)* routes to `NavDestination::Workspace`.

> ### ⚠️ R8 — Fork pipeline can hang on Downloading if every asset fails 🟡
>
> `fork_extract_complete` *(`stage_fork_download.rs:122-152`)* requires `archives_observed > 0 || update_selected_update_assets is empty` (the OR-clause covers the "all assets already on disk, all skipped" case). But if the share code references assets AND every asset fails BOTH the hash-skip phase AND the streaming download (worst-case network outage), `archives_observed` stays 0 AND `update_selected_update_assets` is non-empty → the gate never trips → the user is stranded on the fork-Downloading screen with no error surfaced. `pipeline_arm_error` only catches arm-time failures, not post-arm streamer failures; `archive_skip` worker disconnects bypass the gate by setting `archive_skip_completed = true`, but `stream_downloader` failure surfacing isn't audited here. Worth verifying with a forced-outage test.

> ### ⚠️ R10 — `pending_saved_log_download` deliberately not armed 🟡
>
> `arm_auto_build` *(`auto_build_driver.rs:65-79`, comment at `:75`)* sets `pending_saved_log_apply = true` and `pending_saved_log_update_preview = true` but deliberately does NOT set `pending_saved_log_download = true`. This is the redesign's silent contract that the orchestrator owns the parallel `stream_downloader` — BIO's `apply_saved_weidu_log_selection` would otherwise also try to download serially. If a future change ever reverts to BIO's serial downloader (or removes `stream_downloader` from the orchestrator), the absence of this flag-arm would cause zero downloads to fire and the install would silently produce an empty Mods folder. The contract has no in-code assertion; the only signal is the inline comment.

### `DestChoice` + `destination_prep`

`DestChoice` *(`src/ui/install/state_install.rs:18`)* has three variants: `Clear` / `Backup` / `Continue`.

**`DestChoice::to_flags`** *(`state_install.rs:54-77`)* maps to `DestFlags`:

| Choice | `prepare_target_dirs_before_install` | `backup_targets_before_eet_copy` | `skip_installed` | `check_last_installed` |
|---|---|---|---|---|
| Clear | true | false | false | false |
| Backup | true | true | false | false |
| Continue | false | false | true | true |

**`destination_prep::prepare_destination`** *(`src/install_runtime/destination_prep.rs:29`)* operates on the **entire destination directory**, not just BIO's EET subdirs:
- `Clear` → `fs::remove_dir_all(dest)?` + `fs::create_dir_all(dest)?` *(`destination_prep.rs:72-78`)* — permanently gone, no Recycle Bin (test at `:280`).
- `Backup` → `fs::rename(dest, _bio_backup_<name>_<unix_ts>)?` + recreate *(`destination_prep.rs:80-85, 99-111`)*.
- `Continue` / `None` → `Skipped { reason: Continue }` no-op *(`destination_prep.rs:40-50`)*.
- `dest` doesn't exist → `Skipped { reason: DoesNotExist }`.
- `dest` exists but empty → `Skipped { reason: AlreadyEmpty }`.
- `dest` exists but is a file → `io::Error::InvalidInput` *(`destination_prep.rs:57-62`)*.

**Four arm-time call sites:**
1. **Install-paste** — `stage_downloading::arm_pipeline_once` calls it before the pipeline arms *(`stage_downloading.rs:579-593`)*.
2. **Create-Fork** — `fork_pipeline_arm::mint_and_arm` records `pending_destination_prep` on `workspace.json`; `arm_pipeline_once` (same call site as #1) reads `install_screen_state.destination_choice` and applies it inside the shared arm.
3. **Reinstall** — `reinstall_route::start_reinstall` sets `destination_choice = Some(DestChoice::Clear)` *(`reinstall_route.rs:18`)*, which `arm_pipeline_once` then applies.
4. **Workspace Step-5 fresh-Install** — `workspace::step5::page_workspace_step5::consume_pending_destination_prep` reads `pending_destination_prep` off `workspace.json` and applies it *(`page_workspace_step5.rs:164-214`)*.

`Create → Scratch` also calls `prepare_destination` (directly, not via `arm_pipeline_once`) inside `prepare_scratch_mods_folder` *(`page_create.rs:328-343`)*, since the from-scratch path mints the modlist immediately and skips the Install screen.

> ### ✅ R1 — Fork install runs `prepare_destination` twice on Clear/Backup 🔴 — **CLOSED (PR #18, 2026-05-27)**
>
> **Status:** Closed. `fork_pipeline_arm::mint_and_arm` now writes `pending_destination_prep: None` regardless of the user's Clear/Backup choice *(`fork_pipeline_arm.rs:80-83`)* — the first fix-shape proposed below. The single-fire invariant is codified by the regression test `minted_workspace_never_persists_destination_prep_regardless_of_user_choice` *(`fork_pipeline_arm.rs:330-363`)*, which iterates every `DestChoice` variant and asserts the persisted field is `None`. The Workspace Step-5 consumer (`start_pending_destination_prep`) early-returns on `None` *(`page_workspace_step5.rs:183-185`)*, so consumer A2 is now a no-op on every fork install — the user's Clear/Backup intent is honored exactly once, at fork-download arm (consumer A1). The diagnosis below remains as forensic history.
>
> Fork mint persists the destination-prep choice into TWO state slots, and TWO different code sites later consume it:
>
> - **Slot A** — `fork_pipeline_arm::mint_and_arm` *(`fork_pipeline_arm.rs:55-58, 87`)* writes `pending_destination_prep: choice` to `workspace.json` AND sets `install_screen_state.destination_choice = choice`.
> - **Consumer A1** — `stage_downloading::arm_pipeline_once` *(`stage_downloading.rs:579-593`)* reads `install_screen_state.destination_choice` and runs `prepare_destination` at fork-download arm.
> - **Consumer A2** — `page_workspace_step5::consume_pending_destination_prep` *(`page_workspace_step5.rs:164-214`)* reads `workspace_state.pending_destination_prep` at Step-5 Install click and runs `prepare_destination` AGAIN against the same destination — which now contains the fork-extracted Mods/, weidu logs, and `modlist-import-code.txt`.
>
> **On Clear:** `consume_pending_destination_prep` runs `fs::remove_dir_all(dest)` + `fs::create_dir_all(dest)` *(`destination_prep.rs:72-78`)* — permanently wipes the freshly-extracted contents. No Recycle Bin (per test at `:280`). The install then proceeds against an empty destination, needs to re-download everything (and Step-5's `apply_flags` likely won't re-arm `pending_saved_log_apply`, so the imported selection may or may not survive).
>
> **On Backup:** `fs::rename(dest, _bio_backup_<name>_<unix_ts>)` *(`destination_prep.rs:80-85, 99-111`)* — moves the entire freshly-extracted workspace (Mods/, weidu_log_source/, import-code.txt, etc.) into a *second* `_bio_backup_..._<ts>` snapshot. (The first one, if there was prior content, was created at the fork arm consumer A1.) The install then proceeds against an empty destination, re-downloading everything — the original fork-download is wasted, though recoverable from the backup snapshot.
>
> **Fix shape:** either drop the `pending_destination_prep: choice` write in `mint_and_arm` (since A1 consumes the choice immediately and A2 should never need to repeat), or have `consume_pending_destination_prep` short-circuit when `install_screen_state.pipeline_flags.armed() && archives_ingested()` (the choice was already consumed). Mirror change for the analogous Backup variant of Workflow 4 if/when Reinstall ever surfaces a non-Clear option.

### Registry / Workspace persistence

**`modlists.json`** (the modlist registry) lives at `%APPDATA%\bio\modlists.json` via `RegistryStore::new_default()`. Writes happen via:
- **Atomic writes** (`store.save(&registry)`) at: every install-start *(`start_hooks.rs:80-82`, `install_modlist_registration.rs`)*, `flip_to_installed` *(`registry_transition.rs:99-108`)*, `flip_to_in_progress` *(`registry_transition.rs:264`)*, `delete_modlist` *(`operations.rs:61`)*, fork mint *(`fork_pipeline_arm.rs:72`)*, scratch mint *(`page_create.rs:307`)*, and the size-fill worker *(`orchestrator_app.rs:443`)*.
- **Debounced cycle** — `RegistryPersistenceCycle::persist_registry_if_needed` *(`src/registry/persistence_cycle.rs`)* fires via `OrchestratorApp::tick_persistence` *(`orchestrator_app.rs:961-1016`)* every frame; `mark_registry_dirty` is called by every code path above that's not the atomic save.
- **Drop / on_exit flush** — `flush_all_now` *(`orchestrator_app.rs:1056`)* wipes pending state to disk on exit; `Drop for OrchestratorApp` does the same on drop *(`orchestrator_app.rs:1267-1271`)*.

**`%APPDATA%\bio\modlists\<id>\workspace.json`** holds per-modlist state. Writes via `WorkspaceStore::save` from:
- Scratch mint *(`page_create.rs:292`)*.
- Install-paste registration *(`install_modlist_registration.rs:197`)*.
- Fork mint *(`fork_pipeline_arm.rs:59`)*.
- Workspace nav-away flush *(`page_router.rs:170-182`)*.
- Fork-extract-complete persist *(`stage_fork_download.rs:217`)*.
- Step-5 destination-prep consume *(`page_workspace_step5.rs:191`)*.
- Debounced cycle via `mark_workspace_dirty(<id>)`.

**`%APPDATA%\bio\bio_settings.json`** holds the BIO global settings. Writes happen via `tick_bio_settings` *(`orchestrator_app.rs:1034-1054`)*, debounced 1000ms. The snapshot is built by `bio_settings_snapshot` *(`orchestrator_app.rs:1018-1032`)*, which **clones `wizard_state.step1` and runs `sanitize_step1_for_settings_persistence`** *(`src/install_runtime/settings_sanitizer.rs:7-37`)* against the clone — restoring 11 per-install path fields + `weidu_log_mode` + 5 per-install booleans to the previously-persisted global values, so per-install pollution never persists.

**`<destination>/modlist-import-code.txt`** is written by `import_code_writer::write_modlist_import_code_txt` *(`src/install_runtime/import_code_writer.rs:10-18`)* — `fs::create_dir_all` + `fs::write`. Called from:
- `start_hooks::write_install_start_artifacts` *(`start_hooks.rs:92`)* — used by Workspace fresh-Install path; builds the code from `wizard_state` via `pack_meta` with `allow_auto_install = false`.
- `start_hooks::write_install_start_artifacts_with_code` *(`start_hooks.rs:148`)* — used by Install-paste / Reinstall via `install_modlist_registration`; takes a held code and flips its `allow_auto_install` bit to `false` (via `share_export::set_allow_auto_install`).
- `registry_transition::flip_to_installed` *(`registry_transition.rs:118`)* — overwrites with the `allow_auto_install = true` verified code on clean exit.

---

## Workflow 1 — Install from paste

> **Risks in this workflow:** R5 🟡 (lifecycle gating) — see inline callout in §1.1 Workspace vs Install lifecycle. R10 🟡 (auto-build silent contract) — applies to every share-code-consuming flow.

**Entry chain:** Home → `paste import code` CTA *(`add_a_modlist.rs`)* → `NavDestination::Install` → `page_install::render` → `InstallStage::Paste`.

### 1.1 New folder

#### User journey *(SPEC §4.1–§4.4)*

1. User clicks `paste import code` on Home → routes to Install.
2. Paste screen ("Install shared modlist"): user fills the destination FolderInput, then pastes the BIO-MODLIST-V1 code into the textarea.
3. Destination is new on disk → no `DestinationNotEmptyWarning` Box, no choice required.
4. Footer primary `Preview →` becomes enabled (destination is a valid dir AND code is non-empty); user clicks.
5. Preview parses, renders Overview + 6 tabs; user clicks `Import Modlist →`.
6. Downloading screen runs the live pipeline (Hashing → Downloading → Extracting → Preparing to install …); per-mod grid live-updates.
7. Auto-route to `InstallingStub` where Step-5 console takes over; user watches install.
8. On clean exit (`install_running == false && last_exit_code == Some(0) && !last_install_failed`): success banner + `Return to Home` / `Open install folder` row.

#### Code path

`page_install::render` *(`src/ui/install/page_install.rs:21-103`)* → `stage_paste::render` returns `PasteOutcome::Advance(InstallStage::Preview)` → `run_preview_parse` *(`page_install.rs:105-117`)* calls `preview_modlist_share_code` and caches → stage flips to `Preview` → user clicks Preview's primary → `PreviewOutcome::Advance` → stage flips to `Downloading` → `stage_downloading::render_live` *(`stage_downloading.rs:502-530`)* runs the five `_once` functions described above → at completion `pipeline_reached_install` returns true → stage flips to `InstallingStub` → `stage_installing::render` *(`stage_installing.rs:29`)* embeds Step-5 → install proceeds → drain functions on `OrchestratorApp` update `wizard_state.step5` → `eframe::App::update` *(`orchestrator_app.rs:1158-1264`)* observes `install_running` transition to false → `maybe_flip_to_installed_on_clean_exit` *(`orchestrator_app.rs:365-431`)* fires.

`register_and_write_install_start_artifacts` *(`install_modlist_registration.rs:84-136`)*:
- Detects `pending_reinstall_id.is_none()` AND `existing_entry_id_for_destination` returns None *(`:138-162`)* → registers a fresh `ModlistEntry` in `ModlistState::InProgress` via `register_install_modlist_paste` *(`:20-67`)*.
- Sets `active_install_modlist_id = Some(<new-id>)`.
- Calls `write_install_start_artifacts_with_code(variant, &code_source, ...)` which writes `modlist-import-code.txt` with `allow_auto_install = false`.

#### State mutations

| Field | New value | Where (file:line) |
|---|---|---|
| `install_screen_state.stage` | `Paste → Preview → Downloading → InstallingStub` | `page_install.rs:93-101` |
| `install_screen_state.destination` | user-typed path | `stage_paste.rs:46-52` |
| `install_screen_state.destination_choice` | `None` (folder doesn't exist) | n/a (warning not rendered) |
| `install_screen_state.import_code` | pasted code | `stage_paste.rs:78` |
| `install_screen_state.parsed_preview` | `Some(ModlistSharePreview)` | `page_install.rs:108` |
| `install_screen_state.pipeline_flags.{armed, archives_staged, …}` | sequentially true | `stage_downloading.rs:572, 651, 681, 743, 791, 856` |
| `wizard_state.step1.mods_folder`, `weidu_log_folder`, `weidu_log_mode`, log-source-folders, EET-clone-dirs, `generate_directory` | derived per-install | `per_install_dirs::derive_per_install_dirs` (`per_install_dirs.rs:121-149`) via `auto_build_driver::prepare_install_dirs_and_maybe_import` |
| `wizard_state.step1.{download_archive, mods_archive_folder, download}` | true / Settings value / true | `auto_build_driver::arm_download_archive_policy` (`auto_build_driver.rs:57-63`) |
| `wizard_state.modlist_auto_build_active` | true | `arm_auto_build` (`auto_build_driver.rs:65-79`) |
| `wizard_state.modlist_auto_build_waiting_for_install` | true | `kick_streaming_downloader_once` (`stage_downloading.rs:739`) |
| `wizard_state.step2.pending_saved_log_apply` | true | `arm_auto_build` (`auto_build_driver.rs:74`) |
| `wizard_state.step2.pending_saved_log_update_preview` | true | `arm_auto_build` (`auto_build_driver.rs:75`) |
| `wizard_state.step2.pending_saved_log_download` | **NOT armed** (orchestrator owns download) | `auto_build_driver.rs` (deliberately absent) |
| `wizard_state.step2.update_selected_*` | filled by drain functions | `orchestrator_app.rs:527-877` |
| `wizard_state.step5.last_exit_code = Some(0)` + `install_running = false` | post-install | drained by `app_step5_flow::poll_step5_terminal` |
| `OrchestratorApp.active_install_modlist_id` | `Some(<id>)` | `install_modlist_registration.rs:128` |
| `OrchestratorApp.post_install_reset_gate` | `Pending` | `maybe_flip_to_installed_on_clean_exit` (`orchestrator_app.rs:429`) |

Note: `from_workspace = self.workspace_view.loaded_workspace_id.is_some()` *(`orchestrator_app.rs:370`)*. Install-paste is **not** workspace-loaded, so `from_workspace = false` and the reset gate arms.

> ### ⚠️ R5 — `from_workspace` gates five distinct decisions 🟡
>
> Inside `maybe_flip_to_installed_on_clean_exit` *(`orchestrator_app.rs:365-431`)*, the single boolean `from_workspace = loaded_workspace_id.is_some()` is read at five separate gates:
>
> 1. **ID selection** *(`:371-376`)* — workspace path uses `loaded_workspace_id`; non-workspace uses `active_install_modlist_id`.
> 2. **Share-code override** *(`:385-392`)* — non-workspace reads `latest_share_code` from the registry (verbatim preservation); workspace path uses None (regenerate via `pack_meta`).
> 3. **Saved-log pre-flip** *(`:394-397`)* — only non-workspace runs `apply_saved_weidu_log_selection + sync_step3_from_step2`.
> 4. **`active_install_modlist_id` clear** *(`:417-419`)* — only non-workspace clears it (workspace install keeps the success banner alive).
> 5. **`post_install_reset_gate` arm** *(`:428-430`)* — only non-workspace arms `Pending`.
>
> Any future workflow that needs only one of these behaviors (e.g. wants saved-log pre-flip on the workspace path, OR wants to share-code-preserve on workspace) has nowhere to express that — the boolean is shared. Run 5's bug was exactly this kind of conflation (gate #5 mis-firing for workspace). Each decision deserves its own named gate.

#### File writes

| Path | Trigger | Content |
|---|---|---|
| `%APPDATA%\bio\modlists.json` | `start_hooks::write_install_start_artifacts_with_code` (`start_hooks.rs:137`); `flip_to_installed` (`registry_transition.rs:99`) | New entry InProgress at install start; flipped to Installed on clean exit |
| `%APPDATA%\bio\modlists\<id>\workspace.json` | `install_modlist_registration::persist_new_install_workspace` (`install_modlist_registration.rs:194-217`) | Empty `ModlistWorkspaceState::default()` |
| `<destination>/modlist-import-code.txt` | At install start: `start_hooks::write_install_start_artifacts_with_code` (`start_hooks.rs:148`) with `allow_auto_install=false`; on clean exit: `registry_transition::flip_to_installed` (`registry_transition.rs:118`) with `allow_auto_install=true` | BIO-MODLIST-V1 code |
| `<destination>/weidu_log_source/{bgee,bg2ee}/weidu.log` | `import_modlist_share_code` (BIO) inside `auto_build_driver::prepare_install_dirs_and_maybe_import` (`auto_build_driver.rs:39`) | Imported WeiDU log from share-code payload |
| `<destination>/Mods/<MOD>/...` | `extract_parallel` (`OrchestratorApp::handle_extract_finished` collects extracted sources at `orchestrator_app.rs:697`) | Extracted archives |
| `<destination>/weidu_component_logs/...` | WeiDU during install (per `weidu_log_mode = "log <folder>"`) | Per-component WeiDU logs |
| `<mods-archive>/{name}` AND `{name}__{hash}.{ext}` | `archive_store::ingest_downloaded_archives` (`archive_store.rs:258-330`) | Content-addressed dedupe pair (hardlinked) |
| `%APPDATA%\bio\bio_settings.json` | `tick_bio_settings` (`orchestrator_app.rs:1034-1054`) — debounced 1000ms; sanitizer scrubs per-install fields | Global settings only |

#### Auto-build flag flow

Timeline:

1. **Pipeline arm** — `arm_auto_build` sets `modlist_auto_build_active = true`, `pending_saved_log_apply = true`, `pending_saved_log_update_preview = true`, `pending_saved_log_download = false` (deliberately — orchestrator owns the parallel download).
2. **Drain `archive_skip_async::Finished`** — sets `pipeline_flags.archive_skip_completed = true` *(`orchestrator_app.rs:847`)*. The `pending_saved_log_apply` + `update_preview` still pending.
3. **`kick_streaming_downloader_once`** *(`stage_downloading.rs:717`)* — waits on `download_gate_open` *(`auto_build_driver.rs:82`)* which requires `pending_saved_log_apply && pending_saved_log_update_preview` to both be false. So this DOES NOT actually fire until the saved-log flow has cleared those. **`advance_pending_saved_log_flow`** *(`app_step2_saved_log_flow.rs:28-129`)* — called every frame via `OrchestratorApp::poll_step2_channels` *(`orchestrator_app.rs:517`)* — kicks a `start_step2_scan`, then on scan completion calls `apply_saved_weidu_log_selection` and `preview_update_selected`, clearing the two pending flags.
4. **Streamer downloads** → `drain_stream_download::Finished` → spawns `start_parallel_extract`.
5. **Extracts complete** → `drain_extract_parallel::Finished` → re-arms `pending_saved_log_apply = true`, kicks another scan.
6. **Scan completes; apply runs again** → `pending_saved_log_apply = false`. `advance_pending_saved_log_flow` then sees `modlist_auto_build_active && modlist_auto_build_waiting_for_install && nothing running` (`app_step2_saved_log_flow.rs:111-128`) → calls `start_auto_build_install` which clears both auto_build flags AND sets `step5.start_install_requested = true`, `current_step = 4`.
7. **`pipeline_reached_install` returns true** → `stage_downloading::render_live` returns `Advance` → install screen flips to `InstallingStub`.
8. **Step-5 runs the install** in `stage_installing`.

#### Workspace vs Install lifecycle

- `loaded_workspace_id` stays None throughout this workflow (Install-paste never enters a Workspace).
- `active_install_modlist_id` is set at `register_and_write_install_start_artifacts` *(`install_modlist_registration.rs:128`)* and cleared by `maybe_flip_to_installed_on_clean_exit` *(`orchestrator_app.rs:418`)* on clean exit (since `from_workspace = false`).
- `post_install_reset_gate` is armed `Pending` ONLY when `from_workspace = false` *(`orchestrator_app.rs:428-430`)*.
- `reset_completed_install_route_on_nav_away` *(`page_router.rs:236`)* fires when the user navs away from Install with the gate Pending and no Step-5 attempt in progress → calls `reset_completed_install_runtime` *(`page_router.rs:266-293`)*: drops step5 terminal, console-view, `pending_reinstall_id`, `active_install_modlist_id`, runs `install_screen_state.reset_to_paste()`, `wizard_state.reset_workflow_keep_step1()`, then `sanitize_step1_for_settings_persistence` against `bio_settings_last_saved.step1`.
- Alternative: `reset_completed_install_route_on_enter_install` *(`page_router.rs:249`)* fires when the user re-enters Install from any other screen with the gate Pending → same `reset_completed_install_runtime` call.
- `clear_pending_reinstall_on_nav_away_from_install` *(`page_router.rs:194-234`)* clears `pending_reinstall_id` AND `active_install_modlist_id` if nav-away from Install with no install running. **Exception:** explicit carve-out for `nav == Create && create_screen_state.stage == CreateStage::ForkDownload` — does not clear (would trap user on fork-download).

### 1.2 Existing folder → Clear

#### User journey

Same as 1.1, but the destination already contains files. The Paste screen detects the non-empty destination *(`stage_paste.rs:57`, `destination_is_non_empty`)* and renders the `DestinationNotEmptyWarning` Box with three toggle-style buttons: `Clear contents` / `Backup contents then proceed` / `Continue partial installation` *(`destination_not_empty::render`)*. User picks `Clear`. Footer primary remains `Preview →`.

#### Code path

Same as 1.1, except:
- `state.destination_choice = Some(DestChoice::Clear)` *(`stage_paste.rs:59-62`)*.
- `arm_pipeline_once` *(`stage_downloading.rs:579-593`)* calls `destination_prep::prepare_destination(&dest_path, Some(DestChoice::Clear))` which runs `fs::remove_dir_all(dest) + fs::create_dir_all(dest)` *(`destination_prep.rs:72-78`)* — permanently wipes the destination before any download starts.
- Result: `DestinationPrepReport::Cleaned { children_removed }`.
- On failure: `pipeline_arm_error = Some(msg)` *(`stage_downloading.rs:584`)*, surfaced via the non-masking banner in `render_chrome` *(`stage_downloading.rs:940-943`)*.

#### State mutations

Same as 1.1 plus `install_screen_state.destination_choice = Some(DestChoice::Clear)`. `DestChoice::to_flags` is not applied here — `flag_policies::apply_flags` runs via the workspace path (`start_hooks::on_install_start`), and Install-paste hits `register_and_write_install_start_artifacts` which uses `share_export::set_allow_auto_install`, not `apply_flags`. The `prepare_target_dirs_before_install` boolean stays false on this path; **the wider `prepare_destination` already wiped the destination**, so the BIO subdir-prep call becomes a defensive no-op (SPEC §13.12 #6).

#### File writes

Same as 1.1, plus: **destination's entire prior content is deleted at arm time** (no Recycle Bin per the test at `destination_prep.rs:280`). Then the same writes proceed against the now-empty destination.

#### Auto-build flag flow

Identical to 1.1.

#### Workspace vs Install lifecycle

Identical to 1.1.

### 1.3 Existing folder → Backup

#### User journey

Same as 1.2 but user picks `Backup contents then proceed`.

#### Code path

Same as 1.2, except `destination_prep::prepare_destination(_, Some(DestChoice::Backup))` runs `fs::rename(dest, _bio_backup_<name>_<unix_ts>)` + `fs::create_dir_all(dest)` *(`destination_prep.rs:80-85`)*. Backup path = `<parent>/_bio_backup_<dest-name>_<unix_ts>` *(`destination_prep.rs:99-111`)*. The prior contents survive at the backup path.

#### State mutations

Same as 1.1 plus `install_screen_state.destination_choice = Some(DestChoice::Backup)`.

#### File writes

Same as 1.1, plus the rename — full prior tree now at `_bio_backup_<name>_<ts>`. Empty `dest` is recreated. Subsequent install writes go into the recreated empty `dest`.

#### Auto-build flag flow

Identical to 1.1.

#### Workspace vs Install lifecycle

Identical to 1.1.

---

## Workflow 2 — Create from scratch

> **Risks in this workflow:** R11 🟡 (silent install failure when Settings → Advanced download is off and user has unstaged mods) — see callout in §2.1 code path.

**Entry chain:** Home → `create your own` CTA → `NavDestination::Create` → `page_create::render` → `CreateStage::Choose`. **Choose** stage = Setup Box (name + game + destination) + two starting-point boxes. User picks **New modlist from downloaded mods**. Single bottom-pinned `Start →` button.

Note: this workflow does NOT collect a share code. It mints an in-progress entry immediately and lands the user on **Workspace Step 2** with an empty `step2.bgee_mods` / `bg2ee_mods` selection. The user runs a manual mod scan (dev-only via `-d`) or — in production — must wait for P7.T17's per-install scan path, but the modlist already exists.

### 2.1 New folder

#### User journey *(SPEC §5.1)*

1. User clicks `create your own` → routes to Create → Choose stage.
2. Setup Box: user types `modlist name`, picks `game` from ComboBox (default EET), picks destination via FolderInput.
3. **From-scratch box** is selected by default. User leaves it selected.
4. User clicks `Start →`.
5. Lands on Workspace at Step 2 with an empty modlist set.

#### Code path

`page_create::collect_stage_request` *(`page_create.rs:70-111`)*:
- `CreateStage::Choose` → `stage_choose::render` returns `ChooseOutcome::StartScratch` → `handle_create_request` calls `start_scratch` *(`page_create.rs:239-326`)*.

`start_scratch` *(`page_create.rs:239`)*:
- Resolves `name` (rejects empty), `game`, `dest` (defaults to `default_destination(name)` if blank).
- Calls **`prepare_scratch_mods_folder`** *(`page_create.rs:328-343`)*:
  - Calls `destination_prep::prepare_destination(dest, choice)` if `choice` is Some — **this is the from-scratch destination prep path**, separate from `arm_pipeline_once`'s call.
  - Calls `per_install_dirs::resolve(destination, game)` to get the mods-folder path.
  - Creates `mods_folder` with `fs::create_dir_all`.
  - Returns `mods_folder` as a `String`.
- Calls `create_modlist(&name, game, &dest, &mut registry)` *(`src/registry/operations_create.rs`)* — mints a new `ModlistEntry` in `ModlistState::InProgress` with a fresh ULID id; pushes onto `registry.entries`.
- Builds canonical `WorkspaceStore::new_for_id(&entry.id)` and saves `ModlistWorkspaceState { scratch_mods_folder: Some(scratch_mods_folder), ..Default }`.
- Atomic `registry_store.save(&registry)`.
- `persistence_cycle.mark_registry_dirty`.
- Clears `create_screen_state.modlist_name`, `destination`, `destination_choice`.
- Sets `create_screen_state.resumed_build_id = Some(new_id.clone())`.
- Sets `orchestrator.nav = NavDestination::Workspace { modlist_id: Some(new_id) }`.

The user is now on Workspace Step 2; **no install runtime is engaged yet**. Steps 2/3/4 are user-driven; Step 5 is where the install actually starts via `page_workspace_step5::handle_start_install` *(`page_workspace_step5.rs:96-162`)*.

#### State mutations

| Field | New value | Where |
|---|---|---|
| `create_screen_state.{modlist_name, destination, destination_choice}` | cleared after mint | `page_create.rs:319-321` |
| `create_screen_state.resumed_build_id` | `Some(new_id)` | `page_create.rs:322` |
| `registry.entries[].state` | `InProgress` | `create_modlist` |
| `workspace_state[<id>]` | `ModlistWorkspaceState { scratch_mods_folder: Some(path), ..Default }` | `page_create.rs:288-302` |
| `workspace_stores[<id>]` | `WorkspaceStore::new_for_id` | `page_create.rs:304-306` |
| `nav` | `Workspace { modlist_id: Some(new_id) }` | `page_create.rs:322-325` |

**At Step 5 Install click** (`page_workspace_step5::handle_start_install`):
- `workspace_step5.install_clicked = true` *(`page_workspace_step5.rs:97`)*.
- `consume_pending_destination_prep` *(`:117, :164`)* — for from-scratch, this is None (the prep already ran at `start_scratch` time), so this returns true unconditionally.
- `workflow = registry.find(id).filter(|e| !e.forked_from.is_empty()).map_or(FreshCreate, |_| ShareCodeConsuming)` — for a from-scratch modlist with empty `forked_from`, this is `InstallWorkflow::FreshCreate` *(`page_workspace_step5.rs:122-128`)*.
- `start_hooks::on_install_start(modlist_id, variant, workflow, ...)` *(`start_hooks.rs:161-204`)*:
  - `flag_policies::apply_flags(step1, FreshCreate, settings)` — clears `-s`/`-c`; download follows Settings → Advanced.
  - `write_install_start_artifacts(modlist_id, variant, wizard_state, registry, store)` — uses `share_export::pack_meta(wizard_state, ShareMeta::from_entry(entry, false))` *(`start_hooks.rs:71`)* — builds a fresh code (allow_auto_install=false) from the populated `wizard_state` and writes `<dest>/modlist-import-code.txt`.
  - `per_install_dirs::derive_per_install_dirs(step1, dest, game)` *(`per_install_dirs.rs:104-172`)* — sets the same per-install dir set described in workflow 1.
- `wizard_state.step5.start_install_requested = true` *(`page_workspace_step5.rs:150`)*.
- Step-5 prep runs → install runs → drains → `maybe_flip_to_installed_on_clean_exit` fires.

> ### ⚠️ R11 — Workspace from-scratch install can silently fail when downloads are needed but disabled 🟡
>
> At Step-5 Install click *(`page_workspace_step5.rs:122-128`)*, the workflow is computed as `InstallWorkflow::FreshCreate` when `forked_from.is_empty()`, regardless of whether `wizard_state.step3` references mods that need to be downloaded. `apply_flags(FreshCreate, settings)` then leaves `wizard_state.step1.download` following the Settings → Advanced `download` flag (default `true`). If the user has explicitly disabled download in Settings AND the user's modlist references download-requiring mods, the install runs with `download = false`, BIO does not fetch the archives, extraction has nothing to extract, and the install proceeds against a half-empty Mods folder. No on-screen surface flags this; the user sees a generic install failure (or an install that completes but skipped half the mods). Fix shape: in `FreshCreate`, force `download = true` when `step3` has any entry not already resolvable from local `mods_archive_folder` / scratch mods folder; or scan step3 against the archive index and warn at Install-click.

`maybe_flip_to_installed_on_clean_exit` *(`orchestrator_app.rs:365-431`)*:
- `from_workspace = true` here (the workspace is loaded).
- `share_code_override = None` (`from_workspace == true` → don't read from `registry.latest_share_code`).
- Skips the `apply_saved_weidu_log_selection + sync_step3_from_step2` pre-flip *(`:394-397`)* (those are install-paste only).
- Calls `flip_to_installed(id, registry, store, wizard_state, None)` — this regenerates the code via `pack_meta` since the override is None *(`registry_transition.rs:138, 166-181`)* and rewrites `<dest>/modlist-import-code.txt` with `allow_auto_install = true`.
- **DOES NOT** set `post_install_reset_gate = Pending` (`from_workspace == true` *(`:428`)*).
- **DOES NOT** clear `active_install_modlist_id` *(`:418`)* — the workspace install keeps the success banner + console alive.

#### File writes

| Path | Trigger | Content |
|---|---|---|
| `%APPDATA%\bio\modlists.json` | `registry_store.save` (`page_create.rs:307`); flip_to_installed | New entry InProgress → Installed |
| `%APPDATA%\bio\modlists\<id>\workspace.json` | `canonical_store.save` (`page_create.rs:292`) | `ModlistWorkspaceState { scratch_mods_folder: Some(...), ..Default }` |
| `<dest>` | recreated (and prior contents handled) by `prepare_destination` if user picked a non-empty existing folder | n/a |
| `<dest>/mods/` | `prepare_scratch_mods_folder` (`page_create.rs:340`) | empty staging folder; user populates via scan |
| `<dest>/modlist-import-code.txt` | at Step-5 Install click: `write_install_start_artifacts` (`start_hooks.rs:71-92`) builds via `pack_meta` with `allow_auto_install=false`; on clean exit: `flip_to_installed` rewrites with `=true` | |
| `<dest>/weidu_log_source/{bgee,bg2ee}/weidu.log` | At Step-4 → Step-5 nav: **`write_step4_weidu_logs_unconditional`** (`workspace_view.rs:117-181`) writes the user's step3 selection; BIO's `auto_save_step4_weidu_logs` would skip in `install_exactly_from_weidu_logs` mode, but from-scratch is `INSTALL_MODE_BUILD_FROM_SCANNED_MODS` so BIO's `auto_save_step4_weidu_logs` would also write — both paths now write at this seam. | Per-game weidu.log |
| `<dest>/Mods/<MOD>/...` | extract pipeline (if user-imported mods) | n/a for purely scanned local-mods |
| `<dest>/weidu_component_logs/...` | WeiDU during install | per-component logs |
| `<dest>/Baldur's Gate Enhanced Edition/` etc. | BIO's install runner (with `-p`/`-n`/`-g`) | cloned game folders |

#### Auto-build flag flow

`modlist_auto_build_active` stays **false** throughout this workflow — from-scratch never goes through `arm_auto_build` *(`auto_build_driver.rs:65`)*. Instead Step-5 explicitly drives via `step5.start_install_requested = true`. The `pending_saved_log_*` flags also stay false; there's no imported weidu.log to apply.

#### Workspace vs Install lifecycle

- `loaded_workspace_id` is set on workspace open *(`page_router.rs:124`)*.
- `post_install_reset_gate` stays `Idle` on clean exit (workspace-install). No `reset_completed_install_runtime` fires.
- `active_install_modlist_id` is **only set by Install-paste / Fork-paste**; for from-scratch it stays None. The clean-exit flip uses `loaded_workspace_id` *(`orchestrator_app.rs:371-376`)* to identify the modlist.
- `clear_pending_reinstall_on_nav_away_from_install` is a no-op (nothing set).
- Nav-away from Workspace before install completion: `flush_workspace_on_nav_away` *(`page_router.rs:137-192`)* extracts `wizard_state` → `workspace.json` (gated on `restore_pending(&step2)` to avoid wiping in-flight rescan/resume).

### 2.2 Existing folder → Clear

#### User journey

Same as 2.1 but destination already contains content; Setup Box renders the `DestinationNotEmptyWarning` (Clear / Backup only; Continue is disallowed in Create per SPEC §5.1.3).

#### Code path

Same as 2.1, except `create_screen_state.destination_choice = Some(DestChoice::Clear)` and `prepare_scratch_mods_folder(dest, game, Some(DestChoice::Clear))` calls `destination_prep::prepare_destination` first — full `fs::remove_dir_all` + recreate, then creates the mods folder.

#### State mutations + file writes

Same as 2.1 plus: destination's prior contents are deleted at `start_scratch` time, before the modlist is even minted. If `prepare_destination` errors, `start_scratch` returns early *(`page_create.rs:266-274`)* and no registry mutation happens.

### 2.3 Existing folder → Backup

#### User journey

Same as 2.1 but user picks Backup.

#### Code path

Same as 2.2 with `DestChoice::Backup` → `fs::rename` of prior tree to `_bio_backup_<name>_<ts>`, then recreate empty dest + mods folder.

#### State mutations + file writes

Same as 2.2 except the prior content is preserved at `_bio_backup_<name>_<ts>` instead of being deleted.

---

## Workflow 3 — Import then Modify (fork)

> **Risks in this workflow:** R1 🔴 (double `prepare_destination` on Clear/Backup, see callout in §3.1 + §3.2/§3.3), R4 🟡 (auto-build suppression race, callouts at the Run 4 and Run 5 patch sites), R12 🟡 (preview-clear at route-to-workspace), R13 🟡 (import-code is re-packed, not the parent's verbatim), R6 🟠 (related — Continue partial install jumps past the fork pipeline if a user ever picked it here, but Continue isn't surfaced on the fork-paste screen so this is a latent path).

**Entry chain:** Home → `create your own` → Create → Choose stage. User picks the **Import and modify another modlist** box (right side). The Game ComboBox is hidden (replaced by `imported` note); the game derives from the share code. User fills name + destination, clicks `Start →` → routes to `CreateStage::ForkPaste` → pastes share code → `Preview →` → `Begin Import →` (no `allow_auto_install` gate here).

The workflow then reuses **the same five `_once` pipeline functions as Install-paste**, but the route differs: at extract-complete the user lands on Workspace Step 2 with the imported selection visible.

### 3.1 New folder

#### User journey *(SPEC §5.3)*

1. User clicks `create your own` → Create → Choose.
2. Fills `modlist name` + `destination`, clicks Import box → game ComboBox is replaced by `imported` note.
3. Clicks `Start →` → routes to `ForkPaste` stage.
4. Pastes share code into the textarea → `Preview →`.
5. Preview shows the parent's title + provenance subline + 6 tabs; user clicks `Begin Import →`.
6. Fork-download stage runs the same Hashing → Downloading → Extracting pipeline.
7. **Crucially, the fork does NOT auto-route to Install** — `fork_extract_complete` triggers the route to Workspace Step 2.
8. User lands on Workspace Step 2 with the imported weidu-log selection applied.
9. User edits Step 2/3 selection → Step 4 → Step 5 → Install (manual click, not auto-route).
10. Clean exit → success banner + post-install row.

#### Code path

`page_create::collect_stage_request` *(`page_create.rs:70-111`)*:
- `Choose` → `ChooseOutcome::GoForkPaste` → `stage = ForkPaste`.
- `ForkPaste` → `ForkPasteOutcome::Preview` → `run_fork_preview_parse` *(`page_create.rs:362-374`)* → `stage = ForkPreview`.
- `ForkPreview` → `ForkPreviewOutcome::BeginImport` → **`fork_pipeline_arm::mint_and_arm(orchestrator)`** *(`fork_pipeline_arm.rs:26-98`)*:
  - `derive_inputs`: takes parent `name`/`author`/`game_install` from `preview`, user's `modlist_name` and `destination` from `create_screen_state` (fallbacks: `<parent> (fork)` if name blank; `default_destination(fork_name)` if dest blank); `choice` = `state.destination_choice`.
  - `create_forked_modlist(ForkedModlistInput { ... })` *(`operations_create.rs`)* mints the new entry with `forked_from = parent.forked_from ++ [{parent.name, parent.author}]` (the append rule — SPEC §13.3); state = `InProgress`.
  - `WorkspaceStore::new_for_id(&entry.id).save(&ModlistWorkspaceState { pending_destination_prep: choice, ..Default })`.
  - Atomic registry save.
  - **Populates `install_screen_state`**:
    - `clear_preview()` then `destination = dest`, `import_code = code`, `destination_choice = choice`, `parsed_preview = Some(preview)`, `preview_cached = true`, `stage = InstallStage::Downloading` *(`fork_pipeline_arm.rs:83-92`)*.
  - `create_screen_state.destination_choice = None`.
  - `pending_reinstall_id = None`; `active_install_modlist_id = Some(modlist_id)` *(`fork_pipeline_arm.rs:94-95`)*.
  - Returns `Ok(ForkMintReport { modlist_id })`.
- `page_create::handle_create_request::CreateRequest::ForkBeginImport` *(`page_create.rs:155-165`)* → sets `create_screen_state.stage = ForkDownload`.
- **NOTE**: the orchestrator is in `NavDestination::Create` and `create_screen_state.stage == ForkDownload`. The page_router still renders `page_create`. `page_create::collect_stage_request` hits the `ForkDownload` arm *(`page_create.rs:102-109`)* → `stage_fork_download::render_live`.

`stage_fork_download::render_live` *(`stage_fork_download.rs:47-120`)*:
- Calls **`stage_downloading::arm_pipeline_once`** *(`stage_fork_download.rs:51`)* — same `LivePipelineInputs` extraction (but `inputs.destination` from `install_screen_state.destination` = the **user's** destination, since `fork_pipeline_arm::mint_and_arm` populated it).
- Calls `stage_and_kick_archive_skip_once` / `kick_streaming_downloader_once` / `verify_downloaded_archives_once` / `ingest_downloaded_archives_once` — same as Install-paste.
- **Run 4 patch** *(`stage_fork_download.rs:65-79`)*: once `armed && archives_ingested && !update_selected_extract_running`, clears `modlist_auto_build_active = false` AND `modlist_auto_build_waiting_for_install = false`. This stops `advance_pending_saved_log_flow` from firing `start_auto_build_install` on the fork path.
- **Run 5 fork-extract-complete patch** *(`stage_fork_download.rs:81-101`)*: when `fork_extract_complete(orchestrator)` is true → clears the auto-build flags again, plus `pending_saved_log_update_preview = false`, `pending_saved_log_download = false`, all popup-open flags; then `persist_fork_resume_workspace_state` *(`stage_fork_download.rs:182-231`)* extracts `scratch_mods_folder` from `wizard_state.step1.mods_folder` and saves to `workspace.json` so the cold-resume path can re-scan if the user closes the app.

> ### ⚠️ R4 — Auto-build suppression is a race 🟡 (Mitigated via Runs 4 and 5)
>
> The fork pipeline arms `modlist_auto_build_active = true` (inside `prepare_install_dirs_and_maybe_import` for the `ShareCodeConsuming` workflow). At extract-completion, three things race:
>
> 1. `advance_pending_saved_log_flow` *(`app_step2_saved_log_flow.rs:111-128`)* — when apply settles AND `modlist_auto_build_active && _waiting_for_install`, it fires `start_auto_build_install` → flips `step5.start_install_requested = true` → routes the user to Install instead of Workspace.
> 2. **Run 4 patch** *(`stage_fork_download.rs:65-79`)* — when `armed && archives_ingested && !extract_running`, clears `_active` + `_waiting_for_install`.
> 3. **Run 5 patch** *(`stage_fork_download.rs:81-101`)* — idempotent re-clear plus `pending_saved_log_update_preview = false` + popup flags, at `fork_extract_complete`.
>
> The fix relies on Run 4 firing AT OR BEFORE the apply-completion frame that would otherwise satisfy gate 1's condition. The frame ordering inside `eframe::App::update` *(`orchestrator_app.rs:1158-1264`)* drives this — `stage_fork_download::render_live` runs inside the per-frame `page_create::render`, and `advance_pending_saved_log_flow` runs inside `OrchestratorApp::poll_step2_channels` *(`orchestrator_app.rs:517`)*. If frame-ordering ever changes, the suppression could be defeated. The clears are idempotent (Run 5 mops up), but the suppression has no in-code assertion or invariant — it's frame-ordering by convention.
- Returns `ForkDownloadOutcome::Import` when complete.

`page_create::collect_stage_request::CreateStage::ForkDownload` *(`page_create.rs:102-109`)*:
- `ForkDownloadOutcome::Import` → `active_install_modlist_id.clone().map(CreateRequest::ForkExtractCompleteRouteToWorkspace)`.
- `handle_create_request::CreateRequest::ForkExtractCompleteRouteToWorkspace(id)` → calls `fork_extract_complete_route_to_workspace(orchestrator, id)` *(`page_create.rs:383-397`)*:
  - `orchestrator.reset_install_screen_to_paste()` *(`orchestrator_app.rs:347-363`)* — drops the 3 pipeline receivers, clears install_screen_state, clears auto-build flags, resets stage to `Paste`, **clears `pending_reinstall_id` AND `active_install_modlist_id`** *(via `reset_install_pipeline_state` in `orchestrator_app.rs:1119-1156`)*.
  - Clears `create_screen_state.{fork_code, fork_preview, modlist_name, destination, destination_choice, fork_download_progress}`, sets `stage = Choose`, `resumed_build_id = Some(id)`, `nav = Workspace { modlist_id: Some(id) }`.

> ### ⚠️ R12 — `reset_install_screen_to_paste` clears `parsed_preview` 🟡
>
> `reset_install_screen_to_paste` calls `install_screen_state.clear_preview()` which sets `parsed_preview = None`. After fork-route-to-workspace, the Workspace renders fork lineage via `fork_meta_from_entry` *(`page_router.rs:345-357`)*, which reads `registry.find(id).forked_from`, NOT `install_screen_state.parsed_preview`. So today this clear is harmless. But any future code that reads the workspace's parent/lineage from `install_screen_state.parsed_preview` would silently get `None` and the parent info would not render. The "lineage source = registry, not install_screen_state" invariant has no in-code assertion.

Workspace open *(`page_router.rs:60-131`)* loads the modlist:
- `workspace_state.contains_key(id)` is true (the fork mint already inserted it).
- `workspace_state_loader::populate_wizard_state_from_workspace` *(`workspace_state_loader.rs:12-53`)* — sets game from entry, calls `sync_paths_from_settings` (resets step1 globals), applies `apply_scratch_mods_folder` (mods_folder + INSTALL_MODE_BUILD_FROM_SCANNED_MODS) IF `scratch_mods_folder` is Some, resets scanned step2 set, applies order from `workspace.json`.

**But wait** — when the fork-extract finishes, the saved-log selection has already been **applied to `wizard_state.step2` mid-pipeline** by `apply_saved_weidu_log_selection`. The orchestrator persists `wizard_state.step2/step3` into `workspace.json` via `persist_fork_resume_workspace_state` *(`stage_fork_download.rs:182`)*, which calls `extract_workspace_state_from_wizard` — but the `populate_wizard_state_from_workspace` on workspace-open **resets** step2 first *(`workspace_state_loader.rs:55-66`)*, then re-applies the order from the persisted `order_bgee` / `order_bg2ee`. So the imported selection survives via the persisted order, not via direct in-memory carry.

#### State mutations

| Field | New value | Where |
|---|---|---|
| `create_screen_state.{fork_code, modlist_name, destination, destination_choice}` | cleared after mint or route | `fork_pipeline_arm.rs:93`, `page_create.rs:385-392` |
| `create_screen_state.resumed_build_id` | `Some(new-id)` | `page_create.rs:393` |
| `registry.entries[]` | new entry InProgress with forked_from chain appended | `create_forked_modlist` |
| `workspace_state[<id>]` | `{ pending_destination_prep: choice, ..Default }` then later persisted with `order_bgee`/`order_bg2ee` from imported log | `fork_pipeline_arm.rs:55-58`, `stage_fork_download.rs:182` |
| `install_screen_state.{destination, import_code, destination_choice, parsed_preview, preview_cached, stage}` | populated by `mint_and_arm` | `fork_pipeline_arm.rs:83-92` |
| `install_screen_state.pipeline_flags.*` | sequential true | same as workflow 1 |
| `wizard_state.modlist_auto_build_active` | true at arm, **cleared by Run 4 patch** when archives_ingested + not running | `auto_build_driver.rs:66` + `stage_fork_download.rs:75-78` |
| `wizard_state.modlist_auto_build_waiting_for_install` | true at kick, cleared by Run 4 patch | `stage_downloading.rs:739` + `stage_fork_download.rs:78` |
| `wizard_state.step2.pending_saved_log_*` | toggled by `advance_pending_saved_log_flow`; finally cleared by Run 5 patch on extract complete | `stage_fork_download.rs:88-91` |
| `OrchestratorApp.active_install_modlist_id` | `Some(modlist_id)` at mint; cleared by `reset_install_screen_to_paste` at route-to-workspace | `fork_pipeline_arm.rs:95`, `orchestrator_app.rs:1155` |
| `OrchestratorApp.pending_reinstall_id` | `None` (explicit) | `fork_pipeline_arm.rs:94` |
| `nav` | `Workspace { modlist_id: Some(id) }` | `page_create.rs:394-396` |

**At Step 5 Install click** (after user reviews on Workspace):
- `consume_pending_destination_prep` *(`page_workspace_step5.rs:164-214`)* — for a fork, this is `Some(choice)` if user picked Clear/Backup on Create. It runs `destination_prep::prepare_destination(dest, Some(choice))` AGAIN at install-click time, then clears it.

> See R1 callout above (Shared/`destination_prep` section). This is the SECOND consumption site (consumer A2). The Clear variant of this call wipes the fork-extracted contents; the Backup variant archives them into a second `_bio_backup_<name>_<ts>`. **Data loss / wasted download.** ✅ **Closed by PR #18** — `mint_and_arm` now persists `None`; this consumer early-returns and never runs the second prep.

- `workflow = ShareCodeConsuming` because `!forked_from.is_empty()` *(`page_workspace_step5.rs:122-128`)*.
- `start_hooks::on_install_start(modlist_id, variant, ShareCodeConsuming, ...)` — `apply_flags` clears -s/-c, forces download=true, calls `write_install_start_artifacts(modlist_id, variant, wizard_state, registry, store)` → `pack_meta` builds a fresh code from wizard_state with `allow_auto_install=false`.

> ### ⚠️ R13 — Fork install rewrites `modlist-import-code.txt` with re-packed code 🟡 (by design, read-misleading)
>
> The on-disk `modlist-import-code.txt` written at fork install-start is the orchestrator's freshly-packed code from `wizard_state.step3` via `pack_meta`, NOT the originally-pasted parent code. This is intentional per the `forked_from` lineage append rule (SPEC §13.3) — the fork is a NEW share code with its own provenance, parented to the parent via `forked_from`. But a casual reader of the trace might expect the parent code to be preserved verbatim. The Reinstall flow (§4) IS verbatim-preserved via `share_code_override` *(`registry_transition.rs:139-165`)*; the fork flow is not.

- `wizard_state.step5.start_install_requested = true` → install runs → drains → clean exit.

`maybe_flip_to_installed_on_clean_exit` — `from_workspace = true`, `share_code_override = None` → `pack_meta` rebuilds the verified code with `allow_auto_install=true` and `archive_meta`. Workspace install banner stays visible.

#### File writes

Same as workflow 1 with these differences:
- `workspace.json` written at fork mint *(`fork_pipeline_arm.rs:59`)* with `pending_destination_prep`, then persisted again at extract-complete with `scratch_mods_folder` *(`stage_fork_download.rs:217`)*, then again at Step-5 Install click clearing `pending_destination_prep`, then again at clean exit via the registry transition flow.
- `<dest>/weidu_log_source/{bgee,bg2ee}/weidu.log` is written by `import_modlist_share_code` (BIO) when `prepare_install_dirs_and_maybe_import` runs at fork arm. This is the **parent's** weidu log.
- At Step-4 → Step-5 nav edge, `write_step4_weidu_logs_unconditional` *(`workspace_view.rs:117`)* **overwrites** those same log files with the user's edited step3 selection — the Run 5 patch. BIO's `auto_save_step4_weidu_logs` early-returns for `install_exactly_from_weidu_logs` (the mode the parent share code propagates), so without this redesign-side writer the user's edits would never reach disk.

#### Auto-build flag flow

Critical: the fork pipeline arms `modlist_auto_build_active = true` via `arm_auto_build` (inside `auto_build_driver::prepare_install_dirs_and_maybe_import` for `ShareCodeConsuming` workflow), but then **Run 4 and Run 5 patches actively clear these flags** at the extract-complete edge to prevent `advance_pending_saved_log_flow::start_auto_build_install` from auto-routing to Install. Result: user lands on Workspace, not Install.

Timeline:
1. **Fork arm** — `modlist_auto_build_active = true`, `pending_saved_log_apply = true`, `pending_saved_log_update_preview = true`.
2. **Pipeline runs** — hash → download → extract → drain re-arms `pending_saved_log_apply = true` after extract.
3. **Saved-log apply runs** → step2 populated with imported selection; `pending_saved_log_apply = false`.
4. **Run 4 patch** *(`stage_fork_download.rs:65-79`)* — `armed && archives_ingested && !extract_running` → clears `modlist_auto_build_active` + `_waiting_for_install`. This BEATS `advance_pending_saved_log_flow`'s race to `start_auto_build_install`.
5. **Run 5 patch** *(`stage_fork_download.rs:81-101`)* — `fork_extract_complete` returns true → clears the auto-build flags AGAIN (idempotent) plus the remaining pending flags + popup-open flags.
6. **Route to Workspace** — user is on Step 2 for review.

#### Workspace vs Install lifecycle

The fork install lifecycle has FOUR distinct life-stages:
1. **Create → ForkDownload** — `active_install_modlist_id` is set; `loaded_workspace_id` is None; `pending_reinstall_id` is None.
2. **Route to Workspace** — `reset_install_screen_to_paste` clears `active_install_modlist_id` to None; `page_router` opens the workspace and sets `loaded_workspace_id = Some(id)`.
3. **User edits + Step 5 Install click** — `start_install_requested = true`; no `active_install_modlist_id` is re-set (workspace path identifies by `loaded_workspace_id`).
4. **Clean exit** — `from_workspace = true` → flip stays on workspace, no post-install reset gate.

`clear_pending_reinstall_on_nav_away_from_install` carve-out *(`page_router.rs:204-211`)* — when `nav == Create && create_screen_state.stage == CreateStage::ForkDownload`, does NOT clear `active_install_modlist_id`. This is Run 1's fix — without it the fork-pipeline got `active_install_modlist_id` reset to None mid-pipeline, breaking the route at extract-complete.

### 3.2 Existing folder → Clear

> See R1 callout above (Shared/`destination_prep` section) — this variant produces the **Clear** branch of the double-prep: pipeline extracts into `<dest>/`, then Step-5 click runs `fs::remove_dir_all(<dest>)`, permanently destroying the extracted Mods/ folder, weidu_log_source/, and the install-start `modlist-import-code.txt`. Install then proceeds against an empty destination and must re-download. ✅ **Closed by PR #18** — Step-5 consumer no longer fires; the user's Clear choice is honored once at fork-download arm.

Same as 3.1 but `create_screen_state.destination_choice = Some(DestChoice::Clear)`. `prepare_destination` runs at fork arm *(`stage_downloading.rs:579-593`)*, wipes destination, then pipeline downloads + extracts into the empty destination. `workspace.json`'s `pending_destination_prep` is also set to `Some(Clear)`. At the eventual Workspace Step-5 Install click, `consume_pending_destination_prep` will fire `prepare_destination` again — wiping the freshly-extracted contents. The Mods folder is at `<dest>/mods/` per `per_install_dirs::resolve` *(`per_install_dirs.rs:74`)`; `fs::remove_dir_all(dest)` recursively wipes everything including `<dest>/mods/`.

### 3.3 Existing folder → Backup

> See R1 callout above (Shared/`destination_prep` section) — this variant produces the **Backup** branch: pipeline extracts into `<dest>/`, then Step-5 click runs `fs::rename(<dest>, _bio_backup_<name>_<ts2>)`, archiving the fresh extract into a second backup snapshot (the first one, from fork-arm time, captured the user's pre-fork content if any). Install then proceeds against an empty destination — the fork download is wasted but recoverable from the backup. ✅ **Closed by PR #18** — Step-5 consumer no longer fires; the user's Backup choice is honored once at fork-download arm.

Same risk on Backup — would back up the freshly-extracted Mods/ along with everything else; the second `_bio_backup_<name>_<ts>` would contain the extract output.

---

## Workflow 4 — Reinstall

> **Risks in this workflow:** R3 🟡 (variant computed as `Install` not `Reinstall` at install-start, callout in §4 code path), R5 🟡 (lifecycle gating, see Workflow 1).

**Entry chain:** Home → installed modlist card → Kebab → `Reinstall` → confirm dialog → `confirm_reinstall(orchestrator, &id)` → `reinstall_route::start_reinstall(entry, orchestrator)` → `NavDestination::Install` at stage `Preview`.

### Variant policy

**Reinstall always forces `DestChoice::Clear`** per SPEC §3.1. The `DestinationNotEmptyWarning` is NOT shown — the user already confirmed via the Reinstall danger dialog *(`reinstall_route.rs:18`)*. So this workflow has only one variant.

### User journey *(SPEC §3.1)*

1. User opens Home; sees an Installed card; clicks Kebab → `Reinstall`.
2. Danger `ConfirmDialog` (`Reinstall "<name>"?`) confirms; user clicks Reinstall.
3. App routes to Install screen, stage = Preview, with the stored `latest_share_code` pre-parsed.
4. User clicks `Reinstall →` primary CTA on the preview (SPEC §3.2 mentions `Reinstall →` not `Import Modlist →`; the redesign reuses the preview button label flow but the action is the same).
5. Downloading runs; install runs; clean exit → success banner + post-install actions.

### Code path

`page_home::render_reinstall_confirm` *(`page_home.rs:258-281`)*:
- On `Confirmed`: calls `reinstall_route_wire::confirm_reinstall(orchestrator, &id)` *(`reinstall_route_wire.rs:9-19`)*.
- `confirm_reinstall` clones the entry from registry; calls `reinstall_route::start_reinstall(entry, orchestrator)` *(`reinstall_route.rs:12-52`)*:
  - `install_screen_state.destination = modlist.destination_folder.clone()`.
  - `install_screen_state.import_code = modlist.latest_share_code.unwrap_or_default()`.
  - `install_screen_state.destination_choice = Some(DestChoice::Clear)` *(`reinstall_route.rs:18`)*.
  - `clear_preview()`; parse `import_code` via `preview_modlist_share_code` → `parsed_preview = Some(_)`; `preview_cached = true`.
  - Sets BIO step1 flags directly: `prepare_target_dirs_before_install = true`, `backup_targets_before_eet_copy = false` *(`reinstall_route.rs:41-46`)* (mirror of `DestChoice::Clear.to_flags()`).
  - `pending_reinstall_id = Some(modlist.id.clone())`.
  - `install_screen_state.stage = InstallStage::Preview`.
  - `nav = NavDestination::Install`.

User clicks Preview's primary on `page_install`:
- `PreviewOutcome::Advance` *(`page_install.rs:48-66`)*:
  - **Reinstall-specific** branch: `pending_reinstall_id` is `Some` → calls `start_hooks::reinstall_flip_at_install_click(&reinstall_id, wizard_state, registry, registry_store, pending_reinstall_id)` *(`start_hooks.rs:206-242`)*:
    - Reads variant via `InstallButtonVariant::from_step5_and_reinstall` → `Reinstall` (since `pending == Some(id)`).
    - Calls `registry_transition::flip_to_in_progress(id, registry, store)` *(`registry_transition.rs:237-278`)* — flips `Installed → InProgress`, persists.
    - Clears `pending_reinstall_id` so the next frame doesn't re-flip.
  - `stage = Downloading`.

`stage_downloading::render_live` *(`page_install.rs:70-80`)* runs the five `_once` functions exactly as Workflow 1:
- `arm_pipeline_once` calls `prepare_destination(dest, Some(DestChoice::Clear))` → wipes the destination.
- `prepare_install_dirs_and_maybe_import` with `workflow = ShareCodeConsuming` (since `parsed_preview` is Some and `is_partial = false`).
- `import_modlist_share_code` writes the stored share code's `weidu.log`s.
- `arm_auto_build` arms the pending_saved_log_* flags.
- Pipeline runs → `pipeline_reached_install` → `Advance` → `InstallingStub`.

`register_and_write_install_start_artifacts` *(`install_modlist_registration.rs:84`)*:
- `install_start_modlist_id` *(`:138-162`)*: `pending_reinstall_id` was just cleared by `reinstall_flip_at_install_click`. So this falls through to `existing_entry_id_for_destination` — finds the existing entry by destination match. Reuses the existing modlist ID (no double-registration).
- `variant = Reinstall` (via `from_step5_and_reinstall` — but `pending_reinstall_id` is None now). Actually wait — let me re-check.

  Looking more carefully: at `page_install.rs:49-64`, `reinstall_flip_at_install_click` is called BEFORE `stage = Downloading`. So when `arm_pipeline_once` runs the next frame, `pending_reinstall_id` is **already None**. Then `register_and_write_install_start_artifacts::install_start_modlist_id` checks `pending_reinstall_id` — None — falls through to `existing_entry_id_for_destination` (the destination matches the entry that just flipped to InProgress) and returns the existing ID.

  Variant computed at `install_modlist_registration.rs:95`: `InstallButtonVariant::from_step5_and_reinstall(&wizard_state, &modlist_id, pending_reinstall_id.as_deref())` = `from_step5(state, pending == Some(modlist_id))` = `from_step5(state, false)` since pending is None. So variant is `Install`, `Restart`, or `Resume` depending on `step5.{resume_available, has_run_once}`. For a fresh reinstall click, both are false → variant = `Install`.

> ### ⚠️ R3 — Reinstall variant transitions to `Install` before registration runs 🟡
>
> Frame N: User clicks Preview's primary. `page_install.rs:48-66` runs `reinstall_flip_at_install_click(reinstall_id, ...)` which (a) flips registry `Installed → InProgress`, (b) **clears `pending_reinstall_id`**. Then stage = `Downloading`. Frame N+1: `arm_pipeline_once` runs `register_and_write_install_start_artifacts`, which computes variant at `install_modlist_registration.rs:95` — but `pending_reinstall_id` is already None, so variant = `Install` (not `Reinstall`). This is correct by SPEC §13.13 (`Install` and `Reinstall` both `write/overwrite` the import code with `allow_auto_install=false`), but anyone tracing the code expecting the variant to remain `Reinstall` at install-start would be wrong. The behavior is right; the name is misleading. A renamed/restructured variant (e.g., explicit `ReinstallInProgress` carried via `wizard_state` through the frame edge) would make this less surprising.

  **Open Question 2 (resolved by R3 above): is the variant `Install` correct for the Reinstall flow?** The Reinstall flag was consumed by `reinstall_flip_at_install_click` (which is what flips the registry state). After that point, the install proceeds like a fresh Install-paste install — variant `Install`, writes the import code via `write_install_start_artifacts_with_code` with `allow_auto_install=false`. This matches SPEC §13.13's `Reinstall → write/overwrite` semantics (the file is overwritten with the new draft).

- `write_install_start_artifacts_with_code(modlist_id, variant, &code_source, ...)`:
  - `code_source` = `registry.find(modlist_id).latest_share_code` (non-empty for a previously-installed modlist) — uses the stored verified code as source.
  - `share_export::set_allow_auto_install(trimmed_source, false)` flips the bit to false (this is the install-start draft).
  - Saves registry with new `install_started_at` + `latest_share_code` = bit-flipped code.
  - Writes `<dest>/modlist-import-code.txt` with the bit-flipped code.
- `active_install_modlist_id = Some(modlist_id)`.

Clean exit → `maybe_flip_to_installed_on_clean_exit`:
- `from_workspace = false` (we're on Install screen, not Workspace).
- `share_code_override = registry.find(id).latest_share_code` (non-empty since the prior install left it set).
- Runs `apply_saved_weidu_log_selection + sync_step3_from_step2` *(`orchestrator_app.rs:394-397`)* — for the Install-Modlist-paste path, this re-derives step3 from the imported saved log so `count_mods_and_components` produces non-zero counts.
- Calls `flip_to_installed(id, registry, store, wizard_state, share_code_override.as_deref())`:
  - `share_code_override` is Some → `build_verified_code` takes the override path *(`registry_transition.rs:139-165`)*: `set_allow_auto_install(src, true)` + `bake_archive_meta_into_code(_, archive_meta)`. **Does NOT use `pack_meta`** — preserves the pasted code's provenance verbatim.
  - Rewrites `<dest>/modlist-import-code.txt` with the verified code *(`registry_transition.rs:118`)*.
- Clears `active_install_modlist_id` (`from_workspace == false`).
- Sets `post_install_reset_gate = Pending`.

### State mutations

| Field | New value | Where |
|---|---|---|
| `home_screen_state.reinstall_target` | `Some(id)` then `None` after confirm | `page_home.rs:195, 273` |
| `install_screen_state.destination` | entry.destination_folder | `reinstall_route.rs:15` |
| `install_screen_state.import_code` | entry.latest_share_code | `reinstall_route.rs:16` |
| `install_screen_state.destination_choice` | `Some(DestChoice::Clear)` | `reinstall_route.rs:18` |
| `install_screen_state.parsed_preview` | parsed from stored code | `reinstall_route.rs:23` |
| `install_screen_state.stage` | `Preview → Downloading → InstallingStub` | `reinstall_route.rs:50`, `page_install.rs:65,76` |
| `wizard_state.step1.{prepare_target_dirs_before_install, backup_targets_before_eet_copy}` | true/false (mirror of Clear) | `reinstall_route.rs:41-46` |
| `pending_reinstall_id` | `Some(id)` then `None` after flip | `reinstall_route.rs:48`, `start_hooks.rs:225` |
| `registry.entries[id].state` | `Installed → InProgress` | `flip_to_in_progress` (`registry_transition.rs:262`) |
| `active_install_modlist_id` | `Some(id)` then `None` on clean exit | `install_modlist_registration.rs:128`, `orchestrator_app.rs:418` |
| `post_install_reset_gate` | `Pending` on clean exit | `orchestrator_app.rs:429` |
| `nav` | `Install` | `reinstall_route.rs:51` |

### File writes

| Path | Trigger |
|---|---|
| `modlists.json` | `flip_to_in_progress` (Install-click); `flip_to_installed` (clean exit) |
| `workspace.json` | reused from prior install (Reinstall does NOT re-create it; but `populate_wizard_state_from_workspace` is never called because the Install-paste flow doesn't open the Workspace) |
| `<dest>` | wiped at arm by `prepare_destination(_, Clear)` |
| `<dest>/modlist-import-code.txt` | install-start: bit-flipped to false; clean-exit: bit-flipped to true with archive_meta |
| `<dest>/weidu_log_source/{bgee,bg2ee}/weidu.log` | `import_modlist_share_code` (BIO) at arm |
| `<dest>/Mods/<MOD>/...` | extract pipeline |
| `<dest>/weidu_component_logs/...` | WeiDU during install |
| `<mods-archive>/...` | content-addressed staging |
| `bio_settings.json` | sanitized debounced cycle |

### Auto-build flag flow

Same as Workflow 1 — `arm_auto_build` arms the flags; `advance_pending_saved_log_flow` runs the scan + apply + preview; `download_gate_open` lets the streamer kick; extract → re-arm → apply → `start_auto_build_install` → install starts.

### Workspace vs Install lifecycle

- `loaded_workspace_id` stays None (Install screen, not Workspace).
- `pending_reinstall_id` is the workflow-defining marker; cleared at Install click by `reinstall_flip_at_install_click`.
- `active_install_modlist_id` is set at install-start by `register_and_write_install_start_artifacts`; cleared at clean exit by `maybe_flip_to_installed_on_clean_exit`.
- `clear_pending_reinstall_on_nav_away_from_install` — if user navs away from Install before clicking Install, this clears `pending_reinstall_id`, leaving the modlist Installed (matching SPEC §3.1 — cancelling the preview leaves the modlist Installed).
- Post-install: `post_install_reset_gate = Pending` → next nav-away or re-enter-Install fires `reset_completed_install_runtime`.

---

## Workflow 5 — Continue partial install — **NOT FULLY WIRED**

> **Risks in this workflow:** R6 🟠 (entire workflow is dead UI) — see callout below.

The SPEC §4.1 calls for a "Continue partial installation" third option on the `DestinationNotEmptyWarning` that lets users resume an aborted install. The codebase has **partial scaffolding** for this but it does NOT reach the workspace or registry.

> ### ⚠️ R6 — Continue partial install is dead UI 🟠 (Spec-only)
>
> Clicking `Continue Install →` from the Paste stage *(`stage_paste.rs:112-116`)* returns `PasteOutcome::Advance(InstallStage::InstallingStub)` — jumps DIRECTLY to `InstallingStub`, skipping `Preview` AND `Downloading` AND every `_once` arm function. Consequence: no `arm_pipeline_once`, no `register_and_write_install_start_artifacts`, no `prepare_install_dirs_and_maybe_import`, no `apply_flags`, no `active_install_modlist_id`. The Step-5 panel renders inside `InstallingStub` *(`stage_installing.rs:88-101`)* with whatever `wizard_state.step1` happens to hold in memory — likely the previous install's per-install dirs OR the sanitizer-restored globals. Clicking Step-5's own Install button would dispatch `Step5Action::StartInstall` *(`stage_installing.rs:103-117`)* → `start_install_requested = true` → BIO runs against whatever state is there. The expected `ContinuePartialInstall` flag set (skip_installed=true, check_last_installed=true) is NEVER applied. **Recommend either gating the UI behind a dev flag or hiding the Continue button until SPEC §4.1's wiring lands.**

### Evidence of partial wiring

1. **`DestChoice::Continue` variant exists** *(`state_install.rs:21`)* — and `to_flags` maps it to `prepare_target_dirs_before_install=false, backup_targets_before_eet_copy=false, skip_installed=true, check_last_installed=true` (SPEC §13.12 #1).
2. **`destination_not_empty::render`** *(`src/ui/install/destination_not_empty.rs`)* renders the third button when `show_continue = true`.
3. **`stage_paste::render`** *(`stage_paste.rs:60`)* passes `true` for `show_continue` so the Continue button appears.
4. **`InstallScreenState::is_partial`** *(`state_install.rs:244-247`)* returns true when `destination_choice == Some(DestChoice::Continue)`.
5. **Paste stage UI** *(`stage_paste.rs:67-80`)* hides the import-code box when `is_partial == true`; renders `partial_info_box` ("Existing mod files detected at {dest}…") instead.
6. **Paste footer primary becomes `Continue Install →`** *(`stage_paste.rs:95-117`)* and `PasteOutcome::Advance(InstallStage::InstallingStub)` jumps **directly to `InstallingStub`**, skipping Preview AND Downloading entirely.
7. **`InstallWorkflow::ContinuePartialInstall` variant exists** *(`flag_policies.rs:13`)*, used by `LivePipelineInputs::from` *(`stage_downloading.rs:550-554`)* when `state.is_partial()`.

### Evidence of MISSING wiring

1. **Stage transition skips Preview AND Downloading** *(`stage_paste.rs:112-116`)* — jumps `Paste → InstallingStub` directly. So the entire `arm_pipeline_once → stage_and_kick_archive_skip_once → kick_streaming_downloader_once → verify_downloaded_archives_once → ingest_downloaded_archives_once` chain **is never reached** for Continue.
2. **`InstallingStub` is the bare BIO Step-5 render** *(`stage_installing.rs:88-101`)*. It has no special "Continue" branch; it just embeds Step-5 with `register_and_write_install_start_artifacts` happening earlier — but actually since we never went through Downloading, `register_and_write_install_start_artifacts` is **never called** for Continue.
3. **No registry entry registration for Continue** — the only places that register an install-start modlist are `register_and_write_install_start_artifacts` (Install-paste / Reinstall on Downloading) and `mint_and_arm` (Create-Fork). Continue jumps past `Downloading`, so it never hits these.
4. **`active_install_modlist_id` is never set on Continue** — same reason.
5. **`apply_flags` for `ContinuePartialInstall` is never called** — that requires `start_hooks::on_install_start` (workspace path) or routing through Downloading (which calls `prepare_install_dirs_and_maybe_import`). The Continue path hits neither.
6. **No registry write happens** — the modlist either doesn't exist in the registry (fresh attempt) or stays at whatever state it had (mid-install) without a state flip.
7. **`pipeline_arm_error` cannot surface** — the arm never runs.

### Conclusion

The Continue-partial-install UI is **rendered but not wired**. Clicking `Continue Install →` from Paste jumps to `InstallingStub`, which renders Step-5 — but `wizard_state.step1` has no per-install dirs derived, no flags applied, no registry entry minted, and no `active_install_modlist_id` set. Clicking the actual Step-5 `Install` button inside `InstallingStub` (via the BIO `page_step5::render` embedded panel) would dispatch `Step5Action::StartInstall` *(`stage_installing.rs:103-117`)*, which sets `start_install_requested = true` directly — but the install would fail or run against whatever step1 state happens to be in memory, with no `-s`/`-c` flag set, no per-install dirs, no registry tracking.

This appears to be **deferred functionality**, not a bug — the SPEC explicitly calls Continue out as a path, but the implementation hooks needed to wire it (registration of an in-progress modlist for the partial-install destination, application of `ContinuePartialInstall` flags, derivation of per-install dirs, hook into the existing install runner) are not present.

Recommend documenting as "Continue is not currently wired; clicking Continue → Continue Install jumps to a bare Step-5 console that will fail without manual flag setup."

---

## Workflow 6 — Delete modlist

> **Risks in this workflow:** R2 🟠 (orphaned `%APPDATA%\bio\modlists\<id>\` + in-memory HashMaps), R7 🔴 (no symlink check in safety guard — Linux data loss risk).

### User journey *(SPEC §3.2 Delete semantics)*

1. User clicks Kebab → `Delete` on a modlist card (in-progress OR installed).
2. Danger `ConfirmDialog` (`Delete "<name>"?`) opens.
3. User clicks `Delete` → registry entry removed; install folder recursively deleted (guard-checked); success toast.

### Code path

`page_home::render_delete_confirm` *(`page_home.rs:216-256`)*:
- On `Confirmed`: clones name, calls `operations::delete_modlist(&id, &registry_store, &mut registry)`.
- On success: `persistence_cycle.last_saved_registry = registry.clone()`; success toast.

`operations::delete_modlist` *(`operations.rs:39-64`)*:
- Finds entry by id; removes from `registry.entries`.
- Clones the destination folder path.
- Calls `is_safe_install_folder(&dest)` *(`operations.rs:19-37`)* — the safety guard:
  - Trim empty → false.
  - Not absolute → false (rejects `modlists/foo`, `./foo`, `foo`).
  - No parent (filesystem root like `C:\` or `/`) → false.
  - Not a directory on disk → false.
  - Otherwise → true.
- If guard passes: `fs::remove_dir_all(dest.trim())` — recursive deletion.
  - Success → `DeleteOutcome::EntryAndFolderRemoved`.
  - Error → `DeleteOutcome::EntryRemovedFolderError(err.to_string())`.
- If guard fails: `DeleteOutcome::EntryRemovedFolderSkipped`.
- `store.save(&registry)` — atomic write of the modlists.json (the entry is now gone from memory; the persisted file matches).

### State mutations

| Field | Value | Where |
|---|---|---|
| `home_screen_state.delete_target` | `Some(id)` then `None` | `page_home.rs:192, 231` |
| `registry.entries` | entry removed | `operations.rs:50` |
| `persistence_cycle.last_saved_registry` | refreshed | `page_home.rs:239-240` |
| `home_screen_state.toast` | success or error message | `page_home.rs:241, 245` |
| `<dest>` on disk | recursively deleted if guard passes | `operations.rs:53` |

### File writes

- **`modlists.json`** — atomic write via `store.save(&registry)` *(`operations.rs:61`)* removes the entry from disk.
- **`<dest>` recursive delete** — entire destination tree gone if guard passes.

### Workspace.json + per-modlist directory cleanup

**`operations::delete_modlist` does NOT remove the `%APPDATA%\bio\modlists\<id>\workspace.json` file.** Inspection of the code path:
- `operations.rs:50` removes from in-memory registry.
- `operations.rs:52-59` deletes the on-disk `<dest>` folder (the user's install folder) but never touches `%APPDATA%\bio\modlists\<id>\`.
- The orchestrator's `workspace_state` HashMap *(`orchestrator_app.rs:118`)* and `workspace_stores` HashMap *(`:119`)* still contain entries keyed by the deleted modlist's id. There's no code path that removes those entries.

This is a **garbage-collection gap**: the per-modlist workspace folder under `%APPDATA%\bio\modlists\<id>\` survives the modlist's deletion. The user can delete and re-add modlists indefinitely; orphaned `workspace.json` files accumulate. The `workspace_state` and `workspace_stores` HashMaps similarly accumulate stale entries during a session.

> ### ⚠️ R2 — `delete_modlist` orphans per-modlist workspace folder + in-memory state 🟠
>
> `operations::delete_modlist` *(`operations.rs:39-64`)* does three things: removes the entry from `registry.entries` (in-memory), `fs::remove_dir_all`s the install destination (if guard passes), and atomic-saves the new `modlists.json`. It does NOT:
>
> - Remove `%APPDATA%\bio\modlists\<id>\workspace.json` (and any other future per-modlist files in that directory).
> - Remove the corresponding entry from `orchestrator.workspace_state` *(`orchestrator_app.rs:118`)* — a `HashMap<String, ModlistWorkspaceState>`.
> - Remove the corresponding entry from `orchestrator.workspace_stores` *(`orchestrator_app.rs:119`)* — a `HashMap<String, WorkspaceStore>`.
>
> Result: orphaned files accumulate across sessions; orphaned HashMap entries accumulate across the single session. The orphaned `workspace.json` files are small (~hundreds of bytes each), but they are also a privacy/forensic leak — a deleted modlist's order, expand state, fork lineage, scratch path, and prompt answers survive on disk indefinitely. Fix shape: after `store.save(&registry)`, fire `fs::remove_dir_all("%APPDATA%/bio/modlists/<id>")` (with its own safety guard since this path is BIO-owned, not user-typed) and `orchestrator.workspace_state.remove(id)` + `orchestrator.workspace_stores.remove(id)`.

### Safety guard analysis

The `is_safe_install_folder` *(`operations.rs:19-37`)* check is conservative:
- Empty strings: refused.
- Relative paths: refused (prevents accidental project-root deletion).
- Filesystem roots (`C:\`, `/`, `\\server\share`): refused.
- Non-existent paths: refused (handled at fs::remove_dir_all level too, but the explicit check prevents partial deletes if the path is suspicious).
- Files (not dirs): refused.

`fs::remove_dir_all` follows symlinks on Linux, doesn't on Windows — both behaviors documented but worth noting since neither is guarded against in this code path. A symlink-as-destination would, on Linux, recursively delete the target. There's no `path.is_symlink()` check.

> ### ⚠️ R7 — Missing symlink check in `is_safe_install_folder` 🔴 (Linux)
>
> `is_safe_install_folder` *(`operations.rs:19-37`)* checks: trim-empty, is-absolute, has-parent, is-dir. It does NOT check `path.is_symlink()`. On Linux/macOS, if a user typed a destination like `~/games/EET → /important/data` (symlink) into the destination FolderInput, the registry stores the symlink path verbatim, and `delete_modlist` calls `fs::remove_dir_all` on it. On Linux, `std::fs::remove_dir_all` follows the symlink and recursively deletes the target — destroying `/important/data` even though the user only wanted to delete their modlist. Windows is safer (Windows `remove_dir_all` does not follow directory junctions/symlinks by default), but the codebase is cross-platform.
>
> Fix shape: add `if path.symlink_metadata()?.file_type().is_symlink() { return false; }` early in `is_safe_install_folder`. Alternative: use `fs::symlink_metadata` to detect, then either refuse or `fs::remove_file` (unlink the symlink without following).

---

## Cross-workflow concerns

### Persistence boundary (SPEC §13.12a `sanitize_step1_for_settings_persistence`)

The sanitizer *(`settings_sanitizer.rs:7-37`)* scrubs **11 per-install string fields** (mods_folder, weidu_log_folder, bgee_log_folder, bg2ee_log_folder, eet_bgee_log_folder, eet_bg2ee_log_folder, bgee_log_file, bg2ee_log_file, eet_pre_dir, eet_new_dir, generate_directory) + **`weidu_log_mode`** (which embeds the per-install log folder token) + **5 per-install booleans** (weidu_log_log_component, have_weidu_logs, new_pre_eet_dir_enabled, new_eet_dir_enabled, generate_directory_enabled). Total = 17 fields restored to global Settings values.

**Untouched globals:** `mods_archive_folder`, `mods_backup_folder`, `bgee_game_folder`, `bg2ee_game_folder`, `iwdee_game_folder`, `eet_bgee_game_folder`, `eet_bg2ee_game_folder`, `weidu_binary`, `mod_installer_binary`, `download`, `download_archive`, etc.

**Call sites:**
1. **`bio_settings_snapshot`** *(`orchestrator_app.rs:1018-1032`)* — runs against a clone of `wizard_state.step1` before every debounced `bio_settings.json` write. Protects across-session leakage.
2. **`reset_completed_install_runtime`** *(`page_router.rs:289-292`)* — runs against the LIVE `wizard_state.step1` after a clean Install-paste/Reinstall completion. Protects in-session leakage (Settings → Paths tab reads from `wizard_state.step1` live, not from `bio_settings.json`).

### State that survives session restart

| File | Survives | Cleared by |
|---|---|---|
| `modlists.json` | All registered modlists | `delete_modlist`, corrupt-file rename |
| `modlists/<id>/workspace.json` | Per-modlist order/expand/prompt state, `pending_destination_prep`, `scratch_mods_folder`, `dev_scanned_mods_folder` | Never explicitly cleared on modlist delete (stale entries accumulate) |
| `bio_settings.json` | Global paths/tools/advanced flags (sanitized — no per-install pollution) | Settings → Paths edits |
| `bio_redesign_settings.json` | Theme, user name, language, validate-on-startup, diagnostic_mode | Settings → General edits; corrupt-file rename + default |
| `<dest>/modlist-import-code.txt` | Always (since SPEC §13.13) | Manual user delete |
| `<mods-archive>/...` | All downloaded archives, content-addressed | Manual user delete |
| `<dest>/_bio_backup_*` | Backup snapshots from `DestChoice::Backup` | Manual user delete |

### Conflicting paths / known regressions

**Run 1** `2c95df9` *(branch HEAD~4)*: fork-route un-blocked — `clear_pending_reinstall_on_nav_away_from_install` carve-out for `Create + ForkDownload` *(page_router.rs:204-211)*. Without this the fork pipeline's `active_install_modlist_id` got cleared mid-pipeline.

**Run 2** `b284525`: Apply-imported-WeiDU-log selection on Workspace landing + suppress auto-opened Update Check popup. The `wizard_state.step2.update_selected_popup_open` got auto-opened during the fork pipeline because the `preview_update_selected` flow populates it; Run 5's `stage_fork_download.rs:91-99` extension clears all the popup-open flags.

**Run 3** `b67695b`: rescan-wipes-step2 via new `PostInstallResetGate` enum + Step 4 → Step 5 auto-save weidu logs (`write_step4_weidu_logs_unconditional`). Replaced an `has_route_context()` heuristic that mis-fired for fork-then-modify workspaces with the explicit `PostInstallResetGate::Pending` flag.

**Run 4** `57bc5c4`: suppress an auto-build install regression — `arm_auto_build` leaves the auto-build flags true through the fork pipeline; `advance_pending_saved_log_flow` would fire `start_auto_build_install` once apply settled, routing the user to install instead of the workspace. Fix at `stage_fork_download.rs:65-79`.

**Run 5** `7386195`: two cascading bugs — (i) workspace install's success banner + Step-5 console were wiped right after install completion because Run 3's `PostInstallResetGate` flag fired for both Install-paste AND Workspace installs. Fix: gate the flag-set on `!from_workspace` in `maybe_flip_to_installed_on_clean_exit` *(`orchestrator_app.rs:428-430`)*. (ii) `write_step4_weidu_logs_unconditional` writes the per-install weidu.log unconditionally so fork edits reach disk even when `install_mode == install_exactly_from_weidu_logs` (BIO's `auto_save_step4_weidu_logs` early-returns for that mode).

**Conflict surface that remains (observed during this trace):**
1. **Double `prepare_destination` on Fork → Workspace → Step-5 Install with non-None destination_choice** (see Open Question 1). The choice is consumed twice: once at fork arm via `install_screen_state.destination_choice`, once at Step-5 install via `workspace_state.pending_destination_prep`.
2. **Orphaned per-modlist workspace folders** survive `delete_modlist`. Workflow 6 doesn't clean up `%APPDATA%\bio\modlists\<id>\` or remove the in-memory `workspace_state`/`workspace_stores` HashMap entries.
3. **Continue partial install jumps Downloading entirely**. No registration, no flag application, no per-install dir derivation. Workflow 5 above.
4. **Reinstall variant ambiguity at install-start** (Open Question 2). `reinstall_flip_at_install_click` consumes the `pending_reinstall_id` BEFORE `register_and_write_install_start_artifacts` runs, so the latter sees variant `Install` not `Reinstall`. The matrix at `start_hooks.rs:300-330` treats this correctly (`Install` writes the import code, just like `Reinstall` would), but the trace through the code is non-obvious.
5. **`reset_install_screen_to_paste` called at fork-extract-complete route-to-workspace clears `active_install_modlist_id`** *(`orchestrator_app.rs:1155`)* — but the fork's clean-exit flip then reads `active_install_modlist_id.or_else(|| ...)` *(`orchestrator_app.rs:375`)*. Since `loaded_workspace_id = Some(id)` after the route, the flip uses `loaded_workspace_id` and the clear is harmless. But if the fork install fails or is cancelled and the user navs away, there's no `active_install_modlist_id` to clear. This is fine — the entry stays InProgress in the registry.

---

## Open questions

1. **Double destination-prep on fork install with non-None choice** *(stage_downloading.rs:579-593 then page_workspace_step5.rs:178)*. When a user picks `Backup` on Create's Setup Box for a fork:
   - At fork arm, `prepare_destination(_, Backup)` runs (fresh destination already empty → no-op on AlreadyEmpty; OR moves prior contents to `_bio_backup_..._<ts1>`).
   - Pipeline downloads + extracts into the destination, populating `<dest>/mods/`, weidu logs, etc.
   - User reviews workspace, clicks Step-5 Install.
   - `consume_pending_destination_prep` reads `workspace.json::pending_destination_prep = Some(Backup)` and **runs `prepare_destination` AGAIN** *(`page_workspace_step5.rs:178`)*. On Backup this MOVES the now-populated `<dest>` (containing extracted mods) to `_bio_backup_..._<ts2>` and creates a fresh empty `<dest>`.
   - Result: the user's freshly-extracted mods are now in a backup snapshot, and the actual install starts against an empty destination. The install runner then needs to re-download — but the `pending_saved_log_apply` machinery + scan-cache likely doesn't re-arm here because the workspace path treats this as a normal Step-5 install.
   - **Is this intended?** If yes, the double-prep wastes the fork download. If no, `consume_pending_destination_prep` should be a no-op when the fork pipeline already consumed the choice. Possible fix shape: `mint_and_arm` should NOT write `pending_destination_prep` to `workspace.json` at all, since `arm_pipeline_once` consumes the choice immediately.

2. **Reinstall variant transition** — by the time `register_and_write_install_start_artifacts` runs (inside `arm_pipeline_once`), `pending_reinstall_id` has been cleared by `reinstall_flip_at_install_click` (which fires earlier at `PreviewOutcome::Advance`). So the variant computed at *(`install_modlist_registration.rs:95`)* is `Install`, not `Reinstall`. Is this intended? The behavior is correct (both variants write the import code), but anyone tracing this would assume `variant == Reinstall` at install-start, which is wrong. Document or rename for clarity.

3. **Continue partial install** — the UI is rendered but the wiring doesn't exist. Should this be removed from the UI (and the Continue branch in `DestChoice`/`InstallWorkflow`) until it's implemented, or kept as a UX placeholder?

4. **Orphaned workspace.json on delete** — `delete_modlist` doesn't touch `%APPDATA%\bio\modlists\<id>\`. Should it? The in-memory `workspace_state` and `workspace_stores` HashMaps also retain entries for deleted modlists during a session. Cleanup gap.

5. **`reset_install_screen_to_paste`'s `clear_preview` may strip a still-needed `parsed_preview`** — On the fork-route-to-workspace transition *(`page_create.rs:384`)*, `reset_install_screen_to_paste` calls `clear_preview()` which sets `parsed_preview = None`. The workspace doesn't read `install_screen_state.parsed_preview` (it reads `registry.find(id).forked_from` via `fork_meta_from_entry` *(`page_router.rs:345-357`)*), so this is fine in practice. But documenting that the workspace's fork info is sourced from the registry entry, not from `install_screen_state`, is worth pinning.

6. **`flush_workspace_on_nav_away` blocks on `restore_pending(&step2)`** *(`page_router.rs:149-152`)* — when a rescan or cold-resume is in flight, the nav-away flush is skipped to avoid clobbering an empty `wizard_state` over the user's persisted order. The trade-off: if the user navs away during a rescan, their pre-rescan edits are NOT persisted. Acceptable per the comment about Fix-Run-4, but the user-visible behavior (a manual nav-away mid-scan loses local-only edits) deserves a callout.

7. **`active_install_modlist_id` lifecycle in fork path** — set by `mint_and_arm`, cleared by `reset_install_screen_to_paste` at route-to-workspace. The `page_router::clear_pending_reinstall_on_nav_away_from_install` carve-out for `Create + ForkDownload` preserves it during the download stage. But once the user is on Workspace, the clean-exit flip relies on `loaded_workspace_id` (not `active_install_modlist_id`). The dual-tracking is correct but fragile — any change to the route-to-workspace order could break the lookup.

8. **`pending_saved_log_download` is deliberately NOT armed by `arm_auto_build`** *(`auto_build_driver.rs:65-79`, comment at `:75`)*. This is the redesign's signal that the orchestrator owns the parallel `stream_downloader`. If BIO ever needs to fall back to its serial downloader (e.g., a backout commit), the flag would need to be re-armed. Worth a comment in `arm_auto_build`.

9. **`fork_extract_complete`'s `archives_observed > 0 || empty assets` condition** *(`stage_fork_download.rs:147`)* — what if a fork has assets but all of them got `Skipped` (already-present hashes)? Then `update_selected_extracted_sources` would be empty (no extraction happened) AND `skip_indices.len() > 0`. The OR-clause covers that — `archives_observed = 0 + skipped > 0 > 0` is true. But if the fork has assets, all fail to hash-skip AND all fail to download (worst-case network outage), then `archives_observed == 0 && update_selected_update_assets != []` → never reaches completion. The pipeline would hang on the Downloading screen. The non-masking `pipeline_arm_error` banner doesn't cover post-arm failures; only `archive_skip` worker disconnects bypass the gate by setting `archive_skip_completed = true`. Worth verifying this hangs vs surfaces.

10. **Workspace Step-5 from-scratch install — no share-code-consuming workflow even when the user manually imports archives** — `page_workspace_step5.rs:122-128` picks `FreshCreate` when `forked_from.is_empty()`, regardless of whether the user has populated `step3` with mods that resolve to downloads. The `apply_flags(FreshCreate, settings)` then leaves `download` following the Settings → Advanced `download` flag (default true). If the user's modlist references download-requiring mods, this works; if Settings download is off and the user has unstaged mods, the install fails silently. Worth flagging.

---

## Recommended fix order

Ranking per agent's view, based on severity × likelihood × user-visible cost:

1. ~~**R1 🔴 — Fork double `prepare_destination` on Clear/Backup**~~ ✅ **Closed by PR #18 (2026-05-27).** Fix shape #2 from this entry was taken: `mint_and_arm` no longer persists the choice into `workspace.json::pending_destination_prep` *(`fork_pipeline_arm.rs:80-83`)*; the regression test `minted_workspace_never_persists_destination_prep_regardless_of_user_choice` pins the single-fire invariant *(`fork_pipeline_arm.rs:330-363`)*.
2. **R7 🔴 — Symlink check missing in `is_safe_install_folder` (Linux)**. Now the highest-priority open data-loss risk. Niche (Linux + symlinked destination) but the impact is catastrophic — silent delete of arbitrary user files. One-line fix: `if path.symlink_metadata()?.file_type().is_symlink() { return false; }`.
3. **R6 🟠 — Continue partial install is dead UI**. Either remove the button + the `DestChoice::Continue` variant + the `ContinuePartialInstall` flag set, OR finish the wiring per SPEC §4.1. Either resolves the trap.
4. **R2 🟠 — `delete_modlist` orphans per-modlist files + HashMaps**. Adds a couple of in-memory `HashMap::remove` calls + an `fs::remove_dir_all` of `%APPDATA%\bio\modlists\<id>` (BIO-controlled path, no user guard needed). Privacy-relevant since forked-from lineage survives on disk after delete.
5. **R3 🟡 — Reinstall variant naming**. No behavior change needed but the trace is misleading. Either rename `pending_reinstall_id`'s consumption to make the lifecycle explicit, or carry the variant through `wizard_state` so `register_and_write_install_start_artifacts` can read it.
6. **R5 🟡 — `from_workspace` overloaded**. Refactor target after the data-loss fixes land. Each of the five decisions deserves its own named state (e.g., `should_preserve_share_code`, `should_arm_post_install_reset`, etc.).
7. **R11 🟡 — `FreshCreate` workflow silently disables download**. Add a pre-install check that warns if `step3` references downloads and `download` is off.
8. **R10 🟡 — `pending_saved_log_download` silent contract**. Add an assertion or a comment-as-contract that the orchestrator owns the parallel downloader.
9. **R4 🟡 — Auto-build suppression race (Mitigated)**. Already works via Runs 4 and 5. Lower priority; pin the frame-ordering invariant in a comment or test.
10. **R8 🟡 — Fork pipeline hang on full network outage**. Add a timeout/error-surface for the all-assets-failed case.
11. **R9 🟠 — `flush_workspace_on_nav_away` discards mid-rescan edits**. Acceptable per Fix-Run-4 commentary; surface a banner ("edits not saved during scan") if needed.
12. **R12 🟡 — `parsed_preview` clear at fork-route-to-workspace**. Harmless today; document the lineage-source invariant.
13. **R13 🟡 — Fork import code is re-packed, not parent-verbatim**. By design; just needs better documentation.
