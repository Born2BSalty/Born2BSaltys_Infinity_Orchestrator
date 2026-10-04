// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceGame {
    Bgee,
    Bg2ee,
    Iwdee,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum SodDlcState {
    #[default]
    NotApplicable,
    Absent,
    Unmerged {
        archive: PathBuf,
    },
    Merged,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ResidueSummary {
    pub(crate) weidu_log: bool,
    pub(crate) weidu_external: bool,
    pub(crate) debug_files: usize,
    pub(crate) setup_executables: usize,
    pub(crate) mod_folders: usize,
    pub(crate) override_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SourceReport {
    pub(crate) sod: SodDlcState,
    pub(crate) residue: ResidueSummary,
    pub(crate) game_version: Option<crate::app::game_version::GameVersion>,
}

const ENGINE_FOLDERS: [&str; 14] = [
    "data",
    "lang",
    "movies",
    "music",
    "scripts",
    "override",
    "dlc",
    "manuals",
    "characters",
    "portraits",
    "save",
    "mpsave",
    "sod-dlc",
    "weidu_external",
];

fn child_named(root: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(root).ok()?.flatten().find_map(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
            .then(|| entry.path())
    })
}

fn find_sod_archive(root: &Path) -> Option<PathBuf> {
    child_named(root, "sod-dlc.zip")
        .filter(|path| path.is_file())
        .or_else(|| {
            child_named(root, "dlc")
                .and_then(|dlc| child_named(&dlc, "sod-dlc.zip"))
                .filter(|path| path.is_file())
        })
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[must_use]
pub(crate) fn sod_state(folder: &Path) -> SodDlcState {
    if let Ok(bytes) = std::fs::read(folder.join("chitin.key")) {
        let lowered: Vec<u8> = bytes.iter().map(u8::to_ascii_lowercase).collect();
        if contains_bytes(&lowered, b"sod-dlc") || contains_bytes(&lowered, b"sodareas") {
            return SodDlcState::Merged;
        }
    }
    find_sod_archive(folder).map_or(SodDlcState::Absent, |archive| SodDlcState::Unmerged {
        archive,
    })
}

fn contains_tp2(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry.file_type().is_ok_and(|file_type| file_type.is_file())
                && entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .ends_with(".tp2")
        })
    })
}

fn residue_summary(folder: &Path) -> ResidueSummary {
    let mut summary = ResidueSummary::default();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return summary;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() {
            if name == "weidu.log" {
                summary.weidu_log = true;
            } else if Path::new(&name)
                .extension()
                .is_some_and(|ext| ext == "debug")
            {
                summary.debug_files += 1;
            } else if name.starts_with("setup-") {
                summary.setup_executables += 1;
            }
        } else if file_type.is_dir() {
            if name == "weidu_external" {
                summary.weidu_external = true;
            }
            if !ENGINE_FOLDERS.contains(&name.as_str()) && contains_tp2(&entry.path()) {
                summary.mod_folders += 1;
            }
        }
    }
    summary.override_files = folder
        .join("override")
        .read_dir()
        .map_or(0, |entries| entries.flatten().count());
    summary
}

#[must_use]
pub(crate) fn inspect(folder: &Path, game: SourceGame) -> SourceReport {
    SourceReport {
        sod: if matches!(game, SourceGame::Bgee) {
            sod_state(folder)
        } else {
            SodDlcState::NotApplicable
        },
        residue: residue_summary(folder),
        game_version: crate::app::game_version::read_game_version(folder),
    }
}

fn group_thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped.chars().rev().collect()
}

impl ResidueSummary {
    #[must_use]
    pub(crate) const fn is_clean(&self) -> bool {
        !self.weidu_log
            && !self.weidu_external
            && self.debug_files == 0
            && self.setup_executables == 0
            && self.mod_folders == 0
            && self.override_files == 0
    }

    #[must_use]
    pub(crate) fn describe(&self) -> String {
        let mut parts = Vec::new();
        if self.weidu_log {
            parts.push("WeiDU.log".to_string());
        }
        if self.weidu_external {
            parts.push("weidu_external".to_string());
        }
        if self.debug_files > 0 {
            parts.push(format!("{} debug file(s)", self.debug_files));
        }
        if self.setup_executables > 0 {
            parts.push(format!("{} setup executable(s)", self.setup_executables));
        }
        if self.mod_folders > 0 {
            parts.push(format!("{} mod folder(s)", self.mod_folders));
        }
        if self.override_files > 0 {
            parts.push(format!(
                "override {} file(s)",
                group_thousands(self.override_files)
            ));
        }
        parts.join(" \u{00B7} ")
    }
}

