// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::app::game_authority::{GameSlot, TAB_BG2EE, TAB_BGEE};
use crate::app::language_names::UNTAGGED_LANGUAGE;
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::state::{Step2ComponentState, Step2ModState, Step2State, WizardState};
use crate::app::wlb_inputs::{marker_spec, parse_identity, replace_marker, strip_marker};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AnswerSets(BTreeMap<String, String>);

impl AnswerSets {
    #[must_use]
    pub fn get(&self, language: &str) -> Option<&str> {
        self.0.get(language).map(String::as_str)
    }

    pub fn set(&mut self, language: &str, spec: &str) {
        if spec.trim().is_empty() {
            self.0.remove(language);
        } else {
            self.0.insert(language.to_string(), spec.to_string());
        }
    }

    pub fn remove(&mut self, language: &str) -> Option<String> {
        self.0.remove(language)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    #[must_use]
    pub fn from_legacy(spec: Option<&str>) -> Self {
        let mut sets = Self::default();
        if let Some(spec) = spec {
            sets.set(UNTAGGED_LANGUAGE, spec);
        }
        sets
    }

    #[must_use]
    pub fn split(mut self, language: &str) -> (Option<String>, Self) {
        let active = self.remove(language);
        (active, self)
    }

    #[must_use]
    pub fn merged(&self, language: &str, active: Option<&str>) -> Self {
        let mut sets = self.clone();
        match active {
            Some(spec) => sets.set(language, spec),
            None => {
                sets.remove(language);
            }
        }
        sets
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnswerKey {
    pub slot: GameSlot,
    pub tp2: String,
    pub component_id: i64,
}

impl AnswerKey {
    #[must_use]
    pub fn new(slot: GameSlot, tp2: &str, component_id: i64) -> Self {
        Self {
            slot,
            tp2: normalize_mod_download_tp2(tp2),
            component_id,
        }
    }

    #[must_use]
    pub(crate) fn for_line(slot: GameSlot, raw_line: &str) -> Option<Self> {
        parse_identity(raw_line).map(|identity| Self {
            slot,
            tp2: identity.normalized_tp2,
            component_id: identity.component_id,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnswerBook(BTreeMap<AnswerKey, AnswerSets>);

impl AnswerBook {
    #[must_use]
    pub fn others(&self, key: &AnswerKey) -> Option<&AnswerSets> {
        self.0.get(key)
    }

    pub fn set_others(&mut self, key: AnswerKey, sets: AnswerSets) {
        if sets.is_empty() {
            self.0.remove(&key);
        } else {
            self.0.insert(key, sets);
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[must_use]
pub fn install_language<'a>(step2: &'a Step2State, slot: GameSlot, tp2: &str) -> &'a str {
    let tab = if slot == GameSlot::Second {
        TAB_BG2EE
    } else {
        TAB_BGEE
    };
    step2
        .mod_install_language(tab, tp2)
        .unwrap_or(UNTAGGED_LANGUAGE)
}

fn language_in<'a>(languages: &'a BTreeMap<String, String>, tp2: &str) -> &'a str {
    languages
        .get(&normalize_mod_download_tp2(tp2))
        .map_or(UNTAGGED_LANGUAGE, String::as_str)
}

fn line_tp2(raw_line: &str, fallback_tp2: &str) -> String {
    parse_identity(raw_line).map_or_else(
        || normalize_mod_download_tp2(fallback_tp2),
        |identity| identity.normalized_tp2,
    )
}

fn with_marker(raw_line: &str, spec: &str) -> String {
    replace_marker(raw_line, spec).unwrap_or_else(|_| {
        if spec.contains(['\r', '\n', '\0']) {
            strip_marker(raw_line)
        } else {
            format!("{} // @wlb-inputs: {spec}", strip_marker(raw_line))
        }
    })
}

fn line_marker(raw_line: &str) -> Option<&str> {
    marker_spec(raw_line).filter(|spec| !spec.is_empty())
}

pub(crate) fn intake(
    component: &mut Step2ComponentState,
    slot: GameSlot,
    sets: &AnswerSets,
    language: &str,
    book: &mut AnswerBook,
) {
    let Some(key) = AnswerKey::for_line(slot, &component.raw_line) else {
        return;
    };
    let (active, rest) = sets.clone().split(language);
    component.raw_line = active.map_or_else(
        || strip_marker(&component.raw_line),
        |spec| with_marker(&component.raw_line, &spec),
    );
    book.set_others(key, rest);
}

#[must_use]
pub(crate) fn outgoing(
    raw_line: &str,
    slot: GameSlot,
    language: &str,
    book: &AnswerBook,
) -> AnswerSets {
    AnswerKey::for_line(slot, raw_line)
        .and_then(|key| book.others(&key).cloned())
        .unwrap_or_default()
        .merged(language, marker_spec(raw_line))
}

#[must_use]
pub(crate) fn outgoing_for_line(
    raw_line: &str,
    fallback_tp2: &str,
    slot: GameSlot,
    step2: &Step2State,
    book: &AnswerBook,
) -> AnswerSets {
    let language = install_language(step2, slot, &line_tp2(raw_line, fallback_tp2));
    outgoing(raw_line, slot, language, book)
}

pub(crate) struct AnswerSide<'a> {
    pub(crate) slot: GameSlot,
    pub(crate) languages: &'a BTreeMap<String, String>,
    pub(crate) book: &'a mut AnswerBook,
}

impl AnswerSide<'_> {
    fn language_of(&self, raw_line: &str) -> &str {
        language_in(self.languages, &line_tp2(raw_line, ""))
    }

    pub(crate) fn intake_component(
        &mut self,
        component: &mut Step2ComponentState,
        sets: &AnswerSets,
    ) {
        let languages = self.languages;
        let language = language_in(languages, &line_tp2(&component.raw_line, ""));
        intake(component, self.slot, sets, language, self.book);
    }

    pub(crate) fn intake_log_answers(
        &mut self,
        component: &mut Step2ComponentState,
        log_spec: &str,
        previous_marker: Option<&str>,
    ) {
        let Some(key) = AnswerKey::for_line(self.slot, &component.raw_line) else {
            return;
        };
        let language = self.language_of(&component.raw_line).to_string();
        let mut sets = self
            .book
            .others(&key)
            .cloned()
            .unwrap_or_default()
            .merged(UNTAGGED_LANGUAGE, Some(log_spec));
        if language != UNTAGGED_LANGUAGE
            && let Some(previous) = previous_marker
        {
            sets = sets.merged(&language, Some(previous));
        }
        self.intake_component(component, &sets);
    }

    pub(crate) fn settle(&mut self, mods: &mut [Step2ModState]) {
        for component in mods
            .iter_mut()
            .flat_map(|mod_state| mod_state.components.iter_mut())
        {
            let Some(key) = AnswerKey::for_line(self.slot, &component.raw_line) else {
                continue;
            };
            let language = self.language_of(&component.raw_line);
            let Some(mut others) = self.book.others(&key).cloned() else {
                continue;
            };
            let Some(spec) = others.remove(language) else {
                continue;
            };
            if line_marker(&component.raw_line).is_none() {
                component.raw_line = with_marker(&component.raw_line, &spec);
            }
            self.book.set_others(key, others);
        }
    }
}

pub(crate) const fn tree_with_answers(
    state: &mut WizardState,
    slot: GameSlot,
) -> (&mut [Step2ModState], AnswerSide<'_>) {
    let WizardState { step2, step3, .. } = state;
    let (mods, languages) = match slot {
        GameSlot::First => (step2.bgee_mods.as_mut_slice(), &step2.bgee_mod_languages),
        GameSlot::Second => (step2.bg2ee_mods.as_mut_slice(), &step2.bg2ee_mod_languages),
    };
    (
        mods,
        AnswerSide {
            slot,
            languages,
            book: &mut step3.answer_book,
        },
    )
}

#[cfg(test)]
#[path = "answer_sets_tests.rs"]
mod tests;
