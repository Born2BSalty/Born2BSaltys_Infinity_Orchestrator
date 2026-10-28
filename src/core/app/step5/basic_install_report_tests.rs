// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::*;

#[test]
fn completion_distinguishes_exit_failure_console_failure_and_cancel() {
    let mut step5 = Step5State {
        last_exit_code: Some(0),
        ..Step5State::default()
    };
    assert_eq!(completion_state(&step5), "finished");
    step5.last_exit_code = Some(1);
    assert_eq!(completion_state(&step5), "failed");
    step5.last_exit_code = Some(0);
    step5.last_install_failed = true;
    assert_eq!(completion_state(&step5), "failed");
    step5.last_exit_code = Some(1);
    step5.last_cancel_mode = "force".to_string();
    assert_eq!(completion_state(&step5), "stopped");
    step5.last_cancel_mode = "graceful".to_string();
    assert_eq!(completion_state(&step5), "stopped");
}

#[test]
fn report_is_beside_targets_for_all_supported_games() {
    let workspace = std::env::temp_dir().join("bio_report_path_test");
    let mut step1 = Step1State::default();
    let expected = workspace.join("bio_support").join("install_test-run.txt");
    for game in ["BGEE", "BG2EE", "IWDEE", "EET"] {
        step1.game_install = game.to_string();
        step1.generate_directory = workspace.join("single-game").to_string_lossy().into_owned();
        step1.eet_pre_dir = workspace.join("bg1-target").to_string_lossy().into_owned();
        step1.eet_new_dir = workspace.join("bg2-target").to_string_lossy().into_owned();
        assert_eq!(report_path(&step1, "test-run"), Some(expected.clone()));
    }
    step1.eet_pre_dir.clear();
    assert_eq!(report_path(&step1, "test-run"), None);
    step1.game_install = "BG2EE".to_string();
    step1.generate_directory.clear();
    assert_eq!(report_path(&step1, "test-run"), None);
}

#[test]
fn late_tool_versions_preserve_final_failure_report_and_previous_runs() {
    let root = std::env::temp_dir().join(format!(
        "bio_report_lifecycle_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    fs::create_dir(&root).unwrap();
    let mut state = WizardState::default();
    state.step1.game_install = "BG2EE".to_string();
    state.step1.generate_directory = root.join("game").to_string_lossy().into_owned();
    state.step1.mod_installer_binary.clear();
    state.step1.weidu_binary.clear();
    state.modlist_share_name = Some("Report test modlist".to_string());
    state.step5.install_choice = "Review and modify".to_string();
    state.step5.install_source = "Pasted share code".to_string();

    begin(&mut state, "first");
    let first = state.step5.basic_install_report_path.clone().unwrap();
    assert!(first.is_file());
    state.step5.last_status_text = "Install start failed: missing executable".to_string();
    update(&mut state, "start_failed", None);
    state.step5.basic_install_tool_versions = Some(BasicInstallToolVersionLookup(Arc::new(
        Mutex::new(Some(("installer 1.0".to_string(), "WeiDU 249".to_string()))),
    )));
    poll(&mut state, None);
    let report = fs::read_to_string(&first).unwrap();
    for expected in [
        "State: start_failed",
        "Name: Report test modlist",
        "Install source: Pasted share code",
        "Install choice: Review and modify",
        "Install start failed: missing executable",
        "Version: installer 1.0",
        "Version: WeiDU 249",
    ] {
        assert!(
            report.contains(expected),
            "missing report field: {expected}"
        );
    }
    assert!(state.step5.basic_install_tool_versions.is_none());
    begin(&mut state, "second");
    assert_ne!(state.step5.basic_install_report_path.as_ref(), Some(&first));
    assert_eq!(fs::read_to_string(first).unwrap(), report);
    assert!(root.starts_with(std::env::temp_dir()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn version_query_kills_a_hung_probe() {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args([
        "--exact",
        "app::step5::basic_install_report::tests::version_probe_child",
    ]);
    command.env("BIO_REPORT_TEST_HANG_PROBE", "1");
    let start = std::time::Instant::now();
    assert_eq!(
        command_version(command, Duration::from_millis(100)),
        "version query timed out"
    );
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
fn version_probe_child() {
    if std::env::var_os("BIO_REPORT_TEST_HANG_PROBE").is_some() {
        std::thread::sleep(Duration::from_secs(30));
    }
}
