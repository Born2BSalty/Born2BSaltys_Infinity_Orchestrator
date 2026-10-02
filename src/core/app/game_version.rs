// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::state::Step1State;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameVersion {
    V2_6,
    V2_7,
}

impl GameVersion {
    #[must_use]
    pub(crate) const fn tag(self) -> &'static str {
        match self {
            Self::V2_6 => "2.6",
            Self::V2_7 => "2.7",
        }
    }
}

fn highest_patch_number(lowered: &[u8]) -> Option<u32> {
    let mut highest: Option<u32> = None;
    for prefix in [b"data/patch".as_slice(), b"data\\patch".as_slice()] {
        let mut start = 0;
        while let Some(offset) = find_bytes(&lowered[start..], prefix) {
            let digits_start = start + offset + prefix.len();
            let digits = lowered
                .get(digits_start..digits_start + 2)
                .filter(|slice| slice.iter().all(u8::is_ascii_digit));
            if let Some(digits) = digits
                && lowered[digits_start + 2..].starts_with(b".bif")
            {
                let number = u32::from(digits[0] - b'0') * 10 + u32::from(digits[1] - b'0');
                highest = Some(highest.map_or(number, |current| current.max(number)));
            }
            start += offset + prefix.len();
        }
    }
    highest
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[must_use]
pub(crate) fn read_game_version(folder: &Path) -> Option<GameVersion> {
    let bytes = std::fs::read(folder.join("chitin.key")).ok()?;
    let lowered: Vec<u8> = bytes.iter().map(u8::to_ascii_lowercase).collect();
    match highest_patch_number(&lowered) {
        Some(26) => Some(GameVersion::V2_6),
        Some(27) => Some(GameVersion::V2_7),
        _ => None,
    }
}

#[must_use]
pub(crate) fn installed_game_folders<'a>(
    step1: &'a Step1State,
    game_install: &str,
) -> Vec<(&'static str, &'a str)> {
    match game_install.trim() {
        "EET" => vec![
            ("BGEE", step1.eet_pre_dir.trim()),
            ("BG2EE", step1.eet_new_dir.trim()),
        ],
        "BGEE" => vec![("BGEE", step1.generate_directory.trim())],
        "BG2EE" => vec![("BG2EE", step1.generate_directory.trim())],
        "IWDEE" => vec![("IWDEE", step1.generate_directory.trim())],
        _ => vec![],
    }
}

#[must_use]
pub(crate) fn mint_tag(step1: &Step1State, game_install: &str) -> Option<GameVersion> {
    let folders = installed_game_folders(step1, game_install);
    if folders.is_empty() {
        return None;
    }
    let mut agreed: Option<GameVersion> = None;
    for (_, folder) in folders {
        if folder.is_empty() {
            return None;
        }
        let version = read_game_version(Path::new(folder))?;
        match agreed {
            None => agreed = Some(version),
            Some(existing) if existing == version => {}
            Some(_) => return None,
        }
    }
    agreed
}

fn folder_version_fragment(label: &str, folder: &str) -> String {
    read_game_version(Path::new(folder)).map_or_else(
        || format!("BIO could not tell the version of the {label} game folder."),
        |version| format!("The {label} game folder is version {}.", version.tag()),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallBlock {
    VersionMismatch,
    VersionUnknown,
}

impl InstallBlock {
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::VersionMismatch => "Game version mismatch",
            Self::VersionUnknown => "Game version unknown",
        }
    }
}

#[must_use]
pub(crate) fn install_block_cached(
    step1: &Step1State,
    game_install: &str,
    tag: &str,
) -> Option<InstallBlock> {
    let mut unknown = false;
    for (_, folder) in crate::app::game_authority::source_game_folders(step1, game_install) {
        if folder.is_empty() {
            continue;
        }
        match crate::app::compat_dlc_source::probed_game_version(step1, folder) {
            Some(version) if version.tag() == tag => {}
            Some(_) => return Some(InstallBlock::VersionMismatch),
            None => unknown = true,
        }
    }
    unknown.then_some(InstallBlock::VersionUnknown)
}

