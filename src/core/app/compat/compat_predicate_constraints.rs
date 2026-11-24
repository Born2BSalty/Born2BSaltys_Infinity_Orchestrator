// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::app::compat_mismatch_scan::{
    RequirementGuard, guards_from_loaded_text, ordinary_guards_from_text,
};
use crate::app::state::{Step1State, Step2ModState, Step3ItemState};

const ASSET: &str = "builtin_predicate_constraints.toml";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::app) struct PredicateConstraint {
    pub(crate) id: String,
    pub(crate) provenance: String,
    pub(crate) source_aliases: Vec<String>,
    pub(crate) components: Vec<String>,
    pub(crate) mode: String,
    pub(crate) tab: String,
    pub(crate) semantics: String,
    pub(crate) guard: String,
    pub(crate) target_aliases: Vec<String>,
    pub(crate) target_components: Vec<String>,
}

#[derive(Debug, Default)]
pub(in crate::app) struct ConstraintMetadata {
    pub(crate) records: Vec<PredicateConstraint>,
    pub(crate) diagnostics: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConstraintDocument {
    schema_version: u32,
    constraints: Vec<toml::Value>,
}

pub(in crate::app) fn builtin_metadata() -> ConstraintMetadata {
    metadata_from_text(include_str!(
        "../../config/builtin_predicate_constraints.toml"
    ))
}

pub(in crate::app) fn metadata_from_text(text: &str) -> ConstraintMetadata {
    let mut metadata = ConstraintMetadata::default();
    let document = match toml::from_str::<ConstraintDocument>(text) {
        Ok(document) if document.schema_version == 1 => document,
        Ok(_) => {
            metadata
                .diagnostics
                .push(format!("{ASSET}: unsupported schema version"));
            return metadata;
        }
        Err(error) => {
            metadata.diagnostics.push(format!("{ASSET}: {error}"));
            return metadata;
        }
    };
    for (index, value) in document.constraints.into_iter().enumerate() {
        match value.try_into::<PredicateConstraint>() {
            Ok(mut record) => {
                if validate_record(&mut record) {
                    metadata.records.push(record);
                } else {
                    metadata
                        .diagnostics
                        .push(format!("{ASSET}: rejected record {}", record.id));
                }
            }
            Err(error) => metadata
                .diagnostics
                .push(format!("{ASSET}: record {index}: {error}")),
        }
    }
    resolve_overlaps(&mut metadata);
    metadata.records.retain(|record| {
        let valid = record.semantics == "install_before" && guard_matches(record, &record.guard);
        if !valid {
            metadata.diagnostics.push(format!(
                "{ASSET}: rejected semantics/targets in {}",
                record.id
            ));
        }
        valid
    });
    metadata
}

fn validate_record(record: &mut PredicateConstraint) -> bool {
    if record.id.trim().is_empty()
        || record.provenance.trim().is_empty()
        || record.mode.is_empty()
        || record.tab.is_empty()
        || record.mode.contains('*')
        || record.tab.contains('*')
        || record.semantics.is_empty()
        || !valid_aliases(&record.source_aliases)
        || !valid_aliases(&record.target_aliases)
        || !normalize_ids(&mut record.components)
        || !normalize_ids(&mut record.target_components)
    {
        return false;
    }
    strict_atoms(&record.guard).is_some()
}

fn normalize_ids(ids: &mut Vec<String>) -> bool {
    if ids.is_empty() {
        return false;
    }
    let Some(normalized) = ids
        .iter()
        .map(|id| canonical_id(id))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let mut unique = normalized.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != normalized.len() {
        return false;
    }
    *ids = normalized;
    true
}

fn valid_aliases(aliases: &[String]) -> bool {
    if aliases.len() != 2
        || aliases
            .iter()
            .any(|alias| alias.contains(['/', '\\', '*', '%']))
    {
        return false;
    }
    let mut aliases = aliases
        .iter()
        .map(|alias| alias.to_ascii_lowercase())
        .collect::<Vec<_>>();
    aliases.sort();
    aliases.iter().any(|base| {
        !base.starts_with("setup-")
            && Path::new(base)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tp2"))
            && aliases
                .iter()
                .any(|other| *other == format!("setup-{base}"))
    })
}

fn resolve_overlaps(metadata: &mut ConstraintMetadata) {
    let mut blocked = HashSet::new();
    for (index, left) in metadata.records.iter().enumerate() {
        for right in metadata.records.iter().skip(index + 1) {
            if !same_binding(left, right) {
                continue;
            }
            for component in left
                .components
                .iter()
                .filter(|id| right.components.contains(id))
            {
                if !same_permission(left, right) {
                    blocked.insert((left.id.clone(), component.clone()));
                    blocked.insert((right.id.clone(), component.clone()));
                    metadata.diagnostics.push(format!(
                        "{ASSET}: contradictory records {} and {} for #{component}",
                        left.id, right.id
                    ));
                }
            }
        }
    }
    for record in &mut metadata.records {
        record
            .components
            .retain(|id| !blocked.contains(&(record.id.clone(), id.clone())));
    }
    metadata
        .records
        .retain(|record| !record.components.is_empty());
    let mut seen = HashSet::new();
    for record in &mut metadata.records {
        let components = record
            .components
            .iter()
            .filter(|id| seen.insert(binding_key(record, id)))
            .cloned()
            .collect();
        record.components = components;
    }
    metadata
        .records
        .retain(|record| !record.components.is_empty());
}

fn binding_key(record: &PredicateConstraint, component: &str) -> String {
    format!(
        "{:?}|{}|{}|{}|{:?}",
        normalized_aliases(&record.source_aliases),
        component,
        record.mode.to_ascii_uppercase(),
        record.tab.to_ascii_uppercase(),
        strict_atoms(&record.guard)
    )
}

fn same_binding(left: &PredicateConstraint, right: &PredicateConstraint) -> bool {
    normalized_aliases(&left.source_aliases) == normalized_aliases(&right.source_aliases)
        && left.mode.eq_ignore_ascii_case(&right.mode)
        && left.tab.eq_ignore_ascii_case(&right.tab)
        && strict_atoms(&left.guard) == strict_atoms(&right.guard)
}

fn same_permission(left: &PredicateConstraint, right: &PredicateConstraint) -> bool {
    left.semantics == right.semantics
        && normalized_aliases(&left.target_aliases) == normalized_aliases(&right.target_aliases)
        && left.target_components.iter().collect::<HashSet<_>>()
            == right.target_components.iter().collect::<HashSet<_>>()
}

fn normalized_aliases(aliases: &[String]) -> Vec<String> {
    let mut normalized = aliases
        .iter()
        .map(|alias| alias.to_ascii_lowercase())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized
}

pub(in crate::app) fn canonical_id(text: &str) -> Option<String> {
    (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| text.parse::<u32>().ok().map(|id| id.to_string()))
        .flatten()
}

fn basename(text: &str) -> &str {
    text.rsplit(['/', '\\']).next().unwrap_or(text)
}

fn alias_matches(aliases: &[String], identity: &str) -> bool {
    aliases
        .iter()
        .any(|alias| alias.eq_ignore_ascii_case(basename(identity)))
}

pub(in crate::app) fn guard_matches(record: &PredicateConstraint, text: &str) -> bool {
    strict_atoms(text).is_some_and(|atoms| atoms_match(record, &atoms))
}

fn atoms_match(record: &PredicateConstraint, atoms: &[(String, String)]) -> bool {
    atoms.len() == record.target_components.len()
        && atoms.iter().all(|(identity, id)| {
            alias_matches(&record.target_aliases, identity) && record.target_components.contains(id)
        })
        && atoms.iter().map(|(_, id)| id).collect::<HashSet<_>>().len() == atoms.len()
}

#[derive(Debug, PartialEq, Eq)]
enum StrictToken {
    Word(String),
    Literal(String),
    Open,
    Close,
}

fn strict_tokens(text: &str) -> Option<Vec<StrictToken>> {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    let mut tokens = Vec::new();
    while let Some(&byte) = bytes.get(cursor) {
        if byte.is_ascii_whitespace() {
            cursor += 1;
        } else if byte == b'(' || byte == b')' {
            tokens.push(if byte == b'(' {
                StrictToken::Open
            } else {
                StrictToken::Close
            });
            cursor += 1;
        } else if byte == b'~' || byte == b'"' {
            let start = cursor + 1;
            cursor = start;
            while *bytes.get(cursor)? != byte {
                if matches!(bytes[cursor], b'%' | b'$' | b'"' | b'~') {
                    return None;
                }
                cursor += 1;
            }
            tokens.push(StrictToken::Literal(text[start..cursor].to_string()));
            cursor += 1;
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = cursor;
            while bytes
                .get(cursor)
                .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
            {
                cursor += 1;
            }
            tokens.push(StrictToken::Word(text[start..cursor].to_ascii_uppercase()));
        } else {
            return None;
        }
    }
    Some(tokens)
}

fn strict_atoms(text: &str) -> Option<Vec<(String, String)>> {
    let tokens = strict_tokens(text)?;
    let mut cursor = 0;
    let mut atoms = Vec::new();
    parse_conjunction(&tokens, &mut cursor, &mut atoms, 0)?;
    if cursor != tokens.len() {
        return None;
    }
    atoms.sort();
    Some(atoms)
}

fn parse_conjunction(
    tokens: &[StrictToken],
    cursor: &mut usize,
    atoms: &mut Vec<(String, String)>,
    depth: usize,
) -> Option<()> {
    if depth > 64 {
        return None;
    }
    loop {
        if tokens.get(*cursor) == Some(&StrictToken::Open) {
            *cursor += 1;
            parse_conjunction(tokens, cursor, atoms, depth + 1)?;
            if tokens.get(*cursor) != Some(&StrictToken::Close) {
                return None;
            }
            *cursor += 1;
        } else {
            let [
                StrictToken::Word(negative),
                StrictToken::Word(call),
                StrictToken::Literal(identity),
                StrictToken::Literal(id),
            ] = tokens.get(*cursor..*cursor + 4)?
            else {
                return None;
            };
            if negative != "NOT" || call != "MOD_IS_INSTALLED" {
                return None;
            }
            atoms.push((basename(identity).to_ascii_lowercase(), canonical_id(id)?));
            *cursor += 4;
        }
        if tokens.get(*cursor) != Some(&StrictToken::Word("AND".to_string())) {
            return Some(());
        }
        *cursor += 1;
    }
}

#[derive(Debug, Clone)]
pub(in crate::app) struct ConstraintOutcome {
    pub(crate) row_index: usize,
    pub(crate) kind: &'static str,
    pub(crate) source: String,
    pub(crate) message: String,
    pub(crate) raw_evidence: String,
    pub(crate) record_id: String,
    pub(crate) related_mod: Option<String>,
    pub(crate) related_component: Option<String>,
}

pub(in crate::app) struct CoveredSource {
    pub(crate) path: String,
    pub(crate) guards: Vec<RequirementGuard>,
    pub(crate) matched: Vec<usize>,
    pub(crate) records: Vec<PredicateConstraint>,
    pub(crate) error: Option<String>,
}

#[derive(Default)]
pub(in crate::app) struct ConstraintSnapshot {
    pub(crate) rows: HashMap<usize, CoveredSource>,
    pub(crate) outcomes: Vec<ConstraintOutcome>,
    pub(crate) diagnostics: Vec<String>,
}

pub(in crate::app) fn covered_records<'a>(
    metadata: &'a ConstraintMetadata,
    step1: &Step1State,
    tab: &str,
    identity: &str,
    component: &str,
) -> Vec<&'a PredicateConstraint> {
    let Some(component) = canonical_id(component) else {
        return Vec::new();
    };
    metadata
        .records
        .iter()
        .filter(|record| {
            record.mode.eq_ignore_ascii_case(&step1.game_install)
                && record.tab.eq_ignore_ascii_case(tab)
                && alias_matches(&record.source_aliases, identity)
                && record.components.contains(&component)
        })
        .collect()
}

