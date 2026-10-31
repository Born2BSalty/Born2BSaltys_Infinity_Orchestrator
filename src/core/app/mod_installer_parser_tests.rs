use super::{
    ParserInitializer, VersionOutput, capture_choice_phrases_at, merge_parser_documents,
    prepare_parser_at, union_phrases, validate_phrases,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "bio-parser-sharing-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).expect("isolated fixture");
        Self(path)
    }

    fn parser(&self) -> PathBuf {
        self.0.join("parser.toml")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Initializer {
    path: PathBuf,
    document: String,
    calls: AtomicUsize,
    fail_after_write: bool,
}

impl ParserInitializer for Initializer {
    fn run_version(&self, _executable: &Path) -> Result<VersionOutput, String> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        std::fs::write(&self.path, &self.document).map_err(|error| error.to_string())?;
        if self.fail_after_write {
            return Err("fixture initializer failed".to_string());
        }
        Ok(VersionOutput {
            display: "mod_installer 15.0.1".to_string(),
            version: "15.0.1".to_string(),
        })
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn document(text: &str) -> toml::Value {
    toml::from_str(text).expect("fixture TOML")
}

const CURRENT: &str = "choice_phrase = ['Default', 'Shared']\nnew_setting = true\n[metadata]\nmod_installer_version = '15.0.1'\nnew_metadata = 42\n[settings]\nanswer = 'new'\nnew_default = true\n";
const ORIGINAL: &str = "choice_phrase = ['Shared', ' Custom 世界 ']\nunknown_top = [1, 2]\n[metadata]\nmod_installer_version = '14.0.0'\nuser_note = 'keep'\n[settings]\nanswer = 'old'\n[settings.nested]\ncustom = 7\n";

#[test]
fn parser_sharing_union_preserves_exact_text_and_first_seen_order() {
    let first = strings(&["Default", " Custom 世界 ", "default"]);
    let second = strings(&["Default", "Other", " Custom 世界 "]);
    assert_eq!(
        union_phrases(&[&first, &second]).expect("union"),
        strings(&["Default", " Custom 世界 ", "default", "Other"])
    );
}

#[test]
fn parser_sharing_limits_are_utf8_bytes_and_apply_to_final_union() {
    assert!(validate_phrases(&strings(&[""])).is_err());
    assert!(validate_phrases(&strings(&[" \t\n"])).is_err());
    assert!(validate_phrases(&["é".repeat(4096)]).is_ok());
    assert!(validate_phrases(&["é".repeat(4097)]).is_err());
    let maximum: Vec<String> = (0..4096).map(|index| format!("phrase-{index}")).collect();
    assert!(validate_phrases(&maximum).is_ok());
    assert!(union_phrases(&[&maximum, &strings(&["extra"])]).is_err());
    let total: Vec<String> = (0..65)
        .map(|index| format!("{index:04}{}", "x".repeat(8188)))
        .collect();
    assert!(validate_phrases(&total).is_err());
}

#[test]
fn parser_sharing_capture_is_bounded_and_never_initializes_missing_file() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    assert_eq!(
        capture_choice_phrases_at(&path).expect("missing"),
        Vec::<String>::new()
    );
    assert!(!path.exists());
    std::fs::write(&path, CURRENT).expect("fixture write");
    assert_eq!(
        capture_choice_phrases_at(&path).expect("capture defaults"),
        strings(&["Default", "Shared"])
    );
    for invalid in [
        "broken = [",
        "choice_phrase = 3",
        "choice_phrase = ['valid', 2]",
        "[metadata]\nversion = 'x'",
    ] {
        std::fs::write(&path, invalid).expect("fixture write");
        assert!(capture_choice_phrases_at(&path).is_err(), "{invalid}");
        assert_eq!(std::fs::read_to_string(&path).expect("unchanged"), invalid);
    }
    std::fs::write(&path, vec![b' '; 2 * 1024 * 1024 + 1]).expect("oversize fixture");
    assert!(capture_choice_phrases_at(&path).is_err());
}

#[test]
fn parser_sharing_merge_preserves_old_settings_and_uses_current_metadata() {
    let original = document(ORIGINAL);
    let current = document(CURRENT);
    let imported = strings(&["Imported", "Default"]);
    let merged = merge_parser_documents(Some(&original), &current, &imported).expect("merge");
    assert_eq!(
        merged["choice_phrase"],
        toml::Value::Array(
            strings(&["Default", "Shared", " Custom 世界 ", "Imported"])
                .into_iter()
                .map(toml::Value::String)
                .collect()
        )
    );
    assert_eq!(merged["settings"]["answer"].as_str(), Some("old"));
    assert_eq!(merged["settings"]["nested"], original["settings"]["nested"]);
    assert_eq!(merged["unknown_top"], original["unknown_top"]);
    assert_eq!(
        merged["metadata"]["user_note"],
        original["metadata"]["user_note"]
    );
    assert_eq!(
        merged["metadata"]["mod_installer_version"],
        current["metadata"]["mod_installer_version"]
    );
    assert_eq!(
        merged["metadata"]["new_metadata"],
        current["metadata"]["new_metadata"]
    );
    assert_eq!(
        merged["settings"]["new_default"],
        current["settings"]["new_default"]
    );
    assert_eq!(
        merge_parser_documents(Some(&merged), &current, &imported).expect("repeat"),
        merged
    );
    let wrong_shape = document("choice_phrase = ['x']\nmetadata = 'bad'");
    assert!(merge_parser_documents(Some(&wrong_shape), &current, &imported).is_err());
}

#[test]
fn parser_sharing_version_reset_retains_exact_backup_and_verified_merge() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    std::fs::write(&path, ORIGINAL).expect("original");
    let initializer = Initializer {
        path: path.clone(),
        document: CURRENT.to_string(),
        calls: AtomicUsize::new(0),
        fail_after_write: false,
    };
    let prepared = prepare_parser_at(
        &path,
        &fixture.0.join("never-executed.exe"),
        &strings(&["Imported"]),
        &initializer,
    )
    .unwrap_or_else(|error| panic!("prepare: {error}"));
    assert_eq!(initializer.calls.load(Ordering::Relaxed), 1);
    assert_eq!(prepared.preparation.version.version, "15.0.1");
    assert_eq!(
        prepared.preparation.phrases,
        strings(&["Default", "Shared", " Custom 世界 ", "Imported"])
    );
    let backup = prepared.preparation.backup_path.as_ref().expect("backup");
    assert_eq!(backup.parent(), path.parent());
    assert_eq!(
        std::fs::read(backup).expect("backup bytes"),
        ORIGINAL.as_bytes()
    );
    assert_eq!(
        document(&std::fs::read_to_string(&path).expect("merged parser")),
        merge_parser_documents(
            Some(&document(ORIGINAL)),
            &document(CURRENT),
            &strings(&["Imported"])
        )
        .expect("expected merge")
    );
}

