# Infinity Orchestrator — Handoff

A working snapshot of where the redesign sits today, what's left, and the context needed to finish.

The project is the redesign of the existing `bio` Rust crate (Born2BSalty's Infinity Orchestrator) into a multi-modlist workspace app. Built on `eframe` / `egui`. The redesign preserves BIO's deterministic install pipeline and adds a new visual language + a modlist registry on top.

---

## Status at handoff

| Phase | Subject | Status |
|---|---|---|
| 1 | Theme tokens, fonts, shell modules, new binary entry | ✓ done, builds clean |
| 2 | Navigation + routing (`OrchestratorApp`, shell chrome, left rail, page router, stubs) | ✓ done, builds clean |
| 3 | Modlist registry + per-modlist workspace state files | ✓ done, builds clean |
| 4 | Settings screen (5 sub-tabs) + per-edit debounced path validation | ✓ done, builds clean |
| 5 | Home + Install Modlist (paste / preview / download stages) | ✓ done |
| 6 | Create screen + Workspace shell (Steps 2–4) | ✓ done |
| 7 | Step 5 install runtime + Reinstall + install concurrency + rail-nav lock | ✓ **COMPLETE** via PR #5 + PR #6. Item #4 Create-fork post-route UX **arc-closing PR open for `xgatt/create-then-modify-fix`** — Runs 1-6 landed; both 🔴 data-loss bugs closed (R1 double `prepare_destination` + wrong-write-path `sync_paths_from_settings` clobber). Remaining deferred items: **R7** Linux symlink follow on `delete_modlist`; **R6** Continue-partial-install dead UI; **Gap #4** silent skip in `build_extract_jobs`. Full risk map in `.claude/INSTALL_WORKFLOWS_TRACE.md` (13 risks numbered R1-R13). |
| 8 | Popup reskins + state-aware theme reads across BIO surfaces + polish | not started |

After phases 5–8 land, the binary is feature-complete per the SPEC (modulo the deferred items in Appendix B and the known caveats below).

---

## Recent ships + process / tooling changes (2026-05-20 → 2026-05-22)

**Code:**
- **Carve-out #8 — Step-2 SUBCOMPONENT umbrella render restoration** (branch `fix/step2-umbrella-grouping`, 2026-05-22) — single-function rewrite of `render_weidu_group` in `src/ui/step2/tree/tree_components_step2.rs` to restore v0.1.0-beta.19 (`18d2f43`) behavior that was inadvertently regressed by `7247da1` "cleanup clippy" (Born2BSalty, 2026-05-20). The regression had the WeiDU branch route through the shared `render_component_group` helper, which only renders flat rows and never reaches the nested-umbrella detection helpers (`render_collapsible_component_group`, `render_subcomponent_group`); components with both a WeiDU GROUP and a SUBCOMPONENT family (the EEUITweaks shape) lost their nested collapsing header. Fix: inline the egui collapsing-header machinery (build `header_id`, load `CollapsingState`, handle `jump_to_selected_requested`, `.show_header().body()`) inside `render_weidu_group` and call `render_component_rows_range` (the umbrella-detecting helper) inside `.body(...)`. `render_component_group` is preserved for the collapsible + subcomponent paths (terminal — bodies don't recurse into nested groups). New `tests` module pins the structural invariant via `egui::Context::default()` + `set_everything_is_visible(true)`: a fixture with two components sharing both `weidu_group = Some("Engine")` and `collapsible_group = Some("Family")` must store a `CollapsingState` at the nested `step2_collapsible_group` id after render — the test FAILS on the regressed code (verified) and passes on the fix. ZERO build-side change; the `Step2ComponentState.collapsible_group` field population in `worker_build_states_groups_collapsible.rs` was always correct. **Carve-out count is now 8.** `cargo test --lib` **503/0** (+1 from 502 baseline); both binaries no-op rebuild; whole-codebase clippy pedantic+nursery gate exits with 0 warnings.
- **DL Fix-Set v2** (`25184fa`) — 5 user-approved fixes from the live re-test of v1: pure-count download bar fallback, per-asset push to downloaded/failed vectors, `extract_intercept` snapshot infrastructure, `apply_saved_weidu_log_selection` + `sync_step3_from_step2` on the paste-path before flip, `archive_skip` keeps assets in list + pre-populates `downloaded_sources`. ZERO BIO, six carve-outs.
- **DL Fix-Set v3** (`7962b3f`) — parallel extract coordinator (new `install_runtime::extract_parallel`, pool 10) + async hashing (new `install_runtime::archive_skip_async`, pool 10) + visual collapse to "✓ downloaded" + Hashing phase + row sort + "Preparing to install" overlay + **7th BIO carve-out** (narrow visibility-only, 6 edits across 3 files: `extract_one_archive`, `Step2UpdateExtractJob` + fields, `build_extract_jobs` all `pub(super)` → `pub(crate)` + the parent's `mod archive;` / `mod plan;` → `pub(crate) mod`). The carve-out count is now **7**. The v2 `extract_intercept` module was REPLACED by `extract_parallel`. Both v2 + v3 pushed.
- **DL Fix-Set v4 + Step-5 bundled fix-set** (branch `fix/install-screen-bundled-fixes`, PR pending) — 7 bugs in one plan-implementer run, ZERO BIO source touched, 7 carve-outs stand. **Cancel handler rewrite**: new shared `OrchestratorApp::reset_install_screen_to_paste` drops the 3 pipeline receivers, clears the 5 BIO auto-build/saved-log latches + `update_selected_download_running`/`_extract_running`, calls `clear_preview`, wipes `pending_reinstall_id`/`active_install_modlist_id`, blanks the shared `Arc<Mutex<_>>` hash + extract snapshots, drains the Step-5 terminal buffer + console-view (the helper closes Bug 1 Cancel AND Step-5 #2/#3 nav-away-after-complete in one path; `page_router::reset_install_screen_on_nav_away_after_complete` invokes it on the install-completion edge). **Archive dedupe**: new private `link_or_copy` in `archive_store.rs` (hardlink first, copy fallback on cross-volume) used at both `stage_known_archives` and `ingest_downloaded_archives` — one NTFS inode, two names, no full duplicate. **Hashing classification**: new `hashed_indices: HashSet<usize>` on `InstallScreenState`, populated by `drain_archive_skip_events` per `AssetHashed`; `from_wizard_state_full` gains an optional `hashed_indices` parameter and returns `Hashing` for unhashed indices while the pass is alive (caller passes `None` after `archive_skip_completed`, falling back to BIO's Queued/Downloading/Extracting/Staged grouping). **Snapshot reset**: `clear_preview` now resets `download_progress.hash_progress` AND `extract_progress` so a fresh install can't inherit the previous one's `(N, N)`. **Bug 4 pin**: `drain_stream_download::AssetDone` calls `set_asset_bytes(i, final_bytes, Some(final_bytes))` independent of Content-Length, so the row bar fills to 100% before the next frame's `from_wizard_state_full` flips status to ✓. **Step-5 console clip**: `page_workspace_step5` now wraps the BIO `page_step5::render` call in a `clipped_pane` (child Ui with `max_rect` + `set_clip_rect`); over-wide paint from BIO's `TextWrapMode::Extend` is dropped at the central column's right edge — no bleed into shell chrome / rail / statusbar. **Bug 5 (5-parallel-downloads-cap)**: user accepted GitHub's per-IP host limit; `POOL_SIZE = 10` unchanged. `cargo test --lib` 466/0 (+10 from 456 baseline); both binaries no-op rebuild; whole-codebase clippy pedantic+nursery gate exits with 0 warnings; DATA-LOSS sentinel byte-identical (21 files); 2 runtime trace artifacts + 15 render-gate PNGs (4 v4 scenes × 3 widths + 1 Step-5 console-clip × 3 widths).
- **Create/Fork wiring + destination-prep widening** (branch `fix/install-screen-bundled-fixes`, second commit on PR #5) — closes the Create/Fork fix-set + the destination-prep scope widening as one bundle, ZERO BIO source touched, 7 carve-outs stand. **Fork pipeline wiring**: new `install_runtime::fork_pipeline_arm::mint_and_arm` is the `ForkBeginImport` entry — mints the forked entry via `create_forked_modlist` reading `state.modlist_name` (with `<parent> (fork)` fallback) and `state.destination` (with `default_destination` fallback), persists `workspace.json`, populates `install_screen_state.destination` / `import_code` / `destination_choice`, and lets the shared `stage_downloading::*_once` arming sequence run against the fork against the minted modlist's destination. New `stage_fork_download::render_live` drives the live pipeline (skip pool → streamer → verify → ingest) and disarms BIO's auto-build latches at the **extract-complete** seam so the user routes to Workspace Step 2 instead of being auto-routed to Step 5 install. The old inline `fork_import` retires; `fork_extract_complete_route_to_workspace` is the new route handler. Cancel uses the shared `reset_install_screen_to_paste`. **Destination-prep widening**: new `install_runtime::destination_prep::prepare_destination` operates on the entire destination directory — Clear `fs::remove_dir_all` + `fs::create_dir_all` (covers all 4 per-install categories: Mods / weidu_component_logs / WeiDU-log sources / game install folders, not just BIO's EET subdirs), Backup `fs::rename` aside as `_bio_backup_<name>_<ts>`, Continue / None / empty / missing are no-ops. Status-quo confirmation level — the user's explicit click on the destination-not-empty warning is treated as sufficient consent (no extra dialog, no Recycle Bin route). Hooked at four arm points: (1) Install-paste — `arm_pipeline_once` calls `prepare_destination` before the pipeline arms; (2) Create-fork — `mint_and_arm` populates `install_screen_state.destination_choice` so the same `arm_pipeline_once` consumes it; (3) Reinstall — `reinstall_route` sets `DestChoice::Clear` explicitly so the shared arm path empties the destination before re-install; (4) Workspace Step-5 fresh-Install — new `pending_destination_prep: Option<DestChoice>` field on `ModlistWorkspaceState` (schema-additive, `skip_serializing_if = "Option::is_none"` preserves byte-identity for default workspaces) persists the choice from Create-time and `consume_pending_destination_prep` applies it at install-arm then clears the field. BIO's `prepare_target_dirs_before_install` becomes a defensive no-op after orchestrator's wider prep wiped the subdirs. SPEC §5.3 + §13.12 #6 doc-synced in the same commit pinning the routing semantics. `cargo test --lib` **489/0** (+23 from 466 baseline); both binaries no-op rebuild; whole-codebase clippy pedantic+nursery gate exits with 0 warnings; DATA-LOSS sentinel byte-identical (21 files, byte-checked before+after); 2 runtime trace artifacts + 12 fork-pipeline render-gate PNGs (4 scenes × 3 widths).
- **Item #5 deeper-fix (Path L)** (2026-05-21, branch `fix/install-screen-bundled-fixes`, fifth commit on PR #5) — closes the in-session Settings → Paths leak that commit `f640596`'s sanitizer left open (the sanitizer prevented cross-session leak via `bio_settings.json`, but the in-session Settings tab read from `wizard_state.step1.mods_folder` LIVE, so the per-install Mods folder kept showing in Settings → Paths "Temp" until the next session restart). Two-part fix, ZERO BIO source touched: (a) drop the "Temp" path-row from `src/ui/settings/tab_paths.rs::render` — per SPEC §13.12a the per-install Mods folder is "derived (never asked)" and shouldn't be a user-editable Settings surface at all; (b) call `install_runtime::settings_sanitizer::sanitize_step1_for_settings_persistence` against the LIVE `wizard_state.step1` (not a clone) at the install-completion edge in `page_router::reset_completed_install_runtime`, so any other UI path that reads `step1` after install ends sees the global Settings values restored (per-install dirs are unwound). The live step1 is only polluted during the install-running window; the moment the install ends (clean exit + nav transition), step1 is restored. Comment on `tab_paths.rs::render` explains the "Temp" omission. Plus a one-char `;` fix in `extract_parallel.rs` for `clippy::semicolon_if_nothing_returned` on the post-cherry-pick rustfmt-expanded `thread::spawn` closure. `cargo test --lib` 501/0 (unchanged from `f640596`); whole-codebase clippy pedantic+nursery gate 0 warnings; both binaries no-op rebuild post-gate. **Open follow-up**: Item #4 (Create-fork extract hangs at 1/2 50%) still unresolved — user's latest reproduction with the post-`f640596` lifecycle INFO logs confirmed `extract_parallel` itself COMPLETES (all workers joined, 2 extracted, 0 failed), so the hang is downstream of extract (likely the `fork_extract_complete` predicate's `archives_ingested` gate never flips true on the soft-fallback unverifiable path, OR `drain_extract_parallel` isn't processing the 2nd AssetDone + Finished events). Next iteration: add INFO logs to `drain_extract_parallel` + `ingest_downloaded_archives_once` + `fork_extract_complete` to pin the exact gate.
- **Post-cherry-pick 4-bug fix-set** (2026-05-21, branch `fix/install-screen-bundled-fixes`, fourth commit on PR #5; the upstream cherry-pick `3164f17` closed the Step-5 console-wrap finding in-tree) — 4 user-surfaced regressions in one plan-implementer run, ZERO BIO source touched, 7 carve-outs stand. **Item #2 hardlink fallback diagnostic visibility**: `archive_store::link_or_copy`'s silent `Err(_) => fs::copy(...)` now surfaces the hardlink failure via `tracing::warn` before falling back so a future cross-filesystem / AV / permissions regression is diagnostic, not invisible (the user's `fsutil hardlink list` verified production dedupe still works — the warn is pure instrumentation, no behavior change on the happy path). **Item #3 workspace nav glyph-font selection**: `workspace_nav_bar::paint_button_text`'s broken `paint.first.len() == 1` heuristic routed both the leading `← Previous` arrow AND the trailing `Next →` arrow to the Latin-subset `poppins_medium` font, dropping them to the missing-glyph `?` fallback (Poppins is 217-glyph; arrows live in `firacode_nerd`'s PUA range). Replaced with an explicit `leading_is_glyph: bool` field on `ButtonTextPaint` + a pure `pick_button_fonts(visuals, leading_is_glyph) -> (FontId, FontId)` helper; two unit tests assert the correct font for both leading and trailing positions. **Item #4 extract_parallel hang diagnostic + count-alignment assertion**: investigated the user's "2-mod fork import hangs at extract 1/2 50% with no Finished event" — both stated hypotheses ruled out (Hypothesis A: `start_parallel_extract` initializes `extract_progress = Some((0, jobs.len()))` so the bar's denominator IS the actual job count, not the pre-skip asset count; Hypothesis B: `fork_extract_complete`'s disarm gates on `!update_selected_extract_running` which only flips at `Finished`, so the disarm is downstream of, not upstream of, Finished). Likely cause is an extract worker hang inside `extract_one_archive` for one specific archive — no direct evidence isolated. Added the brief-authorized `tracing::warn` instrumentation on every silent `let _ = tx.send(...)` path (worker AssetDone success, worker AssetDone failure, coordinator empty-jobs Finished, coordinator main Finished) + on worker `JoinHandle` panic during join, so any future similar hang surfaces a diagnostic. New unit test `extract_progress_total_tracks_actual_jobs_not_asset_count` pins the count-alignment invariant: 3 assets, 2 on-disk archives ⇒ `extract_progress = Some((0, 2))` (NOT `(0, 3)`) — a regression here is the "extract stalls at N-1 / N forever" shape. Worker loop split into a named `worker_loop` function (was a 99-line `thread::spawn(move || { loop { ... } })`; pedantic gate's `too_many_lines` was tripping). **Item #5 per-install dirs leaking into global Settings → Paths**: SPEC §13.12a separates global (Mods archive, Mods backup, Game sources) from per-install (Mods folder, `weidu_component_logs`, WeiDU-log source folders, game-clone dirs) — but `OrchestratorApp::bio_settings_snapshot` cloned `wizard_state.step1` whole, dragging the per-install paths the install runtime had written into `bio_settings.json` and from there into the Settings → Paths "Temp" field for the next session. Fix: new `install_runtime::settings_sanitizer::sanitize_step1_for_settings_persistence(&mut Step1State, &Step1Settings)` runs against a `step1` clone in `bio_settings_snapshot` and restores the 11 per-install string fields (`mods_folder`, `weidu_log_folder`, `bgee_log_folder`/`bg2ee_log_folder`/`eet_bgee_log_folder`/`eet_bg2ee_log_folder`, `bgee_log_file`/`bg2ee_log_file`, `eet_pre_dir`/`eet_new_dir`, `generate_directory`) + `weidu_log_mode` (which embeds the per-install `log <folder>` token) + the 5 per-install booleans `weidu_log_log_component`/`have_weidu_logs`/`new_pre_eet_dir_enabled`/`new_eet_dir_enabled`/`generate_directory_enabled` to the previously-persisted global values. The live `wizard_state.step1` is NOT touched. SPEC §13.12a now pins the persistence boundary explicitly. Unit test `sanitize_resets_per_install_fields_to_global_values_and_keeps_globals_intact` asserts the 19 reset + 5 untouched-global invariant. **Caveat surfaced (Item #5 step 4 finding)**: Settings → Paths "Temp" field reads from `wizard_state.step1.mods_folder` LIVE, not from `settings_store` — so the persistence fix prevents the cross-session leak but the in-session Settings → Paths display still shows the per-install path until the orchestrator's settings-sync re-feeds settings → step1. The display-source change is deferred (propose-don't-incorporate). `cargo test --lib` **501/0** (+4 from 497 baseline: 2 nav-font picker tests + 1 extract-progress alignment test + 1 sanitizer test); both binaries no-op rebuild; whole-codebase clippy pedantic+nursery gate exits with 0 warnings; DATA-LOSS sentinel byte-identical (24 files: modlists.json + 23 workspace.json — the user's manual testing added 2 more modlists between commits 3 and 4, hashes captured + verified pre/post test run).

**Process:**
- **Per-run branch + PR workflow** (PR #1, merged at `10a2ac0`) — every commit (code AND doc) goes via a per-run branch off `overhaul/infinity_orchestrator` + PR + user squash-merge. Orchestrator never auto-merges. Branch naming: `fix/<slug>` / `feat/<slug>` / `docs/<slug>`. PR body sections: Scope / Change list / Verification / Manual-test script / Judgment calls. Pure-doc commits SKIP the final rebuild gate (saved 30s wastes + the Windows file-lock collision). Full rules in the orchestrator skill's "Per-run branch + PR workflow" subsection.
- **`.claude/` + `CLAUDE.md` files untracked** (PR #2, merged at `5976b4f`) — the orchestrator skill / plan-implementer agent / spec-authority / orchestrator-handoff / per-directory `CLAUDE.md` files are kept locally for the harness but no longer tracked. `.gitignore` flipped from "tracked on purpose so AI orientation travels" to blanket `.claude/` + `CLAUDE.md` excludes. References scrubbed from this HANDOFF + plan/revision-log + wireframe-preview/GAP_ANALYSIS (read path now one-way: AI agents start inside `.claude` and branch out; tracked docs never point back). **Gotcha for fresh clones / `git pull` consumers**: the pull deletes the local `.claude/` files from the working tree as the merge propagates. Restore via `git restore --worktree --source=<commit-with-files> -- .claude/ CLAUDE.md src/CLAUDE.md src/core/CLAUDE.md src/core/app/CLAUDE.md src/core/app/compat/CLAUDE.md src/settings/CLAUDE.md src/ui/CLAUDE.md` (use `2e7f72c` for the latest orchestrator skill; `d3b3c79` for the rest).

**Tooling:**
- **Full clippy pedantic + nursery gate clean across the codebase** (PR #3, commit `517e45b`, merged at `118672f`) — whole-codebase canonical gate `cargo clippy --all-targets --all-features -- -D warnings -W unreachable_pub -W clippy::pedantic -W clippy::nursery -A missing_errors_doc -A missing_panics_doc` now exits 0 with zero warnings. Removed file-level + per-item `#[allow(clippy::...)]` suppressions, stripped forensic header comments (spec / fix-set / run / phase / wireframe-line pointers — comments now describe the code, not its paper trail), trimmed first-doc-paragraphs, fixed `as`-cast lints. **Implication: the touched-files-only grandfathering rule is essentially moot — there are no pre-existing `#[allow]`s or forensic comments to grandfather. Future fix-sets land on a clippy-clean codebase; just don't regress.**
- **New `/code-hygiene` shared skill** (LOCAL, `.claude/skills/code-hygiene/SKILL.md` — not tracked) — both the orchestrator AND the plan-implementer agent read it before touching code. Encodes the clippy pedantic + nursery / no-`#[allow]` / concise-rustdoc / no-`#[doc = "..."]` / no-SPEC-§-or-phase/run/fix-set-pointers-in-code rules. Net behavior: dispatch briefs no longer restate the rules; the agent reads the contract at session start; the first draft is compliant. Gates are a backstop, not a forgive-and-fix loop.

---

## Queued fix-sets — in dispatch order (next first)

**Dispatch policy:** each fix-set as one plan-implementer run per the dispatch-grouped-fixes memory; full gate set (BIO-source guard + scoped clippy + comment hygiene + scoped rustfmt + `cargo test --lib` + both binaries + DATA-LOSS sentinel + runtime trace + render PNGs); per-run branch + PR per the workflow above. Each set must land + user-test cleanly before the next dispatches.

**PR #5 (5 commits) + PR #6 (Born2BSalty cleanup) BOTH MERGED into `overhaul/infinity_orchestrator`** (HEAD `6ea7b5c` as of 2026-05-21). Working tree clean.

**Item #4 Create-fork post-route UX — arc-closing PR open** (branch `xgatt/create-then-modify-fix`, single-branch arc, six runs). Both 🔴 data-loss bugs surfaced by the workflow-trace audit closed by Run 6.

**Verifiably fixed (Runs 1-6, all on origin):**
- **Run 1** `2c95df9` — fork-route un-blocked. `page_router::clear_pending_reinstall_on_nav_away_from_install` was clearing `active_install_modlist_id` while the fork pipeline was mid-extract on the Create stage; skipped during `CreateStage::ForkDownload`.
- **Run 2** `b284525` — apply imported WeiDU log selection on Workspace landing + suppress auto-opened Update Check popup + persist `scratch_mods_folder` for non-dev resume.
- **Run 3** `b67695b` — rescan-wipes-step2 via new `PostInstallResetGate` enum flag (set in `maybe_flip_to_installed_on_clean_exit` after `flip_to_installed`, cleared in `reset_completed_install_runtime`); Step 4 → Step 5 weidu-log auto-save; `fork_extract_complete` gate-eval log spam tightened.
- **Run 4** `57bc5c4` — suppress auto-build install regression unmasked by Run 3; `stage_fork_download::render_live` clears `modlist_auto_build_active` + `modlist_auto_build_waiting_for_install` as soon as extract is done so `advance_pending_saved_log_flow` can't fire `start_auto_build_install` post-apply.
- **Run 5** `7386195` — Workspace install no longer wipes its own success banner + Step-5 console (gate `PostInstallResetGate::Pending` arm on `!from_workspace` in `maybe_flip_to_installed_on_clean_exit`); weidu-log write at Step-4 → Step-5 nav bypasses BIO's `install_mode` early-return via a redesign-side `write_step4_weidu_logs_unconditional`.
- **Run 6** `e8c911f` — closes both 🔴 data-loss bugs surfaced by the post-Run-5 workflow-trace audit. (i) **R1 double `prepare_destination`**: `fork_pipeline_arm::mint_and_arm` was persisting the user's `DestChoice` (Backup/Clear) as `pending_destination_prep` even though the fork-arm's `stage_downloading::arm_pipeline_once` had already run `prepare_destination` synchronously; at Step-5 Install click, `consume_pending_destination_prep` re-ran prep against the freshly-extracted Mods/ + weidu_log_source/ + modlist-import-code.txt (Backup double-archived; Clear permanently deleted). Fix: `mint_and_arm` now writes `pending_destination_prep: None`; scratch flow's separate write site untouched. (ii) **Wrong write path**: `populate_wizard_state_from_workspace`'s `sync_paths_from_settings` call was clobbering per-install `step1` fields with stale global settings values, so weidu.log auto-save at Step-4 → Step-5 nav landed at the previous modlist's destination instead of the current fork's. Fix: call `derive_per_install_dirs(&mut wizard_state.step1, &entry.destination_folder, entry.game)` after the sync, with a `warn!`-and-continue fallback for empty destinations (in-progress drafts). Two regression tests added. ZERO BIO source touched.

**Still open — 🔴 Linux-only:**
- **R7: `delete_modlist` follows symlinks on Linux** — `fs::remove_dir_all` follows symlinks; a user with a symlinked install path gets their actual target destroyed by Delete. Cross-platform-target concern only — Windows users unaffected.

**Workspace-trace audit (`.claude/INSTALL_WORKFLOWS_TRACE.md`, 918 lines):** 13 numbered risks (R1-R13) with severity icons + per-workspace TL;DR banners + inline callouts at every code site where each risk surfaces. R1 is the most critical (data destruction on common fork flow); R7 is data destruction too but Linux-only AND requires user-typed symlink. R6 (Continue partial install dead UI — the third radio in `destination_not_empty.rs` renders but `stage_paste.rs:112-116` jumps Paste → InstallingStub skipping all arming/registration/dir-derivation) is the most visible cleanup item but doesn't destroy anything.

**State architecture analysis (`.claude/state-architecture.md`):** the underlying spaghetti — "current workflow settings" is shattered across 7+ fields on `OrchestratorApp` + the shared `wizard_state`. Seven scattered partial-reset functions fire on every page_router::render frame; no formal `transition_to_workflow` contract. Item #4's six runs all touched this surface; each fix unmasked the next regression. The Phase 8 refactor candidate (a `CurrentWorkflow` enum + single transition function + per-install paths off `wizard_state.step1`) is documented for future agent reference. **Not authorized work**; descriptive only.

**Auto-update / asset-substitution concern remains deferred per user decision (Born2BSalty's thread).** Earlier this session a throwaway probe empirically confirmed that `apply_successful_update_check_outcome` substitutes the asset list when the TP2's parsed version differs from the upstream's current tag — but the user's specific repro dodged it because both mods' TP2 versions matched. Left as latent behavior.

**Next major arc:** Phase 8 (Popup reskins + state-aware theme reads + polish) is the open project work after Item #4 is closed (or independently scheduled if Item #4 turns out to be a small fix).

---

## What ships today

- Two binaries coexist:
  - `BIO` — the legacy linear-wizard app, untouched in behavior, still launches from `cargo run --bin BIO`.
  - `infinity_orchestrator` — the new redesigned app, launches from `cargo run --bin infinity_orchestrator`.
- Builds natively on Windows, Linux (Ubuntu CI), and macOS. Cross-compilation has known toolchain issues — see *Windows builds* below for the historical detail.
- 515/515 lib tests pass.
- Cargo version `0.1.0-Alpha.1`.
- The orchestrator binary opens an `eframe` window (1280×820, min 1024×700) with:
  - **Titlebar** (34px, sketchy border, `Infinity Orchestrator` title centered, traffic-light dots top-left).
  - **Left rail** (200px) with the brand mark + 4 nav items (Home / Install / Create / Settings) + a bottom status indicator (`weidu vN · all paths ok` or per-path error count).
  - **Body** with the active destination's content.
  - **Statusbar** (26px) at the bottom showing modlist count + jobs-running placeholder.
- **Home**: title + subtitle, filter chips (Installed / In progress / All) with counts + default-selection logic, modlist cards (in-progress `resume` / installed `open` + Kebab), `add a modlist` CTAs, `game installs detected` block, first-launch setup CTA, bottom-center toasts. Kebab actions: Copy import code, Rename (registry write), Delete (danger confirm → registry entry + guarded on-disk folder removal), Open install folder, Reinstall.
- **Install Modlist (paste)**: 4-stage flow — Paste → Preview → Downloading → Installing. Paste collects destination + `DestinationNotEmptyWarning` (Clear / Backup / Continue) + the import-code textarea; a valid existing destination directory is required to proceed (SPEC §4.1). Preview parses the share code → packed `name`/`author` title + subline + Overview Box + 6 file-folder tabs + `allow_auto_install` draft-gate (disabled Import + `Open in Create →` for draft codes); `⑂ fork info` opens `ForkInfoPopup` when the lineage is non-empty. Downloading runs the live three-phase pipeline (Hashing → Downloading → Extracting), 10-worker pool per phase, real per-mod byte progress, checksum-then-skip cache. Installing embeds BIO's Step 5 console + post-install actions (Return to Home / Open install folder).
- **Create**: Choose stage = Setup Box (modlist name + game + destination + conditional `DestinationNotEmptyWarning` with Clear / Backup only) + a `Choose one` header + two selectable boxes (`New modlist from downloaded mods` / `Import and modify another modlist`) + a single bottom-right `Start →`. From-scratch lands on the Workspace at Step 2 with an empty selection. Import-and-modify routes through paste → preview → fork-download (live pipeline targeting the destination's per-install Mods folder); lands on Workspace Step 2 at extract-complete with the parent's lineage appended. Load Draft dialog lists in-progress builds with `resume` + Kebab (Copy import code / Delete).
- **Workspace** (Steps 2–5): workspace header with editable modlist name (✎ inline rename) + Share-import-code button; nav bar with `← Previous` / `Next →` glyph buttons (rail-locked during in-flight installs). Step 2 = Scan and Select; Step 3 = Reorder and Resolve; Step 4 = Review; Step 5 = Install with BIO's runtime, C3-gated success banner, post-install actions, and an embedded console with wrap + `break_anywhere` for long log lines.
- **Settings**: five-tab screen (General / Paths / Tools / Accounts / Advanced):
  - Live theme-palette toggle (Light / Dark) updates next frame.
  - Per-keystroke debounced path validation updates the rail status row.
  - GitHub OAuth `connect` opens BIO's existing device-flow popup.
  - Persistence: global paths → `bio_settings.json`; orchestrator-only prefs (theme, user name for the share-code `author`, etc.) → `bio_redesign_settings.json`. Per-install fields are filtered out of the persistence snapshot by `install_runtime::settings_sanitizer` so per-install Mods folder / weidu_component_logs / game-clone paths never leak into Settings → Paths across sessions.
- Modlist registry (`modlists.json`) + per-modlist workspace state (`modlists/<id>/workspace.json`) read/write via the orchestrator-owned persistence cycle. Atomic writes via temp-file-then-rename. Corrupt registry → terminal error pane on next launch (no silent recovery).

---

## Build setup

Required toolchains (Windows / Linux / macOS — all native, no cross-compile):

- **Rust** stable, via [rustup](https://rustup.rs/).
- **Java** (JDK 11+), needed by `lapdu-parser-rust`'s build script for ANTLR codegen.

Build / test commands:

```bash
cargo build --bin BIO --release
cargo build --bin infinity_orchestrator --release
cargo test --lib
```

Both binaries land in `target/release/`.

Run the orchestrator:

```bash
./target/release/infinity_orchestrator        # production mode
./target/release/infinity_orchestrator -d     # dev mode (diagnostics export + extra logging)
```

Logging level is controlled via `--log-level {trace|debug|info|warn|error}` (default `info`). Note that `RUST_LOG` is not read by this codebase — use the CLI flag. On Windows, capture logs via PowerShell:

```powershell
& .\target\release\infinity_orchestrator.exe -d 2>&1 | Tee-Object -FilePath log.txt
```

---

## File layout

```
infinity_orchestrator/                  # this folder (artifacts: spec, plan, wireframe, handoff)
├── SPEC.md                             # canonical product spec (read first)
├── HANDOFF.md                          # this file
├── plan/                               # (streamlined 2026-05-19; see plan/revision-log.md)
│   ├── overview.md                     # phasing philosophy + phase table + architecture
│   ├── revision-log.md                 # dated history — split out of overview.md
│   ├── phase-01-theme-and-shell.md     # SHIPPED — ~10-line status stub (full doc in archive/)
│   ├── phase-02-nav-routing.md         # SHIPPED — stub
│   ├── phase-03-modlist-registry.md    # SHIPPED — stub
│   ├── phase-04-settings.md            # SHIPPED — stub
│   ├── phase-05-home-install-paste.md  # SHIPPED — stub
│   ├── phase-06-create-workspace-shell.md  # SHIPPED — stub
│   ├── phase-07-install-runtime.md     # SHIPPED — stub
│   ├── phase-08-popup-reskins-polish.md  # LIVE
│   └── archive/                        # full pre-streamline phase-01..07 docs (audit-only)
└── wireframe-preview/                  # canonical visual reference (HTML+React preview)
    ├── build.html                      # built single-file preview (open in browser)
    ├── index.html                      # CSS tokens + font-face declarations + shell layout
    ├── screens.jsx                     # every screen + popup component
    ├── app.jsx                         # top-level shell + nav + route dispatch
    └── tweaks-panel.jsx                # design-iteration tool only (NOT shipped)

src/                                    # the actual `bio` crate
├── lib.rs                              # library root (Phase 1 carve-out #3)
├── main.rs                             # thin shim for the BIO binary
├── bin/
│   └── infinity_orchestrator.rs        # the new binary's main (Phase 1+2)
├── core/                               # BIO's existing core logic — TREAT AS PROTECTED
│   ├── app/                            # state machines, install runner, scan worker, ...
│   ├── cli/                            # CLI args
│   ├── config/                         # compat rules, mod-source manifests
│   ├── install/                        # install pipeline
│   ├── parser/                         # TP2 / weidu.log parsing (ANTLR-generated)
│   └── ...
├── settings/                           # bio_settings.json model + store
│   ├── model.rs                        # AppSettings + Step1Settings (BIO source)
│   ├── store.rs                        # SettingsStore (BIO source)
│   ├── redesign_fields.rs              # RedesignSettings (Phase 4 net-new)
│   └── redesign_store.rs               # RedesignSettingsStore (Phase 4 net-new)
├── ui/                                 # all UI rendering
│   ├── shared/                         # theme tokens, fonts, layout constants
│   │   ├── theme_global.rs             # BIO existing
│   │   ├── layout_tokens_global.rs     # BIO existing
│   │   ├── typography_global.rs        # BIO existing
│   │   ├── redesign_tokens.rs          # Phase 1 — REDESIGN CANONICAL TOKEN STORE
│   │   └── redesign_fonts.rs           # Phase 1 — font loader
│   ├── shell/                          # Phase 1 — shell chrome
│   │   ├── shell_chrome.rs
│   │   ├── shell_titlebar.rs
│   │   └── shell_statusbar.rs
│   ├── orchestrator/                   # Phase 2 — new orchestrator code
│   │   ├── orchestrator_app.rs         # OrchestratorApp (eframe::App impl)
│   │   ├── nav_destination.rs          # NavDestination enum + rail items
│   │   ├── left_rail.rs                # left rail widget
│   │   ├── page_router.rs              # destination dispatch
│   │   ├── nav_status.rs               # path-validation summary for rail status
│   │   ├── registry_error_panel.rs     # Phase 3 — terminal error UI
│   │   ├── widgets/                    # btn, r_box, label, screen_title
│   │   └── stubs/                      # placeholder destinations
│   ├── settings/                       # Phase 4 — Settings screen
│   │   ├── page_settings.rs            # 5-tab top-level
│   │   ├── state_settings.rs
│   │   ├── tab_general.rs              # name + theme + language + validate-on-startup + diag
│   │   ├── tab_paths.rs                # game/working folders + Validate now
│   │   ├── tab_tools.rs                # weidu / mod_installer / 7z / git binaries
│   │   ├── tab_accounts.rs             # GitHub / Nexus / Mega cards
│   │   ├── tab_advanced.rs             # timing + install behavior + WeiDU flags
│   │   ├── validate_now.rs             # synchronous validation
│   │   ├── validate_debounce.rs        # H11 — per-edit debounced validation
│   │   ├── oauth_glue.rs               # GitHub OAuth wrapper
│   │   └── widgets/                    # tab_strip, path_row, value_row, etc.
│   ├── step1/  step2/  step3/  step4/  step5/   # BIO existing — protected by CRITICAL DIRECTIVE
│   ├── app.rs  app_methods.rs ...      # BIO existing — WizardApp + handlers
│   └── frame/                          # BIO existing — window setup
├── registry/                           # Phase 3 — modlist registry
│   ├── model.rs                        # ModlistRegistry, ModlistEntry, ModlistState
│   ├── store.rs                        # RegistryStore (atomic load/save)
│   ├── workspace_model.rs              # ModlistWorkspaceState
│   ├── store_workspace.rs              # WorkspaceStore
│   ├── persistence_cycle.rs            # debounced writes + flush
│   ├── dev_seed.rs                     # dev-only seed for Phase 3 testing
│   ├── ids.rs                          # ULID-style ID generator
│   ├── errors.rs                       # RegistryError { Io, Parse, Corrupt }
│   └── operations.rs                   # stub for Phase 5 (create/rename/delete)
└── ...

assets/
├── fonts/                              # Poppins (300/500/700) + FiraCode Nerd 300 TTFs
├── icon.ico                            # Windows
└── icon.png                            # cross-platform

target/release/
├── BIO                                 # existing binary, ~26 MB
└── infinity_orchestrator               # new binary, ~11 MB after Phase 4

vendor/
└── lapdu-parser-rust-master/           # vendored TP2 parser (needs Java for ANTLR codegen)

third_party/
└── egui_term/                          # patched egui_term crate
```

Persistence files at runtime live in:
- macOS: `~/Library/Application Support/bio/`
- Linux: `~/.config/bio/`
- Windows: `%APPDATA%\bio\`

Specifically:
- `bio_settings.json` (existing BIO)
- `bio_redesign_settings.json` (Phase 4 net-new)
- `modlists.json` (Phase 3 net-new)
- `modlists/<id>/workspace.json` (Phase 3 net-new, one per modlist)
- `prompt_answers.json` (existing BIO)
- `step2_compat_rules_user.toml` (existing BIO)
- `mod_downloads_user.toml` (existing BIO)

---

## The CRITICAL DIRECTIVE (do not modify existing BIO components)

`SPEC.md` §1 is the single most important rule for this project. Read it before touching code.

**Two legal options for every redesign surface:**
1. Reuse the existing BIO component as-is (with theme-token styling applied).
2. Create a net-new component alongside (not on top of) the existing BIO code.

**Six approved carve-outs** for mild refactors to existing BIO source — anything outside these is disallowed:

1. **Theme-token extraction** — swap inline `Color32::from_rgb(...)` / `f32` literals for token reads. Pure value substitution.
2. **Window-chrome config flips** — single-line `.collapsible(false)` → `.collapsible(true)` and similar on `egui::Window` builders. Body content, signatures, behavior unchanged.
3. **Library/binary structural split** — done in Phase 1. Adds `src/lib.rs`, slims `src/main.rs` to a shim, adds a new `[[bin]]`. *Companion provision:* additive `pub mod foo;` lines in existing BIO `mod.rs` files are allowed to register new sibling modules (no reordering, no edits to existing lines).
4. **WizardApp → WizardState signature refactor** — BIO functions whose body only mutates `app.state` may be refactored to take `&mut WizardState`. Body unchanged. Audit found no actual cases needing this in v1; carve-out stays as a safety net.
5. **Schema-additive serde field additions** — new optional `#[serde(default = ...)]` fields on existing BIO serde structs. Default must preserve today's BIO behavior. Used by `allow_auto_install` on `ModlistSharePayload` (Phase 7).
6. **State-aware theme-token reads** — Phase 8 expanded. Inline color literals inside state-dependent conditionals may be swapped for `redesign_*(palette)` accessor calls, **provided** the conditional structure is unchanged (no new branches, no removed branches, no logic mutations). Function gains a `palette: ThemePalette` argument.

**Decision order when a BIO function is not a clean fit:**

1. **Direct reuse** if any `bio::app::*` / `bio::core::*` / `bio::ui::shared::*` public API does what's needed.
2. **Net-new sibling** for *simple* workflows (state mutations, dialog wrappers, format helpers, single-screen panels). This is the default fallback.
3. **Carve-out escalation** for *complex* workflows that can't be cleanly siblinged (install pipeline, share-code interop, multi-step state coordination). Requires explicit user approval.

Net-new is for simple things; carve-outs are for complex things that can't be cleanly cloned. Most "BIO function isn't reachable" flags are simple — build a sibling and move on.

---

## Source-of-truth ordering for new work

When phase implementation needs a value or behavior:

1. **`infinity_orchestrator/SPEC.md`** — the canonical product spec.
2. **`infinity_orchestrator/wireframe-preview/build.html`** + its source files — the canonical visual reference. **For UI / UX / layout / copy / spacing / pixel values, the wireframe wins over the spec.**
3. **The relevant `infinity_orchestrator/plan/phase-XX-*.md`** — your work order for this phase.
4. Existing BIO behavior — fallback only when spec, wireframe, and plan are silent.

Wireframe source files to read directly (don't paraphrase via the spec):
- `wireframe-preview/index.html` — CSS `:root` variables + font-face declarations.
- `wireframe-preview/screens.jsx` — every screen + popup component.
- `wireframe-preview/app.jsx` — top-level shell + nav.

The Tweaks panel (`tweaks-panel.jsx`) is wireframe-iteration only and does NOT ship.

---

## Remaining phases — quick reference

Each phase doc in `infinity_orchestrator/plan/` is the canonical work order. Summaries below.

### Phase 5 — Home + Install Modlist (paste / preview / download stages)

**Ships:**
- Real Home screen replaces the stub: title row, filter chips (`Installed (N)` / `In progress (P)` / `All (N+P)`), scrollable card list (mod name + meta line + `play` (renamed from wireframe's `play`, opens install folder for v1) / `resume` + Kebab), "Add a modlist" Box with `paste import code` / `create your own` CTAs, `game installs detected` block driven by Phase 4's path validation events, first-launch empty-registry CTA card.
- Install Modlist destination's first three stages: paste textarea + destination folder + DestinationNotEmptyWarning (3 radio options: `clear` / `backup` / `continue partial install`); preview screen with overview Box + 6-tab content Box (Summary / BGEE WeiDU / BG2EE WeiDU / User Downloads / Installed Refs / Mod Configs); downloading stage with per-mod progress grid.
- Stage 4 (the actual install) is stubbed and rolled in during Phase 7.
- Delete confirm dialog removes the modlist registry entry **and** the install folder.
- `allow_auto_install` flag check at preview stage: codes generated by drafts have the bit `false`; the preview disables the Install button and routes the user to `Create → Import and modify` instead. Per SPEC §4.2 + §13.3.

**Dependencies:** Phases 2 + 3 + 4.

### Phase 6 — Create + Workspace shell

**Ships:**
- Create destination: choose-mode setup Box (name + game ComboBox + destination FolderInput) + two starting-point cards (`New modlist from downloaded mods` / `Import and modify another modlist`) + `load draft` button opening the Load Draft dialog.
- Fork-paste / fork-preview / fork-download sub-flow for "Import and modify".
- WorkspaceView shell hosting Steps 2–4 (Step 5 stubbed; Phase 7 wires it):
  - Header row: `Editing <modlist name>` + ✎ rename inline edit + Fork badge + `save draft` / `Share import code` buttons.
  - 4-step progress bar.
  - Per-step hint line.
  - Step body — **Step 2 is an orchestrator-side C4 chrome wrapper** (`workspace_step2::render`, P6.T2c — net-new redesign chrome (`step2_tab_row` + `step2_search`): title, full-width `flex` search + Rescan (disabled pre-Phase-7 per §13.12a) / Cancel-while-scanning, redesign GameTabs + Select-via-Log + Updates + clickable compat/prompt pills + count + Kebab[Show/Hide Details · Clear All · Select Visible · Collapse All · Expand All · Jump to Selected], **no** "Restart App With Diagnostics", Details pane hidden-by-default per SPEC §6) that reuses **only** BIO's tree / details / compat / prompt / updates sub-renderers + the same `pub(crate)` toolbar-action helpers `render_controls` itself calls (read-only); BIO's `render_controls` / `render_tabs` / `render_header` / `page_step2` / `frame_step2` are **not** called (the 2026-05-16 SPEC-CONFLICT resolution — the wireframe's Step 2 is structurally different from BIO's and colour-only carve-out #6 cannot restructure `frame_step2`). **Step 3 is an orchestrator-side C4 chrome wrapper** (`workspace_step3::render`, P6.T2d — **shipped**): net-new redesign chrome (`step3_action_row` + `step3_tab_row`): the action-row count "_N_ components ready to install on _<tab>_ · across _M_ mods" (reuses the shared `pub` `workspace_step4::active_tab_items` resolver so the Step-3/Step-4 count never drifts; no Save button — Save is Step-4 only per §7.6), the **shared** redesign GameTabs (the one `widgets::game_tab` widget Steps 2/3/4 render identically; single-game skips the strip via `workspace_step4::is_dual_game`), the aggregate conflict/prompt clickable pills (open the compat/prompt popups via the same `pub(crate)` `toolbar_support_step3::open_toolbar_issue_popup` / `prompt_popup_step2::open_toolbar_prompt_popup` BIO uses), and redesign `Undo`/`Redo`/`Collapse All`/`Expand All` (the exact `pub(crate)` `toolbar_support_step3::{undo_active,redo_active,collapse_all_active,expand_all_active}` helpers `content_step3::render_toolbar` calls), **no** "Export diagnostics" / "Restart App With Diagnostics", **no** BIO heading/hint (the shell renders the per-step hint). It reuses **only** BIO's drag-reorder list body (`bio::ui::step3::list_step3::render`, `pub(crate)`, read-only) inside an orchestrator-owned hard-clipped fixed-size rect (the verbatim Step-2 `clipped_pane`) so the list never bleeds into the nav bar; the reused-seam prologue (`state_step3::normalize_active_tab` → `toolbar_support_step3::build_toolbar_summary` → set `step3.{bgee,bg2ee}_has_conflict` → list → `content_step2::render_compat_popup` + `prompt_popup_step2::render_prompt_popup`) matches `content_step3::render` (`content_step3.rs:224-261`) exactly. BIO's `page_step3`/`content_step3`/`render_toolbar`/`frame_step3` are **not** called (the legacy `BIO` binary still renders its own — unaffected). No `Step3Action` (H2) — returns `()`; the dirty-bit fingerprint over `wizard_state.step3.<tab>_items` detects reorder/collapse/undo for persistence (unchanged). Step 4 is an orchestrator-side renderer (per C4 — replaces BIO's `page_step4::render` to avoid the double Save button).
  - The 6 Step-2 background-thread receivers are drained every frame by `OrchestratorApp::poll_step2_channels` (the narrower-call mirror of `bio::app::app_update_cycle::poll_before_render`'s Step-2 portion — `poll_before_render` is monolithic and also requires Step-5 runtime args the orchestrator does not own pre-Phase-7). Without it the scan worker starts but never reports and Cancel never completes. A **dev-only** scan-folder affordance (behind `-d`; absent in normal mode; enabled on `dev_mode` only) lets a developer point BIO's scan at an arbitrary folder, because pre-Phase-7 there is no per-install extracted-mods folder to scan (SPEC §13.12a) — it is the **functional scan path until P7.T17**. The **production "Rescan Mods Folder" button is inert (disabled + explanatory tooltip) pre-Phase-7** for the same reason (no valid per-install extracted-mods target until P7.T17) — the same accepted §13.12a Phase-7 deferral pattern as the §4.3 Downloading chassis; it lights up automatically when P7.T17 derives the per-install Mods folder. While a scan runs, the Rescan button is replaced in place by **"Cancel Scan"** (`Step2Action::CancelScan`) — a deliberate wireframe-omission addition (a necessary capability; recorded). **Rescan is non-destructive (SPEC §6.3): it reconciles, never wipes** — net-new orchestrator logic (`step2_rescan_reconcile`, BIO has no reusable rescan-preserves-selection mechanism) snapshots the selection at scan-trigger time and, on scan-**completion** (after `poll_step2_channels` lands the fresh mods; only on a *successful* `Finished`), re-applies it onto the freshly-scanned mod list (matched by `tp2`+component id, `selected_order` preserved), drops only mods/components no longer present + surfaces _"N component(s) dropped — M mod(s) no longer present"_ in the scan-status footer (no dialog). Step 3 is re-synced from Step 2 on the Step2→Step3 forward nav edge by mirroring BIO's exact `decide_next_action`→`sync_step3_from_step2` trigger (no BIO edit; Step-3 reorder preserved via BIO's own `reconcile_step3_items`). **Select-via-WeiDU-Log is destructive (it replaces every selection on the tab) and is gated by the SPEC §6.10 danger `ConfirmDialog`** (the shared widget Home Delete/Reinstall reuse — wireframe `askWeiduImport` copy verbatim): clicking the tab-row button arms `WorkspaceStep2State::pending_weidu_log_confirm` (target tab); `workspace_step2::render` shows the danger confirm; only on **Confirm** does the `step2_log_glue` picker+apply path run; **either cancel point — the dialog *or* the file picker — is a no-op that preserves the current selection** (the picker no longer falls back to the resolved default log; that BIO-legacy parity silently wiped selections). On **Confirm** the picker+apply path additionally **discards the imported tab's stale Step-3 order** (`9b5b9d5`) so the next Step2→Step3 sync rebuilds it fresh in weidu.log order — cold-resume pre-populates `state.step3.<tab>_items`, and without this reset BIO's `reconcile_step3_items` would preserve the stale resumed order on a destructive whole-tab re-import; only the imported tab is cleared, and BIO's incremental-edit preserve behavior is deliberately untouched.
  - Workspace nav bar: `← Previous` / `Next →`. On the **first** workspace step (Step 2) `← Previous` routes back to **Home** (SPEC §2.2 — affordance-forward: the user reached the workspace via a Home `resume`/`open`; recorded, overview 2026-05-16, deviation from the wireframe's former first-step *disabled* state). `← Previous` is enabled on the first step and force-disabled **only** by `disable_prev` (the Phase-7 install-running / post-install gate; `false` until Phase 7).
- Workspace state loader: populates `WizardState` from per-modlist `workspace.json` on open; extracts back on save / nav-away / debounced write. **Loader is never invoked while an install is running** (per C5 — rail-nav lock).
- `sync_paths_from_settings` re-asserts `Step1Settings` paths into `wizard_state.step1` **once on workspace open** (not per frame): the orchestrator's Settings → Paths tab edits the same in-memory `wizard_state.step1` the workspace renders from, so edits propagate by construction without a close/reopen (M2 — open-only; overview 2026-05-16).
- Per-frame dirty bit gates persistence writes (per H1) — no per-frame extract+compare.
- Step action dispatch tables in the phase doc enumerate every `Step2Action` / `Step4Action` variant + which `bio::app::*` public function handles it (per M4).

**Dependencies:** Phases 2 + 3 + 4 + 5.

### Phase 7 — Step 5 install runtime + Reinstall + import-code auto-write + install concurrency + rail-nav lock

**Ships:**
- Step 5 inside the workspace renders BIO's existing `page_step5::render` (the full embedded panel: Command card, Summary card, Actions/Diagnostics menus, Prompt Answers, console box wrapping `EmbeddedTerminal`, prompt input row). New chrome wraps **around** it — success banner row above, post-install action row above-and-adjacent to the (now disabled) Install button (per SPEC §9.2 + H9).
- Install start hook: writes `modlist-import-code.txt` to the install destination before WeiDU runs (per SPEC §13.13). Write semantics per button variant: `Install` / `Restart Install` / `Reinstall` overwrite; `Resume Install` does not (per H10).
- Post-install state transition: clean exit (the C3 triple: `install_running == false && last_exit_code == Some(0) && last_install_failed == false`) flips the registry entry from `in-progress` to `installed`, regenerates `latest_share_code` with `allow_auto_install = true`, **and rewrites the on-disk `<destination>/modlist-import-code.txt` with that same `allow_auto_install = true` code** (SPEC §13.13 — the on-disk artifact becomes the verified, directly-installable code post-success, matching `latest_share_code`; reuses `import_code_writer`'s path/filename; non-fatal/defensive; **H8: import-code file only — never a registry snapshot to disk**). A non-clean exit leaves the install-start `allow_auto_install = false` draft on disk untouched (the rewrite is exclusively inside the C3-gated `flip_to_installed`). Async size computation on a worker thread (per M5).
- `SharePasteCodeDialog` opens from the workspace header's `Share import code` button (post-install only).
- Reinstall flow from Home Kebab: danger confirm modal → routes to Install Modlist preview stage with overwrite-install forced → user clicks Install → registry flips back to in-progress → install runs (per SPEC §3.1 + H2). Cancel-preview leaves modlist in `installed` (per M5).
- **Install concurrency policy** (SPEC §13.15): only one install runs at a time. **Rail navigation is hard-locked** while an install runs (per C5) — every left-rail item disabled with the SPEC tooltip. User can only stay in the running install's workspace until cancel or completion.
- Install Modlist stage 4 wired (the real install runtime; not in the workspace chrome).
- **P7.T17 — per-install dirs + content-addressed archive staging + import→auto-build pipeline drive (SPEC §13.12a).** Derives the per-install Mods folder + the install-critical dirs (#2 per-component `weidu_component_logs/` [conveyed via the `weidu_log_mode` `log <folder>` token — **no `-u` arg in BIO** — by read-only reuse of BIO's `sync_weidu_log_mode`], #3 `-p`/`-n`, #4 `-g`) inside the destination with forced clone flags; the **download is the net-new parallel `stream_downloader`** (Run 4 / #1 — replaces BIO's serial worker, no double-download), with a net-new content-addressed staging layer around it; **`app_step2_update_extract` is reused unchanged** (extract not forked); drives `import_modlist_share_code` → saved-log/auto-build → download/extract → install; binds the Phase-5 §4.3 Downloading chassis (and the Phase-6 fork-download chassis) to live data with a real per-mod byte fraction. Global paths come from Settings → Paths via `sync_paths_from_settings`.
- `pending_reinstall_id: Option<String>` on `OrchestratorApp` (per L12) tracks the in-flight reinstall route.
- Automatic flag policies: #1 (`-s` / `-c`) + #5 (`--download`) wired in Phase 7 P7.T16; **#2 (per-component logging — the `weidu_log_mode` `log <folder>` token, NOT a `-u` flag) + #3 (`-p`/`-n`) + #4 (`-g`) wired in Phase 7 P7.T17** (their per-install dirs are install-critical — an install can't run without them, so they cannot be deferred to Phase 8 — SPEC §13.12a). Only #6 (`prepare_target_dirs`/`backup` from the `DestChoice` mapping, already the pure `DestChoice::to_flags` from Run 3) and #7 (`-autolog`/`-logapp`/`-log-extern`, hardcoded) remain for Phase 8.

**Dependencies:** Phases 2 + 3 + 5 + 6.

### Phase 8 — Popup reskins + state-aware theme reads + polish

**Ships:**
- Theme-token extraction (carve-out #1) on the popup files: `compat_popup_step2.rs`, `compat_window_step2.rs`, `prompt_popup_step2.rs`, `update_check_popup_step2.rs` + its companions, `github_auth_popup_step1.rs`.
- `.collapsible(false)` → `.collapsible(true)` flips (carve-out #2) on those popups so the global collapse chevron pattern works.
- **State-aware theme-token reads (carve-out #6)** on the Step 2 tree (`tree_compat_display_step2.rs`, `tree_component_row_step2.rs`, `tree_parent_step2.rs`, `tree_header_marker_step2.rs`, `format_step2.rs`), Step 2 Details panel (`details_pane_step2.rs`, `details_paths_step2.rs`, `details_selection_step2.rs`), Step 3 reorder list (`list_rows_step3.rs`, `content_step3.rs`, `format_step3.rs`, `toolbar_compat_step2.rs`), Step 5 sub-renderers (`content_install_row_step5.rs`, `content_cancel_step5.rs`, `content_dev_header_step5.rs`, `status_phase_step5.rs`, `status_console_step5.rs`).
- Anchor-on-collapse wrapper for popups (if egui's native title-bar collapse doesn't auto-anchor) in `src/ui/orchestrator/widgets/popup_collapse_anchor.rs`.
- Residual automatic flag policies from SPEC §13.12: **#6 + #7 only** (#1/#5 → Phase 7 P7.T16; #2/#3/#4 install-critical per-install dirs → Phase 7 P7.T17, SPEC §13.12a) + Settings-surface removal.
- Dotted radial background pattern matching the wireframe's `body` background.
- Toast notifications, hover affordances, copy-to-clipboard polish.
- Final smoke pass.

After Phase 8, every workspace surface visually matches the wireframe — Step 2 tree, Step 3 list, Step 5 console all render in the redesign's dark teal-on-slate palette.

**Dependencies:** Phases 1–7 (phases 5+6+7 surface the BIO renderers Phase 8 touches).

---

## Known caveats (carry these forward)

- **Latin-subset fonts.** Phase 1 derived Poppins TTFs from the wireframe's `.woff2` Latin-only subsets. FiraCode Nerd is full coverage. Before any non-stub UI ships in production, replace Poppins TTFs in `assets/fonts/` with full Latin-Extended `.ttf` builds from upstream Google Fonts. Code uses `include_bytes!` so swapping the files + rebuilding is sufficient.
- **`PromptInfo` private-interface warnings** (2 warnings) in `src/core/app/terminal/output.rs` are pre-existing BIO source; not introduced by the orchestrator work. Mention if you ever clean them; otherwise leave.
- **BIO's `configure_typography` must not be called from orchestrator code.** It calls `ctx.set_fonts(FontDefinitions::default())` which wipes the redesign font registrations. The orchestrator only calls `install_redesign_fonts`.
- **`FontFamily::Name("X")` requires `X` to be registered** in `install_redesign_fonts`. Registered names: `poppins_light`, `poppins_medium`, `poppins_bold`, `firacode_nerd`. Using an unregistered name causes a runtime panic (`FontFamily::Name(\"X\") is not bound to any fonts`).
- **Symbol-glyph coverage is split — know which font has what (cmap-verified).** The shipped Poppins TTFs are a Latin-only **217-glyph subset**: every non-Latin glyph tofus in any `poppins_*` family (and a tofu'd `✓` silently becomes `?`, which has masqueraded as real state — a *detected* game looked *missing*). `assets/fonts/FiraCodeNerdFont-Light.ttf` is the **full** 10,801-glyph Nerd build and **does** cover base-FiraCode ranges — math/arrows/dingbat-checks: `∞` U+221E, `✓` U+2713, `→` U+2192, `←` U+2190 all present (cmap-verified) — so render *those* glyphs in `firacode_nerd`, prose in Poppins. **But FiraCode Nerd does NOT cover the "Miscellaneous Symbols" block (U+2600–26FF).** Verified absent even in the full build: `⚠` U+26A0 (and by the same token `⚙` U+2699, `☰` U+2630, etc. — note `nav_destination::icon()` still returns these as strings, harmless only because `left_rail` paints the rail icons as vectors and never renders that string). For any Misc-Symbols / emoji glyph, **paint a vector** — `left_rail.rs`'s nav icons and `destination_not_empty.rs`'s `paint_warning_triangle` are the precedent; it decouples from font coverage entirely. The Latin-Extended Poppins swap above fixes none of this (these aren't Latin-Extended). Don't assume a glyph exists in a font — check the cmap (`python -m fontTools` / the bundled fonttools approach used in this session).
- **Per-frame theme propagation.** The active `ThemePalette` lives on `OrchestratorApp::theme_palette`. Pass it explicitly into every render function that needs colors. There is no global theme state.
- **Share-code provenance (`name`/`author`/`forked_from`).** Packed *inside* the BIO-MODLIST-V1 payload (not a string prefix) via the net-new orchestrator `registry::share_export::pack_meta` envelope — BIO's generator/consumer are unmodified beyond the carve-out #5 `#[serde(default)]` fields. `author` ← `RedesignSettings.user_name` (SPEC §11.1); `name` ← `ModlistEntry.name`; `forked_from` is append-only (`ForkAncestor { name, author }`, oldest→newest) so original creators stay credited. Displayed in the Install/fork preview title+subline and the `ForkInfoPopup` (SPEC §10.9; the same `fork_info_popup.rs` widget serves the Install preview, fork-preview, and workspace `⑂ view fork details`). Codes lacking the fields fall back to `Shared modlist` / author-less — intentional, not a defect. Full spec: SPEC §13.3 (Provenance + Generation mechanism), §1 carve-out #5; rationale in overview.md 2026-05-15 revision-log entry.
- **Directory architecture + content-addressed archives (SPEC §13.12a).** Global, Settings-defined (§11.2): **Mods archive** (ALL downloads for ALL modlists, always), **Mods backup**, **Game sources**. **Every per-install directory lives inside the modlist's destination** (no exception): the **Mods extract/stage/scan folder** (removed on clean success), the **`weidu_component_logs/`** dir (SPEC §13.12 #2 — conveyed via the `weidu_log_mode` `log <folder>` token, **not** a `-u` flag; there is no `-u` arg in BIO — read-only reuse of BIO's own `sync_weidu_log_mode`, zero BIO source, no carve-out), the **WeiDU-log SOURCE folders** (`<dest>/weidu_log_source/{bgee,bg2ee}`), + the **game-install clone dirs** (already specced by §13.12 #3/#4 — always cloned, fixed names; the redesign never surfaces BIO's no-clone path, BIO untouched). `per_install_dirs::resolve(destination, game)` / `derive_per_install_dirs(step1, destination, game)` are the 3-arg pre-id-threading form; they take no `modlist_id` and have no `_with_data_dir` split (tests stay DATA-LOSS-safe by being temp-path / in-memory only). Upstream BIO fix **`a38e360`** ("Allow spaces in per-component WeiDU log folder", merged into `overhaul/infinity_orchestrator` as **`8df994a`**) removed WeiDU's per-component-log-folder no-space preflight from `src/core/app/state/state_validation_paths.rs`, so there is no longer any constraint pushing the per-component-log dir out of the user's free-form destination — it sits beside the other per-install dirs. (`registry::store_workspace::modlist_data_dir` remains the single canonical **`workspace.json`**-parent resolver; it no longer anchors any per-component-log/Kebab path.) The global Mods-archive is **content-addressed**: hash-on-write, same name+hash ⇒ cross-modlist dedupe, same name/different hash ⇒ both coexist, the modlist lock records its hash, extract selects the matching archive per install. **The download is the orchestrator's net-new parallel `stream_downloader`** (Run 4 / #1 — replaces BIO's serial `app_step2_update_download` worker; `pending_saved_log_download` not armed so it never double-downloads; `archive_file_name` reused read-only); **`app_step2_update_extract` is reused UNCHANGED** (extract is NOT forked); the content-addressed staging layer is net-new orchestrator code around that (zero BIO modification). The orchestrator drives BIO's `import_modlist_share_code` → saved-log/auto-build pipeline; global paths reach the owned `WizardState` via `sync_paths_from_settings` (the Install screen does NOT collect game paths). Lands in **Phase 7 P7.T17** (the pipeline terminates in the install runtime); Phase 5 shipped the §4.3 chassis only. Rationale: overview.md 2026-05-16 + 2026-05-18 (THE AMENDMENT + the fix arc) revision log.
- **Per-install WeiDU-log source folders are part of the §13.12a derived set (settled — the real "Install-Modlist-paste download never starts / inert 0/0" root cause).** `per_install_dirs::derive_per_install_dirs` derives, inside the destination, **two** per-install WeiDU-log source phase folders (`<dest>/weidu_log_source/{bgee,bg2ee}` — distinct so an EET import's BGEE-phase + BG2EE-phase logs never collide) and sets the six `Step1State` log fields (`<game>_log_folder` / `<game>_log_file` / the `eet_*` pair) **before** `import_modlist_share_code`, so BIO's importer write target (`modlist_share::import_log_target_path`) **and** the saved-log/auto-build applier read path (`app_step2_log::resolve_*_weidu_log_path`) resolve to the same per-install file in every install mode the imported payload can carry. Previously these stayed `Step1State::default()` empties ⇒ the importer `Err`'d ⇒ the whole Install-Modlist-paste / Reinstall pipeline never armed (permanent inert "0 / 0 mods"). The fields survive the import's `clone` + `reset_workflow_keep_step1`; zero BIO edit (every field is a pre-existing `pub` field BIO's own importer/applier read). A non-masking arm-failure banner (`InstallScreenState::pipeline_arm_error`, surfaced in `stage_downloading::render_chrome`) now makes any residual prep/import failure diagnosable instead of a silent inert screen (the one-shot latch is kept — no per-frame re-import churn). SPEC §13.12a (per-install set + Pipeline-reuse contract) + plan P7.T17 carry the full record. This **supersedes** the prior FIX-1 (`arm_download_archive_policy`) as the actual root cause for that symptom — FIX-1 is a necessary downstream precondition (kept as-is), not the fix.
- **`ForkInfoPopup` collapse-chevron is a Phase-8 deferral, not missing scope.** SPEC §10.9 says the popup carries the global §10 collapse chevron. Like every redesign popup that gets one, the chevron is the Phase 8 carve-out #2 `.collapsible(true)` flip / `popup_collapse_anchor.rs` work — `fork_info_popup.rs` ships before Phase 8 with `.collapsible(false)` **by design**, bit-for-bit consistent with its sibling popups (`confirm_dialog.rs`).
- **§4.3 Downloading-window source/status column collision at narrow widths is a Phase-8 chassis nit.** At ~1045 px the source / status columns of the mod-progress grid collide — a Phase-5-chassis column-width responsiveness item (app min width is 1024). The live data and the two-phase Downloading / Extract model are correct (DL-Run 2 verified ALL byte-fraction + two-phase-handoff contract points); fixed-column widening already takes from flex so 1045 is clean post-DL-Run-2, but the underlying column-width responsiveness fix lives in Phase 8's polish pass.
- **Lessons — phase-local reasoning is a real failure mode.** Run 5's implementer concluded "the Install screen has no game paths" because its brief was phase-local (phase-05 + the BIO download engine) and never stated that Phase-4 Settings → Paths supplies them globally + `sync_paths_from_settings` feeds the owned `WizardState`. Fix operationalized in the orchestrator's runbook: every implementer brief now carries a standing **"Already built / cross-phase context"** block, and on verify the orchestrator sanity-checks an escalation's *premises* against the HANDOFF status table — not just its novel technical claim.
- **Cold-resume Step-2 restore (Run 2b — the #1 fix, settled).** `workspace_state_loader::populate` only *marks* the persisted `order_<tab>` onto the **currently-scanned** mod set; on a cold resume that set is empty so nothing matched (the Run-1 MISS vs P6.T1). Restore is `src/ui/workspace/step2/step2_resume_scan.rs`: on workspace open it re-points `wizard_state.step1.mods_folder` to the workspace's recorded `ModlistWorkspaceState.dev_scanned_mods_folder` and re-runs **BIO's own scan** via the existing `Step2Action::StartScan` dispatch. BIO's scan worker persists its scan cache (`save_scan_cache`); a per-tp2 `cache_get` hit skips WeiDU (`bio::app::step2::scan`, read-only), so the resume scan is WeiDU-free when the cache is fresh + tp2 files unchanged — BIO's `to_mod_states` rebuilds the full set, zero reimplementation. On the async scan-completion edge `step2_rescan_reconcile::reconcile_on_scan_complete` re-applies a snapshot built from the persisted order (preserving `selected_order`) and, for the resume case (`workspace_view.step2.resume_pending`), rebuilds Step 3 via BIO's `step3_sync::build_step3_items`. The recorded folder is written synchronously by the dev-scan trigger **before** `StartScan` (the §13.13 survive-a-crash rationale). Production records nothing (no per-install mods folder until Phase-7 P7.T17 — §13.12a), so production resume legitimately finds nothing — that is the §13.12a deferral, **not** a bug. Do not re-flag.
- **Step-4 WeiDU-line is 3 hues (RESOLVED — overview 2026-05-16; settled, do not re-litigate).** `weidu_line.rs::build_weidu_job` renders the canonical line in **three high-contrast hues per SPEC §6.7**: TP2 path amber `#d4a35c` (literal), `#0 #id` dark-blue `#2f6fb7` (literal), comment success-green `redesign_success` (theme-aware). The earlier plan-pinned neutral-grey mapping (which read 2-tone) was the drift; the user confirmed SPEC §6.7 / the wireframe `WeiduLine` as authoritative all along. `weidu_line.rs` is the correct 3-hue renderer (its `three_colour_split_matches_wireframe_hues` test asserts the literal hexes). **Scope:** SPEC §6.7 is explicitly "WeiDU log syntax coloring (**Step 3 / Step 4 only**)" + the inline Step-2 leaf encoding — it does **not** extend to the Install/fork **preview** weidu-log tabs, which the canonical wireframe `PreviewText` (`screens.jsx:512-529`) renders as a single flat `var(--text)` monospace block and SPEC §4.2 specifies only as "monospace". (This bears on Fix-Run-2 item #7b — see the run report's `SPEC CONFLICT`.)
- **OPEN PLAN GAP — per-component prompt-popup vertical growth (Fix-Run-2 #4a, NOT fixed — awaiting a directive decision).** BIO's `src/ui/step2/prompt/prompt_popup_step2.rs` `render_prompt_popup` (`PromptPopupMode::Text` branch) does `ui.set_min_size(ui.available_size())` inside its `egui::Window` — a classic egui sizing feedback loop: content min = window available ⇒ window grows ⇒ available grows ⇒ … (visibly grows as the pointer moves, egui repainting each frame). The orchestrator reuses this BIO popup verbatim (the `WizardApp::render_shared_popups` path); the loop is **entirely inside BIO's `egui::Window` closure** and independent of the C4 wrapper's rect/clip (the `ui` the wrapper passes does not affect the window's internal `available_size()`). The root cause is in **protected BIO source**; no carve-out covers a prompt-popup sizing fix; the CRITICAL DIRECTIVE forbids editing it. Per the brief's hard constraint ("ZERO BIO source edits … if a BIO edit seems needed → STOP, emit `PLAN GAP`") this was **not** fixed and is **not** worked around with a fragile orchestrator-side egui-memory clamp (non-root, fights BIO's `resizable(true)`, contrary to the brief's "root-cause + fix" intent). Awaiting a user/directive decision (e.g. authorize a 1-line BIO carve-out: `set_min_size` → a fixed/bounded size; or a sanctioned orchestrator clamp). The aggregate (toolbar) prompt popup has the same line but was explicitly out of #4a scope. See the Fix-Run-2 report `PLAN GAP`.

---

## How to implement a phase

For each remaining phase (5–8), the recommended flow is:

1. Read the phase doc at `infinity_orchestrator/plan/phase-XX-*.md` end to end.
2. Read the relevant SPEC sections (cross-referenced in the phase doc).
3. Read the matching wireframe components in `wireframe-preview/screens.jsx` directly — don't paraphrase through the spec for visual values.
4. Read the specific BIO files the phase doc cites; explore `src/core/app/` and `src/ui/` directly for codebase orientation.
5. Implement the phase's tasks in order. Strictly additive new files except where the phase explicitly authorizes a carve-out.
6. After each task, `cargo build --bin infinity_orchestrator --release` and visually verify against the wireframe.
7. Run `cargo test --lib` regularly — 116/116 should pass plus whatever new tests the phase adds.
8. End each phase with `cargo build --bin BIO --release` to confirm the legacy binary is still unaffected.

### Sample implementation-agent prompt template

If dispatching an AI agent to implement a phase:

```
Execute Phase N of the Infinity Orchestrator implementation plan. Follow the
plan and spec EXACTLY. Surface plan-vs-spec or plan-vs-source conflicts in
your final report — don't invent fixes; flag them.

## Hard rules

1. Scope = Phase N only. Nothing from later phases.
2. CRITICAL DIRECTIVE compliance. Only the 6 SPEC carve-outs. New files
   only, except where the phase doc explicitly authorizes a carve-out.
3. Phases 1–(N-1) artifacts already exist on disk. Build on them.
4. Plan ↔ source-of-truth conflict → surface, don't decide.

## Required reading

1. `infinity_orchestrator/SPEC.md` §1 CRITICAL DIRECTIVE + relevant sections
   for Phase N.
2. `infinity_orchestrator/plan/phase-NN-*.md` — full doc, your work order.
3. `infinity_orchestrator/plan/overview.md` — architecture context.
4. `infinity_orchestrator/HANDOFF.md` — current project state + caveats.
5. Phase 1–(N-1) deliverables on disk.
6. BIO source files cited by the phase doc.

## Build verification

After each task / at end of phase:

  export PATH="/opt/homebrew/opt/openjdk/bin:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
  cargo build --bin BIO --release
  cargo build --bin infinity_orchestrator --release
  cargo test --lib

Both binaries must build clean. Tests must pass.

## Output

1. Tasks completed (P_N.T# with file paths).
2. Discrepancies surfaced.
3. Build result.
4. Run result (`./target/release/infinity_orchestrator` stays alive past
   window-open with no panic).
5. Files created / modified.

Begin.
```

---

## Lessons learned (carry these forward)

These tripped us up in earlier phases; flagging so future phases can avoid them.

- **`pub mod foo;` in existing BIO `mod.rs` was initially ambiguous.** Phase 1 surfaced this as a discrepancy because the CRITICAL DIRECTIVE originally only authorized `main.rs` + `lib.rs` edits. Resolution: carve-out #3 got a "companion provision" allowing additive `pub mod` lines in existing `mod.rs` files. Phases 5–8 can use this freely for registering new sibling modules in `src/ui/orchestrator/`, `src/ui/settings/`, etc.
- **`configure_typography` wipes the redesign font config.** Phase 1's first run panicked because the orchestrator called BIO's `configure_typography` after `install_redesign_fonts`. BIO's function calls `ctx.set_fonts(FontDefinitions::default())` which replaces everything. The orchestrator now skips `configure_typography` entirely. Don't reintroduce it.
- **Plan task-numbering can drift from prose references.** When the plan says "see P7.T6" but the actual task numbered T6 covers something different, trust the prose and renumber. Agent runs caught a few of these (M4 referenced P7.T5 but the size computation lives in P7.T6).
- **Step renderer signatures aren't all symmetric.** Step 2 returns `Option<Step2Action>`, Step 3 returns `()` (no action enum — mutates `WizardState` directly), Step 4 has an action enum but the orchestrator doesn't call BIO's renderer (per C4 it uses its own orchestrator-side body), Step 5 has extra channel-receiver arguments. The plan's overview enumerates each (per M1).
- **`OrchestratorApp` needs 6 Step 2 channel receivers, not 5** (per the late-surfaced M-new-1 from review pass 3). `bio::app::app_update_cycle::poll_before_render` takes a `step2_update_extract_rx` in addition to the more obvious 5. Phase 7 P7.T1 enumerates the field set.
- **`ModlistSharePreview` needs `allow_auto_install` added too** (per M-new-2), not just `ModlistSharePayload`. Phase 5 P5.T10 documents the addition under carve-out #5.
- **Carve-out #5 now carries the provenance trio, not just `allow_auto_install`** (user-directed spec change, 2026-05-15 — overview.md revision log). The Phase-5 Run-4 BIO touch on `modlist_share.rs` is exactly: `allow_auto_install` + `name`/`author`/`forked_from` (`#[serde(default)]`) on `ModlistSharePayload` **and** `ModlistSharePreview`, a `default_true()` fn, a `ForkAncestor` struct (`#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` — the full set, so Phase 6's `ModlistEntry.forked_from` reuse needs no follow-up BIO edit), and 4 `share_preview()` propagation lines — nothing else. SPEC §1 "Modlist-share provenance application" is the exact authorized surface; the BIO-source guard must still find this the *only* BIO edit in Phase 5.
- **Share-code generation is a net-new orchestrator sibling (`registry::share_export::pack_meta`), never a BIO edit.** It composes `export_modlist_share_code` and does a standard zlib+base64url+`serde_json::Value` envelope round-trip injecting the carve-out-#5 keys plus the orchestrator-owned `archive_meta`. **This fixed a latent plan defect:** the earlier P7.T3/T6 wording ("re-decode the payload, flip the bit, re-encode") was unimplementable because BIO's envelope primitives (`base64url_*`/`zlib_*`/`decode_share_payload`) are *private*. Generation lands Phase 7 (P7.T3 install-start, P7.T6 `flip_to_installed`); fork-lineage append + `ModlistEntry.author`/`forked_from` land Phase 6 (P6.T8). Run 4 is consume-only.
- **`unrar-sys` is hostile to cross-compilation from non-Windows.** Three different toolchains (local MinGW, cross via Docker, cargo-xwin with clang-cl) hit three different errors. If a Windows build is needed before all phases are done, the realistic path is GitHub Actions running on `windows-latest` (a `.github/workflows/build-windows.yml` setup — see Windows section below).

---

## Windows builds

Native Windows is the primary development platform — `cargo build --bin infinity_orchestrator --release` runs without issue on any real Windows machine. Linux (Ubuntu CI) and macOS native builds also work.

**Cross-compilation is the only known issue.** The `unrar-sys` crate has heavy Windows-native C++ build assumptions; targeting Windows from macOS / Linux fails under MinGW (missing Win32 symbols, pthread static-vs-dynamic conflicts), `cross` (Docker MinGW header-case sensitivity on `PowrProf.h`), and `cargo-xwin` (SSSE3 intrinsics in `unrar-sys` need `-mssse3`, not propagatable via env vars). Use the native toolchain on each platform — or GitHub Actions on `windows-latest` / `ubuntu-latest` / `macos-latest` for release artifacts. The repo's CI already runs the Ubuntu native build (`cargo test --all-targets --locked`).

---

## Adversarial review history

The plan went through three adversarial review passes before implementation. All findings (5 critical + 11 high + 12 medium + ~14 low across three reviews) have been resolved or applied. Highlights worth remembering:

- **C1 — lib+bin split.** Original plan assumed `OrchestratorApp` could host `WizardApp`; reviewer found this required `pub(super)` flips not authorized by the directive. Resolution: standalone `OrchestratorApp` + lib+bin split (carve-out #3). The orchestrator and `WizardApp` are parallel `eframe::App` impls, both compiled from the same `bio` library.
- **C4 — orchestrator-side step renderers (Steps 2 / 3 / 4).** The wireframe's Step-2/3/4 chrome is structurally different from BIO's, and colour-only carve-out #6 cannot restructure BIO's `frame_step2` / `content_step3` / `content_step4` toolbars (the CRITICAL DIRECTIVE forbids editing them). Resolution: Phase 6 ships net-new orchestrator-side renderers that rebuild the wireframe chrome and reuse **only** BIO's heavy interaction sub-renderers read-only — **Step 4** (`workspace_step4`, also resolves the original "double Save button": BIO's `content_step4` paints its own `Save weidu.log's`, so the orchestrator must not call it), **Step 2** (`workspace_step2` — net-new chrome, reuses BIO's tree/details/popups), and **Step 3 — SHIPPED** (`src/ui/workspace/step3/`, P6.T2d: net-new action-row count + shared redesign GameTabs + aggregate conflict/prompt pills + redesign Undo/Redo/Collapse/Expand, **no** Export-diagnostics, **no** BIO heading; reuses BIO's drag-reorder list body `list_step3::render` + the `pub(crate)` `toolbar_support_step3` / `prompt_popup_step2` helpers `content_step3::render_toolbar` itself calls, all read-only — no behavior change). For each, BIO's `page_step{2,3,4}` / `frame_step{2,3}` / `content_step3` / `render_toolbar` are **not** called by the workspace step router; the legacy `BIO` binary still renders its own (unaffected). Step-3 returns `()` (no action enum, H2 — the dirty-bit fingerprint over `wizard_state.step3.<tab>_items` detects reorder/collapse/undo). The earlier premature Step-3 doc-cascade (retracted, overview 2026-05-17 §3) is now superseded by this **shipped** implementation + the real cascade landed with it (SPEC §7.1 / phase-06 P6.T2d / phase-08 P8.T5).
- **C5 — workspace state corruption mid-install.** If the user navigated to a different modlist while an install was running, the workspace state loader would reset `WizardState.step5`, panicking the install runtime. Resolution: rail navigation is hard-locked while an install runs.
- **`allow_auto_install` bit** (introduced by user mid-plan as a new feature). Draft / mid-install share codes have the bit `false`; auto-install is gated in the Install Modlist preview. Only `flip_to_installed` produces codes with the bit `true`. Carve-out #5 authorizes the schema addition on `ModlistSharePayload` and `ModlistSharePreview`.
- **Share-code provenance trio** (user-directed spec change, 2026-05-15). `name` / `author` / `forked_from` are packed into the payload alongside `allow_auto_install` under the same carve-out #5 (now 4 fields). Generation is the net-new `registry::share_export::pack_meta` envelope (composes BIO, zero BIO-generator edit); `forked_from` lineage is append-only so original modlist authors stay credited through forks; surfaced via the `ForkInfoPopup` (SPEC §10.9). This also resolved a latent defect where the prior P7 plan assumed BIO's *private* envelope primitives were reachable. SPEC §13.3 + §1 carve-out #5 + overview.md revision log carry the full record.
- **Phase 8 visual reskin** (M7) — originally Phase 8 aggressively pruned to only popups + console line tones, leaving Step 2/3/4 visually mismatched with the wireframe. The user authorized carve-out #6 (state-aware theme-token reads) to expand Phase 8 to cover all 24 in-scope BIO files. After Phase 8 every workspace surface visually matches the wireframe.

For the full review reports, the second-pass review is preserved at `/tmp/review2.md` (transient; may be cleaned up by the OS). The plan's `overview.md` revision log captures the high-level decisions.

---

## Finishing the plan — recommended pacing

- **Phase 5** is the next-biggest visible milestone. After it lands, Home looks real, Install Modlist's first three stages work end-to-end, modlist cards persist across launches. ~1 long agent run.
- **Phase 6** is the most complex remaining phase (workspace shell + 4 steps + Create + Load Draft). Plan on dispatching it as its own dedicated agent run with the full Phase 6 doc as the work order.
- **Phase 7** is the install-runtime phase. Substantial but the install pipeline itself is BIO's existing code — Phase 7's work is wrapping it. Expect agent run ~similar to Phase 6.
- **Phase 8** is mostly mechanical (theme-token extraction + carve-out #6 conditional swaps across ~24 files) but slow due to file count. Can be split into 8a (popups + console — carve-outs #1 + #2) and 8b (Step 2/3/5 state-aware — carve-out #6) if a single run is too long.

After Phase 8: cargo build, smoke test, ship.
