// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::per_install_dirs` — Phase 7 P7.T17 piece 1
// (SPEC §13.12a "Per-install — created inside the modlist's destination
// folder, derived (never asked)" + SPEC §13.12 #2/#3/#4).
//
// Before an install starts, the redesign derives — into the orchestrator-
// owned `WizardState.step1` BIO's install runner reads — every per-install
// directory **inside the modlist's destination folder**, and forces the
// clone flags so the redesign **never** surfaces BIO's
// install-into-a-clean-dir-without-cloning path (SPEC §13.12a: "clone is
// forced for every install"). The global Mods-archive / Mods-backup /
// Game-source folders are NOT touched here — they come from Settings →
// Paths via the established `sync_paths_from_settings` (the Install/
// Workspace screens never collect game paths).
//
// **The exact per-install set (SPEC §13.12a + §13.12 #2/#3/#4):**
//   - **Mods folder** (§13.12a) — `<dest>/mods`. Archives extract/stage
//     here and this is what "scan mods folder" reads. Removed on a clean
//     successful install (`cleanup_per_install_mods_folder`); a failed /
//     cancelled install leaves it for diagnosis/resume.
//   - **#2 `-u` per-component logs** — `<dest>/weidu_component_logs`, with
//     `weidu_log_log_component = true` (and `weidu_log_mode_enabled` stays
//     BIO's default `true`). BIO derives `-u` from `weidu_log_folder` +
//     `weidu_log_log_component` (`state_validation_paths.rs:77-92`); the
//     user does not configure the path (SPEC §13.12 #2).
//   - **WeiDU-log SOURCE folders** (§13.12a per-install derived set) —
//     `<dest>/weidu_log_source/bgee` + `<dest>/weidu_log_source/bg2ee`.
//     The share-code importer (`modlist_share.rs`
//     `write_imported_weidu_logs` → `import_log_target_path`) writes the
//     code's baked-in `weidu.log` here, and BIO's saved-log / auto-build
//     applier (`app_step2_log.rs` `apply_saved_weidu_log_selection` →
//     `resolve_bgee_weidu_log_path` / `resolve_bg2_weidu_log_path`) reads
//     it back from here — the importer write target and the applier read
//     path MUST agree (else the pipeline scans nothing → permanent inert
//     "0 / 0 mods", the Install-Modlist-paste / Reinstall download-never-
//     starts root cause). The redesign never asks for these (§13.12a:
//     per-install, inside the destination, derived); BIO requires them
//     set before importing. EET uses *both* phase folders (its importer
//     writes a BGEE-phase log AND a BG2EE-phase log — distinct folders so
//     neither clobbers the other); single-game uses only the matching one.
//     `<game>_log_folder` (folder-mode), `<game>_log_file`
//     (`<folder>/weidu.log`, exact-log mode), and the `eet_*_log_folder`
//     pair are ALL set to the per-install paths so the importer↔applier
//     agree in **every** install mode the payload can carry
//     (`build_from_scanned_mods` / `install_exactly_from_weidu_logs` /
//     `start_from_weidu_logs_then_review_edit` — the paste route's mode is
//     the imported payload's, recomputed by `import_modlist_share_code`'s
//     `sync_install_mode_flags`, so the derivation must be mode-agnostic).
//   - **#3 EET `-p` / `-n`** (EET installs) — `new_pre_eet_dir_enabled =
//     true` + `eet_pre_dir = <dest>/Baldur's Gate Enhanced Edition`
//     (the Pre-EET / BG1 clone target, `--new-pre-eet-dir`);
//     `new_eet_dir_enabled = true` + `eet_new_dir = <dest>/Baldur's Gate
//     II Enhanced Edition` (the EET-final / BG2 clone target,
//     `--new-eet-dir`). The **source** game folders
//     (`bgee_game_folder` / `bg2ee_game_folder` — Settings → Paths) are
//     left untouched; BIO clones source → target (SPEC §13.12 #3,
//     `step5_command_install.rs:11-40`). Fixed names; not user-overridable.
//   - **#4 single-game `-g`** (BGEE / BG2EE / IWDEE) —
//     `generate_directory_enabled = true` + `generate_directory =
//     <dest>/<fixed game name>` (`--generate-directory`,
//     `step5_command_install.rs:57-60`). The source folder (Settings →
//     Paths) is left untouched. Fixed name; not user-overridable.
//
// The no-clone path (`new_pre_eet_dir_enabled = false` /
// `generate_directory_enabled = false`) is **never** set — SPEC §13.12a
// "The redesign never surfaces BIO's install-into-a-clean-dir-without-
// cloning path". BIO's capability is unchanged; the option is simply not
// presented.
//
// **Zero BIO source.** Every field written is a `pub` field on BIO's
// existing `Step1State` that BIO's existing command builder already reads;
// the directory creation is plain `std::fs`. No carve-out.
//
// SPEC: §13.12a, §13.12 #2/#3/#4.

use std::path::{Path, PathBuf};

use tracing::warn;

use crate::app::state::Step1State;
use crate::registry::model::Game;

/// The fixed per-install **Mods extract/stage/scan** folder name
/// (SPEC §13.12a). `<destination>/mods`.
pub const MODS_DIRNAME: &str = "mods";

/// The fixed per-install **`-u` per-component WeiDU log** folder name
/// (SPEC §13.12 #2). `<destination>/weidu_component_logs`.
pub const WEIDU_COMPONENT_LOGS_DIRNAME: &str = "weidu_component_logs";

