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

/// The schema-additive sibling key carrying the per-archive `{name, size,
/// hash}` triples (the **Wabbajack-compile analog** — SPEC §13.3 / §13.12a).
/// Injected at the top level of the decoded payload object by [`pack_meta`]
/// / [`bake_archive_meta_into_code`] **exactly like the provenance trio**
/// (an opaque `serde_json::Value` round-trip in this orchestrator-owned
/// envelope — NOT a BIO edit; SPEC §13.3 "generation is not a BIO
/// modification"). Absent ⇒ [`decode_archive_meta`] yields an empty vec ⇒
/// "no expected hashes known" ⇒ the install-time skip falls back to today's
/// always-download for those archives (never an error — older /
/// third-party / pre-redesign codes decode and behave bit-for-bit as
/// today).
const ARCHIVE_META_KEY: &str = "archive_meta";

/// One archive's content-identity record, baked into the share code by the
/// machine that **has** the archive bytes (the Wabbajack-compile model: the
/// author/exporter hashes each archive and ships `{size, hash}` in the
/// modlist). The installer size-prefilters then hash-matches local files
/// against these (see `install_runtime::archive_skip`).
///
/// - `name` — the **logical archive name** (`bio::app::app_step2_update_
///   download::archive_file_name(asset)` — the SAME key the content-
///   addressed store / index / per-install lock use). Stable, deterministic
///   from the resolved asset; the join key between the decoded share code
///   and the resolved asset set on the install side.
/// - `size` — the archive's exact byte length (the cheap pre-filter: skip
///   hashing any on-disk candidate whose length differs).
/// - `hash` — the archive's content hash, the **same** stable seedless
///   128-bit FNV-1a (`archive_store::hash_file`, 32 hex chars) used
///   everywhere else for content-addressing (one hashing path, zero drift —
///   it is an identity/dedupe hash, not a security primitive).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveMeta {
    /// The logical archive name (`archive_file_name(asset)`).
    pub name: String,
    /// Exact archive byte length (the size pre-filter).
    pub size: u64,
    /// Content hash (`archive_store::hash_file` — FNV-1a-128, 32 hex).
    pub hash: String,
}

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
    /// The per-archive `{name, size, hash}` triples (the Wabbajack-compile
    /// analog — SPEC §13.3 / §13.12a). Baked into the emitted code as the
    /// schema-additive [`ARCHIVE_META_KEY`] sibling **exactly like the
    /// provenance trio** (an opaque `serde_json::Value` round-trip in this
    /// orchestrator-owned envelope — never a BIO edit). Empty ⇒ the key is
    /// **omitted** (today's-behavior fallback: a code with no archive meta
    /// decodes/behaves bit-for-bit as today; the install-time skip then
    /// falls back to always-download for those archives). On the Workspace /
    /// build-from-scanned-mods `pack_meta` path the caller fills this from
    /// the on-disk resolved archives ([`build_archive_meta_for_assets`]);
    /// on a generation point that structurally lacks the files it is left
    /// empty.
    pub archive_meta: Vec<ArchiveMeta>,
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
            // Default empty — a caller that has the resolved archives on
            // disk (the Workspace / build-from-scanned-mods path) sets this
            // explicitly via `with_archive_meta`; a generation point that
            // structurally lacks the files leaves it empty (the key is then
            // omitted ⇒ today's-behavior fallback).
            archive_meta: Vec::new(),
        }
    }

    /// Builder: attach the per-archive `{name, size, hash}` triples (the
    /// Wabbajack-compile analog). The caller computes these from the
    /// on-disk resolved archives via [`build_archive_meta_for_assets`] at a
    /// generation point where the machine **has** the bytes.
    #[must_use]
    pub fn with_archive_meta(mut self, archive_meta: Vec<ArchiveMeta>) -> Self {
        self.archive_meta = archive_meta;
        self
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
    // The per-archive `{name, size, hash}` sibling (the Wabbajack-compile
    // analog) — injected the SAME opaque way as the provenance trio. Empty
    // ⇒ key omitted (today's-behavior fallback; a fieldless code decodes
    // bit-for-bit as today). NOT a BIO edit (SPEC §13.3 "generation is not
    // a BIO modification").
    insert_archive_meta(obj, &meta.archive_meta);

    // 4. Re-serialize → zlib-deflate → base64url-encode → re-attach prefix.
    let out_bytes =
        serde_json::to_vec(&payload).map_err(|err| format!("re-serialize failed: {err}"))?;
    let recompressed = zlib_compress(&out_bytes)?;
    Ok(format!(
        "{SHARE_CODE_PREFIX}{}",
        base64url_encode(&recompressed)
    ))
}

