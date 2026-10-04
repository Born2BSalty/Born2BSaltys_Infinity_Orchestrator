// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, SystemTime};

use super::manual_archive_probe::{ArchiveProbe, is_in_progress_name, probe_archive};

pub enum WatchEvent {
    Candidate(ArchiveProbe),
    Error(String),
    Tick,
}

#[must_use]
pub fn start_watch<S: std::hash::BuildHasher + Send + 'static>(
    archive_dir: PathBuf,
    wanted_sizes: HashSet<u64, S>,
    ignored_names: HashSet<String, S>,
    poll: Duration,
) -> (Receiver<WatchEvent>, Arc<()>) {
    spawn_watch_thread(archive_dir, wanted_sizes, ignored_names, poll)
}

pub(crate) fn spawn_watch_thread<S: std::hash::BuildHasher + Send + 'static>(
    archive_dir: PathBuf,
    wanted_sizes: HashSet<u64, S>,
    ignored_names: HashSet<String, S>,
    poll: Duration,
) -> (Receiver<WatchEvent>, Arc<()>) {
    let (tx, rx) = mpsc::channel();
    let alive = Arc::new(());
    let alive_thread = Arc::clone(&alive);
    thread::spawn(move || {
        let _ = &alive_thread;
        let _ = fs::create_dir_all(&archive_dir);
        let mut last_seen: HashMap<PathBuf, (u64, SystemTime)> = HashMap::new();
        let mut reported: HashSet<(PathBuf, u64, SystemTime)> = HashSet::new();
        loop {
            let mut current: HashMap<PathBuf, (u64, SystemTime)> = HashMap::new();
            if let Ok(read_dir) = fs::read_dir(&archive_dir) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    let Ok(metadata) = entry.metadata() else {
                        continue;
                    };
                    if !metadata.is_file() {
                        continue;
                    }
                    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                        continue;
                    };
                    if is_in_progress_name(name)
                        || name.starts_with(".bio-")
                        || ignored_names.contains(&name.to_lowercase())
                    {
                        continue;
                    }
                    let size = metadata.len();
                    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    current.insert(path.clone(), (size, modified));
                    let observed = (path.clone(), size, modified);
                    if reported.contains(&observed) {
                        continue;
                    }
                    if last_seen.get(&path) == Some(&(size, modified)) {
                        reported.insert(observed);
                        let event = match probe_archive(&path, &wanted_sizes) {
                            Ok(probe) => WatchEvent::Candidate(probe),
                            Err(err) => WatchEvent::Error(format!("{name}: {err}")),
                        };
                        if tx.send(event).is_err() {
                            return;
                        }
                    }
                }
            }
            last_seen = current;
            if tx.send(WatchEvent::Tick).is_err() {
                return;
            }
            thread::sleep(poll);
        }
    });
    (rx, alive)
}

