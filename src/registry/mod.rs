// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod dev_seed;
pub mod errors;
pub mod ids;
pub mod model;
pub mod operations;
pub mod operations_create;
pub mod operations_rename;
pub mod persistence_cycle;
pub mod share_export;
pub mod store;
pub mod store_workspace;
pub mod workspace_model;

pub use errors::RegistryError;
pub use model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
pub use persistence_cycle::RegistryPersistenceCycle;
pub use store::RegistryStore;
pub use store_workspace::WorkspaceStore;
pub use workspace_model::{ComponentRef, ModlistWorkspaceState, PromptOverride};
