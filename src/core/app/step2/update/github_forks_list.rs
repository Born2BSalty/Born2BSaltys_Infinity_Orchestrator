// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::app::state::{Step2DiscoveredFork, VersionsSheet, WizardState};

type ForksResult = Result<Vec<Step2DiscoveredFork>, String>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ForksStatus {
    #[default]
    Idle,
    Loading,
    Ready(Vec<Step2DiscoveredFork>),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ForksListState {
    pub(crate) repo: String,
    pub(crate) status: ForksStatus,
}

pub(crate) struct ForksFetch {
    repo: String,
    rx: Receiver<ForksResult>,
}

pub(crate) fn spawn_forks_fetch(repo: &str) -> ForksFetch {
    let (tx, rx) = mpsc::channel::<ForksResult>();
    let owner_repo = repo.to_string();
    thread::spawn(move || {
        let _ = tx.send(super::app_step2_update_github_forks::fetch_github_forks(
            &owner_repo,
        ));
    });
    ForksFetch {
        repo: repo.to_string(),
        rx,
    }
}

pub(crate) fn poll_forks(state: &mut ForksListState, fetch: &mut Option<ForksFetch>) {
    if let Some(in_flight) = fetch.as_ref() {
        let answers_current = in_flight.repo == state.repo && state.status == ForksStatus::Loading;
        match in_flight.rx.try_recv() {
            Ok(result) => {
                if answers_current {
                    state.status = match result {
                        Ok(forks) => ForksStatus::Ready(forks),
                        Err(err) => ForksStatus::Failed(err),
                    };
                }
                *fetch = None;
                return;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                if answers_current {
                    state.status = ForksStatus::Failed("The fork lookup stopped".to_string());
                }
                *fetch = None;
                return;
            }
        }
    }
    if state.status == ForksStatus::Loading
        && fetch
            .as_ref()
            .is_none_or(|in_flight| in_flight.repo != state.repo)
    {
        *fetch = Some(spawn_forks_fetch(&state.repo));
    }
}

pub(crate) fn poll_forks_sheet(state: &mut WizardState, fetch: &mut Option<ForksFetch>) {
    let step2 = &mut state.step2;
    if step2.versions_ui.sheet != Some(VersionsSheet::Forks) {
        *fetch = None;
        if step2.forks_list.status == ForksStatus::Loading {
            step2.forks_list.status = ForksStatus::Idle;
        }
        return;
    }
    poll_forks(&mut step2.forks_list, fetch);
    match std::mem::take(&mut step2.forks_list.status) {
        ForksStatus::Ready(forks) => step2.mod_download_forks = forks,
        ForksStatus::Failed(err) => step2.mod_download_forks_popup_error = Some(err),
        still_waiting => step2.forks_list.status = still_waiting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fork(full_name: &str) -> Step2DiscoveredFork {
        Step2DiscoveredFork {
            full_name: full_name.to_string(),
            html_url: format!("https://github.com/{full_name}"),
            owner_login: full_name.split('/').next().unwrap_or_default().to_string(),
            default_branch: "master".to_string(),
            updated_at: "2026-09-27T00:00:00Z".to_string(),
        }
    }

    fn loading(repo: &str) -> ForksListState {
        ForksListState {
            repo: repo.to_string(),
            status: ForksStatus::Loading,
        }
    }

    fn filled_fetch(repo: &str, result: ForksResult) -> ForksFetch {
        let (tx, rx) = mpsc::channel::<ForksResult>();
        tx.send(result).expect("channel open");
        ForksFetch {
            repo: repo.to_string(),
            rx,
        }
    }

    #[test]
    fn stale_repo_result_is_discarded() {
        let mut state = loading("owner/a");
        let mut fetch = Some(filled_fetch("owner/b", Ok(vec![sample_fork("someone/b")])));

        poll_forks(&mut state, &mut fetch);

        assert_eq!(state.status, ForksStatus::Loading);
        assert_eq!(state.repo, "owner/a");
        assert!(fetch.is_none());
    }

    #[test]
    fn failed_fetch_sets_failed() {
        let mut state = loading("owner/a");
        let mut fetch = Some(filled_fetch("owner/a", Err("rate limited".to_string())));

        poll_forks(&mut state, &mut fetch);

        assert_eq!(
            state.status,
            ForksStatus::Failed("rate limited".to_string())
        );
        assert!(fetch.is_none());
    }

    #[test]
    fn matching_result_fills_the_sheet_and_goes_idle() {
        let mut state = WizardState::default();
        state.step2.versions_ui.sheet = Some(VersionsSheet::Forks);
        state.step2.forks_list = loading("owner/a");
        let mut fetch = Some(filled_fetch("owner/a", Ok(vec![sample_fork("someone/a")])));

        poll_forks_sheet(&mut state, &mut fetch);

        assert_eq!(state.step2.forks_list.status, ForksStatus::Idle);
        assert_eq!(state.step2.mod_download_forks.len(), 1);
        assert_eq!(state.step2.mod_download_forks[0].full_name, "someone/a");
        assert!(fetch.is_none());
    }

    #[test]
    fn closed_sheet_drops_the_lookup() {
        let mut state = WizardState::default();
        state.step2.forks_list = loading("owner/a");
        let mut fetch = Some(filled_fetch("owner/a", Ok(vec![sample_fork("someone/a")])));

        poll_forks_sheet(&mut state, &mut fetch);

        assert_eq!(state.step2.forks_list.status, ForksStatus::Idle);
        assert_eq!(state.step2.mod_download_forks.len(), 0);
        assert!(fetch.is_none());
    }
}
