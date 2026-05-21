// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::install::state_install::DestChoice;

const BACKUP_PREFIX: &str = "_bio_backup";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestinationPrepReport {
    Skipped { reason: SkipReason },

    Cleaned { children_removed: usize },

    BackedUp { backup_path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    AlreadyEmpty,
    Continue,
    DoesNotExist,
}

pub fn prepare_destination(
    dest: &Path,
    choice: Option<DestChoice>,
) -> io::Result<DestinationPrepReport> {
    let trimmed = dest.as_os_str();
    if trimmed.is_empty() {
        return Ok(DestinationPrepReport::Skipped {
            reason: SkipReason::DoesNotExist,
        });
    }

    let Some(choice) = choice else {
        return Ok(DestinationPrepReport::Skipped {
            reason: SkipReason::Continue,
        });
    };

    if matches!(choice, DestChoice::Continue) {
        return Ok(DestinationPrepReport::Skipped {
            reason: SkipReason::Continue,
        });
    }

    if !dest.exists() {
        return Ok(DestinationPrepReport::Skipped {
            reason: SkipReason::DoesNotExist,
        });
    }
    if !dest.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("destination prep: {} is not a directory", dest.display()),
        ));
    }

    let mut entries_iter = fs::read_dir(dest)?;
    if entries_iter.next().is_none() {
        return Ok(DestinationPrepReport::Skipped {
            reason: SkipReason::AlreadyEmpty,
        });
    }

    match choice {
        DestChoice::Clear => {
            let removed = count_children(dest)?;
            fs::remove_dir_all(dest)?;
            fs::create_dir_all(dest)?;
            Ok(DestinationPrepReport::Cleaned {
                children_removed: removed,
            })
        }
        DestChoice::Backup => {
            let backup_path = backup_target_path(dest);
            fs::rename(dest, &backup_path)?;
            fs::create_dir_all(dest)?;
            Ok(DestinationPrepReport::BackedUp { backup_path })
        }
        DestChoice::Continue => unreachable!("Continue handled above"),
    }
}

fn count_children(dir: &Path) -> io::Result<usize> {
    let mut count = 0usize;
    for entry in fs::read_dir(dir)? {
        let _ = entry?;
        count += 1;
    }
    Ok(count)
}

