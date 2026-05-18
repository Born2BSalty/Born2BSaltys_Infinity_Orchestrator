// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::import_code_writer` — writes `modlist-import-code.txt`
// to the install destination at install start (SPEC §13.13 "Import code
// auto-generated on install start").
//
// Net-new orchestrator code (std::fs only) — no BIO source involved. The
// **variant gating** (Install / Restart / Reinstall write/overwrite;
// Resume skips) is the caller's responsibility (`start_hooks
// ::on_install_start` decides whether to call this); this module's single
// job is the unconditional write/overwrite + parent-dir creation.
//
// Per SPEC §13.13 the file is written **upfront, before WeiDU runs**, so
// the artifact survives a crash / cancel / errored install. The content is
// always a code with `allow_auto_install = false` (the install hasn't
// succeeded yet — the caller composes that via `registry::share_export
// ::pack_meta`); this module does not interpret the content.
//
// SPEC: §13.13.

use std::fs;
use std::io;
use std::path::Path;

/// The canonical filename written to the destination (SPEC §13.13: "The
/// default filename is `modlist-import-code.txt`").
pub const IMPORT_CODE_FILENAME: &str = "modlist-import-code.txt";

/// Write (creating, or **overwriting** if present) `modlist-import-code.txt`
/// into `install_destination` with the given share code as its full
/// contents.
///
/// `install_destination` is the modlist's destination folder. The parent
/// (the destination itself) is created if it does not yet exist — at
/// install start the destination may not have been touched by WeiDU yet,
/// and SPEC §13.13 requires the file to exist *before* WeiDU runs.
///
/// Returns `Ok(())` on success, or an `io::Error` (the caller surfaces it
/// per SPEC §13.14 — the install-start hook should not silently swallow a
/// write failure of the recovery artifact).
pub fn write_modlist_import_code_txt(
    install_destination: &Path,
    share_code: &str,
) -> io::Result<()> {
    // The destination must exist for the file to land in it. `create_dir_all`
    // is idempotent (Ok if it already exists) and creates intermediate
    // components — the destination folder may not have been created yet at
    // install-start time (clone / prepare-target-dirs runs later, in BIO's
    // pipeline).
    fs::create_dir_all(install_destination)?;
    let path = install_destination.join(IMPORT_CODE_FILENAME);
    // `fs::write` truncates an existing file — exactly the SPEC §13.13
    // "overwrite if present" semantics for the Install / Restart / Reinstall
    // variants (the Resume "do not overwrite" case never reaches this fn —
    // the caller gates it out).
    fs::write(path, share_code.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique temp dir per test (DATA-LOSS hygiene — never `%APPDATA%`).
    fn temp_dest(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "bio_import_code_test_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        p
    }

    #[test]
    fn writes_file_into_a_not_yet_existing_destination() {
        let dest = temp_dest("fresh");
        assert!(!dest.exists(), "precondition: destination does not exist");
        write_modlist_import_code_txt(&dest, "BIO-MODLIST-V1:abc").expect("write");
        let written =
            fs::read_to_string(dest.join(IMPORT_CODE_FILENAME)).expect("read back the file");
        assert_eq!(written, "BIO-MODLIST-V1:abc");
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn overwrites_an_existing_file() {
        let dest = temp_dest("overwrite");
        fs::create_dir_all(&dest).unwrap();
        let path = dest.join(IMPORT_CODE_FILENAME);
        fs::write(&path, "OLD-CODE-from-a-cancelled-attempt").unwrap();
        // SPEC §13.13: Restart / Reinstall overwrite with the current code.
        write_modlist_import_code_txt(&dest, "BIO-MODLIST-V1:new").expect("overwrite");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "BIO-MODLIST-V1:new",
            "the previous (cancelled-attempt) file must be discarded"
        );
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn idempotent_when_destination_already_exists() {
        let dest = temp_dest("idem");
        fs::create_dir_all(&dest).unwrap();
        write_modlist_import_code_txt(&dest, "X").expect("first write");
        write_modlist_import_code_txt(&dest, "X").expect("second write — create_dir_all is Ok");
        let _ = fs::remove_dir_all(&dest);
    }
}
