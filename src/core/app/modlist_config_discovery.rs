// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::SystemTime;

use flate2::read::GzDecoder;
use tracing::warn;
use walkdir::{DirEntry, WalkDir};

use crate::app::app_step2_update_download::{safe_archive_segment, tp2_archive_name};
use crate::app::app_step2_update_extract::archive::accepted_tp2_names;
use crate::app::app_step2_update_extract::archive::rar_extract::is_rar_archive;
use crate::app::app_step2_update_extract::archive::seven_zip_extract::is_seven_zip_archive;
use crate::app::app_step2_update_extract::archive::tar_gz_extract::is_tar_gz_archive;
use crate::app::app_step2_update_extract::archive::zip_extract::is_zip_archive;
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::modlist_config_files::{is_os_artifact_file, validate_relative_config_path};
use crate::app::modlist_share::commit_sha_from_installed_ref;

pub(crate) const MAX_CONFIG_FILE_BYTES: u64 = 256 * 1024;
pub(crate) const MAX_CONFIG_TOTAL_BYTES: u64 = 2 * 1024 * 1024;
pub(crate) const TEXT_SNIFF_BYTES: usize = 8 * 1024;

const ARCHIVE_SUFFIXES: [&str; 5] = [".zip", ".7z", ".rar", ".tar.gz", ".tgz"];
const SKIPPED_DIR_NAMES: [&str; 2] = ["backup", "__macosx"];
const SKIPPED_EXTENSIONS: [&str; 2] = ["debug", "exe"];
const SKIPPED_FILE_NAMES: [&str; 1] = ["weidu.log"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ConfigFileReason {
    Changed,
    Catalog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoveredConfigFile {
    pub(crate) relative_path: String,
    pub(crate) reason: ConfigFileReason,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ConfigDiscovery {
    pub(crate) files: Vec<DiscoveredConfigFile>,
    pub(crate) compared_against: Option<String>,
    pub(crate) truncated: bool,
    pub(crate) missing_catalog_files: Vec<String>,
    pub(crate) invalid_catalog_names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArchiveEntry {
    size: u64,
    crc32: u32,
}

#[derive(Debug, Clone)]
struct RelativeEntry {
    archive_key: String,
    entry: ArchiveEntry,
}

struct ResolvedIndex {
    entries: BTreeMap<String, RelativeEntry>,
    child_dir: Option<String>,
}

struct PendingComparison {
    relative_path: String,
    archive_key: String,
    bytes: Vec<u8>,
}

struct DiscoveryRequest<'a> {
    mod_root: &'a Path,
    archive: Option<&'a Path>,
    tp_file: &'a str,
    aliases: &'a [String],
    subdir_require: Option<&'a str>,
    catalog_files: &'a [String],
}

struct WalkedFile {
    relative_path: String,
    path: PathBuf,
    size: u64,
    modified: Option<SystemTime>,
}

struct WalkedTree {
    root: PathBuf,
    root_level: bool,
    files: Vec<WalkedFile>,
}

type FileStamp = (u64, Option<SystemTime>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetadataSnapshot {
    walk_root: Option<PathBuf>,
    walked: BTreeMap<String, FileStamp>,
    catalog: BTreeMap<PathBuf, Option<FileStamp>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    mod_root: String,
    archive: String,
    archive_modified: Option<SystemTime>,
    inputs: u64,
}

struct CacheEntry {
    key: CacheKey,
    snapshot: MetadataSnapshot,
    discovery: ConfigDiscovery,
}

static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
static PENDING_WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LAST_WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub(crate) fn push_pending_warnings(warnings: Vec<String>) {
    let mut last = LAST_WARNINGS.lock().unwrap_or_else(PoisonError::into_inner);
    if *last == warnings {
        return;
    }
    last.clone_from(&warnings);
    drop(last);
    if warnings.is_empty() {
        return;
    }
    PENDING_WARNINGS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .extend(warnings);
}

#[must_use]
pub(crate) fn take_pending_warnings() -> Vec<String> {
    std::mem::take(
        &mut *PENDING_WARNINGS
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    )
}

#[cfg(test)]
pub(crate) fn clear_discovery_cache() {
    discovery_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
}

fn discovery_cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn discover_config_files(
    mod_root: &Path,
    archive: Option<&Path>,
    tp_file: &str,
    aliases: &[String],
    subdir_require: Option<&str>,
    catalog_files: &[String],
) -> Result<ConfigDiscovery, String> {
    let request = DiscoveryRequest {
        mod_root,
        archive,
        tp_file,
        aliases,
        subdir_require,
        catalog_files,
    };
    let walk = walk_tree(&request);
    let snapshot = metadata_snapshot(&request, walk.as_ref());
    let key = cache_key(&request);
    if let Some(cached) = cached_discovery(&key, &snapshot) {
        return Ok(cached);
    }
    let discovery = compute_discovery(&request, walk.as_ref())?;
    discovery_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(
            key.mod_root.clone(),
            CacheEntry {
                key,
                snapshot,
                discovery: discovery.clone(),
            },
        );
    Ok(discovery)
}

fn cached_discovery(key: &CacheKey, snapshot: &MetadataSnapshot) -> Option<ConfigDiscovery> {
    let cache = discovery_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let hit = cache
        .get(&key.mod_root)
        .filter(|entry| entry.key == *key && entry.snapshot == *snapshot)
        .map(|entry| entry.discovery.clone());
    drop(cache);
    hit
}

fn cache_key(request: &DiscoveryRequest<'_>) -> CacheKey {
    let archive_modified = request.archive.and_then(|archive| {
        fs::metadata(archive)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    let mut hasher = DefaultHasher::new();
    request.tp_file.hash(&mut hasher);
    request.aliases.hash(&mut hasher);
    request.subdir_require.hash(&mut hasher);
    request.catalog_files.hash(&mut hasher);
    CacheKey {
        mod_root: request.mod_root.to_string_lossy().to_lowercase(),
        archive: request
            .archive
            .map(|archive| archive.to_string_lossy().to_string())
            .unwrap_or_default(),
        archive_modified,
        inputs: hasher.finish(),
    }
}

fn walk_tree(request: &DiscoveryRequest<'_>) -> Option<WalkedTree> {
    request.archive?;
    let accepted = accepted_tp2_names(request.tp_file, request.aliases);
    let root_level = !request
        .mod_root
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name_is_accepted(name, &accepted));
    let root = if root_level {
        matching_child_dir_on_disk(request.mod_root, &accepted)?
    } else {
        request.mod_root.to_path_buf()
    };
    let files = walk_files(&root);
    Some(WalkedTree {
        root,
        root_level,
        files,
    })
}

fn matching_child_dir_on_disk(parent: &Path, accepted: &[String]) -> Option<PathBuf> {
    let mut matches = fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| name_is_accepted(&entry.file_name().to_string_lossy(), accepted))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.pop()
    } else {
        None
    }
}

fn metadata_snapshot(
    request: &DiscoveryRequest<'_>,
    walk: Option<&WalkedTree>,
) -> MetadataSnapshot {
    let walked = walk
        .map(|walk| {
            walk.files
                .iter()
                .map(|file| (file.relative_path.clone(), (file.size, file.modified)))
                .collect()
        })
        .unwrap_or_default();
    let mut catalog = BTreeMap::new();
    let roots = std::iter::once(request.mod_root).chain(walk.map(|walk| walk.root.as_path()));
    for root in roots {
        for name in request.catalog_files {
            let Ok(relative) = validate_relative_config_path(name) else {
                continue;
            };
            let path = root.join(relative);
            let stamp = fs::metadata(&path)
                .ok()
                .filter(fs::Metadata::is_file)
                .map(|metadata| (metadata.len(), metadata.modified().ok()));
            catalog.insert(path, stamp);
        }
    }
    MetadataSnapshot {
        walk_root: walk.map(|walk| walk.root.clone()),
        walked,
        catalog,
    }
}

fn compute_discovery(
    request: &DiscoveryRequest<'_>,
    walk: Option<&WalkedTree>,
) -> Result<ConfigDiscovery, String> {
    let mut discovery = ConfigDiscovery::default();
    let mut catalog_root = request.mod_root;
    if let Some(archive) = request.archive {
        let compared = walk
            .ok_or_else(|| "the mod folder is not a single mod's folder".to_string())
            .and_then(|walk| {
                compare_with_archive(request, archive, walk).map(|result| (result, walk))
            });
        match compared {
            Ok(((files, truncated), walk)) => {
                discovery.files = files;
                discovery.truncated = truncated;
                discovery.compared_against = Some(archive_display_name(archive));
                catalog_root = walk.root.as_path();
            }
            Err(err) => warn!(
                "config files for {} not compared with {} ({err}); using catalog names",
                request.tp_file,
                archive.display()
            ),
        }
    }
    add_catalog_files(catalog_root, request.catalog_files, &mut discovery)?;
    Ok(discovery)
}

pub(crate) fn find_fetched_archive(
    archive_dir: &Path,
    tp_file: &str,
    source_id: &str,
    installed_ref: Option<&str>,
) -> Option<PathBuf> {
    let prefix = format!(
        "{}__{}__",
        safe_archive_segment(&tp2_archive_name(tp_file)),
        safe_archive_segment(source_id)
    )
    .to_ascii_lowercase();
    let candidates = fs::read_dir(archive_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            let rest = name.strip_prefix(prefix.as_str())?.to_string();
            ARCHIVE_SUFFIXES
                .iter()
                .any(|suffix| rest.ends_with(suffix))
                .then(|| {
                    let modified = entry
                        .metadata()
                        .and_then(|metadata| metadata.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    (rest, entry.path(), modified)
                })
        })
        .collect::<Vec<_>>();
    let needle = installed_ref.and_then(installed_ref_needle);
    let matching = needle.as_deref().map_or_else(Vec::new, |needle| {
        candidates
            .iter()
            .filter(|(rest, _, _)| rest.contains(needle))
            .collect::<Vec<_>>()
    });
    let pool = if matching.is_empty() {
        candidates.iter().collect::<Vec<_>>()
    } else {
        matching
    };
    pool.into_iter()
        .max_by_key(|(_, _, modified)| *modified)
        .map(|(_, path, _)| path.clone())
}

fn installed_ref_needle(installed_ref: &str) -> Option<String> {
    if let Some(sha) = commit_sha_from_installed_ref(installed_ref) {
        return Some(sha.chars().take(7).collect::<String>().to_ascii_lowercase());
    }
    let trimmed = installed_ref.trim();
    (!trimmed.is_empty() && !trimmed.contains('@'))
        .then(|| safe_archive_segment(trimmed).to_ascii_lowercase())
}

fn archive_display_name(archive: &Path) -> String {
    archive.file_name().map_or_else(
        || archive.display().to_string(),
        |name| name.to_string_lossy().to_string(),
    )
}

fn compare_with_archive(
    request: &DiscoveryRequest<'_>,
    archive: &Path,
    walk: &WalkedTree,
) -> Result<(Vec<DiscoveredConfigFile>, bool), String> {
    let accepted = accepted_tp2_names(request.tp_file, request.aliases);
    let resolved = mod_relative_index(&archive_index(archive)?, &accepted, request.subdir_require)?;
    if walk.root_level && !child_dir_matches_walk_root(resolved.child_dir.as_deref(), &walk.root) {
        return Err("the archive does not hold the mod in a folder beside its .tp2".to_string());
    }
    let mut files = Vec::new();
    let mut pending = Vec::new();
    for walked in &walk.files {
        if walked.size > MAX_CONFIG_FILE_BYTES {
            continue;
        }
        let Some(relative) = resolved
            .entries
            .get(&walked.relative_path.to_ascii_lowercase())
        else {
            continue;
        };
        let bytes = fs::read(&walked.path)
            .map_err(|err| format!("Read mod config failed ({}): {err}", walked.path.display()))?;
        if looks_binary(&bytes) || entry_matches_bytes(relative.entry, &bytes) {
            continue;
        }
        if !sizes_allow_line_ending_equality(relative.entry.size, &bytes) {
            files.push(DiscoveredConfigFile {
                relative_path: walked.relative_path.clone(),
                reason: ConfigFileReason::Changed,
                bytes,
            });
            continue;
        }
        pending.push(PendingComparison {
            relative_path: walked.relative_path.clone(),
            archive_key: relative.archive_key.clone(),
            bytes,
        });
    }
    files.extend(changed_after_line_endings(archive, pending)?);
    Ok(cap_total_bytes(files))
}

fn child_dir_matches_walk_root(child_dir: Option<&str>, walk_root: &Path) -> bool {
    child_dir.is_some_and(|dir| {
        walk_root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(dir))
    })
}

fn changed_after_line_endings(
    archive: &Path,
    pending: Vec<PendingComparison>,
) -> Result<Vec<DiscoveredConfigFile>, String> {
    if pending.is_empty() {
        return Ok(Vec::new());
    }
    let wanted = pending
        .iter()
        .map(|item| item.archive_key.clone())
        .collect::<BTreeSet<_>>();
    let archived = read_archive_entries(archive, &wanted)?;
    Ok(pending
        .into_iter()
        .filter(|item| {
            !archived.get(&item.archive_key).is_some_and(|original| {
                without_carriage_returns(original) == without_carriage_returns(&item.bytes)
            })
        })
        .map(|item| DiscoveredConfigFile {
            relative_path: item.relative_path,
            reason: ConfigFileReason::Changed,
            bytes: item.bytes,
        })
        .collect())
}

fn entry_matches_bytes(entry: ArchiveEntry, bytes: &[u8]) -> bool {
    u64::try_from(bytes.len()).is_ok_and(|len| len == entry.size)
        && crc32fast::hash(bytes) == entry.crc32
}

fn sizes_allow_line_ending_equality(archived_size: u64, bytes: &[u8]) -> bool {
    let disk_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    archived_size.max(disk_size) <= archived_size.min(disk_size).saturating_mul(2)
}

fn without_carriage_returns(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut iter = bytes.iter().copied().peekable();
    while let Some(byte) = iter.next() {
        if byte == b'\r' && iter.peek() == Some(&b'\n') {
            continue;
        }
        out.push(byte);
    }
    out
}

fn cap_total_bytes(mut files: Vec<DiscoveredConfigFile>) -> (Vec<DiscoveredConfigFile>, bool) {
    sort_discovered(&mut files);
    let mut kept = Vec::new();
    let mut total = 0_u64;
    let mut truncated = false;
    for file in files {
        let size = u64::try_from(file.bytes.len()).unwrap_or(u64::MAX);
        if truncated || total.saturating_add(size) > MAX_CONFIG_TOTAL_BYTES {
            truncated = true;
            continue;
        }
        total = total.saturating_add(size);
        kept.push(file);
    }
    (kept, truncated)
}

fn sort_discovered(files: &mut [DiscoveredConfigFile]) {
    files.sort_by(|left, right| {
        left.reason
            .cmp(&right.reason)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
}

fn walk_files(root: &Path) -> Vec<WalkedFile> {
    let walker = WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| entry.depth() == 0 || !is_skipped_dir(entry))
        .filter_map(Result::ok);
    let mut files = Vec::new();
    for entry in walker {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        if is_skipped_file(relative) {
            continue;
        }
        let relative_path = relative.to_string_lossy().replace('\\', "/");
        if validate_relative_config_path(&relative_path).is_err() {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        files.push(WalkedFile {
            relative_path,
            path: entry.path().to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }
    files
}

fn is_skipped_dir(entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        && entry.file_name().to_str().is_some_and(|name| {
            SKIPPED_DIR_NAMES
                .iter()
                .any(|skipped| name.eq_ignore_ascii_case(skipped))
        })
}

fn is_skipped_file(relative: &Path) -> bool {
    if is_os_artifact_file(relative) {
        return true;
    }
    let extension_skipped = relative
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            SKIPPED_EXTENSIONS
                .iter()
                .any(|skipped| extension.eq_ignore_ascii_case(skipped))
        });
    let name_skipped = relative
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            SKIPPED_FILE_NAMES
                .iter()
                .any(|skipped| name.eq_ignore_ascii_case(skipped))
        });
    extension_skipped || name_skipped
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(TEXT_SNIFF_BYTES).any(|byte| *byte == 0)
}

fn add_catalog_files(
    catalog_root: &Path,
    catalog_files: &[String],
    discovery: &mut ConfigDiscovery,
) -> Result<(), String> {
    let mut catalog = Vec::<DiscoveredConfigFile>::new();
    for name in catalog_files {
        let relative = match validate_relative_config_path(name) {
            Ok(relative) => relative,
            Err(err) => {
                warn!("catalog config file name skipped ({err})");
                discovery.invalid_catalog_names.push(name.clone());
                continue;
            }
        };
        if is_os_artifact_file(&relative) {
            continue;
        }
        let relative_path = relative.to_string_lossy().replace('\\', "/");
        let path = catalog_root.join(&relative);
        if !path.is_file() {
            discovery.missing_catalog_files.push(relative_path);
            continue;
        }
        let already_collected = discovery
            .files
            .iter()
            .chain(catalog.iter())
            .any(|file| file.relative_path.eq_ignore_ascii_case(&relative_path));
        if already_collected {
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|err| format!("Read mod config failed ({}): {err}", path.display()))?;
        catalog.push(DiscoveredConfigFile {
            relative_path,
            reason: ConfigFileReason::Catalog,
            bytes,
        });
    }
    sort_discovered(&mut catalog);
    discovery.files.extend(catalog);
    discovery.missing_catalog_files.sort();
    discovery.missing_catalog_files.dedup();
    discovery.invalid_catalog_names.sort();
    discovery.invalid_catalog_names.dedup();
    Ok(())
}

fn mod_relative_index(
    full: &BTreeMap<String, ArchiveEntry>,
    accepted: &[String],
    subdir_require: Option<&str>,
) -> Result<ResolvedIndex, String> {
    let required = subdir_require
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let tp2_key = find_archive_tp2(full, accepted, required.as_deref()).ok_or_else(|| {
        required.as_deref().map_or_else(
            || "matching .tp2 not found in archive".to_string(),
            |required| {
                format!("matching .tp2 not found in archive under required path: {required}")
            },
        )
    })?;
    let parent = parent_key(&tp2_key);
    let parent_prefix = if parent.is_empty() {
        String::new()
    } else {
        format!("{parent}/")
    };
    let parent_is_mod_dir = !parent.is_empty() && name_is_accepted(file_segment(parent), accepted);
    let child_dir = if parent_is_mod_dir {
        None
    } else {
        Some(
            matching_child_dir(full, &parent_prefix, accepted)
                .ok_or_else(|| "matching mod folder not found for root-level .tp2".to_string())?,
        )
    };
    let prefix = child_dir.as_deref().map_or_else(
        || parent_prefix.clone(),
        |dir| format!("{parent_prefix}{dir}/"),
    );
    let mut entries = full
        .iter()
        .filter_map(|(key, entry)| {
            let rest = key.strip_prefix(prefix.as_str())?;
            (!rest.is_empty()).then(|| {
                (
                    rest.to_string(),
                    RelativeEntry {
                        archive_key: key.clone(),
                        entry: *entry,
                    },
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    if child_dir.is_some()
        && let Some(entry) = full.get(&tp2_key)
    {
        entries
            .entry(file_segment(&tp2_key).to_string())
            .or_insert_with(|| RelativeEntry {
                archive_key: tp2_key.clone(),
                entry: *entry,
            });
    }
    Ok(ResolvedIndex { entries, child_dir })
}

fn find_archive_tp2(
    full: &BTreeMap<String, ArchiveEntry>,
    accepted: &[String],
    required: Option<&str>,
) -> Option<String> {
    let mut fallback = None;
    for key in full.keys() {
        let file = file_segment(key);
        let is_tp2 = Path::new(file)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tp2"));
        if !is_tp2 || !name_is_accepted(file, accepted) {
            continue;
        }
        if required.is_some_and(|required| !key.contains(required)) {
            continue;
        }
        let parent = parent_key(key);
        if !parent.is_empty() && name_is_accepted(file_segment(parent), accepted) {
            return Some(key.clone());
        }
        fallback.get_or_insert_with(|| key.clone());
    }
    fallback
}

fn matching_child_dir(
    full: &BTreeMap<String, ArchiveEntry>,
    parent_prefix: &str,
    accepted: &[String],
) -> Option<String> {
    let dirs = full
        .keys()
        .filter_map(|key| key.strip_prefix(parent_prefix))
        .filter_map(|rest| rest.split_once('/').map(|(dir, _)| dir))
        .filter(|dir| name_is_accepted(dir, accepted))
        .collect::<BTreeSet<_>>();
    if dirs.len() == 1 {
        dirs.into_iter().next().map(str::to_string)
    } else {
        None
    }
}

fn name_is_accepted(name: &str, accepted: &[String]) -> bool {
    let normalized = normalize_mod_download_tp2(name);
    accepted.contains(&normalized)
}

fn parent_key(key: &str) -> &str {
    key.rsplit_once('/').map_or("", |(parent, _)| parent)
}

fn file_segment(key: &str) -> &str {
    key.rsplit_once('/').map_or(key, |(_, file)| file)
}

fn entry_key(name: &str) -> String {
    name.replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase()
}

fn archive_index(archive: &Path) -> Result<BTreeMap<String, ArchiveEntry>, String> {
    if is_zip_archive(archive) {
        return zip_index(archive);
    }
    if is_tar_gz_archive(archive) {
        return tar_gz_index(archive);
    }
    if is_seven_zip_archive(archive) {
        return seven_zip_index(archive);
    }
    if is_rar_archive(archive) {
        return rar_index(archive);
    }
    Err("unsupported archive format".to_string())
}

fn read_archive_entries(
    archive: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if is_zip_archive(archive) {
        return zip_read(archive, wanted);
    }
    if is_tar_gz_archive(archive) {
        return tar_gz_read(archive, wanted);
    }
    if is_seven_zip_archive(archive) {
        return seven_zip_read(archive, wanted);
    }
    if is_rar_archive(archive) {
        return rar_read(archive, wanted);
    }
    Err("unsupported archive format".to_string())
}

fn zip_index(archive: &Path) -> Result<BTreeMap<String, ArchiveEntry>, String> {
    let file = fs::File::open(archive).map_err(|err| err.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|err| err.to_string())?;
    let mut index = BTreeMap::new();
    for position in 0..zip.len() {
        let entry = zip.by_index_raw(position).map_err(|err| err.to_string())?;
        if entry.is_dir() {
            continue;
        }
        index.insert(
            entry_key(entry.name()),
            ArchiveEntry {
                size: entry.size(),
                crc32: entry.crc32(),
            },
        );
    }
    Ok(index)
}

fn zip_read(
    archive: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let file = fs::File::open(archive).map_err(|err| err.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|err| err.to_string())?;
    let mut out = BTreeMap::new();
    for position in 0..zip.len() {
        let mut entry = zip.by_index(position).map_err(|err| err.to_string())?;
        let key = entry_key(entry.name());
        if entry.is_dir() || !wanted.contains(&key) {
            continue;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|err| err.to_string())?;
        out.insert(key, bytes);
    }
    Ok(out)
}

fn seven_zip_index(archive: &Path) -> Result<BTreeMap<String, ArchiveEntry>, String> {
    let parsed = sevenz_rust2::Archive::open(archive).map_err(|err| err.to_string())?;
    Ok(parsed
        .files
        .iter()
        .filter(|entry| !entry.is_directory())
        .map(|entry| {
            (
                entry_key(entry.name()),
                ArchiveEntry {
                    size: entry.size(),
                    crc32: u32::try_from(entry.crc).unwrap_or_default(),
                },
            )
        })
        .collect())
}

fn seven_zip_read(
    archive: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut reader = sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
        .map_err(|err| err.to_string())?;
    let mut out = BTreeMap::new();
    reader
        .for_each_entries(|entry, data| {
            let key = entry_key(entry.name());
            if !entry.is_directory() && wanted.contains(&key) {
                let mut bytes = Vec::new();
                data.read_to_end(&mut bytes)?;
                out.insert(key, bytes);
            } else {
                io::copy(data, &mut io::sink())?;
            }
            Ok(out.len() < wanted.len())
        })
        .map_err(|err| err.to_string())?;
    Ok(out)
}

fn rar_index(archive: &Path) -> Result<BTreeMap<String, ArchiveEntry>, String> {
    let listing = unrar::Archive::new(archive)
        .as_first_part()
        .open_for_listing()
        .map_err(|err| err.to_string())?;
    let mut index = BTreeMap::new();
    for header in listing {
        let header = header.map_err(|err| err.to_string())?;
        if header.is_directory() {
            continue;
        }
        index.insert(
            entry_key(&header.filename.to_string_lossy()),
            ArchiveEntry {
                size: header.unpacked_size,
                crc32: header.file_crc,
            },
        );
    }
    Ok(index)
}

fn rar_read(
    archive: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut open = unrar::Archive::new(archive)
        .as_first_part()
        .open_for_processing()
        .map_err(|err| err.to_string())?;
    let mut out = BTreeMap::new();
    while let Some(header) = open.read_header().map_err(|err| err.to_string())? {
        let key = entry_key(&header.entry().filename.to_string_lossy());
        open = if header.entry().is_file() && wanted.contains(&key) {
            let (bytes, next) = header.read().map_err(|err| err.to_string())?;
            out.insert(key, bytes);
            next
        } else {
            header.skip().map_err(|err| err.to_string())?
        };
    }
    Ok(out)
}

fn tar_gz_index(archive: &Path) -> Result<BTreeMap<String, ArchiveEntry>, String> {
    let file = fs::File::open(archive).map_err(|err| err.to_string())?;
    let mut tar = tar::Archive::new(GzDecoder::new(file));
    let mut index = BTreeMap::new();
    for entry in tar.entries().map_err(|err| err.to_string())? {
        let mut entry = entry.map_err(|err| err.to_string())?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let key = tar_entry_key(&entry)?;
        let size = entry.size();
        let crc32 = crc32_of_reader(&mut entry)?;
        index.insert(key, ArchiveEntry { size, crc32 });
    }
    Ok(index)
}

fn tar_gz_read(
    archive: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let file = fs::File::open(archive).map_err(|err| err.to_string())?;
    let mut tar = tar::Archive::new(GzDecoder::new(file));
    let mut out = BTreeMap::new();
    for entry in tar.entries().map_err(|err| err.to_string())? {
        let mut entry = entry.map_err(|err| err.to_string())?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let key = tar_entry_key(&entry)?;
        if !wanted.contains(&key) {
            continue;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|err| err.to_string())?;
        out.insert(key, bytes);
    }
    Ok(out)
}

fn tar_entry_key(entry: &tar::Entry<'_, impl Read>) -> Result<String, String> {
    let path = entry.path().map_err(|err| err.to_string())?;
    Ok(entry_key(&path.to_string_lossy()))
}

fn crc32_of_reader(reader: &mut impl Read) -> Result<u32, String> {
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => return Ok(hasher.finalize()),
            Ok(read) => read,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err.to_string()),
        };
        hasher.update(&buffer[..read]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::MutexGuard;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
    static SERIAL: Mutex<()> = Mutex::new(());

    const TP2_TEXT: &[u8] = b"BACKUP ~mymod/backup~\nAUTHOR ~someone~\n";

    fn isolated() -> MutexGuard<'static, ()> {
        let guard = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
        clear_discovery_cache();
        guard
    }

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let root = Self(std::env::temp_dir().join(format!(
                "bio_config_discovery_{}_{id}_{label}",
                std::process::id()
            )));
            let _ = fs::remove_dir_all(&root.0);
            fs::create_dir_all(&root.0).expect("create temp root");
            root
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_file(root: &Path, relative: &str, bytes: &[u8]) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, bytes).expect("write fixture file");
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        for (name, bytes) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .expect("start zip entry");
            zip.write_all(bytes).expect("write zip entry");
        }
        zip.finish().expect("finish zip");
    }

    struct Fixture {
        _root: TempRoot,
        mod_root: PathBuf,
        archive: PathBuf,
    }

    fn fixture(label: &str, archived: &[(&str, &[u8])], on_disk: &[(&str, &[u8])]) -> Fixture {
        let root = TempRoot::new(label);
        let mod_root = root.0.join("mods").join("mymod");
        write_file(&mod_root, "mymod.tp2", TP2_TEXT);
        for (relative, bytes) in on_disk {
            write_file(&mod_root, relative, bytes);
        }
        let archive_dir = root.0.join("archives");
        fs::create_dir_all(&archive_dir).expect("create archive dir");
        let archive = archive_dir.join("mymod__someone__v1.zip");
        let mut entries = vec![("MyMod/mymod.tp2", TP2_TEXT)];
        entries.extend_from_slice(archived);
        write_zip(&archive, &entries);
        Fixture {
            _root: root,
            mod_root,
            archive,
        }
    }

    fn discover_with(fixture: &Fixture, catalog: &[String]) -> ConfigDiscovery {
        discover_config_files(
            &fixture.mod_root,
            Some(&fixture.archive),
            "mymod.tp2",
            &[],
            None,
            catalog,
        )
        .expect("discovery runs")
    }

    fn discover(fixture: &Fixture) -> ConfigDiscovery {
        discover_with(fixture, &[])
    }

    fn paths_and_reasons(discovery: &ConfigDiscovery) -> Vec<(String, ConfigFileReason)> {
        discovery
            .files
            .iter()
            .map(|file| (file.relative_path.clone(), file.reason))
            .collect()
    }

    #[test]
    fn changed_text_file_is_discovered() {
        let _serial = isolated();
        let fixture = fixture(
            "changed",
            &[("MyMod/mymod.ini", b"kits=1\n")],
            &[("mymod.ini", b"kits=1\ncustom=7\n")],
        );

        let discovery = discover(&fixture);

        assert_eq!(
            paths_and_reasons(&discovery),
            vec![("mymod.ini".to_string(), ConfigFileReason::Changed)]
        );
        assert_eq!(discovery.files[0].bytes, b"kits=1\ncustom=7\n".to_vec());
        assert_eq!(
            discovery.compared_against.as_deref(),
            Some("mymod__someone__v1.zip")
        );
    }

    #[test]
    fn file_absent_from_the_archive_is_ignored() {
        let _serial = isolated();
        let fixture = fixture("absent", &[], &[("sub/custom.ini", b"extra=1\n")]);

        let discovery = discover(&fixture);

        assert!(discovery.files.is_empty(), "{:?}", discovery.files);
        assert!(discovery.compared_against.is_some());
    }

    #[test]
    fn catalog_named_file_travels_even_when_the_archive_matches() {
        let _serial = isolated();
        let fixture = fixture(
            "catalogmatch",
            &[
                ("MyMod/mymod.ini", b"kits=1\n"),
                ("MyMod/other.ini", b"other=1\n"),
            ],
            &[("mymod.ini", b"kits=1\n"), ("other.ini", b"other=2\n")],
        );

        let discovery = discover_with(
            &fixture,
            &["mymod.ini".to_string(), "other.ini".to_string()],
        );

        assert_eq!(
            paths_and_reasons(&discovery),
            vec![
                ("other.ini".to_string(), ConfigFileReason::Changed),
                ("mymod.ini".to_string(), ConfigFileReason::Catalog),
            ]
        );
        assert!(discovery.compared_against.is_some());
    }

    #[test]
    fn identical_file_is_skipped() {
        let _serial = isolated();
        let fixture = fixture(
            "identical",
            &[("MyMod/mymod.ini", b"kits=1\n")],
            &[("mymod.ini", b"kits=1\n")],
        );

        let discovery = discover(&fixture);

        assert!(discovery.files.is_empty(), "{:?}", discovery.files);
        assert!(discovery.compared_against.is_some());
    }

    #[test]
    fn crlf_only_difference_is_skipped() {
        let _serial = isolated();
        let fixture = fixture(
            "crlf",
            &[("MyMod/mymod.ini", b"kits=1\nmore=2\n")],
            &[("mymod.ini", b"kits=1\r\nmore=2\r\n")],
        );

        let discovery = discover(&fixture);

        assert!(discovery.files.is_empty(), "{:?}", discovery.files);
    }

    #[test]
    fn binary_file_is_skipped() {
        let _serial = isolated();
        let fixture = fixture(
            "binary",
            &[("MyMod/data.bam", b"BAM \0\x01\x02")],
            &[
                ("data.bam", b"BAM \0\x09\x09\x09"),
                ("new.bif", b"BIFF\0\0\0\0"),
            ],
        );

        let discovery = discover(&fixture);

        assert!(discovery.files.is_empty(), "{:?}", discovery.files);
    }

    #[test]
    fn oversize_file_is_skipped() {
        let _serial = isolated();
        let big = vec![b'a'; usize::try_from(MAX_CONFIG_FILE_BYTES).expect("fits") + 1];
        let fixture = fixture(
            "oversize",
            &[("MyMod/big.ini", b"a")],
            &[("big.ini", big.as_slice())],
        );

        let discovery = discover(&fixture);

        assert!(discovery.files.is_empty(), "{:?}", discovery.files);
    }

    #[test]
    fn backup_and_debug_are_skipped() {
        let _serial = isolated();
        let fixture = fixture(
            "skipped",
            &[
                ("MyMod/backup/0/UNINSTALL.0", b"old\n"),
                ("MyMod/setup-mymod.debug", b"old\n"),
                ("MyMod/WeiDU.log", b"old\n"),
            ],
            &[
                ("backup/0/UNINSTALL.0", b"uninstall\n"),
                ("sub/Backup/list.txt", b"list\n"),
                ("__MACOSX/mymod.ini", b"mac\n"),
                ("setup-mymod.debug", b"debug\n"),
                ("WeiDU.log", b"log\n"),
                ("setup-mymod.exe", b"not really an exe\n"),
                ("Thumbs.db", b"thumbs\n"),
            ],
        );

        let discovery = discover(&fixture);

        assert!(discovery.files.is_empty(), "{:?}", discovery.files);
    }

    #[test]
    fn no_archive_uses_catalog_names_and_reports_missing() {
        let _serial = isolated();
        let root = TempRoot::new("catalog");
        let mod_root = root.0.join("mymod");
        write_file(&mod_root, "mymod.tp2", TP2_TEXT);
        write_file(&mod_root, "mymod.ini", b"kits=1\n");
        write_file(&mod_root, "untouched.txt", b"not named\n");

        let discovery = discover_config_files(
            &mod_root,
            None,
            "mymod.tp2",
            &[],
            None,
            &["mymod.ini".to_string(), "absent.ini".to_string()],
        )
        .expect("discovery runs");

        assert_eq!(
            paths_and_reasons(&discovery),
            vec![("mymod.ini".to_string(), ConfigFileReason::Catalog)]
        );
        assert_eq!(
            discovery.missing_catalog_files,
            vec!["absent.ini".to_string()]
        );
        assert!(discovery.compared_against.is_none());
    }

    #[test]
    fn invalid_catalog_name_is_skipped_not_fatal() {
        let _serial = isolated();
        let root = TempRoot::new("invalid");
        let mod_root = root.0.join("mymod");
        write_file(&mod_root, "mymod.tp2", TP2_TEXT);
        write_file(&mod_root, "mymod.ini", b"kits=1\n");

        let discovery = discover_config_files(
            &mod_root,
            None,
            "mymod.tp2",
            &[],
            None,
            &["../escape.ini".to_string(), "mymod.ini".to_string()],
        )
        .expect("an invalid catalog name does not fail discovery");

        assert_eq!(
            paths_and_reasons(&discovery),
            vec![("mymod.ini".to_string(), ConfigFileReason::Catalog)]
        );
        assert_eq!(
            discovery.invalid_catalog_names,
            vec!["../escape.ini".to_string()]
        );
    }

    #[test]
    fn subdir_require_picks_the_required_variant() {
        let _serial = isolated();
        let root = TempRoot::new("subdir");
        let mod_root = root.0.join("mods").join("x");
        write_file(&mod_root, "setup-x.tp2", TP2_TEXT);
        write_file(&mod_root, "x.ini", b"variant=tweak\n");
        let archive = root.0.join("x__someone__v1.zip");
        write_zip(
            &archive,
            &[
                ("ArtisansKitpack/setup-x.tp2", TP2_TEXT),
                ("ArtisansKitpack/x/x.ini", b"variant=plain\n"),
                ("ArtisansKitpack_tweak/setup-x.tp2", TP2_TEXT),
                ("ArtisansKitpack_tweak/x/x.ini", b"variant=tweak\n"),
            ],
        );
        let run = |subdir_require: Option<&str>| {
            discover_config_files(
                &mod_root,
                Some(&archive),
                "setup-x.tp2",
                &[],
                subdir_require,
                &[],
            )
            .expect("discovery runs")
        };

        let required = run(Some("ArtisansKitpack_tweak"));
        let unfiltered = run(None);

        assert!(required.files.is_empty(), "{:?}", required.files);
        assert!(required.compared_against.is_some());
        assert_eq!(
            paths_and_reasons(&unfiltered),
            vec![("x.ini".to_string(), ConfigFileReason::Changed)]
        );
    }

    #[test]
    fn root_level_tp2_layout_walks_only_the_mod_folder() {
        let _serial = isolated();
        let root = TempRoot::new("rootlevel");
        let mods_root = root.0.join("mods");
        write_file(&mods_root, "setup-foo.tp2", TP2_TEXT);
        write_file(&mods_root, "foo/foo.ini", b"value=changed\n");
        write_file(&mods_root, "bar/bar.ini", b"value=bar\n");
        let archive = root.0.join("foo__someone__v1.zip");
        write_zip(
            &archive,
            &[
                ("setup-foo.tp2", TP2_TEXT),
                ("foo/foo.ini", b"value=original\n"),
            ],
        );

        let discovery =
            discover_config_files(&mods_root, Some(&archive), "setup-foo.tp2", &[], None, &[])
                .expect("discovery runs");

        assert_eq!(
            paths_and_reasons(&discovery),
            vec![("foo.ini".to_string(), ConfigFileReason::Changed)]
        );
        assert!(discovery.compared_against.is_some());
    }

    #[test]
    fn cache_hit_reads_no_files() {
        let _serial = isolated();
        let fixture = fixture(
            "cache",
            &[("MyMod/mymod.ini", b"kits=1\n")],
            &[("mymod.ini", b"kits=1\ncustom=7\n")],
        );
        let catalog = vec!["mymod.ini".to_string()];
        let first = discover_with(&fixture, &catalog);
        assert_eq!(
            paths_and_reasons(&first),
            vec![("mymod.ini".to_string(), ConfigFileReason::Changed)]
        );
        let archive_modified = fs::metadata(&fixture.archive)
            .and_then(|metadata| metadata.modified())
            .expect("archive modified time");
        fs::write(&fixture.archive, b"not an archive").expect("break archive");
        fs::File::options()
            .write(true)
            .open(&fixture.archive)
            .expect("open archive")
            .set_modified(archive_modified)
            .expect("keep archive modified time");

        let cached = discover_with(&fixture, &catalog);

        assert_eq!(cached, first);

        write_file(
            &fixture.mod_root,
            "mymod.ini",
            b"kits=1\ncustom=7\nmore=8\n",
        );
        let recomputed = discover_with(&fixture, &catalog);

        assert_eq!(
            paths_and_reasons(&recomputed),
            vec![("mymod.ini".to_string(), ConfigFileReason::Catalog)]
        );
        assert!(recomputed.compared_against.is_none());
    }

    #[test]
    fn seven_zip_index_reads_crc() {
        let _serial = isolated();
        let root = TempRoot::new("sevenzip");
        let archive = root.0.join("mymod__someone__v1.7z");
        let ini = b"kits=1\ncustom=7\n";
        let mut writer = sevenz_rust2::ArchiveWriter::create(&archive).expect("create 7z");
        for (name, bytes) in [("MyMod/mymod.tp2", TP2_TEXT), ("MyMod/mymod.ini", ini)] {
            writer
                .push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(bytes))
                .expect("push 7z entry");
        }
        writer.finish().expect("finish 7z");

        let index = archive_index(&archive).expect("index reads");

        assert_eq!(
            index.get("mymod/mymod.ini"),
            Some(&ArchiveEntry {
                size: u64::try_from(ini.len()).expect("fits"),
                crc32: crc32fast::hash(ini),
            })
        );
        assert!(index.contains_key("mymod/mymod.tp2"));
    }

    #[test]
    fn archive_chosen_by_installed_sha() {
        let _serial = isolated();
        let root = TempRoot::new("choose");
        let older = root.0.join("mymod__someone__commit-1111111.zip");
        let newer = root.0.join("mymod__someone__commit-2222222.zip");
        let other = root.0.join("othermod__someone__commit-1111111.zip");
        for path in [&older, &newer, &other] {
            write_zip(path, &[("MyMod/mymod.tp2", TP2_TEXT)]);
        }
        fs::File::options()
            .write(true)
            .open(&older)
            .expect("open older")
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000))
            .expect("age older archive");

        let by_sha = find_fetched_archive(
            &root.0,
            "mymod.tp2",
            "someone",
            Some("commit@1111111abcdef"),
        );
        let newest = find_fetched_archive(&root.0, "mymod.tp2", "someone", None);

        assert_eq!(by_sha, Some(older));
        assert_eq!(newest, Some(newer));
    }
}
