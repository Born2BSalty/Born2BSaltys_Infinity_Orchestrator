// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Strips per-install fields from a `Step1State` clone before it is
//! converted to a `Step1Settings` snapshot and persisted to
//! `bio_settings.json`.
//!
//! Global path settings (Mods archive, Mods backup, Game sources) are
//! user-owned and live in `bio_settings.json`. Per-install derived
//! dirs (Mods folder, `weidu_component_logs`, WeiDU-log source folders,
//! game-clone dirs) are computed at install-arm from the destination,
//! written onto `wizard_state.step1` so the BIO install pipeline reads
//! them, and must NOT round-trip through the global settings file.
//! Without this sanitizer, the orchestrator's debounced settings tick
//! clones `step1` and converts it to `Step1Settings` whole, leaking
//! per-install state into the persistence and dragging the per-install
//! paths into Settings → Paths for the next session.
//!
//! The live `wizard_state.step1` is NOT touched — the in-flight
//! install still resolves the per-install dirs correctly. Only the
//! persistence path sees the sanitized clone.

use crate::app::state::Step1State;
use crate::settings::model::Step1Settings;

/// Overwrites every per-install path / flag on `step1` with the
/// matching value from `settings` (the persisted Settings → Paths
/// snapshot).
///
/// After this call, the clone can be safely converted to a
/// `Step1Settings` for serialization without leaking per-install dirs
/// such as `<dest>/mods` into the global settings file. The global
/// fields BIO settings persist correctly — Mods archive, Mods backup,
/// Game sources, the user-toggleable Advanced flags — are not touched
/// by this function; they are read from `step1` as the user edits them.
pub fn sanitize_step1_for_settings_persistence(step1: &mut Step1State, settings: &Step1Settings) {
    // Per-install derived path strings — every field
    // `derive_per_install_dirs` writes onto `step1`.
    step1.mods_folder.clone_from(&settings.mods_folder);
    step1.weidu_log_folder.clone_from(&settings.weidu_log_folder);
    step1.bgee_log_folder.clone_from(&settings.bgee_log_folder);
    step1.bg2ee_log_folder.clone_from(&settings.bg2ee_log_folder);
    step1
        .eet_bgee_log_folder
        .clone_from(&settings.eet_bgee_log_folder);
    step1
        .eet_bg2ee_log_folder
        .clone_from(&settings.eet_bg2ee_log_folder);
    step1.bgee_log_file.clone_from(&settings.bgee_log_file);
    step1.bg2ee_log_file.clone_from(&settings.bg2ee_log_file);
    step1.eet_pre_dir.clone_from(&settings.eet_pre_dir);
    step1.eet_new_dir.clone_from(&settings.eet_new_dir);
    step1
        .generate_directory
        .clone_from(&settings.generate_directory);

    // The `weidu_log_mode` string is rebuilt by `sync_weidu_log_mode`
    // and embeds the per-install `log <folder>` token — restore it to
    // the persisted value so the token does not leak.
    step1.weidu_log_mode.clone_from(&settings.weidu_log_mode);

    // Per-install booleans that `derive_per_install_dirs` toggles.
    step1.weidu_log_log_component = settings.weidu_log_log_component;
    step1.have_weidu_logs = settings.have_weidu_logs;
    step1.new_pre_eet_dir_enabled = settings.new_pre_eet_dir_enabled;
    step1.new_eet_dir_enabled = settings.new_eet_dir_enabled;
    step1.generate_directory_enabled = settings.generate_directory_enabled;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn global_settings() -> Step1Settings {
        Step1Settings {
            // Global path values the user maintains in Settings → Paths.
            mods_archive_folder: r"C:\Games\BIO\archive".to_string(),
            mods_backup_folder: r"C:\Games\BIO\backup".to_string(),
            bgee_game_folder: r"C:\Games\src\BGEE".to_string(),
            bg2ee_game_folder: r"C:\Games\src\BG2EE".to_string(),
            iwdee_game_folder: r"C:\Games\src\IWDEE".to_string(),
            // Global per-install field defaults — empty in a clean
            // Settings → Paths state (the user never sets these by
            // hand; `derive_per_install_dirs` populates them at
            // install-arm).
            mods_folder: String::new(),
            weidu_log_folder: String::new(),
            bgee_log_folder: String::new(),
            bg2ee_log_folder: String::new(),
            eet_bgee_log_folder: String::new(),
            eet_bg2ee_log_folder: String::new(),
            bgee_log_file: String::new(),
            bg2ee_log_file: String::new(),
            eet_pre_dir: String::new(),
            eet_new_dir: String::new(),
            generate_directory: String::new(),
            weidu_log_mode: "autolog,logapp,log-extern".to_string(),
            weidu_log_log_component: false,
            have_weidu_logs: false,
            new_pre_eet_dir_enabled: false,
            new_eet_dir_enabled: false,
            generate_directory_enabled: false,
            ..Step1Settings::default()
        }
    }

    fn polluted_step1() -> Step1State {
        Step1State {
            // Per-install values the install runtime wrote.
            mods_folder: r"C:\Games\BIO\installations\simpletest fork\mods".to_string(),
            weidu_log_folder: r"C:\Games\BIO\installations\simpletest fork\weidu_component_logs"
                .to_string(),
            bgee_log_folder: r"C:\Games\BIO\installations\simpletest fork\weidu_log_source\bgee"
                .to_string(),
            bg2ee_log_folder: r"C:\Games\BIO\installations\simpletest fork\weidu_log_source\bg2ee"
                .to_string(),
            eet_bgee_log_folder: r"C:\Games\BIO\installations\simpletest fork\weidu_log_source\bgee"
                .to_string(),
            eet_bg2ee_log_folder:
                r"C:\Games\BIO\installations\simpletest fork\weidu_log_source\bg2ee".to_string(),
            bgee_log_file:
                r"C:\Games\BIO\installations\simpletest fork\weidu_log_source\bgee\weidu.log"
                    .to_string(),
            bg2ee_log_file:
                r"C:\Games\BIO\installations\simpletest fork\weidu_log_source\bg2ee\weidu.log"
                    .to_string(),
            eet_pre_dir:
                r"C:\Games\BIO\installations\simpletest fork\Baldur's Gate Enhanced Edition"
                    .to_string(),
            eet_new_dir:
                r"C:\Games\BIO\installations\simpletest fork\Baldur's Gate II Enhanced Edition"
                    .to_string(),
            generate_directory:
                r"C:\Games\BIO\installations\simpletest fork\Baldur's Gate Enhanced Edition"
                    .to_string(),
            weidu_log_mode: "autolog,logapp,log-extern,log C:\\Games\\BIO\\installations\\simpletest fork\\weidu_component_logs"
                .to_string(),
            weidu_log_log_component: true,
            have_weidu_logs: true,
            new_pre_eet_dir_enabled: true,
            new_eet_dir_enabled: true,
            generate_directory_enabled: false,
            // Global values that must NOT be touched by the sanitizer.
            mods_archive_folder: r"C:\Games\BIO\archive".to_string(),
            mods_backup_folder: r"C:\Games\BIO\backup".to_string(),
            bgee_game_folder: r"C:\Games\src\BGEE".to_string(),
            bg2ee_game_folder: r"C:\Games\src\BG2EE".to_string(),
            iwdee_game_folder: r"C:\Games\src\IWDEE".to_string(),
            ..Step1State::default()
        }
    }

    #[test]
    fn sanitize_resets_per_install_fields_to_global_values_and_keeps_globals_intact() {
        let settings = global_settings();
        let mut step1 = polluted_step1();

        sanitize_step1_for_settings_persistence(&mut step1, &settings);

        // Per-install string fields reset to the global (empty) values.
        assert_eq!(step1.mods_folder, "", "mods_folder reset");
        assert_eq!(step1.weidu_log_folder, "", "weidu_log_folder reset");
        assert_eq!(step1.bgee_log_folder, "", "bgee_log_folder reset");
        assert_eq!(step1.bg2ee_log_folder, "", "bg2ee_log_folder reset");
        assert_eq!(step1.eet_bgee_log_folder, "", "eet_bgee_log_folder reset");
        assert_eq!(step1.eet_bg2ee_log_folder, "", "eet_bg2ee_log_folder reset");
        assert_eq!(step1.bgee_log_file, "", "bgee_log_file reset");
        assert_eq!(step1.bg2ee_log_file, "", "bg2ee_log_file reset");
        assert_eq!(step1.eet_pre_dir, "", "eet_pre_dir reset");
        assert_eq!(step1.eet_new_dir, "", "eet_new_dir reset");
        assert_eq!(step1.generate_directory, "", "generate_directory reset");
        assert_eq!(
            step1.weidu_log_mode, "autolog,logapp,log-extern",
            "weidu_log_mode reset to base tokens (the per-install `log \
             <folder>` token is stripped)"
        );
        assert!(!step1.weidu_log_log_component, "boolean reset");
        assert!(!step1.have_weidu_logs, "boolean reset");
        assert!(!step1.new_pre_eet_dir_enabled, "boolean reset");
        assert!(!step1.new_eet_dir_enabled, "boolean reset");
        assert!(!step1.generate_directory_enabled, "boolean reset");

        // Global fields the user maintains in Settings → Paths are NOT
        // touched — these are the values the orchestrator must let
        // settings.json round-trip.
        assert_eq!(
            step1.mods_archive_folder, r"C:\Games\BIO\archive",
            "Mods archive must NOT be touched (global, user-owned)"
        );
        assert_eq!(
            step1.mods_backup_folder, r"C:\Games\BIO\backup",
            "Mods backup must NOT be touched (global, user-owned)"
        );
        assert_eq!(
            step1.bgee_game_folder, r"C:\Games\src\BGEE",
            "BGEE source must NOT be touched (global, user-owned)"
        );
        assert_eq!(
            step1.bg2ee_game_folder, r"C:\Games\src\BG2EE",
            "BG2EE source must NOT be touched (global, user-owned)"
        );
        assert_eq!(
            step1.iwdee_game_folder, r"C:\Games\src\IWDEE",
            "IWDEE source must NOT be touched (global, user-owned)"
        );
    }
}
