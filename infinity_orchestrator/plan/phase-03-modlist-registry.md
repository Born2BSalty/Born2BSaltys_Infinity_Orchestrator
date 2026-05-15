# Phase 3 — Modlist registry + per-modlist workspace state files

## Summary

Add the data layer for the multi-modlist redesign: a `modlists.json` registry indexing every in-progress and installed modlist, and a per-modlist `modlists/<id>/workspace.json` file holding that modlist's workspace state (order arrays, checked components, expand state, prompt overrides, last share code). Wire both into a new orchestrator-owned persistence cycle (mirroring the shape of BIO's existing `app_update_cycle::persist_step1_if_needed` pattern) with debounce + on-exit flush via `eframe::App::on_exit` (primary hook) + `Drop` fallback (per H4). Surface a terminal error UI for corrupt/missing registry or workspace files per SPEC §13.14. No screen reads or writes the registry yet — that arrives in Phase 5 (Home) and Phase 6 (Workspace).

## What ships after this phase

**Framing note (per L4).** Phase 3 is a foundational phase. The binary compiles and runs but Phase 3 alone does not produce user-facing value — that arrives in Phase 5 (Home reads the registry; cards render real entries). Phase 3's exit criteria is "registry plumbing works end-to-end via the dev seed button." The phasing philosophy at the top of `overview.md` continues to apply (each phase must leave the binary in an alpha-shippable state — Phase 3 ships a binary that compiles, launches, and exercises the registry path via dev-only UI; it just doesn't yet expose the registry through production-facing screens).

- `cargo build --bin infinity_orchestrator --release` succeeds.
- App launches; on first run, no `modlists.json` exists yet — the app continues normally (registry treated as empty).
- A dev-mode-only Home stub action `Seed test modlist (dev)` button is added that creates a fake registry entry + workspace file, demonstrating round-trip persistence. After restart, the file is still on disk.
- Manually corrupting `modlists.json` (e.g., writing `{ not json` to the file via terminal) results in a terminal error screen on next launch: a full-pane error panel reading `modlist registry is corrupt or unreadable` + the path + the parse failure + a non-actionable `Restore from backup or delete the file to continue` line. No silent recovery.
- The orchestrator's persistence cycle writes registry + per-modlist files at a debounce cadence comparable to BIO's existing Step 1 settings cycle (~1s debounce, flush on exit via `eframe::App::on_exit` + `Drop` fallback).
- `cargo build --bin BIO --release` is unaffected.

## What's still missing

- Any UI that reads or writes the registry as part of normal flows (Phases 5-7).
- State transitions (`in-progress` → `installed`) — Phase 7.
- The `modlist-import-code.txt` write-on-install-start — Phase 7.
- Reinstall flow that updates the registry — Phase 7.

## Dependencies

