# Plan — Explicit content-addressed reproduce/install path (P8.T-EXPL)

Design pass authored 2026-06-02 (architect agent + orchestrator premise-check). **Not yet implemented.** Replaces the redesign's reuse of BIO's scan-first auto-build for share-code installs/forks with an explicit download-first path. Implement as its own run(s) from a fresh session.

## Verified problem

The redesign reuses BIO's **scan-first** auto-build: `auto_build_driver::arm_auto_build` sets `pending_saved_log_apply` + `pending_saved_log_update_preview`; BIO's `app_step2_saved_log_flow::advance_pending_saved_log_flow` gates everything behind `auto_build_preflight_blocker` → `state_validation::run_path_check`, which **requires `.tp2` files already in the per-install Mods folder**. A fresh reproduce into an empty destination fails the check ("Mods Folder has no .tp2 files within scan depth 5") → `stop_auto_build` → permanent **0/0**, before any download. (Confirmed by runtime trace 2026-06-02: `TRACE auto-build BLOCKED by preflight`.)

**Why paste seemed OK but fork stalls:** `apply_workflow_install_mode_override` forces `install_mode = EXACT_WEIDU_LOGS` for **PasteAndInstall**, which exempts it from the preflight via the `have_weidu_logs && download_archive` carve in `state_validation_paths.rs`. **Fork keeps the share's `build_from_scanned_mods`**, gets no exemption, and stalls. Both should be download-first.

## Key constraint that shapes the design (premise-checked)

**Download-URL resolution lives ONLY inside BIO's update-check worker** (`check_latest_release_for_worker` — github / weaselmods / morpheus / page-archive). The share code's `archive_meta` carries **hashes/sizes only, never URLs** — for both old and new codes. So the explicit path **must still run the update-check worker as the resolver**; it must only stop running (a) the scan-first **preflight** (`run_path_check` `.tp2` gate) and (b) the install-mode workaround. The resolver does **not** need a populated scan: `apply_saved_weidu_log_selection` → `build_log_pending_downloads` derives pending downloads purely from the imported `weidu.log`, and `preview_update_selected` builds the update-check requests from those.

**#38 / carve-out #15 (`reproduce_exact` gate) is load-bearing — MERGE it as a prerequisite.** For a reproduce code every resolved ref equals the pinned ref, so `apply_successful_update_check_outcome` would drop every asset at the `source_ref_matches` early-return — leaving the asset list empty — unless `reproduce_exact_gate` fires. The explicit arm sets `reproduce_exact = true`, so the gate keeps the assets. The explicit path does **not** make #38 obsolete (the resolver, hence the drop, still runs); it makes the **scan-first preflight** + the `EXACT_WEIDU_LOGS` hack obsolete.

## The explicit reproduce path

Branch-off: the Install click (paste → `InstallStage::Downloading` → `stage_downloading::render_live`) and fork "Begin Import →" (→ `fork_pipeline_arm::mint_and_arm` → `stage_fork_download::render_live`). Replace the **arming recipe**, not the screens.

