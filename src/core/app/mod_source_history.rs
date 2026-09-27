// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::mod_downloads::{self, ModDownloadSource};
use crate::platform_defaults::app_config_file;

const MOD_SOURCE_HISTORY_FILE_NAME: &str = "mod_source_history.json";
const HISTORY_CAP_PER_MOD: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct SourceNote {
    #[serde(default)]
    pub(crate) text: String,
    #[serde(default)]
    pub(crate) who: String,
    #[serde(default)]
    pub(crate) date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct HistoryEntry {
    #[serde(default)]
    pub(crate) tp2: String,
    #[serde(default)]
    pub(crate) block: String,
    #[serde(default)]
    pub(crate) saved_to: String,
    #[serde(default)]
    pub(crate) date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct BookmarkEntry {
    #[serde(default)]
    pub(crate) tp2: String,
    #[serde(default)]
    pub(crate) block: String,
    #[serde(default)]
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct ModSourceHistoryStore {
    #[serde(default)]
    pub(crate) notes: BTreeMap<String, SourceNote>,
    #[serde(default)]
    pub(crate) history: Vec<HistoryEntry>,
    #[serde(default)]
    pub(crate) bookmarks: Vec<BookmarkEntry>,
}

fn history_path() -> PathBuf {
    app_config_file(MOD_SOURCE_HISTORY_FILE_NAME, ".")
}

pub(crate) enum LoadedStore {
    Ready(ModSourceHistoryStore),
    Unreadable,
}

pub(crate) fn load_store_checked() -> LoadedStore {
    let path = history_path();
    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<ModSourceHistoryStore>(&content) {
            Ok(store) => LoadedStore::Ready(store),
            Err(err) => {
                warn!(
                    "Parse mod source history failed for {}: {err}",
                    path.display()
                );
                LoadedStore::Unreadable
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            LoadedStore::Ready(ModSourceHistoryStore::default())
        }
        Err(err) => {
            warn!(
                "Load mod source history failed for {}: {err}",
                path.display()
            );
            LoadedStore::Unreadable
        }
    }
}

pub(crate) fn load_store() -> ModSourceHistoryStore {
    match load_store_checked() {
        LoadedStore::Ready(store) => store,
        LoadedStore::Unreadable => ModSourceHistoryStore::default(),
    }
}

pub(crate) fn save_store(store: &ModSourceHistoryStore) -> io::Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(store).map_err(io::Error::other)?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, raw)?;
    fs::rename(&tmp_path, &path)
}

pub(crate) fn with_writable_store<T>(
    mutate: impl FnOnce(&mut ModSourceHistoryStore) -> T,
) -> Result<T, ()> {
    match load_store_checked() {
        LoadedStore::Unreadable => {
            warn!("Mod source history file is unreadable; refusing to save");
            Err(())
        }
        LoadedStore::Ready(mut store) => {
            let result = mutate(&mut store);
            match save_store(&store) {
                Ok(()) => Ok(result),
                Err(err) => {
                    warn!("Save mod source history failed: {err}");
                    Err(())
                }
            }
        }
    }
}

pub(crate) fn rule_signature(source: &ModDownloadSource) -> String {
    let repo_or_url = source.github.as_deref().map_or_else(
        || source.url.trim().to_string(),
        |repo| repo.trim().to_ascii_lowercase(),
    );
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        repo_or_url,
        trimmed(source.commit.as_deref()),
        trimmed(source.tag.as_deref()),
        trimmed(source.branch.as_deref()),
        trimmed(source.release.as_deref()),
        trimmed(source.channel.as_deref()),
        trimmed(source.asset.as_deref()),
    )
}

fn trimmed(value: Option<&str>) -> &str {
    value.map_or("", str::trim)
}

pub(crate) fn note_key(tp2: &str, signature: &str) -> String {
    format!(
        "{}|{signature}",
        mod_downloads::normalize_mod_download_tp2(tp2)
    )
}

pub(crate) fn set_note(
    store: &mut ModSourceHistoryStore,
    key: &str,
    text: &str,
    who: &str,
    date: &str,
) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        store.notes.remove(key);
    } else {
        store.notes.insert(
            key.to_string(),
            SourceNote {
                text: trimmed.to_string(),
                who: who.to_string(),
                date: date.to_string(),
            },
        );
    }
}