- Phase 1 (carve-out #3 split + theme + shell modules + new binary).
- Phase 2 (`OrchestratorApp` struct — the registry fields hang off this struct; the dev seed button hooks into `stubs::render_home_stub`).

## File inventory

### New files

| Path | Purpose | Depends on |
|------|---------|-----------|
| `src/registry/mod.rs` | `pub mod model; pub mod store; pub mod store_workspace; pub mod ids; pub mod errors; pub mod workspace_model; pub mod persistence_cycle; pub mod dev_seed; pub mod operations;` (`operations` is populated in Phase 5; declared here for module visibility). Registered in `src/lib.rs` via `pub mod registry;`. | — |
| `src/registry/model.rs` | Serde structs `ModlistRegistry`, `ModlistEntry`, `ModlistState (InProgress \| Installed)`. Fields per SPEC §13.1: `id`, `name`, `game (Game enum: BGEE/BG2EE/IWDEE/EET)`, `destination_folder`, `state`, `creation_date`, `last_touched_date`, `install_date: Option`, `last_played_date: Option`, `mod_count`, `component_count`, `paused_at_step: Option<u8>` (denormalized workspace step for the in-progress Home card meta line — `None` for installed entries; kept in sync by the Phase 6 workspace-persistence cycle), `total_size_bytes: Option<u64>`, `latest_share_code: Option<String>`, `workspace_file_relpath: PathBuf`. All `#[serde(default)]` so adding fields stays backward-compat. | serde |
| `src/registry/ids.rs` | `pub fn new_modlist_id() -> String` — stable slug-safe ID. Recommended: a 12-character base32 ULID for sortability + readability. | — |
| `src/registry/errors.rs` | `pub enum RegistryError { Io(io::Error), Parse(serde_json::Error), Corrupt { path, message } }`. Implements `Display + Error`. | serde_json |
| `src/registry/store.rs` | `pub struct RegistryStore { path: PathBuf }` with `pub fn new_default() -> Self`, `pub fn load(&self) -> Result<ModlistRegistry, RegistryError>`, `pub fn save(&self, &ModlistRegistry) -> Result<(), RegistryError>`. Mirrors the shape of `bio::settings::store::SettingsStore` (read it as reference; do not modify it). Uses `bio::platform_defaults::app_config_file("modlists.json", ".")` for the path. | `bio::platform_defaults::app_config_file` |
| `src/registry/workspace_model.rs` | Serde struct `ModlistWorkspaceState`: `order_bgee: Vec<ComponentRef>`, `order_bg2ee: Vec<ComponentRef>`, `order_iwdee: Vec<ComponentRef>`, `expand_state: HashMap<String, bool>` (keyed by `<tab>:<tp2>` and `<tab>:<tp2>:<parent>`), `step3_group_collapse: HashMap<String, bool>`, `prompt_overrides: HashMap<String, PromptOverride>`, `last_share_code: Option<String>`. `ComponentRef { tp2: String, id: i64, language: u8 }`. Designed to be **populated from and written back to** the orchestrator's owned `WizardState` (no `WizardState` modification; the loader reads/writes its `pub` fields). | serde |
| `src/registry/store_workspace.rs` | `pub struct WorkspaceStore` with `pub fn new_for_id(modlist_id: &str) -> Self`, `pub fn load(&self) -> Result<ModlistWorkspaceState, RegistryError>`, `pub fn save(&self, &ModlistWorkspaceState) -> Result<(), RegistryError>`. Writes to `<config_dir>/modlists/<id>/workspace.json`. | `errors`, `workspace_model` |
| `src/registry/persistence_cycle.rs` | The orchestrator-owned persistence cycle. `pub struct RegistryPersistenceCycle { last_saved_registry, last_saved_workspaces, debounce_ms, last_dirty_at }`. `pub fn persist_registry_if_needed`, `pub fn persist_workspace_if_needed`, `pub fn flush_all`. Modeled after BIO's existing `bio::app::app_update_cycle::persist_step1_if_needed` pattern (read for reference; do not modify). | std time |
| `src/registry/dev_seed.rs` | `pub fn seed_demo_entry(store: &RegistryStore, ws_store_factory: impl Fn(&str) -> WorkspaceStore) -> Result<ModlistEntry, RegistryError>`. Creates a `ModlistEntry` in `in-progress` state with fake metadata and writes a default `ModlistWorkspaceState`. Used by the Home stub's dev button. | `store`, `store_workspace`, `workspace_model` |
| `src/ui/orchestrator/registry_error_panel.rs` | `pub fn render_registry_error(ui, err: &RegistryError)` — full-pane terminal error UI per SPEC §13.14. Shows the file path in monospace, the error class, and the actionable hint. No "ignore and continue" button. | redesign theme tokens |

### BIO files read from / consumed (no modifications)

- `bio::platform_defaults::app_config_file` — used to resolve the registry path. No modification.
- `bio::app::app_update_cycle` — read as a reference for the debounce / flush pattern; **the orchestrator's persistence cycle is a new sibling module, not an extension of this one.** No modification to the existing cycle.
- `bio::settings::store::SettingsStore` — read as a reference for the load/save pattern. No modification.

### BIO files needing allowed mild refactor

**None.** Phase 3 is entirely additive — new modules + new files only. No edits to BIO source.

The previous plan's "Phase 3 extends the existing persist cycle" wording is removed; the orchestrator owns its own `RegistryPersistenceCycle` in a new module. Each cycle (BIO's for `bio_settings.json`, the orchestrator's for `modlists.json` + workspace files) is independent.

## Implementation tasks

### P3.T1 — Define `ModlistRegistry` + `ModlistEntry` + `ModlistState`

- **What:** Create `src/registry/model.rs` with:
  ```rust
  pub struct ModlistRegistry { pub format_version: u32, pub entries: Vec<ModlistEntry> }
  pub struct ModlistEntry { id, name, game, destination_folder, state, creation_date, last_touched_date, install_date, last_played_date, mod_count, component_count, paused_at_step, total_size_bytes, latest_share_code, workspace_file_relpath }
  pub enum ModlistState { InProgress, Installed }
  pub enum Game { BGEE, BG2EE, IWDEE, EET }
  ```
  Use `chrono::DateTime<Utc>` for timestamps (already in `Cargo.toml` as `chrono = "0.4"`). `format_version` defaults to `1`.
- **Where:** Create new file.
- **Acceptance:** A round-trip `let r = ModlistRegistry::default(); let s = serde_json::to_string(&r)?; let r2: ModlistRegistry = serde_json::from_str(&s)?;` produces an equal value. Adding a new field with `#[serde(default)]` does not break parsing of an older file (verified by a test that strips a field from JSON and re-parses).
- **SPEC:** §13.1, §15.

### P3.T2 — `RegistryStore` load/save

- **What:** Implement `src/registry/store.rs` mirroring `SettingsStore`'s shape. `load()` returns `Ok(empty registry)` when the file does not exist (first-launch scenario); returns `Err(RegistryError::Corrupt {…})` when the file exists but cannot be parsed. `save()` writes pretty-printed JSON and creates parent dirs if needed.
- **IO error handling.** Beyond `Corrupt` (parse failure) and missing-file (returns empty registry), any other `std::io::Error` (permission denied, disk full, file locked by another process, etc.) propagates as a new `RegistryError::Io(io::Error)` variant and surfaces in the same terminal-error panel as `Corrupt`. Same UX as a corrupt file: app continues running with the registry in an unloaded state; user is told the file path + error and must fix the issue.
- **Where:** Create new file.
- **Acceptance:** Unit tests cover: (a) first-launch returns empty, (b) round-trip preserves entries, (c) corrupting the file then loading returns `RegistryError::Corrupt` with the path and the parse error message, (d) an IO error (e.g., simulated permission-denied) returns `RegistryError::Io` with the wrapped error.
- **SPEC:** §13.1, §13.14.

### P3.T3 — Per-modlist workspace model + store

- **What:** Implement `src/registry/workspace_model.rs` and `src/registry/store_workspace.rs`. `WorkspaceStore::load()` for a non-existent file returns `Err(RegistryError::Corrupt)` if the registry entry says the workspace exists. Decision tree: if the registry has no entry for this id, the workspace store should not be queried (caller bug). If the registry has the entry but the workspace file is missing or corrupt, return `Err` (per SPEC §13.14 "Corrupt / missing state files — terminal error policy").
- **Where:** Create new files.
- **Acceptance:** Unit tests: (a) round-trip of a populated `ModlistWorkspaceState`, (b) loading a missing file when expected returns `Corrupt` error variant, (c) saving creates the `modlists/<id>/` subdirectory automatically.
- **SPEC:** §13.1, §13.14.

### P3.T4 — Atomic registry writes

- **What:** `RegistryStore::save` writes to `modlists.json.tmp` then renames to `modlists.json` (POSIX atomic rename on the same filesystem; Windows uses `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING`). This guarantees the registry is never observed in a half-written state by another reader.
- **Where:** Inside `src/registry/store.rs::save`.
- **Acceptance:** Unit test that kills a save mid-flight (via simulated panic) and verifies the original file is still readable. Acceptable if the test merely confirms the temp-file pattern is used.
- **SPEC:** §13.14 ("Modlist registry writes are individually atomic and don't queue").

### P3.T5 — `RegistryError` → terminal UI

- **What:** `src/ui/orchestrator/registry_error_panel.rs::render_registry_error` paints the full main pane (called from `page_router::render` when `OrchestratorApp::registry_error` is `Some`). Renders the error path + parse class + non-actionable hint per SPEC §13.14. **Crucially: no Retry, no Reset, no Continue button.**
- **Where:** Create new file. Wire into `page_router::render` as a top-level early-return: if `orchestrator.registry_error.is_some()`, render the error panel and skip the destination dispatch.
- **Acceptance:** Manually corrupt `modlists.json` (write garbage), restart the app: the error panel appears. The left rail and statusbar still render; only the main pane shows the error.
- **SPEC:** §13.14.

### P3.T6 — Orchestrator-owned persistence cycle

- **What:** Create `src/registry/persistence_cycle.rs` with:
  - `pub struct RegistryPersistenceCycle { last_saved_registry: ModlistRegistry, last_saved_workspaces: HashMap<String, ModlistWorkspaceState>, debounce_ms: u64, last_dirty_at: HashMap<String, Instant> }`
  - `pub fn persist_registry_if_needed(in_memory: &ModlistRegistry, store: &RegistryStore, last_saved: &mut ModlistRegistry, last_dirty_at: &mut Instant)` — analogous in shape to `app_update_cycle::persist_step1_if_needed`. Compares in-memory to last saved; if different and the debounce has elapsed, writes.
  - `pub fn persist_workspace_if_needed(modlist_id: &str, in_memory: &ModlistWorkspaceState, store: &WorkspaceStore, last_saved: &mut HashMap<String, ModlistWorkspaceState>, last_dirty_at: &mut HashMap<String, Instant>)` — same pattern per-modlist.
  - `pub fn flush_all(in_memory_registry, registry_store, in_memory_workspaces, ws_stores)` — synchronous full flush; called from `OrchestratorApp::on_exit` (primary hook per H4) and `Drop for OrchestratorApp` (fallback).

  **No explicit write guard.** Per H6 resolution, the plan does not introduce a `RegistryWriteGuard` type. egui is single-threaded — `OrchestratorApp::update` runs on one thread, and Rust's borrow checker already enforces that only one `&mut ModlistRegistry` exists at a time. Atomic file writes (P3.T4) handle partial-write safety on disk. The registry's mutating operations (`create_modlist`, `rename_modlist`, `delete_modlist`, `flip_to_in_progress`, `flip_to_installed`, plus future state transitions) are implemented as plain `pub fn` functions that take `&mut ModlistRegistry` + `&RegistryStore`, mutate the in-memory registry, and immediately call `store.save(&registry)` atomically. No lock, no guard, no transaction wrapper — the type system + atomic writes are sufficient.

- **Where:** Create new file.
- **Acceptance:** Unit tests for each function. Bench: dev seeding doesn't write to disk more than once per debounce window.
- **SPEC:** §13.14.

### P3.T7 — Wire registry load on app start + flush on exit (H4: `on_exit` primary, `Drop` fallback)

- **What:** Extend `OrchestratorApp::new` (Phase 2 file) to:
  1. Construct `RegistryStore::new_default()`.
  2. Call `store.load()`. On error: store the `RegistryError` in a new `OrchestratorApp::registry_error: Option<RegistryError>` field and skip remaining init.
  3. On success: store the `ModlistRegistry` in a new `OrchestratorApp::registry: ModlistRegistry` field.
  4. Construct `RegistryPersistenceCycle::new()` and store on the app.

  **H4 — persistence-on-exit hooks (both, in order):**

  - **Primary:** Implement `eframe::App::on_exit(&mut self, _gl: Option<&eframe::glow::Context>)` on `OrchestratorApp`. Body calls `persistence_cycle::flush_all(...)`. This is the canonical eframe shutdown hook and is called before the app's `Drop` runs. eframe guarantees `on_exit` runs on the main thread with full access to the app.
  - **Fallback:** Implement `impl Drop for OrchestratorApp` that calls `persistence_cycle::flush_all(...)` defensively. This catches edge cases where `on_exit` is bypassed (e.g., panic-unwind, hard exit). The `Drop` impl is idempotent: `flush_all` no-ops when in-memory == last-saved.

  Both hooks call the same `flush_all` entry point. The Drop fallback handles the unusual paths; `on_exit` handles the common ones.

- **Where:** Edit `src/ui/orchestrator/orchestrator_app.rs` (Phase 2 new file — editable).
- **Acceptance:** Clean registry on disk loads correctly; corrupting it triggers the error panel. Closing the app via the window close button flushes pending writes via `on_exit`. Killing the app process via `SIGTERM` (Unix) or task manager (Windows) — depending on platform — flushes via the `Drop` fallback where possible; if neither hook fires (SIGKILL), the next launch picks up the last successfully-flushed state (debounce-bounded data loss only).
- **SPEC:** §13.1, §13.14.

### P3.T8 — Dev-only seed button in Home stub

- **What:** In `src/ui/orchestrator/stubs.rs::render_home_stub`, when `dev_mode` is true, add a `Seed test modlist (dev)` `redesign_btn` below the existing `Open workspace stub (dev)` button. On click, it calls `bio::registry::dev_seed::seed_demo_entry(...)` which inserts a fake entry into the registry and a corresponding workspace file. Show a toast `Seeded "demo-modlist-<n>"` on success (a minimal toast helper can live inline in Phase 3; Phase 5 introduces the shared toast widget).
- **Where:** Edit `src/ui/orchestrator/stubs.rs` (Phase 2 new file). Wire the registry handle from `OrchestratorApp` via a function argument.
- **Acceptance:** Clicking the button in dev mode creates a new entry. Restarting the app keeps the entry (registry persists to disk via `on_exit`).
- **SPEC:** §13.1 (registry CRUD; this is just exercising it without UI yet).

### P3.T9 — Statusbar `<N> modlists` reads from registry

- **What:** `src/ui/shell/shell_statusbar.rs::render` already accepts a `modlist_count` arg from Phase 1. In Phase 3, `OrchestratorApp::update` computes `self.registry.entries.len()` and passes it. The statusbar reflects the count immediately after seeding a modlist via the dev button.
- **Where:** Edit `src/ui/orchestrator/orchestrator_app.rs` (Phase 2 new file).
- **Acceptance:** Statusbar shows `0 modlists` on first launch; clicking Seed bumps it to `1 modlists`.
- **SPEC:** §1.2 (statusbar segments), §13.1 (registry as source of truth).

### P3.T10 — Backup of corrupt registry file on detect

- **What:** Provide a `pub fn backup_corrupt_file(&self) -> io::Result<PathBuf>` on `RegistryStore` that renames the on-disk `modlists.json` to `modlists.json.corrupt-<unix-timestamp>` and returns the new path. **The rename is a deliberate side-effect, kept out of `load`** so that `load` itself stays pure (read-only) — useful for tests and for repeated load calls without escalating side-effects. The function is **explicitly invoked by `OrchestratorApp::new()` on the error path** when `load()` returns `RegistryError::Corrupt` (or `RegistryError::Io` per M9), and the returned backup path is included in the terminal error UI's hint line. This is a safety net — the SPEC's terminal-error policy still applies (no silent recovery in the live app).
- **Where:** New function in `src/registry/store.rs::backup_corrupt_file`. `load` itself remains a pure read-only function: it returns `Ok` / `Err` and never renames or otherwise mutates the on-disk file.
- **Acceptance:** A corrupt file remains untouched after `load()` alone (`load` is pure); calling `backup_corrupt_file()` renames it alongside the original location with a `.corrupt-<unix-timestamp>` suffix. The terminal error UI's hint line mentions the backup location. `OrchestratorApp::new` orchestrates: call `load()`; on `Corrupt` or `Io`, call `backup_corrupt_file()` and stash the result for the error panel; surface the error to the UI.
- **SPEC:** §13.14 (no silent recovery; this is a non-destructive read-side safety net, consistent with the policy).

## Open questions / risks

- **ID strategy.** SPEC §13.1 specifies `id` exists but does not fix its format. Recommended: a 12-character base32 ULID for human-readability + sortability. Alternative: uuidv4. Either is acceptable as long as it's stable. Open for team confirmation.
- **`total_size_bytes` computation.** SPEC §3.2 shows `47 mods · 2.3 GB · installed 2 days ago`. Total size requires a recursive `du` on the install folder, which is non-trivial on Windows. Compute it post-install in Phase 7 and update the registry entry then; for now, leave as `None` and skip the size segment.
- **Schema migration.** SPEC §13.1 says "No migration — existing BIO users start fresh; the old single-workspace state in `bio_settings.json` is not auto-converted into a registry entry." On first launch of the orchestrator, an existing BIO user's `bio_settings.json` is untouched and the registry is empty. Their previous wizard state is still reachable via the legacy `BIO` binary; the orchestrator simply does not import it as a registry entry. Document this in `BEGINNERS_GUIDE.md` in Phase 8 polish if needed.
- **`WizardState` field paths used by the workspace loader.** `ModlistWorkspaceState`'s fields (order arrays, expand state, prompt overrides) map directly to `WizardState::step2`, `step3`, and `step5` fields. Verify the field names during Phase 6 implementation; if BIO's struct layout has changed since the spec was written, adjust the workspace model fields to match (the workspace model is new, so additive renames are fine). Per Phase 6 verification, the canonical Step 3 order fields are `state.step3.bgee_items` and `state.step3.bg2ee_items` (each a `Vec<Step3ItemState>`, both `pub` per `src/core/app/state/state_step3.rs:23-24`).

## Verification

1. `cargo build --bin infinity_orchestrator --release` succeeds.
2. `cargo test --lib registry` passes (unit tests for round-trip, error variants, atomic rename).
3. Launch the app; statusbar shows `0 modlists`.
4. In dev mode, click `Seed test modlist (dev)` from Home stub: statusbar bumps to `1 modlists`. Inspect `<config_dir>/modlists.json` and `<config_dir>/modlists/<id>/workspace.json` — both exist and are valid JSON.
5. Kill the app via the window close button (`on_exit` path); pending writes flush. Relaunch: seeded entry is still present.
6. Kill the app, corrupt `modlists.json` by writing `{ bad`, relaunch: the terminal error panel renders.
7. Restore the file (delete or fix manually), relaunch: app comes up normally.
8. `cargo build --bin BIO --release` continues to succeed and the legacy wizard launches unchanged.
