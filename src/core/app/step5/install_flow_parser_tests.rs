use super::*;
use crate::platform_defaults::resolve_mod_installer_binary;

fn assert_worker_uses_effective_program(configured: &str) {
    let expected = resolve_mod_installer_binary(configured);
    let step1 = Step1State {
        mod_installer_binary: configured.to_string(),
        ..Default::default()
    };
    let (observed_tx, observed_rx) = std::sync::mpsc::channel();
    let receiver = spawn_install_preparation_with(
        step1,
        vec!["Imported".to_string()],
        false,
        901,
        move |executable, phrases| {
            observed_tx
                .send((executable.to_path_buf(), phrases.to_vec()))
                .expect("observed executable");
            Err("fixture failure before target preparation".to_string())
        },
    );
    let (observed, phrases) = observed_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("injected preparer called");
    let prepared = receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("worker result")
        .unwrap_or_else(|error| panic!("phrase-bearing preparation must not block: {error}"));
    assert_eq!(phrases, ["Imported"]);
    assert_eq!(
        prepared.parser_warning.as_deref(),
        Some("fixture failure before target preparation")
    );
    assert_eq!(observed, Path::new(&expected));
}

#[test]
fn parser_sharing_blank_worker_prepares_the_default_install_program() {
    assert_worker_uses_effective_program("   ");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn parser_sharing_alias_worker_prepares_the_normalized_install_program() {
    assert_worker_uses_effective_program(" mod_installer.exe ");
}

use std::path::PathBuf;

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    fn new() -> std::io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("parser-sharing-resolution-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&directory)?;
        Ok(Self(directory))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}

fn fixture_executable(directory: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, bytes).expect("fixture executable bytes");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .expect("fixture executable permissions");
    }
    path
}

#[test]
fn parser_sharing_explicit_paths_resolve_blank_default_without_environment_changes() {
    let root = FixtureDirectory::new().expect("fixture root");
    let name = crate::platform_defaults::default_mod_installer_binary();
    let executable = fixture_executable(root.path(), &name, b"default fixture");
    let identity = executable_identity_with_paths("  ", &[root.path().to_path_buf()])
        .expect("default executable identity");
    assert_eq!(
        identity.0,
        executable.canonicalize().expect("canonical fixture")
    );
    assert_eq!(identity.1, 15);
}

#[test]
fn parser_sharing_explicit_paths_bind_selection_order_and_replacement() {
    let first = FixtureDirectory::new().expect("first search directory");
    let second = FixtureDirectory::new().expect("second search directory");
    let name = crate::platform_defaults::default_mod_installer_binary();
    let first_executable = fixture_executable(first.path(), &name, b"first");
    let second_executable = fixture_executable(second.path(), &name, b"second");
    let paths = [first.path().to_path_buf(), second.path().to_path_buf()];
    let original = executable_identity_with_paths(&name, &paths).expect("first identity");
    assert_eq!(
        original.0,
        first_executable.canonicalize().expect("first canonical")
    );
    std::fs::write(&first_executable, b"replacement with different length")
        .expect("replace selected executable bytes");
    let replaced = executable_identity_with_paths(&name, &paths).expect("replacement identity");
    assert_ne!(original, replaced);
    assert_eq!(original.0, replaced.0);
    std::fs::remove_file(first_executable).expect("remove first fixture candidate");
    let selected = executable_identity_with_paths(&name, &paths).expect("second selection");
    assert_eq!(
        selected.0,
        second_executable.canonicalize().expect("second canonical")
    );
    assert_ne!(replaced, selected);
}

#[test]
fn parser_sharing_explicit_configured_path_precedes_search_and_rejects_directories() {
    let configured_root = FixtureDirectory::new().expect("configured directory");
    let search_root = FixtureDirectory::new().expect("search directory");
    let name = crate::platform_defaults::default_mod_installer_binary();
    let configured = fixture_executable(configured_root.path(), &name, b"configured");
    fixture_executable(search_root.path(), &name, b"search");
    let paths = [search_root.path().to_path_buf()];
    let identity = executable_identity_with_paths(&configured.to_string_lossy(), &paths)
        .expect("configured identity");
    assert_eq!(
        identity.0,
        configured.canonicalize().expect("configured canonical")
    );
    assert!(
        executable_identity_with_paths(&configured_root.path().to_string_lossy(), &paths).is_none()
    );
    assert!(executable_identity_with_paths("bio-missing-fixture-executable", &paths).is_none());
}

#[cfg(not(target_os = "windows"))]
#[test]
fn parser_sharing_explicit_paths_normalize_bare_exe_alias() {
    let root = FixtureDirectory::new().expect("alias directory");
    let executable = fixture_executable(root.path(), "mod_installer", b"normalized");
    fixture_executable(root.path(), "mod_installer.exe", b"wrong alias");
    let identity =
        executable_identity_with_paths(" mod_installer.exe ", &[root.path().to_path_buf()])
            .expect("normalized alias identity");
    assert_eq!(
        identity.0,
        executable.canonicalize().expect("normalized canonical")
    );
}

#[cfg(unix)]
#[test]
fn parser_sharing_explicit_paths_skip_nonexecutable_candidates() {
    use std::os::unix::fs::PermissionsExt;
    let first = FixtureDirectory::new().expect("nonexecutable directory");
    let second = FixtureDirectory::new().expect("executable directory");
    let name = crate::platform_defaults::default_mod_installer_binary();
    let nonexecutable = fixture_executable(first.path(), &name, b"nonexecutable");
    std::fs::set_permissions(nonexecutable, std::fs::Permissions::from_mode(0o600))
        .expect("nonexecutable permissions");
    let executable = fixture_executable(second.path(), &name, b"executable");
    let identity = executable_identity_with_paths(
        &name,
        &[first.path().to_path_buf(), second.path().to_path_buf()],
    )
    .expect("executable identity");
    assert_eq!(
        identity.0,
        executable.canonicalize().expect("executable canonical")
    );
}
