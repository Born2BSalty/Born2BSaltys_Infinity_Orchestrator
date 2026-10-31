use super::*;
use crate::app::tool_version_probe::ProbeOutput;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn output(stdout: &str, stderr: &str, success: bool) -> ProbeOutput {
    ProbeOutput {
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        success,
    }
}

struct FixtureFile(PathBuf);

impl std::ops::Deref for FixtureFile {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for FixtureFile {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        if let Some(parent) = self.0.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}

fn fixture_file(name: &str) -> FixtureFile {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bio_tool_validation_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::create_dir(&root).unwrap();
    let path = root.join(name);
    std::fs::write(&path, b"controlled test fixture").unwrap();
    FixtureFile(path)
}

fn fake_ticket() -> ProbeTicket {
    Arc::new(Mutex::new(None))
}

fn observe(
    state: &mut ToolValidationState,
    kind: ToolKind,
    value: &str,
    at: Instant,
    ticket: &ProbeTicket,
    calls: &AtomicUsize,
) -> ToolStatus {
    state.observe_with(kind, value, at, |_, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::clone(ticket))
    })
}

#[test]
fn guidance_and_release_links_cover_windows_linux_and_macos() {
    for platform in [
        ToolPlatform::Windows,
        ToolPlatform::Linux,
        ToolPlatform::MacOs,
    ] {
        let text = guidance(ToolKind::WeiDu, platform);
        let installer = guidance(ToolKind::ModInstaller, platform);
        assert!(text.contains("extracted"));
        assert!(installer.contains("official release"));
        match platform {
            ToolPlatform::Windows => {
                assert!(text.contains("weidu.exe"));
                assert!(installer.contains("Windows"));
            }
            ToolPlatform::Linux => {
                assert!(text.contains("weidu"));
                assert!(!text.contains(".exe"));
                assert!(text.contains("Linux"));
                assert!(installer.contains("Linux"));
                assert!(!installer.contains(".exe"));
            }
            ToolPlatform::MacOs => {
                assert!(text.contains("weidu"));
                assert!(!text.contains(".exe"));
                assert!(text.contains("macOS"));
                assert!(installer.contains("macOS"));
                assert!(!installer.contains(".exe"));
            }
        }
    }
    assert_eq!(
        release_url(ToolKind::WeiDu),
        "https://github.com/WeiDUorg/weidu/releases"
    );
    assert_eq!(
        release_url(ToolKind::ModInstaller),
        "https://github.com/dark0dave/mod_installer/releases"
    );
}

#[test]
fn weidu_identity_comes_from_output_including_stderr_and_path_prefix() {
    for sample in [
        output("[C:/renamed/anything.exe] WeiDU version 24900\n", "", true),
        output("", "WeiDU version 25100\nhelp text", false),
    ] {
        assert!(matches!(
            classify(ToolKind::WeiDu, &sample),
            ToolStatus::Detected(version) if version == "24900" || version == "25100"
        ));
    }
}

#[test]
fn mod_installer_identity_accepts_known_names_and_semantic_versions() {
    for identity in ["mod_installer", "mod-installer", "mod_runner"] {
        assert_eq!(
            classify(
                ToolKind::ModInstaller,
                &output(&format!("{identity} v15.0.1-beta.2+test\n"), "", true),
            ),
            ToolStatus::Detected("15.0.1-beta.2+test".to_string()),
        );
    }
}

#[test]
fn unrelated_configuration_helpers_and_version_numbers_are_not_tools() {
    for sample in [
        "config 1.1.0",
        "another_program 15.0.1",
        "15.0.1",
        "mod_installer bananas",
        "WeiDU version nonsense",
        "",
    ] {
        for kind in [ToolKind::WeiDu, ToolKind::ModInstaller] {
            assert!(!matches!(
                classify(kind, &output(sample, "", true)),
                ToolStatus::Detected(_)
            ));
        }
    }
    assert!(!matches!(
        classify(
            ToolKind::ModInstaller,
            &output("mod_installer 15.0.1", "aborted", false)
        ),
        ToolStatus::Detected(_)
    ));
}