pub(in crate::app) struct LoadedSource {
    pub(crate) guards: HashMap<String, Vec<RequirementGuard>>,
    error: Option<String>,
}

pub(in crate::app) type LoadedSources = HashMap<PathBuf, LoadedSource>;

pub(in crate::app) fn resolve_source(
    records: &[&PredicateConstraint],
    identity: &str,
    component: &str,
    mods: &[Step2ModState],
    loaded: &mut LoadedSources,
) -> CoveredSource {
    resolve_source_with_reader(records, identity, component, mods, loaded, &mut |path| {
        super::compat_text::read_compat_text(path).map_err(|error| error.to_string())
    })
}

fn resolve_source_with_reader(
    records: &[&PredicateConstraint],
    identity: &str,
    component: &str,
    mods: &[Step2ModState],
    loaded: &mut LoadedSources,
    reader: &mut impl FnMut(&Path) -> Result<String, String>,
) -> CoveredSource {
    let mut source = CoveredSource {
        path: identity.to_string(),
        guards: Vec::new(),
        matched: Vec::new(),
        records: records.iter().map(|record| (*record).clone()).collect(),
        error: None,
    };
    match resolve_path(records, component, mods) {
        Ok(path) => {
            source.path = path.to_string_lossy().into_owned();
            let loaded = loaded
                .entry(path.clone())
                .or_insert_with(|| load_source(&path, reader));
            source.error.clone_from(&loaded.error);
            if let Some(guards) = canonical_id(component).and_then(|id| loaded.guards.get(&id)) {
                source.guards.clone_from(guards);
                if source.error.is_none() {
                    source.matched = source
                        .guards
                        .iter()
                        .enumerate()
                        .filter_map(|(index, guard)| {
                            records
                                .iter()
                                .any(|record| guard_matches(record, guard.source_expression()))
                                .then_some(index)
                        })
                        .collect();
                }
            } else if source.error.is_none() {
                source.error = Some("covered component unavailable".to_string());
            }
        }
        Err(error) => source.error = Some(error),
    }
    source
}

