// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeSet;

use crate::app::game_authority::{GameSlot, slot_for_tab};
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::prompt_eval_summary::evaluate_component_prompt_summary;
use crate::app::state::{
    InstallerAnswerDraft, InstallerAnswerTarget, InstallerAnswersEditor, Step2ComponentState,
    Step2ModState, Step2Selection, Step3ItemState, WizardState,
};
use crate::app::wlb_inputs::{
    decode_answers, encode_answers, marker_spec, parse_identity, replace_marker,
};
use crate::ui::step2::state_step2::non_scan_controls_locked;
use crate::ui::step2::tree_compat_display_step2::is_effectively_disabled;

fn mods_for_tab<'a>(state: &'a WizardState, tab: &str) -> &'a [Step2ModState] {
    match slot_for_tab(tab) {
        GameSlot::First => &state.step2.bgee_mods,
        GameSlot::Second => &state.step2.bg2ee_mods,
    }
}

pub(crate) fn editing_locked(state: &WizardState) -> bool {
    non_scan_controls_locked(state) || state.step1.installs_exactly_from_weidu_logs()
}

pub(crate) fn bind_workspace(state: &mut WizardState, workspace_id: Option<&str>) {
    if state.step2.installer_answers.is_none() {
        state.step2.installer_answers_workspace = None;
        return;
    }
    if workspace_id.is_none()
        || state
            .step2
            .installer_answers_workspace
            .as_deref()
            .is_some_and(|owner| Some(owner) != workspace_id)
    {
        state.step2.installer_answers = None;
        state.step2.installer_answers_workspace = None;
    } else {
        state.step2.installer_answers_workspace = workspace_id.map(str::to_string);
    }
}

pub(crate) fn current_target_available(state: &WizardState) -> bool {
    state
        .step2
        .installer_answers
        .as_ref()
        .and_then(|editor| editor.targets.get(editor.active))
        .is_some_and(|target| resolve_target(state, target).is_ok())
}

pub(crate) fn checked_snapshot(state: &WizardState) -> Vec<(String, String, String)> {
    mods_for_tab(state, &state.step2.active_game_tab)
        .iter()
        .flat_map(|mod_state| {
            mod_state.components.iter().filter(|c| c.checked).map(|c| {
                (
                    mod_state.tp_file.clone(),
                    mod_state.tp2_path.clone(),
                    c.raw_line.clone(),
                )
            })
        })
        .collect()
}

pub(crate) fn open_newly_selected(state: &mut WizardState, before: &[(String, String, String)]) {
    if editing_locked(state) {
        return;
    }
    let prompt_eval = build_prompt_eval_context(state);
    let before = before.iter().collect::<BTreeSet<_>>();
    let mut targets = Vec::new();
    for mod_state in mods_for_tab(state, &state.step2.active_game_tab) {
        for component in &mod_state.components {
            let key = (
                mod_state.tp_file.clone(),
                mod_state.tp2_path.clone(),
                component.raw_line.clone(),
            );
            if !before.contains(&key)
                && let Some(target) = make_target(state, mod_state, component, &prompt_eval)
            {
                targets.push(target);
            }
        }
    }
    if !targets.is_empty() {
        open_targets(state, targets);
    }
}

fn make_target(
    state: &WizardState,
    mod_state: &Step2ModState,
    component: &Step2ComponentState,
    prompt_eval: &crate::parser::prompt_eval_expr::PromptEvalContext,
) -> Option<InstallerAnswerTarget> {
    if !component.checked || is_effectively_disabled(component) {
        return None;
    }
    let questions = evaluate_component_prompt_summary(component, prompt_eval);
    if questions.trim().is_empty() {
        return None;
    }
    let identity = parse_identity(&component.raw_line)?;
    if identity.normalized_tp2 != normalize_mod_download_tp2(&mod_state.tp_file)
        || component.component_id.parse::<i64>().ok() != Some(identity.component_id)
    {
        return None;
    }
    let answers = marker_spec(&component.raw_line)
        .and_then(|spec| decode_answers(spec).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|text| InstallerAnswerDraft {
            blank_enter: text.is_empty(),
            text,
        })
        .collect::<Vec<_>>();
    Some(InstallerAnswerTarget {
        game_tab: state.step2.active_game_tab.clone(),
        package_path: mod_state.tp2_path.clone(),
        tp_file: mod_state.tp_file.clone(),
        component_id: component.component_id.clone(),
        language: identity.language,
        original_line: component.raw_line.clone(),
        title: format!(
            "{} #{} — {}",
            mod_state.name, component.component_id, component.label
        ),
        questions,
        answers: if answers.is_empty() {
            vec![InstallerAnswerDraft::default()]
        } else {
            answers
        },
    })
}