#[must_use]
pub(crate) fn version_mismatch_message(
    step1: &Step1State,
    game_install: &str,
    tag: &str,
) -> Option<String> {
    let mismatches: Vec<String> =
        crate::app::game_authority::source_game_folders(step1, game_install)
            .into_iter()
            .filter(|(_, folder)| !folder.is_empty())
            .filter(|(_, folder)| {
                read_game_version(Path::new(folder)).map(GameVersion::tag) != Some(tag)
            })
            .map(|(label, folder)| folder_version_fragment(label, folder))
            .collect();

    if mismatches.is_empty() {
        return None;
    }
    Some(format!(
        "This modlist needs game version {tag}. {} Point Settings \u{2192} Paths at a game folder on version {tag}.",
        mismatches.join(" ")
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        GameVersion, InstallBlock, install_block_cached, installed_game_folders, mint_tag,
        read_game_version, version_mismatch_message,
    };
    use crate::app::compat_dlc_source::refresh_source_check;
    use crate::app::state::Step1State;

    struct TempFixture {
        path: std::path::PathBuf,
    }

    impl TempFixture {
        fn new(name: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_game_version_test_{name}_{}_{id}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn write_key(&self, bytes: &[u8]) {
            std::fs::write(self.path.join("chitin.key"), bytes).unwrap();
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn reads_2_7_from_mixed_case_entries() {
        let fixture = TempFixture::new("mixed_case");
        fixture.write_key(b"x\0DATA/PATCH26.BIF\0data/PATCH27.bif\0");
        assert_eq!(read_game_version(&fixture.path), Some(GameVersion::V2_7));
    }

    #[test]
    fn reads_2_6_when_27_is_absent() {
        let fixture = TempFixture::new("absent_27");
        fixture.write_key(b"DATA/PATCH25.BIF\0DATA/PATCH26.BIF");
        assert_eq!(read_game_version(&fixture.path), Some(GameVersion::V2_6));
    }

    #[test]
    fn backslash_entries_count() {
        let fixture = TempFixture::new("backslash");
        fixture.write_key(b"data\\patch26.bif");
        assert_eq!(read_game_version(&fixture.path), Some(GameVersion::V2_6));
    }

    #[test]
    fn single_digit_and_older_patches_are_ignored() {
        let fixture = TempFixture::new("single_digit");
        fixture.write_key(b"data/Patch2.bif\0data/patch13.bif\0data/PATCH26.BIF");
        assert_eq!(read_game_version(&fixture.path), Some(GameVersion::V2_6));
    }

    #[test]
    fn older_patch_only_is_unknown() {
        let fixture = TempFixture::new("older_only");
        fixture.write_key(b"data/PATCH25.BIF");
        assert_eq!(read_game_version(&fixture.path), None);
    }

    #[test]
    fn future_patch_is_unknown() {
        let fixture = TempFixture::new("future");
        fixture.write_key(b"data/PATCH26.BIF\0data/PATCH28.bif");
        assert_eq!(read_game_version(&fixture.path), None);
    }

    #[test]
    fn missing_key_is_unknown() {
        let fixture = TempFixture::new("missing_key");
        assert_eq!(read_game_version(&fixture.path), None);
    }

    #[test]
    fn eet_mint_tag_needs_both_folders_to_agree() {
        let pre = TempFixture::new("eet_pre_agree");
        let new = TempFixture::new("eet_new_agree");
        pre.write_key(b"data/PATCH26.BIF");
        new.write_key(b"data/PATCH26.BIF");
        let step1 = Step1State {
            eet_pre_dir: pre.path.to_string_lossy().to_string(),
            eet_new_dir: new.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        assert_eq!(mint_tag(&step1, "EET"), Some(GameVersion::V2_6));

        let new_mismatch = TempFixture::new("eet_new_mismatch");
        new_mismatch.write_key(b"data/PATCH27.BIF");
        let step1_mismatch = Step1State {
            eet_pre_dir: pre.path.to_string_lossy().to_string(),
            eet_new_dir: new_mismatch.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        assert_eq!(mint_tag(&step1_mismatch, "EET"), None);

        let step1_empty = Step1State {
            eet_pre_dir: pre.path.to_string_lossy().to_string(),
            eet_new_dir: String::new(),
            ..Step1State::default()
        };
        assert_eq!(mint_tag(&step1_empty, "EET"), None);
    }

    #[test]
    fn mint_tag_reads_the_install_copy_not_settings() {
        let settings_folder = TempFixture::new("settings_2_7");
        settings_folder.write_key(b"data/PATCH27.bif");
        let install_copy = TempFixture::new("install_copy");
        install_copy.write_key(b"data/PATCH26.BIF");
        let step1 = Step1State {
            bg2ee_game_folder: settings_folder.path.to_string_lossy().to_string(),
            generate_directory: install_copy.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        assert_eq!(mint_tag(&step1, "BG2EE"), Some(GameVersion::V2_6));

        let step1_empty = Step1State {
            bg2ee_game_folder: settings_folder.path.to_string_lossy().to_string(),
            generate_directory: String::new(),
            ..Step1State::default()
        };
        assert_eq!(mint_tag(&step1_empty, "BG2EE"), None);
    }

    #[test]
    fn mismatch_message_names_each_wrong_folder() {
        let first_fixture = TempFixture::new("mismatch_bgee");
        let second_fixture = TempFixture::new("mismatch_bg2ee");
        first_fixture.write_key(b"data/PATCH26.BIF");
        second_fixture.write_key(b"data/PATCH27.BIF");
        let step1 = Step1State {
            bgee_game_folder: first_fixture.path.to_string_lossy().to_string(),
            bg2ee_game_folder: second_fixture.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        assert_eq!(
            version_mismatch_message(&step1, "EET", "2.6"),
            Some(
                "This modlist needs game version 2.6. The BG2EE game folder is version 2.7. Point Settings \u{2192} Paths at a game folder on version 2.6."
                    .to_string()
            )
        );
    }

    #[test]
    fn unreadable_folder_message() {
        let step1 = Step1State {
            bgee_game_folder: "does-not-exist-anywhere".to_string(),
            ..Step1State::default()
        };
        assert_eq!(
            version_mismatch_message(&step1, "BGEE", "2.6"),
            Some(
                "This modlist needs game version 2.6. BIO could not tell the version of the BGEE game folder. Point Settings \u{2192} Paths at a game folder on version 2.6."
                    .to_string()
            )
        );
    }

    #[test]
    fn matching_and_empty_folders_give_no_message() {
        let bgee = TempFixture::new("match_bgee");
        bgee.write_key(b"data/PATCH26.BIF");
        let step1 = Step1State {
            bgee_game_folder: bgee.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        assert_eq!(version_mismatch_message(&step1, "BGEE", "2.6"), None);

        let step1_empty = Step1State::default();
        assert_eq!(version_mismatch_message(&step1_empty, "BGEE", "2.6"), None);
    }

    #[test]
    fn unknown_tag_never_matches() {
        let bg2ee = TempFixture::new("unknown_tag");
        bg2ee.write_key(b"data/PATCH27.BIF");
        let step1 = Step1State {
            bg2ee_game_folder: bg2ee.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        assert!(version_mismatch_message(&step1, "BG2EE", "2.8").is_some());
    }

    #[test]
    fn installed_game_folders_match_the_table() {
        let step1 = Step1State {
            eet_pre_dir: "  /pre  ".to_string(),
            eet_new_dir: "  /new  ".to_string(),
            generate_directory: "  /single  ".to_string(),
            ..Step1State::default()
        };
        assert_eq!(
            installed_game_folders(&step1, "EET"),
            vec![("BGEE", "/pre"), ("BG2EE", "/new")]
        );
        assert_eq!(
            installed_game_folders(&step1, "BGEE"),
            vec![("BGEE", "/single")]
        );
        assert_eq!(
            installed_game_folders(&step1, "BG2EE"),
            vec![("BG2EE", "/single")]
        );
        assert_eq!(
            installed_game_folders(&step1, "IWDEE"),
            vec![("IWDEE", "/single")]
        );
        assert_eq!(installed_game_folders(&step1, "garbage").len(), 0);
    }

    #[test]
    fn install_block_cached_follows_the_probe() {
        let mismatched = TempFixture::new("install_block_mismatched");
        mismatched.write_key(b"data/PATCH27.BIF");
        let matched = TempFixture::new("install_block_matched");
        matched.write_key(b"data/PATCH26.BIF");
        let unprobed = TempFixture::new("install_block_unprobed");
        unprobed.write_key(b"data/PATCH27.BIF");

        let mut step1 = Step1State {
            bg2ee_game_folder: mismatched.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        refresh_source_check(&mut step1);
        assert_eq!(
            install_block_cached(&step1, "BG2EE", "2.6"),
            Some(InstallBlock::VersionMismatch)
        );

        step1.bg2ee_game_folder = matched.path.to_string_lossy().to_string();
        refresh_source_check(&mut step1);
        assert_eq!(install_block_cached(&step1, "BG2EE", "2.6"), None);

        step1.bg2ee_game_folder = String::new();
        assert_eq!(install_block_cached(&step1, "BG2EE", "2.6"), None);

        step1.bg2ee_game_folder = unprobed.path.to_string_lossy().to_string();
        assert_eq!(
            install_block_cached(&step1, "BG2EE", "2.6"),
            Some(InstallBlock::VersionUnknown)
        );

        let no_key = TempFixture::new("install_block_no_key");
        let eet_bgee = TempFixture::new("install_block_eet_bgee");
        eet_bgee.write_key(b"data/PATCH27.BIF");
        let mut eet_step1 = Step1State {
            bgee_game_folder: eet_bgee.path.to_string_lossy().to_string(),
            bg2ee_game_folder: no_key.path.to_string_lossy().to_string(),
            ..Step1State::default()
        };
        refresh_source_check(&mut eet_step1);
        assert_eq!(
            install_block_cached(&eet_step1, "EET", "2.6"),
            Some(InstallBlock::VersionMismatch)
        );
    }

    #[test]
    fn install_block_labels() {
        assert_eq!(
            InstallBlock::VersionMismatch.label(),
            "Game version mismatch"
        );
        assert_eq!(InstallBlock::VersionUnknown.label(), "Game version unknown");
    }
}
