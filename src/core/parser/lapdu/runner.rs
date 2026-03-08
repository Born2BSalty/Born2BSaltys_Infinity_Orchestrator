// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use super::model::ParserOutput;

pub(super) struct LapduRun {
    pub output: ParserOutput,
    pub raw_json: String,
}

pub(super) fn run_lapdu_parser(
    tp2_path: &Path,
    _mods_root: &Path,
    preferred_lang: Option<&str>,
) -> Result<LapduRun, String> {
    let raw_json = lapdu_parser_rust::parse_path_to_json(tp2_path, preferred_lang)?;
    let output = serde_json::from_str::<ParserOutput>(&raw_json)
        .map_err(|e| format!("failed to parse parser JSON output: {e}"))?;
    Ok(LapduRun { output, raw_json })
}
