// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod action_step5;
pub mod content_step5;
pub mod frame_step5;
pub mod menus_step5;
pub mod page_step5;
pub mod prompt_answers_step5;
pub mod service_auto_answer_step5;
pub mod service_diagnostics_run_step5;
pub mod service_install_flow_step5;
pub mod service_json_fallback_step5;
pub mod service_process_line_step5;
pub mod service_readiness_step5;
pub mod service_scripted_step5;
pub mod service_step5_command_step5;
pub mod service_step5;
pub mod service_timefmt_step5;
pub mod state_step5;
pub mod status_bar_step5;
pub mod top_panels_step5;
#[path = "../../core/app/step5/diagnostics.rs"]
mod diagnostics;
#[path = "../../core/app/step5/log_files.rs"]
mod log_files;
#[path = "../../core/app/step5/prompt_memory.rs"]
mod prompt_memory;
#[path = "../../core/app/step5/scripted_inputs.rs"]
mod scripted_inputs;
