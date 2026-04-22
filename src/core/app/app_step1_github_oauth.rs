// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::app::state::WizardState;

const GITHUB_OAUTH_CLIENT_ID: &str = "Ov23liVv971wvRTUbkrW";
const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Debug, Clone)]
pub(crate) struct GitHubOAuthPrompt {
    pub(crate) user_code: String,
    pub(crate) verification_uri: String,
    pub(crate) status_text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct GitHubOAuthValidatedToken {
    pub(crate) token: String,
    pub(crate) login: String,
}

pub(crate) type GitHubOAuthFlowResult = Result<GitHubOAuthValidatedToken, String>;

#[derive(Debug, Deserialize)]
struct GitHubDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct GitHubAccessTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubViewerResponse {
    login: String,
}

enum GitHubAccessTokenPollOutcome {
    Success(String),
    Pending,
    SlowDown,
    Failed(String),
}

pub(crate) fn start_github_oauth_device_flow()
-> Result<(GitHubOAuthPrompt, Receiver<GitHubOAuthFlowResult>), String> {
    let response = request_device_code()?;

    let (tx, rx) = mpsc::channel::<GitHubOAuthFlowResult>();
    let device_code = response.device_code.clone();
    let interval_secs = response.interval.max(1);
    let expires_in_secs = response.expires_in.max(interval_secs);
    thread::spawn(move || {
        poll_for_access_token(tx, device_code, interval_secs, expires_in_secs);
    });

    Ok((
        GitHubOAuthPrompt {
            user_code: response.user_code,
            verification_uri: response.verification_uri,
            status_text: "Open GitHub from this popup, then enter the code to continue."
                .to_string(),
        },
        rx,
    ))
}

pub(crate) fn load_github_login_from_stored_token() -> Result<Option<String>, String> {
    let Some(token) = super::app_step2_update_github_auth::load_github_token() else {
        if let Some(err) = super::app_step2_update_github_auth::take_last_load_error() {
            return Err(format!("github auth load failed: {err}"));
        }
        return Ok(None);
    };
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .build();
    fetch_github_login_for_token(&agent, &token).map(Some)
}

pub(crate) fn poll_github_oauth_flow(
    state: &mut WizardState,
    github_auth_rx: &mut Option<Receiver<GitHubOAuthFlowResult>>,
) {
    let Some(rx) = github_auth_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => Some(result),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            state.github_auth_running = false;
            state.github_auth_popup_open = true;
            state.github_auth_user_code.clear();
            state.github_auth_verification_uri.clear();
            state.github_auth_status_text =
                "GitHub authorization failed: worker disconnected".to_string();
            state.step2.scan_status = state.github_auth_status_text.clone();
            *github_auth_rx = None;
            return;
        }
    };
    let Some(result) = result else {
        return;
    };

    *github_auth_rx = None;
    state.github_auth_running = false;
    state.github_auth_popup_open = true;
    state.github_auth_user_code.clear();
    state.github_auth_verification_uri.clear();
    match result {
        Ok(validated) => {
            match super::app_step2_update_github_auth::store_github_oauth_token(&validated.token) {
                Ok(()) => {
                    state.github_auth_login = validated.login.clone();
                    state.github_auth_status_text.clear();
                    state.step2.scan_status = format!("Connected as {}.", validated.login);
                }
                Err(err) => {
                    state.github_auth_status_text =
                        format!("GitHub connected, but secure token storage failed: {err}");
                    state.step2.scan_status = state.github_auth_status_text.clone();
                }
            }
        }
        Err(err) => {
            state.github_auth_status_text = err.clone();
            state.step2.scan_status = err;
        }
    }
}

fn request_device_code() -> Result<GitHubDeviceCodeResponse, String> {
    let response = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .build()
        .post(GITHUB_DEVICE_CODE_URL)
        .set("User-Agent", "BIO-github-auth")
        .set("Accept", "application/json")
        .send_form(&[("client_id", GITHUB_OAUTH_CLIENT_ID)])
        .map_err(|err| err.to_string())?;
    let text = response.into_string().map_err(|err| err.to_string())?;
    serde_json::from_str::<GitHubDeviceCodeResponse>(&text).map_err(|err| err.to_string())
}

