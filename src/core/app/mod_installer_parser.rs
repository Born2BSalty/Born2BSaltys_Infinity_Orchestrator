use std::collections::HashSet;
use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[path = "mod_installer_parser/layout.rs"]
mod layout;

#[path = "mod_installer_parser/process.rs"]
mod process;

const MAX_BYTES: u64 = 2 * 1024 * 1024;
static RESERVED: AtomicBool = AtomicBool::new(false);
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

pub(crate) struct VersionOutput {
    pub(crate) display: String,
    pub(crate) version: String,
}

pub(crate) trait ParserInitializer: Send + Sync {
    fn run_version(&self, executable: &Path) -> Result<VersionOutput, String>;
}

pub(crate) struct ParserPreparation {
    pub(crate) phrases: Vec<String>,
    pub(crate) version: VersionOutput,
    pub(crate) backup_path: Option<PathBuf>,
}

pub(crate) struct ParserLease {
    reservation: &'static AtomicBool,
}

impl Drop for ParserLease {
    fn drop(&mut self) {
        self.reservation.store(false, Ordering::Release);
    }
}

pub(crate) struct PreparedParser {
    pub(crate) preparation: ParserPreparation,
    pub(crate) lease: ParserLease,
}

fn acquire(wait: bool) -> Result<ParserLease, String> {
    let start = Instant::now();
    loop {
        if RESERVED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(ParserLease {
                reservation: &RESERVED,
            });
        }
        if !wait || start.elapsed() >= Duration::from_secs(5) {
            return Err("Installer parser is busy; try again shortly".to_string());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn validate_phrases(phrases: &[String]) -> Result<(), String> {
    let unique: HashSet<&str> = phrases.iter().map(String::as_str).collect();
    if unique.len() > 4096 {
        return Err("Parser phrases exceed 4096 unique entries".to_string());
    }
    let mut bytes = 0usize;
    for phrase in phrases {
        if phrase.trim().is_empty() || phrase.len() > 8192 {
            return Err("Parser phrases must be nonblank and at most 8192 UTF-8 bytes".to_string());
        }
        bytes = bytes.saturating_add(phrase.len());
    }
    if bytes > 512 * 1024 {
        return Err("Parser phrases exceed 512 KiB".to_string());
    }
    Ok(())
}

pub(crate) fn union_phrases(groups: &[&[String]]) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for group in groups {
        validate_phrases(group)?;
        for phrase in *group {
            if seen.insert(phrase.as_str()) {
                result.push(phrase.clone());
            }
        }
    }
    validate_phrases(&result)?;
    Ok(result)
}

fn read_bytes(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot read parser: {error}")),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Parser exceeds 2 MiB".to_string());
    }
    Ok(Some(bytes))
}

fn parse(bytes: &[u8]) -> Result<toml::Value, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Parser is not UTF-8".to_string())?;
    toml::from_str(text).map_err(|_| "Parser contains malformed TOML".to_string())
}

fn phrases(document: &toml::Value) -> Result<Vec<String>, String> {
    let array = document
        .get("choice_phrase")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "Parser choice_phrase must be an array of strings".to_string())?;
    let result = array
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "Parser choice_phrase must contain only strings".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_phrases(&result)?;
    Ok(result)
}

fn validate_document(document: &toml::Value) -> Result<(), String> {
    document
        .as_table()
        .ok_or_else(|| "Parser root must be a table".to_string())?;
    phrases(document)?;
    if let Some(metadata) = document.get("metadata") {
        let table = metadata
            .as_table()
            .ok_or_else(|| "Parser metadata must be a table".to_string())?;
        if table
            .get("mod_installer_version")
            .is_some_and(|value| !value.is_str())
        {
            return Err("Parser version metadata must be a string".to_string());
        }
    }
    Ok(())
}

pub(crate) fn capture_choice_phrases_at(path: &Path) -> Result<Vec<String>, String> {
    read_bytes(path)?.map_or_else(|| Ok(Vec::new()), |bytes| phrases(&parse(&bytes)?))
}

pub(crate) fn resolve_parser_path() -> Result<PathBuf, String> {
    confy::get_configuration_file_path("mod_installer", "parser")
        .map_err(|error| format!("Cannot resolve parser path: {error}"))
}

