// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod app;
pub mod frame;
pub mod layout;
pub mod run;
pub mod shared;
pub mod step1;
pub mod step2;
pub mod step3;
pub mod step4;
pub mod step5;

// Redesign module (Infinity Orchestrator) — additive registration per
// CRITICAL DIRECTIVE carve-out #3 companion provision.
pub mod orchestrator;
pub mod settings;
pub mod shell;

pub use run::run;