/// The fixed per-install **WeiDU-log SOURCE** parent folder name
/// (SPEC §13.12a — per-install, inside the destination, derived, never
/// asked). This is **distinct from** the `-u` per-component logs dir
/// (`weidu_component_logs`, SPEC §13.12 #2) and from the game-clone dirs:
/// it is where the share-code importer writes the code's baked-in
/// `weidu.log` and where BIO's saved-log/auto-build flow reads it back.
///
/// Two phase subfolders live under it so an **EET** import (which writes
/// *both* a BGEE-phase log and a BG2EE-phase log — `modlist_share.rs`
/// `write_imported_weidu_logs` EET arm) cannot clobber one with the other
/// (a single shared folder ⇒ both `<folder>/weidu.log` ⇒ the BG2EE write
/// overwrites the BGEE write). Single-game uses only the matching one.
pub const WEIDU_LOG_SOURCE_DIRNAME: &str = "weidu_log_source";
/// The BGEE-/BG1-phase WeiDU-log source subfolder name (under
/// [`WEIDU_LOG_SOURCE_DIRNAME`]).
pub const WEIDU_LOG_SOURCE_BGEE_SUBDIR: &str = "bgee";
/// The BG2EE-/BG2-phase WeiDU-log source subfolder name (under
/// [`WEIDU_LOG_SOURCE_DIRNAME`]).
pub const WEIDU_LOG_SOURCE_BG2EE_SUBDIR: &str = "bg2ee";
/// The fixed file name BIO's importer writes and the saved-log applier
/// reads inside each phase subfolder (BIO's `import_log_target_path` joins
/// `weidu.log` onto the folder; `resolve_*_weidu_log_path` joins the same).
pub const WEIDU_LOG_FILENAME: &str = "weidu.log";

/// The fixed per-install game-clone folder names (SPEC §13.12 #3/#4 —
/// "standard fixed names ... users cannot override the names or
/// locations"). EET clones BG1 → the BGEE-named dir and BG2 → the
/// BG2EE-named dir; single-game clones source → the matching name.
pub const BGEE_CLONE_DIRNAME: &str = "Baldur's Gate Enhanced Edition";
pub const BG2EE_CLONE_DIRNAME: &str = "Baldur's Gate II Enhanced Edition";
pub const IWDEE_CLONE_DIRNAME: &str = "Icewind Dale Enhanced Edition";

/// The directories `derive_per_install_dirs` resolved (the decision,
/// separate from the `Step1State` it is written into — testable in
/// isolation, like `flag_policies::ResolvedFlags`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerInstallDirs {
    /// `<dest>/mods` — extract/stage + scan target (SPEC §13.12a).
    pub mods_folder: PathBuf,
    /// `<dest>/weidu_component_logs` — `-u` target (SPEC §13.12 #2).
    pub weidu_component_logs: PathBuf,
    /// `<dest>/weidu_log_source/bgee` — the BGEE/BG1-phase WeiDU-log
    /// SOURCE folder (SPEC §13.12a). The importer writes
    /// `<this>/weidu.log`; the applier reads it back from the same path.
    pub weidu_log_source_bgee: PathBuf,
    /// `<dest>/weidu_log_source/bg2ee` — the BG2EE/BG2-phase WeiDU-log
    /// SOURCE folder (SPEC §13.12a). Distinct from the BGEE one so an EET
    /// import's two logs never collide.
    pub weidu_log_source_bg2ee: PathBuf,
    /// EET only: `(pre_eet_dir, eet_final_dir)` — `-p` / `-n` clone
    /// targets (SPEC §13.12 #3). `None` for single-game.
    pub eet_clone_dirs: Option<(PathBuf, PathBuf)>,
    /// Single-game only: the `-g` clone target (SPEC §13.12 #4). `None`
    /// for EET.
    pub single_game_clone_dir: Option<PathBuf>,
}

impl PerInstallDirs {
    /// Every directory this resolution requires. All are under the
    /// destination; `create_dir_all` creates each (incl. the
    /// `weidu_log_source` parent of the two phase subfolders) so order is
    /// immaterial — the caller guarantees the destination exists.
    fn all_dirs(&self) -> Vec<&Path> {
        let mut out: Vec<&Path> = vec![
            &self.mods_folder,
            &self.weidu_component_logs,
            &self.weidu_log_source_bgee,
            &self.weidu_log_source_bg2ee,
        ];
        if let Some((pre, fin)) = self.eet_clone_dirs.as_ref() {
            out.push(pre);
            out.push(fin);
        }
        if let Some(g) = self.single_game_clone_dir.as_ref() {
            out.push(g);
        }
        out
    }

    /// The BGEE/BG1-phase `weidu.log` file path — exactly what BIO's
    /// `import_log_target_path` (folder-mode: `<folder>/weidu.log`) writes
    /// and `resolve_bgee_weidu_log_path` reads. Used to set
    /// `bgee_log_file` (exact-log mode) so the importer↔applier agree in
    /// that mode too.
    #[must_use]
    pub fn weidu_log_source_bgee_file(&self) -> PathBuf {
        self.weidu_log_source_bgee.join(WEIDU_LOG_FILENAME)
    }

    /// The BG2EE/BG2-phase `weidu.log` file path (the BG2EE analogue of
    /// [`Self::weidu_log_source_bgee_file`]).
    #[must_use]
    pub fn weidu_log_source_bg2ee_file(&self) -> PathBuf {
        self.weidu_log_source_bg2ee.join(WEIDU_LOG_FILENAME)
    }
}