#[test]
fn parser_sharing_absent_parser_uses_explicit_initializer_path_without_backup() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    let initializer = Initializer {
        path: path.clone(),
        document: CURRENT.to_string(),
        calls: AtomicUsize::new(0),
        fail_after_write: false,
    };
    let prepared = prepare_parser_at(
        &path,
        &fixture.0.join("never-executed.exe"),
        &strings(&["Imported"]),
        &initializer,
    )
    .unwrap_or_else(|error| panic!("prepare: {error}"));
    assert!(prepared.preparation.backup_path.is_none());
    assert_eq!(
        capture_choice_phrases_at(&path).expect("capture"),
        strings(&["Default", "Shared", "Imported"])
    );
}

#[test]
fn parser_sharing_malformed_original_never_invokes_initializer() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    let original = b"choice_phrase = ['x', 2]\n";
    std::fs::write(&path, original).expect("original");
    let initializer = Initializer {
        path: path.clone(),
        document: CURRENT.to_string(),
        calls: AtomicUsize::new(0),
        fail_after_write: false,
    };
    assert!(
        prepare_parser_at(
            &path,
            &fixture.0.join("never-executed.exe"),
            &strings(&["Imported"]),
            &initializer
        )
        .is_err()
    );
    assert_eq!(initializer.calls.load(Ordering::Relaxed), 0);
    assert_eq!(std::fs::read(&path).expect("original retained"), original);
}

