// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::game_authority::{self, GameSlot, slot_for_tab};
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::prompt_eval_summary_step3::evaluate_step3_item_prompt_summary;
use crate::app::state::{
    InstallerAnswerDraft, InstallerAnswerTarget, InstallerAnswersEditor, Step2ModState,
    Step2Selection, Step2State, Step3ItemState, WizardState,
};
use crate::app::wlb_inputs::{
    decode_answers, encode_answers, marker_spec, parse_identity, replace_marker,
};

fn items_for_tab<'a>(state: &'a WizardState, tab: &str) -> &'a [Step3ItemState] {
    match slot_for_tab(tab) {
        GameSlot::First => &state.step3.bgee_items,
        GameSlot::Second => &state.step3.bg2ee_items,
    }
}

fn mods_for_tab<'a>(state: &'a WizardState, tab: &str) -> &'a [Step2ModState] {
    match slot_for_tab(tab) {
        GameSlot::First => &state.step2.bgee_mods,
        GameSlot::Second => &state.step2.bg2ee_mods,
    }
}

pub(crate) fn prompt_context_for_tab(
    state: &WizardState,
    game_tab: &str,
) -> crate::parser::prompt_eval_expr::PromptEvalContext {
    let context_state = WizardState {
        step1: state.step1.clone(),
        step2: Step2State {
            active_game_tab: game_tab.to_string(),
            ..Step2State::default()
        },
        ..WizardState::default()
    };
    let mut context = build_prompt_eval_context(&context_state);
    context.checked_components = items_for_tab(state, game_tab)
        .iter()
        .filter(|item| !item.is_parent && !item.parent_placeholder)
        .map(|item| {
            (
                crate::parser::prompt_eval_expr::normalize_tp2_stem(&item.tp_file),
                item.component_id.trim().to_string(),
            )
        })
        .collect();
    let is_eet = slot_for_tab(game_tab) == GameSlot::Second
        && context
            .checked_components
            .contains(&("eet".to_string(), "0".to_string()));
    if is_eet {
        context.active_games.clear();
        context.active_games.insert("eet".to_string());
    }
    let mut checked = context
        .checked_components
        .iter()
        .map(|(tp2, component)| format!("{tp2}|{component}"))
        .collect::<Vec<_>>();
    checked.sort_unstable();
    context.signature = format!(
        "{};step3={};eet={is_eet}",
        context.signature,
        checked.join(";")
    );
    context
}

pub(crate) fn editing_locked(state: &WizardState) -> bool {
    state.step2.is_scanning
        || state.step5.install_running
        || state.step1.installs_exactly_from_weidu_logs()
}

pub(crate) fn bind_workspace(state: &mut WizardState, workspace_id: Option<&str>) {
    if state.step3.installer_answers.is_none() {
        state.step3.installer_answers_workspace = None;
        return;
    }
    if workspace_id.is_none()
        || state
            .step3
            .installer_answers_workspace
            .as_deref()
            .is_some_and(|owner| Some(owner) != workspace_id)
    {
        state.step3.installer_answers = None;
        state.step3.installer_answers_workspace = None;
    } else {
        state.step3.installer_answers_workspace = workspace_id.map(str::to_string);
    }
}

pub(crate) fn current_target_available(state: &WizardState) -> bool {
    state
        .step3
        .installer_answers
        .as_ref()
        .and_then(|editor| editor.targets.get(editor.active))
        .is_some_and(|target| resolve_target(state, target).is_ok())
}

pub(crate) fn has_saved_answers(raw_line: &str) -> bool {
    marker_spec(raw_line).is_some_and(|spec| {
        !spec.is_empty() && decode_answers(spec).is_ok_and(|answers| !answers.is_empty())
    })
}

