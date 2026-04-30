// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use chrono::{DateTime, Local, Utc};

pub(super) fn github_api_get_json<T: serde::de::DeserializeOwned>(
    agent: &ureq::Agent,
    url: &str,
) -> Result<T, String> {
    let response = github_api_request(agent, url)?;
    let text = response.into_string().map_err(|err| err.to_string())?;
    serde_json::from_str::<T>(&text).map_err(|err| err.to_string())
}

pub(super) fn github_api_get_json_optional<T: serde::de::DeserializeOwned>(
    agent: &ureq::Agent,
    url: &str,
) -> Result<Option<T>, String> {
    let response = match github_api_request(agent, url) {
        Ok(response) => response,
        Err(err) if err.contains("status code 404") => return Ok(None),
        Err(err) => return Err(err),
    };
    let text = response.into_string().map_err(|err| err.to_string())?;
    serde_json::from_str::<T>(&text)
        .map(Some)
        .map_err(|err| err.to_string())
}

fn github_api_request(agent: &ureq::Agent, url: &str) -> Result<ureq::Response, String> {
    let mut request = agent
        .get(url)
        .set("User-Agent", "BIO-update-check")
        .set("Accept", "application/vnd.github+json");
    if let Some(token) = super::app_step2_update_github_auth::load_github_token() {
        request = request.set("Authorization", &format!("Bearer {token}"));
    } else if let Some(err) = super::app_step2_update_github_auth::take_last_load_error() {
        return Err(format!("github auth load failed: {err}"));
    }
    request
        .call()
        .map_err(|err| format_github_request_error(url, err))
}

fn format_github_request_error(url: &str, err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, response) => format_github_status_error(url, code, &response),
        other => other.to_string(),
    }
}

fn format_github_status_error(url: &str, code: u16, response: &ureq::Response) -> String {
    if code == 403
        && let Some(reset_text) = github_rate_limit_reset_text(response)
    {
        return format!("{url}: status code 403 (rate limit, resets at {reset_text})");
    }
    format!("{url}: status code {code}")
}

fn github_rate_limit_reset_text(response: &ureq::Response) -> Option<String> {
    let remaining = response.header("x-ratelimit-remaining")?.trim();
    if remaining != "0" {
        return None;
    }
    let reset = response.header("x-ratelimit-reset")?.trim();
    let reset_epoch = reset.parse::<i64>().ok()?;
    let dt_utc: DateTime<Utc> = DateTime::from_timestamp(reset_epoch, 0)?;
    let dt_local = dt_utc.with_timezone(&Local);
    Some(dt_local.format("%Y-%m-%d %H:%M local").to_string())
}