fn open_targets(state: &mut WizardState, mut targets: Vec<InstallerAnswerTarget>) {
    if let Some(previous) = state.step2.installer_answers.as_ref() {
        for target in &mut targets {
            if let Some(old) = previous.targets.iter().find(|old| {
                old.game_tab == target.game_tab
                    && old.package_path == target.package_path
                    && old.tp_file == target.tp_file
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
    state.step2.prompt_popup_open = false;
    state.step2.installer_answers_workspace = None;
    state.step2.installer_answers = Some(InstallerAnswersEditor {
        targets,
        active: 0,
        status,
    });
}

pub(crate) fn open_selected(state: &mut WizardState, selection: &Step2Selection) -> bool {
    if editing_locked(state) {
        return false;
    }
    let Step2Selection::Component {
        game_tab,
        tp_file,
        component_id,
        component_key,
    } = selection
    else {
        return false;
    };
    if game_tab != &state.step2.active_game_tab {
        return false;
    }
    let prompt_eval = build_prompt_eval_context(state);
    let targets = mods_for_tab(state, game_tab)
        .iter()
        .filter(|m| &m.tp_file == tp_file)
        .flat_map(|m| m.components.iter().map(move |c| (m, c)))
        .filter(|(_, c)| &c.component_id == component_id && &c.raw_line == component_key)
        .filter_map(|(m, c)| make_target(state, m, c, &prompt_eval))
        .collect::<Vec<_>>();
    if targets.len() != 1 {
        return false;
    }
    open_targets(state, targets);
    true
}

fn resolve_target(
    state: &WizardState,
    target: &InstallerAnswerTarget,
) -> Result<(usize, usize), String> {
    if editing_locked(state) || state.step2.active_game_tab != target.game_tab {
        return Err(
            "This component is no longer available for editing. Reopen Installer answers."
                .to_string(),
        );
    }
    let identity =
        parse_identity(&target.original_line).ok_or("Cannot identify this component.")?;
    let mut matches = mods_for_tab(state, &target.game_tab)
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            let same_package = m.tp2_path == target.package_path;
            same_package && m.tp_file == target.tp_file
        })
        .flat_map(|(mi, m)| {
            m.components
                .iter()
                .enumerate()
                .map(move |(ci, c)| (mi, ci, c))
        })
        .filter(|(_, _, c)| {
            c.component_id == target.component_id
                && parse_identity(&c.raw_line).as_ref() == Some(&identity)
        });
    let Some((mi, ci, component)) = matches.next() else {
        return Err("This component changed. Reopen Installer answers.".to_string());
    };
    if matches.next().is_some()
        || component.raw_line != target.original_line
        || !component.checked
        || is_effectively_disabled(component)
        || identity.language != target.language
    {
        return Err(
            "This component changed or is ambiguous. Reopen Installer answers.".to_string(),
        );
    }
    Ok((mi, ci))
}

fn step3_target(
    items: &[Step3ItemState],
    target: &InstallerAnswerTarget,
) -> Result<Option<usize>, String> {
    let identity = parse_identity(&target.original_line);
    let mut matches = items.iter().enumerate().filter(|(_, item)| {
        !item.is_parent
            && normalize_mod_download_tp2(&item.tp_file)
                == normalize_mod_download_tp2(&target.tp_file)
            && parse_identity(&item.raw_line) == identity
    });
    let first = matches.next();
    if matches.next().is_some()
        || first.is_some_and(|(_, item)| item.raw_line != target.original_line)
    {
        return Err(
            "The install order contains a changed or repeated component. Reopen Installer answers."
                .to_string(),
        );
    }
    Ok(first.map(|(index, _)| index))
}

fn encode_drafts(target: &InstallerAnswerTarget) -> Result<String, String> {
    if let Some(spec) = marker_spec(&target.original_line) {
        decode_answers(spec).map_err(|_| {
            "The existing saved answers are invalid. Fix them before replacing them.".to_string()
        })?;
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

pub(crate) fn save_current(state: &mut WizardState) -> Result<(), String> {
    let target = state
        .step2
        .installer_answers
        .as_ref()
        .and_then(|editor| editor.targets.get(editor.active))
        .ok_or("No component is open.")?
        .clone();
    let (mi, ci) = resolve_target(state, &target)?;
    let replacement = encode_drafts(&target)?;
    let items = match slot_for_tab(&target.game_tab) {
        GameSlot::First => &state.step3.bgee_items,
        GameSlot::Second => &state.step3.bg2ee_items,
    };
    let mirror = step3_target(items, &target)?;
    let mods = match slot_for_tab(&target.game_tab) {
        GameSlot::First => &mut state.step2.bgee_mods,
        GameSlot::Second => &mut state.step2.bg2ee_mods,
    };
    mods[mi].components[ci].raw_line.clone_from(&replacement);
    if let Some(index) = mirror {
        let items = match slot_for_tab(&target.game_tab) {
            GameSlot::First => &mut state.step3.bgee_items,
            GameSlot::Second => &mut state.step3.bg2ee_items,
        };
        items[index].raw_line.clone_from(&replacement);
    }
    update_selection(state, &target, &replacement);
    if let Some(editor) = state.step2.installer_answers.as_mut() {
        editor.targets[editor.active].original_line = replacement;
        editor.status = Some("Answers saved to this list.".to_string());
    }
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
#[path = "installer_answers_step2_tests.rs"]
mod tests;