/// **Set `allow_auto_install` on an ALREADY-EXISTING BIO-MODLIST-V1 code's
/// payload — without re-deriving the payload from `WizardState`.**
///
/// This is `pack_meta`'s envelope (steps 2 → 3 → 4) **minus step 1**: it
/// does NOT call `bio::app::modlist_share::export_modlist_share_code`. It
/// takes a code the orchestrator already holds (the user-pasted Install-
/// Modlist code, or a registry entry's stored `latest_share_code`), decodes
/// its payload to an **opaque `serde_json::Value`**, sets *only* the
/// `allow_auto_install` sibling key, and re-encodes via the same standard
/// zlib + base64url codec `pack_meta` uses (byte-identical to BIO's private
/// codec, so BIO's own `preview_modlist_share_code` / `import_modlist_share
/// _code` round-trip it unchanged).
///
/// **Why this exists (the Install-Modlist-paste / Reinstall path).** SPEC
/// §13.13 needs the install-start `modlist-import-code.txt` written with
/// `allow_auto_install = false` and the clean-exit rewrite with `= true`.
/// For the **Install-Modlist** entry points the user already has the code
/// (they pasted it); regenerating it via `pack_meta` is impossible there —
/// BIO's `export_modlist_share_code` reads `state.step3.{bgee,bg2ee}_items`
/// (build-from-scanned-mods) or the exact-WeiDU-log source, and at the
/// install-start arm point `state.step3` is empty (the share-code import's
/// `reset_workflow_keep_step1` clears it, and the async scan→apply-log has
/// not run yet) ⇒ `export_modlist_share_code` `Err`s. So the orchestrator
/// **persists the code it already has**, only flipping the
/// `allow_auto_install` bit on its decoded payload — the path BIO's
/// `import_modlist_share_code` can re-consume, with the pasted code's
/// baked-in `name` / `author` / `forked_from` riding through **verbatim**
/// (the payload is opaque — provenance + every other key is preserved
/// bit-for-bit; this is the SPEC §13.3 Provenance "the real code carries the
/// name" property, achieved by *not* rewriting it).
///
/// The **Workspace / build-from-scanned-mods** path is UNCHANGED — there
/// `state.step3` IS populated (built in the workspace) so `pack_meta`
/// regeneration is correct; this fn is **not** used on that path.
///
/// Returns the re-encoded `BIO-MODLIST-V1:` string, or an `Err(String)` if
/// `code` is not a decodable BIO-MODLIST-V1 code (the caller falls back to
/// persisting `code` **verbatim** — per the user's resolution, persisting
/// the real code is the priority over the false→true draft nicety).
///
/// **Never patches BIO** — the envelope is `share_export`'s own standard
/// codec; `export_modlist_share_code` is not called here at all (SPEC §1
/// carve-out #5).
pub fn set_allow_auto_install(code: &str, allow_auto_install: bool) -> Result<String, String> {
    // 1. Strip prefix → base64url-decode → zlib-inflate → opaque Value
    //    (identical to `pack_meta` step 2, but the input is the code the
    //    orchestrator already has — NOT a freshly-built BIO base).
    let encoded = code
        .trim()
        .strip_prefix(SHARE_CODE_PREFIX)
        .ok_or_else(|| "share code did not start with BIO-MODLIST-V1:".to_string())?;
    let compressed = base64url_decode(encoded)?;
    let json_bytes = zlib_decompress(&compressed)?;
    let mut payload: Value = serde_json::from_slice(&json_bytes)
        .map_err(|err| format!("share payload was not valid JSON: {err}"))?;

    // 2. Set ONLY the `allow_auto_install` sibling key. Every other key —
    //    incl. the provenance trio `name` / `author` / `forked_from` baked
    //    into the pasted code — rides through **untouched** (opaque Value;
    //    SPEC §13.3 Provenance: the real code carries the name, so the
    //    "Shared modlist" fallback stops because we did NOT rewrite it).
    let obj = payload
        .as_object_mut()
        .ok_or_else(|| "share payload was not a JSON object".to_string())?;
    obj.insert(
        "allow_auto_install".to_string(),
        Value::Bool(allow_auto_install),
    );

    // 3. Re-serialize → zlib-deflate → base64url-encode → re-attach prefix
    //    (identical to `pack_meta` step 4 — the same byte-format BIO's own
    //    decoder round-trips).
    let out_bytes =
        serde_json::to_vec(&payload).map_err(|err| format!("re-serialize failed: {err}"))?;
    let recompressed = zlib_compress(&out_bytes)?;
    Ok(format!(
        "{SHARE_CODE_PREFIX}{}",
        base64url_encode(&recompressed)
    ))
}