#[test]
fn parser_sharing_initializer_failure_restores_original_exact_bytes() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    std::fs::write(&path, ORIGINAL).expect("original");
    let initializer = Initializer {
        path: path.clone(),
        document: CURRENT.to_string(),
        calls: AtomicUsize::new(0),
        fail_after_write: true,
    };
    assert!(
        prepare_parser_at(
            &path,
            &fixture.0.join("never-executed.exe"),
            &strings(&["Imported"]),
            &initializer
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&path).expect("restored"), ORIGINAL.as_bytes());
    assert_eq!(rolling_inventory(&fixture), ["parser.toml"]);
}
#[test]
fn parser_sharing_faults_preserve_backup_and_restore_owned_bytes() {
    use super::{FileStage, prepare_parser_at_with};
    for stage in [
        FileStage::Backup,
        FileStage::Write,
        FileStage::Replace,
        FileStage::Verify,
    ] {
        let fixture = Fixture::new();
        let path = fixture.parser();
        std::fs::write(&path, ORIGINAL).expect("original");
        let initializer = Initializer {
            path: path.clone(),
            document: CURRENT.to_string(),
            calls: AtomicUsize::new(0),
            fail_after_write: false,
        };
        let fired = std::cell::Cell::new(false);
        let result = prepare_parser_at_with(
            &path,
            &fixture.0.join("never.exe"),
            &strings(&["Imported"]),
            &initializer,
            &|current| {
                if current == stage && !fired.replace(true) {
                    Err("injected fault".to_string())
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(
            initializer.calls.load(Ordering::Relaxed),
            usize::from(stage != FileStage::Backup)
        );
        assert_eq!(
            std::fs::read(&path).expect("original restored"),
            ORIGINAL.as_bytes()
        );
    }
}

#[test]
fn parser_sharing_verification_conflict_preserves_external_writer_and_backup() {
    use super::{FileStage, prepare_parser_at_with};
    let fixture = Fixture::new();
    let path = fixture.parser();
    std::fs::write(&path, ORIGINAL).expect("original");
    let initializer = Initializer {
        path: path.clone(),
        document: CURRENT.to_string(),
        calls: AtomicUsize::new(0),
        fail_after_write: false,
    };
    let external = "choice_phrase = ['External writer']\n";
    let result = prepare_parser_at_with(
        &path,
        &fixture.0.join("never.exe"),
        &strings(&["Imported"]),
        &initializer,
        &|stage| {
            if stage == FileStage::Verify {
                std::fs::write(&path, external).expect("external write");
            }
            Ok(())
        },
    );
    let error = result.err().expect("conflict");
    assert!(error.contains("restoration failed"));
    assert!(error.contains("recovery:"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("external preserved"),
        external
    );
    assert_eq!(
        std::fs::read(fixture.0.join("parser.toml.bio-recovery")).expect("retained recovery"),
        ORIGINAL.as_bytes()
    );
    assert_eq!(
        rolling_inventory(&fixture),
        ["parser.toml", "parser.toml.bio-recovery"]
    );
}

#[test]
fn parser_sharing_wrong_post_version_rolls_back_and_lease_blocks_capture_until_drop() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    std::fs::write(&path, ORIGINAL).expect("original");
    let mut initializer = Initializer {
        path: path.clone(),
        document: CURRENT.replace("15.0.1", "wrong"),
        calls: AtomicUsize::new(0),
        fail_after_write: false,
    };
    assert!(prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer).is_err());
    assert_eq!(std::fs::read(&path).expect("restored"), ORIGINAL.as_bytes());
    initializer.document = CURRENT.to_string();
    let prepared = prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer)
        .unwrap_or_else(|error| panic!("prepare: {error}"));
    super::with_parser_path_for_test(&path, || {
        assert!(super::capture_local_choice_phrases().is_err());
    });
    std::thread::spawn(move || drop(prepared))
        .join()
        .expect("owned Send lease");
    assert_eq!(
        capture_choice_phrases_at(&path).expect("read verified file"),
        strings(&["Default", "Shared", " Custom 世界 "])
    );
}

#[test]
fn parser_sharing_initializer_timeout_error_and_failed_restore_report_backup() {
    struct TimedOut(PathBuf);
    impl ParserInitializer for TimedOut {
        fn run_version(&self, _: &Path) -> Result<VersionOutput, String> {
            std::fs::write(&self.0, CURRENT).map_err(|error| error.to_string())?;
            Err("fixture initializer timed out after five seconds".to_string())
        }
    }
    let fixture = Fixture::new();
    let path = fixture.parser();
    std::fs::write(&path, ORIGINAL).expect("original");
    let error = prepare_parser_at(
        &path,
        &fixture.0.join("never.exe"),
        &[],
        &TimedOut(path.clone()),
    )
    .err()
    .expect("timeout");
    assert!(error.contains("timed out"));
    assert_eq!(std::fs::read(&path).expect("restored"), ORIGINAL.as_bytes());
    let error = super::prepare_parser_at_with(
        &path,
        &fixture.0.join("never.exe"),
        &[],
        &TimedOut(path.clone()),
        &|stage| {
            if stage == super::FileStage::Restore {
                Err("injected restore failure".to_string())
            } else {
                Ok(())
            }
        },
    )
    .err()
    .expect("restore failure");
    assert!(error.contains("timed out"));
    assert!(error.contains("restoration failed: injected restore failure"));
    assert!(error.contains("recovery:"));
    assert_eq!(
        std::fs::read_to_string(path).expect("initialized bytes retained"),
        CURRENT
    );
}

fn prepare_layout_fixture(
    original: Option<&str>,
    defaults_text: &str,
    imported: &[&str],
) -> String {
    let fixture = Fixture::new();
    let path = fixture.parser();
    if let Some(original) = original {
        std::fs::write(&path, original.as_bytes()).expect("original fixture");
    }
    let initializer = Initializer {
        path: path.clone(),
        document: defaults_text.to_string(),
        calls: AtomicUsize::new(0),
        fail_after_write: false,
    };
    let prepared = prepare_parser_at(
        &path,
        &fixture.0.join("never-executed.exe"),
        &strings(imported),
        &initializer,
    )
    .unwrap_or_else(|error| panic!("prepare layout: {error}"));
    let result = std::fs::read_to_string(&path).expect("prepared bytes");
    assert_eq!(
        document(&result),
        merge_parser_documents(
            original.map(document).as_ref(),
            &document(defaults_text),
            &strings(imported)
        )
        .expect("same merge semantics")
    );
    if let Some(original) = original.filter(|original| original.as_bytes() != result.as_bytes()) {
        assert_eq!(
            std::fs::read(prepared.preparation.backup_path.as_ref().expect("backup"))
                .expect("backup bytes"),
            original.as_bytes()
        );
    } else {
        assert!(prepared.preparation.backup_path.is_none());
    }
    result
}

const MULTILINE_LAYOUT: &str = "# user parser\nin_progress_words = [\n    \"creating\",\n    \"installing\",\n]\nchoice_phrase = [\n    \"do you want\", # existing recognition\n]\nchoice_words = [ 'choice', 'choose' ] # retain spacing\n\n[metadata] # version table\nmod_installer_version = '15.0.1'\nuser_note = 'keep # [ brackets ]'\n\n[custom] # unknown table\ntext = '''literal [ ] # ,\n世界'''\n";

#[test]
fn parser_sharing_layout_multiline_append_changes_only_phrase_array_lf_and_crlf() {
    for newline in ["\n", "\r\n"] {
        let original = MULTILINE_LAYOUT.replace('\n', newline);
        let result = prepare_layout_fixture(Some(&original), &original, &["1) yes"]);
        let needle = format!("    \"do you want\", # existing recognition{newline}]");
        let replacement = format!(
            "    \"do you want\", # existing recognition{newline}    \"1) yes\",{newline}]"
        );
        assert_eq!(result, original.replacen(&needle, &replacement, 1));
    }
}

#[test]
fn parser_sharing_layout_noop_restores_original_bytes_even_after_initializer_reformat() {
    let compact = toml::to_string(&document(MULTILINE_LAYOUT)).expect("fixture serialization");
    let result = prepare_layout_fixture(Some(MULTILINE_LAYOUT), &compact, &[]);
    assert_eq!(result, MULTILINE_LAYOUT);
}

#[test]
fn parser_sharing_layout_compact_array_does_not_reformat_unrelated_source() {
    let original = "unknown = { spacing = 2 } # keep\nchoice_phrase = [ 'do you want' ] # compact\n\n[metadata]\nmod_installer_version = '15.0.1'\n";
    let result = prepare_layout_fixture(Some(original), original, &["1) yes"]);
    let (before, after) = result.split_once("choice_phrase = ").expect("array key");
    assert_eq!(before, "unknown = { spacing = 2 } # keep\n");
    assert!(!after.split(']').next().expect("array body").contains('\n'));
    assert!(after.ends_with(" # compact\n\n[metadata]\nmod_installer_version = '15.0.1'\n"));
    assert!(after.contains("'do you want'"));
}

#[test]
fn parser_sharing_layout_first_run_uses_initializer_multiline_layout() {
    let result = prepare_layout_fixture(None, MULTILINE_LAYOUT, &["1) yes"]);
    assert_eq!(
        result,
        MULTILINE_LAYOUT.replace(
            "    \"do you want\", # existing recognition\n]",
            "    \"do you want\", # existing recognition\n    \"1) yes\",\n]"
        )
    );
}

#[test]
fn parser_sharing_layout_version_reset_preserves_old_source_and_new_defaults() {
    let original = MULTILINE_LAYOUT.replace("15.0.1", "14.0.0");
    let initialized = "choice_phrase = ['do you want']\nnew_default = 42\n[metadata]\nmod_installer_version = '15.0.1'\n";
    let result = prepare_layout_fixture(Some(&original), initialized, &[]);
    assert!(result.contains(
        "# user parser\nin_progress_words = [\n    \"creating\",\n    \"installing\",\n]"
    ));
    assert!(result.contains("# existing recognition"));
    assert!(result.contains("choice_words = [ 'choice', 'choose' ] # retain spacing"));
    assert!(result.contains("[metadata] # version table"));
    assert!(result.contains("[custom] # unknown table\ntext = '''literal [ ] # ,\n世界'''"));
    assert_eq!(document(&result)["new_default"].as_integer(), Some(42));
}

#[test]
fn parser_sharing_layout_reordered_union_retains_each_existing_phrase_comment() {
    let original = "choice_phrase = [\n    'Old', # old custom phrase\n    'Shared', # shared recognition\n]\n[metadata]\nmod_installer_version = '14.0.0'\n";
    let initialized =
        "choice_phrase = ['Shared', 'Default']\n[metadata]\nmod_installer_version = '15.0.1'\n";
    let result = prepare_layout_fixture(
        Some(original),
        initialized,
        &["a,b # [ ] 世界", "quote\" and slash\\"],
    );
    assert!(result.contains("'Old', # old custom phrase"));
    assert!(result.contains("'Shared', # shared recognition"));
}

#[test]
fn parser_sharing_layout_actual_default_snapshot_keeps_every_other_byte() {
    let original = r#"in_progress_words = [
    "creating",
    "installing",
]
useful_status_words = [
    "copied",
    "copying",
    "creating",
    "installed",
    "installing",
    "patched",
    "patching",
    "processed",
    "processing",
]
choice_words = [
    "choice",
    "choose",
    "enter",
    "select",
]
choice_phrase = [
    "[a]ccept, [r]etry, [c]ancel",
    "[y]es or [n]o",
    "[y]es, [n]o, [c]ancel",
    "answer [y]es or [n]o.",
    "do you want",
    "enter a new",
    "is this correct?",
    "leave blank",
    "please enter",
    "please select",
    "would you like",
]
completed_with_warnings = ["installed with warnings"]
failed_with_error = [
    "installation aborted",
    "not installed due to errors",
]
finished = [
    "process ended",
    "successfully installed      ",
]
eet_auto_fill = "Enter the full path to your BG:EE+SoD installation then press Enter."
bell_character = "\u0007"

[metadata]
mod_installer_version = "15.0.1"

[metadata.created]
secs_since_epoch = 1778156858
nanos_since_epoch = 639088600
"#;
    let expected = original.replacen(
        "    \"would you like\",\n]",
        "    \"would you like\",\n    \"1) yes\",\n]",
        1,
    );
    assert_eq!(
        prepare_layout_fixture(Some(original), original, &["1) yes"]),
        expected
    );
}

