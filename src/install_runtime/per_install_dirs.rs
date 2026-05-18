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
//   - **#2 `-u` per-component logs** —
//     `%APPDATA%\bio\modlists\<id>\weidu_component_logs` (the per-modlist
//     **appdata** dir that already holds `workspace.json`, resolved via
//     `registry::store_workspace::modlist_data_dir` — **NOT** the user's
//     destination), with `weidu_log_log_component = true` (and
//     `weidu_log_mode_enabled` stays BIO's default `true`). BIO derives
//     `-u` from `weidu_log_folder` + `weidu_log_log_component`
//     (`state_validation_paths.rs:77-92`); the user does not configure the
//     path (SPEC §13.12 #2).
//
//     **Why this dir — and ONLY this dir — moved out of the destination
//     (user-approved Fix A, 2026-05-18).** BIO's `-u`-mode preflight
//     (`state_validation_paths.rs:84-92`) rejects a `weidu_log_folder`
//     containing a space ("WeiDU log folder in -u mode cannot contain
//     spaces on this backend"). Destinations are user-free-form (e.g.
//     `…\test oli rp`), so `<dest>/weidu_component_logs` inherits any space
//     in the destination ⇒ the preflight fails ⇒ auto-build stops ⇒ a
//     permanent inert 0/0. The per-modlist appdata dir is program-
//     controlled (`%APPDATA%\bio\modlists\<id>` — `<id>` is a generated
//     12-char ULID-style token, `%APPDATA%` is `…\Roaming\…`, none of
//     which contain spaces on a sane Windows profile), so the `-u` dir
//     gets a no-space anchor while the modlist's reproducibility data
//     (`workspace.json`) already lives in the same place. **Premise-checked
//     (run report):** `run_path_check` has exactly ONE no-space-class
//     preflight and it is `weidu_log_folder`-only — the Mods folder, the
//     WeiDU-log SOURCE folders, and the game-clone dirs are NOT
//     space-restricted, so they all correctly stay in the destination.
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
/// (SPEC §13.12 #2). Joined onto the per-modlist **appdata** dir
/// (`%APPDATA%\bio\modlists\<id>\weidu_component_logs`) — **not** the
/// destination — because WeiDU's `-u` backend forbids spaces and the
/// destination is user-free-form (user-approved Fix A; see the module
/// header). Resolved via `registry::store_workspace::modlist_data_dir`.
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

/// Resolve the per-install directory set for `modlist_id` + `destination`
/// + `game` (pure — no disk I/O, no state mutation; the `Step1State` write
/// + `fs::create_dir_all` are [`derive_per_install_dirs`]).
///
/// `destination` is the modlist's `ModlistEntry.destination_folder`;
/// `modlist_id` is the registry entry id (the same id `workspace.json` is
/// keyed by). Per SPEC §13.12a the Mods folder, the WeiDU-log SOURCE
/// folders, and the game-clone dirs live **inside the destination** with
/// the fixed §13.12 #3/#4 names; the **`-u` `weidu_component_logs` dir is
/// the lone exception** — it resolves to the per-modlist **appdata** dir
/// (`%APPDATA%\bio\modlists\<id>\weidu_component_logs`, via
/// `registry::store_workspace::modlist_data_dir` — the SAME resolver that
/// yields `workspace.json`'s parent) so WeiDU's no-space `-u` preflight is
/// not defeated by a space in the user's free-form destination
/// (user-approved Fix A — see the module header).
#[must_use]
pub fn resolve(modlist_id: &str, destination: &str, game: Game) -> PerInstallDirs {
    // SPEC §13.12 #2 / §13.12a + Fix A: the `-u` `weidu_component_logs` dir
    // is the ONLY per-install dir NOT under the destination — it lives in
    // the per-modlist appdata dir (no-space, program-controlled) alongside
    // `workspace.json`. Resolve that dir via the **canonical**
    // `registry::store_workspace::modlist_data_dir` (the SAME resolver that
    // yields `workspace.json`'s parent) — never hand-join `%APPDATA%` (the
    // brief's hard constraint). The pure path-join is factored into
    // [`resolve_with_data_dir`] so tests can exercise the relocation logic
    // against a throwaway temp data-dir without touching the real
    // `%APPDATA%\bio\` (DATA-LOSS-safe by construction).
    let modlist_data_dir = crate::registry::store_workspace::modlist_data_dir(modlist_id);
    resolve_with_data_dir(&modlist_data_dir, destination, game)
}

