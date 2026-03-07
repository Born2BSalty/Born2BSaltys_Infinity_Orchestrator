// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use super::PromptSummaryIndex;

mod map_to_bio;
mod model;
mod runner;

pub fn collect_prompt_summary_index(
    tp2_path: &Path,
    mods_root: &Path,
    preferred_lang: Option<&str>,
    preferred_game: Option<&str>,
) -> Result<PromptSummaryIndex, String> {
    let output = runner::run_lapdu_parser(tp2_path, mods_root, preferred_lang)?;
    Ok(map_to_bio::build_prompt_summary_index(&output, preferred_game))
}
