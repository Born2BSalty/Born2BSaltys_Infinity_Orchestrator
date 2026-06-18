// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use tracing::warn;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::registry::model::ModlistEntry;
use crate::ui::install::state_install::{DestChoice, InstallStage, PreviewTab};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

pub fn start_reinstall(modlist: &ModlistEntry, orchestrator: &mut OrchestratorApp) {
    let st = &mut orchestrator.install_screen_state;

    st.destination.clone_from(&modlist.destination_folder);
    st.import_code = modlist.latest_share_code.clone().unwrap_or_default();

    st.destination_choice = Some(DestChoice::Clear);

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

    let dest_flags = DestChoice::Clear.to_flags();
    orchestrator
        .wizard_state
        .step1
        .prepare_target_dirs_before_install = dest_flags.prepare_target_dirs_before_install;
    orchestrator
        .wizard_state
        .step1
        .backup_targets_before_eet_copy = dest_flags.backup_targets_before_eet_copy;

    orchestrator.pending_reinstall_id = Some(modlist.id.clone());

    orchestrator.install_screen_state.stage = InstallStage::Preview;
    orchestrator.nav = NavDestination::Install;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistState};

    fn entry() -> ModlistEntry {
        ModlistEntry {
            id: "REINSTALL0001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            destination_folder: "D:\\eet install".to_string(),
            state: ModlistState::Installed,
            latest_share_code: Some("BIO-MODLIST-V1:NOT-A-REAL-CODE".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn reinstall_forces_clear_overwrite_destchoice_mapping() {
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
