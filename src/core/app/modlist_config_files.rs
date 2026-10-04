// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::platform_defaults::app_config_file;

const PENDING_MOD_CONFIGS_FILE_NAME: &str = "modlist_pending_mod_configs.toml";

#[derive(Debug, Default, Deserialize, Serialize)]
struct PendingModConfigsFile {
    format_version: u64,
    #[serde(default)]
    files: Vec<PendingModConfigFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PendingModConfigFile {
    tp2: String,
    source_id: String,
    relative_path: String,
    base64_data: String,
}

pub(crate) fn pending_mod_configs_path() -> PathBuf {
    app_config_file(PENDING_MOD_CONFIGS_FILE_NAME, "config")
}

pub(crate) fn save_pending_mod_configs(
    files: &[crate::app::modlist_share::ModlistShareConfigFile],
) -> Result<(), String> {
    for file in files {
        validate_relative_config_path(&file.relative_path)?;
    }
    let path = pending_mod_configs_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let pending = PendingModConfigsFile {
        format_version: 1,
        files: files
            .iter()
            .map(|file| PendingModConfigFile {
                tp2: file.tp2.clone(),
                source_id: file.source_id.clone(),
                relative_path: file.relative_path.clone(),
                base64_data: file.base64_data.clone(),
            })
            .collect(),
    };
    let content = toml::to_string_pretty(&pending).map_err(|err| err.to_string())?;
    fs::write(path, content).map_err(|err| err.to_string())
}

const OS_ARTIFACT_FILE_NAMES: [&str; 3] = ["desktop.ini", "thumbs.db", ".ds_store"];

#[must_use]
pub(crate) fn is_os_artifact_file(relative_path: &Path) -> bool {
    relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            OS_ARTIFACT_FILE_NAMES
                .iter()
                .any(|artifact| name.eq_ignore_ascii_case(artifact))
        })
}

pub(crate) fn validate_relative_config_path(relative_path: &str) -> Result<PathBuf, String> {
    let normalized = relative_path.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err("mod config path is empty".to_string());
    }
    if normalized.starts_with('/') || normalized.starts_with('\\') {
        return Err(format!("mod config path is root-prefixed: {relative_path}"));
    }
    if has_drive_prefix(&normalized) {
        return Err(format!(
            "mod config path has a drive prefix: {relative_path}"
        ));
    }
    if Path::new(&normalized).is_absolute() {
        return Err(format!("mod config path is absolute: {relative_path}"));
    }
    if normalized.split('/').any(|part| part == "..") {
        return Err(format!(
            "mod config path contains traversal: {relative_path}"
        ));
    }
    if let Some(reason) = normalized.split('/').find_map(segment_refusal) {
        return Err(format!("mod config path {reason}: {relative_path}"));
    }
    Ok(PathBuf::from(normalized))
}

fn segment_refusal(segment: &str) -> Option<&'static str> {
    if segment.is_empty() {
        return Some("has an empty segment");
    }
    if segment.chars().all(|ch| ch == '.') {
        return Some("has a dot-only segment");
    }
    if segment
        .chars()
        .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*') || ch < '\u{20}')
    {
        return Some("uses a character Windows refuses");
    }
    if segment.ends_with('.') || segment.ends_with(' ') {
        return Some("has a segment ending in a dot or space");
    }
    if is_windows_reserved_name(segment) {
        return Some("uses a reserved Windows name");
    }
    None
}

pub(crate) fn restore_pending_mod_configs_for_mod(
    tp2: &str,
    source_id: &str,
    aliases: &[String],
    target_root: &Path,
) -> Result<(), String> {
    let path = pending_mod_configs_path();
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let pending = toml::from_str::<PendingModConfigsFile>(&content).map_err(|err| {
        format!(
            "Read pending mod config metadata failed ({}): {err}",
            path.display()
        )
    })?;
    let accepted_tp2s = accepted_tp2_keys(tp2, aliases);
    let source_id = source_id.trim().to_ascii_lowercase();
    for file in pending.files {
        let pending_tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&file.tp2);
        if !accepted_tp2s.contains(&pending_tp2) {
            continue;
        }
        let pending_source_id = file.source_id.trim().to_ascii_lowercase();
        if !pending_source_id.is_empty() && pending_source_id != source_id {
            continue;
        }
        let relative_path = validate_relative_config_path(&file.relative_path)?;
        if is_os_artifact_file(&relative_path) {
            continue;
        }
        let destination = safe_config_destination(target_root, &relative_path)?;
        let bytes = base64url_decode(&file.base64_data)
            .map_err(|err| format!("Decode pending mod config failed: {err}"))?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        fs::write(&destination, bytes).map_err(|err| {
            format!(
                "Write pending mod config failed ({}): {err}",
                destination.display()
            )
        })?;
    }
    Ok(())
}

