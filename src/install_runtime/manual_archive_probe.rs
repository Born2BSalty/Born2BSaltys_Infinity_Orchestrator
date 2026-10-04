// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::app::app_step2_update_download::{
    archive_extension, safe_archive_segment, tp2_archive_name,
};
use crate::app::app_step2_update_extract::archive::{
    accepted_tp2_names, rar_extract, seven_zip_extract, tar_gz_extract, zip_extract,
};
use crate::app::app_step2_update_morpheus_mart::version_from_filename;
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::state::ManualDownloadRequest;
use crate::install_runtime::archive_store::hash_file;
use crate::registry::share_export::ArchiveMeta;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveProbe {
    pub path: PathBuf,
    pub file_name: String,
    pub size: u64,
    pub hash: Option<String>,
    pub tp2_names: Vec<String>,
    pub format: ProbeFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeFormat {
    Zip,
    SevenZip,
    Rar,
    TarGz,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeMatch {
    Meta { store_name: String },
    Tp2 { store_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeRefusal {
    Unsupported,
    NoTp2 { wanted: String },
}

#[must_use]
pub fn is_in_progress_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    [".part", ".crdownload", ".tmp", ".download", ".partial"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

pub fn probe_archive<S: std::hash::BuildHasher>(
    path: &Path,
    wanted_sizes: &HashSet<u64, S>,
) -> std::io::Result<ArchiveProbe> {
    let metadata = fs::metadata(path)?;
    let size = metadata.len();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    let hash = if wanted_sizes.contains(&size) {
        Some(hash_file(path)?)
    } else {
        None
    };
    let format = detect_format(path);
    let tp2_names = list_tp2_names(path, format)?;
    Ok(ArchiveProbe {
        path: path.to_path_buf(),
        file_name,
        size,
        hash,
        tp2_names,
        format,
    })
}

fn detect_format(path: &Path) -> ProbeFormat {
    if zip_extract::is_zip_archive(path) {
        ProbeFormat::Zip
    } else if tar_gz_extract::is_tar_gz_archive(path) {
        ProbeFormat::TarGz
    } else if seven_zip_extract::is_seven_zip_archive(path) {
        ProbeFormat::SevenZip
    } else if rar_extract::is_rar_archive(path) {
        ProbeFormat::Rar
    } else {
        ProbeFormat::Unsupported
    }
}

fn list_tp2_names(path: &Path, format: ProbeFormat) -> std::io::Result<Vec<String>> {
    let entries = match format {
        ProbeFormat::Zip => list_zip_entries(path)?,
        ProbeFormat::TarGz => list_tar_gz_entries(path)?,
        ProbeFormat::SevenZip => list_seven_zip_entries(path)?,
        ProbeFormat::Rar => list_rar_entries(path)?,
        ProbeFormat::Unsupported => Vec::new(),
    };
    Ok(entries
        .into_iter()
        .filter(|name| name.to_ascii_lowercase().ends_with(".tp2"))
        .filter_map(|name| {
            name.replace('\\', "/")
                .rsplit('/')
                .next()
                .map(str::to_ascii_lowercase)
        })
        .collect())
}

fn list_zip_entries(path: &Path) -> std::io::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let archive =
        zip::read::ZipArchive::new(file).map_err(|err| std::io::Error::other(err.to_string()))?;
    Ok(archive.file_names().map(ToString::to_string).collect())
}

fn list_tar_gz_entries(path: &Path) -> std::io::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut names = Vec::new();
    for entry in archive.entries()? {
        let entry = entry?;
        names.push(entry.path()?.to_string_lossy().into_owned());
    }
    Ok(names)
}

fn list_seven_zip_entries(path: &Path) -> std::io::Result<Vec<String>> {
    let archive =
        sevenz_rust2::Archive::open(path).map_err(|err| std::io::Error::other(err.to_string()))?;
    Ok(archive.files.into_iter().map(|entry| entry.name).collect())
}

fn list_rar_entries(path: &Path) -> std::io::Result<Vec<String>> {
    let listing = unrar::Archive::new(path)
        .open_for_listing()
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    let mut names = Vec::new();
    for header in listing {
        let header = header.map_err(|err| std::io::Error::other(err.to_string()))?;
        names.push(header.filename.to_string_lossy().into_owned());
    }
    Ok(names)
}

pub fn match_request(
    probe: &ArchiveProbe,
    request: &ManualDownloadRequest,
    expected: &[ArchiveMeta],
) -> Result<ProbeMatch, ProbeRefusal> {
    let expected_prefix = format!("{}__", tp2_archive_name(&request.tp_file));
    if let Some(hash) = probe.hash.as_deref()
        && let Some(entry) = expected.iter().find(|entry| {
            entry.size == probe.size
                && entry.hash == hash
                && entry.name.starts_with(&expected_prefix)
        })
    {
        return Ok(ProbeMatch::Meta {
            store_name: entry.name.clone(),
        });
    }
    if probe.format == ProbeFormat::Unsupported {
        return Err(ProbeRefusal::Unsupported);
    }
    let accepted = accepted_tp2_names(&request.tp_file, &request.aliases);
    if probe.tp2_names.iter().any(|name| {
        accepted
            .iter()
            .any(|expected| normalize_mod_download_tp2(name) == *expected)
    }) {
        return Ok(ProbeMatch::Tp2 {
            store_name: manual_store_name(request, &probe.file_name),
        });
    }
    Err(ProbeRefusal::NoTp2 {
        wanted: wanted_tp2_display(&request.tp_file),
    })
}

fn wanted_tp2_display(tp_file: &str) -> String {
    tp_file
        .replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or(tp_file)
        .trim()
        .to_ascii_lowercase()
}

#[must_use]
pub fn manual_store_name(request: &ManualDownloadRequest, file_name: &str) -> String {
    let tp2 = safe_archive_segment(&tp2_archive_name(&request.tp_file));
    let source = if request.source_id.trim().is_empty() {
        "manual".to_string()
    } else {
        safe_archive_segment(&request.source_id)
    };
    let tag = safe_archive_segment(
        &version_from_filename(file_name).unwrap_or_else(|| "manual".to_string()),
    );
    let ext = archive_extension(file_name);
    format!("{tp2}__{source}__{tag}{ext}")
}

#[must_use]
pub fn page_url_to_open(url: &str) -> String {
    let trimmed = url.trim();
    let is_nexus = trimmed
        .split_once("://")
        .map_or(trimmed, |(_, rest)| rest)
        .split(['/', '?'])
        .next()
        .is_some_and(|host| {
            host.eq_ignore_ascii_case("nexusmods.com")
                || host.eq_ignore_ascii_case("www.nexusmods.com")
        });
    if is_nexus && !trimmed.contains('?') {
        format!("{trimmed}?tab=files")
    } else {
        trimmed.to_string()
    }
}

pub fn place_into_store(
    probe_path: &Path,
    archive_dir: &Path,
    store_name: &str,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(archive_dir)?;
    let target = archive_dir.join(store_name);
    if probe_path == target {
        return Ok(target);
    }
    if target.exists() {
        return Ok(target);
    }
    if fs::rename(probe_path, &target).is_err() {
        fs::copy(probe_path, &target)?;
        fs::remove_file(probe_path)?;
    }
    Ok(target)
}

pub fn copy_into_store(
    probe_path: &Path,
    archive_dir: &Path,
    store_name: &str,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(archive_dir)?;
    let target = archive_dir.join(store_name);
    if probe_path == target {
        return Ok(target);
    }
    if target.exists() {
        return Ok(target);
    }
    fs::copy(probe_path, &target)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::ManualDownloadReason;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "bio_manualdl_{}_{}_{tag}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn request(tp_file: &str, source_id: &str) -> ManualDownloadRequest {
        ManualDownloadRequest {
            game_tab: "BGEE".to_string(),
            tp_file: tp_file.to_string(),
            label: tp_file.to_string(),
            source_id: source_id.to_string(),
            page_url: String::new(),
            reason: ManualDownloadReason::NotAutoResolvable,
            aliases: Vec::new(),
            display_name: String::new(),
        }
    }

    #[test]
    fn in_progress_names_are_skipped() {
        assert!(is_in_progress_name("Foo.part"));
        assert!(is_in_progress_name("Foo.crdownload"));
        assert!(is_in_progress_name("Foo.tmp"));
        assert!(!is_in_progress_name("Foo.zip"));
    }

    #[test]
    fn probe_zip_lists_tp2_names() {
        let root = TempRoot::new("zip");
        let path = root.path.join("archive.zip");
        {
            let file = fs::File::create(&path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("foo/setup-foo.tp2", options).unwrap();
            writer.write_all(b"tp2 body").unwrap();
            writer.start_file("readme.txt", options).unwrap();
            writer.write_all(b"read me").unwrap();
            writer.finish().unwrap();
        }
        let size = fs::metadata(&path).unwrap().len();

        let probe_unwanted = probe_archive(&path, &HashSet::new()).unwrap();
        assert_eq!(probe_unwanted.tp2_names, vec!["setup-foo.tp2".to_string()]);
        assert_eq!(probe_unwanted.format, ProbeFormat::Zip);
        assert!(probe_unwanted.hash.is_none());

        let mut wanted = HashSet::new();
        wanted.insert(size);
        let probe_wanted = probe_archive(&path, &wanted).unwrap();
        assert!(probe_wanted.hash.is_some());
    }

    #[test]
    fn probe_tar_gz_lists_tp2_names() {
        let root = TempRoot::new("targz");
        let path = root.path.join("archive.tar.gz");
        {
            let file = fs::File::create(&path).unwrap();
            let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut builder = tar::Builder::new(encoder);
            let data = b"tp2 body";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_cksum();
            builder
                .append_data(&mut header, "foo/setup-foo.tp2", &data[..])
                .unwrap();
            let encoder = builder.into_inner().unwrap();
            encoder.finish().unwrap();
        }
        let probe = probe_archive(&path, &HashSet::new()).unwrap();
        assert_eq!(probe.tp2_names, vec!["setup-foo.tp2".to_string()]);
        assert_eq!(probe.format, ProbeFormat::TarGz);
    }

    #[test]
    fn probe_unsupported_format() {
        let root = TempRoot::new("unsupported");
        let path = root.path.join("notes.txt");
        fs::write(&path, b"hello").unwrap();
        let probe = probe_archive(&path, &HashSet::new()).unwrap();
        assert_eq!(probe.format, ProbeFormat::Unsupported);
        assert_eq!(probe.tp2_names.len(), 0);
    }

    #[test]
    fn match_prefers_meta_over_tp2() {
        let probe = ArchiveProbe {
            path: PathBuf::from("Foo.zip"),
            file_name: "Foo.zip".to_string(),
            size: 10,
            hash: Some("deadbeef".to_string()),
            tp2_names: vec!["foo.tp2".to_string()],
            format: ProbeFormat::Zip,
        };
        let request = request("foo.tp2", "github");
        let expected = vec![ArchiveMeta {
            name: "foo__github__v1.zip".to_string(),
            size: 10,
            hash: "deadbeef".to_string(),
        }];
        let result = match_request(&probe, &request, &expected).unwrap();
        assert_eq!(
            result,
            ProbeMatch::Meta {
                store_name: "foo__github__v1.zip".to_string()
            }
        );
    }

    #[test]
    fn match_by_tp2_builds_manual_store_name() {
        let probe = ArchiveProbe {
            path: PathBuf::from("Foo-v2.1.zip"),
            file_name: "Foo-v2.1.zip".to_string(),
            size: 10,
            hash: None,
            tp2_names: vec!["setup-foo.tp2".to_string()],
            format: ProbeFormat::Zip,
        };
        let request = request("setup-foo.tp2", "");
        let result = match_request(&probe, &request, &[]).unwrap();
        assert_eq!(
            result,
            ProbeMatch::Tp2 {
                store_name: "setup-foo__manual__2.1.zip".to_string()
            }
        );
    }

    #[test]
    fn match_refuses_without_tp2() {
        let probe = ArchiveProbe {
            path: PathBuf::from("Bar.zip"),
            file_name: "Bar.zip".to_string(),
            size: 10,
            hash: None,
            tp2_names: vec!["setup-bar.tp2".to_string()],
            format: ProbeFormat::Zip,
        };
        let request = request("setup-foo.tp2", "");
        let result = match_request(&probe, &request, &[]);
        assert_eq!(
            result,
            Err(ProbeRefusal::NoTp2 {
                wanted: "setup-foo.tp2".to_string()
            })
        );
    }

    #[test]
    fn page_url_to_open_adds_files_tab() {
        assert_eq!(
            page_url_to_open("https://www.nexusmods.com/baldursgate2ee/mods/12"),
            "https://www.nexusmods.com/baldursgate2ee/mods/12?tab=files"
        );
        assert_eq!(
            page_url_to_open("https://www.nexusmods.com/baldursgate2ee/mods/12?tab=files"),
            "https://www.nexusmods.com/baldursgate2ee/mods/12?tab=files"
        );
        assert_eq!(
            page_url_to_open("https://example.com/mods/12"),
            "https://example.com/mods/12"
        );
    }

    #[test]
    fn manual_store_name_sanitizes_tag() {
        let req = request("setup-foo.tp2", "");
        assert_eq!(
            manual_store_name(&req, "Foo-v1 rc?.zip"),
            "setup-foo__manual__1-rc.zip"
        );
    }

    #[test]
    fn alias_tp2_matches() {
        let probe = ArchiveProbe {
            path: PathBuf::from("Bar.zip"),
            file_name: "Bar.zip".to_string(),
            size: 10,
            hash: None,
            tp2_names: vec!["setup-baralias.tp2".to_string()],
            format: ProbeFormat::Zip,
        };
        let mut req = request("setup-bar.tp2", "");
        req.aliases = vec!["setup-baralias.tp2".to_string()];
        let result = match_request(&probe, &req, &[]).unwrap();
        assert_eq!(
            result,
            ProbeMatch::Tp2 {
                store_name: "setup-bar__manual__manual.zip".to_string()
            }
        );
    }

    #[test]
    fn place_into_store_renames_and_keeps_existing() {
        let root = TempRoot::new("place");
        let src = root.path.join("dropped.zip");
        fs::write(&src, b"content").unwrap();
        let dest_dir = root.path.join("archives");
        let placed = place_into_store(&src, &dest_dir, "foo__manual__manual.zip").unwrap();
        assert!(placed.exists());
        assert!(!src.exists());

        let src2 = root.path.join("dropped2.zip");
        fs::write(&src2, b"other content").unwrap();
        let placed2 = place_into_store(&src2, &dest_dir, "foo__manual__manual.zip").unwrap();
        assert_eq!(placed2, placed);
        assert!(placed2.exists());
        assert!(src2.exists());
    }
}
