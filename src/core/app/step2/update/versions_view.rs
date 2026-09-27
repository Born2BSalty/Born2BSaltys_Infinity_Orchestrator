// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeSet;

use crate::app::mod_downloads::{self, ModDownloadSource, SourceTier, SourceTiers};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CardSourceOption {
    pub(crate) source_id: String,
    pub(crate) layer: &'static str,
    pub(crate) rule_words: String,
    pub(crate) location: String,
    pub(crate) current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub(crate) open_url: Option<String>,
    pub(crate) repo: Option<String>,
    pub(crate) source_id: Option<String>,
    pub(crate) sources: Vec<CardSourceOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub(crate) fn build_versions_view(state: &WizardState, tiers: &SourceTiers) -> VersionsView {
    let cards = collect_card_basis(state)
        .into_iter()
        .map(|basis| build_card(state, tiers, basis))
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
    VersionsView {
        cards,
        fetch_count,
        attention_count,
        locked_count,
        log_missing_count,
    }
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
        if !mod_is_selected(mod_state) {
            continue;
        }
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

fn mod_is_selected(mod_state: &Step2ModState) -> bool {
    mod_state.checked
        || mod_state
            .components
            .iter()
            .any(|component| component.checked)
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
    let just_fetched = state
        .step2
        .update_selected_extracted_sources
        .iter()
        .any(|entry| entry.starts_with(&format!("{name} -> ")));
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
            status_line: arrow_status_line(version, &asset.tag),
            target: Some(asset.tag.clone()),
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
            status_line: locked_available_status_line(version, &asset.tag),
            target: Some(asset.tag.clone()),
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

fn build_card(state: &WizardState, tiers: &SourceTiers, basis: CardBasis) -> VersionCard {
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

    let (layer, rule_words_value) = source.as_ref().map_or_else(
        || ("", String::new()),
        |source| {
            (
                layer_name(tiers.tier_of(&source.tp2, &source.source_id)),
                rule_words(source),
            )
        },
    );
    let repo = source.as_ref().and_then(|source| source.github.clone());
    let open_url = source.as_ref().and_then(mod_downloads::source_open_url);
    let source_id = source.as_ref().map(|source| source.source_id.clone());
    let can_fetch = outcome.status == CardStatus::Fetch && !basis.locked;

    let sources = tiers
        .find_sources(&basis.tp2_key)
        .into_iter()
        .map(|option_source| CardSourceOption {
            layer: layer_name(tiers.tier_of(&option_source.tp2, &option_source.source_id)),
            rule_words: rule_words(&option_source),
            location: location_for(&option_source),
            current: source_id.as_deref() == Some(option_source.source_id.as_str()),
            source_id: option_source.source_id,
        })
        .collect();

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
        open_url,
        repo,
        source_id,
        sources,
    }
}

pub(crate) fn rule_words(source: &ModDownloadSource) -> String {
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
        return non_empty(source.asset.as_deref()).map_or_else(
            || format!("Release {release}"),
            |asset| format!("Release {release} \u{b7} {asset}"),
        );
    }
    if source.github.is_some() {
        return match source.channel.as_deref().map(str::trim) {
            Some("preonly") => "Newest pre-release".to_string(),
            Some("pre-release") => "Newest release + pre-releases".to_string(),
            Some("master") => "Latest code".to_string(),
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

        let view = build_versions_view(&state, &empty_tiers());
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

        let view = build_versions_view(&state, &empty_tiers());
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

        let view = build_versions_view(&state, &empty_tiers());
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

        let view = build_versions_view(&state, &empty_tiers());
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

        let view = build_versions_view(&state, &empty_tiers());
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

        let view = build_versions_view(&state, &tiers);
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

        let view = build_versions_view(&state, &tiers);
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

        let view = build_versions_view(&state, &tiers);
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

        let view = build_versions_view(&state, &tiers);
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
        assert_eq!(rule_words(&source), "Latest code");

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

        let view = build_versions_view(&state, &empty_tiers());
        assert_eq!(view.cards.len(), 2);
        assert_eq!(view.cards[0].name, "Alpha");
        assert_eq!(view.cards[1].name, "Zeta");
    }
}
