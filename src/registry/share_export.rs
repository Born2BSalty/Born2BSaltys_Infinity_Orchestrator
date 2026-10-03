// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use serde_json::Value;

use crate::app::modlist_share::ForkAncestor;
use crate::app::state::WizardState;
use crate::registry::model::ModlistEntry;

const SHARE_CODE_PREFIX: &str = "BIO-MODLIST-V1:";

const ARCHIVE_META_KEY: &str = "archive_meta";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveMeta {
    pub name: String,

    pub size: u64,

    pub hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShareMeta {
    pub allow_auto_install: bool,

    pub name: Option<String>,

    pub author: Option<String>,

    pub description: Option<String>,

    pub(crate) forked_from: Vec<ForkAncestor>,

    pub archive_meta: Vec<ArchiveMeta>,
}

impl ShareMeta {
    #[must_use]
    pub fn from_entry(entry: &ModlistEntry, allow_auto_install: bool) -> Self {
        let name = Some(entry.name.trim().to_string()).filter(|s| !s.is_empty());
        let author = entry
            .author
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let description = entry
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        Self {
            allow_auto_install,
            name,
            author,
            description,
            forked_from: entry.forked_from.clone(),

            archive_meta: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_archive_meta(mut self, archive_meta: Vec<ArchiveMeta>) -> Self {
        self.archive_meta = archive_meta;
        self
    }
}

pub fn pack_meta(wizard_state: &WizardState, meta: &ShareMeta) -> Result<String, String> {
    let base = crate::app::modlist_share::export_modlist_share_code(wizard_state)?;
    pack_meta_onto_base(&base, meta)
}

pub fn pack_meta_for_completed_install(
    wizard_state: &WizardState,
    meta: &ShareMeta,
) -> Result<String, String> {
    let base =
        crate::app::modlist_share::export_modlist_share_code_for_completed_install(wizard_state)?;
    pack_meta_onto_base(&base, meta)
}

fn pack_meta_onto_base(base: &str, meta: &ShareMeta) -> Result<String, String> {
    rewrite_payload(base, |obj| {
        obj.insert(
            "allow_auto_install".to_string(),
            Value::Bool(meta.allow_auto_install),
        );
        if let Some(name) = &meta.name {
            obj.insert("name".to_string(), Value::String(name.clone()));
        }
        if let Some(author) = &meta.author {
            obj.insert("author".to_string(), Value::String(author.clone()));
        }
        if let Some(description) = &meta.description {
            obj.insert(
                "description".to_string(),
                Value::String(description.clone()),
            );
        }
        if !meta.forked_from.is_empty() {
            obj.insert("forked_from".to_string(), lineage_json(&meta.forked_from));
        }
        insert_archive_meta(obj, &meta.archive_meta);
    })
}

fn rewrite_payload(
    code: &str,
    edit: impl FnOnce(&mut serde_json::Map<String, Value>),
) -> Result<String, String> {
    let encoded = code
        .trim()
        .strip_prefix(SHARE_CODE_PREFIX)
        .ok_or_else(|| "share code did not start with BIO-MODLIST-V1:".to_string())?;
    let compressed = base64url_decode(encoded)?;
    let json_bytes = zlib_decompress(&compressed)?;
    let mut payload: Value = serde_json::from_slice(&json_bytes)
        .map_err(|err| format!("share payload was not valid JSON: {err}"))?;

    let obj = payload
        .as_object_mut()
        .ok_or_else(|| "share payload was not a JSON object".to_string())?;
    edit(obj);

    let out_bytes =
        serde_json::to_vec(&payload).map_err(|err| format!("re-serialize failed: {err}"))?;
    let recompressed = zlib_compress(&out_bytes)?;
    Ok(format!(
        "{SHARE_CODE_PREFIX}{}",
        base64url_encode(&recompressed)
    ))
}

fn lineage_json(forked_from: &[ForkAncestor]) -> Value {
    Value::Array(
        forked_from
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "author": a.author,
                })
            })
            .collect(),
    )
}

pub fn set_allow_auto_install(code: &str, allow_auto_install: bool) -> Result<String, String> {
    rewrite_payload(code, |obj| {
        obj.insert(
            "allow_auto_install".to_string(),
            Value::Bool(allow_auto_install),
        );
    })
}

pub fn set_packed_name(code: &str, name: &str) -> Result<String, String> {
    rewrite_payload(code, |obj| {
        obj.insert("name".to_string(), Value::String(name.to_string()));
    })
}