#[test]
fn parser_sharing_layout_opening_comment_append_lf_and_crlf() {
    for newline in ["\n", "\r\n"] {
        let original = "choice_phrase = [ # opening note 世界\n    'do you want', # retained phrase\n]\n[metadata]\nmod_installer_version = '15.0.1'\n".replace('\n', newline);
        let expected = original.replace(
            &format!(" # retained phrase{newline}]"),
            &format!(" # retained phrase{newline}    \"1) yes\",{newline}]"),
        );
        assert_eq!(
            prepare_layout_fixture(Some(&original), &original, &["1) yes"]),
            expected
        );
    }
}

#[test]
fn parser_sharing_layout_opening_comment_reorder_lf_and_crlf() {
    for newline in ["\n", "\r\n"] {
        let original = "choice_phrase = [ # array opening note\n    'Old', # old phrase\n    'Shared', # shared phrase\n]\n[metadata]\nmod_installer_version = '14.0.0'\n".replace('\n', newline);
        let initialized =
            "choice_phrase = ['Shared', 'Default']\n[metadata]\nmod_installer_version = '15.0.1'\n"
                .replace('\n', newline);
        let expected = "choice_phrase = [ # array opening note\n    'Shared', # shared phrase\n    \"Default\",\n    'Old', # old phrase\n]\n[metadata]\nmod_installer_version = \"15.0.1\"\n".replace('\n', newline);
        assert_eq!(
            prepare_layout_fixture(Some(&original), &initialized, &[]),
            expected
        );
    }
}

#[test]
fn parser_sharing_layout_opening_comment_empty_lf_and_crlf() {
    for newline in ["\n", "\r\n"] {
        let original = "choice_phrase = [ # empty array opening note\n]\n[metadata]\nmod_installer_version = '15.0.1'\n".replace('\n', newline);
        let expected = "choice_phrase = [ # empty array opening note\n\"1) yes\"\n]\n[metadata]\nmod_installer_version = '15.0.1'\n".replace('\n', newline);
        assert_eq!(
            prepare_layout_fixture(Some(&original), &original, &["1) yes"]),
            expected
        );
    }
}