/// Resolve the per-install directory set for `destination` + `game`
/// (pure — no disk I/O, no state mutation; the `Step1State` write +
/// `fs::create_dir_all` are [`derive_per_install_dirs`]).
///
/// `destination` is the modlist's `ModlistEntry.destination_folder`.
/// Per SPEC §13.12a everything lives inside it with the fixed §13.12
/// #2/#3/#4 names; the WeiDU-log source folders are two phase subfolders
/// of `weidu_log_source/`.
#[must_use]
pub fn resolve(destination: &str, game: Game) -> PerInstallDirs {
    let dest = Path::new(destination.trim());
    let mods_folder = dest.join(MODS_DIRNAME);
    let weidu_component_logs = dest.join(WEIDU_COMPONENT_LOGS_DIRNAME);
    let weidu_log_source_root = dest.join(WEIDU_LOG_SOURCE_DIRNAME);
    let weidu_log_source_bgee = weidu_log_source_root.join(WEIDU_LOG_SOURCE_BGEE_SUBDIR);
    let weidu_log_source_bg2ee = weidu_log_source_root.join(WEIDU_LOG_SOURCE_BG2EE_SUBDIR);

    let (eet_clone_dirs, single_game_clone_dir) = match game {
        // EET: two clone targets (#3). BG1 phase → the BGEE-named dir,
        // EET-final phase → the BG2EE-named dir (SPEC §13.12 #3 verbatim
        // fixed names).
        Game::EET => (
            Some((
                dest.join(BGEE_CLONE_DIRNAME),
                dest.join(BG2EE_CLONE_DIRNAME),
            )),
            None,
        ),
        // Single-game: one clone target (#4) named per the game.
        Game::BGEE => (None, Some(dest.join(BGEE_CLONE_DIRNAME))),
        Game::BG2EE => (None, Some(dest.join(BG2EE_CLONE_DIRNAME))),
        Game::IWDEE => (None, Some(dest.join(IWDEE_CLONE_DIRNAME))),
    };

    PerInstallDirs {
        mods_folder,
        weidu_component_logs,
        weidu_log_source_bgee,
        weidu_log_source_bg2ee,
        eet_clone_dirs,
        single_game_clone_dir,
    }
}

