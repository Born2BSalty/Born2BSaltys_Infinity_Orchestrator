// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::flag_policies` — SPEC §13.12 automatic flag policies,
// **#1 and #5 only** (P7.T16 scope).
//
//   #1 `-s` (skip installed) + `-c` (check last installed) — automatically
//      ON for the **Continue Partial Install** workflow (the "this folder
//      isn't empty" Box's `continue` choice); OFF for fresh installs.
//   #5 `--download` (download missing mods) — automatically ON for
//      share-code-consuming workflows (Install Modlist paste, Create →
//      Import-and-modify, Load Draft). For **Create → New** (fresh-create)
//      it falls back to the user's Settings → Advanced "Download missing
//      mods and keep archives" toggle (default ON).
//
// Other policies are explicitly out of P7.T16 scope: #2 `-u` / #3 `-p`/`-n`
// / #4 `-g` are install-critical per-install dirs owned by P7.T17 (Run 4);
// #6 (`DestChoice` mapping — already the pure `DestChoice::to_flags`) + #7
// (hardcoded `-autolog`/`-logapp`/`-log-extern`) are Phase 8.
//
// **No BIO source.** `compute_flags` is a pure decision function;
// `apply_flags` writes the decision into the orchestrator-owned
// `WizardState.step1` `pub` fields BIO's existing install runner already
// reads (`skip_installed` → `-s`, `check_last_installed` → `-c`, `download`
// → `--download`). BIO's command builder is unchanged.
//
// SPEC: §13.12 #1, #5.

use crate::app::state::Step1State;
use crate::settings::model::Step1Settings;

/// The install workflow this install was started from — the context SPEC
/// §13.12 #1/#5 key off. Net-new orchestrator enum (the redesign's
/// workflow taxonomy; BIO has no equivalent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallWorkflow {
    /// Create → New modlist from downloaded mods (fresh-create). No
    /// `-s`/`-c`; `--download` follows the Settings → Advanced toggle
    /// (SPEC §13.12 #5 fresh-create fallback).
    FreshCreate,
    /// A share-code-consuming workflow: **Install Modlist** (paste import
    /// code), **Create → Import-and-modify**, or **Load Draft**. No
    /// `-s`/`-c` (it is a fresh attempt — overwrite); `--download`
    /// **forced ON** (SPEC §13.12 #5).
    ShareCodeConsuming,
    /// Continue Partial Install (the destination-not-empty Box's
    /// `continue` choice). `-s` **and** `-c` forced ON (SPEC §13.12 #1);
    /// `--download` forced ON (it resumes a share-code-derived install —
    /// SPEC §13.12 #5 lists Install-Modlist/import/draft, and a partial
    /// continuation is of such an install; keeping it ON guarantees a
    /// resumed import still fetches anything not yet downloaded).
    ContinuePartialInstall,
    /// Reinstall (Home Kebab → Reinstall on an installed card). No
    /// `-s`/`-c` (it is an overwrite reinstall — SPEC §13.12 #1 "OFF for
    /// fresh installs", and Reinstall is a full from-scratch reinstall per
    /// SPEC §3.1); `--download` forced ON (it reads the registry's
    /// `latest_share_code` — a share-code-consuming path, SPEC §13.12 #5).
    Reinstall,
}

/// The flags `compute_flags` resolves for a workflow (the policy decision,
/// separate from the BIO state it is written into — testable in isolation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedFlags {
    /// `-s` (skip installed) — SPEC §13.12 #1.
    pub skip_installed: bool,
    /// `-c` (check last installed) — SPEC §13.12 #1.
    pub check_last_installed: bool,
    /// `--download` (download missing mods) — SPEC §13.12 #5.
    pub download: bool,
}