fn poll_for_access_token(
    tx: mpsc::Sender<GitHubOAuthFlowResult>,
    device_code: String,
    interval_secs: u64,
    expires_in_secs: u64,
) {
    let started = Instant::now();
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .build();
    let mut interval = interval_secs.max(1);
    loop {
        if started.elapsed() >= Duration::from_secs(expires_in_secs) {
            let _ = tx.send(Err(
                "GitHub authorization expired. Start Connect GitHub again.".to_string(),
            ));
            return;
        }
        thread::sleep(Duration::from_secs(interval));
        match poll_access_token_once(&agent, &device_code) {
            Ok(GitHubAccessTokenPollOutcome::Success(token)) => {
                let _ = tx.send(
                    fetch_github_login_for_token(&agent, &token)
                        .map(|login| GitHubOAuthValidatedToken { token, login }),
                );
                return;
            }
            Ok(GitHubAccessTokenPollOutcome::Pending) => {}
            Ok(GitHubAccessTokenPollOutcome::SlowDown) => {
                interval = interval.saturating_add(5);
            }
            Ok(GitHubAccessTokenPollOutcome::Failed(err)) => {
                let _ = tx.send(Err(err));
                return;
            }
            Err(err) => {
                let _ = tx.send(Err(err));
                return;
            }
        }
    }
}

fn poll_access_token_once(
    agent: &ureq::Agent,
    device_code: &str,
) -> Result<GitHubAccessTokenPollOutcome, String> {
    let response = agent
        .post(GITHUB_ACCESS_TOKEN_URL)
        .set("User-Agent", "BIO-github-auth")
        .set("Accept", "application/json")
        .send_form(&[
            ("client_id", GITHUB_OAUTH_CLIENT_ID),
            ("device_code", device_code),
            ("grant_type", GITHUB_DEVICE_GRANT_TYPE),
        ])
        .map_err(|err| err.to_string())?;
    let text = response.into_string().map_err(|err| err.to_string())?;
    let parsed =
        serde_json::from_str::<GitHubAccessTokenResponse>(&text).map_err(|err| err.to_string())?;
    if let Some(token) = parsed.access_token {
        return Ok(GitHubAccessTokenPollOutcome::Success(token));
    }
    match parsed.error.as_deref() {
        Some("authorization_pending") => Ok(GitHubAccessTokenPollOutcome::Pending),
        Some("slow_down") => Ok(GitHubAccessTokenPollOutcome::SlowDown),
        Some("access_denied") => Ok(GitHubAccessTokenPollOutcome::Failed(
            "GitHub authorization was denied.".to_string(),
        )),
        Some("expired_token") => Ok(GitHubAccessTokenPollOutcome::Failed(
            "GitHub authorization expired. Start Connect GitHub again.".to_string(),
        )),
        Some(error) => Ok(GitHubAccessTokenPollOutcome::Failed(
            parsed.error_description.map_or_else(
                || format!("GitHub authorization failed: {error}"),
                |description| format!("GitHub authorization failed: {description}"),
            ),
        )),
        None => Ok(GitHubAccessTokenPollOutcome::Failed(
            "GitHub authorization failed: missing access token".to_string(),
        )),
    }
}

fn fetch_github_login_for_token(agent: &ureq::Agent, token: &str) -> Result<String, String> {
    let response = agent
        .get("https://api.github.com/user")
        .set("User-Agent", "BIO-github-auth")
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| err.to_string())?;
    let text = response.into_string().map_err(|err| err.to_string())?;
    let viewer =
        serde_json::from_str::<GitHubViewerResponse>(&text).map_err(|err| err.to_string())?;
    let login = viewer.login.trim();
    if login.is_empty() {
        Err("GitHub authorization failed: authenticated user login missing".to_string())
    } else {
        Ok(login.to_string())
    }
}