/// Derive + create the per-install directories for `destination` / `game`
/// and write them into the orchestrator-owned `wizard_state_step1` BIO's
/// install runner reads (SPEC §13.12a + §13.12 #2/#3/#4). Forces the
/// clone flags ON (the no-clone path is never set — SPEC §13.12a).
///
/// Returns the resolved [`PerInstallDirs`] (so the caller / tests can
/// assert the exact paths). On a directory-creation failure returns
/// `Err(String)` — an install cannot run without these dirs (they are
/// install-critical per SPEC §13.12a), so the caller must surface it and
/// **not** start the install.
///
/// **Order vs. `sync_paths_from_settings`.** This MUST run *after*
/// `sync_paths_from_settings` has populated the global source folders
/// (`bgee_game_folder` etc.), because it deliberately leaves those
/// untouched and only overrides the per-install *target* fields
/// (`mods_folder`, `eet_pre_dir`/`eet_new_dir`, `generate_directory`,
/// `weidu_log_folder`, and the WeiDU-log SOURCE fields below —
/// `bgee_log_folder` / `bg2ee_log_folder` / `eet_bgee_log_folder` /
/// `eet_bg2ee_log_folder` / `bgee_log_file` / `bg2ee_log_file`).
/// `sync_paths_from_settings` copies empty per-install targets out of the
/// redesign's Settings (which never surfaces them); this derivation
/// replaces those empties with the per-install paths.
///
/// **Order vs. `import_modlist_share_code` (the download-never-starts
/// fix).** This also MUST run *before* the share-code import. The
/// importer's `write_imported_weidu_logs` writes the code's baked-in
/// `weidu.log` to `import_log_target_path`, which reads the SAME six
/// WeiDU-log fields this sets; with BIO's `Step1State::default()` empties
/// the importer `Err`s ("Set BGEE WeiDU Log Folder before importing.") and
/// the whole Install-Modlist-paste / Reinstall pipeline aborts (the
/// reported inert "0 / 0 mods · no mods queued"). `import_modlist_share_
/// code` does `step1 = state.step1.clone()` then mutates only
/// game/mode/`sync_install_mode_flags` and `reset_workflow_keep_step1`
/// (which keeps `step1`), so the fields set here survive the import and
/// are read by *both* the importer's write AND BIO's saved-log /
/// auto-build applier's read (`apply_saved_weidu_log_selection` →
/// `resolve_*_weidu_log_path`) — the two now agree, per-install, inside
/// the destination. The fields are set for **every** game pair
/// (single-game `bgee/bg2ee_log_folder` AND EET `eet_*_log_folder`)
/// because the importer/applier branch on the post-import payload game,
/// not the `game` arg here.
pub fn derive_per_install_dirs(
    wizard_state_step1: &mut Step1State,
    destination: &str,
    game: Game,
) -> Result<PerInstallDirs, String> {
    if destination.trim().is_empty() {
        return Err(
            "destination folder is empty — cannot derive per-install directories".to_string(),
        );
    }
    let dirs = resolve(destination, game);

    // Create every per-install dir (idempotent; a re-attempt / resume
    // re-creates harmlessly). The destination itself is the user's chosen
    // folder — `create_dir_all` also creates it if absent (a fresh
    // Create → New whose folder does not yet exist).
    for dir in dirs.all_dirs() {
        std::fs::create_dir_all(dir)
            .map_err(|err| format!("create per-install dir {}: {err}", dir.display()))?;
    }

    // ── Write the per-install targets into Step1State (the exact `pub`
    //    fields BIO's command builder reads — zero BIO edit). ──

    // SPEC §13.12a Mods folder — extract/stage + scan target.
    wizard_state_step1.mods_folder = path_string(&dirs.mods_folder);

    // SPEC §13.12 #2 `-u` — per-component logs dir, always ON.
    wizard_state_step1.weidu_log_log_component = true;
    wizard_state_step1.weidu_log_folder = path_string(&dirs.weidu_component_logs);

    // ── SPEC §13.12a WeiDU-log SOURCE folders (the download-never-starts
    //    root-cause fix). The share-code importer's write target
    //    (`modlist_share.rs` `import_log_target_path`) and BIO's saved-log
    //    applier's read path (`app_step2_log.rs`
    //    `resolve_bgee/bg2_weidu_log_path`) both read these. Setting them
    //    to the per-install phase folders/files makes the importer write
    //    and the applier read resolve to the SAME file in EVERY install
    //    mode the imported payload can carry:
    //      • folder mode (`build_from_scanned_mods` /
    //        `start_from_weidu_logs_then_review_edit`): both sides resolve
    //        `<folder>/weidu.log` → set `<game>_log_folder` =
    //        `<dest>/weidu_log_source/<phase>`.
    //      • exact-log mode (`install_exactly_from_weidu_logs`): the
    //        importer writes `<game>_log_file` directly and the applier
    //        reads `<game>_log_file` (guarded by `have_weidu_logs`, which
    //        is `true` in that mode) → set `<game>_log_file` =
    //        `<dest>/weidu_log_source/<phase>/weidu.log` (== the folder
    //        mode's resolved file, so the two modes are interchangeable
    //        and the importer↔applier still agree).
    //    EET writes BOTH a BGEE-phase and a BG2EE-phase log, so the
    //    `eet_*` pair points at the two DISTINCT phase folders (a single
    //    shared folder ⇒ both `<f>/weidu.log` ⇒ the BG2EE write clobbers
    //    the BGEE write). Single-game uses only the matching one. ALL
    //    pairs are set unconditionally (not gated on the `game` arg)
    //    because the importer/applier branch on the *post-import payload*
    //    game (`import_modlist_share_code` sets `game_install` from the
    //    payload), not the registry entry's game known here. Zero BIO
    //    edit — every field is a pre-existing `pub` `Step1State` field
    //    BIO's own importer/applier already read.
    let bgee_dir = path_string(&dirs.weidu_log_source_bgee);
    let bg2ee_dir = path_string(&dirs.weidu_log_source_bg2ee);
    let bgee_file = path_string(&dirs.weidu_log_source_bgee_file());
    let bg2ee_file = path_string(&dirs.weidu_log_source_bg2ee_file());
    // Single-game folder-mode read/write target.
    wizard_state_step1.bgee_log_folder = bgee_dir.clone();
    wizard_state_step1.bg2ee_log_folder = bg2ee_dir.clone();
    // EET folder-mode read/write target (distinct phase folders).
    wizard_state_step1.eet_bgee_log_folder = bgee_dir;
    wizard_state_step1.eet_bg2ee_log_folder = bg2ee_dir;
    // Exact-log-mode read/write target (the same resolved file as folder
    // mode, so the importer↔applier agree regardless of payload mode).
    wizard_state_step1.bgee_log_file = bgee_file;
    wizard_state_step1.bg2ee_log_file = bg2ee_file;
    // `have_weidu_logs` is authoritatively (re)derived from the payload
    // mode by `import_modlist_share_code`'s `sync_install_mode_flags()`
    // *after* this runs; set it now to the mode-consistent value so the
    // pre-import state is self-consistent and the function is correct even
    // if ever called outside the import path (e.g. fresh Create → New,
    // which keeps the default `build_from_scanned_mods` ⇒ `false`). The
    // importer↔applier agreement above does NOT depend on this flag's
    // value — both folder- and exact-mode resolved files are identical —
    // so the post-import recompute cannot break it.
    wizard_state_step1.have_weidu_logs = wizard_state_step1.uses_source_weidu_logs();

    match (&dirs.eet_clone_dirs, &dirs.single_game_clone_dir) {
        // SPEC §13.12 #3 — EET clones forced ON (`-p` / `-n`). The source
        // game folders stay from Settings → Paths (`bgee_game_folder` /
        // `bg2ee_game_folder`); BIO's `build_install_invocation` uses
        // them as the clone source when these flags are ON.
        (Some((pre, fin)), None) => {
            wizard_state_step1.new_pre_eet_dir_enabled = true;
            wizard_state_step1.eet_pre_dir = path_string(pre);
            wizard_state_step1.new_eet_dir_enabled = true;
            wizard_state_step1.eet_new_dir = path_string(fin);
            // Single-game clone is irrelevant for EET — make sure a stale
            // value from a prior single-game attempt on a reused
            // WizardState cannot leak.
            wizard_state_step1.generate_directory_enabled = false;
        }
        // SPEC §13.12 #4 — single-game clone forced ON (`-g`). The source
        // folder stays from Settings → Paths.
        (None, Some(g)) => {
            wizard_state_step1.generate_directory_enabled = true;
            wizard_state_step1.generate_directory = path_string(g);
            // EET clone flags irrelevant for single-game — clear any
            // stale leftovers from a prior EET attempt.
            wizard_state_step1.new_pre_eet_dir_enabled = false;
            wizard_state_step1.new_eet_dir_enabled = false;
        }
        // `resolve` only ever returns exactly one of the two arms; this
        // is unreachable, but never silently set the no-clone path
        // (SPEC §13.12a forbids surfacing it).
        _ => {
            return Err(
                "internal: per-install clone resolution produced neither EET nor single-game \
                 dirs (the no-clone path must never be set — SPEC §13.12a)"
                    .to_string(),
            );
        }
    }

    Ok(dirs)
}

