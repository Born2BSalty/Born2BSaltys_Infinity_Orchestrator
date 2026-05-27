// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step1State;
use crate::settings::model::Step1Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallWorkflow {
    FreshCreate,

    PasteAndInstall,

    ForkAndModify,

    ContinuePartialInstall,

    Reinstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedFlags {
    pub skip_installed: bool,

    pub check_last_installed: bool,

    pub download: bool,
}

#[must_use]
pub fn compute_flags(workflow: InstallWorkflow, settings: &Step1Settings) -> Vec<&'static str> {
    let f = resolve(workflow, settings);
    let mut out = Vec::new();
    if f.skip_installed {
        out.push("-s");
    }
    if f.check_last_installed {
        out.push("-c");
    }
    if f.download {
        out.push("--download");
    }
    out
}

#[must_use]
pub const fn resolve(workflow: InstallWorkflow, settings: &Step1Settings) -> ResolvedFlags {
    let download = match workflow {
        InstallWorkflow::FreshCreate => settings.download,
        InstallWorkflow::PasteAndInstall
        | InstallWorkflow::ForkAndModify
        | InstallWorkflow::ContinuePartialInstall
        | InstallWorkflow::Reinstall => true,
    };
    ResolvedFlags {
        skip_installed: settings.skip_installed,
        check_last_installed: settings.check_last_installed,
        download,
    }
}

pub const fn apply_flags(
    step1: &mut Step1State,
    workflow: InstallWorkflow,
    settings: &Step1Settings,
) {
    let f = resolve(workflow, settings);
    step1.skip_installed = f.skip_installed;
    step1.check_last_installed = f.check_last_installed;
    step1.download = f.download;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with_flags(
        skip_installed: bool,
        check_last_installed: bool,
        download: bool,
    ) -> Step1Settings {
        Step1Settings {
            skip_installed,
            download,
            check_last_installed,
            ..Default::default()
        }
    }

    #[test]
    fn fresh_create_s_c_and_download_follow_settings() {
        let on = compute_flags(
            InstallWorkflow::FreshCreate,
            &settings_with_flags(true, true, true),
        );
        assert_eq!(
            on,
            vec!["-s", "-c", "--download"],
            "Settings ON ⇒ -s, -c, --download"
        );
        let off = compute_flags(
            InstallWorkflow::FreshCreate,
            &settings_with_flags(false, false, false),
        );
        assert!(off.is_empty(), "Settings OFF ⇒ no flags for fresh-create");
    }

    #[test]
    fn paste_and_fork_force_download_but_s_c_follow_settings() {
        for workflow in [
            InstallWorkflow::PasteAndInstall,
            InstallWorkflow::ForkAndModify,
        ] {
            let no_sc = compute_flags(workflow, &settings_with_flags(false, false, false));
            assert_eq!(
                no_sc,
                vec!["--download"],
                "share-code workflow {workflow:?} forces only --download when -s/-c are OFF"
            );
            let with_sc = compute_flags(workflow, &settings_with_flags(true, true, false));
            assert_eq!(
                with_sc,
                vec!["-s", "-c", "--download"],
                "share-code workflow {workflow:?} keeps user -s/-c while forcing --download"
            );
        }
    }

    #[test]
    fn continue_partial_forces_download_but_s_c_follow_settings() {
        let off = compute_flags(
            InstallWorkflow::ContinuePartialInstall,
            &settings_with_flags(false, false, false),
        );
        assert_eq!(
            off,
            vec!["--download"],
            "Continue Partial Install ⇒ --download, with -s/-c following Settings OFF"
        );
        let on = compute_flags(
            InstallWorkflow::ContinuePartialInstall,
            &settings_with_flags(true, true, false),
        );
        assert_eq!(
            on,
            vec!["-s", "-c", "--download"],
            "Continue Partial Install ⇒ --download, with -s/-c following Settings ON"
        );
    }

    #[test]
    fn reinstall_forces_download_but_s_c_follow_settings() {
        let off = compute_flags(
            InstallWorkflow::Reinstall,
            &settings_with_flags(false, false, false),
        );
        assert_eq!(off, vec!["--download"]);
        let on = compute_flags(
            InstallWorkflow::Reinstall,
            &settings_with_flags(true, true, false),
        );
        assert_eq!(on, vec!["-s", "-c", "--download"]);
    }

    #[test]
    fn apply_flags_writes_step1_booleans() {
        let mut step1 = Step1State {
            skip_installed: true,
            check_last_installed: true,
            download: false,
            ..Default::default()
        };
        apply_flags(
            &mut step1,
            InstallWorkflow::FreshCreate,
            &settings_with_flags(false, false, true),
        );
        assert!(
            !step1.skip_installed,
            "fresh-create follows Settings -s OFF"
        );
        assert!(
            !step1.check_last_installed,
            "fresh-create follows Settings -c OFF"
        );
        assert!(step1.download, "fresh-create download follows Settings ON");

        apply_flags(
            &mut step1,
            InstallWorkflow::ContinuePartialInstall,
            &settings_with_flags(true, false, false),
        );
        assert!(
            step1.skip_installed,
            "Continue Partial follows Settings -s ON"
        );
        assert!(
            !step1.check_last_installed,
            "Continue Partial follows Settings -c OFF"
        );
        assert!(step1.download, "Continue Partial forces --download");
    }
}
