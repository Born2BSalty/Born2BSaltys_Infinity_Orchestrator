// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod log_open;
mod path_validators;
mod source_logs;
mod target_prep;

pub use log_open::{open_console_logs_folder, open_last_log_file, save_console_log};
pub use path_validators::{
    validate_resume_paths, validate_runtime_prep_paths, verify_targets_prepared,
};
pub use source_logs::{copy_source_weidu_logs, source_log_infos};
pub use target_prep::prepare_target_dirs_before_install;