#[cfg(test)]
thread_local! {
    static TEST_CAPTURE_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
struct TestCaptureGuard(Option<PathBuf>);

#[cfg(test)]
impl Drop for TestCaptureGuard {
    fn drop(&mut self) {
        TEST_CAPTURE_PATH.with(|path| {
            path.replace(self.0.take());
        });
    }
}

#[cfg(test)]
pub(crate) fn with_parser_path_for_test<T>(path: &Path, action: impl FnOnce() -> T) -> T {
    let previous = TEST_CAPTURE_PATH.with(|value| value.replace(Some(path.to_path_buf())));
    let _restore = TestCaptureGuard(previous);
    action()
}

pub(crate) fn capture_local_choice_phrases() -> Result<Vec<String>, String> {
    #[cfg(test)]
    if TEST_CAPTURE_PATH.with(|path| path.borrow().is_none()) {
        return Ok(Vec::new());
    }
    let _lease = acquire(false)?;
    #[cfg(test)]
    {
        TEST_CAPTURE_PATH.with(|path| {
            path.borrow()
                .as_deref()
                .map_or_else(|| Ok(Vec::new()), capture_choice_phrases_at)
        })
    }
    #[cfg(not(test))]
    {
        capture_choice_phrases_at(&resolve_parser_path()?)
    }
}

fn overlay(base: &mut toml::Value, original: &toml::Value) -> Result<(), String> {
    if let (Some(base), Some(original)) = (base.as_table_mut(), original.as_table()) {
        for (key, value) in original {
            if let Some(current) = base.get_mut(key) {
                if current.is_table() != value.is_table() {
                    return Err(format!(
                        "Parser setting has incompatible table shape: {key}"
                    ));
                }
                overlay(current, value)?;
            } else {
                base.insert(key.clone(), value.clone());
            }
        }
    } else {
        *base = original.clone();
    }
    Ok(())
}

pub(crate) fn merge_parser_documents(
    original: Option<&toml::Value>,
    initialized: &toml::Value,
    imported: &[String],
) -> Result<toml::Value, String> {
    validate_document(initialized)?;
    validate_phrases(imported)?;
    let current_version = initialized
        .get("metadata")
        .and_then(|value| value.get("mod_installer_version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "Initialized parser has no version metadata".to_string())?
        .to_string();
    let initial_phrases = phrases(initialized)?;
    let mut merged = initialized.clone();
    let original_phrases = if let Some(original) = original {
        validate_document(original)?;
        overlay(&mut merged, original)?;
        phrases(original)?
    } else {
        Vec::new()
    };
    let combined = union_phrases(&[&initial_phrases, &original_phrases, imported])?;
    merged["choice_phrase"] =
        toml::Value::Array(combined.into_iter().map(toml::Value::String).collect());
    merged["metadata"]["mod_installer_version"] = toml::Value::String(current_version);
    validate_document(&merged)?;
    Ok(merged)
}

fn sibling(path: &Path, kind: &str) -> PathBuf {
    let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |value| value.as_nanos());
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bio-{kind}-{}-{stamp}-{id}", std::process::id()));
    path.with_file_name(name)
}

fn fixed_sibling(path: &Path, kind: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bio-{kind}"));
    path.with_file_name(name)
}

fn exclusive_write(
    path: &Path,
    bytes: &[u8],
    permissions: Option<&Permissions>,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    if let Some(permissions) = permissions {
        file.set_permissions(permissions.clone())
            .map_err(|error| error.to_string())?;
    }
    file.sync_all().map_err(|error| error.to_string())
}

#[derive(Clone, PartialEq, Eq)]
struct FileStamp {
    length: u64,
    created: Option<std::time::SystemTime>,
    modified: Option<std::time::SystemTime>,
    readonly: bool,
}

fn stamp(path: &Path) -> Result<Option<FileStamp>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Parser path must be a regular file".to_string());
    }
    Ok(Some(FileStamp {
        length: metadata.len(),
        created: metadata.created().ok(),
        modified: metadata.modified().ok(),
        readonly: metadata.permissions().readonly(),
    }))
}

#[derive(Clone, PartialEq, Eq)]
struct Snapshot {
    bytes: Option<Vec<u8>>,
    stamp: Option<FileStamp>,
}