/// Remove the per-install **Mods** folder on a clean successful install
/// (SPEC §13.12a: "removed on a clean successful install (a failed/
/// cancelled install leaves it for diagnosis/resume)"). Best-effort: a
/// removal failure is logged, not fatal (the install already succeeded;
/// a leftover staging folder is a cosmetic disk-space issue, never a
/// correctness one — so it must not turn a clean install into a failure).
///
/// Only the `<dest>/mods` extract/stage folder is removed — the cloned,
/// modded game folders (#3/#4) are the install **product** and stay.
pub fn cleanup_per_install_mods_folder(destination: &str) {
    let mods = Path::new(destination.trim()).join(MODS_DIRNAME);
    if !mods.exists() {
        return;
    }
    if let Err(err) = std::fs::remove_dir_all(&mods) {
        warn!(
            target = "orchestrator",
            "post-install cleanup: removing per-install Mods folder {} failed: {err} \
             (non-fatal — the install succeeded; the staging folder is left behind)",
            mods.display()
        );
    }
}

/// `Path` → the `String` BIO's `Step1State` string fields expect. Uses
/// the platform-native representation (`Path::display` is lossy for
/// non-UTF-8, but every component here is an ASCII fixed name joined onto
/// the user's destination, which BIO already stores as a `String`).
fn path_string(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn td() -> std::path::PathBuf {
        // DATA-LOSS-safe: a unique temp path; this module never binds the
        // real `%APPDATA%\bio\` (it derives dirs under an arbitrary
        // destination — here a throwaway temp dir).
        use std::sync::atomic::{AtomicU64, Ordering};
        static C: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "bio_per_install_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn resolve_eet_uses_two_fixed_clone_dirs_no_single_game() {
        // SPEC §13.12 #3: EET clones to the BGEE-named + BG2EE-named dirs
        // (verbatim fixed names) inside the destination; no `-g` dir.
        let d = resolve(r"C:\games\my-eet", Game::EET);
        let (pre, fin) = d.eet_clone_dirs.expect("EET has two clone dirs");
        assert!(pre.ends_with(BGEE_CLONE_DIRNAME));
        assert!(fin.ends_with(BG2EE_CLONE_DIRNAME));
        assert_eq!(d.single_game_clone_dir, None);
        assert!(d.mods_folder.ends_with(MODS_DIRNAME));
        assert!(
            d.weidu_component_logs
                .ends_with(WEIDU_COMPONENT_LOGS_DIRNAME)
        );
    }

    #[test]
    fn resolve_single_game_names_match_game_no_eet() {
        // SPEC §13.12 #4: one `-g` clone dir named per the game; no EET
        // `-p`/`-n` dirs.
        for (game, name) in [
            (Game::BGEE, BGEE_CLONE_DIRNAME),
            (Game::BG2EE, BG2EE_CLONE_DIRNAME),
            (Game::IWDEE, IWDEE_CLONE_DIRNAME),
        ] {
            let d = resolve("/games/x", game);
            assert_eq!(d.eet_clone_dirs, None, "{game:?} has no EET dirs");
            let g = d.single_game_clone_dir.expect("single-game clone dir");
            assert!(g.ends_with(name), "{game:?} → {name}");
        }
    }

    #[test]
    fn derive_eet_forces_p_n_flags_and_creates_dirs_leaving_source_untouched() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut step1 = Step1State {
            // Source folders come from Settings → Paths — must survive.
            bgee_game_folder: r"S:\src\BGEE".to_string(),
            bg2ee_game_folder: r"S:\src\BG2EE".to_string(),
            // Stale single-game leftovers from a prior attempt on a reused
            // WizardState — must be cleared for an EET install.
            generate_directory_enabled: true,
            generate_directory: "stale".to_string(),
            ..Default::default()
        };

        let dirs = derive_per_install_dirs(&mut step1, &dest_s, Game::EET).expect("derive EET");

        // #3 flags forced ON; source folders untouched.
        assert!(step1.new_pre_eet_dir_enabled);
        assert!(step1.new_eet_dir_enabled);
        assert!(
            !step1.generate_directory_enabled,
            "single-game clone must be cleared for EET (no stale leak)"
        );
        assert_eq!(step1.bgee_game_folder, r"S:\src\BGEE", "source untouched");
        assert_eq!(step1.bg2ee_game_folder, r"S:\src\BG2EE", "source untouched");
        // Per-install targets written + on disk.
        assert_eq!(
            step1.eet_pre_dir,
            dirs.eet_clone_dirs.as_ref().unwrap().0.to_string_lossy()
        );
        assert_eq!(
            step1.eet_new_dir,
            dirs.eet_clone_dirs.as_ref().unwrap().1.to_string_lossy()
        );
        assert_eq!(step1.mods_folder, dest.join(MODS_DIRNAME).to_string_lossy());
        // #2 `-u` forced ON + dir under destination.
        assert!(step1.weidu_log_log_component);
        assert_eq!(
            step1.weidu_log_folder,
            dest.join(WEIDU_COMPONENT_LOGS_DIRNAME).to_string_lossy()
        );
        for d in dirs.all_dirs() {
            assert!(d.exists(), "dir created: {}", d.display());
        }

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn derive_single_game_forces_g_flag_and_clears_stale_eet() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut step1 = Step1State {
            bgee_game_folder: r"S:\src\BGEE".to_string(),
            // Stale EET leftovers — must be cleared for a single-game
            // install.
            new_pre_eet_dir_enabled: true,
            new_eet_dir_enabled: true,
            eet_pre_dir: "stale".to_string(),
            ..Default::default()
        };

        let dirs = derive_per_install_dirs(&mut step1, &dest_s, Game::BGEE).expect("derive BGEE");

        assert!(step1.generate_directory_enabled, "#4 -g forced ON");
        assert!(
            !step1.new_pre_eet_dir_enabled && !step1.new_eet_dir_enabled,
            "EET clone flags cleared for single-game (no stale leak)"
        );
        assert_eq!(step1.bgee_game_folder, r"S:\src\BGEE", "source untouched");
        assert_eq!(
            step1.generate_directory,
            dirs.single_game_clone_dir
                .as_ref()
                .unwrap()
                .to_string_lossy()
        );
        for d in dirs.all_dirs() {
            assert!(d.exists());
        }

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn no_clone_path_is_never_set() {
        // SPEC §13.12a: the redesign never surfaces BIO's
        // install-into-a-clean-dir-without-cloning path. After derive,
        // for ANY game exactly one clone mode is ON (never both-off).
        for game in [Game::BGEE, Game::BG2EE, Game::IWDEE, Game::EET] {
            let dest = td();
            let dest_s = dest.to_string_lossy().into_owned();
            let mut step1 = Step1State::default();
            derive_per_install_dirs(&mut step1, &dest_s, game).expect("derive");
            let eet_on = step1.new_pre_eet_dir_enabled && step1.new_eet_dir_enabled;
            let single_on = step1.generate_directory_enabled;
            assert!(
                eet_on ^ single_on,
                "{game:?}: exactly one clone mode must be ON (no-clone never set)"
            );
            let _ = std::fs::remove_dir_all(&dest);
        }
    }

    #[test]
    fn empty_destination_is_an_error() {
        let mut step1 = Step1State::default();
        assert!(derive_per_install_dirs(&mut step1, "   ", Game::EET).is_err());
    }

    // ───── SPEC §13.12a WeiDU-log SOURCE folders (the Install-Modlist-
    //       paste / Reinstall "download never starts / inert 0/0" root-
    //       cause fix). In-memory `Step1State` only — pure derivation +
    //       BIO's pure `pub(crate)` `resolve_*_weidu_log_path` reader; no
    //       store, no real `%APPDATA%\bio` (DATA-LOSS-safe by
    //       construction; the only I/O is `create_dir_all` under a
    //       throwaway temp destination). ─────

    #[test]
    fn resolve_places_two_distinct_weidu_log_source_phase_folders_under_dest() {
        // SPEC §13.12a: per-install, inside the destination, derived. The
        // two phase folders must be DISTINCT (an EET import writes a
        // BGEE-phase AND a BG2EE-phase log — a shared folder ⇒ the second
        // clobbers the first).
        let d = resolve(r"C:\games\m", Game::EET);
        assert!(
            d.weidu_log_source_bgee.ends_with(format!(
                "{WEIDU_LOG_SOURCE_DIRNAME}/{WEIDU_LOG_SOURCE_BGEE_SUBDIR}"
            )) || d.weidu_log_source_bgee.ends_with(format!(
                "{WEIDU_LOG_SOURCE_DIRNAME}\\{WEIDU_LOG_SOURCE_BGEE_SUBDIR}"
            )),
            "{}",
            d.weidu_log_source_bgee.display()
        );
        assert!(
            d.weidu_log_source_bg2ee
                .ends_with(WEIDU_LOG_SOURCE_BG2EE_SUBDIR)
        );
        assert_ne!(
            d.weidu_log_source_bgee, d.weidu_log_source_bg2ee,
            "the two phase folders MUST be distinct (no EET log clobber)"
        );
        // Both are inside the destination, and distinct from the Mods /
        // `-u` / clone dirs.
        assert!(d.weidu_log_source_bgee.starts_with(r"C:\games\m"));
        assert_ne!(d.weidu_log_source_bgee, d.mods_folder);
        assert_ne!(d.weidu_log_source_bgee, d.weidu_component_logs);
        // The file helpers join `weidu.log` (what BIO's importer/applier
        // do).
        assert!(d.weidu_log_source_bgee_file().ends_with(WEIDU_LOG_FILENAME));
        assert_eq!(
            d.weidu_log_source_bgee_file().parent().unwrap(),
            d.weidu_log_source_bgee
        );
    }

    /// The exact resolution `modlist_share.rs::import_log_target_path`
    /// performs (it is a private BIO fn — we replicate its observable
    /// contract verbatim so this test fails the moment the post-derive
    /// state would make the importer write somewhere the applier does not
    /// read). `bgee == true` ⇒ the BGEE/BG1-phase log target.
    fn importer_write_target(s: &Step1State, bgee: bool) -> Result<std::path::PathBuf, String> {
        if s.installs_exactly_from_weidu_logs() {
            let v = if bgee {
                &s.bgee_log_file
            } else {
                &s.bg2ee_log_file
            };
            if v.trim().is_empty() {
                return Err("empty log file".into());
            }
            return Ok(std::path::PathBuf::from(v.trim()));
        }
        let v = match (s.game_install.as_str(), bgee) {
            ("EET", true) => &s.eet_bgee_log_folder,
            ("EET", false) => &s.eet_bg2ee_log_folder,
            (_, true) => &s.bgee_log_folder,
            (_, false) => &s.bg2ee_log_folder,
        };
        if v.trim().is_empty() {
            return Err("empty log folder".into());
        }
        Ok(std::path::PathBuf::from(v.trim()).join(WEIDU_LOG_FILENAME))
    }

    #[test]
    fn default_step1_breaks_the_importer_and_applier_baseline() {
        // The pre-fix baseline the brief calls out: with BIO's
        // `Step1State::default()` the importer write target ERRs ("Set
        // BGEE WeiDU Log Folder before importing.") and the applier read
        // path is NONE — the whole pipeline aborts ⇒ inert "0 / 0 mods".
        let s = Step1State::default(); // build_from_scanned_mods, empties
        assert!(
            importer_write_target(&s, true).is_err(),
            "pre-fix: importer write target Errs on default Step1State"
        );
        assert!(
            crate::app::app_step2_log::resolve_bgee_weidu_log_path(&s).is_none(),
            "pre-fix: applier read path is None on default Step1State"
        );
    }

    #[test]
    fn derive_makes_importer_and_applier_agree_under_dest_every_mode_and_game() {
        // THE root-cause proof. After `derive_per_install_dirs`, for every
        // install mode the imported payload can carry AND every game pair,
        // the importer's write target == the applier's read path AND both
        // are non-empty + under the destination (previously Err / None).
        let modes = [
            Step1State::INSTALL_MODE_BUILD_FROM_SCANNED_MODS,
            Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS,
            Step1State::INSTALL_MODE_WEIDU_LOGS_REVIEW_EDIT,
        ];
        for mode in modes {
            // ── Single-game (BGEE) ──────────────────────────────────────
            {
                let dest = td();
                let dest_s = dest.to_string_lossy().into_owned();
                let mut s = Step1State::default();
                derive_per_install_dirs(&mut s, &dest_s, Game::BGEE).expect("derive BGEE");
                // Simulate `import_modlist_share_code`'s post-derive state
                // handling: it sets game/mode from the payload then
                // `sync_install_mode_flags()` (recomputes `have_weidu_logs`).
                s.game_install = "BGEE".to_string();
                s.install_mode = mode.to_string();
                s.sync_install_mode_flags();

                let w = importer_write_target(&s, true).expect("BGEE write target");
                let r = crate::app::app_step2_log::resolve_bgee_weidu_log_path(&s)
                    .expect("BGEE applier read path");
                assert_eq!(
                    w, r,
                    "BGEE/{mode}: importer write target must equal applier read path"
                );
                assert!(
                    w.starts_with(dest.join(WEIDU_LOG_SOURCE_DIRNAME)),
                    "BGEE/{mode}: target {} not under the per-install WeiDU-log source dir",
                    w.display()
                );
                let _ = std::fs::remove_dir_all(&dest);
            }
            // ── EET (BGEE-phase AND BG2EE-phase, distinct, no clobber) ──
            {
                let dest = td();
                let dest_s = dest.to_string_lossy().into_owned();
                let mut s = Step1State::default();
                derive_per_install_dirs(&mut s, &dest_s, Game::EET).expect("derive EET");
                s.game_install = "EET".to_string();
                s.install_mode = mode.to_string();
                s.sync_install_mode_flags();

                let wb = importer_write_target(&s, true).expect("EET BGEE write");
                let rb = crate::app::app_step2_log::resolve_bgee_weidu_log_path(&s)
                    .expect("EET BGEE read");
                let w2 = importer_write_target(&s, false).expect("EET BG2EE write");
                let r2 = crate::app::app_step2_log::resolve_bg2_weidu_log_path(&s)
                    .expect("EET BG2EE read");
                assert_eq!(wb, rb, "EET/{mode}: BGEE-phase importer == applier");
                assert_eq!(w2, r2, "EET/{mode}: BG2EE-phase importer == applier");
                assert_ne!(
                    wb, w2,
                    "EET/{mode}: the two phase logs MUST write to distinct files \
                     (else the BG2EE write clobbers the BGEE write)"
                );
                assert!(wb.starts_with(dest.join(WEIDU_LOG_SOURCE_DIRNAME)));
                assert!(w2.starts_with(dest.join(WEIDU_LOG_SOURCE_DIRNAME)));
                let _ = std::fs::remove_dir_all(&dest);
            }
        }
    }

    #[test]
    fn derive_sets_have_weidu_logs_consistently_and_creates_the_source_dirs() {
        // `have_weidu_logs` is mode-consistent post-derive (the import's
        // `sync_install_mode_flags()` re-derives it identically from the
        // payload mode; the importer↔applier agreement does NOT depend on
        // it). And the two phase folders are created on disk (BIO's
        // importer `fs::write`s into them; the applier `LogFile::from_path`
        // reads them).
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut s = Step1State::default(); // build_from_scanned_mods
        let dirs = derive_per_install_dirs(&mut s, &dest_s, Game::EET).expect("derive");
        assert!(
            !s.have_weidu_logs,
            "build_from_scanned_mods ⇒ have_weidu_logs false (mode-consistent)"
        );
        // Exact-log mode ⇒ true.
        let mut s2 = Step1State::default();
        derive_per_install_dirs(&mut s2, &dest_s, Game::BGEE).expect("derive");
        s2.install_mode = Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS.to_string();
        s2.sync_install_mode_flags();
        assert!(
            s2.have_weidu_logs,
            "install_exactly_from_weidu_logs ⇒ have_weidu_logs true"
        );
        // Both phase folders exist on disk + are in `all_dirs()`.
        assert!(dirs.weidu_log_source_bgee.exists());
        assert!(dirs.weidu_log_source_bg2ee.exists());
        assert!(
            dirs.all_dirs()
                .contains(&dirs.weidu_log_source_bgee.as_path()),
            "the WeiDU-log source folders must be in the created set"
        );
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn weidu_log_source_fields_survive_import_step1_handling() {
        // FIX relies on `import_modlist_share_code` preserving these six
        // fields: it does `step1 = state.step1.clone()`, mutates only
        // game/mode/`sync_install_mode_flags`, writes back, then
        // `reset_workflow_keep_step1()` (keeps `step1`). Pin that
        // invariant so a future BIO change to the import/reset path that
        // clobbered these would fail HERE (zero BIO edit; defense-in-depth
        // — the derivation runs BEFORE the import by contract).
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut s = crate::app::state::WizardState::default();
        derive_per_install_dirs(&mut s.step1, &dest_s, Game::EET).expect("derive");
        let snap = (
            s.step1.bgee_log_folder.clone(),
            s.step1.bg2ee_log_folder.clone(),
            s.step1.eet_bgee_log_folder.clone(),
            s.step1.eet_bg2ee_log_folder.clone(),
            s.step1.bgee_log_file.clone(),
            s.step1.bg2ee_log_file.clone(),
        );
        assert!(
            !snap.0.is_empty() && !snap.4.is_empty(),
            "derived non-empty"
        );
        // Simulate BIO's import step1 handling.
        let cloned = s.step1.clone();
        s.step1 = cloned;
        s.step1.game_install = "EET".to_string();
        s.step1.install_mode = Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS.to_string();
        s.step1.sync_install_mode_flags();
        s.reset_workflow_keep_step1();
        assert_eq!(
            (
                s.step1.bgee_log_folder.clone(),
                s.step1.bg2ee_log_folder.clone(),
                s.step1.eet_bgee_log_folder.clone(),
                s.step1.eet_bg2ee_log_folder.clone(),
                s.step1.bgee_log_file.clone(),
                s.step1.bg2ee_log_file.clone(),
            ),
            snap,
            "the six WeiDU-log source fields must survive the import's \
             clone + sync_install_mode_flags + reset_workflow_keep_step1"
        );
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn cleanup_removes_only_the_mods_folder() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut step1 = Step1State::default();
        let dirs = derive_per_install_dirs(&mut step1, &dest_s, Game::BGEE).expect("derive");
        assert!(dirs.mods_folder.exists());
        let clone = dirs.single_game_clone_dir.clone().unwrap();
        assert!(clone.exists());

        cleanup_per_install_mods_folder(&dest_s);

        assert!(
            !dirs.mods_folder.exists(),
            "Mods staging folder removed on clean success"
        );
        assert!(
            clone.exists(),
            "the cloned game folder (#4) is the install product — must NOT be removed"
        );
        // Idempotent: a second cleanup on an already-removed folder is a
        // silent no-op (not an error).
        cleanup_per_install_mods_folder(&dest_s);

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn weidu_component_logs_resolves_inside_the_destination_not_appdata() {
        // THE AMENDMENT (2026-05-18) regression guard. Upstream `a38e360`
        // (merged `8df994a`) removed WeiDU's "-u log folder cannot contain
        // spaces" preflight from `state_validation_paths.rs`, so the prior
        // Fix-A relocation of the `-u` `weidu_component_logs` dir to
        // `%APPDATA%\bio\modlists\<id>\` is unwound: the `-u` dir is
        // `<destination>/weidu_component_logs` again (original SPEC §13.12
        // #2 / §13.12a per-install-inside-destination), even when the
        // user's free-form destination contains a space. This test fails
        // the moment any future change re-relocates the `-u` dir out of the
        // destination. DATA-LOSS-safe: `resolve` derives purely from the
        // passed destination string — it never calls `app_config_dir()` /
        // touches the real `%APPDATA%\bio\`.
        let dest_with_space = r"C:\Games\test oli rp";
        let d = resolve(dest_with_space, Game::EET);
        assert_eq!(
            d.weidu_component_logs,
            Path::new(dest_with_space).join(WEIDU_COMPONENT_LOGS_DIRNAME),
            "AMENDMENT: the `-u` weidu_component_logs dir is inside the \
             destination (NOT an appdata path)"
        );
        assert!(
            d.weidu_component_logs.starts_with(dest_with_space),
            "AMENDMENT: the `-u` dir must be under the destination"
        );
        // It is a sibling of the Mods + WeiDU-log SOURCE + clone dirs, all
        // under the destination (the only per-install layout post-AMENDMENT
        // — there is no longer a lone appdata exception).
        assert!(d.mods_folder.starts_with(dest_with_space));
        assert!(d.weidu_log_source_bgee.starts_with(dest_with_space));
        assert!(d.weidu_log_source_bg2ee.starts_with(dest_with_space));

        // And the `Step1State` field BIO's `-u` derivation reads
        // (`weidu_log_folder`) points at that same in-destination path.
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut step1 = Step1State::default();
        let dirs = derive_per_install_dirs(&mut step1, &dest_s, Game::BGEE).expect("derive");
        assert!(step1.weidu_log_log_component, "#2 `-u` stays forced ON");
        assert_eq!(
            step1.weidu_log_folder,
            dirs.weidu_component_logs.to_string_lossy(),
            "AMENDMENT: weidu_log_folder == <dest>/weidu_component_logs"
        );
        assert!(
            step1.weidu_log_folder.starts_with(&dest_s),
            "AMENDMENT: weidu_log_folder is inside the destination"
        );
        let _ = std::fs::remove_dir_all(&dest);
    }
}
