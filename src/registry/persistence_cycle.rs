// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `RegistryPersistenceCycle` — per-frame debounce cycle for the modlist
// registry + per-modlist workspace files.
//
// Per Phase 3 P3.T6 + H4 + H6:
//   - Mirrors the shape of BIO's `bio::app::app_update_cycle::persist_step1_if_needed`
//     (read for reference; not modified).
//   - `last_saved_registry` snapshots the last successfully-written registry;
//     compared each frame to the in-memory one to decide whether to save.
//   - `last_dirty_at` tracks per-key dirty timestamps so writes wait until
//     `debounce_ms` of idle time has passed (default 1s — matches BIO's
//     Step 1 cycle).
//   - `flush_all` is called from `eframe::App::on_exit` (primary) and
//     `Drop for OrchestratorApp` (fallback). Both call sites are idempotent —
//     if nothing changed, `flush_all` no-ops.
//
// **No RegistryWriteGuard.** Per H6: egui is single-threaded; Rust's borrow
// checker enforces single-mutator. Atomic file writes (P3.T4) handle disk
// safety. Mutating helpers (`create_modlist`, `rename_modlist`, etc.) will
// live in `operations.rs` in Phase 5; each takes `&mut ModlistRegistry +
// &RegistryStore` and saves atomically.
//
// **P6.T11 — dirty-bit-gated workspace cadence (this run).** The per-modlist
// `workspace.json` debounce (`persist_workspace_if_needed`, Phase 3) already
// debounces a write once a workspace's in-memory state differs from the last
// saved snapshot. What Run 4 adds is the **gating** so the live `WizardState`
// → `ModlistWorkspaceState` extract is **only** performed when an explicit
// dirty bit is set — never per frame (review finding **H1**: a per-frame
// `extract_workspace_state_from_wizard` + full compare on every idle frame is
// the cost H1 forbids). The dirty bit + the extract live on the orchestrator
// (`OrchestratorApp::workspace_state_dirty` + `mark_workspace_dirty`, Run 1;
// the extract is `workspace_state_loader::extract_workspace_state_from_wizard`
// — a UI-layer module the registry layer must not import). This file owns
// the cadence + a **debug counter** (`workspace_extract_debug_count`) the
// orchestrator bumps each time it actually performs the gated extract — so
// the H1 "zero idle cost" property is *observable*: an idle workspace leaves
// it flat (the P6.T11 acceptance check). Mutating call sites that set the
// dirty bit: `step_action_dispatch::dispatch_step2`/`dispatch_step4` (the
// mutating Step 2/4 variants — Run 1) and the Step-3 fingerprint detector in
// `workspace_step_router` (Step 3 has no action enum — H2). Rename sets the
// *registry* dirty bit, never this one.
//
// SPEC: §13.14.

// rationale: `#[must_use]` on a trivial query is churn (Cat 3); the test-only
// `Default::default()` + single-field reassign cannot be a struct literal
// (the struct has private fields), so the field-reassign lint is suppressed
// rather than restructured (Cat 3).
#![allow(clippy::must_use_candidate, clippy::field_reassign_with_default)]

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;
use crate::registry::store::RegistryStore;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;

/// Default debounce window. Mirrors BIO's Step 1 settings-persistence cadence.
pub const DEFAULT_DEBOUNCE_MS: u64 = 1000;

/// The registry-side debounce key. The workspace cycle is keyed per modlist id.
const REGISTRY_KEY: &str = "::registry::";

#[derive(Debug)]
pub struct RegistryPersistenceCycle {
    /// Snapshot of the last successfully-written registry; comparison drives
    /// whether the next frame needs to save.
    pub last_saved_registry: ModlistRegistry,
    /// Per-modlist snapshots of the last successfully-written workspace
    /// state. Keyed by modlist id.
    pub last_saved_workspaces: HashMap<String, ModlistWorkspaceState>,
    /// Idle-time threshold before flushing. Defaults to `DEFAULT_DEBOUNCE_MS`.
    pub debounce_ms: u64,
    /// Per-key dirty timestamps. Key `"::registry::"` for the registry;
    /// `<modlist_id>` for each workspace.
    pub last_dirty_at: HashMap<String, Instant>,
    /// **P6.T11 H1 observability counter.** Incremented (via
    /// [`RegistryPersistenceCycle::note_workspace_extract`]) by the
    /// orchestrator every time it actually performs the dirty-gated
    /// `WizardState` → `ModlistWorkspaceState` extract+compare. It must stay
    /// **flat while a workspace is idle** — that is the H1 "zero per-frame
    /// extract overhead" property the P6.T11 acceptance verifies (the dirty
    /// bit is `false` on an idle frame, so the orchestrator skips the
    /// extract entirely and this never increments). Not persisted; purely a
    /// runtime diagnostic.
    pub workspace_extract_debug_count: u64,
}

