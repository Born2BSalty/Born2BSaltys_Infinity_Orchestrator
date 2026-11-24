// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::app::compat_rules::COMPAT_RULES_USER_FILE_NAME;
use crate::app::mod_downloads::MOD_DOWNLOADS_USER_FILE_NAME;
use crate::app::mod_downloads_migrate::{
    COMPAT_RULES_DEFAULT_FILE_NAME, MOD_DOWNLOADS_DEFAULT_FILE_NAME,
};
use crate::platform_defaults::{CONFIG_USER_FOLDER, app_config_dir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigRelocation {
    Moved { from: PathBuf, to: PathBuf },
    LeftInPlace { stale: PathBuf, kept: PathBuf },
    RemovedFlatReference(PathBuf),
    SkippedNotAFile(PathBuf),
    Failed { path: PathBuf, error: String },
}

const FLAT_REFERENCE_FILE_NAMES: [&str; 2] = [
    MOD_DOWNLOADS_DEFAULT_FILE_NAME,
    COMPAT_RULES_DEFAULT_FILE_NAME,
];

fn mod_downloads_backup_name() -> String {
    format!("{MOD_DOWNLOADS_USER_FILE_NAME}.v1.bak")
}

fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_file())
}

fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    if let Err(err) = fs::copy(from, to) {
        let _ = fs::remove_file(to);
        return Err(err);
    }
    fs::remove_file(from)
}

fn relocate_user_file(config_dir: &Path, name: &str) -> Option<ConfigRelocation> {
    let from = config_dir.join(name);
    if fs::symlink_metadata(&from).is_err() {
        return None;
    }
    if !is_regular_file(&from) {
        return Some(ConfigRelocation::SkippedNotAFile(from));
    }
    let to = config_dir.join(CONFIG_USER_FOLDER).join(name);
    if to.exists() {
        return Some(ConfigRelocation::LeftInPlace {
            stale: from,
            kept: to,
        });
    }
    Some(match move_file(&from, &to) {
        Ok(()) => ConfigRelocation::Moved { from, to },
        Err(err) => ConfigRelocation::Failed {
            path: from,
            error: err.to_string(),
        },
    })
}

fn remove_flat_reference(config_dir: &Path, name: &str) -> Option<ConfigRelocation> {
    let path = config_dir.join(name);
    if !is_regular_file(&path) {
        return None;
    }
    Some(match fs::remove_file(&path) {
        Ok(()) => ConfigRelocation::RemovedFlatReference(path),
        Err(err) => ConfigRelocation::Failed {
            path,
            error: err.to_string(),
        },
    })
}

