// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::registry` — Phase 3 data layer for the modlist registry +
// per-modlist workspace state. Owned by the orchestrator; no BIO source
// modifications.
//
// Per Phase 3 P3.T1..T10:
//   - `model`              → `ModlistRegistry`, `ModlistEntry`, `ModlistState`, `Game`.
//   - `store`              → `RegistryStore` (load/save with atomic rename
//                            + corrupt-file backup).
//   - `workspace_model`    → `ModlistWorkspaceState` + `ComponentRef`.
//   - `store_workspace`    → `WorkspaceStore` (per-modlist workspace.json
//                            load/save).
//   - `errors`             → `RegistryError` enum.
//   - `ids`                → `new_modlist_id` (12-char base32 ULID-ish).
//   - `persistence_cycle`  → `RegistryPersistenceCycle` (per-frame debounce
//                            + `flush_all` for the `on_exit` + `Drop`
//                            hooks per H4).
//   - `dev_seed`           → `seed_demo_entry` for the dev-only Home button.
//   - `operations`         → CRUD entry points (populated in Phase 5; the
//                            module is declared here so visibility is stable
//                            across phases).
//   - `operations_create`  → Phase 6 P6.T7: `create_modlist` (allocate id +
//                            insert an `in_progress` entry + write the empty
//                            workspace.json; SPEC §5.1 / §13.1). The fork
//                            variant is Run 4 (P6.T8), not defined yet.
//   - `operations_rename`  → Phase 6 P6.T5: `rename_modlist` (registry-entry
//                            rename ONLY — no on-disk folder rename, SPEC
//                            §2.2; debounced via the registry persistence
//                            path, NOT `workspace_state_dirty`).
//
// SPEC: §13.1, §13.14, §2.2.

pub mod dev_seed;
pub mod errors;
pub mod ids;
pub mod model;
pub mod operations;
pub mod operations_create;
pub mod operations_rename;
pub mod persistence_cycle;
pub mod store;
pub mod store_workspace;
pub mod workspace_model;

pub use errors::RegistryError;
pub use model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
pub use persistence_cycle::RegistryPersistenceCycle;
pub use store::RegistryStore;
pub use store_workspace::WorkspaceStore;
pub use workspace_model::{ComponentRef, ModlistWorkspaceState, PromptOverride};