/// Insert the [`ARCHIVE_META_KEY`] sibling into the decoded payload object.
/// **The one place the key is written** (both `pack_meta` and
/// [`bake_archive_meta_into_code`] call this — one shape, no drift). Empty
/// ⇒ the key is **omitted** so a code with no archive meta is byte-for-byte
/// what today's BIO produces (the install-time skip then falls back to
/// always-download for those archives — never an error). Each element is
/// the flat `{ "name": String, "size": u64, "hash": String }` object.
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

/// **Bake the per-archive `{name, size, hash}` sibling into an
/// ALREADY-EXISTING BIO-MODLIST-V1 code — the Wabbajack-compile analog for
/// the Install-Modlist-paste / Reinstall path** (where the install machine
/// has just downloaded+verified the archives, so it *has* the bytes, but
/// per SPEC §13.3 it **persists the held code** rather than regenerating).
///
/// This is the [`set_allow_auto_install`] envelope with `archive_meta`
/// injected instead of `allow_auto_install`: it decodes the held code to an
/// **opaque `serde_json::Value`**, inserts ONLY the [`ARCHIVE_META_KEY`]
/// sibling (every other key — incl. the provenance trio + the
/// `allow_auto_install` bit + every BIO payload field — rides through
/// **verbatim**, opaque), and re-encodes with the same byte-format BIO's
/// own decoder round-trips. Composes — never patches — BIO (it does not
/// call `export_modlist_share_code` at all; SPEC §1 carve-out #5 / §13.3
/// "generation is not a BIO modification").
///
/// Empty `archive_meta` ⇒ the code is returned **unchanged** (re-encoded
/// through the same lossless codec; the key is omitted). A non-decodable
/// `code` ⇒ `Err` (the caller falls back to persisting the code verbatim —
/// the established "the real code is the priority" rule).
pub fn bake_archive_meta_into_code(
    code: &str,
    archive_meta: &[ArchiveMeta],
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
    // Insert ONLY archive_meta — every other key (provenance trio,
    // allow_auto_install, all BIO fields) is preserved bit-for-bit (opaque
    // Value). Empty ⇒ key omitted (the code is then a lossless re-encode).
    insert_archive_meta(obj, archive_meta);
    let out_bytes =
        serde_json::to_vec(&payload).map_err(|err| format!("re-serialize failed: {err}"))?;
    let recompressed = zlib_compress(&out_bytes)?;
    Ok(format!(
        "{SHARE_CODE_PREFIX}{}",
        base64url_encode(&recompressed)
    ))
}

