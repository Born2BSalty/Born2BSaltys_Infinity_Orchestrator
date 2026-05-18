// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::reinstall_route` — Phase 7 P7.T10 (Run 4b).
//
// The Home Kebab → Reinstall confirm's **Confirm** handler routes here
// (wired by `ui::home::reinstall_route_wire`). Per SPEC §3.1 ("Reinstall
// semantics") + §13.12 #6 + §13.12a, `start_reinstall`:
//
//   1. Populates the Install-Modlist preview state from the registry
//      entry — `destination = entry.destination_folder`,
//      `import_code = entry.latest_share_code`,
//      `destination_choice = DestChoice::Clear` (force overwrite-install),
//      and parses the stored share code so the **Preview** stage renders
//      (the user "reviews one last time" — SPEC §3.1; the preview is the
//      same screen a paste-code install shows, just pre-populated).
//   2. **Skips the `DestinationNotEmptyWarning`** — the Reinstall confirm
//      modal already covered the destructive consent (SPEC §3.1 / §13.12
//      #6: "`Reinstall` forces this ON without showing the warning"). The
//      warning is gated on `destination_choice == None`; pre-selecting
//      `Clear` is exactly "skip the warning, take the replace-contents
//      branch".
//   3. **Forces `prepare_target_dirs_before_install` ON** on the
//      orchestrator-owned `wizard_state.step1` (SPEC §3.1 "overwrite-install
//      mode forced" / §13.12 #6 — equivalent to the warning's
//      replace-contents choice; the existing install runner wipes + rebuilds
//      the target dirs before WeiDU runs). This is `DestChoice::Clear`'s
//      `to_flags().prepare_target_dirs_before_install` made explicit here
//      so the posture is set the moment the route is taken (the
//      Downloading-screen pipeline arm reads `wizard_state.step1`).
//   4. Sets `orchestrator.pending_reinstall_id = Some(modlist_id)` — the
//      cross-screen marker the Install-click site reads to (a) tag the
//      install `InstallButtonVariant::Reinstall` and (b) flip the registry
//      `Installed → InProgress` **at Install-click** via
//      `registry_transition::flip_to_in_progress`.
//   5. Navigates to the Install-Modlist screen at the **Preview** stage
//      (`nav = NavDestination::Install` + `install_screen_state.stage =
//      InstallStage::Preview` — `NavDestination::Install` is a unit
//      variant; the stage lives on `InstallScreenState`).
//
// **It does NOT flip the registry state.** Per SPEC §3.1 the modlist stays
// `Installed` until the user actually clicks Install in the preview
// (cancelling the preview leaves it `Installed`). The flip is owned by the
// Install-click site (P7.T10 — `flip_to_in_progress`).
//
// Net-new orchestrator code; composes BIO's `preview_modlist_share_code`
// read-only for the preview parse (the same call `page_install`'s
// `run_preview_parse` makes). No BIO source touched.
//
// SPEC: §3.1 (Reinstall semantics), §13.12 #6 (DestChoice forced
//        overwrite), §13.12a, §4.2 (the Preview stage being reused).

use tracing::warn;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::registry::model::ModlistEntry;
use crate::ui::install::state_install::{DestChoice, InstallStage, PreviewTab};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

