// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::game_authority;
use crate::app::state::{Step1State, Step2ComponentState, Step2ModState};

use super::compat_conflict_scan::apply_step2_scan_conflict;
use super::compat_deprecated_scan::apply_step2_scan_deprecated;
use super::compat_mismatch_scan::apply_step2_scan_mismatch;
use super::compat_missing_dep_scan::apply_step2_scan_missing_dep;
use super::compat_path_scan::apply_step2_scan_path_requirement;
use super::compat_rule_runtime::{
    active_item_order, clear_kind_matches, collect_step2_active_items, compat_component_matches,
    compat_mod_matches, direct_rule_applies, first_related_target, kind_disables_selection,
    match_kind_matches, matched_related_target, mode_matches, non_empty, normalize_kind,
    normalize_mod_key, relation_rule_applies, single_related_target, tab_matches,
};
use super::compat_rules::{CompatRule, compat_rule_source_path, load_rules};

pub(crate) fn apply_step2_compat_rules(
    step1: &Step1State,
    first_game_mods: &mut [Step2ModState],
    second_game_mods: &mut [Step2ModState],
) -> Option<String> {
    clear_step2_compat_state(first_game_mods);
    clear_step2_compat_state(second_game_mods);

    let first_tab = game_authority::first_slot_tab(&step1.game_install);

    apply_step2_scan_mismatch(step1, first_tab, first_game_mods);
    apply_step2_scan_mismatch(step1, "BG2EE", second_game_mods);
    apply_step2_scan_path_requirement(step1, first_tab, first_game_mods);
    apply_step2_scan_path_requirement(step1, "BG2EE", second_game_mods);
    apply_step2_scan_missing_dep(first_game_mods);
    apply_step2_scan_missing_dep(second_game_mods);
    apply_step2_scan_conflict(first_game_mods);
    apply_step2_scan_conflict(second_game_mods);
    apply_step2_scan_deprecated(first_game_mods);
    apply_step2_scan_deprecated(second_game_mods);

    let loaded = load_rules();
    let rules = loaded.rules;
    if !rules.is_empty() {
        apply_direct_rules_to_tab(step1, first_tab, &rules, first_game_mods);
        apply_direct_rules_to_tab(step1, "BG2EE", &rules, second_game_mods);
        finalize_step2_compat_state(first_game_mods);
        finalize_step2_compat_state(second_game_mods);

        apply_relation_rules_to_tab(step1, first_tab, &rules, first_game_mods);
        apply_relation_rules_to_tab(step1, "BG2EE", &rules, second_game_mods);
    }

    super::compat_dlc_source::apply_step2(step1, first_game_mods, second_game_mods);
    finalize_step2_compat_state(first_game_mods);
    finalize_step2_compat_state(second_game_mods);
    loaded.error
}

pub(crate) fn clear_step2_compat_state(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            component.disabled = false;
            component.compat_kind = None;
            component.compat_source = None;
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.compat_graph = None;
            component.compat_evidence = None;
            component.disabled_reason = None;
        }
        refresh_mod_checked_state(mod_state);
    }
}

fn apply_direct_rules_to_tab(
    step1: &Step1State,
    tab: &str,
    rules: &[CompatRule],
    mods: &mut [Step2ModState],
) {
    for mod_state in mods {
        let tp_file = mod_state.tp_file.clone();
        let mod_name = mod_state.name.clone();
        for component_idx in 0..mod_state.components.len() {
            for rule in rules {
                let component = &mod_state.components[component_idx];
                if !rule_selector_matches(rule, step1, tab, &tp_file, &mod_name, component) {
                    continue;
                }
                let component = &mut mod_state.components[component_idx];
                clear_rule_kinds(rule, component);
                if !direct_rule_applies(rule, step1, tab) {
                    continue;
                }
                apply_rule(rule, &tp_file, component, single_related_target(rule));
            }
        }
    }
}

fn apply_relation_rules_to_tab(
    step1: &Step1State,
    tab: &str,
    rules: &[CompatRule],
    mods: &mut [Step2ModState],
) {
    let active_items = collect_step2_active_items(mods);
    for mod_state in mods {
        let tp_file = mod_state.tp_file.clone();
        let mod_name = mod_state.name.clone();
        for component_idx in 0..mod_state.components.len() {
            for rule in rules {
                let component = &mod_state.components[component_idx];
                if !rule_selector_matches(rule, step1, tab, &tp_file, &mod_name, component) {
                    continue;
                }
                if normalize_kind(&rule.kind).eq_ignore_ascii_case("order_block") {
                    continue;
                }
                if normalize_kind(&rule.kind).eq_ignore_ascii_case("conflict") && !component.checked
                {
                    continue;
                }
                let component = &mut mod_state.components[component_idx];
                let component_id = component.component_id.clone();
                clear_rule_kinds(rule, component);
                if !relation_rule_applies(
                    rule,
                    &tp_file,
                    &component_id,
                    active_item_order(&active_items, &tp_file, &component_id),
                    &active_items,
                ) {
                    continue;
                }
                apply_rule(
                    rule,
                    &tp_file,
                    component,
                    matched_related_target(rule, &tp_file, &component_id, &active_items)
                        .or_else(|| first_related_target(rule)),
                );
            }
        }
    }
}

