// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
    let total = requests.len();
    if total == 0 {
        let _ = tx.send(Step2UpdateCheckEvent::Finished(Vec::new()));
        return rx;
    }

    let shared_requests = Arc::new(Mutex::new(requests.into_iter()));
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
                let outcome =
                    super::app_step2_update_check::check_latest_release_for_worker(&agent, request);
                if let Ok(mut guard) = outcomes.lock() {
                    guard.push(outcome);
                }
                let done = if let Ok(mut guard) = completed.lock() {
                    *guard += 1;
                    *guard
                } else {
                    0
                };
                let _ = tx.send(Step2UpdateCheckEvent::Progress(Step2UpdateCheckProgress {
                    completed: done,
                    total,
                }));
            }
        });
    }

    thread::spawn(move || {
        loop {
            let done = completed.lock().map(|value| *value).unwrap_or(0);
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
