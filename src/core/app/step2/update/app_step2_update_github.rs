// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io::Read;

use serde::Deserialize;

use super::app_step2_update_check::{
    Step2PackageKind, Step2UpdateCheckOutcome, Step2UpdateCheckRequest, failed_outcome,
};
use crate::app::mod_downloads;
use crate::parser::weidu_version::normalize_version_text;

pub(super) fn check_github_download_page(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    if let Some(tag) = request.tag.as_deref() {
        return tag_source_outcome(agent, request, tag);
    }
    if let Some(branch) = request.branch.as_deref() {
        return branch_source_outcome(agent, request, branch);
    }
    if request.requested_version.is_some() {
        return check_github_exact_version_download_page(agent, request);
    }
    if request.channel.is_some() || request.asset.is_some() || request.pkg.is_some() {
        return check_github_modhub_download_page(agent, request);
    }
    check_github_default_api_download_page(agent, request)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitHubChannel {
    Stable,
    Pre,
    PreOnly,
    Master,
    IFeelLucky,
}

fn check_github_modhub_download_page(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    let channel = requested_github_channel(request.channel.as_deref());
    if (request.asset.is_some() || request.pkg.is_some())
        && matches!(channel, GitHubChannel::Master | GitHubChannel::IFeelLucky)
    {
        return failed_outcome(
            request.clone(),
            "GitHub channel does not support asset selection",
        );
    }
    if matches!(channel, GitHubChannel::Master) {
        return repo_source_outcome(agent, request);
    }
    let releases = match fetch_github_releases(agent, &request.repo) {
        Ok(releases) => releases,
        Err(err) => return failed_outcome(request.clone(), &err),
    };
    for release in &releases {
        if !release_matches_channel(release, channel) {
            continue;
        }
        if let Some(asset_name) = request.asset.as_deref() {
            if let Some(outcome) = named_release_asset_outcome(request, release, asset_name) {
                return outcome;
            }
            return failed_outcome(
                request.clone(),
                &format!("GitHub release asset not found: {}", asset_name.trim()),
            );
        }
        if let Some(pkg_list) = request.pkg.as_deref() {
            if let Some(outcome) = packaged_release_outcome(request, release, pkg_list) {
                return outcome;
            }
            return tagged_source_outcome(request, release);
        }
        return tagged_source_outcome(request, release);
    }
    if matches!(channel, GitHubChannel::IFeelLucky) {
        return repo_source_outcome(agent, request);
    }
    failed_outcome(request.clone(), "no matching GitHub release found")
}

fn check_github_exact_version_download_page(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    let Some(requested_version) = request.requested_version.as_deref() else {
        return failed_outcome(request.clone(), "requested version is missing");
    };
    let requested_normalized = normalize_version_text(requested_version);
    let mut candidate_repos = vec![request.repo.trim().to_string()];
    for repo in &request.exact_github {
        let repo = repo.trim();
        if repo.is_empty()
            || candidate_repos
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(repo))
        {
            continue;
        }
        candidate_repos.push(repo.to_string());
    }
    let mut first_error = None;
    for repo in candidate_repos {
        match exact_version_outcome_for_repo(agent, request, &repo, &requested_normalized) {
            Ok(Some(outcome)) => return outcome,
            Ok(None) => {}
            Err(err) => {
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
        }
    }
    if let Some(err) = first_error {
        return failed_outcome(request.clone(), &err);
    }
    failed_outcome(
        request.clone(),
        &format!("exact version not found: {requested_version}"),
    )
}

fn exact_version_outcome_for_repo(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
    repo: &str,
    requested_normalized: &str,
) -> Result<Option<Step2UpdateCheckOutcome>, String> {
    let request = request_for_repo(request, repo);
    let releases = fetch_github_releases(agent, &request.repo)?;
    if let Some(release) = releases
        .iter()
        .find(|release| normalize_version_text(&release.tag_name) == requested_normalized)
    {
        if let Some(asset_name) = request.asset.as_deref() {
            if let Some(outcome) = named_release_asset_outcome(&request, release, asset_name) {
                return Ok(Some(outcome));
            }
            return Ok(Some(failed_outcome(
                request.clone(),
                &format!("GitHub release asset not found: {}", asset_name.trim()),
            )));
        }
        if let Some(pkg_list) = request.pkg.as_deref() {
            if let Some(outcome) = packaged_release_outcome(&request, release, pkg_list) {
                return Ok(Some(outcome));
            }
            return Ok(Some(tagged_source_outcome(&request, release)));
        }
        if let Some(outcome) = release_asset_outcome(&request, release) {
            return Ok(Some(outcome));
        }
        return Ok(Some(tagged_source_outcome(&request, release)));
    }
    if let Ok(tags) = fetch_github_tags(agent, &request.repo)
        && let Some(tag) = tags
            .iter()
            .find(|tag| normalize_version_text(&tag.name) == requested_normalized)
    {
        return Ok(Some(tagged_tag_source_outcome(&request, tag)));
    }
    if repo_source_declares_requested_version(agent, &request, requested_normalized) {
        return Ok(Some(repo_source_outcome(agent, &request)));
    }
    Ok(None)
}

fn latest_repo_source_zip(agent: &ureq::Agent, repo: &str) -> Option<(String, String, String)> {
    let repo = fetch_github_repo(agent, repo).ok()?;
    let branch = if repo.default_branch.trim().is_empty() {
        "main".to_string()
    } else {
        repo.default_branch
    };
    let commit = fetch_github_branch_head(agent, repo.full_name.trim(), &branch).ok()?;
    let repo_name = repo.name.trim().to_string();
    Some((
        super::app_step2_update_github_ref::format_branch_head_ref(&branch, &commit.sha),
        format!("{repo_name}-{branch}-source.zip"),
        format!(
            "https://github.com/{}/archive/refs/heads/{}.zip",
            repo.full_name.trim(),
            branch
        ),
    ))
}

fn branch_source_zip(
    agent: &ureq::Agent,
    repo: &str,
    branch: &str,
) -> Option<(String, String, String)> {
    let repo = fetch_github_repo(agent, repo).ok()?;
    let branch = branch.trim();
    if branch.is_empty() {
        return None;
    }
    let commit = fetch_github_branch_head(agent, repo.full_name.trim(), branch).ok()?;
    let repo_name = repo.name.trim().to_string();
    Some((
        super::app_step2_update_github_ref::format_branch_head_ref(branch, &commit.sha),
        format!("{repo_name}-{branch}-source.zip"),
        format!(
            "https://github.com/{}/archive/refs/heads/{}.zip",
            repo.full_name.trim(),
            branch
        ),
    ))
}

fn check_github_default_api_download_page(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    let release = match fetch_latest_proper_release(agent, &request.repo) {
        Ok(Some(release)) => release,
        Ok(None) => return repo_source_outcome(agent, request),
        Err(err) => return failed_outcome(request.clone(), &err),
    };
    if let Some(outcome) = release_asset_outcome(request, &release) {
        return outcome;
    }
    tagged_source_outcome(request, &release)
}

fn requested_github_channel(value: Option<&str>) -> GitHubChannel {
    match value
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("pre-release") => GitHubChannel::Pre,
        Some("preonly") => GitHubChannel::PreOnly,
        Some("releases") => GitHubChannel::Stable,
        Some("master") => GitHubChannel::Master,
        Some("ifeellucky") => GitHubChannel::IFeelLucky,
        _ => GitHubChannel::Stable,
    }
}