/// **Decode the per-archive `{name, size, hash}` sibling out of a
/// BIO-MODLIST-V1 code** (the install-side read — the Wabbajack-installer
/// "expected hashes" input). Orchestrator-owned: it reads the key off the
/// **opaque** decoded payload exactly the way [`pack_meta`] *wrote* it — it
/// does **not** add a field to BIO's `ModlistSharePayload` (BIO does not
/// need it; SPEC §13.3 "generation is not a BIO modification" — the
/// symmetric consume side is equally orchestrator-owned and equally
/// not-a-BIO-edit).
///
/// **Backward-compatible by construction:** a code with no
/// [`ARCHIVE_META_KEY`] (pre-redesign, third-party, or any code generated
/// before this ships) ⇒ `Ok(vec![])` (NOT an error) ⇒ the caller treats it
/// as "no expected hashes known" and falls back to today's always-download
/// for those archives. A malformed key (wrong type / missing fields) is
/// skipped element-wise (a best-effort dedupe accelerator must never harden
/// into a parse failure that blocks an install). A `code` that is not a
/// decodable BIO-MODLIST-V1 string ⇒ `Err` (the caller already had to
/// decode it to preview/import, so this only fails on a genuinely broken
/// input — handled the same as today).
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
        // Absent (or not an array) ⇒ today's-behavior fallback: no expected
        // hashes known. NOT an error.
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(arr.len());
    for el in arr {
        let (Some(name), Some(size), Some(hash)) = (
            el.get("name").and_then(Value::as_str),
            el.get("size").and_then(Value::as_u64),
            el.get("hash").and_then(Value::as_str),
        ) else {
            // Skip a malformed element rather than failing the whole decode
            // (the worst case is a redundant re-download of that one
            // archive — never a blocked install).
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

/// **Export-time hashing (the Wabbajack-compile step).** For every resolved
/// update asset whose archive is on disk at BIO's deterministic path
/// (`<archive_dir>/<archive_file_name(asset)>`), compute `{name, size,
/// hash}` — `name` = `archive_file_name(asset)` (the SAME logical key the
/// content-addressed store / lock / the install-time skip use), `size` =
/// the file's exact byte length, `hash` = `archive_store::hash_file` (the
/// ONE stable seedless FNV-1a-128 used everywhere — one hashing path, zero
/// drift).
///
/// An asset whose archive is **absent** on disk is simply skipped (no
/// entry) — the share code then carries no expected hash for it and the
/// recipient's install-time skip falls back to always-download for that one
/// (honest, never wrong). This is read-only on the archive dir and reuses
/// BIO's `archive_file_name` **read-only** (zero BIO edit).
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
            continue; // absent ⇒ no expected hash baked for this archive
        };
        if !meta.is_file() {
            continue;
        }
        let size = meta.len();
        // The SAME content hash everything else uses (one hashing path —
        // the brief's premise-check outcome: reuse `archive_store`'s stable
        // seedless FNV-1a-128, do NOT introduce a second algorithm).
        match crate::install_runtime::archive_store::hash_file(&path) {
            Ok(hash) => out.push(ArchiveMeta { name, size, hash }),
            Err(_) => continue, // unreadable ⇒ skip (honest "no hash")
        }
    }
    out
}