fn snapshot(path: &Path) -> Result<Snapshot, String> {
    let before = stamp(path)?;
    let bytes = read_bytes(path)?;
    let after = stamp(path)?;
    if before != after || bytes.is_some() != after.is_some() {
        return Err("Concurrent parser change detected while reading".to_string());
    }
    Ok(Snapshot {
        bytes,
        stamp: after,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileStage {
    Read,
    Backup,
    RecoveryWrite,
    RecoverySync,
    Promote,
    Cleanup,
    Write,
    Replace,
    Verify,
    Restore,
}

type OperationHook<'a> = &'a dyn Fn(FileStage) -> Result<(), String>;

fn replace_owned(source: &Path, destination: &Path, expected: &Snapshot) -> Result<(), String> {
    if snapshot(destination)? != *expected {
        return Err("Concurrent destination change preserved before replacement".to_string());
    }
    #[cfg(windows)]
    if expected.stamp.as_ref().is_some_and(|stamp| stamp.readonly) {
        use std::os::windows::{ffi::OsStrExt, fs::MetadataExt};
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY, SetFileAttributesW,
        };
        let metadata = fs::metadata(destination).map_err(|error| error.to_string())?;
        let permissions = metadata.permissions();
        if snapshot(destination)? != *expected {
            return Err(
                "Concurrent destination change preserved before permission repair".to_string(),
            );
        }
        let attributes = metadata.file_attributes() & !FILE_ATTRIBUTE_READONLY;
        let attributes = if attributes == 0 {
            FILE_ATTRIBUTE_NORMAL
        } else {
            attributes
        };
        let destination_wide: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        if unsafe { SetFileAttributesW(destination_wide.as_ptr(), attributes) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut writable = expected.clone();
        if let Some(stamp) = writable.stamp.as_mut() {
            stamp.readonly = false;
        }
        if snapshot(destination)? != writable {
            return Err("Destination ownership changed during permission repair".to_string());
        }
        if let Err(error) = process::replace(source, destination) {
            let restored = (|| {
                if snapshot(destination)? != writable {
                    return Err(
                        "Concurrent destination change preserved after failed replacement"
                            .to_string(),
                    );
                }
                fs::set_permissions(destination, permissions).map_err(|error| error.to_string())?;
                if snapshot(destination)? != *expected {
                    return Err(
                        "Destination permission restoration could not be verified".to_string()
                    );
                }
                Ok(())
            })();
            return match restored {
                Ok(()) => Err(error),
                Err(restore_error) => Err(format!(
                    "{error}; destination permission restoration failed: {restore_error}"
                )),
            };
        }
        return Ok(());
    }
    process::replace(source, destination)
}

fn atomic_write(
    path: &Path,
    bytes: &[u8],
    permissions: Option<&Permissions>,
    expected: &Snapshot,
    hook: OperationHook<'_>,
) -> Result<Snapshot, String> {
    let temporary = sibling(path, "temp");
    let result = (|| {
        hook(FileStage::Write)?;
        exclusive_write(&temporary, bytes, permissions)?;
        let replacement = snapshot(&temporary)?;
        if snapshot(path)? != *expected {
            return Err("Concurrent parser change preserved before replacement".to_string());
        }
        hook(FileStage::Replace)?;
        replace_owned(&temporary, path, expected)?;
        Ok(replacement)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn permissions_match(original: &Permissions, current: &Permissions) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        original.mode() == current.mode()
    }
    #[cfg(not(unix))]
    {
        original.readonly() == current.readonly()
    }
}

struct Transaction<'a> {
    path: &'a Path,
    original: Snapshot,
    original_document: Option<toml::Value>,
    permissions: Option<Permissions>,
    backup_path: PathBuf,
    backup_original: Option<Snapshot>,
    recovery_path: PathBuf,
    recovery_owned: Option<Snapshot>,
    owned: Option<Snapshot>,
    hook: OperationHook<'a>,
}

impl<'a> Transaction<'a> {
    fn begin(path: &'a Path, hook: OperationHook<'a>) -> Result<Self, String> {
        hook(FileStage::Read)?;
        let original = snapshot(path)?;
        let original_document = original.bytes.as_deref().map(parse).transpose()?;
        if let Some(document) = original_document.as_ref() {
            validate_document(document)?;
        }
        let permissions = original
            .bytes
            .as_ref()
            .map(|_| {
                fs::metadata(path)
                    .map(|value| value.permissions())
                    .map_err(|error| error.to_string())
            })
            .transpose()?;
        let recovery_path = fixed_sibling(path, "recovery");
        match fs::symlink_metadata(&recovery_path) {
            Ok(_) => {
                return Err(format!(
                    "Unresolved parser recovery requires separate handling: {}",
                    recovery_path.display()
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Cannot inspect parser recovery {}: {error}",
                    recovery_path.display()
                ));
            }
        }
        let backup_path = fixed_sibling(path, "backup");
        let backup_original = original
            .bytes
            .as_ref()
            .map(|_| snapshot(&backup_path))
            .transpose()?;
        let mut transaction = Self {
            path,
            original,
            original_document,
            permissions,
            backup_path,
            backup_original,
            recovery_path,
            recovery_owned: None,
            owned: None,
            hook,
        };
        let result = transaction.stage_recovery().and_then(|()| {
            if snapshot(path)? != transaction.original {
                return Err("Concurrent parser change before initialization".to_string());
            }
            Ok(())
        });
        if let Err(error) = result {
            let cleanup = if snapshot(path).as_ref().ok() == Some(&transaction.original) {
                transaction.cleanup_recovery()
            } else {
                Err("Original parser ownership changed before initialization".to_string())
            };
            return Err(transaction.recovery_error(&error, cleanup));
        }
        Ok(transaction)
    }

    fn stage_recovery(&mut self) -> Result<(), String> {
        let Some(bytes) = self.original.bytes.as_ref() else {
            return Ok(());
        };
        (self.hook)(FileStage::Backup)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.recovery_path)
            .map_err(|error| error.to_string())?;
        self.recovery_owned = Some(snapshot(&self.recovery_path)?);
        (self.hook)(FileStage::RecoveryWrite)?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        if let Some(permissions) = self.permissions.as_ref() {
            file.set_permissions(permissions.clone())
                .map_err(|error| error.to_string())?;
        }
        self.recovery_owned = Some(snapshot(&self.recovery_path)?);
        (self.hook)(FileStage::RecoverySync)?;
        file.sync_all().map_err(|error| error.to_string())?;
        if snapshot(&self.recovery_path)?.bytes.as_ref() != Some(bytes)
            || self.recovery_owned.as_ref() != Some(&snapshot(&self.recovery_path)?)
        {
            return Err("Parser recovery changed while staging".to_string());
        }
        let recovery_permissions = fs::metadata(&self.recovery_path)
            .map_err(|error| error.to_string())?
            .permissions();
        if self
            .permissions
            .as_ref()
            .is_some_and(|original| !permissions_match(original, &recovery_permissions))
        {
            return Err("Parser recovery permissions changed while staging".to_string());
        }
        Ok(())
    }

    fn cleanup_recovery(&self) -> Result<(), String> {
        let Some(owned) = self.recovery_owned.as_ref() else {
            return match fs::symlink_metadata(&self.recovery_path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Ok(_) => Err("Unowned recovery entry preserved".to_string()),
                Err(error) => Err(error.to_string()),
            };
        };
        (self.hook)(FileStage::Cleanup)?;
        if snapshot(&self.recovery_path)? != *owned {
            return Err("Concurrent recovery change preserved before cleanup".to_string());
        }
        fs::remove_file(&self.recovery_path).map_err(|error| error.to_string())
    }

    fn recovery_error(&self, error: &str, cleanup: Result<(), String>) -> String {
        let error: String = error.chars().take(1024).collect();
        match cleanup {
            Ok(()) => error,
            Err(cleanup_error) => format!(
                "{error}; recovery retained at {}: {cleanup_error}",
                self.recovery_path.display()
            )
            .chars()
            .take(4096)
            .collect(),
        }
    }

    fn finish(
        &mut self,
        version: VersionOutput,
        imported: &[String],
    ) -> Result<ParserPreparation, String> {
        let owned = self
            .owned
            .as_ref()
            .ok_or_else(|| "Parser state could not be observed after initialization".to_string())?;
        let initialized = parse(
            owned
                .bytes
                .as_deref()
                .ok_or_else(|| "Initializer did not create parser".to_string())?,
        )?;
        let stored_version = initialized
            .get("metadata")
            .and_then(|value| value.get("mod_installer_version"))
            .and_then(toml::Value::as_str);
        if stored_version != Some(version.version.as_str()) {
            return Err("Initialized parser version does not match executable".to_string());
        }
        let merged =
            merge_parser_documents(self.original_document.as_ref(), &initialized, imported)?;
        if snapshot(self.path)? != *owned {
            return Err("Concurrent parser change detected".to_string());
        }
        let current_permissions = fs::metadata(self.path)
            .map_err(|error| error.to_string())?
            .permissions();
        let permissions_changed = self
            .permissions
            .as_ref()
            .is_some_and(|original| !permissions_match(original, &current_permissions));
        let initialized_bytes = owned
            .bytes
            .as_deref()
            .ok_or_else(|| "Initializer did not create parser".to_string())?;
        let base = self.original.bytes.as_deref().unwrap_or(initialized_bytes);
        let bytes = layout::render(base, &merged)?;
        if bytes != initialized_bytes || permissions_changed {
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Merged parser exceeds 2 MiB".to_string());
            }
            self.owned = Some(atomic_write(
                self.path,
                &bytes,
                self.permissions.as_ref(),
                owned,
                self.hook,
            )?);
        }
        (self.hook)(FileStage::Verify)?;
        let verified = snapshot(self.path)?;
        if self.owned.as_ref() != Some(&verified) {
            return Err("Parser verification failed or concurrent change detected".to_string());
        }
        let verified_document = parse(
            verified
                .bytes
                .as_deref()
                .ok_or_else(|| "Parser disappeared during verification".to_string())?,
        )?;
        if verified_document != merged {
            return Err("Parser semantic verification failed".to_string());
        }
        let changed = self.original.bytes.is_some() && verified.bytes != self.original.bytes;
        let preparation = ParserPreparation {
            phrases: phrases(&verified_document)?,
            version,
            backup_path: changed.then(|| self.backup_path.clone()),
        };
        self.finalize_backup(&verified, changed)?;
        Ok(preparation)
    }

    fn finalize_backup(&self, verified: &Snapshot, changed: bool) -> Result<(), String> {
        if changed {
            (self.hook)(FileStage::Promote)?;
            if snapshot(self.path)? != *verified {
                return Err("Concurrent parser change preserved before promotion".to_string());
            }
            if self.recovery_owned.as_ref() != Some(&snapshot(&self.recovery_path)?) {
                return Err("Concurrent recovery change preserved before promotion".to_string());
            }
            let backup_original = self.backup_original.as_ref().ok_or_else(|| {
                "Rolling backup ownership is unavailable before promotion".to_string()
            })?;
            if *backup_original != snapshot(&self.backup_path)? {
                return Err(
                    "Concurrent rolling backup change preserved before promotion".to_string(),
                );
            }
            replace_owned(&self.recovery_path, &self.backup_path, backup_original)?;
        } else {
            self.cleanup_recovery()?;
        }
        Ok(())
    }

    fn restore(&self) -> Result<(), String> {
        let Some(original) = self.original.bytes.as_deref() else {
            return Ok(());
        };
        let owned = self.owned.as_ref().ok_or_else(|| {
            "Unknown post-initializer state preserved; retained recovery is available".to_string()
        })?;
        (self.hook)(FileStage::Restore)?;
        let restored = atomic_write(
            self.path,
            original,
            self.permissions.as_ref(),
            owned,
            self.hook,
        )?;
        if snapshot(self.path)? != restored {
            return Err("Restored parser verification failed".to_string());
        }
        Ok(())
    }

    fn failure(&self, error: &str) -> String {
        match self.restore() {
            Ok(()) => self.recovery_error(error, self.cleanup_recovery()),
            Err(restore_error) => {
                self.recovery_error(error, Err(format!("restoration failed: {restore_error}")))
            }
        }
    }
}

pub(crate) fn prepare_parser_at(
    path: &Path,
    executable: &Path,
    imported: &[String],
    initializer: &dyn ParserInitializer,
) -> Result<PreparedParser, String> {
    prepare_parser_at_with(path, executable, imported, initializer, &|_| Ok(()))
}

fn prepare_parser_at_with(
    path: &Path,
    executable: &Path,
    imported: &[String],
    initializer: &dyn ParserInitializer,
    hook: OperationHook<'_>,
) -> Result<PreparedParser, String> {
    validate_phrases(imported)?;
    let lease = acquire(true)?;
    let mut transaction = Transaction::begin(path, hook)?;
    let version = initializer.run_version(executable);
    let observation = snapshot(path);
    let observation_error = observation.as_ref().err().cloned();
    transaction.owned = observation.ok();
    let result = version.and_then(|version| {
        if let Some(error) = observation_error {
            return Err(error);
        }
        transaction.finish(version, imported)
    });
    match result {
        Ok(preparation) => Ok(PreparedParser { preparation, lease }),
        Err(error) => Err(transaction.failure(&error)),
    }
}

pub(crate) fn prepare_local_parser(
    executable: &Path,
    imported: &[String],
) -> Result<PreparedParser, String> {
    prepare_parser_at(
        &resolve_parser_path()?,
        executable,
        imported,
        &process::ProductionInitializer,
    )
}

#[cfg(test)]
#[path = "mod_installer_parser_tests.rs"]
mod tests;