fn load_source(
    path: &Path,
    reader: &mut impl FnMut(&Path) -> Result<String, String>,
) -> LoadedSource {
    match reader(path) {
        Ok(text) => match guards_from_loaded_text(&text) {
            Ok(guards) => LoadedSource {
                guards,
                error: None,
            },
            Err(error) => LoadedSource {
                guards: ordinary_guards_from_text(&text),
                error: Some(error),
            },
        },
        Err(error) => LoadedSource {
            guards: HashMap::new(),
            error: Some(error),
        },
    }
}

fn resolve_path(
    records: &[&PredicateConstraint],
    component: &str,
    mods: &[Step2ModState],
) -> Result<PathBuf, String> {
    let mut paths = HashSet::new();
    for candidate in mods.iter().filter(|candidate| {
        records.iter().any(|record| {
            alias_matches(&record.source_aliases, &candidate.tp_file)
                && candidate
                    .components
                    .iter()
                    .any(|value| canonical_id(&value.component_id) == canonical_id(component))
        })
    }) {
        if candidate.tp2_path.is_empty()
            || !records
                .iter()
                .any(|record| alias_matches(&record.source_aliases, &candidate.tp2_path))
        {
            return Err("missing or mismatched MOD SOURCE TP2 path".to_string());
        }
        let path = Path::new(&candidate.tp2_path)
            .canonicalize()
            .map_err(|error| format!("{}: {error}", candidate.tp2_path))?;
        if !records
            .iter()
            .any(|record| alias_matches(&record.source_aliases, &path.to_string_lossy()))
        {
            return Err("canonical MOD SOURCE TP2 basename mismatch".to_string());
        }
        paths.insert(path);
    }
    if paths.len() != 1 {
        return Err(format!(
            "expected one physical MOD SOURCE TP2, found {}",
            paths.len()
        ));
    }
    paths
        .into_iter()
        .next()
        .ok_or_else(|| "MOD SOURCE TP2 unavailable".to_string())
}

