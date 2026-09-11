// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use tracing::warn;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::registry::model::ModlistEntry;
use crate::ui::install::state_install::{DestChoice, InstallStage, PipelineKind, ReviewOrigin};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

pub fn start_reinstall(modlist: &ModlistEntry, orchestrator: &mut OrchestratorApp) {
    let code = modlist.latest_share_code.clone().unwrap_or_default();

    let preview = match preview_modlist_share_code(code.trim()) {
        Ok(preview) => preview,
        Err(msg) => {
            warn!(
                target = "orchestrator",
                "Reinstall: stored share code for {} did not parse: {msg}", modlist.id
            );
            orchestrator.notification_manager.error(format!(
                "Could not read the stored share code for \"{}\": {msg}",
                modlist.name
            ));
            return;
        }
    };

    let st = &mut orchestrator.install_screen_state;

    st.destination.clone_from(&modlist.destination_folder);
    st.import_code = code;

    st.destination_choice = Some(DestChoice::Clear);
    st.pipeline_kind = PipelineKind::Install;
    st.review.origin = ReviewOrigin::Reinstall;
    st.review.name.clone_from(&modlist.name);
    st.review.modify = false;

    st.clear_preview();
    st.parsed_preview = Some(preview);
    st.preview_cached = true;

    let dest_flags = DestChoice::Clear.to_flags();
    orchestrator
        .wizard_state
        .step1
        .prepare_target_dirs_before_install = dest_flags.prepare_target_dirs_before_install;
    orchestrator
        .wizard_state
        .step1
        .backup_targets_before_eet_copy = dest_flags.backup_targets_before_eet_copy;

    crate::ui::install::page_install::refresh_source_compat_issue(
        &mut orchestrator.install_screen_state,
        &orchestrator.wizard_state.step1,
    );

    orchestrator.pending_reinstall_id = Some(modlist.id.clone());

    orchestrator.install_screen_state.stage = InstallStage::Details;
    orchestrator.nav = NavDestination::Install;
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistState};

    fn orch_for_reinstall_test() -> OrchestratorApp {
        OrchestratorApp::new_isolated_for_test("reinstalltest")
    }

    fn entry() -> ModlistEntry {
        let stub = crate::ui::install::gallery::catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = crate::ui::install::gallery::catalog::share_code(stub).expect("stub code");
        ModlistEntry {
            id: "REINSTALL0001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            destination_folder: "D:\\eet install".to_string(),
            state: ModlistState::Installed,
            latest_share_code: Some(code),
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
    fn reinstall_lands_on_details_with_the_entry_name_folder_and_clear_forced() {
        let modlist = entry();
        let mut app = orch_for_reinstall_test();

        start_reinstall(&modlist, &mut app);

        let st = &app.install_screen_state;
        assert_eq!(st.stage, InstallStage::Details);
        assert_eq!(st.review.origin, ReviewOrigin::Reinstall);
        assert_eq!(st.review.name, "Polished EET");
        assert!(!st.review.modify);
        assert_eq!(st.destination, "D:\\eet install");
        assert_eq!(st.destination_choice, Some(DestChoice::Clear));
        assert_eq!(st.pipeline_kind, PipelineKind::Install);
        assert_eq!(app.nav, NavDestination::Install);
        assert_eq!(app.pending_reinstall_id.as_deref(), Some("REINSTALL0001"));
    }

    #[test]
    fn reinstall_stores_the_dlc_issue_when_the_source_holds_the_archive() {
        use crate::ui::install::gallery::catalog::{GalleryEntry, GalleryMod, share_code};

        const TWEAKS_ONLY: GalleryEntry = GalleryEntry {
            id: "tweaks-only",
            name: "Tweaks only",
            author: "Test",
            game: Game::BGEE,
            tags: &[],
            starter: false,
            sample: false,
            description: "",
            requirements: "",
            version: "1.0.0",
            mods: &[GalleryMod {
                mod_name: "CDTweaks",
                tp_file: "SETUP-CDTWEAKS.TP2",
                component_id: "2010",
                component_label: "Increase Ammo Stacking",
                target: Game::BGEE,
                wlb_inputs: None,
            }],
        };

        let source =
            std::env::temp_dir().join(format!("bio-reinstall-source-{}", std::process::id()));
        std::fs::create_dir_all(&source).expect("create source fixture");
        std::fs::write(source.join("sod-dlc.zip"), b"synthetic archive fixture")
            .expect("write synthetic archive");

        let mut app = orch_for_reinstall_test();
        app.wizard_state.step1.bgee_game_folder = source.to_string_lossy().into_owned();
        assert!(crate::app::compat_dlc_source::refresh_source_check(
            &mut app.wizard_state.step1
        ));
        let modlist = ModlistEntry {
            game: Game::BGEE,
            latest_share_code: Some(share_code(&TWEAKS_ONLY).expect("tweaks-only code")),
            ..entry()
        };

        start_reinstall(&modlist, &mut app);

        assert_eq!(app.install_screen_state.stage, InstallStage::Details);
        assert!(app.install_screen_state.source_compat_issue.is_some());

        std::fs::remove_dir_all(&source).expect("clean up source fixture");
    }

    #[test]
    fn reinstall_with_unreadable_code_stays_put_and_reports() {
        let modlist = ModlistEntry {
            latest_share_code: Some("not a share code".to_string()),
            ..entry()
        };
        let mut app = orch_for_reinstall_test();
        let nav_before = app.nav.clone();
        let stage_before = app.install_screen_state.stage;

        start_reinstall(&modlist, &mut app);

        assert_eq!(app.nav, nav_before);
        assert_eq!(app.install_screen_state.stage, stage_before);
        assert!(app.pending_reinstall_id.is_none());
        let errors: Vec<_> = app
            .notification_manager
            .history()
            .iter()
            .filter(|record| matches!(record.kind, egui_toast::ToastKind::Error))
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].text.contains(&modlist.name));
    }

    #[test]
    fn reinstall_back_returns_to_the_gallery() {
        assert_eq!(ReviewOrigin::Reinstall.back_stage(), InstallStage::Gallery);
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
