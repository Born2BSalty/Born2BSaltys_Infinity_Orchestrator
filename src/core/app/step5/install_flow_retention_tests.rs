use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wait_timeout::ChildExt;

use super::*;
use crate::app::app_step5_flow::start_if_requested;

const CHILD_ROOT: &str = "BIO_RETENTION_TEST_ROOT";
const CHILD_CASE: &str = "BIO_RETENTION_TEST_CASE";
const CURRENT_ID: &str = "retention_current";

fn isolated(case: &str, scenario: fn()) {
    if std::env::var(CHILD_CASE).as_deref() == Ok(case) {
        let root = PathBuf::from(std::env::var_os(CHILD_ROOT).expect("sandbox root"));
        assert!(root.is_absolute());
        assert_eq!(
            fs::canonicalize(root).unwrap(),
            fs::canonicalize(std::env::current_dir().unwrap()).unwrap()
        );
        scenario();
        return;
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bio_retention_{}_{nonce}_{case}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("app::step5::install_flow::retention_tests::{case}"),
            "--nocapture",
        ])
        .env(CHILD_ROOT, &root)
        .env(CHILD_CASE, case)
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let finished = child
        .wait_timeout(Duration::from_secs(30))
        .unwrap()
        .is_some();
    if !finished {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    let output = child.wait_with_output().unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert!(
        finished && output.status.success(),
        "{case}:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture() -> WizardState {
    for directory in [
        "diagnostics/run_previous/nested",
        "diagnostics/unrelated",
        "bio_support",
    ] {
        fs::create_dir_all(directory).unwrap();
    }
    for (path, bytes) in [
        (
            "diagnostics/run_previous/nested/sentinel",
            "previous diagnostics",
        ),
        ("diagnostics/unrelated/sentinel", "unrelated directory"),
        ("diagnostics/run_notes", "unrelated file"),
        ("bio_support/install_previous.txt", "previous report"),
    ] {
        fs::write(path, bytes).unwrap();
    }
    let mut state = WizardState::default();
    let root = fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
    let missing = root.join("missing").to_string_lossy().into_owned();
    let step1 = &mut state.step1;
    for path in [
        &mut step1.mod_installer_binary,
        &mut step1.weidu_binary,
        &mut step1.bgee_game_folder,
        &mut step1.bgee_log_folder,
        &mut step1.bgee_log_file,
        &mut step1.bg2ee_game_folder,
        &mut step1.bg2ee_log_folder,
        &mut step1.bg2ee_log_file,
        &mut step1.iwdee_game_folder,
        &mut step1.eet_bgee_game_folder,
        &mut step1.eet_bgee_log_folder,
        &mut step1.eet_bg2ee_game_folder,
        &mut step1.eet_bg2ee_log_folder,
        &mut step1.eet_pre_dir,
        &mut step1.eet_new_dir,
        &mut step1.weidu_log_folder,
        &mut step1.log_file,
        &mut step1.mods_folder,
        &mut step1.global_mods_folder,
        &mut step1.mods_archive_folder,
        &mut step1.mods_backup_folder,
    ] {
        path.clone_from(&missing);
    }
    step1.game_install = "BGEE".to_string();
    step1.generate_directory = root.join("target_game").to_string_lossy().into_owned();
    step1.generate_directory_enabled = false;
    step1.bio_full_debug = false;
    step1.log_raw_output_dev = false;
    state
}

fn assert_unrelated_preserved() {
    for (path, expected) in [
        ("diagnostics/unrelated/sentinel", "unrelated directory"),
        ("diagnostics/run_notes", "unrelated file"),
        ("bio_support/install_previous.txt", "previous report"),
    ] {
        assert_eq!(fs::read_to_string(path).unwrap(), expected, "{path}");
    }
}

fn assert_previous_preserved() {
    assert_eq!(
        fs::read_to_string("diagnostics/run_previous/nested/sentinel")
            .expect("previous diagnostics must survive an attempt that did not start"),
        "previous diagnostics"
    );
    assert_unrelated_preserved();
}

fn assert_report(state: &WizardState, expected: &str) {
    assert_eq!(state.step5.basic_install_report_state, expected);
    let path = state
        .step5
        .basic_install_report_path
        .as_ref()
        .expect("attempt report path");
    assert!(path.starts_with(fs::canonicalize(std::env::current_dir().unwrap()).unwrap()));
    assert!(
        fs::read_to_string(path)
            .unwrap()
            .contains(&format!("State: {expected}\n"))
    );
}

fn begin_attempt(state: &mut WizardState) {
    let id = begin_new_run(&mut state.step5);
    basic_install_report::begin(state, &id);
}

#[test]
fn allocation_preserves_diagnostics() {
    isolated("allocation_preserves_diagnostics", || {
        let mut state = fixture();
        let id = begin_new_run(&mut state.step5);
        assert_eq!(state.step5.diagnostics_run_id.as_deref(), Some(id.as_str()));
        assert_previous_preserved();
    });
}

#[test]
fn terminal_unavailable_preserves_diagnostics() {
    isolated("terminal_unavailable_preserves_diagnostics", || {
        let mut state = fixture();
        state.step5.start_install_requested = true;
        assert!(!start_if_requested(
            &mut state,
            &mut None,
            &mut Some("sandbox terminal unavailable".to_string()),
            &mut None,
            &mut None
        ));
        assert!(!state.step5.install_running);
        assert_report(&state, "start_failed");
        assert_previous_preserved();
    });
}

#[test]
fn preflight_failure_preserves_diagnostics() {
    isolated("preflight_failure_preserves_diagnostics", || {
        let mut state = fixture();
        state.step5.start_install_requested = true;
        let mut terminal = Some(EmbeddedTerminal::new().unwrap());
        assert!(!start_if_requested(
            &mut state,
            &mut terminal,
            &mut None,
            &mut None,
            &mut None
        ));
        assert!(!state.step5.install_running);
        assert_report(&state, "preflight_failed");
        assert_previous_preserved();
    });
}

#[test]
fn preparation_failure_preserves_diagnostics() {
    isolated("preparation_failure_preserves_diagnostics", || {
        let mut state = fixture();
        begin_attempt(&mut state);
        state.step5.prep_running = true;
        let mut terminal = EmbeddedTerminal::new().unwrap();
        assert!(!apply_completed_prep(
            &mut state,
            &mut terminal,
            Err("sandbox preparation failed".to_string())
        ));
        assert!(!state.step5.prep_running);
        assert!(!state.step5.install_running);
        assert_report(&state, "preparation_failed");
        assert_previous_preserved();
    });
}

fn process_attempt(success: bool) {
    let mut state = fixture();
    state.step5.diagnostics_run_id = Some(CURRENT_ID.to_string());
    basic_install_report::begin(&mut state, CURRENT_ID);
    let current = format!("diagnostics/run_{CURRENT_ID}/sentinel");
    fs::create_dir_all(format!("diagnostics/run_{CURRENT_ID}")).unwrap();
    fs::write(&current, "current diagnostics").unwrap();
    state.step1.log_raw_output_dev = true;
    let pending = PendingInstallStart {
        program: if success {
            std::env::current_exe().unwrap()
        } else {
            std::env::current_dir()
                .unwrap()
                .join("missing-installer.exe")
        }
        .to_string_lossy()
        .into_owned(),
        args: vec![
            "--exact".to_string(),
            "app::step5::install_flow::retention_tests::harmless_install_child".to_string(),
        ],
        resume_mode: false,
        restart_mode: false,
    };
    let mut terminal = EmbeddedTerminal::new().unwrap();
    start_install_process(&mut state, &mut terminal, pending);
    let deadline = Instant::now() + Duration::from_secs(10);
    while terminal.process_id().is_some() && Instant::now() < deadline {
        terminal.poll_output();
        std::thread::sleep(Duration::from_millis(10));
    }
    let exited = terminal.process_id().is_none();
    if !exited {
        terminal.force_terminate();
    }
    assert!(
        exited,
        "harmless install child must exit before sandbox cleanup"
    );
    assert_eq!(state.step5.install_running, success);
    assert_report(&state, if success { "running" } else { "start_failed" });
    assert_eq!(fs::read_to_string(current).unwrap(), "current diagnostics");
    assert_unrelated_preserved();
    if success {
        assert_eq!(terminal.take_exit_code(), Some(0));
        assert!(
            !std::path::Path::new("diagnostics/run_previous").exists(),
            "successful process start must prune previous diagnostics"
        );
    } else {
        assert_previous_preserved();
    }
}

#[test]
fn failed_spawn_preserves_diagnostics() {
    isolated("failed_spawn_preserves_diagnostics", || {
        process_attempt(false);
    });
}

#[test]
fn successful_spawn_prunes_only_previous_diagnostics() {
    isolated("successful_spawn_prunes_only_previous_diagnostics", || {
        process_attempt(true);
    });
}

#[test]
fn harmless_install_child() {}