fn make_target(
    state: &WizardState,
    game_tab: &str,
    row_index: usize,
    prompt_eval: &crate::parser::prompt_eval_expr::PromptEvalContext,
) -> Option<InstallerAnswerTarget> {
    let item = items_for_tab(state, game_tab).get(row_index)?;
    if item.is_parent || item.parent_placeholder {
        return None;
    }
    let questions = evaluate_step3_item_prompt_summary(item, prompt_eval);
    if questions.trim().is_empty() {
        return None;
    }
    let identity = parse_identity(&item.raw_line)?;
    if identity.normalized_tp2 != normalize_mod_download_tp2(&item.tp_file)
        || item.component_id.parse::<i64>().ok() != Some(identity.component_id)
    {
        return None;
    }
    let answers = marker_spec(&item.raw_line)
        .filter(|spec| !spec.is_empty())
        .and_then(|spec| decode_answers(spec).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|text| InstallerAnswerDraft {
            blank_enter: text.is_empty(),
            text,
        })
        .collect::<Vec<_>>();
    Some(InstallerAnswerTarget {
        game_tab: game_tab.to_string(),
        package_path: item
            .raw_line
            .trim_start()
            .strip_prefix('~')?
            .split_once('~')?
            .0
            .to_string(),
        row_index,
        block_id: item.block_id.clone(),
        tp_file: item.tp_file.clone(),
        component_id: item.component_id.clone(),
        language: identity.language,
        original_line: item.raw_line.clone(),
        title: format!(
            "{} #{} — {}",
            item.mod_name, item.component_id, item.component_label
        ),
        questions,
        answers: if answers.is_empty() {
            vec![InstallerAnswerDraft::default()]
        } else {
            answers
        },
    })
}

fn open_targets(
    state: &mut WizardState,
    mut targets: Vec<InstallerAnswerTarget>,
    review_missing: bool,
) {
    if let Some(previous) = state.step3.installer_answers.as_ref() {
        for target in &mut targets {
            if let Some(old) = previous.targets.iter().find(|old| {
                old.game_tab == target.game_tab
                    && old.block_id == target.block_id
                    && old.row_index == target.row_index
                    && old.original_line == target.original_line
            }) {
                target.answers.clone_from(&old.answers);
            }
        }
    }
    let status = targets
        .iter()
        .any(|target| {
            marker_spec(&target.original_line).is_some_and(|spec| decode_answers(spec).is_err())
        })
        .then(|| "Existing saved answers are invalid and cannot be overwritten here.".to_string());
    if let Some(target) = targets.first() {
        state.step3.active_game_tab.clone_from(&target.game_tab);
    }
    state.step2.prompt_popup_open = false;
    state.step3.prompt_edit_open = false;
    state.step3.installer_answers_workspace = None;
    state.step3.installer_answers = Some(InstallerAnswersEditor {
        targets,
        active: 0,
        status,
        review_missing,
        game_install: state.step1.game_install.clone(),
    });
}

pub(crate) fn open_component(state: &mut WizardState, game_tab: &str, row_index: usize) -> bool {
    if editing_locked(state)
        || !game_authority::tabs_for_install(&state.step1.game_install).contains(&game_tab)
    {
        return false;
    }
    let prompt_eval = prompt_context_for_tab(state, game_tab);
    let Some(target) = make_target(state, game_tab, row_index, &prompt_eval) else {
        return false;
    };
    open_targets(state, vec![target], false);
    true
}

pub(crate) fn open_missing(state: &mut WizardState) -> bool {
    if editing_locked(state) {
        return false;
    }
    let mut targets = Vec::new();
    for game_tab in game_authority::tabs_for_install(&state.step1.game_install) {
        let prompt_eval = prompt_context_for_tab(state, game_tab);
        for (index, item) in items_for_tab(state, game_tab).iter().enumerate() {
            if !has_saved_answers(&item.raw_line)
                && let Some(target) = make_target(state, game_tab, index, &prompt_eval)
            {
                targets.push(target);
            }
        }
    }
    if targets.is_empty() {
        return false;
    }
    open_targets(state, targets, true);
    true
}

fn resolve_target<'a>(
    state: &'a WizardState,
    target: &InstallerAnswerTarget,
) -> Result<&'a Step3ItemState, String> {
    if editing_locked(state)
        || state
            .step3
            .installer_answers
            .as_ref()
            .is_some_and(|editor| editor.game_install != state.step1.game_install)
        || !game_authority::tabs_for_install(&state.step1.game_install)
            .contains(&target.game_tab.as_str())
    {
        return Err(
            "This component is no longer available for editing. Reopen Installer answers."
                .to_string(),
        );
    }
    let identity =
        parse_identity(&target.original_line).ok_or("Cannot identify this component.")?;
    let items = items_for_tab(state, &target.game_tab);
    let item = items
        .get(target.row_index)
        .ok_or("This component changed. Reopen Installer answers.")?;
    let snapshot_changed = item.raw_line != target.original_line;
    if item.is_parent
        || item.parent_placeholder
        || item.block_id != target.block_id
        || snapshot_changed
        || item.component_id != target.component_id
        || item
            .raw_line
            .trim_start()
            .strip_prefix('~')
            .and_then(|raw| raw.split_once('~'))
            .map(|(path, _)| path)
            != Some(target.package_path.as_str())
        || item.tp_file != target.tp_file
        || parse_identity(&item.raw_line).as_ref() != Some(&identity)
        || identity.language != target.language
        || items
            .iter()
            .filter(|other| {
                !other.is_parent && parse_identity(&other.raw_line).as_ref() == Some(&identity)
            })
            .count()
            != 1
    {
        return Err(
            "This component changed or is ambiguous. Reopen Installer answers.".to_string(),
        );
    }
    Ok(item)
}