fn rule_selector_matches(
    rule: &CompatRule,
    step1: &Step1State,
    tab: &str,
    tp_file: &str,
    mod_name: &str,
    component: &Step2ComponentState,
) -> bool {
    mode_matches(rule, &step1.game_install)
        && tab_matches(rule, tab)
        && match_kind_matches(rule.match_kind.as_ref(), component.compat_kind.as_deref())
        && compat_mod_matches(rule, tp_file, mod_name)
        && compat_component_matches(
            rule,
            &component.component_id,
            &component.label,
            &component.raw_line,
        )
}

fn finalize_step2_compat_state(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            if component.disabled || component_should_disable_selection(component) {
                component.disabled = true;
                component.checked = false;
                component.selected_order = None;
            }
        }
        refresh_mod_checked_state(mod_state);
    }
}

fn component_should_disable_selection(component: &Step2ComponentState) -> bool {
    let Some(kind) = component.compat_kind.as_deref() else {
        return false;
    };
    if kind.eq_ignore_ascii_case("conflict")
        && component.compat_evidence.as_deref().is_some_and(|raw| {
            raw.trim_start()
                .to_ascii_uppercase()
                .starts_with("FORBID_COMPONENT")
        })
    {
        return false;
    }
    kind_disables_selection(kind)
}

fn refresh_mod_checked_state(mod_state: &mut Step2ModState) {
    mod_state.checked = mod_state
        .components
        .iter()
        .filter(|component| !component.disabled)
        .any(|component| component.checked)
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
}

fn apply_rule(
    rule: &CompatRule,
    tp_file: &str,
    component: &mut Step2ComponentState,
    related_target: Option<(String, Option<String>)>,
) {
    let kind = normalize_kind(&rule.kind);
    if kind.eq_ignore_ascii_case("allow") {
        clear_component_compat(component);
        return;
    }
    component.disabled = false;
    component.compat_kind = Some(kind.to_string());
    component.compat_source = Some(compat_rule_source_path(rule));
    component.compat_related_mod = related_target
        .as_ref()
        .map(|(related_mod, _)| related_mod.clone());
    component.compat_related_component =
        related_target.and_then(|(_, related_component)| related_component);
    component.compat_graph = build_graph(tp_file, component.component_id.as_str(), kind, rule);
    component.compat_evidence = Some(rule_debug_line(rule));
    component.disabled_reason = non_empty(Some(rule.message.as_str()));
    if kind.eq_ignore_ascii_case("deprecated") || kind_disables_selection(kind) {
        component.disabled = true;
    }
}

fn clear_rule_kinds(rule: &CompatRule, component: &mut Step2ComponentState) {
    if clear_kind_matches(rule.clear_kinds.as_ref(), component.compat_kind.as_deref()) {
        clear_component_compat(component);
    }
}

fn clear_component_compat(component: &mut Step2ComponentState) {
    component.disabled = false;
    component.compat_kind = None;
    component.compat_source = None;
    component.compat_related_mod = None;
    component.compat_related_component = None;
    component.compat_graph = None;
    component.compat_evidence = None;
    component.disabled_reason = None;
}

fn build_graph(tp_file: &str, component_id: &str, kind: &str, rule: &CompatRule) -> Option<String> {
    let (related_mod, related_component) = single_related_target(rule)?;
    let left = format!("{} #{}", normalize_mod_key(tp_file), component_id.trim());
    let right = if related_component.as_deref().is_none_or(str::is_empty) {
        normalize_mod_key(&related_mod)
    } else {
        format!(
            "{} #{}",
            normalize_mod_key(&related_mod),
            related_component.unwrap_or_default()
        )
    };
    Some(format!("{left} {kind} {right}"))
}

