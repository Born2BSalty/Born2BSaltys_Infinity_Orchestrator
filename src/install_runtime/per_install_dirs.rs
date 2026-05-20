// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use tracing::warn;

use crate::app::state::Step1State;
use crate::registry::model::Game;

pub const MODS_DIRNAME: &str = "mods";

pub const WEIDU_COMPONENT_LOGS_DIRNAME: &str = "weidu_component_logs";

pub const WEIDU_LOG_SOURCE_DIRNAME: &str = "weidu_log_source";

pub const WEIDU_LOG_SOURCE_BGEE_SUBDIR: &str = "bgee";

pub const WEIDU_LOG_SOURCE_BG2EE_SUBDIR: &str = "bg2ee";

pub const WEIDU_LOG_FILENAME: &str = "weidu.log";

pub const BGEE_CLONE_DIRNAME: &str = "Baldur's Gate Enhanced Edition";
pub const BG2EE_CLONE_DIRNAME: &str = "Baldur's Gate II Enhanced Edition";
pub const IWDEE_CLONE_DIRNAME: &str = "Icewind Dale Enhanced Edition";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerInstallDirs {
    pub mods_folder: PathBuf,

    pub weidu_component_logs: PathBuf,

    pub weidu_log_source_bgee: PathBuf,

    pub weidu_log_source_bg2ee: PathBuf,

    pub eet_clone_dirs: Option<(PathBuf, PathBuf)>,

    pub single_game_clone_dir: Option<PathBuf>,
}

impl PerInstallDirs {
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

    #[must_use]
    pub fn weidu_log_source_bgee_file(&self) -> PathBuf {
        self.weidu_log_source_bgee.join(WEIDU_LOG_FILENAME)
    }

    #[must_use]
    pub fn weidu_log_source_bg2ee_file(&self) -> PathBuf {
        self.weidu_log_source_bg2ee.join(WEIDU_LOG_FILENAME)
    }
}