fn rolling_inventory(fixture: &Fixture) -> Vec<String> {
    let mut names = std::fs::read_dir(&fixture.0)
        .expect("fixture inventory")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn rolling_initializer(path: &Path, text: &str) -> Initializer {
    Initializer {
        path: path.to_path_buf(),
        document: text.to_string(),
        calls: AtomicUsize::new(0),
        fail_after_write: false,
    }
}

#[test]
fn parser_sharing_rolling_repeated_changes_and_noop_keep_one_backup() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    let backup = fixture.0.join("parser.toml.bio-backup");
    let history = fixture.0.join("parser.toml.bio-backup-historical");
    std::fs::write(&history, b"historical sentinel").expect("history");
    std::fs::write(&path, ORIGINAL).expect("original");
    let initializer = rolling_initializer(&path, CURRENT);
    let first = prepare_parser_at(
        &path,
        &fixture.0.join("never.exe"),
        &strings(&["first"]),
        &initializer,
    )
    .unwrap_or_else(|error| panic!("first preparation: {error}"));
    assert_eq!(first.preparation.backup_path.as_ref(), Some(&backup));
    assert_eq!(
        std::fs::read(&backup).expect("fixed backup"),
        ORIGINAL.as_bytes()
    );
    let previous = std::fs::read(&path).expect("first output");
    drop(first);
    let second = prepare_parser_at(
        &path,
        &fixture.0.join("never.exe"),
        &strings(&["second"]),
        &initializer,
    )
    .unwrap_or_else(|error| panic!("second preparation: {error}"));
    assert_eq!(second.preparation.backup_path.as_ref(), Some(&backup));
    assert_eq!(std::fs::read(&backup).expect("refreshed backup"), previous);
    drop(second);
    let before = super::snapshot(&backup).expect("backup snapshot");
    let noop = prepare_parser_at(
        &path,
        &fixture.0.join("never.exe"),
        &strings(&["second"]),
        &initializer,
    )
    .unwrap_or_else(|error| panic!("no-op preparation: {error}"));
    assert!(noop.preparation.backup_path.is_none());
    assert!(super::snapshot(&backup).expect("unchanged snapshot") == before);
    assert_eq!(
        std::fs::read(&history).expect("history retained"),
        b"historical sentinel"
    );
    assert_eq!(
        rolling_inventory(&fixture),
        [
            "parser.toml",
            "parser.toml.bio-backup",
            "parser.toml.bio-backup-historical"
        ]
    );
}

#[test]
fn parser_sharing_rolling_noop_reformat_preserves_absent_or_existing_backup() {
    for sentinel in [None, Some(b"old rolling bytes".as_slice())] {
        for newline in ["\n", "\r\n"] {
            let fixture = Fixture::new();
            let path = fixture.parser();
            let backup = fixture.0.join("parser.toml.bio-backup");
            let original = MULTILINE_LAYOUT.replace('\n', newline);
            std::fs::write(&path, &original).expect("original");
            if let Some(bytes) = sentinel {
                std::fs::write(&backup, bytes).expect("old backup");
            }
            let before = super::snapshot(&backup).expect("backup snapshot");
            let reformatted = toml::to_string(&document(&original)).expect("reformat");
            let initializer = rolling_initializer(&path, &reformatted);
            let prepared =
                prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer)
                    .unwrap_or_else(|error| panic!("prepare: {error}"));
            assert!(prepared.preparation.backup_path.is_none());
            assert_eq!(
                std::fs::read(&path).expect("exact original"),
                original.as_bytes()
            );
            assert!(super::snapshot(&backup).expect("unchanged backup") == before);
            let expected = if sentinel.is_some() {
                vec!["parser.toml", "parser.toml.bio-backup"]
            } else {
                vec!["parser.toml"]
            };
            assert_eq!(rolling_inventory(&fixture), expected);
        }
    }
}

#[test]
fn parser_sharing_rolling_initializer_observes_original_recovery_and_permissions() {
    struct Observing<'a> {
        initializer: Initializer,
        recovery: &'a Path,
        permissions: std::fs::Permissions,
    }
    impl ParserInitializer for Observing<'_> {
        fn run_version(&self, executable: &Path) -> Result<VersionOutput, String> {
            assert_eq!(
                std::fs::read(self.recovery).expect("recovery before initializer"),
                ORIGINAL.as_bytes()
            );
            assert!(super::permissions_match(
                &self.permissions,
                &std::fs::metadata(self.recovery)
                    .expect("recovery metadata")
                    .permissions()
            ));
            self.initializer.run_version(executable)
        }
    }
    let fixture = Fixture::new();
    let path = fixture.parser();
    let recovery = fixture.0.join("parser.toml.bio-recovery");
    let backup = fixture.0.join("parser.toml.bio-backup");
    std::fs::write(&path, ORIGINAL).expect("original");
    let permissions = std::fs::metadata(&path).expect("metadata").permissions();
    let initializer = Observing {
        initializer: rolling_initializer(&path, CURRENT),
        recovery: &recovery,
        permissions: permissions.clone(),
    };
    let prepared = prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer)
        .unwrap_or_else(|error| panic!("prepare: {error}"));
    assert_eq!(prepared.preparation.backup_path.as_ref(), Some(&backup));
    assert_eq!(std::fs::read(&backup).expect("backup"), ORIGINAL.as_bytes());
    assert!(super::permissions_match(
        &permissions,
        &std::fs::metadata(&backup)
            .expect("backup metadata")
            .permissions()
    ));
    assert_eq!(
        rolling_inventory(&fixture),
        ["parser.toml", "parser.toml.bio-backup"]
    );
}

