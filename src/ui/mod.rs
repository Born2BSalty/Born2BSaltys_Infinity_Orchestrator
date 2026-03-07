// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "../core/app/app.rs"]
pub mod app;
#[path = "../core/app/controller/mod.rs"]
pub mod controller;
pub mod frame;
#[path = "../core/app/layout.rs"]
pub mod layout;
pub mod run;
#[path = "../core/app/scan/mod.rs"]
pub mod scan;
pub mod shared;
#[path = "../core/app/state.rs"]
pub mod state;
#[path = "../core/app/state_convert.rs"]
pub mod state_convert;
#[path = "../core/app/state_nav.rs"]
pub mod state_nav;
#[path = "../core/app/state_validation.rs"]
pub mod state_validation;
pub mod step1;
pub mod step2;
#[path = "../core/app/step2_worker.rs"]
pub mod step2_worker;
pub mod step3;
pub mod step4;
pub mod step5;
#[path = "../core/app/terminal/mod.rs"]
pub mod terminal;

pub use run::run;
