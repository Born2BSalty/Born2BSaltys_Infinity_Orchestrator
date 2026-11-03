use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::extract_rar_archive;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
const FIXTURE: &[u8] = include_bytes!("test_data/rar-single-file.rar");
const PAYLOAD: &[u8] = b"BIO exact-path RAR regression\n";

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "bio-rar-exact-path-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn archive(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, FIXTURE).unwrap();
        path
    }

    fn output(&self) -> PathBuf {
        let path = self.0.join("extracted");
        fs::create_dir(&path).unwrap();
        path
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn assert_extracts(name: &str) {
    let root = TestRoot::new();
    let archive = root.archive(name);
    let output = root.output();
    assert_eq!(extract_rar_archive(&archive, &output), Ok(()));
    assert_eq!(fs::read(output.join("payload.txt")).unwrap(), PAYLOAD);
    assert_eq!(fs::read_dir(output).unwrap().count(), 1);
    assert_eq!(fs::read(archive).unwrap(), FIXTURE);
}

#[test]
fn rar_exact_path_extracts_nonversioned_archive() {
    assert_extracts("peacespeaker.rar");
}

#[test]
fn rar_exact_path_extracts_v2_3_archive() {
    assert_extracts("peacespeaker__bucketfulofsunshine__V2.3.rar");
}

#[test]
fn rar_exact_path_extracts_v1_12_archive() {
    assert_extracts("mod__source__v1.12.rar");
}

#[test]
fn rar_exact_path_missing_input_does_not_extract_a_guessed_sibling() {
    let root = TestRoot::new();
    root.archive("peacespeaker__bucketfulofsunshine__V2.1.rar");
    let missing = root.0.join("peacespeaker__bucketfulofsunshine__V2.3.rar");
    let output = root.output();
    assert!(extract_rar_archive(&missing, &output).is_err());
    assert_eq!(fs::read_dir(output).unwrap().count(), 0);
}
