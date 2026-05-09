// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Duration;

use serde::Deserialize;

use crate::app::state::Step2DiscoveredFork;

#[derive(Debug, Deserialize)]
struct GitHubForkOwner {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubFork {
    full_name: String,
    html_url: String,
    owner: GitHubForkOwner,
    default_branch: String,
    updated_at: String,
}

pub(crate) fn fetch_github_forks(repo: &str) -> Result<Vec<Step2DiscoveredFork>, String> {
    let repo = repo.trim().trim_matches('/');
    if repo.split('/').count() != 2 {
        return Err(format!("GitHub repo must be owner/repo: {repo}"));
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .build();
    let url = format!("https://api.github.com/repos/{repo}/forks?per_page=100");
    let forks =
        super::app_step2_update_github_http::github_api_get_json::<Vec<GitHubFork>>(&agent, &url)?;
    Ok(forks
        .into_iter()
        .map(|fork| Step2DiscoveredFork {
            full_name: fork.full_name,
            html_url: fork.html_url,
            owner_login: fork.owner.login,
            default_branch: fork.default_branch,
            updated_at: fork.updated_at,
        })
        .collect())
}