/// **Export-time hashing from the per-install lock (the primary
/// clean-exit bake source).** At a clean exit the just-downloaded+verified
/// archives are on disk and `archive_store`'s per-install lock
/// (`<destination>/.bio-install-lock.json`) records exactly `name → hash`
/// for THIS modlist (written by `ingest_downloaded_archives` /
/// `verify_downloaded_archives`). This turns that authoritative record into
/// the `{name, size, hash}` triples to bake into the verified, re-shareable
/// code — `name`/`hash` straight from the lock, `size` by stat-ing the
/// archive on disk (the content-addressed `<name>.<hash>.<ext>` copy, or
/// the deterministic `<name>`).
///
/// An entry whose archive cannot be stat-ed on disk is **omitted** (honest
/// — the recipient then always-downloads that one; never wrong). Empty lock
/// ⇒ empty vec (the key is then omitted by [`insert_archive_meta`] — a code
/// with no archive meta, today's-behavior fallback). Read-only on the lock
/// + the archive dir; zero BIO edit.
#[must_use]
pub fn build_archive_meta_from_install_lock(
    destination: &str,
    archive_dir: &std::path::Path,
) -> Vec<ArchiveMeta> {
    use crate::install_runtime::archive_store::{InstallArchiveLock, stored_filename};
    let lock = InstallArchiveLock::load(destination);
    let mut out = Vec::with_capacity(lock.resolved.len());
    for (name, hash) in &lock.resolved {
        // Prefer the content-addressed stored copy (it is the canonical,
        // hash-pinned file `ingest` wrote); fall back to the deterministic
        // path. Whichever exists, its byte length is the archive's size.
        let stored = archive_dir.join(stored_filename(name, hash));
        let deterministic = archive_dir.join(name);
        let size = std::fs::metadata(&stored)
            .or_else(|_| std::fs::metadata(&deterministic))
            .ok()
            .filter(std::fs::Metadata::is_file)
            .map(|m| m.len());
        let Some(size) = size else {
            continue; // not on disk ⇒ omit (recipient always-downloads it)
        };
        out.push(ArchiveMeta {
            name: name.clone(),
            size,
            hash: hash.clone(),
        });
    }
    out
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
        // Mirror `pack_meta`'s archive_meta injection (the SAME shared
        // injector the production path uses) so the envelope-only test
        // reflects the real shape.
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
            archive_meta: vec![],
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
            archive_meta: vec![],
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
                forked_from: vec![],
                archive_meta: vec![],
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

    // ───── Run 2 (re-scoped) — `set_allow_auto_install` (the
    //       Install-Modlist-paste / Reinstall decode-flip path) ─────
    //
    // The user's resolution (2026-05-18): for the Install-Modlist paste /
    // Reinstall entry points the orchestrator PERSISTS the code it already
    // has (the pasted code / the entry's stored code) — it does NOT
    // regenerate via `pack_meta` (impossible there: `state.step3` is empty
    // at install-start). Only the `allow_auto_install` bit is flipped on the
    // already-existing code's decoded payload (false at install-start, true
    // at clean exit), with the pasted code's baked-in provenance riding
    // through verbatim. These pin: the bit is set, every other key
    // (incl. the provenance trio) is byte-preserved, a non-decodable code
    // Errs (so the caller can fall back to verbatim), and BIO's own decoder
    // round-trips the result.

    #[test]
    fn set_allow_auto_install_flips_bit_and_preserves_all_other_keys() {
        // A code carrying full provenance + arbitrary BIO payload keys
        // (built via the same codec the real pasted code uses).
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

        // Install-start posture: flip to false.
        let at_start = set_allow_auto_install(&original, false).expect("decode-flip ok");
        let v = decode_payload(&at_start);
        assert_eq!(
            v["allow_auto_install"],
            json!(false),
            "install-start code carries allow_auto_install=false (SPEC §13.13)"
        );
        // EVERY other key is preserved bit-for-bit — the pasted code's
        // baked-in provenance + payload ride through verbatim (SPEC §13.3:
        // the real code carries the name, so the 'Shared modlist' fallback
        // stops because we did NOT rewrite it).
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

        // Clean-exit posture: flip the SAME code to true (the
        // `flip_to_installed` rewrite for the Install-Modlist path).
        let at_clean = set_allow_auto_install(&at_start, true).expect("re-flip ok");
        let v2 = decode_payload(&at_clean);
        assert_eq!(
            v2["allow_auto_install"],
            json!(true),
            "clean-exit rewrite carries allow_auto_install=true (SPEC §13.13)"
        );
        // Provenance STILL verbatim after the second flip.
        assert_eq!(v2["name"], json!("Tactical EET 2026"));
        assert_eq!(v2["author"], json!("@sharer"));
        assert_eq!(
            v2["forked_from"],
            json!([{ "name": "Root", "author": "@root" }])
        );
    }

    #[test]
    fn set_allow_auto_install_adds_the_bit_when_absent_pre_redesign_code() {
        // A pre-redesign / third-party code with NO allow_auto_install key
        // (and no provenance) — set_allow_auto_install must ADD the key
        // (not error), leaving the rest opaque-untouched.
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
        // A non-BIO-MODLIST-V1 string ⇒ Err (the caller persists the code
        // VERBATIM instead — the user's resolution: the real code is the
        // priority over the false→true draft nicety).
        assert!(set_allow_auto_install("not a share code", false).is_err());
        assert!(
            set_allow_auto_install("BIO-MODLIST-V1:!!!not-base64!!!", true).is_err(),
            "a prefixed-but-undecodable code Errs so the caller falls back \
             to verbatim"
        );
    }

    #[test]
    fn set_allow_auto_install_output_is_bio_decoder_round_trippable() {
        // The re-encoded code uses the SAME byte-format `pack_meta` emits
        // (zlib + URL-safe base64, no padding) — so BIO's own decoder
        // round-trips it. Proven here by the inverse codec yielding the
        // exact mutated payload (the codec round-trip `pack_meta`'s tests
        // already prove BIO-decoder-equivalent).
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

    // ───── Download-Overhaul Run 1 — the per-archive {name,size,hash}
    //       sibling (the Wabbajack-compile/-install model) ─────
    //
    // The share code carries `{name, size, hash}` per archive, injected the
    // SAME opaque way as the provenance trio (not a BIO edit). These pin:
    // the key round-trips byte-exact, it rides through `set_allow_auto_
    // install` verbatim (the Install-Modlist path), a fieldless code decodes
    // to an empty vec (today's-behavior fallback — never an error), and a
    // malformed element is skipped (a dedupe accelerator never blocks an
    // install).

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
            forked_from: vec![],
            archive_meta: vec![
                am("a__github__v1.zip", 17, "deadbeef00000000deadbeef00000000"),
                am("b__weasel__v2.7z", 4096, "0123456789abcdef0123456789abcdef"),
            ],
        };
        let code = pack_meta_envelope_only(&base, &meta).expect("pack");
        // The raw JSON object carries the flat sibling array.
        let v = decode_payload(&code);
        assert_eq!(
            v["archive_meta"],
            json!([
                { "name": "a__github__v1.zip", "size": 17, "hash": "deadbeef00000000deadbeef00000000" },
                { "name": "b__weasel__v2.7z", "size": 4096, "hash": "0123456789abcdef0123456789abcdef" },
            ]),
            "archive_meta is a flat sibling key (like forked_from), NOT a wrapper"
        );
        // The orchestrator-owned decoder recovers the triples byte-exact.
        let decoded = decode_archive_meta(&code).expect("decode");
        assert_eq!(decoded, meta.archive_meta, "round-trip byte-exact");
    }

    #[test]
    fn empty_archive_meta_omits_the_key_and_decodes_as_today() {
        // A code with no archive meta is byte-for-byte what today's BIO
        // produces (the key is OMITTED, not an empty array) ⇒ decode yields
        // an empty vec (NOT an error) ⇒ the install-time skip falls back to
        // always-download for those archives.
        let base = make_base(&json!({ "format_version": 1, "game_install": "BGEE" }));
        let meta = ShareMeta {
            allow_auto_install: true,
            name: None,
            author: None,
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
        // A genuinely pre-redesign / third-party code (built straight, no
        // envelope at all) also decodes to empty — never an error.
        let raw_old = make_base(&json!({ "format_version": 1, "game_install": "EET" }));
        assert_eq!(
            decode_archive_meta(&raw_old).expect("old code decodes"),
            Vec::<ArchiveMeta>::new(),
            "pre-redesign code (no archive_meta) ⇒ empty vec, not an error"
        );
    }

    #[test]
    fn bake_archive_meta_into_code_preserves_every_other_key_verbatim() {
        // The Install-Modlist-paste / Reinstall path: the machine HAS the
        // bytes but persists the held code. `bake_archive_meta_into_code`
        // adds ONLY the archive_meta sibling; every other key (provenance,
        // the allow_auto_install bit, all BIO payload fields) rides through
        // verbatim (opaque Value — exactly like set_allow_auto_install).
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
        // archive_meta added.
        assert_eq!(decode_archive_meta(&out).unwrap(), metas);
        // EVERY other key preserved bit-for-bit.
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
        // The Install-Modlist lifecycle: install-start bakes the archive
        // meta + flips the bit false; clean-exit flips it true. The
        // archive_meta must ride through BOTH flips verbatim (opaque Value)
        // — `set_allow_auto_install` only touches the one bit.
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
        // Empty ⇒ the code is returned unchanged (re-encoded losslessly;
        // key omitted). A non-BIO string ⇒ Err (caller falls back to
        // verbatim — the established "the real code is the priority" rule).
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
        // A best-effort dedupe accelerator must never harden into a parse
        // failure that blocks an install: a malformed element (wrong type /
        // missing field / empty name) is skipped element-wise; the good
        // ones still decode.
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
        // The Wabbajack-compile step: hash the resolved archives that are
        // on disk; an absent archive ⇒ no entry (the recipient then
        // always-downloads that one — honest, never wrong). `name` ==
        // `archive_file_name(asset)`, `hash` == `archive_store::hash_file`
        // (the ONE hashing path — same as everywhere else).
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

        let metas = build_archive_meta_for_assets(&[present.clone(), absent], &dir);
        assert_eq!(metas.len(), 1, "only the on-disk archive is hashed");
        assert_eq!(metas[0].name, present_name);
        assert_eq!(metas[0].size, "ARCHIVE-BYTES-123".len() as u64);
        // The hash is EXACTLY archive_store::hash_file (one hashing path).
        assert_eq!(
            metas[0].hash,
            crate::install_runtime::archive_store::hash_file(&dir.join(&present_name)).unwrap(),
            "the baked hash is the SAME stable FNV-1a-128 the content-\
             addressed store uses (one hashing path, zero drift)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
