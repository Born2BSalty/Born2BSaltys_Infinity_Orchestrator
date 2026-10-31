// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::game_authority::GameSlot;
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::state::WizardState;
use crate::app::wlb_inputs::{WlbIdentity, encode_answers, parse_identity, replace_marker};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureRunKey {
    pub(crate) run_id: usize,
    pub(crate) workspace_id: Option<String>,
    pub(crate) generation: u64,
    pub(crate) game_install: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureComponentKey {
    pub(crate) slot: GameSlot,
    pub(crate) identity: WlbIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptOccurrence {
    pub(crate) monotonic_id: u64,
    pub(crate) block_key: String,
    pub(crate) component_attempt: u64,
    pub(crate) output_position: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnswerOrigin {
    Manual,
    Scripted,
    JsonFallback,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComponentOutcome {
    Success,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfirmedAnswers {
    pub(crate) run: CaptureRunKey,
    pub(crate) component: CaptureComponentKey,
    pub(crate) component_attempt: u64,
    pub(crate) answers: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CaptureError {
    StaleRun,
    InvalidAnswer(String),
    MissingComponent,
    AmbiguousComponent,
    InvalidMarker(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CaptureEvent {
    Phase(GameSlot),
    BeginComponent(Vec<CaptureComponentKey>),
    Prompt {
        block_key: String,
        output_position: u64,
        explicit_retry: bool,
    },
    FinishComponent {
        component: CaptureComponentKey,
        outcome: ComponentOutcome,
    },
    Invalidate,
}
#[derive(Debug, Clone)]
pub(crate) struct ReplayEntry {
    pub(crate) component: CaptureComponentKey,
    pub(crate) answers: Vec<String>,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct ReplayPlan {
    pub(crate) entries: Vec<ReplayEntry>,
    pub(crate) legacy_fallback: Vec<(String, Vec<String>)>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum CaptureStatus {
    Active,
    Ambiguous,
    Invalid,
}
struct AnswerSlot {
    block: String,
    value: Option<String>,
    manual: bool,
}
pub(crate) struct CaptureSession {
    run: CaptureRunKey,
    components: Vec<CaptureComponentKey>,
    phase: Option<GameSlot>,
    owner: Option<CaptureComponentKey>,
    attempt: u64,
    next_id: u64,
    position: u64,
    occurrence: Option<PromptOccurrence>,
    answers: Vec<AnswerSlot>,
    current_slot: Option<usize>,
    status: CaptureStatus,
    confirmed: Vec<ConfirmedAnswers>,
    warnings: Vec<String>,
    gathering: bool,
    block: String,
    retry: bool,
    prompt_retry: bool,
    batch: Vec<(CaptureComponentKey, String)>,
}
impl CaptureSession {
    pub(crate) fn new(run: CaptureRunKey, components: Vec<CaptureComponentKey>) -> Self {
        let phase = match run.game_install.as_str() {
            "EET" => None,
            "BG2EE" => Some(GameSlot::Second),
            _ => Some(GameSlot::First),
        };
        Self {
            run,
            components,
            phase,
            owner: None,
            attempt: 0,
            next_id: 0,
            position: 0,
            occurrence: None,
            answers: Vec::new(),
            current_slot: None,
            status: CaptureStatus::Active,
            confirmed: Vec::new(),
            warnings: Vec::new(),
            gathering: false,
            block: String::new(),
            retry: false,
            prompt_retry: false,
            batch: Vec::new(),
        }
    }
    pub(crate) const fn phase(&self) -> Option<GameSlot> {
        self.phase
    }
    pub(crate) fn legacy_fallback_allowed(&self, key: &str) -> bool {
        let Some((tp2, component)) = key.rsplit_once('#') else {
            return false;
        };
        let normalized = normalize_mod_download_tp2(tp2);
        let component = component.parse::<i64>().ok();
        self.replay_component(key).is_some()
            && self
                .components
                .iter()
                .filter(|key| {
                    key.identity.normalized_tp2 == normalized
                        && Some(key.identity.component_id) == component
                })
                .count()
                == 1
    }
    pub(crate) fn replay_component(&self, key: &str) -> Option<&CaptureComponentKey> {
        let (tp2, component) = key.rsplit_once('#')?;
        let normalized = normalize_mod_download_tp2(tp2);
        let component = component.parse::<i64>().ok()?;
        let mut candidates = self.components.iter().filter(|key| {
            self.phase == Some(key.slot)
                && key.identity.normalized_tp2 == normalized
                && key.identity.component_id == component
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then_some(candidate)
    }
    pub(crate) const fn active_component(&self) -> Option<&CaptureComponentKey> {
        self.owner.as_ref()
    }
    pub(crate) fn observe_event(&mut self, event: CaptureEvent) {
        if matches!(event, CaptureEvent::Invalidate) {
            self.invalidate();
            return;
        }
        if self.status == CaptureStatus::Invalid {
            return;
        }
        match event {
            CaptureEvent::Phase(slot) => {
                self.phase = Some(slot);
                self.clear_attempt();
                self.batch.clear();
            }
            CaptureEvent::BeginComponent(keys) => {
                self.clear_attempt();
                self.attempt = self.attempt.saturating_add(1);
                if let [key] = keys.as_slice()
                    && self.phase == Some(key.slot)
                    && self.components.contains(key)
                {
                    self.owner = Some(key.clone());
                }
            }
            CaptureEvent::Prompt {
                block_key,
                output_position,
                explicit_retry,
            } => {
                if self.owner.is_none() || self.status == CaptureStatus::Ambiguous {
                    return;
                }
                let repeated = explicit_retry
                    .then_some(self.current_slot)
                    .flatten()
                    .or_else(|| {
                        self.answers
                            .iter()
                            .position(|answer| answer.block == block_key)
                    });
                let slot = if let Some(index) = repeated {
                    if self.answers[index].value.is_some() && !explicit_retry {
                        self.status = CaptureStatus::Ambiguous;
                        self.occurrence = None;
                        self.warnings.push(
                            "Answers were not saved: repeated question is ambiguous.".to_string(),
                        );
                        return;
                    }
                    if explicit_retry {
                        self.answers[index].value = None;
                        self.answers[index].manual = false;
                    }
                    index
                } else {
                    self.answers.push(AnswerSlot {
                        block: block_key.clone(),
                        value: None,
                        manual: false,
                    });
                    self.answers.len() - 1
                };
                self.next_id = self.next_id.saturating_add(1);
                self.current_slot = Some(slot);
                self.occurrence = Some(PromptOccurrence {
                    monotonic_id: self.next_id,
                    block_key,
                    component_attempt: self.attempt,
                    output_position,
                });
            }
            CaptureEvent::FinishComponent { component, outcome } => {
                if self.owner.as_ref() == Some(&component) {
                    if outcome == ComponentOutcome::Success
                        && self.status != CaptureStatus::Ambiguous
                        && self.answers.iter().any(|answer| answer.manual)
                        && !self.answers.is_empty()
                        && self.answers.iter().all(|answer| answer.value.is_some())
                    {
                        self.confirmed.push(ConfirmedAnswers {
                            run: self.run.clone(),
                            component,
                            component_attempt: self.attempt,
                            answers: self
                                .answers
                                .iter()
                                .filter_map(|answer| answer.value.clone())
                                .collect(),
                        });
                    }
                    self.clear_attempt();
                }
            }
            CaptureEvent::Invalidate => {}
        }
    }
    fn clear_attempt(&mut self) {
        self.owner = None;
        self.occurrence = None;
        self.current_slot = None;
        self.answers.clear();
        self.status = CaptureStatus::Active;
        self.gathering = false;
        self.block.clear();
        self.retry = false;
        self.prompt_retry = false;
    }
    pub(crate) fn current_occurrence(&self) -> Option<PromptOccurrence> {
        let mut occurrence = self.occurrence.clone()?;
        if self.gathering {
            if self.block.is_empty() {
                return None;
            }
            occurrence.block_key.clone_from(&self.block);
        }
        Some(occurrence)
    }
    pub(crate) fn record_sent(
        &mut self,
        occurrence: &PromptOccurrence,
        value: &str,
        origin: AnswerOrigin,
    ) -> Result<bool, CaptureError> {
        if value.contains(['\r', '\n', '\0']) {
            return Err(CaptureError::InvalidAnswer(
                "Answer must be one line".to_string(),
            ));
        }
        if self.status == CaptureStatus::Invalid
            || self.status == CaptureStatus::Ambiguous
            || self.current_occurrence().as_ref() != Some(occurrence)
        {
            return Ok(false);
        }
        let Some(index) = self.current_slot else {
            return Ok(false);
        };
        if self.gathering
            && !self.prompt_retry
            && self.answers.iter().enumerate().any(|(i, answer)| {
                let same_block = answer.block == occurrence.block_key;
                i != index && same_block && answer.value.is_some()
            })
        {
            self.status = CaptureStatus::Ambiguous;
            self.warnings
                .push("Answers were not saved: repeated question is ambiguous.".to_string());
            return Ok(false);
        }
        self.answers[index].block.clone_from(&occurrence.block_key);
        self.answers[index].value = Some(value.to_string());
        self.answers[index].manual |= origin == AnswerOrigin::Manual;
        self.occurrence = Some(occurrence.clone());
        self.gathering = false;
        Ok(true)
    }
    pub(crate) fn take_confirmed_answers(&mut self) -> Vec<ConfirmedAnswers> {
        std::mem::take(&mut self.confirmed)
    }
    pub(crate) fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
    pub(crate) fn invalidate(&mut self) {
        self.clear_attempt();
        self.status = CaptureStatus::Invalid;
        self.batch.clear();
    }
    pub(crate) fn observe_line(&mut self, line: &str) {
        if self.status == CaptureStatus::Invalid {
            return;
        }
        self.position = self.position.saturating_add(1);
        let lower = line.to_ascii_lowercase();
        if lower.contains("beginning pre eet install process") {
            self.observe_event(CaptureEvent::Phase(GameSlot::First));
            return;
        }
        if lower.contains("beginning eet install process") {
            self.observe_event(CaptureEvent::Phase(GameSlot::Second));
            return;
        }
        if self.observe_install_line(line, &lower) || self.observe_completion_line(line, &lower) {
            return;
        }
        if lower.contains("invalid answer")
            || lower.contains("try again")
            || lower.contains("please retry")
        {
            self.retry = true;
            if let Some(index) = self.current_slot {
                self.answers[index].value = None;
                self.answers[index].manual = false;
            }
            self.occurrence = None;
            self.gathering = false;
            self.block.clear();
        }
        let user_header = lower.contains("user input required");
        let question_header = lower.contains("question is");
        if user_header || question_header {
            if question_header && self.gathering {
                return;
            }
            let retry = std::mem::take(&mut self.retry);
            self.prompt_retry = retry;
            self.observe_event(CaptureEvent::Prompt {
                block_key: String::new(),
                output_position: self.position,
                explicit_retry: retry,
            });
            self.gathering = self.occurrence.is_some();
            self.block.clear();
            return;
        }
        if self.gathering {
            let text = line.split_once(']').map_or(line, |(_, rest)| rest).trim();
            if !text.is_empty() {
                if !self.block.is_empty() {
                    self.block.push('\n');
                }
                self.block.push_str(text);
            }
        }
    }
    fn observe_install_line(&mut self, line: &str, lower: &str) -> bool {
        if lower.contains("mod_installer::installers")
            && (lower.contains("installing mod component")
                || lower.contains("installing mod weidubatchedcomponents"))
        {
            let entries = line
                .split("tp_file: ")
                .skip(1)
                .filter_map(|entry| {
                    let tp2 = entry.strip_prefix('"')?.split('"').next()?;
                    let id = field(entry, "component")?.parse::<i64>().ok()?;
                    let language = field(entry, "lang").and_then(|value| value.parse::<u8>().ok());
                    let candidates: Vec<_> = self
                        .components
                        .iter()
                        .filter(|key| {
                            self.phase == Some(key.slot)
                                && key.identity.normalized_tp2 == normalize_mod_download_tp2(tp2)
                                && key.identity.component_id == id
                                && language.is_none_or(|language| key.identity.language == language)
                        })
                        .cloned()
                        .collect();
                    let [key] = candidates.as_slice() else {
                        return None;
                    };
                    Some((
                        key.clone(),
                        field(entry, "component_name").unwrap_or_default(),
                    ))
                })
                .collect::<Vec<_>>();
            let expected = line.matches("tp_file: ").count();
            let keys = if expected == entries.len() {
                entries.iter().map(|(key, _)| key.clone()).collect()
            } else {
                Vec::new()
            };
            self.observe_event(CaptureEvent::BeginComponent(keys));
            self.batch = entries;
            return true;
        }
        false
    }
    fn observe_completion_line(&mut self, line: &str, lower: &str) -> bool {
        if lower.contains("installed mod component")
            || lower.contains("installed mod weiducomponent")
            || lower.contains("installed mod weidubatchedcomponents")
        {
            if lower.contains("mod_installer::installers")
                && !lower.contains("with warnings")
                && line.matches("tp_file: ").count() == 1
                && self.batch.len() == 1
                && let Some((key, _)) = self.batch.first().cloned()
            {
                let tp2 = field(line, "tp_file");
                let component = field(line, "component").and_then(|id| id.parse::<i64>().ok());
                let language = field(line, "lang").and_then(|lang| lang.parse::<u8>().ok());
                if tp2.is_some_and(|tp2| {
                    normalize_mod_download_tp2(&tp2) == key.identity.normalized_tp2
                }) && component == Some(key.identity.component_id)
                    && language == Some(key.identity.language)
                    && self.phase == Some(key.slot)
                {
                    self.observe_event(CaptureEvent::FinishComponent {
                        component: key,
                        outcome: ComponentOutcome::Success,
                    });
                }
            }
            return true;
        }
        if let Some(start) = lower.find("successfully installed") {
            let success_start = start + "successfully installed".len();
            let text = line[success_start..].trim();
            let matches: Vec<_> = self
                .batch
                .iter()
                .filter(|(_, name)| {
                    !name.is_empty()
                        && (text == name
                            || text
                                .strip_prefix(name.as_str())
                                .is_some_and(|tail| tail.starts_with(" -> ")))
                })
                .map(|(key, _)| key.clone())
                .collect();
            if let [key] = matches.as_slice() {
                self.observe_event(CaptureEvent::FinishComponent {
                    component: key.clone(),
                    outcome: ComponentOutcome::Success,
                });
            }
            return true;
        }
        if lower.contains("error installing")
            || lower.contains("rolling back")
            || lower.contains("installation failed")
        {
            if let Some(key) = self.owner.clone() {
                self.observe_event(CaptureEvent::FinishComponent {
                    component: key,
                    outcome: ComponentOutcome::Failed,
                });
            }
            return true;
        }
        false
    }
}
fn field(line: &str, name: &str) -> Option<String> {
    let needle = format!("{name}: \"");
    let (_, tail) = line.split_once(&needle)?;
    let mut escaped = false;
    for (index, character) in tail.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return serde_json::from_str(&format!("\"{}\"", &tail[..index])).ok();
        }
    }
    None
}
pub(crate) fn frozen_components(state: &WizardState) -> Vec<CaptureComponentKey> {
    [
        (GameSlot::First, &state.step2.bgee_mods),
        (GameSlot::Second, &state.step2.bg2ee_mods),
    ]
    .into_iter()
    .flat_map(|(slot, mods)| {
        mods.iter().flat_map(move |item| {
            item.components
                .iter()
                .filter(|component| component.checked)
                .filter_map(move |component| {
                    parse_identity(&component.raw_line)
                        .map(|identity| CaptureComponentKey { slot, identity })
                })
        })
    })
    .collect()
}
pub(crate) fn apply_confirmed(
    state: &mut WizardState,
    confirmed: ConfirmedAnswers,
) -> Result<bool, CaptureError> {
    if state.step5.capture_run.as_ref() != Some(&confirmed.run) {
        return Err(CaptureError::StaleRun);
    }
    let (mods, items) = match confirmed.component.slot {
        GameSlot::First => (&mut state.step2.bgee_mods, &mut state.step3.bgee_items),
        GameSlot::Second => (&mut state.step2.bg2ee_mods, &mut state.step3.bg2ee_items),
    };
    let identity = Some(confirmed.component.identity);
    let identity = &identity;
    let matches2: Vec<_> = mods
        .iter()
        .enumerate()
        .flat_map(move |(m, item)| {
            item.components
                .iter()
                .enumerate()
                .filter(move |(_, component)| parse_identity(&component.raw_line) == *identity)
                .map(move |(c, _)| (m, c))
        })
        .collect();
    let matches3: Vec<_> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.is_parent && parse_identity(&item.raw_line) == *identity)
        .map(|(index, _)| index)
        .collect();
    let ([(m, c)], [index]) = (matches2.as_slice(), matches3.as_slice()) else {
        return Err(if matches2.is_empty() || matches3.is_empty() {
            CaptureError::MissingComponent
        } else {
            CaptureError::AmbiguousComponent
        });
    };
    let Some(spec) = encode_answers(&confirmed.answers).map_err(CaptureError::InvalidMarker)?
    else {
        return Ok(false);
    };
    let raw2 = replace_marker(&mods[*m].components[*c].raw_line, &spec)
        .map_err(CaptureError::InvalidMarker)?;
    let raw3 =
        replace_marker(&items[*index].raw_line, &spec).map_err(CaptureError::InvalidMarker)?;
    if raw2 == mods[*m].components[*c].raw_line && raw3 == items[*index].raw_line {
        return Ok(false);
    }
    mods[*m].components[*c].raw_line = raw2;
    items[*index].raw_line = raw3;
    state.step5.capture_revision = state.step5.capture_revision.saturating_add(1);
    Ok(true)
}
