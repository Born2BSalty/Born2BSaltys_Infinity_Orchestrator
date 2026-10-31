// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use crate::app::compat_tp2_blocks::{
    ComponentFlags, FamilyIdentity, PredicateFlag, PredicateFlagKind, collect_component_flags,
};

use super::super::compat_mismatch_eval::render_requirement_evidence;

#[derive(Debug, Clone)]
pub(super) struct GuardOrigin {
    component_id: String,
    line: usize,
    kind: PredicateFlagKind,
    inherited: bool,
}

#[derive(Debug, Clone)]
pub(super) struct RequirementGuard {
    pub(super) display_line: String,
    pub(super) eval_text: String,
    pub(super) origin: Option<GuardOrigin>,
}

impl RequirementGuard {
    pub(super) fn inherited_provenance(&self) -> Option<String> {
        let origin = self.origin.as_ref().filter(|origin| origin.inherited)?;
        Some(format!(
            "Shared requirement inherited from first subcomponent #{}, TP2 line {}.",
            origin.component_id, origin.line
        ))
    }
}

pub(super) fn load_component_guards(tp2_path: &str) -> HashMap<String, Vec<RequirementGuard>> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let cache = requirement_guard_file_cache();
    let mut cache = cache.lock().expect("compat mismatch cache lock poisoned");
    let stamp = cache_stamp(tp2_path);
    if let Some(entry) = cache.get(tp2_path)
        && entry.stamp == stamp
    {
        return entry.guards.clone();
    }
    let guards = load_component_guards_uncached(tp2_path);
    cache.insert(
        tp2_path.to_string(),
        CachedRequirementGuards {
            stamp,
            guards: guards.clone(),
        },
    );
    guards
}

fn load_component_guards_uncached(tp2_path: &str) -> HashMap<String, Vec<RequirementGuard>> {
    let Ok(tp2_text) = crate::app::compat_text::read_compat_text(std::path::Path::new(tp2_path))
    else {
        return HashMap::new();
    };
    let mut first_requirements = HashMap::<FamilyIdentity, Vec<RequirementGuard>>::new();
    let mut out = HashMap::<String, Vec<RequirementGuard>>::new();
    for component in collect_component_flags(&tp2_text) {
        let mut guards = component_guards(&component);
        if let Some(family) = component.family {
            if let Some(inherited) = first_requirements.get(&family) {
                guards.extend(inherited.iter().cloned().map(|mut guard| {
                    if let Some(origin) = &mut guard.origin {
                        origin.inherited = true;
                    }
                    guard
                }));
            } else {
                first_requirements.insert(
                    family,
                    guards
                        .iter()
                        .filter(|guard| {
                            guard
                                .origin
                                .as_ref()
                                .is_some_and(|origin| origin.kind == PredicateFlagKind::Requirement)
                        })
                        .cloned()
                        .collect(),
                );
            }
        }
        if !guards.is_empty() {
            out.insert(component.component_id, guards);
        }
    }
    out
}

fn component_guards(component: &ComponentFlags) -> Vec<RequirementGuard> {
    component
        .predicates
        .iter()
        .map(|flag| requirement_guard(&component.component_id, flag))
        .collect()
}

fn requirement_guard(component_id: &str, flag: &PredicateFlag) -> RequirementGuard {
    let display_expression =
        render_requirement_evidence(&flag.expression).unwrap_or_else(|| flag.expression.clone());
    RequirementGuard {
        display_line: format!("{} {display_expression}", flag.prefix),
        eval_text: flag.expression.clone(),
        origin: Some(GuardOrigin {
            component_id: component_id.to_string(),
            line: flag.line,
            kind: flag.kind,
            inherited: false,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCacheStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct CachedRequirementGuards {
    stamp: FileCacheStamp,
    guards: HashMap<String, Vec<RequirementGuard>>,
}

fn requirement_guard_file_cache() -> &'static Mutex<HashMap<String, CachedRequirementGuards>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedRequirementGuards>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_stamp(tp2_path: &str) -> FileCacheStamp {
    fs::metadata(tp2_path).map_or(
        FileCacheStamp {
            modified: None,
            len: 0,
        },
        |meta| FileCacheStamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        },
    )
}

#[cfg(test)]
pub(super) fn collect_requirement_guards(block: &[&str]) -> Vec<RequirementGuard> {
    let text = block.join("\n");
    let components = collect_component_flags(&text);
    components.first().map_or_else(Vec::new, component_guards)
}