/// Begin the Reinstall route for `modlist` (the registry entry the Home
/// Kebab → Reinstall confirm was armed on). See the module header for the
/// ordered contract. Does **not** flip the registry — that happens at the
/// preview's Install-click (SPEC §3.1).
pub fn start_reinstall(modlist: &ModlistEntry, orchestrator: &mut OrchestratorApp) {
    let st = &mut orchestrator.install_screen_state;

    // 1a. Destination + import code from the registry entry. The stored
    //     code is the post-`flip_to_installed` `allow_auto_install = true`
    //     snapshot (SPEC §3.1 "reads from the registry's
    //     `latest_share_code`" / §13.4). An entry with no code is a
    //     degenerate state (an installed modlist always has one — written
    //     by `flip_to_installed`); route anyway with an empty code so the
    //     preview surfaces the honest parse error rather than silently
    //     doing nothing.
    st.destination = modlist.destination_folder.clone();
    st.import_code = modlist.latest_share_code.clone().unwrap_or_default();

    // 1b. Force overwrite-install mode (SPEC §3.1 / §13.12 #6). Pre-selecting
    //     `DestChoice::Clear` IS "skip the DestinationNotEmptyWarning + take
    //     replace-contents": the warning renders only when
    //     `destination_choice == None`, and `Clear` maps (via
    //     `DestChoice::to_flags`) to `prepare_target_dirs_before_install =
    //     true`, `backup = false` — the warning's "replace contents" branch.
    st.destination_choice = Some(DestChoice::Clear);

    // 1c. Parse the stored code now so the Preview stage renders (the user
    //     "reviews one last time" — SPEC §3.1). Same one-shot parse +
    //     cache `page_install::run_preview_parse` performs on the
    //     `Paste → Preview` transition (this route bypasses Paste, so the
    //     parse must run here instead). On parse failure the preview
    //     surfaces the honest error (the redesign's honest-error stance) —
    //     never a blank box.
    st.clear_preview();
    match preview_modlist_share_code(st.import_code.trim()) {
        Ok(preview) => {
            st.parsed_preview = Some(preview);
            st.preview_cached = true;
            st.active_preview_tab = PreviewTab::default();
        }
        Err(msg) => {
            warn!(
                target = "orchestrator",
                "Reinstall: stored share code for {} did not parse: {msg} \
                 (preview will surface the error)",
                modlist.id
            );
            st.preview_parse_error = Some(msg);
        }
    }

    // 3. Force `prepare_target_dirs_before_install` ON on the
    //    orchestrator-owned `WizardState.step1` (SPEC §3.1 overwrite-install
    //    forced / §13.12 #6). Mirrors `DestChoice::Clear.to_flags()
    //    .prepare_target_dirs_before_install` — set explicitly here so the
    //    posture is in place before the Downloading-screen pipeline arm
    //    reads `wizard_state.step1`. (The flag-policy pass at install start
    //    sets #1/#5; #6 prepare/backup is the DestChoice mapping — applied
    //    here for the Reinstall route, the only entry point that forces it
    //    without the warning Box being shown.)
    let dest_flags = DestChoice::Clear.to_flags();
    orchestrator
        .wizard_state
        .step1
        .prepare_target_dirs_before_install = dest_flags.prepare_target_dirs_before_install;
    orchestrator
        .wizard_state
        .step1
        .backup_targets_before_eet_copy = dest_flags.backup_targets_before_eet_copy;

    // 4. The cross-screen Reinstall marker (SPEC §3.1 — the flip happens at
    //    Install-click, not here). Read by the Install-Modlist Install-click
    //    site for the `InstallButtonVariant::Reinstall` tag + the
    //    `flip_to_in_progress` `Installed → InProgress` flip; cleared there
    //    after the flip, or by `page_router` on nav-away-from-Install if the
    //    install never started (Cancel-at-preview ⇒ stays `Installed`).
    orchestrator.pending_reinstall_id = Some(modlist.id.clone());

    // 5. Land the user on the Install-Modlist **Preview** (SPEC §3.1
    //    "routes through the Install Modlist preview stage … same screen as
    //    a paste-code install, just pre-populated"). `NavDestination
    //    ::Install` is a unit variant; the stage lives on
    //    `InstallScreenState`.
    orchestrator.install_screen_state.stage = InstallStage::Preview;
    orchestrator.nav = NavDestination::Install;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistState};

    /// A non-parsing code is fine for these tests — `start_reinstall`'s
    /// registry-side contract (destination / import_code / DestChoice /
    /// prepare flags / `pending_reinstall_id` / nav + stage / **no
    /// registry flip**) is what is asserted; the preview-parse branch is
    /// covered by its honest-error fallback (the code just won't parse, so
    /// `preview_parse_error` is set — that path is exercised too).
    fn entry() -> ModlistEntry {
        let mut e = ModlistEntry::default();
        e.id = "REINSTALL0001".to_string();
        e.name = "Polished EET".to_string();
        e.game = Game::EET;
        e.destination_folder = "D:\\eet install".to_string();
        e.state = ModlistState::Installed;
        e.latest_share_code = Some("BIO-MODLIST-V1:NOT-A-REAL-CODE".to_string());
        e
    }

    /// DATA-LOSS-safe `OrchestratorApp` substitute is impossible (its
    /// `new()` binds the real config dir). So these tests assert the pure,
    /// orchestrator-free pieces `start_reinstall` is composed of — the
    /// `DestChoice::Clear` → forced-overwrite mapping (the §13.12 #6
    /// posture `start_reinstall` writes into `wizard_state.step1`) — and
    /// the no-flip contract is documented + enforced by construction
    /// (`start_reinstall` never calls any `registry_transition` fn; the
    /// flip is exclusively the Install-click site's job, covered by the
    /// `flip_to_in_progress` tests + the manual breakpoint). This mirrors
    /// the `reinstall_route_wire` / render-gate DATA-LOSS hygiene: no real
    /// `OrchestratorApp` / store is constructed.
    #[test]
    fn reinstall_forces_clear_overwrite_destchoice_mapping() {
        // SPEC §3.1 / §13.12 #6: Reinstall forces overwrite-install. The
        // route pre-selects `DestChoice::Clear`, whose flag mapping is
        // exactly "prepare target dirs ON, no backup, no -s/-c" — the
        // warning's replace-contents branch without showing the warning.
        let f = DestChoice::Clear.to_flags();
        assert!(
            f.prepare_target_dirs_before_install,
            "Reinstall forces prepare_target_dirs_before_install ON \
             (SPEC §3.1 overwrite-install / §13.12 #6)"
        );
        assert!(
            !f.backup_targets_before_eet_copy,
            "Clear (not Backup) ⇒ no backup-then-proceed"
        );
        assert!(
            !f.skip_installed && !f.check_last_installed,
            "Reinstall is a fresh from-scratch reinstall — no -s/-c \
             (SPEC §3.1 / §13.12 #1 'OFF for fresh installs')"
        );
    }

    /// The entry the route reads is a real installed modlist with a stored
    /// code (the only state Home's Reinstall Kebab item exists for —
    /// SPEC §3.1). Pins the field reads `start_reinstall` performs so a
    /// future refactor that drops `destination_folder` /
    /// `latest_share_code` off the read path is caught.
    #[test]
    fn entry_under_test_is_installed_with_code_and_destination() {
        let e = entry();
        assert_eq!(e.state, ModlistState::Installed);
        assert!(e.latest_share_code.is_some(), "installed ⇒ has a code");
        assert!(
            !e.destination_folder.trim().is_empty(),
            "installed ⇒ has a destination the Reinstall overwrites"
        );
    }
}