pub(in crate::app) fn planning_snapshot(
    step1: &Step1State,
    tab: &str,
    mods: &[Step2ModState],
    items: &[Step3ItemState],
    metadata: &ConstraintMetadata,
) -> ConstraintSnapshot {
    planning_snapshot_with_reader(step1, tab, mods, items, metadata, &mut |path| {
        super::compat_text::read_compat_text(path).map_err(|error| error.to_string())
    })
}

pub(in crate::app) fn planning_snapshot_with_reader(
    step1: &Step1State,
    tab: &str,
    mods: &[Step2ModState],
    items: &[Step3ItemState],
    metadata: &ConstraintMetadata,
    reader: &mut impl FnMut(&Path) -> Result<String, String>,
) -> ConstraintSnapshot {
    let mut snapshot = ConstraintSnapshot {
        diagnostics: metadata.diagnostics.clone(),
        ..ConstraintSnapshot::default()
    };
    let mut loaded = LoadedSources::new();
    for (row_index, item) in items.iter().enumerate().filter(|(_, item)| !item.is_parent) {
        let records = covered_records(metadata, step1, tab, &item.tp_file, &item.component_id);
        if records.is_empty() {
            continue;
        }
        let source = resolve_source_with_reader(
            &records,
            &item.tp_file,
            &item.component_id,
            mods,
            &mut loaded,
            reader,
        );
        if let Some(error) = &source.error {
            snapshot.outcomes.push(ConstraintOutcome {
                row_index,
                kind: "source_unverified",
                source: source.path.clone(),
                message: format!(
                    "source_unverified: {} #{}: {error}",
                    item.tp_file, item.component_id
                ),
                raw_evidence: format!(
                    "{ASSET}: {}",
                    records
                        .iter()
                        .map(|record| record.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                record_id: records[0].id.clone(),
                related_mod: None,
                related_component: None,
            });
        } else {
            collect_order_outcomes(&mut snapshot.outcomes, row_index, &source, items);
        }
        snapshot.rows.insert(row_index, source);
    }
    snapshot
}

fn collect_order_outcomes(
    outcomes: &mut Vec<ConstraintOutcome>,
    row_index: usize,
    source: &CoveredSource,
    items: &[Step3ItemState],
) {
    for &guard_index in &source.matched {
        let guard = &source.guards[guard_index];
        for record in source
            .records
            .iter()
            .filter(|record| guard_matches(record, guard.source_expression()))
        {
            for (target_index, target) in items.iter().enumerate().filter(|(index, target)| {
                *index < row_index
                    && !target.is_parent
                    && alias_matches(&record.target_aliases, &target.tp_file)
                    && canonical_id(&target.component_id)
                        .is_some_and(|id| record.target_components.contains(&id))
            }) {
                outcomes.push(ConstraintOutcome {
                    row_index,
                    kind: "order_violation",
                    source: source.path.clone(),
                    message: format!(
                        "Must be before {} #{} (plan row {}); {}.",
                        target.tp_file,
                        target.component_id,
                        target_index + 1,
                        record.provenance
                    ),
                    raw_evidence: format!("{ASSET}: {}\n{}", record.id, guard.display_line),
                    record_id: record.id.clone(),
                    related_mod: Some(target.tp_file.clone()),
                    related_component: canonical_id(&target.component_id),
                });
            }
        }
    }
}
