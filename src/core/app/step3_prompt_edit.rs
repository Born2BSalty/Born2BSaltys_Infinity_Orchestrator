// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::{Deserialize, Serialize};

use crate::app::state::{Step3ItemState, WizardState};
use crate::platform_defaults::{compose_component_key, normalize_tp2_filename};

const WLB_MARKER: &str = "@wlb-inputs:";

#[derive(Debug, Clone)]
pub(crate) enum PromptActionRequest {
    SetWlb {
        tp_file: String,
        component_id: String,
        component_label: String,
        mod_name: String,
    },
    EditJson {
        tp_file: String,
        component_id: String,
        component_label: String,
        mod_name: String,
    },
    Clear {
        tp_file: String,
        component_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdvancedPromptEntry {
    tp2_file: String,
    component_id: String,
    component_name: String,
    mod_name: String,
    answer: String,
    raw_line: String,
}

pub(crate) fn apply_prompt_actions(state: &mut WizardState, requests: &[PromptActionRequest]) {
    for req in requests {
        match req {
            PromptActionRequest::SetWlb {
                tp_file,
                component_id,
                component_label,
                mod_name,
            } => open_wlb_editor(state, tp_file, component_id, component_label, mod_name),
            PromptActionRequest::EditJson {
                tp_file,
                component_id,
                component_label,
                mod_name,
            } => open_json_editor(state, tp_file, component_id, component_label, mod_name),
            PromptActionRequest::Clear {
                tp_file,
                component_id,
            } => clear_prompt_data(state, tp_file, component_id),
        }
    }
}

pub(crate) fn save_wlb_answer(state: &mut WizardState, answer: &str) -> Result<(), String> {
    let answer = answer.trim().to_string();
    if answer.is_empty() {
        return Err("Answer is empty. Use Clear Prompt Data to remove this entry.".to_string());
    }
    let tp2_file = state.step3.prompt_edit_tp2_file.clone();
    let component_id = state.step3.prompt_edit_component_id.clone();
    let Some(item) = find_component_mut(state, &tp2_file, &component_id) else {
        return Err("Save failed: component not found in current Step 3 tab.".to_string());
    };
    item.raw_line = set_wlb_inputs(&effective_raw_line(item), &answer);
    state.step5.last_status_text = format!(
        "Saved @wlb-inputs on weidu line for {} #{}",
        item.mod_name, item.component_id
    );
    state.step3.prompt_edit_open = false;
    Ok(())
}

pub(crate) fn save_json_entry(state: &mut WizardState, raw_json: &str) -> Result<String, String> {
    let parsed =
        serde_json::from_str::<AdvancedPromptEntry>(raw_json).map_err(|err| err.to_string())?;
    let Some(item) = find_component_mut(state, &parsed.tp2_file, &parsed.component_id) else {
        return Err("Save failed: component not found in current Step 3 tab.".to_string());
    };
    if !parsed.raw_line.trim().is_empty() {
        item.raw_line = parsed.raw_line.trim().to_string();
    } else if !parsed.answer.trim().is_empty() {
        item.raw_line = set_wlb_inputs(&effective_raw_line(item), parsed.answer.trim());
    } else {
        item.raw_line = strip_wlb_marker(&effective_raw_line(item));
    }
    Ok("Saved to weidu line.".to_string())
}

pub(crate) fn delete_prompt_data_from_editor(state: &mut WizardState) -> Result<String, String> {
    let tp2_file = state.step3.prompt_edit_tp2_file.clone();
    let component_id = state.step3.prompt_edit_component_id.clone();
    let Some(item) = find_component_mut(state, &tp2_file, &component_id) else {
        return Err("Delete failed: component not found in current Step 3 tab.".to_string());
    };
    item.raw_line = strip_wlb_marker(&effective_raw_line(item));
    Ok("Deleted from weidu line.".to_string())
}

fn open_wlb_editor(
    state: &mut WizardState,
    tp_file: &str,
    component_id: &str,
    component_label: &str,
    mod_name: &str,
) {
    let component_key = compose_component_key(tp_file, component_id);
    state.step3.prompt_edit_key = component_key.clone();
    state.step3.prompt_edit_component_key = component_key;
    state.step3.prompt_edit_tp2_file = normalize_tp2_filename(tp_file);
    state.step3.prompt_edit_component_id = component_id.trim().to_string();
    state.step3.prompt_edit_component_name = component_label.to_string();
    state.step3.prompt_edit_mod_name = mod_name.to_string();

    state.step3.prompt_edit_answer = find_component_mut(state, tp_file, component_id)
        .and_then(|item| extract_wlb_inputs(&item.raw_line))
        .unwrap_or_default();

    state.step3.prompt_edit_status.clear();
    state.step3.prompt_edit_mode = "wlb".to_string();
    state.step3.prompt_edit_open = true;
}

fn open_json_editor(
    state: &mut WizardState,
    tp_file: &str,
    component_id: &str,
    component_label: &str,
    mod_name: &str,
) {
    let component_key = compose_component_key(tp_file, component_id);
    state.step3.prompt_edit_key = component_key.clone();
    state.step3.prompt_edit_component_key = component_key;
    state.step3.prompt_edit_tp2_file = normalize_tp2_filename(tp_file);
    state.step3.prompt_edit_component_id = component_id.trim().to_string();
    state.step3.prompt_edit_component_name = component_label.to_string();
    state.step3.prompt_edit_mod_name = mod_name.to_string();

    let (answer, raw_line) = if let Some(item) = find_component_mut(state, tp_file, component_id) {
        (
            extract_wlb_inputs(&item.raw_line).unwrap_or_default(),
            effective_raw_line(item),
        )
    } else {
        (String::new(), String::new())
    };

    let entry = AdvancedPromptEntry {
        tp2_file: normalize_tp2_filename(tp_file),
        component_id: component_id.trim().to_string(),
        component_name: component_label.to_string(),
        mod_name: mod_name.to_string(),
        answer,
        raw_line,
    };

    state.step3.prompt_edit_json = serde_json::to_string_pretty(&entry).unwrap_or_default();
    state.step3.prompt_edit_status.clear();
    state.step3.prompt_edit_mode = "json".to_string();
    state.step3.prompt_edit_open = true;
}

fn clear_prompt_data(state: &mut WizardState, tp_file: &str, component_id: &str) {
    if let Some(item) = find_component_mut(state, tp_file, component_id) {
        item.raw_line = strip_wlb_marker(&effective_raw_line(item));
        state.step5.last_status_text = format!(
            "Cleared @wlb-inputs for {} #{}",
            item.mod_name, item.component_id
        );
    }
}

fn find_component_mut<'a>(
    state: &'a mut WizardState,
    tp_file: &str,
    component_id: &str,
) -> Option<&'a mut Step3ItemState> {
    let tp_norm = normalize_tp2_filename(tp_file);
    let id_norm = component_id.trim();
    let items = if state.step3.active_game_tab == "BGEE" {
        &mut state.step3.bgee_items
    } else {
        &mut state.step3.bg2ee_items
    };
    items.iter_mut().find(|item| {
        !item.is_parent
            && normalize_tp2_filename(&item.tp_file) == tp_norm
            && item.component_id.trim() == id_norm
    })
}

