// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step1State;
use crate::settings::model::Step1Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallWorkflow {
    FreshCreate,

    PasteAndInstall,

    ForkDownload,

    ForkInstall,

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
    match workflow {
        InstallWorkflow::FreshCreate => ResolvedFlags {
            skip_installed: false,
            check_last_installed: false,
            download: settings.download,
        },

        InstallWorkflow::PasteAndInstall
        | InstallWorkflow::ForkDownload
        | InstallWorkflow::ForkInstall
        | InstallWorkflow::Reinstall => ResolvedFlags {
            skip_installed: false,
            check_last_installed: false,
            download: true,
        },

        InstallWorkflow::ContinuePartialInstall => ResolvedFlags {
            skip_installed: true,
            check_last_installed: true,
            download: true,
        },
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

    fn settings_with_download(download: bool) -> Step1Settings {
        Step1Settings {
            download,
            ..Default::default()
        }
    }

    #[test]
    fn fresh_create_no_sc_download_follows_settings() {
        let on = compute_flags(InstallWorkflow::FreshCreate, &settings_with_download(true));
        assert_eq!(on, vec!["--download"], "Settings download ON ⇒ --download");
        let off = compute_flags(InstallWorkflow::FreshCreate, &settings_with_download(false));
        assert!(
            off.is_empty(),
            "Settings download OFF ⇒ no flags (fresh-create, no -s/-c)"
        );
    }

    #[test]
    fn paste_and_fork_force_download_no_sc() {
        for workflow in [
            InstallWorkflow::PasteAndInstall,
            InstallWorkflow::ForkDownload,
            InstallWorkflow::ForkInstall,
        ] {
            for settings_dl in [true, false] {
                let on = compute_flags(workflow, &settings_with_download(settings_dl));
                assert_eq!(
                    on,
                    vec!["--download"],
                    "share-code workflow {workflow:?} forces --download (Settings={settings_dl})"
                );
            }
        }
    }

    #[test]
    fn continue_partial_forces_s_c_and_download() {
        let on = compute_flags(
            InstallWorkflow::ContinuePartialInstall,
            &settings_with_download(false),
        );
        assert_eq!(
            on,
            vec!["-s", "-c", "--download"],
            "Continue Partial Install ⇒ -s + -c + --download"
        );
    }

    #[test]
    fn reinstall_forces_download_no_sc() {
        let on = compute_flags(InstallWorkflow::Reinstall, &settings_with_download(false));
        assert_eq!(on, vec!["--download"]);
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
            &settings_with_download(true),
        );
        assert!(!step1.skip_installed, "fresh-create clears -s");
        assert!(!step1.check_last_installed, "fresh-create clears -c");
        assert!(step1.download, "fresh-create download follows Settings ON");

        apply_flags(
            &mut step1,
            InstallWorkflow::ContinuePartialInstall,
            &settings_with_download(false),
        );
        assert!(step1.skip_installed, "Continue Partial sets -s");
        assert!(step1.check_last_installed, "Continue Partial sets -c");
        assert!(step1.download, "Continue Partial forces --download");
    }
}
