// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::app::game_authority;
use crate::app::modlist_share::{
    ForkAncestor, ModlistSharePayload, base64url_decode, first_slot_weidu_text,
};
use crate::registry::share_export::ArchiveMeta;

pub const BIOLIST_EXTENSION: &str = "biolist";
pub const BIOLIST_FILTER_LABEL: &str = "BIO modlist";
pub const SHARE_CODE_ENTRY: &str = "share-code.txt";
pub const NOT_A_MODLIST_FILE: &str = "That is not a BIO modlist file.";

const README_TEXT: &str = "This folder is a readable copy of share-code.txt, split into files.\nBIO does not read anything in this folder; edits here change nothing.\nTo change the modlist, edit it in BIO and export it again.\n";

const MAX_SHARE_CODE_ENTRY_BYTES: u64 = 1024 * 1024;

#[must_use]
pub fn suggested_file_name(modlist_name: &str) -> String {
    let sanitized = sanitize_component(modlist_name.trim());
    let base = if sanitized.is_empty() {
        "modlist".to_string()
    } else {
        sanitized
    };
    format!("{base}.{BIOLIST_EXTENSION}")
}

pub fn build_biolist(code: &str) -> Result<Vec<u8>, String> {
    let payload = crate::app::modlist_share::decode_share_payload(code)?;
    let archives = crate::registry::share_export::decode_archive_meta(code)?;

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));

    write_entry(
        &mut writer,
        SHARE_CODE_ENTRY,
        format!("{}\n", code.trim()).as_bytes(),
    )?;
    write_entry(&mut writer, "reference/README.txt", README_TEXT.as_bytes())?;
    write_reference_modlist(&mut writer, &payload)?;

    if let Some(text) = non_empty(first_slot_weidu_text(&payload.weidu_logs)) {
        let entry_name = format!(
            "reference/weidu/{}",
            game_authority::reference_log_name(game_authority::first_slot_tab(
                &payload.game_install
            ))
        );
        write_entry(&mut writer, &entry_name, text.as_bytes())?;
    }
    if let Some(text) = non_empty(payload.weidu_logs.bg2ee.as_deref()) {
        write_entry(&mut writer, "reference/weidu/BG2EE.log", text.as_bytes())?;
    }
    if let Some(text) = non_empty(payload.source_overrides.mod_downloads_user_toml.as_deref()) {
        write_entry(&mut writer, "reference/sources.toml", text.as_bytes())?;
    }
    if let Some(text) = non_empty(payload.installed_refs.mod_installed_refs_toml.as_deref()) {
        write_entry(
            &mut writer,
            "reference/installed-refs.toml",
            text.as_bytes(),
        )?;
    }
    write_reference_archives(&mut writer, &archives)?;
    write_reference_configs(&mut writer, &payload)?;

    let cursor = writer.finish().map_err(|err| err.to_string())?;
    Ok(cursor.into_inner())
}

pub fn write_biolist(code: &str, path: &Path) -> Result<(), String> {
    let bytes = build_biolist(code)?;
    std::fs::write(path, bytes).map_err(|err| err.to_string())
}

pub fn read_share_code(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|_| NOT_A_MODLIST_FILE.to_string())?;
    read_share_code_from_reader(file)
}

pub fn read_share_code_from_bytes(bytes: &[u8]) -> Result<String, String> {
    read_share_code_from_reader(Cursor::new(bytes))
}

fn read_share_code_from_reader<R: Read + Seek>(reader: R) -> Result<String, String> {
    let mut archive = ZipArchive::new(reader).map_err(|_| NOT_A_MODLIST_FILE.to_string())?;
    let entry = archive
        .by_name(SHARE_CODE_ENTRY)
        .map_err(|_| NOT_A_MODLIST_FILE.to_string())?;
    if entry.size() > MAX_SHARE_CODE_ENTRY_BYTES {
        return Err(NOT_A_MODLIST_FILE.to_string());
    }
    let mut text = String::new();
    let mut limited = entry.take(MAX_SHARE_CODE_ENTRY_BYTES + 1);
    limited
        .read_to_string(&mut text)
        .map_err(|_| NOT_A_MODLIST_FILE.to_string())?;
    let max_len = usize::try_from(MAX_SHARE_CODE_ENTRY_BYTES).unwrap_or(usize::MAX);
    if text.len() > max_len {
        return Err(NOT_A_MODLIST_FILE.to_string());
    }
    Ok(text.trim().to_string())
}

