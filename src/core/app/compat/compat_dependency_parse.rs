// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use crate::parser::collect_tp2_component_blocks;
use crate::parser::compat_dependency_expr::{
    ParsedComponentRequirement, ParsedDependencyTarget, collect_component_requirements,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComponentRequirement {
    pub(crate) raw_line: String,
    pub(crate) targets: Vec<ComponentRequirementTarget>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComponentRequirementTarget {
    pub(crate) target_mod: String,
    pub(crate) target_component_id: String,
}

pub(crate) fn load_component_requirements(
    tp2_path: &str,
) -> HashMap<String, Vec<ComponentRequirement>> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let cache = requirement_cache();
    let mut cache = cache.lock().expect("compat dependency cache lock poisoned");
    let stamp = cache_stamp(tp2_path);

    if let Some(entry) = cache.get(tp2_path)
        && entry.stamp == stamp
    {
        return entry.requirements.clone();
    }

    let requirements = load_component_requirements_uncached(tp2_path);
    cache.insert(
        tp2_path.to_string(),
        CachedRequirements {
            stamp,
            requirements: requirements.clone(),
        },
    );
    requirements
}

fn load_component_requirements_uncached(
    tp2_path: &str,
) -> HashMap<String, Vec<ComponentRequirement>> {
    let Ok(tp2_text) = fs::read_to_string(tp2_path) else {
        return HashMap::new();
    };

    let mut out = HashMap::<String, Vec<ComponentRequirement>>::new();
    for (component_id, block) in collect_tp2_component_blocks(&tp2_text) {
        let requirements = collect_component_requirements(&block)
            .into_iter()
            .map(parsed_requirement_to_core)
            .collect::<Vec<_>>();
        if !requirements.is_empty() {
            out.insert(component_id, requirements);
        }
    }

    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCacheStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct CachedRequirements {
    stamp: FileCacheStamp,
    requirements: HashMap<String, Vec<ComponentRequirement>>,
}

fn requirement_cache() -> &'static Mutex<HashMap<String, CachedRequirements>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedRequirements>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_stamp(tp2_path: &str) -> FileCacheStamp {
    match fs::metadata(tp2_path) {
        Ok(meta) => FileCacheStamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        },
        Err(_) => FileCacheStamp {
            modified: None,
            len: 0,
        },
    }
}

fn parsed_requirement_to_core(parsed: ParsedComponentRequirement) -> ComponentRequirement {
    ComponentRequirement {
        raw_line: parsed.raw_line,
        targets: parsed
            .targets
            .into_iter()
            .map(parsed_target_to_core)
            .collect(),
        message: parsed.message,
    }
}

fn parsed_target_to_core(parsed: ParsedDependencyTarget) -> ComponentRequirementTarget {
    ComponentRequirementTarget {
        target_mod: parsed.target_mod,
        target_component_id: parsed.target_component_id,
    }
}

#[cfg(test)]
#[path = "compat_dependency_parse_tests.rs"]
mod tests;