#[test]
fn parser_sharing_rolling_unresolved_recovery_blocks_existing_and_absent_parser() {
    for existing in [false, true] {
        for directory in [false, true] {
            let fixture = Fixture::new();
            let path = fixture.parser();
            let recovery = fixture.0.join("parser.toml.bio-recovery");
            if existing {
                std::fs::write(&path, ORIGINAL).expect("original");
            }
            if directory {
                std::fs::create_dir(&recovery).expect("recovery directory");
            } else {
                std::fs::write(&recovery, b"unresolved original").expect("recovery");
            }
            let before = rolling_inventory(&fixture);
            let initializer = rolling_initializer(&path, CURRENT);
            let error = prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer)
                .err()
                .expect("must block");
            assert!(error.contains(&recovery.display().to_string()));
            assert_eq!(initializer.calls.load(Ordering::Relaxed), 0);
            assert_eq!(rolling_inventory(&fixture), before);
            if existing {
                assert_eq!(
                    std::fs::read(&path).expect("original retained"),
                    ORIGINAL.as_bytes()
                );
            }
            if !directory {
                assert_eq!(
                    std::fs::read(&recovery).expect("recovery retained"),
                    b"unresolved original"
                );
            }
        }
    }
}

#[test]
fn parser_sharing_rolling_failed_restore_retains_recovery_and_blocks_next_attempt() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    let backup = fixture.0.join("parser.toml.bio-backup");
    let recovery = fixture.0.join("parser.toml.bio-recovery");
    std::fs::write(&path, ORIGINAL).expect("original");
    std::fs::write(&backup, b"previous backup").expect("old backup");
    let mut initializer = rolling_initializer(&path, CURRENT);
    initializer.fail_after_write = true;
    let error = super::prepare_parser_at_with(
        &path,
        &fixture.0.join("never.exe"),
        &[],
        &initializer,
        &|stage| {
            if stage == super::FileStage::Restore {
                Err("injected restore failure".to_string())
            } else {
                Ok(())
            }
        },
    )
    .err()
    .expect("failed restore");
    assert!(error.contains("restoration failed"));
    assert!(error.contains(&recovery.display().to_string()));
    assert_eq!(
        std::fs::read(&recovery).expect("recovery retained"),
        ORIGINAL.as_bytes()
    );
    assert_eq!(
        std::fs::read(&backup).expect("old backup retained"),
        b"previous backup"
    );
    assert_eq!(
        std::fs::read(&path).expect("initializer bytes retained"),
        CURRENT.as_bytes()
    );
    let calls = initializer.calls.load(Ordering::Relaxed);
    assert!(prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer).is_err());
    assert_eq!(initializer.calls.load(Ordering::Relaxed), calls);
    assert_eq!(
        rolling_inventory(&fixture),
        [
            "parser.toml",
            "parser.toml.bio-backup",
            "parser.toml.bio-recovery"
        ]
    );
}

#[test]
fn parser_sharing_rolling_fault_stages_restore_and_leave_previous_backup_untouched() {
    use super::FileStage;
    for stage in [
        FileStage::Backup,
        FileStage::RecoveryWrite,
        FileStage::RecoverySync,
        FileStage::Write,
        FileStage::Replace,
        FileStage::Verify,
        FileStage::Promote,
    ] {
        let fixture = Fixture::new();
        let path = fixture.parser();
        let backup = fixture.0.join("parser.toml.bio-backup");
        std::fs::write(&path, ORIGINAL).expect("original");
        std::fs::write(&backup, b"previous backup").expect("previous backup");
        let before = super::snapshot(&backup).expect("old snapshot");
        let initializer = rolling_initializer(&path, CURRENT);
        let fired = std::cell::Cell::new(false);
        let result = super::prepare_parser_at_with(
            &path,
            &fixture.0.join("never.exe"),
            &strings(&["Imported"]),
            &initializer,
            &|current| {
                if current == stage && !fired.replace(true) {
                    Err("injected lifecycle fault".to_string())
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert!(fired.get());
        assert_eq!(
            initializer.calls.load(Ordering::Relaxed),
            usize::from(
                ![
                    FileStage::Backup,
                    FileStage::RecoveryWrite,
                    FileStage::RecoverySync
                ]
                .contains(&stage)
            )
        );
        assert_eq!(
            std::fs::read(&path).expect("restored original"),
            ORIGINAL.as_bytes()
        );
        assert!(super::snapshot(&backup).expect("preserved backup") == before);
        assert_eq!(
            rolling_inventory(&fixture),
            ["parser.toml", "parser.toml.bio-backup"]
        );
    }
}

#[test]
fn parser_sharing_rolling_promotion_conflicts_preserve_external_artifacts() {
    for modified in [
        "parser.toml",
        "parser.toml.bio-backup",
        "parser.toml.bio-recovery",
    ] {
        let fixture = Fixture::new();
        let path = fixture.parser();
        let backup = fixture.0.join("parser.toml.bio-backup");
        let recovery = fixture.0.join("parser.toml.bio-recovery");
        std::fs::write(&path, ORIGINAL).expect("original");
        std::fs::write(&backup, b"previous backup").expect("previous backup");
        let initializer = rolling_initializer(&path, CURRENT);
        let error = super::prepare_parser_at_with(
            &path,
            &fixture.0.join("never.exe"),
            &strings(&["Imported"]),
            &initializer,
            &|stage| {
                if stage == super::FileStage::Promote {
                    std::fs::write(fixture.0.join(modified), b"external writer")
                        .expect("external modification");
                }
                Ok(())
            },
        )
        .err()
        .expect("promotion conflict");
        assert_eq!(
            std::fs::read(fixture.0.join(modified)).expect("external retained"),
            b"external writer"
        );
        if modified == "parser.toml" {
            assert!(error.contains("restoration failed"));
            assert_eq!(
                std::fs::read(&recovery).expect("original recovery"),
                ORIGINAL.as_bytes()
            );
        } else {
            assert_eq!(
                std::fs::read(&path).expect("original restored"),
                ORIGINAL.as_bytes()
            );
        }
        if modified == "parser.toml.bio-backup" {
            assert!(!recovery.exists());
        } else {
            assert_eq!(
                std::fs::read(&backup).expect("prior backup"),
                b"previous backup"
            );
        }
        if modified != "parser.toml.bio-backup" {
            assert!(error.contains(&recovery.display().to_string()));
            let calls = initializer.calls.load(Ordering::Relaxed);
            assert!(
                prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer).is_err()
            );
            assert_eq!(initializer.calls.load(Ordering::Relaxed), calls);
        }
    }
}

#[test]
fn parser_sharing_rolling_cleanup_failure_blocks_next_preparation() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    let backup = fixture.0.join("parser.toml.bio-backup");
    let recovery = fixture.0.join("parser.toml.bio-recovery");
    std::fs::write(&path, CURRENT).expect("original");
    std::fs::write(&backup, b"previous backup").expect("old backup");
    let initializer = rolling_initializer(&path, CURRENT);
    let error = super::prepare_parser_at_with(
        &path,
        &fixture.0.join("never.exe"),
        &[],
        &initializer,
        &|stage| {
            if stage == super::FileStage::Cleanup {
                Err("injected cleanup failure".to_string())
            } else {
                Ok(())
            }
        },
    )
    .err()
    .expect("cleanup failure");
    assert!(error.contains("injected cleanup failure"));
    assert!(error.contains(&recovery.display().to_string()));
    assert_eq!(std::fs::read(&path).expect("original"), CURRENT.as_bytes());
    assert_eq!(
        std::fs::read(&recovery).expect("original recovery"),
        CURRENT.as_bytes()
    );
    assert_eq!(
        std::fs::read(&backup).expect("old backup"),
        b"previous backup"
    );
    let calls = initializer.calls.load(Ordering::Relaxed);
    assert!(prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer).is_err());
    assert_eq!(initializer.calls.load(Ordering::Relaxed), calls);
    assert_eq!(
        rolling_inventory(&fixture),
        [
            "parser.toml",
            "parser.toml.bio-backup",
            "parser.toml.bio-recovery"
        ]
    );
}

