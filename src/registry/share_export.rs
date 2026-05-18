// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `registry::share_export` — the **net-new orchestrator share-code
// generation envelope** (SPEC §13.3 "Generation mechanism (`pack_meta`)").
//
// Every BIO-MODLIST-V1 code Infinity Orchestrator emits is produced here so
// the four schema-additive sibling keys (`allow_auto_install` + the
// provenance trio `name` / `author` / `forked_from`) are always consistent.
// Per the CRITICAL DIRECTIVE carve-out #5 ("generation is not a BIO
// modification") this **composes** BIO's generator read-only and **never
// patches** `bio::app::modlist_share`:
//
//   1. `let base = bio::app::modlist_share::export_modlist_share_code(
//          &wizard_state)?` — BIO builds the canonical payload, unchanged.
//   2. `pack_meta` strips the `BIO-MODLIST-V1:` prefix, base64url-decodes,
//      zlib-inflates, parses to an **opaque `serde_json::Value`**, inserts
//      the four keys at the top level of that object, re-serializes,
//      zlib-deflates, base64url-encodes, and re-attaches the prefix.
//   3. The result is a single flat `BIO-MODLIST-V1` code that BIO's own
//      `preview_modlist_share_code` / `import_modlist_share_code` decode
//      unchanged (the four keys are the carve-out #5 `#[serde(default)]`
//      fields on `ModlistSharePayload` / `ModlistSharePreview`).
//
// **Why a net-new sibling, not a BIO edit.** BIO's envelope primitives
// (`base64url_encode` / `base64url_decode` / `zlib_compress` /
// `zlib_decompress` / `decode_share_payload`) are *private* `fn`s in
// `modlist_share.rs` — unreachable from orchestrator code even through the
// carve-out-#3 lib+bin split (only `pub(crate)`+ items cross). So the
// earlier-plan "re-decode, flip the bit, re-encode" was not implementable
// against BIO. `pack_meta` re-implements the *standard* zlib + base64url
// codec (only existing crate deps — `flate2`, `serde_json` — plus a
// ~30-line standard base64url codec; there is no `base64` crate dep) and
// rides the payload through as an opaque `Value`, so it is agnostic to any
// future BIO payload change (zero drift). The byte-format is identical to
// BIO's: zlib (`flate2` default `Compression`) of the JSON, then the same
// URL-safe base64 alphabet (`A-Za-z0-9-_`, no `=` padding) BIO's
// `base64url_encode` uses — so BIO's own decoder round-trips it bit-for-bit.
//
// **Provenance is read off the registry entry, NOT re-derived from
// `WizardState`** (SPEC §13.3 Provenance: `name` ← `ModlistEntry.name`,
// `author` ← `ModlistEntry.author`, `forked_from` ← `ModlistEntry
// .forked_from`). `pack_meta` takes a `ShareMeta` snapshot the caller built
// from the entry.
//
// Callers (per SPEC §13.3): the install-start `modlist-import-code.txt`
// write + the registry `latest_share_code` snapshot (P7.T3,
// `allow_auto_install = false`), and `flip_to_installed`'s post-success
// regeneration (P7.T6, `allow_auto_install = true`). Save Draft / Create →
// Import-and-modify reuse the same sibling.
//
// SPEC: §13.3 (Share code BIO-MODLIST-V1 — Provenance + Generation
//        mechanism + `allow_auto_install`), §1 carve-out #5.

// rationale: byte/codec math — `f32 as u8`-style channel/bit roundings do
// not occur here; the base64url shift/mask arithmetic is correct by
// construction and must not be "simplified" (it mirrors BIO's verified
// `base64url_*`). The match-arm readability + `const fn` lints add churn
// without behavior value (Cat 3).
#![allow(clippy::missing_const_for_fn, clippy::match_same_arms)]

use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use serde_json::Value;

use crate::app::modlist_share::ForkAncestor;
use crate::app::state::WizardState;
use crate::registry::model::ModlistEntry;

/// The same prefix BIO's `modlist_share` uses (`SHARE_CODE_PREFIX`).
const SHARE_CODE_PREFIX: &str = "BIO-MODLIST-V1:";

