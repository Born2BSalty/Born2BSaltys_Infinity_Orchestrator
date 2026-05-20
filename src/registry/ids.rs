// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

#[must_use]
pub fn new_modlist_id() -> String {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));

    let (counter, hashed) = derive_entropy(ts_ms);

    let ts_part = ts_ms & 0x0000_00FF_FFFF_FFFF;
    let counter_part = counter & 0x0000_0000_000F_FFFF;
    let hash_part = (hashed >> 32) & 0x0000_00FF;

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

fn encode_base32_12(value: u64) -> String {
    let mut out = [0u8; 12];
    let mut v = value;
    for i in (0..12).rev() {
        out[i] = CROCKFORD_ALPHABET[(v & 0x1F) as usize];
        v >>= 5;
    }

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
