// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "log_files_diagnostics.rs"]
mod diagnostics_logs;
#[path = "log_files_open.rs"]
mod log_open;
#[path = "log_files_source_logs.rs"]
mod source_logs;
#[path = "log_files_target_prep.rs"]
mod target_prep;
#[path = "log_files_validators.rs"]
mod path_validators;

pub use diagnostics_logs::{copy_diagnostic_origin_logs, DiagnosticLogGroup};
pub use log_open::{open_console_logs_folder, open_last_log_file, save_console_log};
pub use path_validators::{
    validate_resume_paths, validate_runtime_prep_paths, verify_targets_prepared,
};
pub use source_logs::{
    copy_saved_weidu_logs, copy_source_weidu_logs, source_log_infos, SourceLogInfo,
};
pub use target_prep::{prepare_target_dirs_before_install, TargetPrepResult};