#[derive(Serialize)]
struct ReferenceModlist {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    game: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    game_version: Option<String>,
    install_mode: String,
    bio_version: String,
    format_version: u64,
    allow_auto_install: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    forked_from: Vec<ForkAncestor>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unresolved_mods: Vec<String>,
}

#[derive(Serialize)]
struct ReferenceArchives {
    archives: Vec<ReferenceArchive>,
}

#[derive(Serialize)]
struct ReferenceArchive {
    name: String,
    size: u64,
    hash: String,
}

fn write_reference_modlist(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    payload: &ModlistSharePayload,
) -> Result<(), String> {
    let reference_modlist = ReferenceModlist {
        name: payload.name.clone(),
        author: payload.author.clone(),
        description: payload.description.clone(),
        game: payload.game_install.clone(),
        game_version: payload.game_version.clone(),
        install_mode: payload.install_mode.clone(),
        bio_version: payload.bio_version.clone(),
        format_version: payload.format_version,
        allow_auto_install: payload.allow_auto_install,
        forked_from: payload.forked_from.clone(),
        unresolved_mods: payload.source_overrides.unresolved_mods.clone(),
    };
    let text = toml::to_string_pretty(&reference_modlist).map_err(|err| err.to_string())?;
    write_entry(writer, "reference/modlist.toml", text.as_bytes())
}

fn write_reference_archives(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    archives: &[ArchiveMeta],
) -> Result<(), String> {
    if archives.is_empty() {
        return Ok(());
    }
    let reference_archives = ReferenceArchives {
        archives: archives
            .iter()
            .map(|meta| ReferenceArchive {
                name: meta.name.clone(),
                size: meta.size,
                hash: meta.hash.clone(),
            })
            .collect(),
    };
    let text = toml::to_string_pretty(&reference_archives).map_err(|err| err.to_string())?;
    write_entry(writer, "reference/archives.toml", text.as_bytes())
}

fn write_reference_configs(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    payload: &ModlistSharePayload,
) -> Result<(), String> {
    for file in &payload.mod_configs.files {
        let Ok(bytes) = base64url_decode(&file.base64_data) else {
            continue;
        };
        let Some(relative) = config_relative_path(&file.relative_path) else {
            continue;
        };
        if crate::app::modlist_config_files::is_os_artifact_file(Path::new(&relative)) {
            continue;
        }
        let folder = config_folder_name(&file.tp2);
        let entry_name = format!("reference/config/{folder}/{relative}");
        write_entry(writer, &entry_name, &bytes)?;
    }
    Ok(())
}

fn write_entry(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    name: &str,
    contents: &[u8],
) -> Result<(), String> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer
        .start_file(name, options)
        .map_err(|err| err.to_string())?;
    writer.write_all(contents).map_err(|err| err.to_string())
}