1. **Destination prep** — unchanged (`destination_prep::spawn_prepare_destination_worker`).
2. **Import** — `prepare_install_dirs_and_maybe_import`: `derive_per_install_dirs` + `import_modlist_share_code` (writes the baked `weidu.log` + `mod_downloads_user.toml` + pinned `installed_refs` into the per-install dirs). **Change:** call a net-new `arm_explicit_reproduce` instead of `arm_auto_build`; retire `apply_workflow_install_mode_override`.
3. **Resolve (no preflight, no scan-first gate)** — net-new `drive_explicit_resolve`: call `apply_saved_weidu_log_selection` then `preview_update_selected` **directly** (one-shot, latched), installing `step2_update_check_rx`. Do **NOT** set `pending_saved_log_apply` / `pending_saved_log_update_preview` (those route through the scan-first preflight). `advance_pending_saved_log_flow` therefore stays inert for the pre-download phase.
4. **Update-check completes → asset list populated.** `poll_step2_update_check` drains outcomes; `reproduce_exact_gate` (#38) keeps the matching-ref assets.
5. **Hash → skip → download → verify → extract → ingest** — the existing latched calls fire unchanged: `stage_and_kick_archive_skip_once`, `kick_streaming_downloader_once`, `verify_downloaded_archives_once`, `start_parallel_extract`, `ingest_downloaded_archives_once`.
6. **Hand-off** — paste → install step (Step 5); fork → Workspace Step 2. The post-extract scan+apply (`handle_extract_finished` already sets `pending_saved_log_apply` after `extracted>0`, when the Mods folder now has `.tp2`) prepares the WeiDU install. Unchanged.

## Per-path behavior (the 5 routes)

| # | Path | Resolution | Skip input | End-state |
|---|------|-----------|------------|-----------|
| 1 | old code → install paste | update-check worker (from imported log) | `decode_archive_meta`→`[]` ⇒ fetch all | extract → install step |
| 2 | old code → create fork | same | `[]` ⇒ fetch all | extract → Workspace Step 2 |
| 3 | new code → install paste | same worker (URLs still from worker) | `archive_meta` hashes ⇒ skip-by-content | extract → install step |
| 4 | new code → create fork | same | hashes ⇒ skip | extract → Workspace Step 2 |
| 5 | create from scratch | none (no code) | n/a | straight to Workspace Step 2, empty selection, **no download** (`is_share_code_consuming(FreshCreate)==false`) |

## Files to change (net-new orchestrator; NO new carve-out — builds on #15)

- `src/install_runtime/auto_build_driver.rs` — add `arm_explicit_reproduce` + `drive_explicit_resolve`; route `prepare_install_dirs_and_maybe_import` to it; retire `apply_workflow_install_mode_override`.
- `src/ui/install/stage_downloading.rs` — latched `kick_explicit_resolve_once` between arm and `stage_and_kick_archive_skip_once`.
- `src/ui/create/stage_fork_download.rs` — same insertion; keep the existing post-extract flag-clear + Step-2 landing.
- `src/ui/orchestrator/orchestrator_app.rs` — receiver lifecycle (reuse existing `step2_update_check_rx`).
- **BIO reused read-only (no carve-out, direct-reuse):** `apply_saved_weidu_log_selection`, `preview_update_selected`, `poll_step2_update_check`, the update-check worker, `import_modlist_share_code`, `archive_file_name`, `sync_weidu_log_mode`.
- **SPEC §13.12a doc-sync:** the explicit download-first arm replaces the scan-first reuse for the redesign install/fork path; scan happens *after* extract for the install hand-off; the `reproduce_exact` bypass note stays.

## Risks (carry into implementation)

- **R1** Don't regress `BIO_legacy`'s `page_step1::start_modlist_auto_build` (shares `advance_pending_saved_log_flow`, scan-first). Touch only the orchestrator arm; leave the BIO flow intact.
- **R2 (trickiest)** The post-extract re-arm sets `pending_saved_log_apply`, re-entering `advance_pending_saved_log_flow` + its preflight — which now passes (Mods has `.tp2`). Mirror the fork path's existing flag-clear for the install path too; consider clearing `modlist_auto_build_active` at hand-off.
- **R3** DATA-LOSS: per-install routing (§13.12b) for `mod_downloads_user.toml`; tests temp-pathed; sentinel = `modlists.json` + seed `workspace.json` + `mod_installed_refs.toml`.
- **R4** Empty-asset clean-finish (no stall) for the install completion predicate (fork's `fork_extract_complete` already handles empty).
- **R5** After retiring `EXACT_WEIDU_LOGS`, confirm install runs with the share's declared mode for all 4 code paths.
- **R6** Resolver network failures surface via the arm-error/failed-source banners, not a stall.

## Tasks

- **Pre-req:** merge PR #38 (carve-out #15 — the `reproduce_exact` gate the explicit arm depends on).
- **T-EXPL.1** — `arm_explicit_reproduce` + `drive_explicit_resolve`; retire `EXACT_WEIDU_LOGS`; route both `prepare_install_dirs_and_maybe_import` callers.
- **T-EXPL.2** — wire `kick_explicit_resolve_once` into `stage_downloading` + `stage_fork_download`; keep `advance_pending_saved_log_flow` inert pre-download; post-extract re-arm/flag-clear for the install path (R2).
- **T-EXPL.3** — empty-asset clean-finish + failed-source surfacing (R4, R6).
- **T-EXPL.4** — SPEC §13.12a doc-sync.

## Verification (every path)

5-path live matrix (each: empty dest, run, expect non-0/0 → extract → correct hand-off; #5 expects no download). Plus: BIO-source guard stages only redesign-owned files; `cargo test --lib` hermetic; both binaries no-op rebuild; DATA-LOSS sentinel byte-identical (incl. `mod_installed_refs.toml`); `BIO_legacy` import-auto-build unchanged.
