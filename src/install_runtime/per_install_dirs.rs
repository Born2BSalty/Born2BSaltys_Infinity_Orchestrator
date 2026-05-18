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
    /// EET only: `(pre_eet_dir, eet_final_dir)` — `-p` / `-n` clone
    /// targets (SPEC §13.12 #3). `None` for single-game.
    pub eet_clone_dirs: Option<(PathBuf, PathBuf)>,
    /// Single-game only: the `-g` clone target (SPEC §13.12 #4). `None`
    /// for EET.
    pub single_game_clone_dir: Option<PathBuf>,
}

impl PerInstallDirs {
    /// Every directory this resolution requires, in creation order
    /// (parents-first by construction — they are all direct children of
    /// the destination, which the caller guarantees exists).
    fn all_dirs(&self) -> Vec<&Path> {
        let mut out: Vec<&Path> = vec![&self.mods_folder, &self.weidu_component_logs];
        if let Some((pre, fin)) = self.eet_clone_dirs.as_ref() {
            out.push(pre);
            out.push(fin);
        }
        if let Some(g) = self.single_game_clone_dir.as_ref() {
            out.push(g);
        }
        out
    }
}

/// Resolve the per-install directory set for `destination` + `game`
/// (pure — no disk I/O, no state mutation; the `Step1State` write +
/// `fs::create_dir_all` are [`derive_per_install_dirs`]).
///
/// `destination` is the modlist's `ModlistEntry.destination_folder`.
/// Per SPEC §13.12a everything is a direct child of it with the fixed
/// §13.12 #2/#3/#4 names.
#[must_use]
pub fn resolve(destination: &str, game: Game) -> PerInstallDirs {
    let dest = Path::new(destination.trim());
    let mods_folder = dest.join(MODS_DIRNAME);
    let weidu_component_logs = dest.join(WEIDU_COMPONENT_LOGS_DIRNAME);

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
/// `weidu_log_folder`). `sync_paths_from_settings` copies empty
/// per-install targets out of the redesign's Settings (which never
/// surfaces them); this derivation replaces those empties with the
/// per-install paths.
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
}