#[must_use]
pub fn resolve(destination: &str, game: Game) -> PerInstallDirs {
    let dest = Path::new(destination.trim());
    let mods_folder = dest.join(MODS_DIRNAME);
    let weidu_component_logs = dest.join(WEIDU_COMPONENT_LOGS_DIRNAME);
    let weidu_log_source_root = dest.join(WEIDU_LOG_SOURCE_DIRNAME);
    let phase_one_log_source = weidu_log_source_root.join(WEIDU_LOG_SOURCE_BGEE_SUBDIR);
    let phase_two_log_source = weidu_log_source_root.join(WEIDU_LOG_SOURCE_BG2EE_SUBDIR);

    let (eet_clone_dirs, single_game_clone_dir) = match game {
        Game::EET => (
            Some((
                dest.join(BGEE_CLONE_DIRNAME),
                dest.join(BG2EE_CLONE_DIRNAME),
            )),
            None,
        ),

        Game::BGEE => (None, Some(dest.join(BGEE_CLONE_DIRNAME))),
        Game::BG2EE => (None, Some(dest.join(BG2EE_CLONE_DIRNAME))),
        Game::IWDEE => (None, Some(dest.join(IWDEE_CLONE_DIRNAME))),
    };

    PerInstallDirs {
        mods_folder,
        weidu_component_logs,
        weidu_log_source_bgee: phase_one_log_source,
        weidu_log_source_bg2ee: phase_two_log_source,
        eet_clone_dirs,
        single_game_clone_dir,
    }
}

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

    for dir in dirs.all_dirs() {
        std::fs::create_dir_all(dir)
            .map_err(|err| format!("create per-install dir {}: {err}", dir.display()))?;
    }

    wizard_state_step1.mods_folder = path_string(&dirs.mods_folder);

    wizard_state_step1.weidu_log_log_component = true;
    wizard_state_step1.weidu_log_folder = path_string(&dirs.weidu_component_logs);

    crate::ui::step1::service_step1::sync_weidu_log_mode(wizard_state_step1);

    let phase_one_log_dir = path_string(&dirs.weidu_log_source_bgee);
    let phase_two_log_dir = path_string(&dirs.weidu_log_source_bg2ee);
    let phase_one_log_file = path_string(&dirs.weidu_log_source_bgee_file());
    let phase_two_log_file = path_string(&dirs.weidu_log_source_bg2ee_file());

    wizard_state_step1.bgee_log_folder = phase_one_log_dir.clone();
    wizard_state_step1.bg2ee_log_folder = phase_two_log_dir.clone();

    wizard_state_step1.eet_bgee_log_folder = phase_one_log_dir;
    wizard_state_step1.eet_bg2ee_log_folder = phase_two_log_dir;

    wizard_state_step1.bgee_log_file = phase_one_log_file;
    wizard_state_step1.bg2ee_log_file = phase_two_log_file;

    wizard_state_step1.have_weidu_logs = wizard_state_step1.uses_source_weidu_logs();

    match (&dirs.eet_clone_dirs, &dirs.single_game_clone_dir) {
        (Some((pre, fin)), None) => {
            wizard_state_step1.new_pre_eet_dir_enabled = true;
            wizard_state_step1.eet_pre_dir = path_string(pre);
            wizard_state_step1.new_eet_dir_enabled = true;
            wizard_state_step1.eet_new_dir = path_string(fin);

            wizard_state_step1.generate_directory_enabled = false;
        }

        (None, Some(g)) => {
            wizard_state_step1.generate_directory_enabled = true;
            wizard_state_step1.generate_directory = path_string(g);

            wizard_state_step1.new_pre_eet_dir_enabled = false;
            wizard_state_step1.new_eet_dir_enabled = false;
        }

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

fn path_string(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn td() -> std::path::PathBuf {
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
            bgee_game_folder: r"S:\src\BGEE".to_string(),
            bg2ee_game_folder: r"S:\src\BG2EE".to_string(),

            generate_directory_enabled: true,
            generate_directory: "stale".to_string(),
            ..Default::default()
        };

        let dirs = derive_per_install_dirs(&mut step1, &dest_s, Game::EET).expect("derive EET");

        assert!(step1.new_pre_eet_dir_enabled);
        assert!(step1.new_eet_dir_enabled);
        assert!(
            !step1.generate_directory_enabled,
            "single-game clone must be cleared for EET (no stale leak)"
        );
        assert_eq!(step1.bgee_game_folder, r"S:\src\BGEE", "source untouched");
        assert_eq!(step1.bg2ee_game_folder, r"S:\src\BG2EE", "source untouched");

        assert_eq!(
            step1.eet_pre_dir,
            dirs.eet_clone_dirs.as_ref().unwrap().0.to_string_lossy()
        );
        assert_eq!(
            step1.eet_new_dir,
            dirs.eet_clone_dirs.as_ref().unwrap().1.to_string_lossy()
        );
        assert_eq!(step1.mods_folder, dest.join(MODS_DIRNAME).to_string_lossy());

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
    fn resolve_places_two_distinct_weidu_log_source_phase_folders_under_dest() {
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

        assert!(d.weidu_log_source_bgee.starts_with(r"C:\games\m"));
        assert_ne!(d.weidu_log_source_bgee, d.mods_folder);
        assert_ne!(d.weidu_log_source_bgee, d.weidu_component_logs);

        assert!(d.weidu_log_source_bgee_file().ends_with(WEIDU_LOG_FILENAME));
        assert_eq!(
            d.weidu_log_source_bgee_file().parent().unwrap(),
            d.weidu_log_source_bgee
        );
    }

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
        let s = Step1State::default();
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
        let modes = [
            Step1State::INSTALL_MODE_BUILD_FROM_SCANNED_MODS,
            Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS,
            Step1State::INSTALL_MODE_WEIDU_LOGS_REVIEW_EDIT,
        ];
        for mode in modes {
            {
                let dest = td();
                let dest_s = dest.to_string_lossy().into_owned();
                let mut s = Step1State::default();
                derive_per_install_dirs(&mut s, &dest_s, Game::BGEE).expect("derive BGEE");

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
    fn amendment_b_compose_importer_applier_under_space_destination() {
        let dest = td();
        let dest_str = format!("{} with space", dest.to_string_lossy());
        let dest_path = std::path::PathBuf::from(&dest_str);
        let mut s = Step1State::default();
        let dirs = derive_per_install_dirs(&mut s, &dest_str, Game::EET).expect("derive EET");

        for d in dirs.all_dirs() {
            assert!(
                d.starts_with(&dest_path),
                "AMENDMENT: {} must be inside the space-containing destination (no appdata exception)",
                d.display()
            );
        }
        assert_eq!(
            dirs.weidu_component_logs,
            dest_path.join(WEIDU_COMPONENT_LOGS_DIRNAME),
            "AMENDMENT: weidu_component_logs == <space dest>/weidu_component_logs"
        );

        for mode in [
            Step1State::INSTALL_MODE_BUILD_FROM_SCANNED_MODS,
            Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS,
            Step1State::INSTALL_MODE_WEIDU_LOGS_REVIEW_EDIT,
        ] {
            s.game_install = "EET".to_string();
            s.install_mode = mode.to_string();
            s.sync_install_mode_flags();
            let wb = importer_write_target(&s, true).expect("EET BGEE importer write target");
            let rb = crate::app::app_step2_log::resolve_bgee_weidu_log_path(&s)
                .expect("EET BGEE real applier read path");
            let w2 = importer_write_target(&s, false).expect("EET BG2EE importer write target");
            let r2 = crate::app::app_step2_log::resolve_bg2_weidu_log_path(&s)
                .expect("EET BG2EE real applier read path");
            assert_eq!(
                wb, rb,
                "{mode}: BGEE importer write target == real applier read path"
            );
            assert_eq!(
                w2, r2,
                "{mode}: BG2EE importer write target == real applier read path"
            );
            assert_ne!(
                wb, w2,
                "{mode}: the two EET phase logs write to distinct files"
            );
            assert!(
                wb.starts_with(&dest_path) && w2.starts_with(&dest_path),
                "{mode}: importer/applier targets are inside the SPACE destination"
            );
        }

        let ulog = dirs.weidu_component_logs.to_string_lossy().into_owned();
        assert_eq!(
            s.weidu_log_mode,
            format!("autolog,logapp,log-extern,log {}", ulog.trim()),
            "B+AMENDMENT compose: weidu_log_mode == base tokens + `log <weidu_component_logs>` \
             where that folder IS AMENDMENT's derived dir under the space dest (same path, not a conflict)"
        );
        assert_eq!(
            s.weidu_log_folder, ulog,
            "B's source-of-truth weidu_log_folder == AMENDMENT's weidu_component_logs under the space dest"
        );
        assert!(
            s.weidu_log_log_component,
            "#2 per-component logging forced ON"
        );

        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_dir_all(&dest_path);
    }

    #[test]
    fn derive_sets_have_weidu_logs_consistently_and_creates_the_source_dirs() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut s = Step1State::default();
        let dirs = derive_per_install_dirs(&mut s, &dest_s, Game::EET).expect("derive");
        assert!(
            !s.have_weidu_logs,
            "build_from_scanned_mods ⇒ have_weidu_logs false (mode-consistent)"
        );

        let mut s2 = Step1State::default();
        derive_per_install_dirs(&mut s2, &dest_s, Game::BGEE).expect("derive");
        s2.install_mode = Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS.to_string();
        s2.sync_install_mode_flags();
        assert!(
            s2.have_weidu_logs,
            "install_exactly_from_weidu_logs ⇒ have_weidu_logs true"
        );

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

        cleanup_per_install_mods_folder(&dest_s);

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn weidu_component_logs_resolves_inside_the_destination_not_appdata() {
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

        assert!(d.mods_folder.starts_with(dest_with_space));
        assert!(d.weidu_log_source_bgee.starts_with(dest_with_space));
        assert!(d.weidu_log_source_bg2ee.starts_with(dest_with_space));

        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut step1 = Step1State::default();
        let dirs = derive_per_install_dirs(&mut step1, &dest_s, Game::BGEE).expect("derive");
        assert!(
            step1.weidu_log_log_component,
            "#2 per-component logging stays forced ON"
        );
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

    fn mode_tokens(s: &str) -> Vec<String> {
        s.split(',').map(|t| t.trim().to_string()).collect()
    }

    #[test]
    fn derive_folds_an_additive_log_folder_token_into_weidu_log_mode() {
        let mut step1 = Step1State::default();
        assert_eq!(
            step1.weidu_log_mode, "autolog,logapp,log-extern",
            "precondition: default weidu_log_mode is the base tokens only \
             (no `log <folder>` ⇒ weidu_component_logs would stay empty — \
             the whole P7 #2 root cause)"
        );
        assert!(
            step1.weidu_log_autolog && step1.weidu_log_logapp && step1.weidu_log_logextern,
            "precondition: the three base booleans are true on this path \
             (so the rebuild is additive, not a clobber)"
        );

        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let dirs = derive_per_install_dirs(&mut step1, &dest_s, Game::BGEE).expect("derive");

        let folder = dirs.weidu_component_logs.to_string_lossy().into_owned();
        let tokens = mode_tokens(&step1.weidu_log_mode);

        for base in ["autolog", "logapp", "log-extern"] {
            assert!(
                tokens.iter().any(|t| t == base),
                "base token `{base}` MUST survive the fold (additive, not a \
                 clobber to just `log <folder>`); got {:?}",
                step1.weidu_log_mode
            );
        }

        let expected_log_token = format!("log {}", folder.trim());
        assert!(
            tokens.iter().any(|t| t == &expected_log_token),
            "the `log <folder>` token MUST be present (this is the #2 \
             mechanism `append_common_args` carries via --weidu-log-mode); \
             expected `{expected_log_token}` in {:?}",
            step1.weidu_log_mode
        );

        assert_eq!(
            step1.weidu_log_mode,
            format!("autolog,logapp,log-extern,log {}", folder.trim()),
            "the rebuilt weidu_log_mode is the base default PLUS the `log \
             <folder>` token — preserved, not replaced"
        );
        assert!(
            step1.weidu_log_log_component,
            "#2 per-component logging stays forced ON (the source-of-truth \
             flag sync_weidu_log_mode reads)"
        );
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn derive_weidu_log_mode_fold_is_idempotent() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut step1 = Step1State::default();
        derive_per_install_dirs(&mut step1, &dest_s, Game::EET).expect("derive #1");
        let after_first = step1.weidu_log_mode.clone();
        derive_per_install_dirs(&mut step1, &dest_s, Game::EET).expect("derive #2");
        assert_eq!(
            step1.weidu_log_mode, after_first,
            "idempotent: a second derive must not change weidu_log_mode \
             (no doubled `log` token)"
        );
        assert_eq!(
            step1
                .weidu_log_mode
                .matches("log ")
                .filter(|_| true)
                .count(),
            1,
            "exactly one `log <folder>` token after repeated derives"
        );
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn emitted_install_args_carry_additive_weidu_log_mode() {
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut step1 = Step1State::default();
        let dirs = derive_per_install_dirs(&mut step1, &dest_s, Game::BGEE).expect("derive");
        let folder = dirs.weidu_component_logs.to_string_lossy().into_owned();

        let config = crate::app::step5::command_config::build_install_command_config(&step1);
        let (_program, args) =
            crate::install::step5_command_install::build_install_invocation(&config);
        let mode = step1.weidu_log_mode.clone();

        let idx = args
            .iter()
            .position(|a| a == "--weidu-log-mode")
            .expect("--weidu-log-mode MUST be emitted (weidu_log_mode_enabled is default true)");
        let value = &args[idx + 1];
        assert_eq!(
            value, &mode,
            "the emitted --weidu-log-mode value is exactly step1.weidu_log_mode"
        );
        let tokens = mode_tokens(value);
        for base in ["autolog", "logapp", "log-extern"] {
            assert!(
                tokens.iter().any(|t| t == base),
                "emitted --weidu-log-mode MUST still carry base token \
                 `{base}` (additive); got `{value}`"
            );
        }
        assert!(
            tokens
                .iter()
                .any(|t| t == &format!("log {}", folder.trim())),
            "emitted --weidu-log-mode MUST carry the `log <weidu_component_\
             logs>` token; got `{value}`"
        );
        let _ = std::fs::remove_dir_all(&dest);
    }
}