#[test]
fn parser_sharing_rolling_absent_original_preserves_previous_backup() {
    let fixture = Fixture::new();
    let path = fixture.parser();
    let backup = fixture.0.join("parser.toml.bio-backup");
    std::fs::write(&backup, b"previous backup").expect("old backup");
    let before = super::snapshot(&backup).expect("old snapshot");
    let initializer = rolling_initializer(&path, CURRENT);
    let prepared = prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer)
        .unwrap_or_else(|error| panic!("first preparation: {error}"));
    assert!(prepared.preparation.backup_path.is_none());
    assert!(super::snapshot(&backup).expect("preserved backup") == before);
    assert_eq!(
        rolling_inventory(&fixture),
        ["parser.toml", "parser.toml.bio-backup"]
    );
}

#[test]
fn parser_sharing_rolling_unsafe_backup_blocks_before_initializer() {
    for oversized in [false, true] {
        let fixture = Fixture::new();
        let path = fixture.parser();
        let backup = fixture.0.join("parser.toml.bio-backup");
        std::fs::write(&path, ORIGINAL).expect("original");
        if oversized {
            std::fs::write(
                &backup,
                vec![b'x'; usize::try_from(super::MAX_BYTES).expect("limit") + 1],
            )
            .expect("oversized backup");
        } else {
            std::fs::create_dir(&backup).expect("backup directory");
        }
        let initializer = rolling_initializer(&path, CURRENT);
        assert!(prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer).is_err());
        assert_eq!(initializer.calls.load(Ordering::Relaxed), 0);
        assert_eq!(std::fs::read(&path).expect("original"), ORIGINAL.as_bytes());
        assert_eq!(
            rolling_inventory(&fixture),
            ["parser.toml", "parser.toml.bio-backup"]
        );
        if oversized {
            assert_eq!(
                std::fs::metadata(&backup)
                    .expect("preserved oversized")
                    .len(),
                super::MAX_BYTES + 1
            );
        }
    }
}

#[test]
fn parser_sharing_rolling_unknown_observation_retains_recovery_and_blocks_retry() {
    struct Unobservable(PathBuf);
    impl ParserInitializer for Unobservable {
        fn run_version(&self, _: &Path) -> Result<VersionOutput, String> {
            std::fs::remove_file(&self.0).expect("remove parser");
            std::fs::create_dir(&self.0).expect("external directory");
            Err("injected initializer failed with unknown observation".to_string())
        }
    }
    let fixture = Fixture::new();
    let path = fixture.parser();
    let recovery = fixture.0.join("parser.toml.bio-recovery");
    std::fs::write(&path, ORIGINAL).expect("original");
    let error = prepare_parser_at(
        &path,
        &fixture.0.join("never.exe"),
        &[],
        &Unobservable(path.clone()),
    )
    .err()
    .expect("unknown ownership");
    assert!(error.contains("restoration failed"));
    assert!(error.contains(&recovery.display().to_string()));
    assert_eq!(
        std::fs::read(&recovery).expect("original recovery"),
        ORIGINAL.as_bytes()
    );
    assert!(path.is_dir());
    let initializer = rolling_initializer(&path, CURRENT);
    assert!(prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer).is_err());
    assert_eq!(initializer.calls.load(Ordering::Relaxed), 0);
    assert_eq!(
        rolling_inventory(&fixture),
        ["parser.toml", "parser.toml.bio-recovery"]
    );
}

