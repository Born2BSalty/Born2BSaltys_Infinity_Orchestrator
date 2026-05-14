# src/core/app/

The wizard backbone. Holds all wizard state, step flows, and the subsystems they drive (compat engine, embedded terminal, install pipeline, modlist sharing).

`mod.rs` is unusually long because most subdirectories are flattened into the `crate::app::*` namespace via `#[path = "..."]` re-declarations. Files on disk live in subdirs even though their module path is flat.

## Files at this level (orientation)

| File | Role |
|------|------|
| `dispatch.rs` | Headless `run(&AppCommandConfig)` — entry for non-GUI subcommands. |
| `normal.rs`, `eet.rs` | Implementations for `BIO normal` / `BIO eet`. |
| `app_lifecycle.rs`, `app_bootstrap_init.rs` | App-level init — settings load, fingerprint, startup checks. |
| `app_stepN_*.rs` | Per-step orchestration glue called from the UI update loop (e.g. `app_step2_scan.rs` spawns the scan worker, `app_step5_flow.rs` drives install). |
| `component_block_preview.rs`, `component_details.rs`, `selected_details.rs`, `selection_jump.rs`, `selection_refs.rs` | Read-side helpers for the Step 2/3 detail panes. |
| `prompt_eval_*.rs`, `prompt_jump_targets.rs`, `prompt_popup_*.rs` | TP2 prompt-summary evaluation + popup nav. |
| `step3_history.rs`, `step3_prompt_edit.rs`, `step3_toolbar.rs` | Step 3 reorder/resolve helpers. |
| `step4_weidu_log_export.rs` | Step 4 export to weidu-log format. |
| `step5_runtime_status.rs` | Step 5 status text formatting. |
| `mod_downloads.rs`, `modlist_config_files.rs`, `modlist_share.rs` | Download manifest + import-code (modlist sharing) logic. |
| `eet.rs` | EET-specific install logic. |

## Subdirectories

| Dir | Role | Drill-down |
|-----|------|-----------|
| `state/` | All wizard state structs. `state_wizard.rs` owns the root `WizardState`; per-step files (`state_step1.rs` … `state_step5.rs`) own each step's substate; `state_validation*.rs` validates Step 1 paths. |
| `navigation/` | `app_nav.rs` (which step is active, can advance, can go back), `app_nav_actions.rs` (handlers for Next/Back/Reset), `app_update_cycle.rs` (the per-frame poll/repaint orchestration called from the UI update loop). |
| `controller/` | `log_apply*.rs` — applying an existing `weidu.log` selection back into the scan results; `step3_sync.rs` — keeping Step 3 state in sync with Step 2 selections. |
| `compat/` | The compatibility-rule engine: scans, runtime evaluation, rule loading, popup wiring. **See `compat/CLAUDE.md`**. |
| `step2/scan/` | TP2 component scanner. `worker.rs` is the threaded entry; `parse.rs` parses TP2; `discovery.rs` walks mod folders; `cache.rs` is the on-disk scan cache (`bio_scan_cache.json`, version-bumped via `SCAN_CACHE_VERSION`). Re-exported as `crate::app::scan` and `crate::app::step2_worker`. |
| `step2/update/` | Mod auto-update: GitHub (with OAuth), Weasel Mods, Morpheus Mart fetchers + asset picking, download, archive extraction (`zip`, `7z`, `rar`, `tar.gz`). Re-exported as `crate::app::app_step2_update_*`. |
| `step5/` | Install execution: `install_flow.rs` (entry), `command_config.rs` (build command line), `auto_answer.rs` + `prompt_memory*.rs` (auto-answer prompts), `scripted*.rs` (`@wlb-inputs` parsing), `log_files*.rs` (target prep), `diagnostics*.rs` (Step 5 diagnostic exports), `readiness.rs`. |
| `terminal/` | `EmbeddedTerminal` — child process + stdout capture + prompt-detection. Public API in `mod.rs` (struct fields are `pub(super)` for sibling modules). `analyze.rs` does prompt detection; `process.rs` spawns the child; `output.rs`/`input.rs` shuttle bytes; `backend.rs` is the worker channel. |
| `modlist_pack/` | Modlist export/import: `export.rs` builds the import-code blob. |

## Wiring with the UI

`crate::app::app_step*_flow` and `crate::app::app_update_cycle` are the seams the UI calls. The UI never reaches into `step2/scan/worker.rs` or `step5/install_flow.rs` directly — it goes through these orchestration files. When adding a new background task, add a channel-based event type alongside `Step2ScanEvent`-style enums and route it through `app_update_cycle`.