pub fn set_packed_identity(code: &str, meta: &ShareMeta) -> Result<String, String> {
    rewrite_payload(code, |obj| {
        if let Some(name) = &meta.name {
            obj.insert("name".to_string(), Value::String(name.clone()));
        }
        match &meta.author {
            Some(author) => {
                obj.insert("author".to_string(), Value::String(author.clone()));
            }
            None => {
                obj.remove("author");
            }
        }
        match &meta.description {
            Some(description) => {
                obj.insert(
                    "description".to_string(),
                    Value::String(description.clone()),
                );
            }
            None => {
                obj.remove("description");
            }
        }
        if meta.forked_from.is_empty() {
            obj.remove("forked_from");
        } else {
            obj.insert("forked_from".to_string(), lineage_json(&meta.forked_from));
        }
    })
}

fn insert_archive_meta(obj: &mut serde_json::Map<String, Value>, archive_meta: &[ArchiveMeta]) {
    if archive_meta.is_empty() {
        return;
    }
    let arr = archive_meta
        .iter()
        .map(|m| {
            serde_json::json!({
                "name": m.name,
                "size": m.size,
                "hash": m.hash,
            })
        })
        .collect::<Vec<_>>();
    obj.insert(ARCHIVE_META_KEY.to_string(), Value::Array(arr));
}

pub fn bake_archive_meta_into_code(
    code: &str,
    archive_meta: &[ArchiveMeta],
) -> Result<String, String> {
    rewrite_payload(code, |obj| insert_archive_meta(obj, archive_meta))
}

pub fn decode_archive_meta(code: &str) -> Result<Vec<ArchiveMeta>, String> {
    let encoded = code
        .trim()
        .strip_prefix(SHARE_CODE_PREFIX)
        .ok_or_else(|| "share code did not start with BIO-MODLIST-V1:".to_string())?;
    let compressed = base64url_decode(encoded)?;
    let json_bytes = zlib_decompress(&compressed)?;
    let payload: Value = serde_json::from_slice(&json_bytes)
        .map_err(|err| format!("share payload was not valid JSON: {err}"))?;
    let Some(arr) = payload.get(ARCHIVE_META_KEY).and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(arr.len());
    for el in arr {
        let (Some(name), Some(size), Some(hash)) = (
            el.get("name").and_then(Value::as_str),
            el.get("size").and_then(Value::as_u64),
            el.get("hash").and_then(Value::as_str),
        ) else {
            continue;
        };
        if name.is_empty() || hash.is_empty() {
            continue;
        }
        out.push(ArchiveMeta {
            name: name.to_string(),
            size,
            hash: hash.to_string(),
        });
    }
    Ok(out)
}

#[must_use]
pub fn build_archive_meta_for_assets(
    assets: &[crate::app::state::Step2UpdateAsset],
    archive_dir: &std::path::Path,
) -> Vec<ArchiveMeta> {
    use crate::app::app_step2_update_download::archive_file_name;
    let mut out = Vec::new();
    for asset in assets {
        let name = archive_file_name(asset);
        let path = archive_dir.join(&name);
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let size = meta.len();

        if let Ok(hash) = crate::install_runtime::archive_store::hash_file(&path) {
            out.push(ArchiveMeta { name, size, hash });
        }
    }
    out
}

#[must_use]
pub fn build_archive_meta_from_install_lock(
    destination: &str,
    archive_dir: &std::path::Path,
) -> Vec<ArchiveMeta> {
    use crate::install_runtime::archive_store::{InstallArchiveLock, stored_filename};
    let lock = InstallArchiveLock::load(destination);
    let mut out = Vec::with_capacity(lock.resolved.len());
    for (name, hash) in &lock.resolved {
        let stored = archive_dir.join(stored_filename(name, hash));
        let deterministic = archive_dir.join(name);
        let size = std::fs::metadata(&stored)
            .or_else(|_| std::fs::metadata(&deterministic))
            .ok()
            .filter(std::fs::Metadata::is_file)
            .map(|m| m.len());
        let Some(size) = size else {
            continue;
        };
        out.push(ArchiveMeta {
            name: name.clone(),
            size,
            hash: hash.clone(),
        });
    }
    out
}

#[must_use]
pub(crate) fn build_archive_meta_from_installed_refs(
    wizard_state: &WizardState,
    lookup: &crate::app::app_step2_update_source_refs::InstalledRefLookup,
) -> Vec<ArchiveMeta> {
    let checked_tp2s = wizard_state
        .step2
        .bgee_mods
        .iter()
        .chain(wizard_state.step2.bg2ee_mods.iter())
        .filter(|mod_state| {
            mod_state
                .components
                .iter()
                .any(|component| component.checked)
        })
        .map(|mod_state| crate::app::mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file))
        .collect::<std::collections::BTreeSet<_>>();
    checked_tp2s
        .iter()
        .filter_map(|tp2| lookup.archive(tp2))
        .map(|record| ArchiveMeta {
            name: record.name.clone(),
            size: record.size,
            hash: record.hash.clone(),
        })
        .collect()
}