#[test]
fn parser_sharing_rolling_initializer_and_version_failures_preserve_old_backup() {
    for failure in ["initializer", "version", "merge"] {
        let fixture = Fixture::new();
        let path = fixture.parser();
        let backup = fixture.0.join("parser.toml.bio-backup");
        let history = fixture.0.join("parser.toml.bio-backup-historical");
        std::fs::write(&path, ORIGINAL).expect("original");
        std::fs::write(&backup, b"previous backup").expect("previous backup");
        std::fs::write(&history, b"historical sentinel").expect("history");
        let before = super::snapshot(&backup).expect("backup snapshot");
        let mut initializer = rolling_initializer(&path, CURRENT);
        match failure {
            "initializer" => initializer.fail_after_write = true,
            "version" => initializer.document = CURRENT.replace("15.0.1", "wrong"),
            _ => initializer.document = CURRENT.replace("answer = 'new'", "nested = 42"),
        }
        let error = prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer)
            .err()
            .expect("preparation must fail");
        assert!(!error.contains("restoration failed"));
        assert_eq!(
            std::fs::read(&path).expect("restored original"),
            ORIGINAL.as_bytes()
        );
        assert!(super::snapshot(&backup).expect("preserved backup") == before);
        assert_eq!(
            std::fs::read(&history).expect("history retained"),
            b"historical sentinel"
        );
        assert_eq!(
            rolling_inventory(&fixture),
            [
                "parser.toml",
                "parser.toml.bio-backup",
                "parser.toml.bio-backup-historical"
            ]
        );
    }
}

#[test]
fn parser_sharing_rolling_suffix_uses_complete_supplied_filename() {
    let fixture = Fixture::new();
    let path = fixture.0.join("custom.parser.世界.toml");
    let backup = fixture.0.join("custom.parser.世界.toml.bio-backup");
    std::fs::write(&path, ORIGINAL).expect("original");
    let initializer = rolling_initializer(&path, CURRENT);
    let prepared = prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer)
        .unwrap_or_else(|error| panic!("custom filename: {error}"));
    assert_eq!(prepared.preparation.backup_path.as_ref(), Some(&backup));
    assert_eq!(std::fs::read(&backup).expect("backup"), ORIGINAL.as_bytes());
    assert_eq!(
        rolling_inventory(&fixture),
        [
            "custom.parser.世界.toml",
            "custom.parser.世界.toml.bio-backup"
        ]
    );
}

#[cfg(windows)]
#[test]
fn parser_sharing_rolling_readonly_noop_and_replacement_keep_permissions() {
    struct WritableInitializer(Initializer, std::fs::Permissions);
    impl ParserInitializer for WritableInitializer {
        fn run_version(&self, executable: &Path) -> Result<VersionOutput, String> {
            std::fs::set_permissions(&self.0.path, self.1.clone())
                .expect("initializer writable parser");
            self.0.run_version(executable)
        }
    }
    struct Cleanup<'a>(&'a Fixture, std::fs::Permissions);
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            for entry in std::fs::read_dir(&self.0.0)
                .expect("fixture cleanup")
                .flatten()
            {
                let _ = std::fs::set_permissions(entry.path(), self.1.clone());
            }
        }
    }
    let fixture = Fixture::new();
    let path = fixture.parser();
    let backup = fixture.0.join("parser.toml.bio-backup");
    std::fs::write(&path, CURRENT).expect("original");
    std::fs::write(&backup, b"previous backup").expect("old backup");
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    let writable_permissions = permissions.clone();
    let _cleanup = Cleanup(&fixture, writable_permissions.clone());
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions.clone()).expect("readonly original");
    std::fs::set_permissions(&backup, permissions).expect("readonly backup");
    let before = super::snapshot(&backup).expect("old backup snapshot");
    let initializer =
        WritableInitializer(rolling_initializer(&path, CURRENT), writable_permissions);
    let noop = prepare_parser_at(&path, &fixture.0.join("never.exe"), &[], &initializer)
        .unwrap_or_else(|error| panic!("readonly no-op: {error}"));
    assert!(noop.preparation.backup_path.is_none());
    assert!(super::snapshot(&backup).expect("unchanged readonly backup") == before);
    assert!(
        std::fs::metadata(&path)
            .expect("restored readonly")
            .permissions()
            .readonly()
    );
    assert_eq!(
        rolling_inventory(&fixture),
        ["parser.toml", "parser.toml.bio-backup"]
    );
    drop(noop);
    for phrase in ["first", "second"] {
        let previous = std::fs::read(&path).expect("previous parser");
        let old_backup = super::snapshot(&backup).expect("previous backup snapshot");
        match prepare_parser_at(
            &path,
            &fixture.0.join("never.exe"),
            &strings(&[phrase]),
            &initializer,
        ) {
            Ok(prepared) => {
                assert_eq!(prepared.preparation.backup_path.as_ref(), Some(&backup));
                assert_eq!(std::fs::read(&backup).expect("new backup"), previous);
                assert!(
                    std::fs::metadata(&backup)
                        .expect("new readonly backup")
                        .permissions()
                        .readonly()
                );
            }
            Err(error) => {
                assert!(!error.contains("restoration failed"), "{error}");
                assert_eq!(std::fs::read(&path).expect("restored parser"), previous);
                assert!(super::snapshot(&backup).expect("old backup preserved") == old_backup);
            }
        }
        assert!(
            std::fs::metadata(&path)
                .expect("readonly parser")
                .permissions()
                .readonly()
        );
        assert_eq!(
            rolling_inventory(&fixture),
            ["parser.toml", "parser.toml.bio-backup"]
        );
    }
}
