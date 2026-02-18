// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod common_checks;
mod run;
mod tp2_depth;

pub(crate) use common_checks::{check_dir, check_file, check_game_dir};
pub use run::run_path_check;
