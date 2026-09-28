// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, BTreeSet};

use crate::app::mod_downloads::{self, ModDownloadSource, SourceTier, SourceTiers};
use crate::app::mod_source_history::{self, BookmarkEntry, HistoryEntry, ModSourceHistoryStore};
use crate::app::state::{Step2ModState, Step2UpdateAsset, WizardState};
use crate::parser::weidu_version::parse_version;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardStatus {
    Fetch,
    Attention,
    InSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardDot {
    Update,
    Warn,
    Bad,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceRowKind {
    Current,
    Layer,
    Pin,
    Bookmark,
    Past,
    Fork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CardSourceOption {
    pub(crate) source_id: String,
    pub(crate) layer: &'static str,
    pub(crate) rule_words: String,
    pub(crate) location: String,
    pub(crate) current: bool,
    pub(crate) kind: SourceRowKind,
    pub(crate) who: String,
    pub(crate) who_hover: Vec<String>,
    pub(crate) version: Option<String>,
    pub(crate) note: Option<(String, String)>,
    pub(crate) block: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedKnownSource {
    source: ModDownloadSource,
    signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct KnownExtras {
    pub(crate) store: ModSourceHistoryStore,
    pin_index: BTreeMap<String, Vec<(String, ParsedKnownSource)>>,
    bookmark_index: BTreeMap<String, Vec<(BookmarkEntry, ParsedKnownSource)>>,
    history_index: BTreeMap<String, Vec<(HistoryEntry, ParsedKnownSource)>>,
}

fn index_known_extras(
    lists: Vec<(String, Vec<ModDownloadSource>)>,
    store: ModSourceHistoryStore,
) -> KnownExtras {
    let mut pin_index: BTreeMap<String, Vec<(String, ParsedKnownSource)>> = BTreeMap::new();
    for (list_name, sources) in lists {
        for source in sources {
            let mut keys = BTreeSet::new();
            let primary = mod_downloads::normalize_mod_download_tp2(&source.tp2);
            if !primary.is_empty() {
                keys.insert(primary);
            }
            for alias in &source.aliases {
                let key = mod_downloads::normalize_mod_download_tp2(alias);
                if !key.is_empty() {
                    keys.insert(key);
                }
            }
            let signature = mod_source_history::rule_signature(&source);
            for key in keys {
                pin_index.entry(key).or_default().push((
                    list_name.clone(),
                    ParsedKnownSource {
                        source: source.clone(),
                        signature: signature.clone(),
                    },
                ));
            }
        }
    }

    let mut bookmark_index: BTreeMap<String, Vec<(BookmarkEntry, ParsedKnownSource)>> =
        BTreeMap::new();
    for bookmark in &store.bookmarks {
        let Some(source) = mod_source_history::source_from_block(&bookmark.tp2, &bookmark.block)
        else {
            continue;
        };
        let signature = mod_source_history::rule_signature(&source);
        bookmark_index
            .entry(bookmark.tp2.clone())
            .or_default()
            .push((bookmark.clone(), ParsedKnownSource { source, signature }));
    }

    let mut history_index: BTreeMap<String, Vec<(HistoryEntry, ParsedKnownSource)>> =
        BTreeMap::new();
    for entry in &store.history {
        let Some(source) = mod_source_history::source_from_block(&entry.tp2, &entry.block) else {
            continue;
        };
        let signature = mod_source_history::rule_signature(&source);
        history_index
            .entry(entry.tp2.clone())
            .or_default()
            .push((entry.clone(), ParsedKnownSource { source, signature }));
    }

    KnownExtras {
        store,
        pin_index,
        bookmark_index,
        history_index,
    }
}

pub(crate) fn load_known_extras(
    active_modlist_id: Option<&str>,
    lists: &[(String, String)],
) -> KnownExtras {
    let mut other_lists = Vec::new();
    for (id, name) in lists {
        if active_modlist_id == Some(id.as_str()) {
            continue;
        }
        let path =
            crate::registry::store_workspace::modlist_data_dir(id).join("mod_downloads_user.toml");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let loaded = mod_downloads::load_mod_download_sources_from_texts("", &text, "");
        if loaded.sources.is_empty() {
            continue;
        }
        other_lists.push((name.clone(), loaded.sources));
    }
    index_known_extras(other_lists, mod_source_history::load_store())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FetchPhase {
    Downloading(Option<f32>),
    Extracting,
    Rescanning,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VersionCard {
    pub(crate) tp2: String,
    pub(crate) name: String,
    pub(crate) status: CardStatus,
    pub(crate) dot: CardDot,
    pub(crate) status_line: String,
    pub(crate) target: Option<String>,
    pub(crate) locked: bool,
    pub(crate) can_fetch: bool,
    pub(crate) layer: &'static str,
    pub(crate) rule_words: String,
    pub(crate) selector_hover: String,
    pub(crate) open_url: Option<String>,
    pub(crate) repo: Option<String>,
    pub(crate) source_id: Option<String>,
    pub(crate) sources: Vec<CardSourceOption>,
    pub(crate) fetching: Option<FetchPhase>,
    pub(crate) queued: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VersionsView {
    pub(crate) cards: Vec<VersionCard>,
    pub(crate) fetch_count: usize,
    pub(crate) attention_count: usize,
    pub(crate) locked_count: usize,
    pub(crate) log_missing_count: usize,
}

struct CardBasis {
    tp2_key: String,
    name: String,
    on_disk_version: Option<String>,
    locked: bool,
    log_pending_only: bool,
}

pub(crate) fn build_versions_view(
    state: &WizardState,
    tiers: &SourceTiers,
    extras: Option<&KnownExtras>,
) -> VersionsView {
    let cards = collect_card_basis(state)
        .into_iter()
        .map(|basis| build_card(state, tiers, extras, basis))
        .collect::<Vec<_>>();
    let fetch_count = cards
        .iter()
        .filter(|card| card.status == CardStatus::Fetch)
        .count();
    let attention_count = cards
        .iter()
        .filter(|card| card.status == CardStatus::Attention)
        .count();
    let locked_count = cards.iter().filter(|card| card.locked).count();
    let log_missing_count = state.step2.log_pending_downloads.len();
    let mut view = VersionsView {
        cards,
        fetch_count,
        attention_count,
        locked_count,
        log_missing_count,
    };
    refresh_fetch_phase(&mut view, state);
    view
}

pub(crate) fn refresh_fetch_phase(view: &mut VersionsView, state: &WizardState) {
    let batch_keys = running_batch_keys(state);
    let fetching_key = fetching_key(state);
    let downloading = state.step2.update_selected_download_running;
    for card in &mut view.cards {
        let is_current = fetching_key.as_deref() == Some(card.tp2.as_str());
        let finished = downloading
            && !is_current
            && state
                .step2
                .update_selected_download_finished
                .contains(&card.tp2);
        card.fetching = if finished {
            Some(FetchPhase::Downloading(Some(1.0)))
        } else {
            fetch_phase_for_with(state, &card.tp2, &batch_keys)
        };
        let waiting_in_batch =
            downloading && batch_keys.contains(&card.tp2) && !is_current && !finished;
        card.queued = waiting_in_batch || state.step2.versions_ui.fetch_queue.contains(&card.tp2);
        if state.step2.is_scanning && extracted_just_now(state, &card.name) {
            card.status = CardStatus::InSync;
            card.dot = CardDot::Neutral;
            card.status_line = "Rescanning\u{2026}".to_string();
            card.can_fetch = false;
            card.target = None;
        }
    }
    view.fetch_count = view
        .cards
        .iter()
        .filter(|card| card.status == CardStatus::Fetch)
        .count();
    view.attention_count = view
        .cards
        .iter()
        .filter(|card| card.status == CardStatus::Attention)
        .count();
}

fn extracted_just_now(state: &WizardState, name: &str) -> bool {
    let prefix = format!("{name} -> ");
    state
        .step2
        .update_selected_extracted_sources
        .iter()
        .any(|entry| entry.starts_with(&prefix))
}

fn running_batch_keys(state: &WizardState) -> BTreeSet<String> {
    let step2 = &state.step2;
    let unscoped =
        step2.update_selected_download_scope.is_none() && step2.versions_ui.fetching_tp2.is_none();
    let running = step2.update_selected_download_running || step2.update_selected_extract_running;
    if !unscoped || !running {
        return BTreeSet::new();
    }
    step2
        .update_selected_update_assets
        .iter()
        .map(|asset| mod_downloads::normalize_mod_download_tp2(&asset.tp_file))
        .filter(|key| !key.is_empty())
        .collect()
}

fn fetching_key(state: &WizardState) -> Option<String> {
    state
        .step2
        .update_selected_download_scope
        .as_deref()
        .or(state.step2.versions_ui.fetching_tp2.as_deref())
        .or(state.step2.update_selected_download_current.as_deref())
        .map(mod_downloads::normalize_mod_download_tp2)
        .filter(|key| !key.is_empty())
}

#[cfg(test)]
pub(crate) fn fetch_phase_for(state: &WizardState, tp2_key: &str) -> Option<FetchPhase> {
    fetch_phase_for_with(state, tp2_key, &running_batch_keys(state))
}

pub(crate) fn fetch_phase_for_with(
    state: &WizardState,
    tp2_key: &str,
    batch_keys: &BTreeSet<String>,
) -> Option<FetchPhase> {
    if fetching_key(state).as_deref() != Some(tp2_key) {
        let extracting_in_batch =
            state.step2.update_selected_extract_running && batch_keys.contains(tp2_key);
        return extracting_in_batch.then_some(FetchPhase::Extracting);
    }
    if state.step2.update_selected_download_running {
        Some(FetchPhase::Downloading(fetch_fraction(state)))
    } else if state.step2.update_selected_extract_running {
        Some(FetchPhase::Extracting)
    } else if state.step2.is_scanning {
        Some(FetchPhase::Rescanning)
    } else {
        None
    }
}

pub(crate) fn fetch_fraction(state: &WizardState) -> Option<f32> {
    let (done, total) = state.step2.update_selected_download_bytes?;
    let total = total.filter(|total| *total > 0)?;
    let basis_points = u128::from(done.min(total)) * 10_000 / u128::from(total);
    let basis_points = u16::try_from(basis_points).unwrap_or(10_000);
    Some(f32::from(basis_points) / 10_000.0)
}

fn collect_card_basis(state: &WizardState) -> Vec<CardBasis> {
    let mut seen = BTreeSet::<String>::new();
    let mut result = Vec::<CardBasis>::new();

    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        let tp2_key = mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
        if tp2_key.is_empty() || !seen.insert(tp2_key.clone()) {
            continue;
        }
        result.push(CardBasis {
            tp2_key,
            name: mod_display_name(mod_state),
            on_disk_version: mod_state
                .components
                .iter()
                .find_map(|component| parse_version(&component.raw_line)),
            locked: mod_state.update_locked,
            log_pending_only: false,
        });
    }

    for pending in &state.step2.log_pending_downloads {
        let tp2_key = mod_downloads::normalize_mod_download_tp2(&pending.tp_file);
        if tp2_key.is_empty() || !seen.insert(tp2_key.clone()) {
            continue;
        }
        result.push(CardBasis {
            tp2_key,
            name: pending.label.clone(),
            on_disk_version: None,
            locked: false,
            log_pending_only: true,
        });
    }

    result.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    result
}

fn mod_display_name(mod_state: &Step2ModState) -> String {
    if mod_state.name.trim().is_empty() {
        mod_state.tp_file.clone()
    } else {
        mod_state.name.clone()
    }
}

struct StatusFacts {
    fetch_asset: Option<Step2UpdateAsset>,
    check_failed: Option<String>,
    fetch_failed: Option<String>,
    locked_update: Option<Step2UpdateAsset>,
    just_fetched: bool,
}

fn status_facts(state: &WizardState, basis: &CardBasis) -> StatusFacts {
    let name = basis.name.as_str();
    let fetch_asset = state
        .step2
        .update_selected_update_assets
        .iter()
        .find(|asset| mod_downloads::normalize_mod_download_tp2(&asset.tp_file) == basis.tp2_key)
        .cloned();
    let check_failed = find_labelled_error(&state.step2.update_selected_failed_sources, name)
        .or_else(|| {
            find_labelled_error(
                &state.step2.update_selected_exact_version_failed_sources,
                name,
            )
        });
    let fetch_failed =
        find_labelled_error(&state.step2.update_selected_download_failed_sources, name).or_else(
            || find_labelled_error(&state.step2.update_selected_extract_failed_sources, name),
        );
    let locked_update = state
        .step2
        .update_selected_locked_update_assets
        .iter()
        .find(|asset| mod_downloads::normalize_mod_download_tp2(&asset.tp_file) == basis.tp2_key)
        .cloned();
    let just_fetched = extracted_just_now(state, name);
    StatusFacts {
        fetch_asset,
        check_failed,
        fetch_failed,
        locked_update,
        just_fetched,
    }
}

fn find_labelled_error(entries: &[String], label: &str) -> Option<String> {
    let prefix = format!("{label}: ");
    entries
        .iter()
        .find_map(|entry| entry.strip_prefix(prefix.as_str()).map(str::to_string))
}

fn version_label(on_disk_version: Option<&str>, log_pending_only: bool) -> String {
    match on_disk_version {
        Some(version) => version.to_string(),
        None if log_pending_only => "not on disk".to_string(),
        None => String::new(),
    }
}

fn display_version(target: &str) -> &str {
    let mut chars = target.chars();
    match (chars.next(), chars.next()) {
        (Some('v' | 'V'), Some(next)) if next.is_ascii_digit() => &target[1..],
        _ => target,
    }
}

fn is_hex(value: &str) -> bool {
    value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn short_target(tag: &str) -> String {
    if tag.len() == 40 && is_hex(tag) {
        return tag[..7].to_string();
    }
    if let Some((reference, hex)) = tag.rsplit_once('@')
        && hex.len() >= 12
        && is_hex(hex)
    {
        return format!("{reference}@{}", &hex[..7]);
    }
    tag.to_string()
}

fn arrow_status_line(version: &str, target: &str) -> String {
    let target = display_version(target);
    if version.is_empty() {
        format!("? \u{2192} {target}")
    } else {
        format!("{version} \u{2192} {target}")
    }
}

fn locked_available_status_line(version: &str, target: &str) -> String {
    let target = display_version(target);
    if version.is_empty() {
        format!("locked \u{b7} {target} available")
    } else {
        format!("{version} \u{b7} locked \u{b7} {target} available")
    }
}

fn is_manual_not_on_disk(source: Option<&ModDownloadSource>, log_pending_only: bool) -> bool {
    log_pending_only
        && source.is_some_and(|source| !mod_downloads::source_is_auto_resolvable(source))
}

struct CardOutcome {
    status: CardStatus,
    dot: CardDot,
    status_line: String,
    target: Option<String>,
}

fn card_outcome(
    state: &WizardState,
    basis: &CardBasis,
    facts: &StatusFacts,
    has_source: bool,
    source: Option<&ModDownloadSource>,
    version: &str,
) -> CardOutcome {
    if let Some(asset) = facts.fetch_asset.as_ref() {
        return CardOutcome {
            status: CardStatus::Fetch,
            dot: CardDot::Update,
            status_line: arrow_status_line(version, &short_target(&asset.tag)),
            target: Some(short_target(&asset.tag)),
        };
    }
    if let Some(error) = facts.check_failed.as_ref() {
        return CardOutcome {
            status: CardStatus::Attention,
            dot: CardDot::Warn,
            status_line: format!("check failed \u{b7} {error}"),
            target: None,
        };
    }
    if let Some(error) = facts.fetch_failed.as_ref() {
        return CardOutcome {
            status: CardStatus::Attention,
            dot: CardDot::Warn,
            status_line: format!("fetch failed \u{b7} {error}"),
            target: None,
        };
    }
    if !has_source {
        return CardOutcome {
            status: CardStatus::Attention,
            dot: CardDot::Bad,
            status_line: "no source".to_string(),
            target: None,
        };
    }
    if is_manual_not_on_disk(source, basis.log_pending_only) {
        return CardOutcome {
            status: CardStatus::Attention,
            dot: CardDot::Warn,
            status_line: "manual download \u{b7} not on disk".to_string(),
            target: None,
        };
    }
    if let Some(asset) = facts.locked_update.as_ref() {
        return CardOutcome {
            status: CardStatus::InSync,
            dot: CardDot::Neutral,
            status_line: locked_available_status_line(version, &short_target(&asset.tag)),
            target: Some(short_target(&asset.tag)),
        };
    }
    if facts.just_fetched {
        return CardOutcome {
            status: CardStatus::InSync,
            dot: CardDot::Neutral,
            status_line: format!("{version} \u{b7} fetched just now"),
            target: None,
        };
    }
    if !state.step2.update_selected_has_run {
        return CardOutcome {
            status: CardStatus::InSync,
            dot: CardDot::Neutral,
            status_line: format!("{version} \u{b7} not checked"),
            target: None,
        };
    }
    let status_line = if basis.locked {
        format!("{version} \u{b7} locked")
    } else {
        version.to_string()
    };
    CardOutcome {
        status: CardStatus::InSync,
        dot: CardDot::Neutral,
        status_line,
        target: None,
    }
}

fn current_source(
    tiers: &SourceTiers,
    tp2: &str,
    selected_source_id: Option<&str>,
) -> Option<ModDownloadSource> {
    let sources = tiers.find_sources(tp2);
    if let Some(selected_source_id) = selected_source_id {
        let key = mod_downloads::normalize_source_id(selected_source_id);
        if let Some(source) = sources
            .iter()
            .find(|source| mod_downloads::normalize_source_id(&source.source_id) == key)
        {
            return Some(source.clone());
        }
    }
    sources.into_iter().next()
}

fn location_for(source: &ModDownloadSource) -> String {
    if let Some(repo) = source.github.as_deref().map(str::trim)
        && !repo.is_empty()
    {
        return repo.to_string();
    }
    mod_downloads::source_link_label(&source.url)
}

pub(crate) fn bookmark_label(state: &WizardState, tp2: &str) -> Option<String> {
    let loaded = mod_downloads::load_mod_download_sources();
    let selected_source_id = state.step2.selected_source_ids.get(tp2).cloned();
    let current = loaded.resolve_source(tp2, selected_source_id.as_deref())?;
    current.github.as_ref()?;
    let refs_file = crate::app::app_step2_update_source_refs::load_refs_file_at(
        &crate::app::app_step2_update_source_refs::installed_source_refs_path(),
    );
    let normalized_tp2 = mod_downloads::normalize_mod_download_tp2(tp2);
    let installed_source_id = refs_file.sources.get(&normalized_tp2).map(String::as_str);
    let installed_ref = refs_file.refs.get(&normalized_tp2).map(String::as_str);
    let (_, label) =
        mod_source_history::bookmark_block(&current, installed_source_id, installed_ref)?;
    Some(format!("Bookmark {label}"))
}

fn build_source_options(
    state: &WizardState,
    tiers: &SourceTiers,
    extras: Option<&KnownExtras>,
    tp2_key: &str,
    name: &str,
    current_source_id: Option<&str>,
) -> Vec<CardSourceOption> {
    let layer_sources = tiers.find_sources(tp2_key);
    let Some(extras) = extras else {
        return layer_sources
            .into_iter()
            .map(|option_source| layer_option(tiers, &option_source, current_source_id))
            .collect();
    };

    let mut current_first = Vec::new();
    let mut other_layers = Vec::new();
    for option_source in layer_sources {
        if current_source_id == Some(option_source.source_id.as_str()) {
            current_first.push(option_source);
        } else {
            other_layers.push(option_source);
        }
    }

    let mut candidates = Vec::<(ModDownloadSource, CardSourceOption)>::new();
    for option_source in current_first.into_iter().chain(other_layers) {
        let option = layer_option(tiers, &option_source, current_source_id);
        candidates.push((option_source, option));
    }
    candidates.extend(pin_rows(extras, tp2_key));
    candidates.extend(bookmark_rows(extras, tp2_key));
    candidates.extend(past_rows(extras, tp2_key));
    candidates.extend(fork_rows(
        &state.step2.mod_download_forks,
        &state.step2.mod_download_forks_popup_tp2,
        tp2_key,
        name,
    ));

    let mut seen = BTreeSet::<String>::new();
    let mut merged = Vec::with_capacity(candidates.len());
    for (candidate_source, mut option) in candidates {
        let signature = mod_source_history::rule_signature(&candidate_source);
        if !seen.insert(signature.clone()) {
            continue;
        }
        let note_key = mod_source_history::note_key(tp2_key, &signature);
        option.note = extras
            .store
            .notes
            .get(&note_key)
            .map(|note| (note.text.clone(), note.who.clone()));
        merged.push(option);
    }
    merged
}

fn layer_option(
    tiers: &SourceTiers,
    option_source: &ModDownloadSource,
    current_source_id: Option<&str>,
) -> CardSourceOption {
    let current = current_source_id == Some(option_source.source_id.as_str());
    let layer = layer_name(tiers.tier_of(&option_source.tp2, &option_source.source_id));
    CardSourceOption {
        source_id: option_source.source_id.clone(),
        layer,
        rule_words: rule_words(option_source),
        location: location_for(option_source),
        current,
        kind: if current {
            SourceRowKind::Current
        } else {
            SourceRowKind::Layer
        },
        who: layer.to_string(),
        who_hover: Vec::new(),
        version: None,
        note: None,
        block: None,
    }
}

fn pin_rows(extras: &KnownExtras, tp2_key: &str) -> Vec<(ModDownloadSource, CardSourceOption)> {
    let Some(entries) = extras.pin_index.get(tp2_key) else {
        return Vec::new();
    };
    let mut grouped = Vec::<(String, ModDownloadSource, Vec<String>)>::new();
    for (list_name, parsed) in entries {
        if let Some(existing) = grouped
            .iter_mut()
            .find(|(sig, _, _)| *sig == parsed.signature)
        {
            if !existing.2.contains(list_name) {
                existing.2.push(list_name.clone());
            }
        } else {
            grouped.push((
                parsed.signature.clone(),
                parsed.source.clone(),
                vec![list_name.clone()],
            ));
        }
    }
    grouped
        .into_iter()
        .map(|(_, source, names)| {
            let who = if names.len() == 1 {
                names[0].clone()
            } else {
                format!("{} modlists", names.len())
            };
            let option = CardSourceOption {
                source_id: source.source_id.clone(),
                layer: "Other list",
                rule_words: rule_words(&source),
                location: location_for(&source),
                current: false,
                kind: SourceRowKind::Pin,
                who,
                who_hover: names,
                version: None,
                note: None,
                block: Some(mod_downloads::complete_source_block(&source)),
            };
            (source, option)
        })
        .collect()
}

fn bookmark_rows(
    extras: &KnownExtras,
    tp2_key: &str,
) -> Vec<(ModDownloadSource, CardSourceOption)> {
    let Some(entries) = extras.bookmark_index.get(tp2_key) else {
        return Vec::new();
    };
    entries
        .iter()
        .map(|(bookmark, parsed)| {
            let option = CardSourceOption {
                source_id: parsed.source.source_id.clone(),
                layer: "Bookmark",
                rule_words: rule_words(&parsed.source),
                location: location_for(&parsed.source),
                current: false,
                kind: SourceRowKind::Bookmark,
                who: bookmark.date.clone(),
                who_hover: Vec::new(),
                version: Some(bookmark.version.clone()),
                note: None,
                block: Some(bookmark.block.clone()),
            };
            (parsed.source.clone(), option)
        })
        .collect()
}

fn past_rows(extras: &KnownExtras, tp2_key: &str) -> Vec<(ModDownloadSource, CardSourceOption)> {
    let Some(entries) = extras.history_index.get(tp2_key) else {
        return Vec::new();
    };
    entries
        .iter()
        .map(|(entry, parsed)| {
            let option = CardSourceOption {
                source_id: parsed.source.source_id.clone(),
                layer: "Past save",
                rule_words: rule_words(&parsed.source),
                location: location_for(&parsed.source),
                current: false,
                kind: SourceRowKind::Past,
                who: entry.date.clone(),
                who_hover: Vec::new(),
                version: None,
                note: None,
                block: Some(entry.block.clone()),
            };
            (parsed.source.clone(), option)
        })
        .collect()
}

fn fork_rows(
    forks: &[crate::app::state::Step2DiscoveredFork],
    forks_tp2: &str,
    tp2_key: &str,
    name: &str,
) -> Vec<(ModDownloadSource, CardSourceOption)> {
    if forks.is_empty() || mod_downloads::normalize_mod_download_tp2(forks_tp2) != tp2_key {
        return Vec::new();
    }
    forks
        .iter()
        .map(|fork| {
            let source = ModDownloadSource {
                tp2: tp2_key.to_string(),
                name: name.to_string(),
                source_id: fork.owner_login.clone(),
                source_label: fork.owner_login.clone(),
                github: Some(fork.full_name.clone()),
                url: format!("https://github.com/{}", fork.full_name),
                branch: Some(fork.default_branch.clone()),
                ..ModDownloadSource::default()
            };
            let option = CardSourceOption {
                source_id: source.source_id.clone(),
                layer: "Fork",
                rule_words: rule_words(&source),
                location: location_for(&source),
                current: false,
                kind: SourceRowKind::Fork,
                who: String::new(),
                who_hover: Vec::new(),
                version: None,
                note: None,
                block: Some(mod_downloads::complete_source_block(&source)),
            };
            (source, option)
        })
        .collect()
}

fn build_card(
    state: &WizardState,
    tiers: &SourceTiers,
    extras: Option<&KnownExtras>,
    basis: CardBasis,
) -> VersionCard {
    let facts = status_facts(state, &basis);
    let version = version_label(basis.on_disk_version.as_deref(), basis.log_pending_only);
    let selected_source_id = state
        .step2
        .selected_source_ids
        .get(&basis.tp2_key)
        .map(String::as_str);
    let source = current_source(tiers, &basis.tp2_key, selected_source_id);
    let has_source = source.is_some();

    let outcome = card_outcome(state, &basis, &facts, has_source, source.as_ref(), &version);

    let (layer, rule_words_value, selector_hover_value) = source.as_ref().map_or_else(
        || ("", String::new(), String::new()),
        |source| {
            (
                layer_name(tiers.tier_of(&source.tp2, &source.source_id)),
                selector_words(source),
                selector_hover(source),
            )
        },
    );
    let repo = source.as_ref().and_then(|source| source.github.clone());
    let open_url = source.as_ref().and_then(mod_downloads::source_open_url);
    let source_id = source.as_ref().map(|source| source.source_id.clone());
    let can_fetch = outcome.status == CardStatus::Fetch && !basis.locked;

    let sources = build_source_options(
        state,
        tiers,
        extras,
        &basis.tp2_key,
        &basis.name,
        source_id.as_deref(),
    );

    VersionCard {
        tp2: basis.tp2_key,
        name: basis.name,
        status: outcome.status,
        dot: outcome.dot,
        status_line: outcome.status_line,
        target: outcome.target,
        locked: basis.locked,
        can_fetch,
        layer,
        rule_words: rule_words_value,
        selector_hover: selector_hover_value,
        open_url,
        repo,
        source_id,
        sources,
        fetching: None,
        queued: false,
    }
}

pub(crate) fn rule_words(source: &ModDownloadSource) -> String {
    rule_words_with(source, true)
}

fn rule_words_without_asset(source: &ModDownloadSource) -> String {
    rule_words_with(source, false)
}

fn rule_words_with(source: &ModDownloadSource, include_asset: bool) -> String {
    if let Some(commit) = non_empty(source.commit.as_deref()) {
        return format!("Commit {}", commit.chars().take(7).collect::<String>());
    }
    if let Some(tag) = non_empty(source.tag.as_deref()) {
        return format!("Tag {tag}");
    }
    if let Some(branch) = non_empty(source.branch.as_deref()) {
        return format!("Branch {branch}");
    }
    if let Some(release) = non_empty(source.release.as_deref()) {
        return non_empty(source.asset.as_deref())
            .filter(|_| include_asset)
            .map_or_else(
                || format!("Release {release}"),
                |asset| format!("Release {release} \u{b7} {asset}"),
            );
    }
    if source.github.is_some() {
        return match source.channel.as_deref().map(str::trim) {
            Some("preonly") => "Newest pre-release".to_string(),
            Some("pre-release") => "Newest release + pre-releases".to_string(),
            Some("master") => "Default branch".to_string(),
            Some("ifeellucky") => "Newest release, else latest code".to_string(),
            _ => "Newest release".to_string(),
        };
    }
    if mod_downloads::source_is_weaselmods_page_url(&source.url) {
        return "Weasel Mods page".to_string();
    }
    if mod_downloads::source_is_morpheus_mart_page_url(&source.url) {
        return "Morpheus Mart page".to_string();
    }
    if mod_downloads::is_direct_archive_url(&source.url) {
        return "Direct archive".to_string();
    }
    "Manual download".to_string()
}

pub(crate) fn selector_words(source: &ModDownloadSource) -> String {
    let rule = rule_words_without_asset(source);
    match non_empty(source.github.as_deref()) {
        Some(repo) => format!("{repo} \u{b7} {rule}"),
        None => rule,
    }
}

pub(crate) fn selector_hover(source: &ModDownloadSource) -> String {
    let rule = rule_words(source);
    match non_empty(source.github.as_deref()) {
        Some(repo) => format!("{rule} \u{b7} {repo}"),
        None => rule,
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub(crate) const fn layer_name(tier: SourceTier) -> &'static str {
    match tier {
        SourceTier::Modlist => "This modlist",
        SourceTier::User => "My default",
        SourceTier::Default => "BIO default",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::mod_downloads;
    use crate::app::state::{Step2ComponentState, Step2LogPendingDownload};

    fn component(id: &str, raw_line: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: id.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: raw_line.to_string(),
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
            checked: true,
            selected_order: Some(1),
        }
    }

    fn mod_state(tp_file: &str, name: &str, raw_line: &str) -> Step2ModState {
        Step2ModState {
            name: name.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: format!("{tp_file}.tp2"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: true,
            hidden_components: Vec::new(),
            components: vec![component("0", raw_line)],
        }
    }

    fn asset(tp_file: &str, label: &str, tag: &str) -> Step2UpdateAsset {
        Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: "primary".to_string(),
            tag: tag.to_string(),
            asset_name: "asset.zip".to_string(),
            asset_url: "https://example.test/asset.zip".to_string(),
            installed_source_ref: None,
        }
    }

    fn github_source(tp2: &str) -> String {
        format!(
            "[[mods]]\nname = \"{tp2}\"\ntp2 = \"{tp2}\"\n\n  [[mods.sources]]\n  id = \"primary\"\n  label = \"GitHub\"\n  type = \"github\"\n  url = \"https://github.com/o/{tp2}\"\n  repo = \"o/{tp2}\"\n  default = true\n"
        )
    }

    fn empty_tiers() -> SourceTiers {
        mod_downloads::source_tiers_from_texts("", "", "")
    }

    #[test]
    fn fetch_phase_follows_scope_and_flags() {
        let mut state = WizardState::default();
        let key = mod_downloads::normalize_mod_download_tp2("setup-mod.tp2");
        assert_eq!(fetch_phase_for(&state, &key), None);

        state.step2.update_selected_download_scope = Some("SETUP-MOD.TP2".to_string());
        state.step2.update_selected_download_running = true;
        assert_eq!(
            fetch_phase_for(&state, &key),
            Some(FetchPhase::Downloading(None))
        );
        state.step2.update_selected_download_bytes = Some((25, Some(100)));
        assert_eq!(
            fetch_phase_for(&state, &key),
            Some(FetchPhase::Downloading(Some(0.25)))
        );
        state.step2.update_selected_download_bytes = Some((25, None));
        assert_eq!(
            fetch_phase_for(&state, &key),
            Some(FetchPhase::Downloading(None))
        );
        assert_eq!(fetch_phase_for(&state, "other"), None);

        state.step2.update_selected_download_running = false;
        state.step2.update_selected_extract_running = true;
        assert_eq!(fetch_phase_for(&state, &key), Some(FetchPhase::Extracting));

        state.step2.update_selected_extract_running = false;
        state.step2.is_scanning = true;
        assert_eq!(fetch_phase_for(&state, &key), Some(FetchPhase::Rescanning));

        state.step2.update_selected_download_scope = None;
        assert_eq!(fetch_phase_for(&state, &key), None);
        state.step2.versions_ui.fetching_tp2 = Some(key.clone());
        assert_eq!(fetch_phase_for(&state, &key), Some(FetchPhase::Rescanning));

        state.step2.is_scanning = false;
        assert_eq!(fetch_phase_for(&state, &key), None);
    }

    #[test]
    fn unscoped_fetch_marks_current_card_downloading_and_others_queued() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("a.tp2", "A", "~a.tp2~ #0 #0 // 1.0"));
        state
            .step2
            .bgee_mods
            .push(mod_state("b.tp2", "B", "~b.tp2~ #0 #0 // 1.0"));
        state
            .step2
            .update_selected_update_assets
            .push(asset("a.tp2", "A", "2.0"));
        state
            .step2
            .update_selected_update_assets
            .push(asset("b.tp2", "B", "2.0"));
        state.step2.update_selected_download_running = true;
        state.step2.update_selected_download_current = Some("a.tp2".to_string());

        let view = build_versions_view(&state, &empty_tiers(), None);
        let card_for = |tp2: &str| {
            let key = mod_downloads::normalize_mod_download_tp2(tp2);
            view.cards
                .iter()
                .find(|card| card.tp2 == key)
                .expect("card present")
        };
        let current = card_for("a.tp2");
        assert!(matches!(current.fetching, Some(FetchPhase::Downloading(_))));
        assert!(!current.queued);
        let waiting = card_for("b.tp2");
        assert!(waiting.queued);
        assert_eq!(waiting.fetching, None);
    }

    fn batch_state(names: &[&str]) -> WizardState {
        let mut state = WizardState::default();
        for name in names {
            let tp_file = format!("{name}.tp2");
            state.step2.bgee_mods.push(mod_state(
                &tp_file,
                name,
                &format!("~{tp_file}~ #0 #0 // 1.0"),
            ));
            state
                .step2
                .update_selected_update_assets
                .push(asset(&tp_file, name, "2.0"));
        }
        state
    }

    fn card_named<'a>(view: &'a VersionsView, tp_file: &str) -> &'a VersionCard {
        let key = mod_downloads::normalize_mod_download_tp2(tp_file);
        view.cards
            .iter()
            .find(|card| card.tp2 == key)
            .expect("card present")
    }

    #[test]
    fn finished_batch_card_reads_full_while_next_downloads_and_rest_queue() {
        use crate::app::app_step2_update_download::{
            Step2UpdateDownloadEvent, poll_step2_update_download,
        };
        let mut state = batch_state(&["a", "b", "c"]);
        state.step2.update_selected_download_running = true;
        let (tx, rx) = std::sync::mpsc::channel::<Step2UpdateDownloadEvent>();
        let mut download_rx = Some(rx);
        let mut extract_rx = None;
        tx.send(Step2UpdateDownloadEvent::Progress {
            tp_file: "a.tp2".to_string(),
            ok: true,
            completed: 1,
            total: 3,
        })
        .unwrap();
        poll_step2_update_download(&mut state, &mut download_rx, &mut extract_rx);
        tx.send(Step2UpdateDownloadEvent::Bytes {
            tp_file: "b.tp2".to_string(),
            done: 10,
            total: Some(100),
        })
        .unwrap();
        poll_step2_update_download(&mut state, &mut download_rx, &mut extract_rx);

        let view = build_versions_view(&state, &empty_tiers(), None);
        let finished = card_named(&view, "a.tp2");
        assert_eq!(finished.fetching, Some(FetchPhase::Downloading(Some(1.0))));
        assert!(!finished.queued);
        let current = card_named(&view, "b.tp2");
        assert!(matches!(current.fetching, Some(FetchPhase::Downloading(_))));
        assert!(!current.queued);
        let waiting = card_named(&view, "c.tp2");
        assert!(waiting.queued);
        assert_eq!(waiting.fetching, None);
    }

    #[test]
    fn unscoped_extract_marks_every_batch_card_extracting() {
        let mut state = batch_state(&["a", "b"]);
        state.step2.update_selected_extract_running = true;

        let view = build_versions_view(&state, &empty_tiers(), None);
        for tp_file in ["a.tp2", "b.tp2"] {
            let card = card_named(&view, tp_file);
            assert_eq!(card.fetching, Some(FetchPhase::Extracting));
            assert!(!card.queued);
        }
    }

    #[test]
    fn unscoped_rescan_shows_no_fetch_phase() {
        let mut state = batch_state(&["a", "b"]);
        state.step2.is_scanning = true;
        state
            .step2
            .update_selected_extracted_sources
            .push("a -> C:/mods/a".to_string());

        let view = build_versions_view(&state, &empty_tiers(), None);
        for tp_file in ["a.tp2", "b.tp2"] {
            let card = card_named(&view, tp_file);
            assert_eq!(card.fetching, None);
            assert!(!card.queued);
        }
        let extracted = card_named(&view, "a.tp2");
        assert_eq!(extracted.status, CardStatus::InSync);
        assert_eq!(extracted.status_line, "Rescanning\u{2026}");
        assert!(!extracted.can_fetch);
        assert_eq!(extracted.target, None);
        assert_eq!(card_named(&view, "b.tp2").status, CardStatus::Fetch);
        assert_eq!(view.fetch_count, 1);

        state.step2.is_scanning = false;
        let view = build_versions_view(&state, &empty_tiers(), None);
        assert_ne!(card_named(&view, "a.tp2").status_line, "Rescanning\u{2026}");
    }

    #[test]
    fn scoped_fetch_leaves_other_asset_cards_unqueued() {
        let mut state = batch_state(&["a", "b"]);
        state.step2.update_selected_download_running = true;
        state.step2.update_selected_download_scope = Some("a".to_string());
        state.step2.versions_ui.fetching_tp2 = Some("a".to_string());

        let view = build_versions_view(&state, &empty_tiers(), None);
        let scoped = card_named(&view, "a.tp2");
        assert!(matches!(scoped.fetching, Some(FetchPhase::Downloading(_))));
        let other = card_named(&view, "b.tp2");
        assert_eq!(other.fetching, None);
        assert!(!other.queued);

        state.step2.update_selected_download_running = false;
        state.step2.update_selected_extract_running = true;
        let view = build_versions_view(&state, &empty_tiers(), None);
        assert_eq!(
            card_named(&view, "a.tp2").fetching,
            Some(FetchPhase::Extracting)
        );
        assert_eq!(card_named(&view, "b.tp2").fetching, None);
    }

    #[test]
    fn short_target_cuts_a_commit_to_seven_and_keeps_tags() {
        assert_eq!(
            short_target("master@6c1f42b8184877d02226a1b2c3d4e5f6a7b8c9d0"),
            "master@6c1f42b"
        );
        assert_eq!(
            short_target("6c1f42b8184877d02226a1b2c3d4e5f6a7b8c9d0"),
            "6c1f42b"
        );
        assert_eq!(short_target("v35.17"), "v35.17");
    }

    #[test]
    fn card_with_asset_is_fetch_with_version_arrow() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("mod.tp2", "Mod", "~mod.tp2~ #0 #0 // 1.0"));
        state
            .step2
            .update_selected_update_assets
            .push(asset("mod.tp2", "Mod", "2.0"));

        let view = build_versions_view(&state, &empty_tiers(), None);
        assert_eq!(view.cards.len(), 1);
        let card = &view.cards[0];
        assert_eq!(card.status, CardStatus::Fetch);
        assert_eq!(card.dot, CardDot::Update);
        assert_eq!(card.status_line, "1.0 \u{2192} 2.0");
        assert_eq!(card.target.as_deref(), Some("2.0"));
        assert_eq!(view.fetch_count, 1);
    }

    #[test]
    fn log_pending_mod_not_on_disk_reads_not_on_disk() {
        let mut state = WizardState::default();
        state
            .step2
            .log_pending_downloads
            .push(Step2LogPendingDownload {
                game_tab: "BGEE".to_string(),
                tp_file: "pending.tp2".to_string(),
                label: "Pending".to_string(),
                requested_version: None,
            });
        state
            .step2
            .update_selected_update_assets
            .push(asset("pending.tp2", "Pending", "3.0"));

        let view = build_versions_view(&state, &empty_tiers(), None);
        assert_eq!(view.cards.len(), 1);
        assert_eq!(view.cards[0].status_line, "not on disk \u{2192} 3.0");
        assert_eq!(view.log_missing_count, 1);
    }

    #[test]
    fn failed_check_is_attention_with_error() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("mod.tp2", "Mod", "~mod.tp2~ #0 #0 // 1.0"));
        state
            .step2
            .update_selected_failed_sources
            .push("Mod: network error".to_string());

        let view = build_versions_view(&state, &empty_tiers(), None);
        let card = &view.cards[0];
        assert_eq!(card.status, CardStatus::Attention);
        assert_eq!(card.dot, CardDot::Warn);
        assert_eq!(card.status_line, "check failed \u{b7} network error");
        assert_eq!(view.attention_count, 1);
    }

    #[test]
    fn failed_fetch_is_attention() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("mod.tp2", "Mod", "~mod.tp2~ #0 #0 // 1.0"));
        state
            .step2
            .update_selected_download_failed_sources
            .push("Mod: disk full".to_string());

        let view = build_versions_view(&state, &empty_tiers(), None);
        let card = &view.cards[0];
        assert_eq!(card.status, CardStatus::Attention);
        assert_eq!(card.status_line, "fetch failed \u{b7} disk full");
    }

    #[test]
    fn mod_without_catalog_source_is_attention_with_bad_dot() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("mod.tp2", "Mod", "~mod.tp2~ #0 #0 // 1.0"));

        let view = build_versions_view(&state, &empty_tiers(), None);
        let card = &view.cards[0];
        assert_eq!(card.status, CardStatus::Attention);
        assert_eq!(card.dot, CardDot::Bad);
        assert_eq!(card.status_line, "no source");
        assert!(card.source_id.is_none());
    }

    #[test]
    fn scanned_manual_mod_without_version_is_in_sync() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("manual.tp2", "Manual", "~manual.tp2~ #0 #0"));
        let text = "[[mods]]\nname = \"Manual\"\ntp2 = \"manual.tp2\"\n\n  [[mods.sources]]\n  id = \"primary\"\n  label = \"Site\"\n  type = \"url\"\n  url = \"https://example.test/page\"\n  default = true\n";
        let tiers = mod_downloads::source_tiers_from_texts(text, "", "");

        let view = build_versions_view(&state, &tiers, None);
        let card = &view.cards[0];
        assert_eq!(card.status, CardStatus::InSync);
        assert_ne!(card.status_line, "manual download \u{b7} not on disk");
    }

    #[test]
    fn blank_version_never_leads_with_a_separator() {
        assert_eq!(arrow_status_line("", "2.0"), "? \u{2192} 2.0");
        assert_eq!(arrow_status_line("1.0", "2.0"), "1.0 \u{2192} 2.0");
        assert_eq!(
            locked_available_status_line("", "2.0"),
            "locked \u{b7} 2.0 available"
        );
        assert_eq!(
            locked_available_status_line("1.0", "2.0"),
            "1.0 \u{b7} locked \u{b7} 2.0 available"
        );
    }

    #[test]
    fn status_line_shows_versions_without_a_v_prefix_mismatch() {
        assert_eq!(arrow_status_line("35.10", "v35.17"), "35.10 \u{2192} 35.17");
        assert_eq!(arrow_status_line("35.10", "35.17"), "35.10 \u{2192} 35.17");
        assert_eq!(
            arrow_status_line("35.10", "version"),
            "35.10 \u{2192} version"
        );
        assert_eq!(
            locked_available_status_line("35.10", "v35.17"),
            "35.10 \u{b7} locked \u{b7} 35.17 available"
        );
    }

    #[test]
    fn manual_source_not_on_disk_is_attention() {
        let mut state = WizardState::default();
        state
            .step2
            .log_pending_downloads
            .push(Step2LogPendingDownload {
                game_tab: "BGEE".to_string(),
                tp_file: "manual.tp2".to_string(),
                label: "Manual".to_string(),
                requested_version: None,
            });
        let text = "[[mods]]\nname = \"Manual\"\ntp2 = \"manual.tp2\"\n\n  [[mods.sources]]\n  id = \"primary\"\n  label = \"Site\"\n  type = \"url\"\n  url = \"https://example.test/page\"\n  default = true\n";
        let tiers = mod_downloads::source_tiers_from_texts(text, "", "");

        let view = build_versions_view(&state, &tiers, None);
        let card = &view.cards[0];
        assert_eq!(card.status, CardStatus::Attention);
        assert_eq!(card.status_line, "manual download \u{b7} not on disk");
    }

    #[test]
    fn locked_mod_with_update_stays_in_sync_and_says_available() {
        let mut state = WizardState::default();
        let mut mod_state_value = mod_state("mod.tp2", "Mod", "~mod.tp2~ #0 #0 // 1.0");
        mod_state_value.update_locked = true;
        state.step2.bgee_mods.push(mod_state_value);
        state
            .step2
            .update_selected_locked_update_assets
            .push(asset("mod.tp2", "Mod", "2.0"));
        let tiers = mod_downloads::source_tiers_from_texts(&github_source("mod.tp2"), "", "");

        let view = build_versions_view(&state, &tiers, None);
        let card = &view.cards[0];
        assert_eq!(card.status, CardStatus::InSync);
        assert_eq!(card.status_line, "1.0 \u{b7} locked \u{b7} 2.0 available");
        assert!(card.locked);
        assert!(!card.can_fetch);
        assert_eq!(view.locked_count, 1);
    }

    #[test]
    fn extracted_mod_says_fetched_just_now() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("mod.tp2", "Mod", "~mod.tp2~ #0 #0 // 1.0"));
        state
            .step2
            .update_selected_extracted_sources
            .push("Mod -> C:\\mods\\mod".to_string());
        let tiers = mod_downloads::source_tiers_from_texts(&github_source("mod.tp2"), "", "");

        let view = build_versions_view(&state, &tiers, None);
        assert_eq!(view.cards[0].status_line, "1.0 \u{b7} fetched just now");
    }

    #[test]
    fn rule_words_cover_every_selector() {
        let mut source = ModDownloadSource {
            github: Some("o/r".to_string()),
            ..ModDownloadSource::default()
        };
        source.commit = Some("abcdef1234".to_string());
        assert_eq!(rule_words(&source), "Commit abcdef1");

        source.commit = None;
        source.tag = Some("v1.0".to_string());
        assert_eq!(rule_words(&source), "Tag v1.0");

        source.tag = None;
        source.branch = Some("main".to_string());
        assert_eq!(rule_words(&source), "Branch main");

        source.branch = None;
        source.release = Some("v2.0".to_string());
        assert_eq!(rule_words(&source), "Release v2.0");
        source.asset = Some("win.zip".to_string());
        assert_eq!(rule_words(&source), "Release v2.0 \u{b7} win.zip");

        source.release = None;
        source.asset = None;
        source.channel = Some("preonly".to_string());
        assert_eq!(rule_words(&source), "Newest pre-release");

        source.channel = Some("pre-release".to_string());
        assert_eq!(rule_words(&source), "Newest release + pre-releases");

        source.channel = Some("master".to_string());
        assert_eq!(rule_words(&source), "Default branch");

        source.channel = Some("ifeellucky".to_string());
        assert_eq!(rule_words(&source), "Newest release, else latest code");

        source.channel = None;
        assert_eq!(rule_words(&source), "Newest release");

        let mut weasel = ModDownloadSource {
            url: "https://downloads.weaselmods.net/download/x".to_string(),
            ..ModDownloadSource::default()
        };
        assert_eq!(rule_words(&weasel), "Weasel Mods page");

        weasel.url = "https://www.morpheus-mart.com/x".to_string();
        assert_eq!(rule_words(&weasel), "Morpheus Mart page");

        weasel.url = "https://example.test/file.zip".to_string();
        assert_eq!(rule_words(&weasel), "Direct archive");

        weasel.url = "https://example.test/page".to_string();
        assert_eq!(rule_words(&weasel), "Manual download");
    }

    #[test]
    fn selector_words_names_the_repo_first_without_the_asset() {
        let release = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            release: Some("v4.9.1".to_string()),
            asset: Some("lefreuts-enhanced-ui.zip".to_string()),
            ..ModDownloadSource::default()
        };
        assert_eq!(selector_words(&release), "owner/repo \u{b7} Release v4.9.1");

        let branch = ModDownloadSource {
            branch: Some("master".to_string()),
            ..ModDownloadSource::default()
        };
        assert_eq!(selector_words(&branch), "Branch master");
    }

    #[test]
    fn selector_hover_keeps_the_asset() {
        let release = ModDownloadSource {
            github: Some("owner/repo".to_string()),
            release: Some("v4.9.1".to_string()),
            asset: Some("lefreuts-enhanced-ui.zip".to_string()),
            ..ModDownloadSource::default()
        };
        assert_eq!(
            selector_hover(&release),
            "Release v4.9.1 \u{b7} lefreuts-enhanced-ui.zip \u{b7} owner/repo"
        );
    }

    #[test]
    fn layer_names_follow_tiers() {
        assert_eq!(layer_name(SourceTier::Modlist), "This modlist");
        assert_eq!(layer_name(SourceTier::User), "My default");
        assert_eq!(layer_name(SourceTier::Default), "BIO default");
    }

    #[test]
    fn cards_are_sorted_by_name_and_deduplicated() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("zeta.tp2", "Zeta", "~zeta.tp2~ #0 #0 // 1.0"));
        state
            .step2
            .bg2ee_mods
            .push(mod_state("zeta.tp2", "Zeta", "~zeta.tp2~ #0 #0 // 1.0"));
        state
            .step2
            .bgee_mods
            .push(mod_state("alpha.tp2", "Alpha", "~alpha.tp2~ #0 #0 // 1.0"));

        let view = build_versions_view(&state, &empty_tiers(), None);
        assert_eq!(view.cards.len(), 2);
        assert_eq!(view.cards[0].name, "Alpha");
        assert_eq!(view.cards[1].name, "Zeta");
    }

    #[test]
    fn every_scanned_mod_gets_a_card() {
        let mut state = WizardState::default();
        assert!(
            build_versions_view(&state, &empty_tiers(), None)
                .cards
                .is_empty()
        );

        let mut unticked = mod_state("mod.tp2", "Mod", "~mod.tp2~ #0 #0 // 1.0");
        unticked.checked = false;
        for component in &mut unticked.components {
            component.checked = false;
            component.selected_order = None;
        }
        state.step2.bg2ee_mods.push(unticked);

        let view = build_versions_view(&state, &empty_tiers(), None);
        assert_eq!(view.cards.len(), 1);
        assert_eq!(view.cards[0].name, "Mod");
    }

    #[test]
    fn known_rows_merge_identical_rules_and_order_by_kind() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(mod_state("mod.tp2", "Mod", "~mod.tp2~ #0 #0 // 1.0"));
        let tiers = mod_downloads::source_tiers_from_texts(&github_source("mod"), "", "");

        let same_as_current = ModDownloadSource {
            tp2: "mod".to_string(),
            source_id: "y".to_string(),
            github: Some("O/MOD".to_string()),
            ..ModDownloadSource::default()
        };
        let distinct_pin = ModDownloadSource {
            tp2: "mod".to_string(),
            source_id: "z".to_string(),
            github: Some("o/mod".to_string()),
            tag: Some("v2.0".to_string()),
            ..ModDownloadSource::default()
        };
        let bookmark_source = ModDownloadSource {
            tp2: "mod".to_string(),
            github: Some("o/mod".to_string()),
            branch: Some("dev".to_string()),
            ..ModDownloadSource::default()
        };
        let history_source = ModDownloadSource {
            tp2: "mod".to_string(),
            github: Some("o/mod".to_string()),
            commit: Some("abcdef1234567".to_string()),
            ..ModDownloadSource::default()
        };

        let extras = index_known_extras(
            vec![
                ("ListB".to_string(), vec![same_as_current]),
                ("ListC".to_string(), vec![distinct_pin]),
            ],
            ModSourceHistoryStore {
                bookmarks: vec![mod_source_history::BookmarkEntry {
                    tp2: "mod".to_string(),
                    block: mod_downloads::complete_source_block(&bookmark_source),
                    version: "v1.2".to_string(),
                    date: "2026-09-20".to_string(),
                }],
                history: vec![mod_source_history::HistoryEntry {
                    tp2: "mod".to_string(),
                    block: mod_downloads::complete_source_block(&history_source),
                    saved_to: "My default".to_string(),
                    date: "2026-09-19".to_string(),
                }],
                notes: std::collections::BTreeMap::new(),
            },
        );

        let view = build_versions_view(&state, &tiers, Some(&extras));
        let card = &view.cards[0];
        assert_eq!(card.sources.len(), 4);
        assert_eq!(card.sources[0].kind, SourceRowKind::Current);
        assert_eq!(card.sources[1].kind, SourceRowKind::Pin);
        assert_eq!(card.sources[1].who, "ListC");
        assert_eq!(card.sources[2].kind, SourceRowKind::Bookmark);
        assert_eq!(card.sources[3].kind, SourceRowKind::Past);
    }

    #[test]
    fn other_lists_pins_name_the_list_or_count() {
        let source_a = ModDownloadSource {
            tp2: "mod".to_string(),
            github: Some("o/mod".to_string()),
            tag: Some("v1.0".to_string()),
            ..ModDownloadSource::default()
        };
        let source_b = ModDownloadSource {
            tp2: "mod".to_string(),
            github: Some("O/MOD".to_string()),
            tag: Some("v1.0".to_string()),
            ..ModDownloadSource::default()
        };
        let source_c = ModDownloadSource {
            tp2: "mod".to_string(),
            github: Some("o/mod".to_string()),
            tag: Some("v2.0".to_string()),
            ..ModDownloadSource::default()
        };
        let extras = index_known_extras(
            vec![
                ("Tactics".to_string(), vec![source_a]),
                ("Speedrun".to_string(), vec![source_b]),
                ("Solo".to_string(), vec![source_c]),
            ],
            ModSourceHistoryStore::default(),
        );

        let rows = pin_rows(&extras, "mod");
        assert_eq!(rows.len(), 2);
        let merged = rows
            .iter()
            .find(|(_, option)| option.who_hover.len() == 2)
            .expect("a merged pin row must exist");
        assert_eq!(merged.1.who, "2 modlists");
        assert!(merged.1.who_hover.contains(&"Tactics".to_string()));
        assert!(merged.1.who_hover.contains(&"Speedrun".to_string()));
        let single = rows
            .iter()
            .find(|(_, option)| option.who_hover.len() == 1)
            .expect("a single-list pin row must exist");
        assert_eq!(single.1.who, "Solo");
    }

    #[test]
    fn known_extras_rows_need_no_reparse() {
        let pin_source = ModDownloadSource {
            tp2: "mod".to_string(),
            github: Some("o/mod".to_string()),
            tag: Some("v1.0".to_string()),
            ..ModDownloadSource::default()
        };
        let bookmark_source = ModDownloadSource {
            tp2: "mod".to_string(),
            github: Some("o/mod".to_string()),
            branch: Some("dev".to_string()),
            ..ModDownloadSource::default()
        };
        let history_source = ModDownloadSource {
            tp2: "mod".to_string(),
            github: Some("o/mod".to_string()),
            commit: Some("abcdef1234567".to_string()),
            ..ModDownloadSource::default()
        };

        let extras = index_known_extras(
            vec![("ListA".to_string(), vec![pin_source])],
            ModSourceHistoryStore {
                bookmarks: vec![mod_source_history::BookmarkEntry {
                    tp2: "mod".to_string(),
                    block: mod_downloads::complete_source_block(&bookmark_source),
                    version: "v1.2".to_string(),
                    date: "2026-09-20".to_string(),
                }],
                history: vec![mod_source_history::HistoryEntry {
                    tp2: "mod".to_string(),
                    block: mod_downloads::complete_source_block(&history_source),
                    saved_to: "My default".to_string(),
                    date: "2026-09-19".to_string(),
                }],
                notes: std::collections::BTreeMap::new(),
            },
        );

        assert_eq!(extras.pin_index.get("mod").map(Vec::len), Some(1));
        assert_eq!(
            extras.pin_index.get("mod").unwrap()[0].1.source.github,
            Some("o/mod".to_string())
        );
        assert_eq!(extras.bookmark_index.get("mod").map(Vec::len), Some(1));
        assert_eq!(
            extras.bookmark_index.get("mod").unwrap()[0].1.source.branch,
            Some("dev".to_string())
        );
        assert_eq!(extras.history_index.get("mod").map(Vec::len), Some(1));
        assert_eq!(
            extras.history_index.get("mod").unwrap()[0].1.source.commit,
            Some("abcdef1234567".to_string())
        );
        assert!(!extras.pin_index.contains_key("missing"));
    }

    struct BookmarkLabelTestRoot {
        config_dir: std::path::PathBuf,
        previous_modlist_dir: Option<std::path::PathBuf>,
    }

    impl BookmarkLabelTestRoot {
        fn create() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let config_dir = std::env::temp_dir().join(format!(
                "bio_versions_view_bookmark_label_test_{}_{id}",
                std::process::id()
            ));
            let previous_modlist_dir = mod_downloads::active_modlist_dir();
            let root = Self {
                config_dir,
                previous_modlist_dir,
            };
            mod_downloads::set_active_modlist_dir(None);
            std::fs::create_dir_all(&root.config_dir).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(root.config_dir.clone()));
            root
        }
    }

    impl Drop for BookmarkLabelTestRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.config_dir);
            mod_downloads::set_active_modlist_dir(self.previous_modlist_dir.take());
            let _ = std::fs::remove_dir_all(&self.config_dir);
        }
    }

    #[test]
    fn bookmark_label_shows_short_commit_or_tag() {
        let _lock = mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _root = BookmarkLabelTestRoot::create();

        let source = ModDownloadSource {
            tp2: "mod".to_string(),
            name: "Mod".to_string(),
            source_id: "primary".to_string(),
            source_label: "GitHub".to_string(),
            github: Some("o/mod".to_string()),
            ..ModDownloadSource::default()
        };
        let header = mod_downloads::template_mod_header("mod", "Mod");
        let block = mod_downloads::complete_source_block(&source);
        std::fs::write(
            mod_downloads::mod_downloads_user_path(),
            format!("{header}\n\n{block}\n"),
        )
        .unwrap();

        std::fs::write(
            crate::app::app_step2_update_source_refs::installed_source_refs_path(),
            "[refs]\nmod = \"commit@abcdef1234\"\n\n[sources]\nmod = \"primary\"\n",
        )
        .unwrap();

        let state = WizardState::default();
        let label = bookmark_label(&state, "mod");
        assert_eq!(label.as_deref(), Some("Bookmark abcdef1"));

        std::fs::write(
            crate::app::app_step2_update_source_refs::installed_source_refs_path(),
            "[refs]\nmod = \"v35.10\"\n\n[sources]\nmod = \"primary\"\n",
        )
        .unwrap();

        let tag_label = bookmark_label(&state, "mod");
        assert_eq!(tag_label.as_deref(), Some("Bookmark v35.10"));
    }
}