#[test]
fn pending_probe_is_nonblocking_deduplicated_and_then_cached() {
    let path = fixture_file("renamed-valid-tool.bin");
    let value = path.to_str().unwrap();
    let mut state = ToolValidationState::default();
    let ticket = fake_ticket();
    let calls = AtomicUsize::new(0);
    let start = Instant::now();
    assert_eq!(
        observe(&mut state, ToolKind::WeiDu, value, start, &ticket, &calls),
        ToolStatus::Pending
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let settled = start + Duration::from_millis(201);
    assert_eq!(
        observe(&mut state, ToolKind::WeiDu, value, settled, &ticket, &calls),
        ToolStatus::Pending
    );
    for step in 1..8 {
        assert_eq!(
            observe(
                &mut state,
                ToolKind::WeiDu,
                value,
                settled + Duration::from_millis(step),
                &ticket,
                &calls
            ),
            ToolStatus::Pending
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    *ticket.lock().unwrap() = Some(ToolStatus::Detected("24900".to_string()));
    for step in 9..12 {
        assert_eq!(
            observe(
                &mut state,
                ToolKind::WeiDu,
                value,
                settled + Duration::from_millis(step),
                &ticket,
                &calls
            ),
            ToolStatus::Detected("24900".to_string())
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn changed_path_discards_previous_green_and_stale_completion() {
    let first = fixture_file("first.exe");
    let second = fixture_file("second.exe");
    let mut state = ToolValidationState::default();
    let old = fake_ticket();
    let fresh = fake_ticket();
    let calls = AtomicUsize::new(0);
    let start = Instant::now();
    observe(
        &mut state,
        ToolKind::WeiDu,
        first.to_str().unwrap(),
        start,
        &old,
        &calls,
    );
    observe(
        &mut state,
        ToolKind::WeiDu,
        first.to_str().unwrap(),
        start + Duration::from_millis(201),
        &old,
        &calls,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        observe(
            &mut state,
            ToolKind::WeiDu,
            second.to_str().unwrap(),
            start + Duration::from_millis(202),
            &fresh,
            &calls
        ),
        ToolStatus::Pending
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    *old.lock().unwrap() = Some(ToolStatus::Detected("old-version".to_string()));
    assert_eq!(
        observe(
            &mut state,
            ToolKind::WeiDu,
            second.to_str().unwrap(),
            start + Duration::from_millis(403),
            &fresh,
            &calls
        ),
        ToolStatus::Pending
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    *fresh.lock().unwrap() = Some(ToolStatus::Detected("new-version".to_string()));
    assert_eq!(
        observe(
            &mut state,
            ToolKind::WeiDu,
            second.to_str().unwrap(),
            start + Duration::from_millis(404),
            &fresh,
            &calls
        ),
        ToolStatus::Detected("new-version".to_string())
    );
}

#[test]
fn replacing_the_same_filename_invalidates_cached_identity() {
    let path = fixture_file("same-name.exe");
    let mut state = ToolValidationState::default();
    let initial = fake_ticket();
    let retry = fake_ticket();
    let calls = AtomicUsize::new(0);
    let start = Instant::now();
    observe(
        &mut state,
        ToolKind::WeiDu,
        path.to_str().unwrap(),
        start,
        &initial,
        &calls,
    );
    observe(
        &mut state,
        ToolKind::WeiDu,
        path.to_str().unwrap(),
        start + Duration::from_millis(201),
        &initial,
        &calls,
    );
    *initial.lock().unwrap() = Some(ToolStatus::Detected("old".to_string()));
    observe(
        &mut state,
        ToolKind::WeiDu,
        path.to_str().unwrap(),
        start + Duration::from_millis(202),
        &initial,
        &calls,
    );
    std::fs::write(&path, b"replacement with a changed file length").unwrap();
    assert_eq!(
        observe(
            &mut state,
            ToolKind::WeiDu,
            path.to_str().unwrap(),
            start + Duration::from_millis(203),
            &retry,
            &calls
        ),
        ToolStatus::Pending
    );
    observe(
        &mut state,
        ToolKind::WeiDu,
        path.to_str().unwrap(),
        start + Duration::from_millis(404),
        &retry,
        &calls,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn clear_missing_and_directory_paths_never_launch_a_probe() {
    let file = fixture_file("fixture.txt");
    let absent = file.with_file_name("absent.exe");
    let mut state = ToolValidationState::default();
    let calls = AtomicUsize::new(0);
    let ticket = fake_ticket();
    let now = Instant::now();
    assert_eq!(
        observe(&mut state, ToolKind::WeiDu, "", now, &ticket, &calls),
        ToolStatus::Empty
    );
    for path in [&absent, file.parent().unwrap()] {
        assert!(matches!(
            observe(
                &mut state,
                ToolKind::WeiDu,
                path.to_str().unwrap(),
                now,
                &ticket,
                &calls
            ),
            ToolStatus::InvalidPath(_)
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn timeout_failure_is_cached_without_repeated_workers() {
    let path = fixture_file("timeout.exe");
    let mut state = ToolValidationState::default();
    let calls = AtomicUsize::new(0);
    let ticket = fake_ticket();
    let start = Instant::now();
    observe(
        &mut state,
        ToolKind::ModInstaller,
        path.to_str().unwrap(),
        start,
        &ticket,
        &calls,
    );
    observe(
        &mut state,
        ToolKind::ModInstaller,
        path.to_str().unwrap(),
        start + Duration::from_millis(201),
        &ticket,
        &calls,
    );
    *ticket.lock().unwrap() = Some(ToolStatus::Failed("version query timed out".to_string()));
    let status = observe(
        &mut state,
        ToolKind::ModInstaller,
        path.to_str().unwrap(),
        start + Duration::from_millis(202),
        &ticket,
        &calls,
    );
    assert!(matches!(status, ToolStatus::Failed(_)));
    assert!(status.hint(ToolKind::ModInstaller).contains("timed out"));
    observe(
        &mut state,
        ToolKind::ModInstaller,
        path.to_str().unwrap(),
        start + Duration::from_secs(10),
        &ticket,
        &calls,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn local_relative_file_is_resolved_for_the_probe_without_rewriting_the_request() {
    let mut state = ToolValidationState::default();
    let ticket = fake_ticket();
    let now = Instant::now();
    let request = "./Cargo.toml";
    let expected = std::fs::canonicalize(request).unwrap();
    state.observe_with(ToolKind::WeiDu, request, now, |_, _| {
        panic!("not debounced")
    });
    let mut seen = None;
    state.observe_with(
        ToolKind::WeiDu,
        request,
        now + Duration::from_millis(201),
        |_, path| {
            assert!(path.is_absolute());
            seen = Some(path.to_path_buf());
            Ok(Arc::clone(&ticket))
        },
    );
    assert_eq!(seen, Some(expected));
}

#[test]
fn two_tool_slots_do_not_spawn_more_jobs_during_rapid_path_changes() {
    let first = fixture_file("first.exe");
    let alternate = fixture_file("alternate.exe");
    let mut state = ToolValidationState::default();
    let calls = AtomicUsize::new(0);
    let ticket = fake_ticket();
    let start = Instant::now();
    for kind in [ToolKind::WeiDu, ToolKind::ModInstaller] {
        observe(
            &mut state,
            kind,
            first.to_str().unwrap(),
            start,
            &ticket,
            &calls,
        );
        observe(
            &mut state,
            kind,
            first.to_str().unwrap(),
            start + Duration::from_millis(201),
            &ticket,
            &calls,
        );
    }
    for step in 1..10 {
        let value = if step % 2 == 0 { &first } else { &alternate };
        for kind in [ToolKind::WeiDu, ToolKind::ModInstaller] {
            assert_eq!(
                observe(
                    &mut state,
                    kind,
                    value.to_str().unwrap(),
                    start + Duration::from_secs(step),
                    &ticket,
                    &calls
                ),
                ToolStatus::Pending
            );
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn launch_failure_becomes_a_stable_status_without_an_inflight_job() {
    let path = fixture_file("spawn-failure.exe");
    let mut state = ToolValidationState::default();
    let now = Instant::now();
    state.observe_with(ToolKind::WeiDu, path.to_str().unwrap(), now, |_, _| {
        panic!("not debounced")
    });
    let status = state.observe_with(
        ToolKind::WeiDu,
        path.to_str().unwrap(),
        now + Duration::from_millis(201),
        |_, _| Err("controlled launch failure".to_string()),
    );
    assert_eq!(
        status,
        ToolStatus::Failed("controlled launch failure".to_string())
    );
    assert_eq!(
        state.observe_with(
            ToolKind::WeiDu,
            path.to_str().unwrap(),
            now + Duration::from_secs(2),
            |_, _| panic!("a failed probe must stay cached"),
        ),
        status
    );
}