fn accepted_tp2_keys(tp2: &str, aliases: &[String]) -> Vec<String> {
    let mut accepted = vec![crate::app::mod_downloads::normalize_mod_download_tp2(tp2)];
    accepted.extend(
        aliases
            .iter()
            .map(|alias| crate::app::mod_downloads::normalize_mod_download_tp2(alias)),
    );
    accepted.sort();
    accepted.dedup();
    accepted
}

fn safe_config_destination(target_root: &Path, relative_path: &Path) -> Result<PathBuf, String> {
    let canonical_root = target_root.canonicalize().map_err(|err| {
        format!(
            "Canonicalize mod config root failed ({}): {err}",
            target_root.display()
        )
    })?;
    let destination = target_root.join(relative_path);
    let parent = destination
        .parent()
        .ok_or_else(|| "mod config destination parent is missing".to_string())?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let canonical_parent = parent.canonicalize().map_err(|err| {
        format!(
            "Canonicalize mod config destination failed ({}): {err}",
            parent.display()
        )
    })?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(format!(
            "mod config destination escapes mod root: {}",
            destination.display()
        ));
    }
    Ok(destination)
}

fn has_drive_prefix(path: &str) -> bool {
    let mut chars = path.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) && chars.next() == Some(':')
}

fn is_windows_reserved_name(segment: &str) -> bool {
    let stem = segment
        .split_once('.')
        .map_or(segment, |(stem, _)| stem)
        .trim()
        .to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix)
                .is_some_and(|digit| matches!(digit.as_bytes(), [b'1'..=b'9']))
        }),
    }
}

