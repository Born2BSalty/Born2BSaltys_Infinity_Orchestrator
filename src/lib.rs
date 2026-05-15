// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "core/app/mod.rs"]
pub mod app;
#[path = "core/cli/mod.rs"]
pub mod cli;
#[path = "core/config/mod.rs"]
pub mod config;
#[path = "core/install/mod.rs"]
pub mod install;
pub mod install_runtime;
#[path = "core/logging/mod.rs"]
pub mod logging;
#[path = "core/mods/mod.rs"]
pub mod mods;
#[path = "core/parser/mod.rs"]
pub mod parser;
#[path = "core/platform_defaults.rs"]
pub mod platform_defaults;
pub mod registry;
pub mod settings;
pub mod ui;