fn step2_mirror(
    state: &WizardState,
    target: &InstallerAnswerTarget,
) -> Result<Option<(usize, usize)>, String> {
    let identity = parse_identity(&target.original_line);
    let mut matches = mods_for_tab(state, &target.game_tab)
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            normalize_mod_download_tp2(&m.tp_file) == normalize_mod_download_tp2(&target.tp_file)
        })
        .flat_map(|(mi, m)| {
            m.components
                .iter()
                .enumerate()
                .map(move |(ci, c)| (mi, ci, c))
        })
        .filter(|(_, _, c)| parse_identity(&c.raw_line) == identity);
    let first = matches.next();
    if matches.next().is_some()
        || first.is_some_and(|(_, _, c)| c.raw_line != target.original_line || !c.checked)
    {
        return Err(
            "The scanned component changed or is ambiguous. Reopen Installer answers.".to_string(),
        );
    }
    Ok(first.map(|(mi, ci, _)| (mi, ci)))
}

fn encode_drafts(target: &InstallerAnswerTarget) -> Result<String, String> {
    if let Some(spec) = marker_spec(&target.original_line) {
        decode_answers(spec).map_err(|_| {
            "The existing saved answers are invalid. Fix them before replacing them.".to_string()
        })?;
    }
    if target
        .answers
        .iter()
        .all(|draft| !draft.blank_enter && draft.text.is_empty())
    {
        return Ok(without_saved_answers(&target.original_line));
    }
    let answers = target
        .answers
        .iter()
        .map(|draft| {
            if draft.blank_enter {
                Ok(String::new())
            } else if draft.text.is_empty() {
                Err("Enter an answer or select Send Enter without text.".to_string())
            } else {
                Ok(draft.text.clone())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let spec = encode_answers(&answers)?.ok_or("Add at least one answer before saving.")?;
    replace_marker(&target.original_line, &spec)
}

fn without_saved_answers(raw_line: &str) -> String {
    raw_line
        .to_ascii_lowercase()
        .find("@wlb-inputs:")
        .map_or_else(
            || raw_line.to_string(),
            |start| {
                let prefix = raw_line[..start].trim_end();
                prefix
                    .strip_suffix("//")
                    .unwrap_or(prefix)
                    .trim_end()
                    .to_string()
            },
        )
}

pub(crate) fn save_current(state: &mut WizardState) -> Result<(), String> {
    let target = state
        .step3
        .installer_answers
        .as_ref()
        .and_then(|editor| editor.targets.get(editor.active))
        .ok_or("No component is open.")?
        .clone();
    resolve_target(state, &target)?;
    let replacement = encode_drafts(&target)?;
    let mirror = step2_mirror(state, &target)?;
    let items = match slot_for_tab(&target.game_tab) {
        GameSlot::First => &mut state.step3.bgee_items,
        GameSlot::Second => &mut state.step3.bg2ee_items,
    };
    items[target.row_index].raw_line.clone_from(&replacement);
    if let Some((mi, ci)) = mirror {
        let mods = match slot_for_tab(&target.game_tab) {
            GameSlot::First => &mut state.step2.bgee_mods,
            GameSlot::Second => &mut state.step2.bg2ee_mods,
        };
        mods[mi].components[ci].raw_line.clone_from(&replacement);
    }
    update_selection(state, &target, &replacement);
    state.step3.installer_answers = None;
    state.step3.installer_answers_workspace = None;
    Ok(())
}

fn update_selection(state: &mut WizardState, target: &InstallerAnswerTarget, replacement: &str) {
    if let Some(Step2Selection::Component {
        game_tab,
        tp_file,
        component_id,
        component_key,
    }) = state.step2.selected.as_mut()
        && *game_tab == target.game_tab
        && *tp_file == target.tp_file
        && *component_id == target.component_id
        && *component_key == target.original_line
    {
        replacement.clone_into(component_key);
    }
}

#[cfg(test)]
#[path = "installer_answers_step3_tests.rs"]
mod tests;