fn backup_target_path(dest: &Path) -> PathBuf {
    let parent = dest
        .parent()
        .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf);
    let name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("target");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    parent.join(format!("{BACKUP_PREFIX}_{name}_{ts}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn td(label: &str) -> PathBuf {
        static C: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "bio_dest_prep_test_{}_{}_{label}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    fn make_populated_dir(path: &Path) {
        fs::create_dir_all(path).unwrap();
        fs::write(path.join("a.txt"), b"a").unwrap();
        fs::create_dir_all(path.join("sub")).unwrap();
        fs::write(path.join("sub").join("b.txt"), b"b").unwrap();
    }

    #[test]
    fn clear_empties_the_directory() {
        let dest = td("clear_populated");
        make_populated_dir(&dest);
        let report = prepare_destination(&dest, Some(DestChoice::Clear)).expect("ok");
        match report {
            DestinationPrepReport::Cleaned { children_removed } => {
                assert!(
                    children_removed > 0,
                    "Cleaned report must record the children-removed count"
                );
            }
            other => panic!("expected Cleaned, got {other:?}"),
        }
        assert!(dest.exists(), "the destination dir is recreated empty");
        let n = fs::read_dir(&dest).unwrap().count();
        assert_eq!(n, 0, "Clear must leave the dir with zero children");
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn backup_renames_then_recreates() {
        let dest = td("backup_populated");
        make_populated_dir(&dest);
        let report = prepare_destination(&dest, Some(DestChoice::Backup)).expect("ok");
        let backup = match report {
            DestinationPrepReport::BackedUp { backup_path } => backup_path,
            other => panic!("expected BackedUp, got {other:?}"),
        };
        assert!(backup.exists(), "the renamed directory still exists");
        let leaf = backup
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        assert!(
            leaf.starts_with(BACKUP_PREFIX),
            "backup name follows the _bio_backup_<name>_<ts> shape; got {leaf}"
        );
        assert!(dest.exists(), "the destination dir is recreated empty");
        assert_eq!(fs::read_dir(&dest).unwrap().count(), 0);
        assert!(
            backup.join("a.txt").exists(),
            "the backed-up contents survive"
        );
        let _ = fs::remove_dir_all(&backup);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn continue_choice_is_a_noop_even_on_populated_dir() {
        let dest = td("continue_noop");
        make_populated_dir(&dest);
        let report = prepare_destination(&dest, Some(DestChoice::Continue)).expect("ok");
        assert!(matches!(
            report,
            DestinationPrepReport::Skipped {
                reason: SkipReason::Continue
            }
        ));
        assert!(
            dest.join("a.txt").exists(),
            "Continue MUST leave files intact"
        );
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn no_choice_is_a_noop() {
        let dest = td("none_noop");
        make_populated_dir(&dest);
        let report = prepare_destination(&dest, None).expect("ok");
        assert!(matches!(
            report,
            DestinationPrepReport::Skipped {
                reason: SkipReason::Continue
            }
        ));
        assert!(dest.join("a.txt").exists());
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn empty_destination_is_a_noop_for_clear_and_backup() {
        let dest = td("empty_noop");
        fs::create_dir_all(&dest).unwrap();
        for choice in [DestChoice::Clear, DestChoice::Backup] {
            let report = prepare_destination(&dest, Some(choice)).expect("ok");
            assert!(matches!(
                report,
                DestinationPrepReport::Skipped {
                    reason: SkipReason::AlreadyEmpty
                }
            ));
            assert!(dest.exists());
            assert_eq!(fs::read_dir(&dest).unwrap().count(), 0);
        }
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn missing_destination_is_a_noop_for_clear_and_backup() {
        let dest = td("missing_noop");
        assert!(!dest.exists(), "precondition: dir does not exist");
        for choice in [DestChoice::Clear, DestChoice::Backup] {
            let report = prepare_destination(&dest, Some(choice)).expect("ok");
            assert!(matches!(
                report,
                DestinationPrepReport::Skipped {
                    reason: SkipReason::DoesNotExist
                }
            ));
            assert!(!dest.exists(), "the dir is not created on the no-op path");
        }
    }

    #[test]
    fn nonexistent_dest_short_circuits_to_skipped() {
        let bogus_dest = td("bogus_parent").join("missing_intermediate").join("dest");
        for choice in [DestChoice::Clear, DestChoice::Backup] {
            let r = prepare_destination(&bogus_dest, Some(choice));
            assert!(
                matches!(
                    r,
                    Ok(DestinationPrepReport::Skipped {
                        reason: SkipReason::DoesNotExist
                    })
                ),
                "a non-existent dest short-circuits as a Skipped no-op \
                 for choice {choice:?}"
            );
        }
    }

    #[test]
    fn not_a_directory_returns_error() {
        let path = td("not_a_dir");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"file").unwrap();
        let r = prepare_destination(&path, Some(DestChoice::Clear));
        assert!(r.is_err(), "a file path is not a directory");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn clear_does_not_route_through_recycle_bin() {
        let dest = td("no_recycle_bin");
        make_populated_dir(&dest);
        let canary = dest.join("canary.bin");
        fs::write(&canary, b"canary").unwrap();
        let r = prepare_destination(&dest, Some(DestChoice::Clear)).expect("ok");
        assert!(matches!(r, DestinationPrepReport::Cleaned { .. }));
        assert!(
            !canary.exists(),
            "the canary file is permanently gone (no Recycle Bin restore)"
        );
        let _ = fs::remove_dir_all(&dest);
    }
}
