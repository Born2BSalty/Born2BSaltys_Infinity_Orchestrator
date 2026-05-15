// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use super::app_step2_update_check::{Step2UpdateCheckOutcome, Step2UpdateCheckRequest};

pub(super) const STEP2_UPDATE_CHECK_WORKERS: usize = 6;

#[derive(Debug, Clone)]
pub(crate) struct Step2UpdateCheckProgress {
    pub(crate) completed: usize,
    pub(crate) total: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum Step2UpdateCheckEvent {
    Progress(Step2UpdateCheckProgress),
    Finished(Vec<Step2UpdateCheckOutcome>),
}

pub(crate) fn spawn_update_check_worker(
    requests: Vec<Step2UpdateCheckRequest>,
) -> mpsc::Receiver<Step2UpdateCheckEvent> {
    let (tx, rx) = mpsc::channel::<Step2UpdateCheckEvent>();
    let grouped_requests = group_update_check_requests(requests);
    let total = grouped_requests.len();
    if total == 0 {
        let _ = tx.send(Step2UpdateCheckEvent::Finished(Vec::new()));
        return rx;
    }

    let shared_requests = Arc::new(Mutex::new(grouped_requests.into_iter()));
    let outcomes = Arc::new(Mutex::new(Vec::<Step2UpdateCheckOutcome>::new()));
    let completed = Arc::new(Mutex::new(0usize));

    for _ in 0..STEP2_UPDATE_CHECK_WORKERS.min(total) {
        let tx = tx.clone();
        let shared_requests = Arc::clone(&shared_requests);
        let outcomes = Arc::clone(&outcomes);
        let completed = Arc::clone(&completed);
        thread::spawn(move || {
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(10))
                .timeout_read(Duration::from_secs(20))
                .build();
            loop {
                let next = {
                    let mut guard = shared_requests.lock().ok();
                    guard.as_mut().and_then(|iter| iter.next())
                };
                let Some(request) = next else {
                    break;
                };
                let outcome = super::app_step2_update_check::check_latest_release_for_worker(
                    &agent,
                    request.canonical.clone(),
                );
                if let Ok(mut guard) = outcomes.lock() {
                    guard.extend(fan_out_update_check_outcome(&outcome, &request.targets));
                }
                let done = completed.lock().map_or(0, |mut guard| {
                    *guard += 1;
                    *guard
                });
                let _ = tx.send(Step2UpdateCheckEvent::Progress(Step2UpdateCheckProgress {
                    completed: done,
                    total,
                }));
            }
        });
    }

    thread::spawn(move || {
        loop {
            let done = completed.lock().map_or(0, |value| *value);
            if done >= total {
                let final_outcomes = outcomes
                    .lock()
                    .map(|value| value.clone())
                    .unwrap_or_default();
                let _ = tx.send(Step2UpdateCheckEvent::Finished(final_outcomes));
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    rx
}

#[derive(Debug, Clone)]
struct Step2UpdateCheckRequestGroup {
    canonical: Step2UpdateCheckRequest,
    targets: Vec<Step2UpdateCheckRequest>,
}

fn group_update_check_requests(
    requests: Vec<Step2UpdateCheckRequest>,
) -> Vec<Step2UpdateCheckRequestGroup> {
    let mut grouped = BTreeMap::<String, Step2UpdateCheckRequestGroup>::new();
    for request in requests {
        let key = update_check_request_key(&request);
        if let Some(group) = grouped.get_mut(&key) {
            group.targets.push(request);
        } else {
            grouped.insert(
                key,
                Step2UpdateCheckRequestGroup {
                    canonical: request.clone(),
                    targets: vec![request],
                },
            );
        }
    }
    grouped.into_values().collect()
}

fn update_check_request_key(request: &Step2UpdateCheckRequest) -> String {
    let exact_github = if request.exact_github.is_empty() {
        String::new()
    } else {
        request
            .exact_github
            .iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        request.repo.trim().to_ascii_lowercase(),
        request.source_url.trim().to_ascii_lowercase(),
        request
            .channel
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
        request
            .tag
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
        request
            .commit
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
        request
            .branch
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
        request
            .asset
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
        request
            .pkg
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
        request
            .requested_version
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
        exact_github
    )
}

fn fan_out_update_check_outcome(
    outcome: &Step2UpdateCheckOutcome,
    targets: &[Step2UpdateCheckRequest],
) -> Vec<Step2UpdateCheckOutcome> {
    targets
        .iter()
        .map(|target| {
            let mut expanded = outcome.clone();
            expanded.game_tab.clone_from(&target.game_tab);
            expanded.tp_file.clone_from(&target.tp_file);
            expanded.label.clone_from(&target.label);
            expanded.source_id.clone_from(&target.source_id);
            expanded
        })
        .collect()
}