impl Default for RegistryPersistenceCycle {
    fn default() -> Self {
        Self {
            last_saved_registry: ModlistRegistry::default(),
            last_saved_workspaces: HashMap::new(),
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            last_dirty_at: HashMap::new(),
            workspace_extract_debug_count: 0,
        }
    }
}

impl RegistryPersistenceCycle {
    /// Construct a cycle initialized with the just-loaded registry baseline.
    /// `last_saved_registry` is set to the loaded value so the first frame
    /// after load doesn't immediately write.
    pub fn new_with_baseline(loaded: ModlistRegistry) -> Self {
        Self {
            last_saved_registry: loaded,
            ..Self::default()
        }
    }

    /// Mark the registry dirty (a write is desired). Records `now` so the
    /// next call to `persist_registry_if_needed` after the debounce window
    /// elapses will actually save.
    pub fn mark_registry_dirty(&mut self, now: Instant) {
        self.last_dirty_at.insert(REGISTRY_KEY.to_string(), now);
    }

    /// Mark a single workspace dirty by id.
    pub fn mark_workspace_dirty(&mut self, modlist_id: &str, now: Instant) {
        self.last_dirty_at.insert(modlist_id.to_string(), now);
    }

    /// Record that the orchestrator performed one dirty-gated
    /// `WizardState` → `ModlistWorkspaceState` extract+compare (P6.T11 / H1).
    /// Bumps [`Self::workspace_extract_debug_count`]. Called by the
    /// orchestrator **only on a frame where `workspace_state_dirty` was
    /// `true`** — so the counter staying flat across idle frames is the
    /// observable proof of the H1 "no per-frame extract" property.
    pub fn note_workspace_extract(&mut self) {
        self.workspace_extract_debug_count = self.workspace_extract_debug_count.saturating_add(1);
    }

    /// Persist the registry if (a) the in-memory copy differs from the last
    /// saved snapshot, and (b) `debounce_ms` of idle time has passed since
    /// the last dirty mark.
    ///
    /// On success: updates `last_saved_registry` and clears the dirty mark.
    /// On error: returns the error and leaves state unchanged — the caller
    /// (orchestrator) can surface the error to the user.
    pub fn persist_registry_if_needed(
        &mut self,
        in_memory: &ModlistRegistry,
        store: &RegistryStore,
        now: Instant,
    ) -> Result<bool, RegistryError> {
        if in_memory == &self.last_saved_registry {
            return Ok(false);
        }
        if !self.is_debounce_elapsed(REGISTRY_KEY, now) {
            return Ok(false);
        }
        store.save(in_memory)?;
        self.last_saved_registry = in_memory.clone();
        self.last_dirty_at.remove(REGISTRY_KEY);
        Ok(true)
    }

    /// Persist a single workspace file if it has changed and the debounce
    /// has elapsed.
    pub fn persist_workspace_if_needed(
        &mut self,
        modlist_id: &str,
        in_memory: &ModlistWorkspaceState,
        store: &WorkspaceStore,
        now: Instant,
    ) -> Result<bool, RegistryError> {
        let needs_write = self.last_saved_workspaces.get(modlist_id) != Some(in_memory);
        if !needs_write {
            return Ok(false);
        }
        if !self.is_debounce_elapsed(modlist_id, now) {
            return Ok(false);
        }
        store.save(in_memory)?;
        self.last_saved_workspaces
            .insert(modlist_id.to_string(), in_memory.clone());
        self.last_dirty_at.remove(modlist_id);
        Ok(true)
    }

    /// Synchronous full flush. Called from `eframe::App::on_exit` (primary
    /// hook per H4) and `Drop for OrchestratorApp` (fallback). Both call
    /// sites pass the same `(registry, registry_store, workspaces,
    /// workspace_stores)` data; this function diffs against the saved
    /// snapshots and writes only what's different — making it idempotent.
    ///
    /// Errors are collected and returned; callers may log/swallow them
    /// (we're shutting down — there's no UI to surface to). The caller is
    /// expected to log the error stream itself.
    pub fn flush_all(
        &mut self,
        in_memory_registry: &ModlistRegistry,
        registry_store: &RegistryStore,
        in_memory_workspaces: &HashMap<String, ModlistWorkspaceState>,
        workspace_stores: &HashMap<String, WorkspaceStore>,
    ) -> Vec<RegistryError> {
        let mut errs = Vec::new();

        if in_memory_registry != &self.last_saved_registry
            && let Err(err) = registry_store.save(in_memory_registry)
        {
            errs.push(err);
        } else {
            self.last_saved_registry = in_memory_registry.clone();
            self.last_dirty_at.remove(REGISTRY_KEY);
        }

        for (id, ws) in in_memory_workspaces {
            let Some(store) = workspace_stores.get(id) else {
                continue;
            };
            let differs = self.last_saved_workspaces.get(id) != Some(ws);
            if !differs {
                continue;
            }
            match store.save(ws) {
                Ok(()) => {
                    self.last_saved_workspaces.insert(id.clone(), ws.clone());
                    self.last_dirty_at.remove(id);
                }
                Err(err) => errs.push(err),
            }
        }

        errs
    }