fn release_matches_channel(release: &GitHubRelease, channel: GitHubChannel) -> bool {
    match channel {
        GitHubChannel::Stable => !release.prerelease,
        GitHubChannel::Pre => release.prerelease,
        GitHubChannel::PreOnly => release.prerelease,
        GitHubChannel::Master | GitHubChannel::IFeelLucky => true,
    }
}

fn release_asset_outcome(
    request: &Step2UpdateCheckRequest,
    release: &GitHubRelease,
) -> Option<Step2UpdateCheckOutcome> {
    let assets = release
        .assets
        .iter()
        .map(|asset| (asset.name.as_str(), asset.browser_download_url.as_str()))
        .collect::<Vec<_>>();
    let (_, asset_name, asset_url) =
        super::app_step2_update_asset_pick::pick_release_asset_for_current_os(&assets)?;
    Some(Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: Some(release.tag_name.clone()),
        asset_name: Some(asset_name),
        asset_url: Some(asset_url),
        error: None,
        package_kind: Step2PackageKind::ReleaseAsset,
    })
}

fn packaged_release_outcome(
    request: &Step2UpdateCheckRequest,
    release: &GitHubRelease,
    pkg_list: &str,
) -> Option<Step2UpdateCheckOutcome> {
    let assets = release
        .assets
        .iter()
        .map(|asset| (asset.name.as_str(), asset.browser_download_url.as_str()))
        .collect::<Vec<_>>();
    let (_, asset_name, asset_url) =
        super::app_step2_update_asset_pick::pick_release_asset_for_pkg_list(&assets, pkg_list)?;
    Some(Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: Some(release.tag_name.clone()),
        asset_name: Some(asset_name),
        asset_url: Some(asset_url),
        error: None,
        package_kind: Step2PackageKind::ReleaseAsset,
    })
}