/// SPEC §13.12 #1 + #5 — resolve the automatic flags for `workflow`.
/// `settings` supplies the fresh-create `--download` fallback (Settings →
/// Advanced "Download missing mods and keep archives", default ON — SPEC
/// §13.12 #5).
///
/// Returns the CLI-flag tokens that this policy turns **ON** (the plan's
/// `compute_flags(...) -> Vec<&'static str>` shape — a stable list of the
/// flags this policy asserts). The structured [`ResolvedFlags`] (used by
/// [`apply_flags`] to write the `WizardState.step1` booleans) is available
/// via [`resolve`]; this `Vec` form is the human-auditable summary the
/// dev-mode diagnostics record / the Command-card inspection check against.
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

/// The structured form of [`compute_flags`] (SPEC §13.12 #1 + #5).
#[must_use]
pub fn resolve(workflow: InstallWorkflow, settings: &Step1Settings) -> ResolvedFlags {
    match workflow {
        // #1: `-s`/`-c` OFF for fresh installs. #5: `--download` follows
        // the Settings → Advanced toggle for fresh-create.
        InstallWorkflow::FreshCreate => ResolvedFlags {
            skip_installed: false,
            check_last_installed: false,
            download: settings.download,
        },
        // #1: OFF (fresh attempt). #5: `--download` forced ON.
        InstallWorkflow::ShareCodeConsuming | InstallWorkflow::Reinstall => ResolvedFlags {
            skip_installed: false,
            check_last_installed: false,
            download: true,
        },
        // #1: `-s` AND `-c` forced ON. #5: `--download` forced ON.
        InstallWorkflow::ContinuePartialInstall => ResolvedFlags {
            skip_installed: true,
            check_last_installed: true,
            download: true,
        },
    }
}

/// Write the resolved #1/#5 flags into the orchestrator-owned
/// `WizardState.step1` — the exact `pub` fields BIO's existing install
/// runner reads when it builds the WeiDU command line (`skip_installed`
/// → `-s`, `check_last_installed` → `-c`, `download` → `--download`). No
/// BIO source is touched; the BIO command builder consumes these unchanged.
/// Called by `start_hooks::on_install_start` before flipping
/// `start_install_requested = true`.
pub fn apply_flags(step1: &mut Step1State, workflow: InstallWorkflow, settings: &Step1Settings) {
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
        // SPEC §13.12 #1: no -s/-c for fresh installs.
        // SPEC §13.12 #5: fresh-create `--download` follows Settings.
        let on = compute_flags(InstallWorkflow::FreshCreate, &settings_with_download(true));
        assert_eq!(on, vec!["--download"], "Settings download ON ⇒ --download");
        let off = compute_flags(InstallWorkflow::FreshCreate, &settings_with_download(false));
        assert!(
            off.is_empty(),
            "Settings download OFF ⇒ no flags (fresh-create, no -s/-c)"
        );
    }

    #[test]
    fn share_code_consuming_forces_download_no_sc() {
        // SPEC §13.12 #5: Install Modlist paste / import / Load Draft ⇒
        // --download forced ON regardless of Settings; #1: no -s/-c.
        for settings_dl in [true, false] {
            let on = compute_flags(
                InstallWorkflow::ShareCodeConsuming,
                &settings_with_download(settings_dl),
            );
            assert_eq!(
                on,
                vec!["--download"],
                "share-code workflow forces --download (Settings={settings_dl})"
            );
        }
    }

    #[test]
    fn continue_partial_forces_s_c_and_download() {
        // SPEC §13.12 #1: Continue Partial Install ⇒ -s AND -c ON.
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
        // SPEC §13.12 #1: Reinstall is overwrite ⇒ no -s/-c.
        // SPEC §13.12 #5: reads latest_share_code ⇒ --download ON.
        let on = compute_flags(InstallWorkflow::Reinstall, &settings_with_download(false));
        assert_eq!(on, vec!["--download"]);
    }

    #[test]
    fn apply_flags_writes_step1_booleans() {
        // The decision must land in the exact BIO `Step1State` fields the
        // install runner reads (-s/-c/--download), and ONLY those.
        let mut step1 = Step1State {
            skip_installed: true, // stale from a prior workflow
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
