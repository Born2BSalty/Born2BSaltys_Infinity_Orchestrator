// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod filters;
mod model;
mod prompt_block;
mod prompt_detect;
mod prompt_match;
mod prompt;
mod utils;

pub(crate) use model::PromptInfo;

pub(crate) use filters::extract_error_block;
pub(crate) use filters::important_line;
pub(crate) use filters::installed_line;
pub(crate) use filters::likely_failure_visible;
pub(crate) use filters::parser_timestamp_line;
pub(crate) use filters::warning_capture_end;
pub(crate) use filters::warning_capture_start;

pub(crate) use prompt::current_prompt_info;
pub(crate) use prompt::likely_input_needed_visible;
pub(crate) use prompt::prompt_capture_end;
pub(crate) use prompt::prompt_capture_start;
pub(crate) use prompt::prompt_headers_ready;
pub(crate) use prompt::prompt_kind_name;