pub(crate) fn record_history(
    store: &mut ModSourceHistoryStore,
    tp2: &str,
    block: String,
    saved_to: &str,
    date: &str,
) {
    let normalized_tp2 = mod_downloads::normalize_mod_download_tp2(tp2);
    store.history.push(HistoryEntry {
        tp2: normalized_tp2.clone(),
        block,
        saved_to: saved_to.to_string(),
        date: date.to_string(),
    });
    while store
        .history
        .iter()
        .filter(|entry| entry.tp2 == normalized_tp2)
        .count()
        > HISTORY_CAP_PER_MOD
    {
        if let Some(pos) = store
            .history
            .iter()
            .position(|entry| entry.tp2 == normalized_tp2)
        {
            store.history.remove(pos);
        } else {
            break;
        }
    }
}

pub(crate) fn source_from_block(tp2: &str, block: &str) -> Option<ModDownloadSource> {
    let header = mod_downloads::template_mod_header(tp2, tp2);
    let wrapped = format!("{header}\n\n{block}\n");
    let loaded = mod_downloads::load_mod_download_sources_from_texts("", &wrapped, "");
    loaded.find_sources(tp2).into_iter().next()
}

pub(crate) fn normalize_source(tp2: &str, source: &ModDownloadSource) -> ModDownloadSource {
    let block = mod_downloads::complete_source_block(source);
    source_from_block(tp2, &block).unwrap_or_else(|| source.clone())
}

pub(crate) fn bookmark_block(
    source: &ModDownloadSource,
    installed_source_id: Option<&str>,
    installed_ref: Option<&str>,
) -> Option<(ModDownloadSource, String)> {
    let current_key = mod_downloads::normalize_source_id(&source.source_id);
    let installed_key = installed_source_id.map(mod_downloads::normalize_source_id);
    if installed_key.as_deref() != Some(current_key.as_str()) {
        return None;
    }
    let installed_ref = installed_ref
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if let Some(sha) = crate::app::modlist_share::commit_sha_from_installed_ref(installed_ref) {
        let mut pinned = source.clone();
        pinned.commit = Some(sha.clone());
        pinned.tag = None;
        pinned.branch = None;
        pinned.release = None;
        pinned.channel = None;
        pinned.asset = None;
        let label = sha.chars().take(7).collect::<String>();
        return Some((pinned, label));
    }
    if installed_ref.contains('@') {
        return None;
    }
    let mut pinned = source.clone();
    pinned.tag = Some(installed_ref.to_string());
    pinned.commit = None;
    pinned.branch = None;
    pinned.release = None;
    pinned.channel = None;
    pinned.asset = None;
    Some((pinned, installed_ref.to_string()))
}

