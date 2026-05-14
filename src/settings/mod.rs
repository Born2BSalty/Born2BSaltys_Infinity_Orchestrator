// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod model;
pub mod store;

// Phase 4 sibling modules. Per the CRITICAL DIRECTIVE carve-out #3 companion
// provision: additive `pub mod` lines for orchestrator-side settings. **Never**
// modify `model::AppSettings` to host these fields — they live in
// `bio_redesign_settings.json` (sibling file).
pub mod redesign_fields;
pub mod redesign_store;