/// The four schema-additive sibling keys the orchestrator injects (SPEC
/// §13.3). Provenance is captured **off the registry entry** by
/// [`ShareMeta::from_entry`] — never re-derived from `WizardState`.
///
/// Derives `PartialEq` but **not** `Eq`: it holds BIO's `ForkAncestor`
/// (carve-out #5), which derives only `PartialEq` — and adding `Eq` to that
/// BIO struct is forbidden by the CRITICAL DIRECTIVE. `PartialEq` is
/// sufficient for the tests / debug comparisons; no `Eq` bound is needed
/// anywhere.
#[derive(Debug, Clone, PartialEq)]
pub struct ShareMeta {
    /// `allow_auto_install` — `false` for any code generated before the
    /// modlist reaches `Installed` (Save Draft, install-start, in-progress
    /// `latest_share_code`); `true` only from `flip_to_installed`. SPEC
    /// §13.3 "`allow_auto_install` (schema-additive field, default
    /// `true`)".
    pub allow_auto_install: bool,
    /// The modlist's display name (`ModlistEntry.name`). `None` ⇒ key
    /// omitted (today's-behavior fallback).
    pub name: Option<String>,
    /// The handle of whoever generated this code (`ModlistEntry.author` ←
    /// `RedesignSettings.user_name` at create/fork; empty ⇒ `None`).
    pub author: Option<String>,
    /// Append-only fork lineage, oldest → newest (`ModlistEntry
    /// .forked_from`). Empty for a from-scratch (non-forked) modlist.
    ///
    /// `pub(crate)` (the other fields are `pub`): the element type is
    /// BIO's `pub(crate)` carve-out-#5 `ForkAncestor`, so this field
    /// cannot be more public than the type it holds (the
    /// `private_interfaces` lint — the **exact same** resolution as the
    /// registry `ModlistEntry::forked_from`). Every reader/builder is
    /// in-crate (`ShareMeta::from_entry`, `pack_meta`, the orchestrator
    /// generate paths), so `pub(crate)` is correct and sufficient.
    pub(crate) forked_from: Vec<ForkAncestor>,
}

impl ShareMeta {
    /// Build the metadata snapshot from a registry entry + the chosen
    /// `allow_auto_install` posture. The provenance trio is read verbatim
    /// off the entry (SPEC §13.3 Provenance — the entry is the source of
    /// truth; `pack_meta` does **not** re-derive it from `WizardState`). An
    /// empty `name` / `author` string is normalized to `None` so the key is
    /// omitted (matching SPEC §13.3 "If `user_name` is empty, `author` is
    /// omitted (`None`)" and the honest-fallback rule for `name`).
    #[must_use]
    pub fn from_entry(entry: &ModlistEntry, allow_auto_install: bool) -> Self {
        let name = Some(entry.name.trim().to_string()).filter(|s| !s.is_empty());
        let author = entry
            .author
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        Self {
            allow_auto_install,
            name,
            author,
            forked_from: entry.forked_from.clone(),
        }
    }
}

/// **The canonical generate-side envelope (SPEC §13.3 "Generation
/// mechanism").** Composes BIO's `export_modlist_share_code` for the base
/// payload, then injects the four sibling keys via a standard zlib +
/// base64url + `serde_json::Value` round-trip.
///
/// Returns the augmented `BIO-MODLIST-V1:<base64url(zlib(json))>` string, or
/// the BIO generator's own `Err` string (e.g. "No WeiDU entries available to
/// export.") propagated unchanged so the caller can surface it per SPEC
/// §13.14.
///
/// **Never patches BIO** — `export_modlist_share_code` is consumed
/// read-only; the envelope is this module's own standard codec.
pub fn pack_meta(wizard_state: &WizardState, meta: &ShareMeta) -> Result<String, String> {
    // 1. BIO builds the canonical payload (unchanged, read-only).
    let base = crate::app::modlist_share::export_modlist_share_code(wizard_state)?;

    // 2. Strip prefix → base64url-decode → zlib-inflate → opaque Value.
    let encoded = base
        .trim()
        .strip_prefix(SHARE_CODE_PREFIX)
        .ok_or_else(|| "BIO share code did not start with BIO-MODLIST-V1:".to_string())?;
    let compressed = base64url_decode(encoded)?;
    let json_bytes = zlib_decompress(&compressed)?;
    let mut payload: Value = serde_json::from_slice(&json_bytes)
        .map_err(|err| format!("BIO share payload was not valid JSON: {err}"))?;

    // 3. Insert the four keys at the top level of the object (SPEC §13.3:
    //    flat sibling keys, NOT a wrapper). Provenance keys are omitted
    //    when absent so the code parses to today's BIO behavior bit-for-bit
    //    on the consume side (the carve-out #5 `#[serde(default)]` fields).
    let obj = payload
        .as_object_mut()
        .ok_or_else(|| "BIO share payload was not a JSON object".to_string())?;
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
    if !meta.forked_from.is_empty() {
        let lineage = meta
            .forked_from
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "author": a.author,
                })
            })
            .collect::<Vec<_>>();
        obj.insert("forked_from".to_string(), Value::Array(lineage));
    }

    // 4. Re-serialize → zlib-deflate → base64url-encode → re-attach prefix.
    let out_bytes =
        serde_json::to_vec(&payload).map_err(|err| format!("re-serialize failed: {err}"))?;
    let recompressed = zlib_compress(&out_bytes)?;
    Ok(format!(
        "{SHARE_CODE_PREFIX}{}",
        base64url_encode(&recompressed)
    ))
}

// ── Standard zlib + base64url codec (only `flate2` + std; byte-identical
//    to BIO's private `zlib_*` / `base64url_*` so BIO's own decoder
//    round-trips the augmented code). ──

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

