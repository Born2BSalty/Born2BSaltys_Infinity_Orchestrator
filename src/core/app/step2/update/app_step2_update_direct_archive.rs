// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::app_step2_update_check::{
    Step2PackageKind, Step2UpdateCheckOutcome, Step2UpdateCheckRequest, failed_outcome,
};

const ARCHIVE_SUFFIXES_LONGEST_FIRST: [&str; 9] = [
    ".tar.bz2", ".tar.gz", ".tar.xz", ".tbz2", ".zip", ".rar", ".tgz", ".txz", ".7z",
];

pub(crate) fn direct_archive_names(url: &str) -> Option<(String, String)> {
    let trimmed = url.trim();
    let without_suffix = trimmed
        .find(['?', '#'])
        .map_or(trimmed, |cut| &trimmed[..cut]);
    let path = without_suffix.trim_end_matches('/');
    let file_name = path.rsplit('/').next().unwrap_or(path);
    if file_name.is_empty() {
        return None;
    }
    let lower = file_name.to_ascii_lowercase();
    let suffix = ARCHIVE_SUFFIXES_LONGEST_FIRST
        .iter()
        .find(|suffix| lower.ends_with(*suffix))?;
    let stem = &file_name[..file_name.len() - suffix.len()];
    if stem.is_empty() {
        return None;
    }
    Some((file_name.to_string(), stem.to_string()))
}

pub(super) fn check_direct_archive(request: &Step2UpdateCheckRequest) -> Step2UpdateCheckOutcome {
    let Some((file_name, stem)) = direct_archive_names(&request.source_url) else {
        return failed_outcome(request.clone(), "direct link has no file name");
    };
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab.clone(),
        tp_file: request.tp_file.clone(),
        label: request.label.clone(),
        source_id: request.source_id.clone(),
        source_url: String::new(),
        tag: Some(stem),
        source_ref: None,
        asset_name: Some(file_name),
        asset_url: Some(request.source_url.trim().to_string()),
        error: None,
        package_kind: Step2PackageKind::ReleaseAsset,
        version_pin_overridden: None,
    }
}

#[cfg(test)]
mod tests {
    use super::direct_archive_names;

    fn names(file_name: &str, stem: &str) -> (String, String) {
        (file_name.to_string(), stem.to_string())
    }

    #[test]
    fn direct_archive_names_splits_a_plain_zip() {
        assert_eq!(
            direct_archive_names("https://pocketplane.net/mods/questpack-v35-win.zip"),
            Some(names("questpack-v35-win.zip", "questpack-v35-win"))
        );
    }

    #[test]
    fn direct_archive_names_drops_query_and_fragment() {
        assert_eq!(
            direct_archive_names("https://h/x/mod.tar.gz?dl=1#top"),
            Some(names("mod.tar.gz", "mod"))
        );
    }

    #[test]
    fn direct_archive_names_is_case_insensitive() {
        assert_eq!(
            direct_archive_names("https://h/BGEETenya15c.RAR"),
            Some(names("BGEETenya15c.RAR", "BGEETenya15c"))
        );
    }

    #[test]
    fn direct_archive_names_refuses_a_bare_extension() {
        assert_eq!(direct_archive_names("https://h/.zip"), None);
    }

    #[test]
    fn direct_archive_names_refuses_a_folder_url() {
        assert_eq!(direct_archive_names("https://h/mods/"), None);
    }
}