#[must_use]
pub fn archive_meta_for_draft(wizard_state: &WizardState) -> Vec<ArchiveMeta> {
    let lookup = crate::app::app_step2_update_source_refs::InstalledRefLookup::load(
        wizard_state.step1.mods_folder.trim(),
    );
    build_archive_meta_from_installed_refs(wizard_state, &lookup)
}

fn zlib_compress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|err| format!("zlib compress failed: {err}"))?;
    encoder
        .finish()
        .map_err(|err| format!("zlib compress finish failed: {err}"))
}

fn zlib_decompress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = ZlibDecoder::new(bytes);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|err| format!("zlib decompress failed: {err}"))?;
    Ok(out)
}

fn base64url_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        }
    }
    out
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
            _ => return Err("share code contains invalid base64url characters".to_string()),
        }
    }
    let remainder = values.len() % 4;
    if remainder == 1 {
        return Err("share code base64url length is invalid".to_string());
    }
    if remainder != 0 {
        values.extend(std::iter::repeat_n(64, 4 - remainder));
    }
    let mut out = Vec::with_capacity(values.len() / 4 * 3);
    for chunk in values.chunks(4) {
        let mut pad = 0;
        for value in chunk {
            if *value == 64 {
                pad += 1;
            }
        }
        if pad > 2 || chunk[..4 - pad].contains(&64) {
            return Err("share code base64 padding is invalid".to_string());
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
    use serde_json::json;

    #[test]
    fn base64url_round_trips_arbitrary_bytes() {
        for case in [
            &b""[..],
            &b"a"[..],
            &b"ab"[..],
            &b"abc"[..],
            &b"abcd"[..],
            &[0u8, 255, 1, 254, 127, 128][..],
        ] {
            let enc = base64url_encode(case);
            assert!(
                !enc.contains('='),
                "base64url must not emit '=' padding (BIO-format parity)"
            );
            let dec = base64url_decode(&enc).expect("decode");
            assert_eq!(dec, case, "lossless round-trip");
        }
    }

    #[test]
    fn zlib_round_trips() {
        let data = b"BIO-MODLIST-V1 payload bytes \x00\x01\x02 with binary";
        let c = zlib_compress(data).expect("compress");
        let d = zlib_decompress(&c).expect("decompress");
        assert_eq!(d, data.to_vec());
    }

    fn pack_meta_envelope_only(base: &str, meta: &ShareMeta) -> Result<String, String> {
        let encoded = base
            .trim()
            .strip_prefix(SHARE_CODE_PREFIX)
            .ok_or_else(|| "no prefix".to_string())?;
        let compressed = base64url_decode(encoded)?;
        let json_bytes = zlib_decompress(&compressed)?;
        let mut payload: Value = serde_json::from_slice(&json_bytes).map_err(|e| e.to_string())?;
        let obj = payload.as_object_mut().ok_or("not object")?;
        obj.insert(
            "allow_auto_install".to_string(),
            Value::Bool(meta.allow_auto_install),
        );
        if let Some(name) = &meta.name {
            obj.insert("name".to_string(), Value::String(name.clone()));
        }
        if let Some(author) = &meta.author {
            obj.insert("author".to_string(), Value::String(author.clone()));
        }
        if let Some(description) = &meta.description {
            obj.insert(
                "description".to_string(),
                Value::String(description.clone()),
            );
        }
        if !meta.forked_from.is_empty() {
            let lineage = meta
                .forked_from
                .iter()
                .map(|a| json!({ "name": a.name, "author": a.author }))
                .collect::<Vec<_>>();
            obj.insert("forked_from".to_string(), Value::Array(lineage));
        }

        insert_archive_meta(obj, &meta.archive_meta);
        let out_bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
        Ok(format!(
            "{SHARE_CODE_PREFIX}{}",
            base64url_encode(&zlib_compress(&out_bytes)?)
        ))
    }

    fn make_base(payload: &Value) -> String {
        let bytes = serde_json::to_vec(payload).unwrap();
        format!(
            "{SHARE_CODE_PREFIX}{}",
            base64url_encode(&zlib_compress(&bytes).unwrap())
        )
    }

    fn decode_payload(code: &str) -> Value {
        let encoded = code.strip_prefix(SHARE_CODE_PREFIX).unwrap();
        let bytes = zlib_decompress(&base64url_decode(encoded).unwrap()).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn pack_meta_injects_false_bit_and_omits_absent_provenance() {
        let base = make_base(&json!({
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "EET",
            "install_mode": "build_from_scanned_mods",
        }));
        let meta = ShareMeta {
            allow_auto_install: false,
            name: None,
            author: None,
            description: None,
            forked_from: vec![],
            archive_meta: vec![],
        };
        let code = pack_meta_envelope_only(&base, &meta).expect("pack");
        let v = decode_payload(&code);

        assert_eq!(v["allow_auto_install"], json!(false));

        assert!(v.get("name").is_none());
        assert!(v.get("author").is_none());
        assert!(v.get("description").is_none());
        assert!(v.get("forked_from").is_none());

        assert_eq!(v["game_install"], json!("EET"));
        assert_eq!(v["bio_version"], json!("0.1.0-test"));
    }

    #[test]
    fn pack_meta_injects_true_bit_and_full_provenance() {
        let base = make_base(&json!({
            "format_version": 1,
            "game_install": "BGEE",
            "install_mode": "build_from_scanned_mods",
        }));
        let meta = ShareMeta {
            allow_auto_install: true,
            name: Some("Polished BG2EE".to_string()),
            author: Some("@b2bs".to_string()),
            description: Some("BG2EE with the fixpack".to_string()),
            forked_from: vec![
                ForkAncestor {
                    name: "EET Basics".to_string(),
                    author: "@olim".to_string(),
                },
                ForkAncestor {
                    name: "EET Tactical".to_string(),
                    author: "@b2bs".to_string(),
                },
            ],
            archive_meta: vec![],
        };
        let code = pack_meta_envelope_only(&base, &meta).expect("pack");
        let v = decode_payload(&code);
        assert_eq!(v["allow_auto_install"], json!(true));
        assert_eq!(v["name"], json!("Polished BG2EE"));
        assert_eq!(v["author"], json!("@b2bs"));
        assert_eq!(v["description"], json!("BG2EE with the fixpack"));

        assert_eq!(
            v["forked_from"],
            json!([
                { "name": "EET Basics", "author": "@olim" },
                { "name": "EET Tactical", "author": "@b2bs" },
            ])
        );
    }

    #[test]
    fn share_meta_from_entry_normalizes_empty_strings_to_none() {
        use crate::registry::model::{Game, ModlistEntry};
        let entry = ModlistEntry {
            id: "ABC".to_string(),
            name: "  Trimmed Name  ".to_string(),
            game: Game::EET,
            author: Some("   ".to_string()),
            description: Some("   ".to_string()),
            forked_from: vec![ForkAncestor {
                name: "Root".to_string(),
                author: "@root".to_string(),
            }],
            ..Default::default()
        };
        let meta = ShareMeta::from_entry(&entry, false);
        assert!(!meta.allow_auto_install);
        assert_eq!(meta.name.as_deref(), Some("Trimmed Name"));
        assert_eq!(
            meta.author, None,
            "whitespace author ⇒ omitted (SPEC §13.3)"
        );
        assert_eq!(meta.description, None, "whitespace description ⇒ omitted");
        assert_eq!(meta.forked_from.len(), 1);

        let entry2 = ModlistEntry {
            name: String::new(),
            ..entry
        };
        assert_eq!(ShareMeta::from_entry(&entry2, true).name, None);
    }

    #[test]
    fn injection_is_idempotent_on_reinjection() {
        let base = make_base(&json!({ "format_version": 1, "game_install": "EET" }));
        let first = pack_meta_envelope_only(
            &base,
            &ShareMeta {
                allow_auto_install: false,
                name: Some("X".to_string()),
                author: None,
                description: None,
                forked_from: vec![],
                archive_meta: vec![],
            },
        )
        .unwrap();
        let second = pack_meta_envelope_only(
            &first,
            &ShareMeta {
                allow_auto_install: true,
                name: Some("X".to_string()),
                author: Some("@me".to_string()),
                description: Some("Now with a description".to_string()),
                forked_from: vec![],
                archive_meta: vec![],
            },
        )
        .unwrap();
        let v = decode_payload(&second);
        assert_eq!(v["allow_auto_install"], json!(true));
        assert_eq!(v["author"], json!("@me"));

        assert!(v.is_object());
        assert_eq!(v["game_install"], json!("EET"));
    }

    #[test]
    fn set_allow_auto_install_flips_bit_and_preserves_all_other_keys() {
        let original = make_base(&json!({
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "EET",
            "install_mode": "build_from_scanned_mods",
            "weidu_logs": { "bgee": "// log\n~A~ #0 #1", "bg2ee": null },
            "allow_auto_install": true,
            "name": "Tactical EET 2026",
            "author": "@sharer",
            "forked_from": [{ "name": "Root", "author": "@root" }],
        }));

        let at_start = set_allow_auto_install(&original, false).expect("decode-flip ok");
        let v = decode_payload(&at_start);
        assert_eq!(
            v["allow_auto_install"],
            json!(false),
            "install-start code carries allow_auto_install=false (SPEC §13.13)"
        );

        assert_eq!(v["name"], json!("Tactical EET 2026"));
        assert_eq!(v["author"], json!("@sharer"));
        assert_eq!(
            v["forked_from"],
            json!([{ "name": "Root", "author": "@root" }])
        );
        assert_eq!(v["game_install"], json!("EET"));
        assert_eq!(v["install_mode"], json!("build_from_scanned_mods"));
        assert_eq!(v["weidu_logs"]["bgee"], json!("// log\n~A~ #0 #1"));
        assert_eq!(v["bio_version"], json!("0.1.0-test"));

        let at_clean = set_allow_auto_install(&at_start, true).expect("re-flip ok");
        let v2 = decode_payload(&at_clean);
        assert_eq!(
            v2["allow_auto_install"],
            json!(true),
            "clean-exit rewrite carries allow_auto_install=true (SPEC §13.13)"
        );

        assert_eq!(v2["name"], json!("Tactical EET 2026"));
        assert_eq!(v2["author"], json!("@sharer"));
        assert_eq!(
            v2["forked_from"],
            json!([{ "name": "Root", "author": "@root" }])
        );
    }

    #[test]
    fn set_allow_auto_install_adds_the_bit_when_absent_pre_redesign_code() {
        let pre_redesign = make_base(&json!({
            "format_version": 1,
            "game_install": "BGEE",
            "install_mode": "install_exactly_from_weidu_logs",
        }));
        assert!(
            decode_payload(&pre_redesign)
                .get("allow_auto_install")
                .is_none(),
            "precondition: the source code has no allow_auto_install key"
        );
        let flipped = set_allow_auto_install(&pre_redesign, false).expect("ok");
        let v = decode_payload(&flipped);
        assert_eq!(v["allow_auto_install"], json!(false), "key added");
        assert_eq!(v["game_install"], json!("BGEE"), "rest opaque-preserved");
        assert_eq!(v["install_mode"], json!("install_exactly_from_weidu_logs"));
    }

    #[test]
    fn set_allow_auto_install_errs_on_non_bio_code_so_caller_can_fallback() {
        assert!(set_allow_auto_install("not a share code", false).is_err());
        assert!(
            set_allow_auto_install("BIO-MODLIST-V1:!!!not-base64!!!", true).is_err(),
            "a prefixed-but-undecodable code Errs so the caller falls back \
             to verbatim"
        );
    }

    #[test]
    fn set_allow_auto_install_output_is_bio_decoder_round_trippable() {
        let original =
            make_base(&json!({ "format_version": 1, "game_install": "BG2EE", "x": [1, 2, 3] }));
        let out = set_allow_auto_install(&original, true).expect("ok");
        assert!(out.starts_with(SHARE_CODE_PREFIX));
        assert!(
            !out.contains('='),
            "URL-safe base64, no '=' padding (BIO-format parity)"
        );
        let v = decode_payload(&out);
        assert_eq!(v["allow_auto_install"], json!(true));
        assert_eq!(v["game_install"], json!("BG2EE"));
        assert_eq!(v["x"], json!([1, 2, 3]));
    }

    #[test]
    fn set_packed_name_overwrites_the_name_and_preserves_all_other_keys() {
        let original = make_base(&json!({
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "EET",
            "install_mode": "build_from_scanned_mods",
            "allow_auto_install": true,
            "name": "Old Name",
            "author": "@sharer",
            "forked_from": [{ "name": "Root", "author": "@root" }],
        }));

        let renamed = set_packed_name(&original, "My Renamed List").expect("rename ok");
        let v = decode_payload(&renamed);

        assert_eq!(v["name"], json!("My Renamed List"));
        assert_eq!(v["author"], json!("@sharer"));
        assert_eq!(v["allow_auto_install"], json!(true));
        assert_eq!(v["game_install"], json!("EET"));
        assert_eq!(v["install_mode"], json!("build_from_scanned_mods"));
        assert_eq!(v["bio_version"], json!("0.1.0-test"));
        assert_eq!(
            v["forked_from"],
            json!([{ "name": "Root", "author": "@root" }])
        );
    }

    #[test]
    fn set_packed_name_errs_on_non_bio_code() {
        assert!(set_packed_name("not a share code", "X").is_err());
        assert!(set_packed_name("BIO-MODLIST-V1:!!!not-base64!!!", "X").is_err());
    }

    #[test]
    fn pack_meta_injects_the_description_and_omits_it_when_absent() {
        let base = make_base(&json!({
            "format_version": 1,
            "game_install": "BGEE",
            "install_mode": "build_from_scanned_mods",
        }));
        let with_description = ShareMeta {
            allow_auto_install: true,
            name: None,
            author: None,
            description: Some("BG2EE with the fixpack".to_string()),
            forked_from: vec![],
            archive_meta: vec![],
        };
        let code = pack_meta_envelope_only(&base, &with_description).expect("pack");
        let v = decode_payload(&code);
        assert_eq!(v["description"], json!("BG2EE with the fixpack"));

        let without_description = ShareMeta {
            description: None,
            ..with_description
        };
        let code = pack_meta_envelope_only(&base, &without_description).expect("pack");
        let v = decode_payload(&code);
        assert!(v.get("description").is_none());
    }

    #[test]
    fn set_packed_identity_errs_on_non_bio_code() {
        let meta = ShareMeta {
            allow_auto_install: false,
            name: Some("X".to_string()),
            author: None,
            description: None,
            forked_from: vec![],
            archive_meta: vec![],
        };
        assert!(set_packed_identity("not a share code", &meta).is_err());
        assert!(set_packed_identity("BIO-MODLIST-V1:!!!not-base64!!!", &meta).is_err());
    }

    fn am(name: &str, size: u64, hash: &str) -> ArchiveMeta {
        ArchiveMeta {
            name: name.to_string(),
            size,
            hash: hash.to_string(),
        }
    }

    #[test]
    fn pack_meta_bakes_archive_meta_and_decode_recovers_it_byte_exact() {
        let base = make_base(&json!({ "format_version": 1, "game_install": "EET" }));
        let meta = ShareMeta {
            allow_auto_install: false,
            name: Some("Tactical".to_string()),
            author: None,
            description: None,
            forked_from: vec![],
            archive_meta: vec![
                am("a__github__v1.zip", 17, "deadbeef00000000deadbeef00000000"),
                am("b__weasel__v2.7z", 4096, "0123456789abcdef0123456789abcdef"),
            ],
        };
        let code = pack_meta_envelope_only(&base, &meta).expect("pack");

        let v = decode_payload(&code);
        assert_eq!(
            v["archive_meta"],
            json!([
                { "name": "a__github__v1.zip", "size": 17, "hash": "deadbeef00000000deadbeef00000000" },
                { "name": "b__weasel__v2.7z", "size": 4096, "hash": "0123456789abcdef0123456789abcdef" },
            ]),
            "archive_meta is a flat sibling key (like forked_from), NOT a wrapper"
        );

        let decoded = decode_archive_meta(&code).expect("decode");
        assert_eq!(decoded, meta.archive_meta, "round-trip byte-exact");
    }

    #[test]
    fn empty_archive_meta_omits_the_key_and_decodes_as_today() {
        let base = make_base(&json!({ "format_version": 1, "game_install": "BGEE" }));
        let meta = ShareMeta {
            allow_auto_install: true,
            name: None,
            author: None,
            description: None,
            forked_from: vec![],
            archive_meta: vec![],
        };
        let code = pack_meta_envelope_only(&base, &meta).expect("pack");
        let v = decode_payload(&code);
        assert!(
            v.get("archive_meta").is_none(),
            "empty ⇒ key omitted (today's-behavior fallback, no schema noise)"
        );
        assert_eq!(
            decode_archive_meta(&code).expect("decode"),
            Vec::<ArchiveMeta>::new(),
            "no key ⇒ empty vec, NOT an error (backward-compatible)"
        );

        let raw_old = make_base(&json!({ "format_version": 1, "game_install": "EET" }));
        assert_eq!(
            decode_archive_meta(&raw_old).expect("old code decodes"),
            Vec::<ArchiveMeta>::new(),
            "pre-redesign code (no archive_meta) ⇒ empty vec, not an error"
        );
    }

    #[test]
    fn bake_archive_meta_into_code_preserves_every_other_key_verbatim() {
        let original = make_base(&json!({
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "EET",
            "weidu_logs": { "bgee": "// log\n~A~ #0", "bg2ee": null },
            "allow_auto_install": false,
            "name": "Tactical EET 2026",
            "author": "@sharer",
            "forked_from": [{ "name": "Root", "author": "@root" }],
        }));
        let metas = vec![am("m__gh__v1.zip", 123, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")];
        let out = bake_archive_meta_into_code(&original, &metas).expect("bake ok");
        let v = decode_payload(&out);

        assert_eq!(decode_archive_meta(&out).unwrap(), metas);

        assert_eq!(v["allow_auto_install"], json!(false));
        assert_eq!(v["name"], json!("Tactical EET 2026"));
        assert_eq!(v["author"], json!("@sharer"));
        assert_eq!(
            v["forked_from"],
            json!([{ "name": "Root", "author": "@root" }])
        );
        assert_eq!(v["game_install"], json!("EET"));
        assert_eq!(v["weidu_logs"]["bgee"], json!("// log\n~A~ #0"));
        assert_eq!(v["bio_version"], json!("0.1.0-test"));
    }

    #[test]
    fn archive_meta_survives_set_allow_auto_install_flip() {
        let base = make_base(&json!({ "format_version": 1, "game_install": "EET" }));
        let metas = vec![
            am("x__gh__v1.zip", 10, "11111111111111111111111111111111"),
            am("y__wm__v2.zip", 20, "22222222222222222222222222222222"),
        ];
        let with_meta = bake_archive_meta_into_code(&base, &metas).expect("bake");
        let at_start = set_allow_auto_install(&with_meta, false).expect("flip false");
        assert_eq!(
            decode_archive_meta(&at_start).unwrap(),
            metas,
            "archive_meta survives the install-start false-flip"
        );
        let at_clean = set_allow_auto_install(&at_start, true).expect("flip true");
        assert_eq!(
            decode_archive_meta(&at_clean).unwrap(),
            metas,
            "archive_meta survives the clean-exit true-flip too (every key \
             but the bit is opaque-preserved)"
        );
        assert_eq!(decode_payload(&at_clean)["allow_auto_install"], json!(true));
    }

    #[test]
    fn bake_empty_archive_meta_is_a_lossless_reencode_and_errs_on_non_bio() {
        let base = make_base(&json!({ "format_version": 1, "game_install": "BGEE", "k": 9 }));
        let out = bake_archive_meta_into_code(&base, &[]).expect("empty ⇒ lossless re-encode");
        let v = decode_payload(&out);
        assert!(v.get("archive_meta").is_none(), "empty ⇒ key omitted");
        assert_eq!(v["k"], json!(9), "payload otherwise identical");
        assert!(bake_archive_meta_into_code("not a code", &[]).is_err());
        assert!(
            bake_archive_meta_into_code("BIO-MODLIST-V1:!!!bad!!!", &[]).is_err(),
            "a prefixed-but-undecodable code Errs (caller persists verbatim)"
        );
    }

    #[test]
    fn decode_archive_meta_skips_malformed_elements_not_fails() {
        let code = make_base(&json!({
            "format_version": 1,
            "archive_meta": [
                { "name": "ok.zip", "size": 5, "hash": "ffffffffffffffffffffffffffffffff" },
                { "name": "missing-size.zip", "hash": "00000000000000000000000000000000" },
                { "size": 7, "hash": "11111111111111111111111111111111" },
                { "name": "", "size": 1, "hash": "22222222222222222222222222222222" },
                "not even an object",
                { "name": "good2.7z", "size": 99, "hash": "33333333333333333333333333333333" },
            ],
        }));
        let decoded = decode_archive_meta(&code).expect("never fails on malformed elements");
        assert_eq!(
            decoded,
            vec![
                am("ok.zip", 5, "ffffffffffffffffffffffffffffffff"),
                am("good2.7z", 99, "33333333333333333333333333333333"),
            ],
            "only the well-formed elements survive; the rest are skipped \
             (a redundant re-download at worst, never a blocked install)"
        );
    }

    #[test]
    fn build_archive_meta_for_assets_hashes_only_on_disk_archives() {
        use crate::app::app_step2_update_download::archive_file_name;
        use crate::app::state::Step2UpdateAsset;
        use std::sync::atomic::{AtomicU64, Ordering};
        static C: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "bio_build_archive_meta_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let mk = |tp: &str, src: &str, tag: &str, name: &str| Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: tp.to_string(),
            label: tp.to_string(),
            source_id: src.to_string(),
            tag: tag.to_string(),
            asset_name: name.to_string(),
            asset_url: format!("https://example/{name}"),
            installed_source_ref: None,
        };
        let present = mk("AMOD/AMOD.TP2", "github", "v1", "A.zip");
        let absent = mk("BMOD/BMOD.TP2", "weasel", "v2", "B.zip");
        let present_name = archive_file_name(&present);
        std::fs::write(dir.join(&present_name), b"ARCHIVE-BYTES-123").unwrap();

        let metas = build_archive_meta_for_assets(&[present, absent], &dir);
        assert_eq!(metas.len(), 1, "only the on-disk archive is hashed");
        assert_eq!(metas[0].name, present_name);
        assert_eq!(metas[0].size, "ARCHIVE-BYTES-123".len() as u64);

        assert_eq!(
            metas[0].hash,
            crate::install_runtime::archive_store::hash_file(&dir.join(&present_name)).unwrap(),
            "the baked hash is the SAME stable FNV-1a-128 the content-\
             addressed store uses (one hashing path, zero drift)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn step2_mod(tp_file: &str, checked: bool) -> crate::app::state::Step2ModState {
        crate::app::state::Step2ModState {
            name: tp_file.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: format!("{tp_file}/{tp_file}.tp2"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked,
            hidden_components: Vec::new(),
            components: vec![crate::app::state::Step2ComponentState {
                component_id: "0".to_string(),
                label: "0".to_string(),
                weidu_group: None,
                collapsible_group: None,
                collapsible_group_is_umbrella: false,
                collapsible_group_combinable: false,
                raw_line: String::new(),
                prompt_summary: None,
                prompt_events: Vec::new(),
                is_meta_mode_component: false,
                disabled: false,
                compat_kind: None,
                compat_source: None,
                compat_related_mod: None,
                compat_related_component: None,
                compat_graph: None,
                compat_evidence: None,
                disabled_reason: None,
                checked,
                selected_order: checked.then_some(1),
            }],
        }
    }

    #[test]
    fn archive_meta_from_installed_refs_covers_checked_mods_only() {
        use crate::app::app_step2_update_source_refs::{
            InstalledArchiveRecord, InstalledRefLookup, ModSourceRefsFile,
        };
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![step2_mod("alpha", true), step2_mod("beta", false)];
        let mut refs_file = ModSourceRefsFile::default();
        refs_file.archives.insert(
            "alpha".to_string(),
            InstalledArchiveRecord {
                name: "alpha__primary__v19.zip".to_string(),
                size: 17,
                hash: "deadbeef00000000deadbeef00000000".to_string(),
                last_modified: None,
                etag: None,
            },
        );
        refs_file.archives.insert(
            "beta".to_string(),
            InstalledArchiveRecord {
                name: "beta__primary__v2.zip".to_string(),
                size: 29,
                hash: "0123456789abcdef0123456789abcdef".to_string(),
                last_modified: None,
                etag: None,
            },
        );

        let metas = build_archive_meta_from_installed_refs(
            &state,
            &InstalledRefLookup::from_files(None, refs_file),
        );

        assert_eq!(
            metas,
            vec![am(
                "alpha__primary__v19.zip",
                17,
                "deadbeef00000000deadbeef00000000"
            )]
        );
    }

    #[test]
    fn set_packed_identity_stamps_name_author_and_lineage_and_leaves_the_rest_verbatim() {
        let base = make_base(&json!({
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "EET",
            "install_mode": "build_from_scanned_mods",
            "allow_auto_install": true,
            "name": "EET Essentials",
            "author": "BIO Team",
            "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" },
            "archive_meta": [{ "name": "a.zip", "size": 7, "hash": "ff" }],
        }));
        let meta = ShareMeta {
            allow_auto_install: false,
            name: Some("EET Essentials +++".to_string()),
            author: Some("Xgatt".to_string()),
            description: Some("my fork".to_string()),
            forked_from: vec![ForkAncestor {
                name: "EET Essentials".to_string(),
                author: "BIO Team".to_string(),
            }],
            archive_meta: vec![],
        };

        let v = decode_payload(&set_packed_identity(&base, &meta).unwrap());

        assert_eq!(v["name"], json!("EET Essentials +++"));
        assert_eq!(v["author"], json!("Xgatt"));
        assert_eq!(v["description"], json!("my fork"));
        assert_eq!(
            v["forked_from"],
            json!([{ "name": "EET Essentials", "author": "BIO Team" }])
        );
        assert_eq!(v["allow_auto_install"], json!(true));
        assert_eq!(
            v["weidu_logs"],
            json!({ "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" })
        );
        assert_eq!(
            v["archive_meta"],
            json!([{ "name": "a.zip", "size": 7, "hash": "ff" }])
        );
    }

    #[test]
    fn set_packed_identity_removes_author_description_and_lineage_the_entry_lacks() {
        let base = make_base(&json!({
            "format_version": 1,
            "game_install": "EET",
            "install_mode": "build_from_scanned_mods",
            "name": "EET Essentials",
            "author": "BIO Team",
            "description": "the gallery's own",
            "forked_from": [{ "name": "Root", "author": "@root" }],
        }));
        let meta = ShareMeta {
            allow_auto_install: false,
            name: None,
            author: None,
            description: None,
            forked_from: vec![],
            archive_meta: vec![],
        };

        let v = decode_payload(&set_packed_identity(&base, &meta).unwrap());

        assert_eq!(v["name"], json!("EET Essentials"));
        assert!(v.get("author").is_none());
        assert!(v.get("description").is_none());
        assert!(v.get("forked_from").is_none());
    }
}