pub(crate) fn add_bookmark(
    store: &mut ModSourceHistoryStore,
    tp2: &str,
    source: &ModDownloadSource,
    version: &str,
    date: &str,
) {
    let normalized_tp2 = mod_downloads::normalize_mod_download_tp2(tp2);
    let signature = rule_signature(source);
    let already_bookmarked = store.bookmarks.iter().any(|bookmark| {
        bookmark.tp2 == normalized_tp2
            && source_from_block(&normalized_tp2, &bookmark.block)
                .as_ref()
                .map(rule_signature)
                .as_deref()
                == Some(signature.as_str())
    });
    if already_bookmarked {
        return;
    }
    store.bookmarks.push(BookmarkEntry {
        tp2: normalized_tp2,
        block: mod_downloads::complete_source_block(source),
        version: version.to_string(),
        date: date.to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HistoryConfigDirGuard(PathBuf);

    impl HistoryConfigDirGuard {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_mod_source_history_test_{}_{}_{label}",
                std::process::id(),
                id
            ));
            std::fs::create_dir_all(&path).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(path.clone()));
            Self(path)
        }
    }

    impl Drop for HistoryConfigDirGuard {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn github_source(repo: &str) -> ModDownloadSource {
        ModDownloadSource {
            github: Some(repo.to_string()),
            url: format!("https://github.com/{repo}"),
            source_id: "primary".to_string(),
            source_label: "GitHub".to_string(),
            ..ModDownloadSource::default()
        }
    }

    #[test]
    fn rule_signature_ignores_identity_and_case() {
        let mut left = github_source("Owner/Repo");
        left.tag = Some("v1.0".to_string());
        left.source_id = "a".to_string();
        left.source_label = "A".to_string();
        left.name = "Name A".to_string();

        let mut right = github_source("owner/repo");
        right.tag = Some("v1.0".to_string());
        right.source_id = "b".to_string();
        right.source_label = "B".to_string();
        right.name = "Name B".to_string();

        assert_eq!(rule_signature(&left), rule_signature(&right));

        let mut different = right.clone();
        different.tag = Some("v2.0".to_string());
        assert_ne!(rule_signature(&right), rule_signature(&different));
    }

    #[test]
    fn history_caps_at_twenty_per_mod() {
        let mut store = ModSourceHistoryStore::default();
        for i in 0..25 {
            record_history(
                &mut store,
                "mod.tp2",
                format!("block-{i}"),
                "My default",
                "2026-09-27",
            );
        }
        let count = store.history.iter().filter(|e| e.tp2 == "mod").count();
        assert_eq!(count, 20);
        assert!(!store.history.iter().any(|e| e.block == "block-0"));
        assert!(store.history.iter().any(|e| e.block == "block-24"));
    }

    #[test]
    fn bookmark_uses_matching_commit_then_installed_tag() {
        let source = github_source("owner/repo");

        let (pinned, label) =
            bookmark_block(&source, Some("primary"), Some("commit@abcdef1234")).unwrap();
        assert_eq!(pinned.commit.as_deref(), Some("abcdef1234"));
        assert!(pinned.tag.is_none());
        assert_eq!(label, "abcdef1");

        let (tag_pinned, tag_label) =
            bookmark_block(&source, Some("primary"), Some("v2.0")).unwrap();
        assert_eq!(tag_pinned.tag.as_deref(), Some("v2.0"));
        assert!(tag_pinned.commit.is_none());
        assert_eq!(tag_label, "v2.0");

        assert!(bookmark_block(&source, Some("primary"), Some("weird@nothex")).is_none());
        assert!(bookmark_block(&source, Some("other"), Some("v2.0")).is_none());
        assert!(bookmark_block(&source, None, Some("v2.0")).is_none());
        assert!(bookmark_block(&source, Some("primary"), None).is_none());
    }

    #[test]
    fn bookmarks_deduplicate_by_signature() {
        let mut store = ModSourceHistoryStore::default();
        let source = github_source("owner/repo");
        let mut tagged = source.clone();
        tagged.tag = Some("v1.0".to_string());

        add_bookmark(&mut store, "mod.tp2", &tagged, "v1.0", "2026-09-27");
        add_bookmark(&mut store, "mod.tp2", &tagged, "v1.0", "2026-09-28");
        assert_eq!(store.bookmarks.len(), 1);

        let mut other = source;
        other.tag = Some("v2.0".to_string());
        add_bookmark(&mut store, "mod.tp2", &other, "v2.0", "2026-09-28");
        assert_eq!(store.bookmarks.len(), 2);
    }

    #[test]
    fn notes_round_trip_and_empty_text_deletes() {
        let _guard = HistoryConfigDirGuard::new("notes_round_trip");
        let key = note_key("mod.tp2", "owner/repo|||||");

        let mut store = load_store();
        set_note(
            &mut store,
            &key,
            "Why this version?",
            "My default",
            "2026-09-27",
        );
        save_store(&store).unwrap();

        let reloaded = load_store();
        let note = reloaded.notes.get(&key).unwrap();
        assert_eq!(note.text, "Why this version?");
        assert_eq!(note.who, "My default");

        let mut store2 = reloaded;
        set_note(&mut store2, &key, "  ", "My default", "2026-09-27");
        save_store(&store2).unwrap();
        let reloaded2 = load_store();
        assert!(!reloaded2.notes.contains_key(&key));
    }

    #[test]
    fn store_parse_error_leaves_file_untouched() {
        let guard = HistoryConfigDirGuard::new("store_parse_error");
        let path = history_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();

        let store = load_store();
        assert!(store.notes.is_empty());
        assert!(store.history.is_empty());
        assert!(store.bookmarks.is_empty());

        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, "not json");
        drop(guard);
    }

    #[test]
    fn unreadable_store_is_never_overwritten() {
        let guard = HistoryConfigDirGuard::new("unreadable_store_never_overwritten");
        let path = history_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();

        let history_result = with_writable_store(|store| {
            record_history(
                store,
                "mod.tp2",
                "block".to_string(),
                "My default",
                "2026-09-27",
            );
        });
        assert!(history_result.is_err());

        let note_result = with_writable_store(|store| {
            set_note(
                store,
                &note_key("mod.tp2", "owner/repo|||||"),
                "note",
                "My default",
                "2026-09-27",
            );
        });
        assert!(note_result.is_err());

        let bookmark_result = with_writable_store(|store| {
            add_bookmark(
                store,
                "mod.tp2",
                &github_source("owner/repo"),
                "v1.0",
                "2026-09-27",
            );
        });
        assert!(bookmark_result.is_err());

        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, "not json");
        drop(guard);
    }
}