fn named_release_asset_outcome(
    request: &Step2UpdateCheckRequest,
    release: &GitHubRelease,
    asset_name: &str,
) -> Option<Step2UpdateCheckOutcome> {
    let assets = release
        .assets
        .iter()
        .map(|asset| (asset.name.as_str(), asset.browser_download_url.as_str()))
        .collect::<Vec<_>>();
    let (_, asset_name, asset_url) =
        super::app_step2_update_asset_pick::pick_release_asset_by_name(&assets, asset_name)?;
    Some(Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: Some(release.tag_name.clone()),
        asset_name: Some(asset_name),
        asset_url: Some(asset_url),
        error: None,
        package_kind: Step2PackageKind::ReleaseAsset,
    })
}

fn tagged_source_outcome(
    request: &Step2UpdateCheckRequest,
    release: &GitHubRelease,
) -> Step2UpdateCheckOutcome {
    let repo_name = request
        .repo
        .rsplit('/')
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("source");
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: Some(release.tag_name.clone()),
        asset_name: Some(format!("{repo_name}-{}-source.zip", release.tag_name)),
        asset_url: Some(release.zipball_url.clone()),
        error: None,
        package_kind: Step2PackageKind::SourceSnapshot,
    }
}

fn branch_source_outcome(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
    branch: &str,
) -> Step2UpdateCheckOutcome {
    if let Some((source_ref, asset_name, asset_url)) =
        branch_source_zip(agent, &request.repo, branch)
    {
        return Step2UpdateCheckOutcome {
            game_tab: request.game_tab.clone(),
            tp_file: request.tp_file.clone(),
            label: request.label.clone(),
            source_id: request.source_id.clone(),
            tag: Some(source_ref),
            asset_name: Some(asset_name),
            asset_url: Some(asset_url),
            error: None,
            package_kind: Step2PackageKind::SourceSnapshot,
        };
    }
    failed_outcome(
        request.clone(),
        &format!("GitHub branch not found: {}", branch.trim()),
    )
}

fn tag_source_outcome(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
    tag: &str,
) -> Step2UpdateCheckOutcome {
    let tag = tag.trim();
    if tag.is_empty() {
        return failed_outcome(request.clone(), "GitHub tag is empty");
    }
    let Ok(tags) = fetch_github_tags(agent, &request.repo) else {
        return failed_outcome(
            request.clone(),
            &format!("GitHub tag not found: {tag}"),
        );
    };
    if let Some(found) = tags.iter().find(|found| found.name == tag) {
        return tagged_tag_source_outcome(request, found);
    }
    failed_outcome(
        request.clone(),
        &format!("GitHub tag not found: {tag}"),
    )
}

fn tagged_tag_source_outcome(
    request: &Step2UpdateCheckRequest,
    tag: &GitHubTag,
) -> Step2UpdateCheckOutcome {
    let repo_name = request
        .repo
        .rsplit('/')
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("source");
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: Some(tag.name.clone()),
        asset_name: Some(format!("{repo_name}-{}-source.zip", tag.name)),
        asset_url: Some(format!(
            "https://github.com/{}/archive/refs/tags/{}.zip",
            request.repo.trim(),
            tag.name.trim()
        )),
        error: None,
        package_kind: Step2PackageKind::SourceSnapshot,
    }
}

fn repo_source_outcome(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    let Some((tag, asset_name, asset_url)) = latest_repo_source_zip(agent, &request.repo) else {
        return failed_outcome(request.clone(), "repo has no default branch source zip");
    };
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        tag: Some(tag),
        asset_name: Some(asset_name),
        asset_url: Some(asset_url),
        error: None,
        package_kind: Step2PackageKind::SourceSnapshot,
    }
}

fn fetch_latest_proper_release(
    agent: &ureq::Agent,
    repo: &str,
) -> Result<Option<GitHubRelease>, String> {
    super::app_step2_update_github_http::github_api_get_json_optional(
        agent,
        &format!(
            "https://api.github.com/repos/{}/releases/latest",
            repo.trim()
        ),
    )
}

fn fetch_github_releases(agent: &ureq::Agent, repo: &str) -> Result<Vec<GitHubRelease>, String> {
    super::app_step2_update_github_http::github_api_get_json(
        agent,
        &format!("https://api.github.com/repos/{}/releases", repo.trim()),
    )
}