impl SodDlcState {
    #[must_use]
    pub(crate) const fn describe(&self) -> &'static str {
        match self {
            Self::NotApplicable => "",
            Self::Absent => "SoD not found",
            Self::Unmerged { .. } => "SoD archive present, not merged",
            Self::Merged => "SoD merged",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{SodDlcState, SourceGame, inspect};
    use crate::app::game_version::GameVersion;

    struct TempFixture {
        path: std::path::PathBuf,
    }

    impl TempFixture {
        fn new(name: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_source_check_test_{name}_{}_{id}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn clean_install_reports_clean_and_sod_absent() {
        let fixture = TempFixture::new("clean");
        std::fs::write(fixture.path.join("chitin.key"), b"clean key data").unwrap();
        let report = inspect(&fixture.path, SourceGame::Bgee);
        assert!(report.residue.is_clean());
        assert_eq!(report.sod, SodDlcState::Absent);
    }

    #[test]
    fn unmerged_archive_in_dlc_dir_is_unmerged() {
        let fixture = TempFixture::new("unmerged_dlc_dir");
        std::fs::write(fixture.path.join("chitin.key"), b"clean key data").unwrap();
        let dlc = fixture.path.join("dlc");
        std::fs::create_dir_all(&dlc).unwrap();
        let archive = dlc.join("sod-dlc.zip");
        std::fs::write(&archive, b"zip").unwrap();
        let report = inspect(&fixture.path, SourceGame::Bgee);
        assert_eq!(report.sod, SodDlcState::Unmerged { archive });
    }

    #[test]
    fn key_mentioning_sod_is_merged_even_with_archive_left() {
        let fixture = TempFixture::new("merged_with_archive");
        std::fs::write(fixture.path.join("chitin.key"), b"data/SoDAreas.bif").unwrap();
        let dlc = fixture.path.join("dlc");
        std::fs::create_dir_all(&dlc).unwrap();
        std::fs::write(dlc.join("sod-dlc.zip"), b"zip").unwrap();
        let report = inspect(&fixture.path, SourceGame::Bgee);
        assert_eq!(report.sod, SodDlcState::Merged);
    }

    #[test]
    fn residue_counts_every_marker() {
        let fixture = TempFixture::new("residue");
        let root = &fixture.path;
        std::fs::write(root.join("WeiDU.log"), b"log").unwrap();
        std::fs::create_dir_all(root.join("weidu_external")).unwrap();
        std::fs::write(root.join("a.debug"), b"d").unwrap();
        std::fs::write(root.join("b.debug"), b"d").unwrap();
        std::fs::write(root.join("setup-foo.exe"), b"e").unwrap();
        for name in ["ModA", "ModB"] {
            let mod_dir = root.join(name);
            std::fs::create_dir_all(&mod_dir).unwrap();
            std::fs::write(mod_dir.join("setup.tp2"), b"tp2").unwrap();
        }
        let override_dir = root.join("override");
        std::fs::create_dir_all(&override_dir).unwrap();
        for index in 0..3 {
            std::fs::write(override_dir.join(format!("file{index}.itm")), b"x").unwrap();
        }
        let report = inspect(root, SourceGame::Bg2ee);
        assert!(!report.residue.is_clean());
        assert!(report.residue.weidu_log);
        assert!(report.residue.weidu_external);
        assert_eq!(report.residue.debug_files, 2);
        assert_eq!(report.residue.setup_executables, 1);
        assert_eq!(report.residue.mod_folders, 2);
        assert_eq!(report.residue.override_files, 3);
    }

    #[test]
    fn engine_folders_are_not_mod_folders() {
        let fixture = TempFixture::new("engine_folders");
        let data_dir = fixture.path.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("stray.tp2"), b"tp2").unwrap();
        let report = inspect(&fixture.path, SourceGame::Bgee);
        assert_eq!(report.residue.mod_folders, 0);
    }

    #[test]
    fn non_bgee_games_report_not_applicable() {
        let fixture = TempFixture::new("non_bgee");
        let report = inspect(&fixture.path, SourceGame::Bg2ee);
        assert_eq!(report.sod, SodDlcState::NotApplicable);
    }

    #[test]
    fn inspect_reads_the_game_version() {
        let solo_game = TempFixture::new("game_version_bgee");
        std::fs::write(solo_game.path.join("chitin.key"), b"data/PATCH26.BIF").unwrap();
        let report = inspect(&solo_game.path, SourceGame::Bgee);
        assert_eq!(report.game_version, Some(GameVersion::V2_6));

        let paired_game = TempFixture::new("game_version_bg2ee");
        std::fs::write(paired_game.path.join("chitin.key"), b"data/PATCH26.BIF").unwrap();
        let report = inspect(&paired_game.path, SourceGame::Bg2ee);
        assert_eq!(report.game_version, Some(GameVersion::V2_6));
    }
}