fn non_empty(text: Option<&str>) -> Option<&str> {
    let text = text?;
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

const fn is_disallowed_char(ch: char) -> bool {
    matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_control()
}

fn sanitize_component(input: &str) -> String {
    input
        .chars()
        .map(|ch| if is_disallowed_char(ch) { '_' } else { ch })
        .collect()
}

fn config_folder_name(tp2: &str) -> String {
    let folder = sanitize_component(&crate::app::mod_downloads::normalize_mod_download_tp2(tp2));
    if folder.is_empty() || folder == "." || folder == ".." {
        "mod".to_string()
    } else {
        folder
    }
}

fn config_relative_path(relative_path: &str) -> Option<String> {
    let normalized = relative_path.replace('\\', "/");
    let components = normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .map(sanitize_component)
        .collect::<Vec<_>>();
    if components.is_empty() {
        None
    } else {
        Some(components.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempFileGuard(std::path::PathBuf);

    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn unique_temp_path(label: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "bio_biolist_test_{}_{}_{label}.biolist",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn full_payload_json() -> String {
        r#"{
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "BGEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": {
                "bgee": "~EEFIXPACK/EEFIXPACK.TP2~ #0 #0 // Core Fixes: 1.0",
                "bg2ee": "~EEFIXPACK/EEFIXPACK.TP2~ #0 #0 // Core Fixes: 1.0"
            },
            "source_overrides": { "mod_downloads_user_toml": "[[mods]]\nname = \"X\"\n", "unresolved_mods": ["Some Mod"] },
            "installed_refs": { "mod_installed_refs_toml": "[sources]\nx = \"y\"\n" },
            "mod_configs": {
                "files": [
                    {
                        "tp2": "EEFIXPACK/EEFIXPACK.TP2",
                        "source_id": "main",
                        "relative_path": "settings.ini",
                        "base64_data": "YT0xCg"
                    },
                    {
                        "tp2": "EEFIXPACK/EEFIXPACK.TP2",
                        "source_id": "main",
                        "relative_path": "desktop.ini",
                        "base64_data": "YT0xCg"
                    }
                ]
            },
            "name": "Polished BG2EE",
            "author": "@b2bs",
            "description": "BG2EE with the fixpack, the UI mod and tweaks",
            "forked_from": [{ "name": "Root build", "author": "@root" }]
        }"#
        .to_string()
    }

    fn minimal_bgee_only_payload_json() -> String {
        r#"{
            "format_version": 1,
            "game_install": "BGEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component: 1.0" }
        }"#
        .to_string()
    }

    fn minimal_iwdee_only_payload_json() -> String {
        r#"{
            "format_version": 1,
            "game_install": "IWDEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "iwdee": "~MOD/MOD.TP2~ #0 #0 // A component: 1.0" }
        }"#
        .to_string()
    }

    fn entry_names(bytes: &[u8]) -> Vec<String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes.to_vec())).expect("open zip");
        let mut names = Vec::with_capacity(archive.len());
        for index in 0..archive.len() {
            let entry = archive.by_index(index).expect("entry");
            names.push(entry.name().to_string());
        }
        names.sort();
        names
    }

    fn entry_text(bytes: &[u8], name: &str) -> String {
        let mut archive = ZipArchive::new(Cursor::new(bytes.to_vec())).expect("open zip");
        let mut entry = archive.by_name(name).expect("entry present");
        let mut text = String::new();
        entry.read_to_string(&mut text).expect("read entry");
        text
    }

    fn entry_bytes(bytes: &[u8], name: &str) -> Vec<u8> {
        let mut archive = ZipArchive::new(Cursor::new(bytes.to_vec())).expect("open zip");
        let mut entry = archive.by_name(name).expect("entry present");
        let mut data = Vec::new();
        entry.read_to_end(&mut data).expect("read entry");
        data
    }

    #[test]
    fn build_biolist_writes_the_code_and_every_reference_file() {
        let code = crate::app::modlist_share::encode_share_payload_text(&full_payload_json())
            .expect("encode");
        let code = crate::registry::share_export::bake_archive_meta_into_code(
            &code,
            &[
                ArchiveMeta {
                    name: "a__github__v1.zip".to_string(),
                    size: 17,
                    hash: "deadbeef00000000deadbeef00000000".to_string(),
                },
                ArchiveMeta {
                    name: "b__weasel__v2.7z".to_string(),
                    size: 4096,
                    hash: "0123456789abcdef0123456789abcdef".to_string(),
                },
            ],
        )
        .expect("bake archive meta");

        let bytes = build_biolist(&code).expect("build biolist");

        let names = entry_names(&bytes);
        assert_eq!(
            names,
            vec![
                "reference/README.txt".to_string(),
                "reference/archives.toml".to_string(),
                "reference/config/eefixpack/settings.ini".to_string(),
                "reference/installed-refs.toml".to_string(),
                "reference/modlist.toml".to_string(),
                "reference/sources.toml".to_string(),
                "reference/weidu/BG2EE.log".to_string(),
                "reference/weidu/BGEE.log".to_string(),
                "share-code.txt".to_string(),
            ],
            "the payload's desktop.ini entry is never unpacked into the reference folder"
        );

        assert_eq!(
            entry_text(&bytes, SHARE_CODE_ENTRY),
            format!("{}\n", code.trim())
        );
        assert_eq!(
            entry_text(&bytes, "reference/sources.toml"),
            "[[mods]]\nname = \"X\"\n"
        );
        assert_eq!(
            entry_bytes(&bytes, "reference/config/eefixpack/settings.ini"),
            base64url_decode("YT0xCg").expect("decode fixture")
        );
        assert_eq!(entry_text(&bytes, "reference/README.txt"), README_TEXT);

        let modlist_toml = entry_text(&bytes, "reference/modlist.toml");
        let parsed: toml::Value = toml::from_str(&modlist_toml).expect("modlist.toml parses");
        assert_eq!(
            parsed.get("description").and_then(toml::Value::as_str),
            Some("BG2EE with the fixpack, the UI mod and tweaks")
        );
        let unresolved_mods = parsed
            .get("unresolved_mods")
            .and_then(toml::Value::as_array)
            .expect("unresolved_mods present")
            .iter()
            .map(|value| value.as_str().expect("string entry").to_string())
            .collect::<Vec<_>>();
        assert_eq!(unresolved_mods, vec!["Some Mod".to_string()]);
    }

    #[test]
    fn reference_modlist_carries_the_game_version_when_tagged() {
        let tagged_json = r#"{
            "format_version": 1,
            "game_install": "BGEE",
            "game_version": "2.6",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component: 1.0" }
        }"#;
        let code =
            crate::app::modlist_share::encode_share_payload_text(tagged_json).expect("encode");
        let bytes = build_biolist(&code).expect("build biolist");
        let modlist_toml = entry_text(&bytes, "reference/modlist.toml");
        let parsed: toml::Value = toml::from_str(&modlist_toml).expect("modlist.toml parses");
        assert_eq!(
            parsed.get("game_version").and_then(toml::Value::as_str),
            Some("2.6")
        );

        let untagged_code =
            crate::app::modlist_share::encode_share_payload_text(&minimal_bgee_only_payload_json())
                .expect("encode");
        let bytes = build_biolist(&untagged_code).expect("build biolist");
        let modlist_toml = entry_text(&bytes, "reference/modlist.toml");
        let parsed: toml::Value = toml::from_str(&modlist_toml).expect("modlist.toml parses");
        assert!(parsed.get("game_version").is_none());
    }

    #[test]
    fn build_biolist_omits_empty_sections() {
        let code =
            crate::app::modlist_share::encode_share_payload_text(&minimal_bgee_only_payload_json())
                .expect("encode");

        let bytes = build_biolist(&code).expect("build biolist");
        let names = entry_names(&bytes);

        assert_eq!(
            names,
            vec![
                "reference/README.txt".to_string(),
                "reference/modlist.toml".to_string(),
                "reference/weidu/BGEE.log".to_string(),
                "share-code.txt".to_string(),
            ]
        );

        let modlist_toml = entry_text(&bytes, "reference/modlist.toml");
        let parsed: toml::Value = toml::from_str(&modlist_toml).expect("modlist.toml parses");
        assert!(parsed.get("unresolved_mods").is_none());
    }

    #[test]
    fn iwdee_biolist_reference_is_iwdee_log() {
        let code = crate::app::modlist_share::encode_share_payload_text(
            &minimal_iwdee_only_payload_json(),
        )
        .expect("encode");

        let bytes = build_biolist(&code).expect("build biolist");
        let names = entry_names(&bytes);

        assert!(names.contains(&"reference/weidu/IWDEE.log".to_string()));
        assert!(!names.contains(&"reference/weidu/BGEE.log".to_string()));
        assert_eq!(
            entry_text(&bytes, "reference/weidu/IWDEE.log"),
            "~MOD/MOD.TP2~ #0 #0 // A component: 1.0"
        );
    }

    #[test]
    fn bgee_biolist_reference_is_still_bgee_log() {
        let code =
            crate::app::modlist_share::encode_share_payload_text(&minimal_bgee_only_payload_json())
                .expect("encode");

        let bytes = build_biolist(&code).expect("build biolist");
        let names = entry_names(&bytes);

        assert!(names.contains(&"reference/weidu/BGEE.log".to_string()));
        assert!(!names.contains(&"reference/weidu/IWDEE.log".to_string()));
    }

    #[test]
    fn config_folder_falls_back_for_dot_and_empty_tp2() {
        assert_eq!(config_folder_name(".."), "mod");
        assert_eq!(config_folder_name(""), "mod");
    }

    #[test]
    fn config_paths_are_sanitized() {
        assert_eq!(
            config_relative_path("..\\..\\evil/../x.ini").as_deref(),
            Some("evil/x.ini")
        );
        assert_eq!(config_relative_path("C:\\x").as_deref(), Some("C_/x"));
    }

    #[test]
    fn write_then_read_round_trips_the_code() {
        let code =
            crate::app::modlist_share::encode_share_payload_text(&minimal_bgee_only_payload_json())
                .expect("encode");
        let path = unique_temp_path("round_trip");
        let _guard = TempFileGuard(path.clone());

        write_biolist(&code, &path).expect("write");
        let read_back = read_share_code(&path).expect("read");

        assert_eq!(read_back, code.trim());
    }

    #[test]
    fn read_share_code_from_bytes_round_trips_a_built_biolist() {
        let code =
            crate::app::modlist_share::encode_share_payload_text(&minimal_bgee_only_payload_json())
                .expect("encode");
        let bytes = build_biolist(&code).expect("build biolist");

        let read_back = read_share_code_from_bytes(&bytes).expect("read");

        assert_eq!(read_back, code.trim());
    }

    #[test]
    fn read_share_code_rejects_a_non_zip_file() {
        let path = unique_temp_path("not_a_zip");
        let _guard = TempFileGuard(path.clone());
        std::fs::write(&path, b"not a zip file").expect("write");

        let err = read_share_code(&path).expect_err("must reject");
        assert_eq!(err, NOT_A_MODLIST_FILE);
    }

    #[test]
    fn read_share_code_rejects_a_zip_without_the_entry() {
        let path = unique_temp_path("no_entry");
        let _guard = TempFileGuard(path.clone());

        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        write_entry(&mut writer, "not-the-entry.txt", b"hello").expect("write entry");
        let bytes = writer.finish().expect("finish").into_inner();
        std::fs::write(&path, bytes).expect("write file");

        let err = read_share_code(&path).expect_err("must reject");
        assert_eq!(err, NOT_A_MODLIST_FILE);
    }

    #[test]
    fn read_share_code_rejects_an_oversized_entry() {
        let path = unique_temp_path("oversized_entry");
        let _guard = TempFileGuard(path.clone());

        let oversized = vec![b'a'; usize::try_from(MAX_SHARE_CODE_ENTRY_BYTES).unwrap() + 1];
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        write_entry(&mut writer, SHARE_CODE_ENTRY, &oversized).expect("write entry");
        let bytes = writer.finish().expect("finish").into_inner();
        std::fs::write(&path, bytes).expect("write file");

        let err = read_share_code(&path).expect_err("must reject");
        assert_eq!(err, NOT_A_MODLIST_FILE);
    }

    #[test]
    fn suggested_file_name_sanitizes_and_falls_back() {
        assert_eq!(
            suggested_file_name("Polished: BG2EE / Tactical*"),
            "Polished_ BG2EE _ Tactical_.biolist"
        );
        assert_eq!(suggested_file_name("   "), "modlist.biolist");
        assert_eq!(suggested_file_name(""), "modlist.biolist");
    }
}