#[must_use]
pub fn relocate_config_files(config_dir: &Path) -> Vec<ConfigRelocation> {
    let main = relocate_user_file(config_dir, MOD_DOWNLOADS_USER_FILE_NAME);
    let backup_follows_main = match &main {
        Some(ConfigRelocation::Moved { .. }) => true,
        None => !config_dir
            .join(CONFIG_USER_FOLDER)
            .join(MOD_DOWNLOADS_USER_FILE_NAME)
            .exists(),
        Some(_) => false,
    };
    let backup = backup_follows_main
        .then(|| relocate_user_file(config_dir, &mod_downloads_backup_name()))
        .flatten();
    let mut outcomes = [
        main,
        backup,
        relocate_user_file(config_dir, COMPAT_RULES_USER_FILE_NAME),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    outcomes.extend(
        FLAT_REFERENCE_FILE_NAMES
            .iter()
            .filter_map(|name| remove_flat_reference(config_dir, name)),
    );
    outcomes
}

fn log_relocation(outcome: &ConfigRelocation) {
    match outcome {
        ConfigRelocation::Moved { from, to } => info!(
            target = "orchestrator",
            "moved {} to {}",
            from.display(),
            to.display()
        ),
        ConfigRelocation::LeftInPlace { stale, kept } => warn!(
            target = "orchestrator",
            "{} already exists, so the older {} was left where it is",
            kept.display(),
            stale.display()
        ),
        ConfigRelocation::RemovedFlatReference(path) => info!(
            target = "orchestrator",
            "removed the old reference copy {}",
            path.display()
        ),
        ConfigRelocation::SkippedNotAFile(path) => warn!(
            target = "orchestrator",
            "{} is not a regular file, so it was not moved into the User folder and BIO no longer reads it",
            path.display()
        ),
        ConfigRelocation::Failed { path, error } => warn!(
            target = "orchestrator",
            "could not move {}: {error}",
            path.display()
        ),
    }
}

pub fn relocate_config_files_at_launch() {
    let Some(config_dir) = app_config_dir() else {
        return;
    };
    for outcome in relocate_config_files(&config_dir) {
        log_relocation(&outcome);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_defaults::CONFIG_DEFAULT_FOLDER;

    struct RelocateTestRoot(PathBuf);

    impl RelocateTestRoot {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_config_folders_test_{}_{id}_{label}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(path.clone()));
            Self(path)
        }
    }

    impl Drop for RelocateTestRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn user_files_resolve_under_the_user_folder() {
        let root = RelocateTestRoot::new("user_paths");
        let user_dir = root.0.join(CONFIG_USER_FOLDER);
        assert_eq!(
            crate::app::mod_downloads::mod_downloads_user_path(),
            user_dir.join("mod_downloads_user.toml")
        );
        assert_eq!(
            crate::app::compat_rules::compat_rules_user_path(),
            user_dir.join("step2_compat_rules_user.toml")
        );
    }

    #[test]
    fn reference_files_are_written_under_the_default_folder() {
        let root = RelocateTestRoot::new("default_paths");
        crate::app::mod_downloads_migrate::migrate_source_files_at_launch();
        let default_dir = root.0.join(CONFIG_DEFAULT_FOLDER);
        assert!(default_dir.join("mod_downloads_default.toml").is_file());
        assert!(
            default_dir
                .join("step2_compat_rules_default.toml")
                .is_file()
        );
        assert!(!root.0.join("mod_downloads_default.toml").exists());
        assert!(!root.0.join("step2_compat_rules_default.toml").exists());
    }

    #[test]
    fn flat_user_files_move_into_the_user_folder_with_their_content() {
        let root = RelocateTestRoot::new("move");
        fs::write(
            root.0.join("mod_downloads_user.toml"),
            "format = 2\n# mine\n",
        )
        .unwrap();
        fs::write(root.0.join("mod_downloads_user.toml.v1.bak"), "# backup\n").unwrap();
        fs::write(root.0.join("step2_compat_rules_user.toml"), "# my rules\n").unwrap();

        let outcomes = relocate_config_files(&root.0);

        let user_dir = root.0.join(CONFIG_USER_FOLDER);
        assert_eq!(
            fs::read_to_string(user_dir.join("mod_downloads_user.toml")).unwrap(),
            "format = 2\n# mine\n"
        );
        assert_eq!(
            fs::read_to_string(user_dir.join("mod_downloads_user.toml.v1.bak")).unwrap(),
            "# backup\n"
        );
        assert_eq!(
            fs::read_to_string(user_dir.join("step2_compat_rules_user.toml")).unwrap(),
            "# my rules\n"
        );
        assert!(!root.0.join("mod_downloads_user.toml").exists());
        assert!(!root.0.join("mod_downloads_user.toml.v1.bak").exists());
        assert!(!root.0.join("step2_compat_rules_user.toml").exists());
        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(outcome, ConfigRelocation::Moved { .. }))
        );
    }

    #[test]
    fn moved_user_files_are_what_bio_reads() {
        let root = RelocateTestRoot::new("read_after_move");
        fs::write(
            root.0.join("mod_downloads_user.toml"),
            "format = 2\n# mine\n",
        )
        .unwrap();
        fs::write(root.0.join("step2_compat_rules_user.toml"), "# my rules\n").unwrap();

        relocate_config_files_at_launch();

        assert_eq!(
            fs::read_to_string(crate::app::mod_downloads::mod_downloads_user_path()).unwrap(),
            "format = 2\n# mine\n"
        );
        assert_eq!(
            fs::read_to_string(crate::app::compat_rules::effective_compat_rules_user_path())
                .unwrap(),
            "# my rules\n"
        );
    }

    #[test]
    fn an_existing_user_folder_file_wins_and_the_flat_one_is_left_alone() {
        let root = RelocateTestRoot::new("both");
        let user_dir = root.0.join(CONFIG_USER_FOLDER);
        fs::create_dir_all(&user_dir).unwrap();
        fs::write(user_dir.join("mod_downloads_user.toml"), "# new\n").unwrap();
        fs::write(root.0.join("mod_downloads_user.toml"), "# old\n").unwrap();

        let outcomes = relocate_config_files(&root.0);

        assert_eq!(
            fs::read_to_string(user_dir.join("mod_downloads_user.toml")).unwrap(),
            "# new\n"
        );
        assert_eq!(
            fs::read_to_string(root.0.join("mod_downloads_user.toml")).unwrap(),
            "# old\n"
        );
        assert!(
            outcomes
                .iter()
                .any(|outcome| matches!(outcome, ConfigRelocation::LeftInPlace { .. }))
        );
    }

    #[test]
    fn the_backup_stays_with_a_main_file_that_was_not_moved() {
        let root = RelocateTestRoot::new("backup_stays");
        let user_dir = root.0.join(CONFIG_USER_FOLDER);
        fs::create_dir_all(&user_dir).unwrap();
        fs::write(user_dir.join("mod_downloads_user.toml"), "# new\n").unwrap();
        fs::write(root.0.join("mod_downloads_user.toml"), "# old\n").unwrap();
        fs::write(
            root.0.join("mod_downloads_user.toml.v1.bak"),
            "# old backup\n",
        )
        .unwrap();

        let _ = relocate_config_files(&root.0);

        assert!(root.0.join("mod_downloads_user.toml.v1.bak").is_file());
        assert!(!user_dir.join("mod_downloads_user.toml.v1.bak").exists());
    }

    #[test]
    fn flat_reference_copies_are_removed() {
        let root = RelocateTestRoot::new("flat_reference");
        fs::write(root.0.join("mod_downloads_default.toml"), "# old ref\n").unwrap();
        fs::write(
            root.0.join("step2_compat_rules_default.toml"),
            "# old ref\n",
        )
        .unwrap();

        let _ = relocate_config_files(&root.0);

        assert!(!root.0.join("mod_downloads_default.toml").exists());
        assert!(!root.0.join("step2_compat_rules_default.toml").exists());
    }

    #[test]
    fn nothing_to_move_changes_nothing() {
        let root = RelocateTestRoot::new("empty");
        assert_eq!(relocate_config_files(&root.0), Vec::new());
        assert!(!root.0.join(CONFIG_USER_FOLDER).exists());
    }
}
