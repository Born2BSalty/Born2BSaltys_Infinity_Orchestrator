// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::app::state::WizardState;
use crate::platform_defaults::app_config_file;

const RELEASE_LIST_QUERY: &str = "query($owner: String!, $repo: String!) {\n  repository(owner: $owner, name: $repo) {\n    releases(first: 100, orderBy: {field: CREATED_AT, direction: DESC}) {\n      nodes {\n        tagName\n        name\n        isPrerelease\n        isDraft\n        publishedAt\n        releaseAssets(first: 50) {\n          nodes {\n            name\n            size\n            contentType\n            downloadUrl\n          }\n        }\n      }\n    }\n  }\n}";

const RELEASE_CACHE_FILE_NAME: &str = "github_release_cache.json";
const RELEASE_CACHE_FRESH_SECONDS: u64 = 24 * 60 * 60;

static CACHE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CachedAsset {
    pub(crate) name: String,
    pub(crate) size: u64,
    pub(crate) download_url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CachedRelease {
    pub(crate) tag: String,
    pub(crate) name: String,
    pub(crate) prerelease: bool,
    pub(crate) published_at: String,
    pub(crate) assets: Vec<CachedAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ReleaseListStatus {
    #[default]
    Idle,
    Loading,
    Ready(Vec<CachedRelease>),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ReleaseListState {
    pub(crate) repo: String,
    pub(crate) status: ReleaseListStatus,
}

#[derive(Debug)]
pub(crate) enum ReleaseListEvent {
    Finished {
        repo: String,
        result: Result<Vec<CachedRelease>, String>,
    },
}

pub(crate) fn spawn_release_list_fetch(
    owner_repo: String,
    force: bool,
) -> Receiver<ReleaseListEvent> {
    let (tx, rx) = mpsc::channel::<ReleaseListEvent>();
    thread::spawn(move || {
        let result = fetch_release_list_cached(&owner_repo, force);
        let _ = tx.send(ReleaseListEvent::Finished {
            repo: owner_repo,
            result,
        });
    });
    rx
}

pub(crate) struct ReleaseListFetch {
    pub(crate) repo: String,
    rx: Receiver<ReleaseListEvent>,
}

pub(crate) fn fetch_is_stale(fetch: &ReleaseListFetch, wanted: &str) -> bool {
    fetch.repo != wanted
}

pub(crate) fn poll_release_list(state: &mut WizardState, fetch: &mut Option<ReleaseListFetch>) {
    let release_list = &state.step2.versions_ui.release_list;
    if release_list.status == ReleaseListStatus::Loading
        && fetch
            .as_ref()
            .is_none_or(|in_flight| fetch_is_stale(in_flight, &release_list.repo))
    {
        let repo = release_list.repo.clone();
        *fetch = Some(ReleaseListFetch {
            rx: spawn_release_list_fetch(repo.clone(), false),
            repo,
        });
        return;
    }
    let Some(in_flight) = fetch.as_ref() else {
        return;
    };
    match in_flight.rx.try_recv() {
        Ok(ReleaseListEvent::Finished { repo, result }) => {
            let finished_in_flight = repo == in_flight.repo;
            if state.step2.versions_ui.release_list.repo == repo {
                state.step2.versions_ui.release_list.status = match result {
                    Ok(releases) => ReleaseListStatus::Ready(releases),
                    Err(err) => ReleaseListStatus::Failed(err),
                };
            }
            if finished_in_flight {
                *fetch = None;
            }
        }
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => *fetch = None,
    }
}

fn fetch_release_list_cached(owner_repo: &str, force: bool) -> Result<Vec<CachedRelease>, String> {
    if !force
        && let Some(entry) = load_cache_entry(owner_repo)
        && release_cache_is_fresh(now_unix(), entry.fetched_at)
    {
        return Ok(entry.releases);
    }
    let (owner, repo) = split_owner_repo(owner_repo)?;
    let token = super::app_step2_update_github_auth::load_github_token()
        .ok_or_else(|| "Not connected to GitHub".to_string())?;
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .build();
    let releases = fetch_release_list(&agent, &owner, &repo, &token)?;
    store_cache_entry(owner_repo, &releases);
    Ok(releases)
}

pub(crate) fn fetch_release_list(
    agent: &ureq::Agent,
    owner: &str,
    repo: &str,
    token: &str,
) -> Result<Vec<CachedRelease>, String> {
    let response = agent
        .post("https://api.github.com/graphql")
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", "BIO-update-check")
        .set("Content-Type", "application/json")
        .send_json(serde_json::json!({
            "query": RELEASE_LIST_QUERY,
            "variables": { "owner": owner, "repo": repo },
        }))
        .map_err(|err| err.to_string())?;
    let text = response.into_string().map_err(|err| err.to_string())?;
    parse_release_list_response(&text)
}

fn split_owner_repo(owner_repo: &str) -> Result<(String, String), String> {
    let trimmed = owner_repo.trim();
    let mut parts = trimmed.splitn(2, '/');
    match (parts.next(), parts.next()) {
        (Some(owner), Some(repo)) if !owner.is_empty() && !repo.is_empty() => {
            Ok((owner.to_string(), repo.to_string()))
        }
        _ => Err(format!("Invalid repository \"{trimmed}\"")),
    }
}

#[derive(Debug, Deserialize)]
struct GraphQlEnvelope {
    #[serde(default)]
    data: Option<GraphQlData>,
    #[serde(default)]
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphQlData {
    repository: Option<GraphQlRepository>,
}

#[derive(Debug, Deserialize)]
struct GraphQlRepository {
    releases: GraphQlReleases,
}

#[derive(Debug, Deserialize)]
struct GraphQlReleases {
    nodes: Vec<GraphQlReleaseNode>,
}

#[derive(Debug, Deserialize)]
struct GraphQlReleaseNode {
    #[serde(rename = "tagName", default)]
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "isPrerelease", default)]
    is_prerelease: bool,
    #[serde(rename = "isDraft", default)]
    is_draft: bool,
    #[serde(rename = "publishedAt", default)]
    published_at: Option<String>,
    #[serde(rename = "releaseAssets", default)]
    release_assets: GraphQlAssets,
}

#[derive(Debug, Default, Deserialize)]
struct GraphQlAssets {
    #[serde(default)]
    nodes: Vec<GraphQlAssetNode>,
}

#[derive(Debug, Deserialize)]
struct GraphQlAssetNode {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(rename = "downloadUrl", default)]
    download_url: String,
}

fn parse_release_list_response(text: &str) -> Result<Vec<CachedRelease>, String> {
    let envelope = serde_json::from_str::<GraphQlEnvelope>(text).map_err(|err| err.to_string())?;
    if let Some(message) = envelope
        .errors
        .and_then(|errors| errors.into_iter().next())
        .map(|error| error.message)
    {
        return Err(message);
    }
    let nodes = envelope
        .data
        .and_then(|data| data.repository)
        .map(|repository| repository.releases.nodes)
        .unwrap_or_default();
    Ok(nodes
        .into_iter()
        .filter(|node| !node.is_draft)
        .map(|node| CachedRelease {
            tag: node.tag_name,
            name: node.name.unwrap_or_default(),
            prerelease: node.is_prerelease,
            published_at: node.published_at.unwrap_or_default(),
            assets: node
                .release_assets
                .nodes
                .into_iter()
                .map(|asset| CachedAsset {
                    name: asset.name,
                    size: asset.size,
                    download_url: asset.download_url,
                })
                .collect(),
        })
        .collect())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ReleaseCacheDisk {
    #[serde(default)]
    entries: BTreeMap<String, ReleaseCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseCacheEntry {
    fetched_at: u64,
    releases: Vec<CachedRelease>,
}

fn release_cache_path() -> PathBuf {
    app_config_file(RELEASE_CACHE_FILE_NAME, ".")
}

fn load_release_cache() -> ReleaseCacheDisk {
    let path = release_cache_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_release_cache(cache: &ReleaseCacheDisk) {
    let path = release_cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(path, json);
    }
}

fn load_cache_entry(owner_repo: &str) -> Option<ReleaseCacheEntry> {
    load_release_cache()
        .entries
        .get(&normalize_repo_key(owner_repo))
        .cloned()
}

fn store_cache_entry(owner_repo: &str, releases: &[CachedRelease]) {
    let _held = CACHE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let mut cache = load_release_cache();
    cache.entries.insert(
        normalize_repo_key(owner_repo),
        ReleaseCacheEntry {
            fetched_at: now_unix(),
            releases: releases.to_vec(),
        },
    );
    save_release_cache(&cache);
}

pub(crate) fn drop_cached_release_lists(repos: &[String]) {
    let _held = CACHE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let mut cache = load_release_cache();
    let mut changed = false;
    for repo in repos {
        if cache.entries.remove(&normalize_repo_key(repo)).is_some() {
            changed = true;
        }
    }
    if changed {
        save_release_cache(&cache);
    }
}

fn normalize_repo_key(owner_repo: &str) -> String {
    owner_repo.trim().to_ascii_lowercase()
}

pub(crate) const fn release_cache_is_fresh(now: u64, fetched_at: u64) -> bool {
    now.saturating_sub(fetched_at) < RELEASE_CACHE_FRESH_SECONDS
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    struct TempConfigDir {
        root: PathBuf,
    }

    impl TempConfigDir {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "bio_github_release_list_test_{}_{}_{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst),
                label
            ));
            fs::create_dir_all(&root).expect("create temp config dir");
            crate::platform_defaults::set_config_dir_override(Some(root.clone()));
            Self { root }
        }
    }

    impl Drop for TempConfigDir {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.root);
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn sample_release(tag: &str) -> CachedRelease {
        CachedRelease {
            tag: tag.to_string(),
            name: tag.to_string(),
            prerelease: false,
            published_at: "2026-09-27T00:00:00Z".to_string(),
            assets: vec![CachedAsset {
                name: format!("{tag}.zip"),
                size: 10,
                download_url: format!("https://example.test/{tag}.zip"),
            }],
        }
    }

    #[test]
    fn fetch_is_stale_when_repo_differs() {
        let (_tx, rx) = mpsc::channel::<ReleaseListEvent>();
        let fetch = ReleaseListFetch {
            repo: "owner/first".to_string(),
            rx,
        };
        assert!(!fetch_is_stale(&fetch, "owner/first"));
        assert!(fetch_is_stale(&fetch, "owner/second"));
    }

    #[test]
    fn graphql_response_parse_drops_drafts() {
        let text = r#"{
            "data": {
                "repository": {
                    "releases": {
                        "nodes": [
                            {"tagName": "v1.0", "name": "v1.0", "isPrerelease": false, "isDraft": false, "publishedAt": "2026-01-01T00:00:00Z", "releaseAssets": {"nodes": []}},
                            {"tagName": "v1.1-draft", "name": "draft", "isPrerelease": false, "isDraft": true, "publishedAt": "2026-01-02T00:00:00Z", "releaseAssets": {"nodes": []}}
                        ]
                    }
                }
            }
        }"#;
        let releases = parse_release_list_response(text).expect("parse succeeds");
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].tag, "v1.0");
    }

    #[test]
    fn graphql_errors_become_err() {
        let text = r#"{"errors": [{"message": "bad credentials"}]}"#;
        let err = parse_release_list_response(text).expect_err("should fail");
        assert_eq!(err, "bad credentials");
    }

    #[test]
    fn release_cache_is_fresh_for_24_hours() {
        assert!(release_cache_is_fresh(1_000_000, 1_000_000));
        assert!(release_cache_is_fresh(1_000_000 + 86_399, 1_000_000));
        assert!(!release_cache_is_fresh(1_000_000 + 86_400, 1_000_000));
        assert!(!release_cache_is_fresh(1_000_000 + 90_000, 1_000_000));
    }

    #[test]
    fn release_cache_round_trips() {
        let _guard = TempConfigDir::new("round-trip");
        assert!(load_cache_entry("owner/repo").is_none());
        store_cache_entry("Owner/Repo", &[sample_release("v1.0")]);
        let entry = load_cache_entry("owner/repo").expect("entry present");
        assert_eq!(entry.releases.len(), 1);
        assert_eq!(entry.releases[0].tag, "v1.0");
    }

    #[test]
    fn drop_cached_release_lists_removes_only_named_repos() {
        let _guard = TempConfigDir::new("drop-only-named");
        store_cache_entry("owner/keep", &[sample_release("v1.0")]);
        store_cache_entry("owner/drop", &[sample_release("v2.0")]);

        drop_cached_release_lists(&["owner/drop".to_string()]);

        assert!(load_cache_entry("owner/keep").is_some());
        assert!(load_cache_entry("owner/drop").is_none());
    }
}