fn fetch_github_tags(agent: &ureq::Agent, repo: &str) -> Result<Vec<GitHubTag>, String> {
    super::app_step2_update_github_http::github_api_get_json(
        agent,
        &format!("https://api.github.com/repos/{}/tags", repo.trim()),
    )
}

fn fetch_github_repo(agent: &ureq::Agent, repo: &str) -> Result<GitHubRepo, String> {
    super::app_step2_update_github_http::github_api_get_json(
        agent,
        &format!("https://api.github.com/repos/{}", repo.trim()),
    )
}

fn fetch_github_branch_head(
    agent: &ureq::Agent,
    repo: &str,
    branch: &str,
) -> Result<GitHubCommitRef, String> {
    super::app_step2_update_github_http::github_api_get_json(
        agent,
        &format!(
            "https://api.github.com/repos/{}/commits/{}",
            repo.trim(),
            branch.trim()
        ),
    )
}

fn fetch_github_tree(
    agent: &ureq::Agent,
    repo: &str,
    tree_sha: &str,
) -> Result<GitHubTreeResponse, String> {
    super::app_step2_update_github_http::github_api_get_json(
        agent,
        &format!(
            "https://api.github.com/repos/{}/git/trees/{}?recursive=1",
            repo.trim(),
            tree_sha.trim()
        ),
    )
}

fn repo_source_declares_requested_version(
    agent: &ureq::Agent,
    request: &Step2UpdateCheckRequest,
    requested_normalized: &str,
) -> bool {
    let Ok(repo) = fetch_github_repo(agent, &request.repo) else {
        return false;
    };
    let branch = if repo.default_branch.trim().is_empty() {
        "main".to_string()
    } else {
        repo.default_branch.clone()
    };
    let Ok(head) = fetch_github_branch_head(agent, repo.full_name.trim(), &branch) else {
        return false;
    };
    let Ok(tree) = fetch_github_tree(agent, repo.full_name.trim(), &head.commit.tree.sha) else {
        return false;
    };
    let tp2_paths = find_repo_tp2_paths(&tree.tree, &request.tp_file);
    if tp2_paths.is_empty() {
        return false;
    }
    tp2_paths.into_iter().any(|tp2_path| {
        let Ok(tp2_text) = fetch_github_raw_text(agent, repo.full_name.trim(), &branch, &tp2_path)
        else {
            return false;
        };
        let Some(version) = parse_tp2_declared_version(&tp2_text) else {
            return false;
        };
        normalize_version_text(&version) == requested_normalized
    })
}

fn find_repo_tp2_paths(entries: &[GitHubTreeEntry], tp_file: &str) -> Vec<String> {
    let expected = mod_downloads::normalize_mod_download_tp2(tp_file);
    entries
        .iter()
        .filter(|entry| entry.kind == "blob")
        .filter_map(|entry| {
            entry
                .path
                .to_ascii_lowercase()
                .ends_with(".tp2")
                .then_some(entry.path.as_str())
        })
        .filter(|path| mod_downloads::normalize_mod_download_tp2(path) == expected)
        .map(ToString::to_string)
        .collect()
}

fn fetch_github_raw_text(
    agent: &ureq::Agent,
    repo: &str,
    branch: &str,
    path: &str,
) -> Result<String, String> {
    let response = agent
        .get(&format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            repo.trim(),
            branch.trim(),
            path
        ))
        .set("User-Agent", "BIO-update-check")
        .call()
        .map_err(|err| err.to_string())?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| err.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_tp2_declared_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("version"))
        {
            continue;
        }
        let value = trimmed[7..]
            .trim()
            .trim_matches(|ch: char| ch == '~' || ch == '"' || ch == '\u{27}')
            .trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn request_for_repo(request: &Step2UpdateCheckRequest, repo: &str) -> Step2UpdateCheckRequest {
    let mut cloned = request.clone();
    cloned.repo = repo.trim().to_string();
    cloned.exact_github.clear();
    cloned
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    prerelease: bool,
    zipball_url: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRepo {
    full_name: String,
    name: String,
    default_branch: String,
}

#[derive(Debug, Deserialize)]
struct GitHubCommitRef {
    sha: String,
    commit: GitHubCommit,
}

#[derive(Debug, Deserialize)]
struct GitHubCommit {
    tree: GitHubTreeRef,
}

#[derive(Debug, Deserialize)]
struct GitHubTreeRef {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GitHubTreeResponse {
    tree: Vec<GitHubTreeEntry>,
}

#[derive(Debug, Deserialize)]
struct GitHubTreeEntry {
    path: String,
    #[serde(rename = "type")]
    kind: String,
}
