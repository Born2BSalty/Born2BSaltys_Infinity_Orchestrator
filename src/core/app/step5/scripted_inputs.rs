// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::answer_capture::{CaptureComponentKey, ReplayEntry, ReplayPlan};
use super::command_config::build_install_command_config;
use crate::app::game_authority::GameSlot;
use crate::app::state::Step1State;
use crate::app::wlb_inputs::{decode_answers, marker_spec, parse_identity};
use crate::install::step5_command_log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};
use std::fs;

pub(crate) fn load_from_step1(step1: &Step1State) -> Result<ReplayPlan, String> {
    let config = build_install_command_config(step1);
    let paths = match config.game_install.as_str() {
        "EET" => vec![
            (GameSlot::First, resolve_bgee_log_file(&config)),
            (GameSlot::Second, resolve_bg2_log_file(&config)),
        ],
        "BG2EE" => vec![(GameSlot::Second, resolve_bg2_log_file(&config))],
        _ => vec![(GameSlot::First, resolve_bgee_log_file(&config))],
    };
    let logs = paths
        .into_iter()
        .filter_map(|(slot, path)| fs::read_to_string(path).ok().map(|text| (slot, text)))
        .collect::<Vec<_>>();
    load_from_logs(
        &logs,
        super::prompt_memory::list_component_sequences()
            .into_iter()
            .collect(),
    )
}
pub(crate) fn load_from_logs(
    logs: &[(GameSlot, String)],
    legacy_fallback: Vec<(String, Vec<String>)>,
) -> Result<ReplayPlan, String> {
    let mut plan = ReplayPlan::default();
    for (slot, raw) in logs {
        for line in raw.lines() {
            let Some(spec) = marker_spec(line) else {
                continue;
            };
            let identity = parse_identity(line)
                .ok_or_else(|| "Invalid @wlb-inputs component identity".to_string())?;
            let component = CaptureComponentKey {
                slot: *slot,
                identity,
            };
            if plan
                .entries
                .iter()
                .any(|entry| entry.component == component)
            {
                return Err("Duplicate @wlb-inputs component identity".to_string());
            }
            plan.entries.push(ReplayEntry {
                component,
                answers: decode_answers(spec)?,
            });
        }
    }
    plan.legacy_fallback = legacy_fallback
        .into_iter()
        .filter(|(key, _)| {
            !plan.entries.iter().any(|entry| {
                key.rsplit_once('#').is_some_and(|(tp2, id)| {
                    crate::app::mod_downloads::normalize_mod_download_tp2(tp2)
                        == entry.component.identity.normalized_tp2
                        && id.parse::<i64>().ok() == Some(entry.component.identity.component_id)
                })
            })
        })
        .collect();
    Ok(plan)
}
