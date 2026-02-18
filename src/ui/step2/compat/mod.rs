// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod apply;
mod defaults;
mod loader;
mod matcher;
mod model;
mod report;

pub use apply::apply_step2_compat_rules;
pub use defaults::create_default_step2_compat_rules_file;
pub use report::export_step2_compat_report;