fn rule_debug_line(rule: &CompatRule) -> String {
    let mut parts = vec![format!("kind={}", rule.kind.trim())];
    let mod_items = rule.r#mod.trimmed_items();
    if !mod_items.is_empty() {
        parts.insert(0, format!("mod={}", mod_items.join(",")));
    }
    if let Some(component) = rule.component.as_ref() {
        let items = component.trimmed_items();
        if !items.is_empty() {
            parts.push(format!("component={}", items.join(",")));
        }
    }
    if let Some(component_id) = rule.component_id.as_ref() {
        let items = component_id.trimmed_items();
        if !items.is_empty() {
            parts.push(format!("component_id={}", items.join(",")));
        }
    }
    if let Some(match_kind) = rule.match_kind.as_ref() {
        parts.push(format!(
            "match_kind={}",
            match_kind.normalized_items().join(",")
        ));
    }
    if let Some(clear_kinds) = rule.clear_kinds.as_ref() {
        parts.push(format!(
            "clear_kinds={}",
            clear_kinds.normalized_items().join(",")
        ));
    }
    if let Some(position) = non_empty(rule.position.as_deref()) {
        parts.push(format!("position={position}"));
    }
    if let Some(path_field) = non_empty(rule.path_field.as_deref()) {
        parts.push(format!("path_field={path_field}"));
    }
    if let Some(game_file) = non_empty(rule.game_file.as_deref()) {
        parts.push(format!("game_file={game_file}"));
    }
    if let Some(game_file_check) = non_empty(rule.game_file_check.as_deref()) {
        parts.push(format!("game_file_check={game_file_check}"));
    }
    if let Some(related_mod) = rule.related_mod.as_ref() {
        let items = related_mod.trimmed_items();
        if !items.is_empty() {
            parts.push(format!("related_mod={}", items.join(",")));
        }
    }
    if let Some(related_component) = rule.related_component.as_ref() {
        let items = related_component.trimmed_items();
        if !items.is_empty() {
            parts.push(format!("related_component={}", items.join(",")));
        }
    }
    if let Some(message) = non_empty(Some(rule.message.as_str())) {
        parts.push(format!("message={message}"));
    }
    parts.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::apply_relation_rules_to_tab;
    use crate::app::compat_rules::{CompatRule, CompatRulesFile, default_step2_rules_content};
    use crate::app::state::{Step1State, Step2ComponentState, Step2ModState};

    fn component(component_id: &str, checked: bool) -> Step2ComponentState {
        Step2ComponentState {
            component_id: component_id.to_string(),
            label: format!("Component {component_id}"),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked,
            selected_order: checked.then_some(1),
        }
    }

    fn mod_with(name: &str, components: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: name.to_string(),
            tp_file: format!("SETUP-{name}.TP2"),
            tp2_path: format!("{name}/SETUP-{name}.TP2"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: components.iter().any(|component| component.checked),
            hidden_components: Vec::new(),
            components,
        }
    }

    fn rules_from(content: &str) -> Vec<CompatRule> {
        toml::from_str::<CompatRulesFile>(content)
            .expect("rules parse")
            .rules
            .into_iter()
            .map(|mut rule| {
                rule.loaded_from = Some("test rules".to_string());
                rule
            })
            .collect()
    }

    fn dependent_issue(mods: &[Step2ModState]) -> &Step2ComponentState {
        &mods[0].components[0]
    }

    #[test]
    fn a_rule_driven_missing_dependency_names_the_unticked_target() {
        let rules = rules_from(
            r#"
[[rules]]
mod = "dependent"
component_id = "0"
kind = "missing_dep"
related_mod = "target"
related_component = "11"
message = "Requires target component 11."
"#,
        );
        let mut mods = vec![
            mod_with("dependent", vec![component("0", true)]),
            mod_with("target", vec![component("11", false)]),
        ];

        apply_relation_rules_to_tab(&Step1State::default(), "BGEE", &rules, &mut mods);

        let issue = dependent_issue(&mods);
        assert_eq!(issue.compat_kind.as_deref(), Some("missing_dep"));
        assert_eq!(issue.compat_related_mod.as_deref(), Some("target"));
        assert_eq!(issue.compat_related_component.as_deref(), Some("11"));
    }

    #[test]
    fn a_rule_driven_conflict_still_names_the_ticked_target() {
        let rules = rules_from(
            r#"
[[rules]]
mod = "dependent"
component_id = "0"
kind = "conflict"
related_mod = "target"
related_component = ["5", "11"]
message = "Conflicts with target."
"#,
        );
        let mut mods = vec![
            mod_with("dependent", vec![component("0", true)]),
            mod_with("target", vec![component("5", false), component("11", true)]),
        ];

        apply_relation_rules_to_tab(&Step1State::default(), "BGEE", &rules, &mut mods);

        let issue = dependent_issue(&mods);
        assert_eq!(issue.compat_kind.as_deref(), Some("conflict"));
        assert_eq!(issue.compat_related_mod.as_deref(), Some("target"));
        assert_eq!(issue.compat_related_component.as_deref(), Some("11"));
    }

    fn has_garrick_rule(
        rules: &[CompatRule],
        kind: &str,
        related_component: &str,
        position: Option<&str>,
    ) -> bool {
        rules.iter().any(|rule| {
            rule.r#mod.trimmed_items() == ["garrick-tt"]
                && rule
                    .component_id
                    .as_ref()
                    .is_some_and(|ids| ids.trimmed_items() == ["0"])
                && rule.kind == kind
                && rule
                    .related_mod
                    .as_ref()
                    .is_some_and(|mods| mods.trimmed_items() == ["bg1npc"])
                && rule
                    .related_component
                    .as_ref()
                    .is_some_and(|ids| ids.trimmed_items() == [related_component])
                && rule.position.as_deref() == position
        })
    }

    #[test]
    fn garrick_tt_requires_and_follows_bg1npc_banters_and_pids() {
        let rules = rules_from(default_step2_rules_content());
        assert!(has_garrick_rule(&rules, "missing_dep", "10", None));
        assert!(has_garrick_rule(&rules, "missing_dep", "200", None));
        assert!(has_garrick_rule(&rules, "order_block", "10", Some("after")));
        assert!(has_garrick_rule(
            &rules,
            "order_block",
            "200",
            Some("after")
        ));
    }
}