/// Pure core of [`resolve`]: given the **already-resolved per-modlist
/// appdata data dir** (`<app_config_dir>/modlists/<id>/`) and the
/// destination, compute the per-install dir set. The `-u`
/// `weidu_component_logs` dir is `<modlist_data_dir>/weidu_component_logs`
/// (Fix A — no-space anchor); every other per-install dir is under
/// `destination`. No disk I/O, no `app_config_dir()` call ⇒ tests pass a
/// temp `modlist_data_dir` and never bind the real `%APPDATA%\bio\`.
#[must_use]
pub fn resolve_with_data_dir(
    modlist_data_dir: &Path,
    destination: &str,
    game: Game,
) -> PerInstallDirs {
    let dest = Path::new(destination.trim());
    let mods_folder = dest.join(MODS_DIRNAME);
    let weidu_component_logs = modlist_data_dir.join(WEIDU_COMPONENT_LOGS_DIRNAME);
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
///
/// `modlist_id` is the registry entry id. It anchors **only** the `-u`
/// `weidu_component_logs` dir to the per-modlist appdata dir
/// (`%APPDATA%\bio\modlists\<id>\weidu_component_logs` — the no-space,
/// program-controlled Fix-A relocation; every other per-install dir still
/// derives from `destination`). It must be the **same id** the modlist's
/// `workspace.json` is keyed by, so the `-u` logs sit beside the
/// reproducibility data. The caller supplies it from whichever anchor it
/// owns (the Workspace path's `modlist_id`; the Reinstall path's
/// `pending_reinstall_id`; the fresh Install-Modlist-paste path mints it
/// once and threads the same id here and into the registration — see the
/// run-report PLAN GAP).
pub fn derive_per_install_dirs(
    wizard_state_step1: &mut Step1State,
    modlist_id: &str,
    destination: &str,
    game: Game,
) -> Result<PerInstallDirs, String> {
    if modlist_id.trim().is_empty() {
        return Err(
            "modlist id is empty — cannot resolve the per-modlist appdata `-u` log dir \
             (SPEC §13.12 #2 / Fix A)"
                .to_string(),
        );
    }
    // Canonical per-modlist appdata data dir (the SAME resolver that yields
    // `workspace.json`'s parent — never hand-join `%APPDATA%`). The pure
    // I/O + `Step1State`-write core is [`derive_per_install_dirs_with_data_
    // dir`] so tests inject a temp data dir (DATA-LOSS-safe — no real
    // `%APPDATA%\bio\` write).
    let modlist_data_dir = crate::registry::store_workspace::modlist_data_dir(modlist_id);
    derive_per_install_dirs_with_data_dir(wizard_state_step1, &modlist_data_dir, destination, game)
}

/// Pure I/O + `Step1State`-write core of [`derive_per_install_dirs`]: takes
/// the **already-resolved per-modlist appdata data dir** so it never calls
/// `app_config_dir()` (tests pass a throwaway temp dir ⇒ the relocated `-u`
/// `weidu_component_logs` dir + every other per-install dir land under
/// temp paths, never the real `%APPDATA%\bio\`). Production reaches this
/// only through [`derive_per_install_dirs`] (canonical resolver).
pub fn derive_per_install_dirs_with_data_dir(
    wizard_state_step1: &mut Step1State,
    modlist_data_dir: &Path,
    destination: &str,
    game: Game,
) -> Result<PerInstallDirs, String> {
    if destination.trim().is_empty() {
        return Err(
            "destination folder is empty — cannot derive per-install directories".to_string(),
        );
    }
    let dirs = resolve_with_data_dir(modlist_data_dir, destination, game);

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

    // SPEC §13.12 #2 `-u` — per-component logs dir, always ON. Points at
    // the per-modlist APPDATA dir (`%APPDATA%\bio\modlists\<id>\
    // weidu_component_logs`), NOT the destination (Fix A — WeiDU's `-u`
    // no-space preflight `state_validation_paths.rs:84-92` would reject a
    // space-containing destination; the appdata path is program-controlled
    // and space-free). `resolve` already routed `dirs.weidu_component_logs`
    // through `modlist_data_dir(modlist_id)`.
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
        // real `%APPDATA%\bio\` (every test goes through the
        // `_with_data_dir` cores with a throwaway temp data dir + a
        // throwaway temp destination — `app_config_dir()` is never called,
        // so `cargo test --lib` cannot touch the user's `%APPDATA%\bio\`).
        use std::sync::atomic::{AtomicU64, Ordering};
        static C: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "bio_per_install_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ))
    }

    /// A throwaway temp **per-modlist data dir** — what
    /// `registry::store_workspace::modlist_data_dir(id)` resolves to in
    /// production (`%APPDATA%\bio\modlists\<id>\`), here a temp path so the
    /// Fix-A `-u`-dir relocation is exercised WITHOUT writing the real
    /// config dir (DATA-LOSS-safe). The `pure-core _with_data_dir` entry
    /// points take this explicitly so no test ever calls `app_config_dir()`.
    fn dd() -> std::path::PathBuf {
        td().join("modlists").join("TESTMODLISTID")
    }

    #[test]
    fn resolve_eet_uses_two_fixed_clone_dirs_no_single_game() {
        // SPEC §13.12 #3: EET clones to the BGEE-named + BG2EE-named dirs
        // (verbatim fixed names) inside the destination; no `-g` dir.
        let data = dd();
        let d = resolve_with_data_dir(&data, r"C:\games\my-eet", Game::EET);
        let (pre, fin) = d.eet_clone_dirs.expect("EET has two clone dirs");
        assert!(pre.ends_with(BGEE_CLONE_DIRNAME));
        assert!(fin.ends_with(BG2EE_CLONE_DIRNAME));
        assert_eq!(d.single_game_clone_dir, None);
        // Mods + clone + WeiDU-log SOURCE dirs stay under the destination.
        assert!(d.mods_folder.starts_with(r"C:\games\my-eet"));
        assert!(d.mods_folder.ends_with(MODS_DIRNAME));
        // ── Fix A: the `-u` `weidu_component_logs` dir is the LONE per-
        //    install dir NOT under the destination — it is under the
        //    per-modlist appdata data dir (`modlist_data_dir(id)`), so
        //    WeiDU's no-space `-u` preflight cannot be defeated by a space
        //    in the user's free-form destination. ──
        assert!(
            d.weidu_component_logs
                .ends_with(WEIDU_COMPONENT_LOGS_DIRNAME)
        );
        assert!(
            d.weidu_component_logs.starts_with(&data),
            "Fix A: -u dir under the per-modlist appdata data dir ({}), got {}",
            data.display(),
            d.weidu_component_logs.display()
        );
        assert!(
            !d.weidu_component_logs.starts_with(r"C:\games\my-eet"),
            "Fix A: the -u dir must NOT be under the destination"
        );
    }

    #[test]
    fn resolve_single_game_names_match_game_no_eet() {
        // SPEC §13.12 #4: one `-g` clone dir named per the game; no EET
        // `-p`/`-n` dirs.
        let data = dd();
        for (game, name) in [
            (Game::BGEE, BGEE_CLONE_DIRNAME),
            (Game::BG2EE, BG2EE_CLONE_DIRNAME),
            (Game::IWDEE, IWDEE_CLONE_DIRNAME),
        ] {
            let d = resolve_with_data_dir(&data, "/games/x", game);
            assert_eq!(d.eet_clone_dirs, None, "{game:?} has no EET dirs");
            let g = d.single_game_clone_dir.expect("single-game clone dir");
            assert!(g.ends_with(name), "{game:?} → {name}");
            // Fix A: -u dir under the appdata data dir regardless of game.
            assert!(d.weidu_component_logs.starts_with(&data));
        }
    }

    #[test]
    fn derive_eet_forces_p_n_flags_and_creates_dirs_leaving_source_untouched() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let data = dd();
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

        let dirs = derive_per_install_dirs_with_data_dir(&mut step1, &data, &dest_s, Game::EET)
            .expect("derive EET");

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
        // #2 `-u` forced ON. Fix A: `weidu_log_folder` is set to the
        // per-modlist APPDATA dir (no-space), NOT the destination.
        assert!(step1.weidu_log_log_component);
        assert_eq!(
            step1.weidu_log_folder,
            data.join(WEIDU_COMPONENT_LOGS_DIRNAME).to_string_lossy(),
            "Fix A: -u dir = <modlist appdata data dir>/weidu_component_logs"
        );
        assert!(
            !step1.weidu_log_folder.starts_with(&dest_s),
            "Fix A: the -u dir must not be inside the (space-prone) destination"
        );
        for d in dirs.all_dirs() {
            assert!(d.exists(), "dir created: {}", d.display());
        }

        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn derive_single_game_forces_g_flag_and_clears_stale_eet() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let data = dd();
        let mut step1 = Step1State {
            bgee_game_folder: r"S:\src\BGEE".to_string(),
            // Stale EET leftovers — must be cleared for a single-game
            // install.
            new_pre_eet_dir_enabled: true,
            new_eet_dir_enabled: true,
            eet_pre_dir: "stale".to_string(),
            ..Default::default()
        };

        let dirs = derive_per_install_dirs_with_data_dir(&mut step1, &data, &dest_s, Game::BGEE)
            .expect("derive BGEE");

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
        // Fix A: -u dir under the appdata data dir, not the destination.
        assert!(dirs.weidu_component_logs.starts_with(&data));
        for d in dirs.all_dirs() {
            assert!(d.exists());
        }

        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn no_clone_path_is_never_set() {
        // SPEC §13.12a: the redesign never surfaces BIO's
        // install-into-a-clean-dir-without-cloning path. After derive,
        // for ANY game exactly one clone mode is ON (never both-off).
        for game in [Game::BGEE, Game::BG2EE, Game::IWDEE, Game::EET] {
            let dest = td();
            let dest_s = dest.to_string_lossy().into_owned();
            let data = dd();
            let mut step1 = Step1State::default();
            derive_per_install_dirs_with_data_dir(&mut step1, &data, &dest_s, game)
                .expect("derive");
            let eet_on = step1.new_pre_eet_dir_enabled && step1.new_eet_dir_enabled;
            let single_on = step1.generate_directory_enabled;
            assert!(
                eet_on ^ single_on,
                "{game:?}: exactly one clone mode must be ON (no-clone never set)"
            );
            let _ = std::fs::remove_dir_all(&dest);
            let _ = std::fs::remove_dir_all(&data);
        }
    }

    #[test]
    fn empty_destination_is_an_error() {
        let mut step1 = Step1State::default();
        let data = dd();
        assert!(
            derive_per_install_dirs_with_data_dir(&mut step1, &data, "   ", Game::EET).is_err()
        );
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn empty_modlist_id_is_an_error() {
        // Fix A: the `-u` dir is keyed by the modlist id (its appdata data
        // dir) — an empty id cannot resolve it, so `derive_per_install_dirs`
        // (the canonical entry that resolves the id → data dir) rejects it.
        let mut step1 = Step1State::default();
        let r = derive_per_install_dirs(&mut step1, "   ", r"C:\some\dest", Game::EET);
        assert!(
            r.is_err(),
            "empty modlist id must error (SPEC §13.12 #2 / Fix A)"
        );
        assert!(r.unwrap_err().contains("modlist id is empty"));
    }

    #[test]
    fn fix_a_space_containing_destination_yields_space_free_minus_u_dir() {
        // THE Fix-A acceptance (the user's empirical bug): a destination
        // containing a space (e.g. `…\test oli rp`) previously produced
        // `<dest>/weidu_component_logs` — a space-containing `-u` path that
        // BIO's no-space `-u` preflight (`state_validation_paths.rs:84-92`)
        // rejects, stopping auto-build → permanent inert 0/0. After Fix A
        // the `-u` dir is `<modlist appdata data dir>/weidu_component_logs`
        // — the data dir is program-controlled (no spaces), so the `-u`
        // path is space-free even when the destination has a space.
        let dest_with_space = r"C:\Games\test oli rp";
        // A realistic, space-free per-modlist appdata data dir (what
        // `modlist_data_dir(id)` yields under `%APPDATA%\bio\modlists\<id>`).
        let space_free_data = std::env::temp_dir()
            .join("bio_appdata_modlists")
            .join("ABCDEFGHIJKL");
        let d = resolve_with_data_dir(&space_free_data, dest_with_space, Game::EET);

        // The destination's space is irrelevant to the -u dir now.
        assert!(
            !d.weidu_component_logs.to_string_lossy().contains(' '),
            "Fix A: the -u dir is space-free ({}) even with a space-containing destination",
            d.weidu_component_logs.display()
        );
        assert!(
            d.weidu_component_logs.starts_with(&space_free_data),
            "the -u dir resolves under the per-modlist appdata data dir, not the destination"
        );
        assert!(
            !d.weidu_component_logs.starts_with(dest_with_space),
            "the -u dir must NOT be under the space-prone destination"
        );

        // BIO's exact `-u` no-space preflight (replicated verbatim from
        // `state_validation_paths.rs:84-92`) must now PASS for the derived
        // `weidu_log_folder`. Pre-Fix-A this Errd for a space destination.
        let mut step1 = Step1State::default();
        let dd_dir = dd();
        derive_per_install_dirs_with_data_dir(&mut step1, &dd_dir, dest_with_space, Game::EET)
            .expect("derive (dirs land under temp paths — DATA-LOSS-safe)");
        let log_dir = step1.weidu_log_folder.trim();
        assert!(
            !log_dir.is_empty() && !log_dir.contains(' '),
            "BIO's -u no-space preflight would now PASS (log dir {log_dir:?} is space-free)"
        );
        assert!(
            step1.weidu_log_log_component,
            "#2 `-u` per-component logging stays forced ON"
        );
        let _ = std::fs::remove_dir_all(&dd_dir);
    }

    #[test]
    fn weidu_log_source_dirs_still_stay_in_the_destination_after_fix_a() {
        // Fix A relocates ONLY the `-u` `weidu_component_logs` dir. The
        // WeiDU-log SOURCE folders are a DIFFERENT code path (not subject to
        // the no-space `-u` preflight — premise-checked) and MUST stay
        // inside the destination (committed f84fdcb behavior — explicitly
        // out of Fix-A scope).
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let data = dd();
        let mut s = Step1State::default();
        let dirs = derive_per_install_dirs_with_data_dir(&mut s, &data, &dest_s, Game::EET)
            .expect("derive");
        assert!(
            dirs.weidu_log_source_bgee.starts_with(&dest),
            "WeiDU-log SOURCE (BGEE) stays in the destination (NOT relocated)"
        );
        assert!(
            dirs.weidu_log_source_bg2ee.starts_with(&dest),
            "WeiDU-log SOURCE (BG2EE) stays in the destination (NOT relocated)"
        );
        assert!(
            s.bgee_log_folder.starts_with(&dest_s) || s.eet_bgee_log_folder.starts_with(&dest_s),
            "the WeiDU-log SOURCE Step1State fields still point inside the destination"
        );
        // And the -u dir is the lone exception (under the data dir).
        assert!(dirs.weidu_component_logs.starts_with(&data));
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_dir_all(&data);
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
        let d = resolve_with_data_dir(&dd(), r"C:\games\m", Game::EET);
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
                let data = dd();
                let mut s = Step1State::default();
                derive_per_install_dirs_with_data_dir(&mut s, &data, &dest_s, Game::BGEE)
                    .expect("derive BGEE");
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
                let _ = std::fs::remove_dir_all(&data);
            }
            // ── EET (BGEE-phase AND BG2EE-phase, distinct, no clobber) ──
            {
                let dest = td();
                let dest_s = dest.to_string_lossy().into_owned();
                let data = dd();
                let mut s = Step1State::default();
                derive_per_install_dirs_with_data_dir(&mut s, &data, &dest_s, Game::EET)
                    .expect("derive EET");
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
                let _ = std::fs::remove_dir_all(&data);
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
        let data = dd();
        let mut s = Step1State::default(); // build_from_scanned_mods
        let dirs = derive_per_install_dirs_with_data_dir(&mut s, &data, &dest_s, Game::EET)
            .expect("derive");
        assert!(
            !s.have_weidu_logs,
            "build_from_scanned_mods ⇒ have_weidu_logs false (mode-consistent)"
        );
        // Exact-log mode ⇒ true.
        let data2 = dd();
        let mut s2 = Step1State::default();
        derive_per_install_dirs_with_data_dir(&mut s2, &data2, &dest_s, Game::BGEE)
            .expect("derive");
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
        // Fix A: the relocated `-u` dir was also created (under the data
        // dir) — `all_dirs()` includes it; `create_dir_all` made the
        // `<temp>/modlists/<id>/` parent chain.
        assert!(
            dirs.weidu_component_logs.exists(),
            "the relocated -u dir is created under the per-modlist data dir"
        );
        assert!(dirs.weidu_component_logs.starts_with(&data));
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&data2);
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
        let data = dd();
        let mut s = crate::app::state::WizardState::default();
        derive_per_install_dirs_with_data_dir(&mut s.step1, &data, &dest_s, Game::EET)
            .expect("derive");
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
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn cleanup_removes_only_the_mods_folder() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let data = dd();
        let mut step1 = Step1State::default();
        let dirs = derive_per_install_dirs_with_data_dir(&mut step1, &data, &dest_s, Game::BGEE)
            .expect("derive");
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
        // Fix A: the relocated `-u` dir lives in the per-modlist APPDATA
        // dir, NOT under the destination — `cleanup_per_install_mods_folder`
        // only removes `<dest>/mods`, so the `-u` logs survive the clean-up
        // (they were never in the destination to begin with). The previous
        // `<dest>/weidu_component_logs` was likewise not removed; this just
        // confirms the relocated path is also untouched.
        assert!(
            dirs.weidu_component_logs.exists(),
            "the relocated -u dir (appdata, not <dest>) is untouched by Mods-folder cleanup"
        );
        // Idempotent: a second cleanup on an already-removed folder is a
        // silent no-op (not an error).
        cleanup_per_install_mods_folder(&dest_s);

        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_dir_all(&data);
    }
}