#[cfg(test)]
pub(crate) fn wait_for_thread_exit(alive: &Arc<()>) {
    for _ in 0..150 {
        if Arc::strong_count(alive) == 1 {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "bio_manualdl_{}_{}_watcher",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn drain_candidates(rx: &Receiver<WatchEvent>, timeout: Duration) -> Vec<PathBuf> {
        let deadline = std::time::Instant::now() + timeout;
        let mut found = Vec::new();
        while std::time::Instant::now() < deadline {
            if let Ok(WatchEvent::Candidate(probe)) = rx.recv_timeout(Duration::from_millis(20)) {
                found.push(probe.path);
            }
        }
        found
    }

    fn drain_errors(rx: &Receiver<WatchEvent>, timeout: Duration) -> Vec<String> {
        let deadline = std::time::Instant::now() + timeout;
        let mut found = Vec::new();
        while std::time::Instant::now() < deadline {
            if let Ok(WatchEvent::Error(message)) = rx.recv_timeout(Duration::from_millis(20)) {
                found.push(message);
            }
        }
        found
    }

    fn wait_for_thread_exit(alive: &Arc<()>) {
        for _ in 0..50 {
            if Arc::strong_count(alive) == 1 {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("watcher thread did not exit in time");
    }

    #[test]
    fn watcher_reports_stable_file_once() {
        let root = TempRoot::new();
        let path = root.path.join("Ready.dat");
        fs::write(&path, b"stable content").unwrap();
        let (rx, alive) = spawn_watch_thread(
            root.path.clone(),
            HashSet::new(),
            HashSet::new(),
            Duration::from_millis(30),
        );
        let first_pass = drain_candidates(&rx, Duration::from_secs(1));
        assert_eq!(first_pass.len(), 1, "reported exactly once within 1s");
        let second_pass = drain_candidates(&rx, Duration::from_millis(200));
        assert!(
            second_pass.is_empty(),
            "not reported again in the following 200ms"
        );
        drop(rx);
        wait_for_thread_exit(&alive);
    }

    #[test]
    fn watcher_ignores_growing_file() {
        let root = TempRoot::new();
        let path = root.path.join("Growing.dat");
        fs::write(&path, b"a").unwrap();
        let (rx, alive) = spawn_watch_thread(
            root.path.clone(),
            HashSet::new(),
            HashSet::new(),
            Duration::from_millis(30),
        );
        let deadline = std::time::Instant::now() + Duration::from_millis(150);
        let mut candidates_while_growing = Vec::new();
        let mut payload = Vec::new();
        while std::time::Instant::now() < deadline {
            payload.push(b'x');
            fs::write(&path, &payload).unwrap();
            std::thread::sleep(Duration::from_millis(5));
            if let Ok(WatchEvent::Candidate(probe)) = rx.recv_timeout(Duration::from_millis(10)) {
                candidates_while_growing.push(probe.path);
            }
        }
        assert!(
            candidates_while_growing.is_empty(),
            "a still-growing file must not be reported"
        );
        let settled = drain_candidates(&rx, Duration::from_millis(300));
        assert_eq!(settled.len(), 1, "reported once the writes stop");
        drop(rx);
        wait_for_thread_exit(&alive);
    }

    #[test]
    fn watcher_reports_probe_errors_once() {
        let root = TempRoot::new();
        let path = root.path.join("Corrupt.zip");
        fs::write(&path, b"not actually a zip file").unwrap();
        let (rx, alive) = spawn_watch_thread(
            root.path.clone(),
            HashSet::new(),
            HashSet::new(),
            Duration::from_millis(30),
        );
        let errors = drain_errors(&rx, Duration::from_secs(1));
        assert_eq!(errors.len(), 1, "reported exactly once within 1s");
        assert!(
            errors[0].starts_with("Corrupt.zip: "),
            "error message names the file: {}",
            errors[0]
        );
        let more_errors = drain_errors(&rx, Duration::from_millis(200));
        assert_eq!(more_errors.len(), 0, "not reported again");
        drop(rx);
        wait_for_thread_exit(&alive);
    }

    #[test]
    fn watcher_stops_when_receiver_dropped() {
        let root = TempRoot::new();
        let (rx, alive) = spawn_watch_thread(
            root.path.clone(),
            HashSet::new(),
            HashSet::new(),
            Duration::from_millis(20),
        );
        drop(rx);
        wait_for_thread_exit(&alive);
    }

    #[test]
    fn watcher_skips_ignored_and_bookkeeping_names() {
        let root = TempRoot::new();
        fs::write(root.path.join("own.zip"), b"already staged").unwrap();
        fs::write(root.path.join(".bio-archive-hashcache.json"), b"{}").unwrap();
        fs::write(root.path.join("other.dat"), b"landed").unwrap();
        let mut ignored_names: HashSet<String> = HashSet::new();
        ignored_names.insert("own.zip".to_string());
        let (rx, alive) = spawn_watch_thread(
            root.path.clone(),
            HashSet::new(),
            ignored_names,
            Duration::from_millis(30),
        );
        let found = drain_candidates(&rx, Duration::from_secs(1));
        assert_eq!(
            found,
            vec![root.path.join("other.dat")],
            "only the file that is neither ignored nor BIO bookkeeping is reported"
        );
        drop(rx);
        wait_for_thread_exit(&alive);
    }
}