/// URL-safe base64 (`A-Za-z0-9-_`), no `=` padding — identical alphabet +
/// chunking to BIO's `modlist_share::base64url_encode`.
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

/// Inverse of [`base64url_encode`] — identical decoding rules to BIO's
/// `modlist_share::base64url_decode` (whitespace-tolerant, `=`-padding
/// optional, validates alphabet + padding).
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
        let pad = chunk.iter().filter(|value| **value == 64).count();
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
    //! `pack_meta` round-trip proof (SPEC §13.3 Generation mechanism).
    //!
    //! The codec is verified standalone (BIO's generator needs file-backed
    //! mod-download / weidu-log state that a unit test cannot stand up, so
    //! the *full* `pack_meta(WizardState, …)` path is exercised by the
    //! Run-2 manual breakpoint, not here). What is unit-tested: the
    //! zlib+base64url codec is a lossless round-trip, and the four-key
    //! injection on an opaque `serde_json::Value` produces the SPEC §13.3
    //! flat-sibling shape that BIO's `#[serde(default)]` consume path reads
    //! (decoded back via BIO's own `preview_modlist_share_code` semantics —
    //! here directly via the inverse codec + `serde_json`).
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

    /// The exact transform `pack_meta` applies to the base payload (steps
    /// 2-4), proven against an in-memory base code (the BIO generator's
    /// step-1 output is file-state-dependent; this isolates the envelope
    /// the orchestrator owns).
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
        if !meta.forked_from.is_empty() {
            let lineage = meta
                .forked_from
                .iter()
                .map(|a| json!({ "name": a.name, "author": a.author }))
                .collect::<Vec<_>>();
            obj.insert("forked_from".to_string(), Value::Array(lineage));
        }
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
        // Install-start posture: allow_auto_install = false, no provenance.
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
            forked_from: vec![],
        };
        let code = pack_meta_envelope_only(&base, &meta).expect("pack");
        let v = decode_payload(&code);
        // SPEC §13.13: install-start codes carry allow_auto_install=false.
        assert_eq!(v["allow_auto_install"], json!(false));
        // Absent provenance ⇒ keys omitted (today's-behavior fallback —
        // BIO's #[serde(default)] parses missing keys to None/empty).
        assert!(v.get("name").is_none());
        assert!(v.get("author").is_none());
        assert!(v.get("forked_from").is_none());
        // The base payload rides through untouched (opaque Value — zero
        // drift vs any future BIO field).
        assert_eq!(v["game_install"], json!("EET"));
        assert_eq!(v["bio_version"], json!("0.1.0-test"));
    }

    #[test]
    fn pack_meta_injects_true_bit_and_full_provenance() {
        // flip_to_installed posture: allow_auto_install = true + the
        // provenance trio off the registry entry.
        let base = make_base(&json!({
            "format_version": 1,
            "game_install": "BGEE",
            "install_mode": "build_from_scanned_mods",
        }));
        let meta = ShareMeta {
            allow_auto_install: true,
            name: Some("Polished BG2EE".to_string()),
            author: Some("@b2bs".to_string()),
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
        };
        let code = pack_meta_envelope_only(&base, &meta).expect("pack");
        let v = decode_payload(&code);
        assert_eq!(v["allow_auto_install"], json!(true));
        assert_eq!(v["name"], json!("Polished BG2EE"));
        assert_eq!(v["author"], json!("@b2bs"));
        // forked_from is the flat oldest→newest array of {name,author}
        // (SPEC §13.3 Provenance lineage append rule).
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
            author: Some("   ".to_string()), // whitespace ⇒ None
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
        assert_eq!(meta.forked_from.len(), 1);

        // Empty name also ⇒ None (honest-fallback rule).
        let entry2 = ModlistEntry {
            name: String::new(),
            ..entry
        };
        assert_eq!(ShareMeta::from_entry(&entry2, true).name, None);
    }

    #[test]
    fn injection_is_idempotent_on_reinjection() {
        // Re-packing an already-packed code overwrites the keys (the
        // flip_to_installed → re-generate path: install-start wrote false,
        // success re-writes true on the freshly-rebuilt BIO base — proven
        // here at the envelope level: re-injecting different meta replaces,
        // never duplicates).
        let base = make_base(&json!({ "format_version": 1, "game_install": "EET" }));
        let first = pack_meta_envelope_only(
            &base,
            &ShareMeta {
                allow_auto_install: false,
                name: Some("X".to_string()),
                author: None,
                forked_from: vec![],
            },
        )
        .unwrap();
        let second = pack_meta_envelope_only(
            &first,
            &ShareMeta {
                allow_auto_install: true,
                name: Some("X".to_string()),
                author: Some("@me".to_string()),
                forked_from: vec![],
            },
        )
        .unwrap();
        let v = decode_payload(&second);
        assert_eq!(v["allow_auto_install"], json!(true));
        assert_eq!(v["author"], json!("@me"));
        // Still a single object, keys not duplicated.
        assert!(v.is_object());
        assert_eq!(v["game_install"], json!("EET"));
    }
}