fn default_raw_line(item: &Step3ItemState) -> String {
    let folder = item.mod_name.replace('/', "\\");
    format!(
        "~{}\\{}~ #0 #{} // {}",
        folder, item.tp_file, item.component_id, item.component_label
    )
}

fn effective_raw_line(item: &Step3ItemState) -> String {
    if item.raw_line.trim().is_empty() {
        default_raw_line(item)
    } else {
        item.raw_line.trim().to_string()
    }
}

fn extract_wlb_inputs(raw_line: &str) -> Option<String> {
    let lower = raw_line.to_ascii_lowercase();
    let start = lower.find(WLB_MARKER)?;
    let value = raw_line[start + WLB_MARKER.len()..].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn strip_wlb_marker(raw_line: &str) -> String {
    let lower = raw_line.to_ascii_lowercase();
    if let Some(start) = lower.find(WLB_MARKER) {
        let mut head = raw_line[..start].to_string();
        while head.ends_with(' ') || head.ends_with('\t') {
            head.pop();
        }
        if head.ends_with("//") {
            head.truncate(head.len().saturating_sub(2));
            while head.ends_with(' ') || head.ends_with('\t') {
                head.pop();
            }
        }
        head
    } else {
        raw_line.trim().to_string()
    }
}

fn set_wlb_inputs(raw_line: &str, answer: &str) -> String {
    let base = strip_wlb_marker(raw_line);
    if answer.trim().is_empty() {
        return base;
    }
    format!("{base} // @wlb-inputs: {}", answer.trim())
}
