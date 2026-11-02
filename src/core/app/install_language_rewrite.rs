// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::app::app_step2_log::{resolve_bg2_weidu_log_path, resolve_bgee_weidu_log_path};
use crate::app::game_authority::{GameSlot, has_slot};
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::state::{Step2ModState, WizardState};
use crate::app::wlb_inputs::{marker_spec, parse_identity, replace_marker, strip_marker};

struct ScannedLine {
    language: u8,
    marker: Option<String>,
}

type ScannedLines = BTreeMap<(String, i64), Option<ScannedLine>>;

pub(crate) fn rewrite_imported_logs(state: &WizardState) -> Result<usize, String> {
    let mut changed = 0;
    for (slot, path) in imported_logs(state) {
        let scanned = scanned_for(state, slot);
        if scanned.is_empty() {
            continue;
        }
        changed += rewrite_file(&path, &scanned)?;
    }
    Ok(changed)
}

fn imported_logs(state: &WizardState) -> Vec<(GameSlot, PathBuf)> {
    let step1 = &state.step1;
    [
        (GameSlot::First, resolve_bgee_weidu_log_path(step1)),
        (GameSlot::Second, resolve_bg2_weidu_log_path(step1)),
    ]
    .into_iter()
    .filter(|(slot, _)| has_slot(&step1.game_install, *slot))
    .filter_map(|(slot, path)| path.map(|path| (slot, path)))
    .collect()
}

fn scanned_for(state: &WizardState, slot: GameSlot) -> ScannedLines {
    let mods = match slot {
        GameSlot::First => &state.step2.bgee_mods,
        GameSlot::Second => &state.step2.bg2ee_mods,
    };
    let mut scanned = scanned_lines(mods);
    if slot == GameSlot::First && state.step1.game_install == "EET" {
        let core = normalize_mod_download_tp2(r"EET\EET.TP2");
        for (key, line) in scanned_lines(&state.step2.bg2ee_mods) {
            if key.0 == core {
                scanned.entry(key).or_insert(line);
            }
        }
    }
    scanned
}

fn present_marker(raw_line: &str) -> Option<&str> {
    marker_spec(raw_line).filter(|spec| !spec.is_empty())
}

fn scanned_lines(mods: &[Step2ModState]) -> ScannedLines {
    let mut out = ScannedLines::new();
    for component in mods.iter().flat_map(|mod_state| &mod_state.components) {
        let Some(identity) = parse_identity(&component.raw_line) else {
            continue;
        };
        let line = ScannedLine {
            language: identity.language,
            marker: present_marker(&component.raw_line).map(str::to_string),
        };
        out.entry((identity.normalized_tp2, identity.component_id))
            .and_modify(|found| *found = None)
            .or_insert(Some(line));
    }
    out
}

fn rewrite_file(path: &Path, scanned: &ScannedLines) -> Result<usize, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("could not read the WeiDU log {}: {err}", path.display()))?;
    let mut changed = 0;
    let mut out = String::with_capacity(text.len());
    for piece in text.split_inclusive('\n') {
        let (body, ending) = split_line_ending(piece);
        if let Some(line) = rewrite_line(body, scanned) {
            out.push_str(&line);
            changed += 1;
        } else {
            out.push_str(body);
        }
        out.push_str(ending);
    }
    if changed > 0 {
        fs::write(path, out)
            .map_err(|err| format!("could not write the WeiDU log {}: {err}", path.display()))?;
    }
    Ok(changed)
}

fn split_line_ending(piece: &str) -> (&str, &str) {
    piece.strip_suffix("\r\n").map_or_else(
        || {
            piece
                .strip_suffix('\n')
                .map_or((piece, ""), |body| (body, "\n"))
        },
        |body| (body, "\r\n"),
    )
}

fn rewrite_line(line: &str, scanned: &ScannedLines) -> Option<String> {
    let identity = parse_identity(line)?;
    let target = scanned
        .get(&(identity.normalized_tp2, identity.component_id))?
        .as_ref()?;
    let mut out = if identity.language == target.language {
        line.to_string()
    } else {
        with_language(line, target.language)?
    };
    if present_marker(&out) != target.marker.as_deref() {
        out = target.marker.as_deref().map_or_else(
            || strip_marker(&out),
            |spec| replace_marker(&out, spec).unwrap_or_else(|_| strip_marker(&out)),
        );
    }
    (out != line).then_some(out)
}

fn with_language(line: &str, language: u8) -> Option<String> {
    let open = line.find('~')?;
    let close = open + 1 + line[open + 1..].find('~')?;
    let after = close + 1;
    let digits = after + line[after..].find('#')? + 1;
    let end = line[digits..]
        .find(char::is_whitespace)
        .map_or(line.len(), |offset| digits + offset);
    Some(format!("{}{language}{}", &line[..digits], &line[end..]))
}

#[cfg(test)]
#[path = "install_language_rewrite_tests.rs"]
mod tests;
