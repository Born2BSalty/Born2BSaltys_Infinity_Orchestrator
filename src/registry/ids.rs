// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `ids` — stable, slug-safe modlist identifier generator.
//
// Per Phase 3 P3.T5: a 12-character Crockford-base32 ULID-style identifier.
// The string is `<48-bit millisecond timestamp><30 bits randomness>` packed
// into 12 base32 digits. Sortable by creation time, no hyphens, only
// uppercase alphanumerics — safe to use as a directory name and a URL slug.
//
// Implementation note: we avoid pulling a `ulid` / `uuid` crate dependency
// for v1 alpha — a hand-rolled 12-char generator with the system clock and
// `std::collections::hash_map::DefaultHasher` for entropy is sufficient.
// The randomness is **not** cryptographic; it only needs to be collision-free
// for the user's local modlist directory.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// Crockford base-32 alphabet — excludes `I L O U` to prevent visual confusion.
const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a fresh 12-character ULID-style modlist ID.
///
/// Layout (60 bits of state packed into 12 base32 digits):
///   - First 40 bits: milliseconds since UNIX epoch truncated (≈ a year
///     of unique window per epoch wrap; sortable lexicographically).
///   - Next 20 bits: process-local monotonic counter + hash entropy.
///
/// The counter is the primary collision-avoidance mechanism for rapid-fire
/// calls (the test seeds 256 IDs in sub-millisecond time on most machines);
/// the hash mixes in process ID and timestamp so two simultaneous orchestrator
/// processes won't generate identical sequences.
///
/// **Not cryptographic.** Only needs to avoid local collisions when seeding /
/// creating modlists within milliseconds of each other.
pub fn new_modlist_id() -> String {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let (counter, hashed) = derive_entropy(ts_ms);
    // 60 bits = 40 timestamp + 20 entropy (12 from counter, 8 from hash mix).
    let ts_part = ts_ms & 0x0000_00FF_FFFF_FFFF; // 40 bits
    let counter_part = counter & 0x0000_0000_000F_FFFF; // 20 bits raw — combined below
    let hash_part = (hashed >> 32) & 0x0000_00FF; // 8 bits

    // Layout: [40 ts][12 counter][8 hash] = 60 bits, fits 12 base32 digits.
    let combined: u64 = (ts_part << 20) | ((counter_part & 0x0FFF) << 8) | hash_part;

    encode_base32_12(combined)
}

fn derive_entropy(seed: u64) -> (u64, u64) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let bump = COUNTER.fetch_add(1, Ordering::Relaxed);

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    bump.hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    (bump, hasher.finish())
}

/// Encode a 60-bit number into 12 Crockford-base32 characters.
fn encode_base32_12(value: u64) -> String {
    let mut out = [0u8; 12];
    let mut v = value;
    for i in (0..12).rev() {
        out[i] = CROCKFORD_ALPHABET[(v & 0x1F) as usize];
        v >>= 5;
    }
    // SAFETY: every byte in `out` is from `CROCKFORD_ALPHABET`, which is ASCII.
    String::from_utf8(out.to_vec()).expect("crockford base32 alphabet is ASCII-only")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_12_chars() {
        for _ in 0..32 {
            let id = new_modlist_id();
            assert_eq!(id.len(), 12, "id `{id}` should be 12 chars");
        }
    }

    #[test]
    fn ids_are_crockford_uppercase_alnum() {
        let id = new_modlist_id();
        for c in id.chars() {
            assert!(
                c.is_ascii_uppercase() || c.is_ascii_digit(),
                "id `{id}` contains illegal char `{c}`"
            );
            assert!(
                !"ILOU".contains(c),
                "id `{id}` contains a Crockford-disallowed letter `{c}`"
            );
        }
    }

    #[test]
    fn ids_do_not_collide_within_tight_loop() {
        let mut seen: HashSet<String> = HashSet::new();
        for _ in 0..256 {
            let id = new_modlist_id();
            assert!(seen.insert(id.clone()), "duplicate id generated: {id}");
        }
    }
}
