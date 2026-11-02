// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::app::answer_sets::AnswerSets;
use crate::app::game_authority::GameSlot;
use crate::app::language_names::{UNTAGGED_LANGUAGE, display_language_name};
use crate::app::mod_downloads::normalize_mod_download_tp2;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SlotAnswers {
    pub(crate) languages: BTreeMap<String, String>,
    pub(crate) components: BTreeMap<(String, i64), AnswerSets>,
}

impl SlotAnswers {
    fn is_empty(&self) -> bool {
        self.languages.is_empty() && self.components.is_empty()
    }

    fn parse(value: &Value) -> Self {
        let mut slot = Self::default();
        if let Some(languages) = value.get("languages").and_then(Value::as_object) {
            for (tp2, language) in languages {
                let tp2 = normalize_mod_download_tp2(tp2);
                let Some(language) = language.as_str().map(str::trim) else {
                    continue;
                };
                if tp2.is_empty() || language.is_empty() || language == UNTAGGED_LANGUAGE {
                    continue;
                }
                slot.languages.insert(tp2, language.to_string());
            }
        }
        if let Some(components) = value.get("components").and_then(Value::as_array) {
            for entry in components {
                let Some((key, sets)) = parse_component(entry) else {
                    continue;
                };
                slot.components.entry(key).or_insert(sets);
            }
        }
        slot
    }

    fn to_value(&self) -> Value {
        let mut slot = Map::new();
        if !self.languages.is_empty() {
            let languages = self
                .languages
                .iter()
                .map(|(tp2, language)| (tp2.clone(), Value::String(language.clone())))
                .collect::<Map<_, _>>();
            slot.insert("languages".to_string(), Value::Object(languages));
        }
        if !self.components.is_empty() {
            let components = self
                .components
                .iter()
                .map(|((tp2, id), sets)| {
                    let mut entry = Map::new();
                    entry.insert("tp2".to_string(), Value::String(tp2.clone()));
                    entry.insert("id".to_string(), Value::from(*id));
                    entry.insert("sets".to_string(), sets_value(sets));
                    Value::Object(entry)
                })
                .collect();
            slot.insert("components".to_string(), Value::Array(components));
        }
        Value::Object(slot)
    }
}

fn parse_component(entry: &Value) -> Option<((String, i64), AnswerSets)> {
    let tp2 = normalize_mod_download_tp2(entry.get("tp2")?.as_str()?);
    if tp2.is_empty() {
        return None;
    }
    let id = entry.get("id")?.as_i64()?;
    let mut sets = AnswerSets::default();
    for (language, spec) in entry.get("sets")?.as_object()? {
        let language = language.trim();
        if language.is_empty() {
            continue;
        }
        if let Some(spec) = spec.as_str() {
            sets.set(language, spec);
        }
    }
    (!sets.is_empty()).then_some(((tp2, id), sets))
}

fn sets_value(sets: &AnswerSets) -> Value {
    Value::Object(
        sets.languages()
            .filter_map(|language| {
                sets.get(language)
                    .map(|spec| (language.to_string(), Value::String(spec.to_string())))
            })
            .collect(),
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ShareAnswers(BTreeMap<String, SlotAnswers>);

impl ShareAnswers {
    #[must_use]
    pub(crate) fn parse(value: &Value) -> Self {
        let Some(slots) = value.as_object() else {
            return Self::default();
        };
        Self(
            slots
                .iter()
                .filter(|(key, _)| slot_for_key(key).is_some())
                .map(|(key, slot)| (key.clone(), SlotAnswers::parse(slot)))
                .filter(|(_, slot)| !slot.is_empty())
                .collect(),
        )
    }

    #[must_use]
    pub(crate) fn to_value(&self) -> Option<Value> {
        let slots = self
            .0
            .iter()
            .filter(|(_, slot)| !slot.is_empty())
            .map(|(key, slot)| (key.clone(), slot.to_value()))
            .collect::<Map<_, _>>();
        (!slots.is_empty()).then_some(Value::Object(slots))
    }

    #[must_use]
    pub(crate) fn line_language(&self, slot_key: &str, tp2: &str) -> &str {
        self.0
            .get(slot_key)
            .and_then(|slot| slot.languages.get(&normalize_mod_download_tp2(tp2)))
            .map_or(UNTAGGED_LANGUAGE, String::as_str)
    }

    #[must_use]
    pub(crate) fn component(&self, slot_key: &str, tp2: &str, id: i64) -> Option<&AnswerSets> {
        self.0
            .get(slot_key)
            .and_then(|slot| slot.components.get(&(normalize_mod_download_tp2(tp2), id)))
    }

    pub(crate) fn set_component(&mut self, slot_key: &str, tp2: &str, id: i64, sets: AnswerSets) {
        let key = (normalize_mod_download_tp2(tp2), id);
        let slot = self.0.entry(slot_key.to_string()).or_default();
        if sets.is_empty() {
            slot.components.remove(&key);
        } else {
            slot.components.insert(key, sets);
        }
    }

    pub(crate) fn set_line_language(&mut self, slot_key: &str, tp2: &str, language: &str) {
        let tp2 = normalize_mod_download_tp2(tp2);
        let slot = self.0.entry(slot_key.to_string()).or_default();
        if language.trim().is_empty() || language == UNTAGGED_LANGUAGE {
            slot.languages.remove(&tp2);
        } else {
            slot.languages.insert(tp2, language.to_string());
        }
    }

    pub(crate) fn slots(&self) -> impl Iterator<Item = (&str, &SlotAnswers)> {
        self.0.iter().map(|(key, slot)| (key.as_str(), slot))
    }
}

#[must_use]
pub(crate) fn slot_for_key(key: &str) -> Option<GameSlot> {
    match key {
        "bgee" | "iwdee" => Some(GameSlot::First),
        "bg2ee" => Some(GameSlot::Second),
        _ => None,
    }
}

#[must_use]
pub(crate) fn sets_label(sets: &AnswerSets) -> String {
    sets.languages()
        .filter_map(|language| {
            sets.get(language)
                .map(|spec| format!("{}: {spec}", display_language_name(language)))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}