    fn is_debounce_elapsed(&self, key: &str, now: Instant) -> bool {
        // A missing dirty mark means "force-write requested without
        // debouncing" — e.g., a direct call after a mutation that wants
        // immediate persistence. Treat as elapsed.
        self.last_dirty_at.get(key).is_none_or(|t| {
            now.saturating_duration_since(*t) >= Duration::from_millis(self.debounce_ms)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::ModlistEntry;
    use crate::registry::store_workspace::WorkspaceStore;

    fn temp_registry_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "bio_cycle_test_{}_{}_modlists.json",
            std::process::id(),
            label
        ))
    }

    #[test]
    fn unchanged_registry_does_not_save() {
        let path = temp_registry_path("unchanged");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new_with_path(&path);
        let registry = ModlistRegistry::default();
        let mut cycle = RegistryPersistenceCycle::new_with_baseline(registry.clone());
        // No mark_registry_dirty + identical in_memory → no save.
        let wrote = cycle
            .persist_registry_if_needed(&registry, &store, Instant::now())
            .expect("ok");
        assert!(!wrote);
        assert!(!path.exists(), "file not created when nothing changed");
    }

    #[test]
    fn changed_registry_saves_after_debounce() {
        let path = temp_registry_path("changed");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new_with_path(&path);
        let baseline = ModlistRegistry::default();
        let mut cycle = RegistryPersistenceCycle::new_with_baseline(baseline);
        cycle.debounce_ms = 0; // force-write immediately

        let mut in_memory = ModlistRegistry::default();
        in_memory.entries.push(ModlistEntry {
            id: "TESTID000000".to_string(),
            ..Default::default()
        });
        cycle.mark_registry_dirty(Instant::now());

        let wrote = cycle
            .persist_registry_if_needed(&in_memory, &store, Instant::now())
            .expect("ok");
        assert!(wrote);
        assert!(path.exists());

        // Calling again does nothing because in_memory == last_saved.
        let wrote2 = cycle
            .persist_registry_if_needed(&in_memory, &store, Instant::now())
            .expect("ok");
        assert!(!wrote2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn workspace_persistence_runs_per_id() {
        let root = std::env::temp_dir().join(format!("bio_cycle_ws_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let path = root.join("modlists/IDX/workspace.json");
        let store = WorkspaceStore::new_with_path(&path);
        let mut cycle = RegistryPersistenceCycle::default();
        cycle.debounce_ms = 0;
        let mut ws = ModlistWorkspaceState::default();
        ws.last_share_code = Some("ABC".to_string());
        cycle.mark_workspace_dirty("IDX", Instant::now());
        let wrote = cycle
            .persist_workspace_if_needed("IDX", &ws, &store, Instant::now())
            .expect("ok");
        assert!(wrote);
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn workspace_extract_debug_count_starts_zero_and_only_bumps_when_noted() {
        // P6.T11 / H1: the counter must start flat and only advance when the
        // orchestrator explicitly notes a dirty-gated extract. An idle
        // workspace (the orchestrator never calls `note_workspace_extract`)
        // leaves it at 0 — the observable "zero per-frame extract" property.
        let mut cycle = RegistryPersistenceCycle::default();
        assert_eq!(cycle.workspace_extract_debug_count, 0);
        // Persisting an unchanged workspace (the idle path) does NOT bump it.
        let root = std::env::temp_dir().join(format!("bio_h1_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = WorkspaceStore::new_with_path(root.join("modlists/H1/workspace.json"));
        let ws = ModlistWorkspaceState::default();
        let _ = cycle.persist_workspace_if_needed("H1", &ws, &store, Instant::now());
        assert_eq!(
            cycle.workspace_extract_debug_count, 0,
            "the cadence itself must never bump the H1 counter"
        );
        // Only an explicit note (the orchestrator's dirty-gated extract) does.
        cycle.note_workspace_extract();
        cycle.note_workspace_extract();
        assert_eq!(cycle.workspace_extract_debug_count, 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn flush_all_is_idempotent_when_nothing_changes() {
        let path = temp_registry_path("idempotent");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new_with_path(&path);
        let registry = ModlistRegistry::default();
        let mut cycle = RegistryPersistenceCycle::new_with_baseline(registry.clone());
        let errs = cycle.flush_all(&registry, &store, &HashMap::new(), &HashMap::new());
        assert!(errs.is_empty());
        assert!(!path.exists(), "no save when nothing changed");
    }
}
