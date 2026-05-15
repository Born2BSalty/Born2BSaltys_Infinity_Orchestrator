// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "log_files_diagnostics.rs"]
pub mod diagnostics_logs;
#[path = "log_files_open.rs"]
pub mod log_open;
#[path = "log_files_validators.rs"]
pub mod path_validators;
#[path = "log_files_source_logs.rs"]
pub mod source_logs;
#[path = "log_files_target_prep.rs"]
pub mod target_prep;

pub(crate) use diagnostics_logs::{
    DiagnosticLogGroup, begin_new_run, copy_diagnostic_origin_logs,
    copy_weidu_logs_for_diagnostics, current_or_new_run_id, prune_old_diagnostics, run_dir_from_id,
};
pub(crate) use log_open::{open_console_logs_folder, open_last_log_file, save_console_log};
pub(crate) use path_validators::{
    validate_resume_paths, validate_runtime_prep_paths, verify_targets_prepared,
};
pub(crate) use source_logs::{SourceLogInfo, source_log_infos};
pub(crate) use target_prep::{TargetPrepResult, prepare_target_dirs_before_install};