fn base64url_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut values = Vec::new();
    for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
        match ch {
            'A'..='Z' => values.push(ch as u8 - b'A'),
            'a'..='z' => values.push(ch as u8 - b'a' + 26),
            '0'..='9' => values.push(ch as u8 - b'0' + 52),
            '-' => values.push(62),
            '_' => values.push(63),
            _ => {
                return Err("pending mod config contains invalid base64url characters.".to_string());
            }
        }
    }
    let remainder = values.len() % 4;
    if remainder == 1 {
        return Err("pending mod config base64url length is invalid.".to_string());
    }
    if remainder != 0 {
        values.extend(std::iter::repeat_n(64, 4 - remainder));
    }
    let mut out = Vec::with_capacity(values.len() / 4 * 3);
    for chunk in values.chunks(4) {
        let pad = usize::from(chunk[3] == 64) + usize::from(chunk[2] == 64);
        if pad > 2 || chunk[..4 - pad].contains(&64) {
            return Err("pending mod config base64 padding is invalid.".to_string());
        }
        let c0 = chunk[0];
        let c1 = chunk[1];
        let c2 = if chunk[2] == 64 { 0 } else { chunk[2] };
        let c3 = if chunk[3] == 64 { 0 } else { chunk[3] };
        out.push((c0 << 2) | (c1 >> 4));
        if pad < 2 {
            out.push(((c1 & 0b0000_1111) << 4) | (c2 >> 2));
        }
        if pad == 0 {
            out.push(((c2 & 0b0000_0011) << 6) | c3);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modlist_share::ModlistShareConfigFile;
    use crate::platform_defaults::{clear_config_dir_override_if, set_config_dir_override};

    struct TempRootGuard(PathBuf);

    impl Drop for TempRootGuard {
        fn drop(&mut self) {
            clear_config_dir_override_if(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn temp_root(tag: &str) -> TempRootGuard {
        let root = std::env::temp_dir().join(format!(
            "bio_modcfg_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&root).expect("temp root");
        set_config_dir_override(Some(root.clone()));
        TempRootGuard(root)
    }

    #[test]
    fn os_artifact_names_are_recognised_case_insensitively() {
        assert!(is_os_artifact_file(Path::new("desktop.ini")));
        assert!(is_os_artifact_file(Path::new("Desktop.INI")));
        assert!(is_os_artifact_file(Path::new("sub/Thumbs.db")));
        assert!(is_os_artifact_file(Path::new(".DS_Store")));
        assert!(!is_os_artifact_file(Path::new("cdtweaks.ini")));
        assert!(!is_os_artifact_file(Path::new("desktop.ini.bak")));
        assert!(!is_os_artifact_file(Path::new("thumbs/settings.ini")));
    }

    #[test]
    fn restore_skips_os_artifacts_and_writes_real_configs() {
        let guard = temp_root("restore_skip");
        let mod_root = guard.0.join("mod");
        fs::create_dir_all(&mod_root).expect("mod root");
        let files = vec![
            ModlistShareConfigFile {
                tp2: "cdtweaks".to_string(),
                source_id: "gibberlings3".to_string(),
                relative_path: "cdtweaks.ini".to_string(),
                base64_data: crate::app::modlist_share::base64url_encode(b"[cfg]\nx=1\n"),
            },
            ModlistShareConfigFile {
                tp2: "cdtweaks".to_string(),
                source_id: "gibberlings3".to_string(),
                relative_path: "desktop.ini".to_string(),
                base64_data: crate::app::modlist_share::base64url_encode(b"[.ShellClassInfo]\n"),
            },
        ];
        assert!(pending_mod_configs_path().starts_with(&guard.0));
        save_pending_mod_configs(&files).expect("pending file written under the override");

        restore_pending_mod_configs_for_mod("cdtweaks", "gibberlings3", &[], &mod_root)
            .expect("restore");

        assert!(mod_root.join("cdtweaks.ini").is_file());
        assert!(!mod_root.join("desktop.ini").exists());
    }

    #[test]
    fn default_sources_list_no_os_artifact_config_files() {
        let load = crate::app::mod_downloads::load_mod_download_sources_from_texts(
            include_str!("../config/default_mod_downloads.toml"),
            "",
            "",
        );
        assert!(load.error.is_none(), "{:?}", load.error);
        let offenders: Vec<String> = load
            .sources
            .iter()
            .flat_map(|source| {
                source
                    .config_files
                    .iter()
                    .filter(|path| {
                        validate_relative_config_path(path)
                            .is_ok_and(|normalized| is_os_artifact_file(&normalized))
                    })
                    .map(|path| format!("{}: {path}", source.tp2))
            })
            .collect();
        assert_eq!(offenders.len(), 0, "{offenders:?}");
    }

    fn assert_refused_with(paths: &[&str], expected: &str) {
        for path in paths {
            let error = validate_relative_config_path(path).expect_err(path);
            assert!(error.contains(expected), "{path:?}: {error}");
            assert!(error.ends_with(&format!(": {path}")), "{path:?}: {error}");
        }
    }

    #[test]
    fn dot_only_segments_are_refused() {
        assert_refused_with(
            &[".../test.ini", "..../a.ini", "./a.ini", "a/.../b.ini"],
            "dot-only",
        );
    }

    #[test]
    fn empty_segments_are_refused() {
        assert_refused_with(&["a//b.ini", "a/"], "empty segment");
    }

    #[test]
    fn windows_refused_characters_are_refused() {
        for path in [
            "a<b.ini",
            "a:b.ini",
            "a|b.ini",
            "a?.ini",
            "a*.ini",
            "\"a\".ini",
            "a\u{7}.ini",
        ] {
            assert!(validate_relative_config_path(path).is_err(), "{path:?}");
        }
        assert_refused_with(
            &[
                "a<b.ini",
                "sub/a:b.ini",
                "a|b.ini",
                "a?.ini",
                "a*.ini",
                "\"a\".ini",
                "a\u{7}.ini",
            ],
            "character Windows refuses",
        );
    }

    #[test]
    fn segments_ending_in_dot_or_space_are_refused() {
        assert_refused_with(
            &["a./b.ini", "a /b.ini", "b.ini."],
            "ending in a dot or space",
        );
    }

    #[test]
    fn windows_reserved_names_are_refused() {
        assert_refused_with(
            &[
                "CON",
                "con.ini",
                "sub/NUL.txt",
                "com1.ini",
                "LPT9",
                "Aux.cfg",
            ],
            "reserved Windows name",
        );
    }

    #[test]
    fn ordinary_names_still_pass() {
        for (path, normalized) in [
            ("a.ini", "a.ini"),
            ("sub/b.ini", "sub/b.ini"),
            ("sub\\c.ini", "sub/c.ini"),
            ("weird name (1).ini", "weird name (1).ini"),
            ("x.y.z.ini", "x.y.z.ini"),
            (".hidden.ini", ".hidden.ini"),
            ("console.ini", "console.ini"),
            ("communist.ini", "communist.ini"),
            ("com10.ini", "com10.ini"),
            ("lpt.ini", "lpt.ini"),
        ] {
            assert_eq!(
                validate_relative_config_path(path),
                Ok(PathBuf::from(normalized)),
                "{path:?}"
            );
        }
    }

    #[test]
    fn traversal_keeps_its_own_message() {
        assert_eq!(
            validate_relative_config_path("../x.ini"),
            Err("mod config path contains traversal: ../x.ini".to_string())
        );
    }
}
