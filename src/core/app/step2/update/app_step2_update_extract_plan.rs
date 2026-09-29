// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use crate::app::app_step2_update_download;
use crate::app::app_step2_update_source_refs::{
    RefsTargets, installed_source_refs_path, mods_folder_refs_path,
};
use crate::app::game_authority::{self, GameSlot};
use crate::app::mod_downloads;
use crate::app::state::{Step2UpdateAsset, WizardState};

#[derive(Debug, Clone)]
pub(crate) struct Step2UpdateExtractJob {
    pub(crate) label: String,
    pub(crate) tp_file: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) tp2_rename: Option<mod_downloads::ModDownloadTp2Rename>,
    pub(crate) subdir_require: Option<String>,
    pub(crate) archive_path: PathBuf,
    pub(crate) mods_root: PathBuf,
    pub(crate) backup_root: PathBuf,
    pub(crate) target_root: Option<PathBuf>,
    pub(crate) backup_version_tag: String,
    pub(crate) installed_source_ref: Option<String>,
    pub(crate) installed_source_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExtractPlan {
    pub(crate) jobs: Vec<Step2UpdateExtractJob>,
    pub(crate) refs_targets: RefsTargets,
}

fn resolve_refs_targets(
    mods_folder: &str,
    install_ctx_installed_refs_path: Option<&Path>,
) -> RefsTargets {
    RefsTargets {
        list: install_ctx_installed_refs_path
            .map_or_else(installed_source_refs_path, Path::to_path_buf),
        folder: mods_folder_refs_path(mods_folder),
        folder_label: Some(mods_folder.to_string()),
    }
}

pub(crate) fn build_extract_jobs(
    state: &mut WizardState,
    archive_dir: &Path,
    install_ctx_installed_refs_path: Option<&Path>,
    scope: Option<&str>,
) -> ExtractPlan {
    let mods_folder = state.step1.mods_folder.trim().to_string();
    let mut plan = ExtractPlan {
        jobs: Vec::new(),
        refs_targets: resolve_refs_targets(&mods_folder, install_ctx_installed_refs_path),
    };
    let mods_root = PathBuf::from(&mods_folder);
    if mods_folder.is_empty() {
        state
            .step2
            .update_selected_extract_failed_sources
            .push("Mods Folder is empty".to_string());
        return plan;
    }
    let source_load = mod_downloads::load_mod_download_sources();
    if let Some(err) = source_load.error.as_ref() {
        state
            .step2
            .update_selected_extract_failed_sources
            .push(err.clone());
    }

    for asset in &state.step2.update_selected_update_assets {
        if scope.is_some_and(|tp2| mod_downloads::normalize_mod_download_tp2(&asset.tp_file) != tp2)
        {
            continue;
        }
        let archive_path = archive_dir.join(app_step2_update_download::archive_file_name(asset));
        if !archive_path.exists() {
            continue;
        }
        let source = resolve_selected_source(state, &source_load, &asset.tp_file);
        let installed_source_id = source.as_ref().map(|source| source.source_id.clone());
        let tp2_rename = source.as_ref().and_then(|source| source.tp2_rename.clone());
        let subdir_require = source
            .as_ref()
            .and_then(|source| source.subdir_require.clone());
        plan.jobs.push(Step2UpdateExtractJob {
            label: asset.label.clone(),
            tp_file: asset.tp_file.clone(),
            aliases: source
                .as_ref()
                .map(|source| source.aliases.clone())
                .unwrap_or_default(),
            tp2_rename,
            subdir_require,
            archive_path,
            mods_root: mods_root.clone(),
            backup_root: PathBuf::from(state.step1.mods_backup_folder.trim()),
            target_root: current_mod_root(state, &asset.game_tab, &asset.tp_file),
            backup_version_tag: asset.tag.clone(),
            installed_source_ref: extract_source_ref(asset, source.as_ref()),
            installed_source_id,
        });
    }
    plan
}

fn resolve_selected_source(
    state: &WizardState,
    sources: &mod_downloads::ModDownloadsLoad,
    tp_file: &str,
) -> Option<mod_downloads::ModDownloadSource> {
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp_file);
    sources.resolve_source(
        tp_file,
        state
            .step2
            .selected_source_ids
            .get(&tp2_key)
            .map(String::as_str),
    )
}

fn extract_source_ref(
    asset: &Step2UpdateAsset,
    source: Option<&mod_downloads::ModDownloadSource>,
) -> Option<String> {
    asset.installed_source_ref.clone().or_else(|| {
        if source.is_some_and(|source| source.github.is_some()) {
            return Some(asset.tag.clone());
        }
        if source
            .and_then(|source| source.asset.as_ref())
            .is_some_and(|value| !value.trim().is_empty())
        {
            return Some(asset.tag.clone());
        }
        let asset_name = asset.asset_name.trim().to_ascii_lowercase();
        if asset_name.ends_with("-source.zip")
            || asset_name.ends_with("-source.tar.gz")
            || asset_name.ends_with("-source.tgz")
        {
            Some(asset.tag.clone())
        } else {
            None
        }
    })
}

fn current_mod_root(state: &WizardState, game_tab: &str, tp_file: &str) -> Option<PathBuf> {
    let mods = if game_authority::slot_for_tab(game_tab) == GameSlot::First {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mod_state = mods.iter().find(|mod_state| mod_state.tp_file == tp_file)?;
    let tp2_path = Path::new(mod_state.tp2_path.trim());
    let tp2_parent = tp2_path.parent()?;
    let mods_root = Path::new(state.step1.mods_folder.trim());
    Some(outer_wrapper_root(mods_root, tp2_parent))
}

fn outer_wrapper_root(mods_root: &Path, tp2_parent: &Path) -> PathBuf {
    let mut current = tp2_parent.to_path_buf();
    while let Some(parent) = current.parent() {
        if parent == mods_root || !is_single_child_wrapper(parent, &current) {
            break;
        }
        current = parent.to_path_buf();
    }
    current
}

fn is_single_child_wrapper(parent: &Path, child: &Path) -> bool {
    let Ok(entries) = fs::read_dir(parent) else {
        return false;
    };
    let mut dir_count = 0usize;
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        let Ok(file_type) = entry.file_type() else {
            return false;
        };
        if file_type.is_dir() {
            dir_count += 1;
            if dir_count > 1 || entry.path() != child {
                return false;
            }
        }
    }
    dir_count == 1
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::app::mod_downloads::AMBIENT_TEST_LOCK;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_extract_plan_{}_{}_{label}",
                std::process::id(),
                id
            ));
            fs::create_dir_all(&path).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(path.clone()));
            Self(path)
        }

        fn archive_dir(&self) -> PathBuf {
            let dir = self.0.join("archives");
            fs::create_dir_all(&dir).unwrap();
            dir
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn asset(tp_file: &str, label: &str, tag: &str) -> Step2UpdateAsset {
        Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: "primary".to_string(),
            tag: tag.to_string(),
            asset_name: "asset.zip".to_string(),
            asset_url: "https://example.test/asset.zip".to_string(),
            installed_source_ref: None,
        }
    }

    #[test]
    fn scoped_extract_plan_only_holds_the_scoped_assets() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = TestRoot::new("scoped_extract_plan");
        let archive_dir = root.archive_dir();

        let mut state = WizardState::default();
        state.step1.mods_folder = archive_dir.to_string_lossy().to_string();
        let asset_a = asset("alpha.tp2", "Alpha", "1.0");
        let asset_b = asset("beta.tp2", "Beta", "1.0");
        fs::write(
            archive_dir.join(app_step2_update_download::archive_file_name(&asset_a)),
            b"x",
        )
        .unwrap();
        fs::write(
            archive_dir.join(app_step2_update_download::archive_file_name(&asset_b)),
            b"x",
        )
        .unwrap();
        state.step2.update_selected_update_assets = vec![asset_a, asset_b];

        let refs_path = archive_dir.join("refs.json");
        let jobs =
            build_extract_jobs(&mut state, &archive_dir, Some(&refs_path), Some("alpha")).jobs;

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].tp_file, "alpha.tp2");
    }

    #[test]
    fn build_extract_jobs_resolves_both_refs_targets() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = TestRoot::new("folder_refs_path");
        let archive_dir = root.archive_dir();

        let mut state = WizardState::default();
        state.step1.mods_folder = root.0.join("mods").to_string_lossy().into_owned();
        let alpha = asset("alpha.tp2", "Alpha", "1.0");
        fs::write(
            archive_dir.join(app_step2_update_download::archive_file_name(&alpha)),
            b"x",
        )
        .unwrap();
        state.step2.update_selected_update_assets = vec![alpha];
        let refs_path = root.0.join("refs.toml");

        let mods = state.step1.mods_folder.clone();
        let plan = build_extract_jobs(&mut state, &archive_dir, Some(&refs_path), None);
        let expected = mods_folder_refs_path(&mods);
        assert!(
            expected
                .as_ref()
                .is_some_and(|path| path.starts_with(&root.0)),
            "{expected:?}"
        );
        assert_eq!(plan.jobs.len(), 1);
        assert_eq!(
            plan.refs_targets,
            RefsTargets {
                list: refs_path.clone(),
                folder: expected,
                folder_label: Some(mods.clone()),
            }
        );

        state.step1.mods_folder = format!("  {mods}  ");
        let padded = build_extract_jobs(&mut state, &archive_dir, None, None);
        assert_eq!(padded.refs_targets.list, installed_source_refs_path());
        assert_eq!(padded.refs_targets.folder, plan.refs_targets.folder);
        assert_eq!(padded.refs_targets.folder_label, Some(mods));

        state.step1.mods_folder = "   ".to_string();
        let blank = build_extract_jobs(&mut state, &archive_dir, Some(&refs_path), None);
        assert!(blank.jobs.is_empty());
        assert_eq!(blank.refs_targets.list, refs_path);
        assert_eq!(blank.refs_targets.folder, None);
    }

    fn named_asset(asset_name: &str, tag: &str) -> Step2UpdateAsset {
        Step2UpdateAsset {
            asset_name: asset_name.to_string(),
            ..asset("mod.tp2", "Mod", tag)
        }
    }

    fn github_source() -> mod_downloads::ModDownloadSource {
        mod_downloads::ModDownloadSource {
            github: Some("owner/repo".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn github_release_asset_records_its_tag_as_the_installed_ref() {
        let asset = named_asset("mod-win.zip", "v19");
        assert_eq!(
            extract_source_ref(&asset, Some(&github_source())).as_deref(),
            Some("v19")
        );
    }

    #[test]
    fn page_archive_asset_keeps_the_old_recording_rule() {
        let page_source = mod_downloads::ModDownloadSource::default();
        assert_eq!(
            extract_source_ref(&named_asset("mod.zip", "1.2"), Some(&page_source)),
            None
        );
        assert_eq!(
            extract_source_ref(&named_asset("mod-source.zip", "1.2"), Some(&page_source))
                .as_deref(),
            Some("1.2")
        );
    }

    #[test]
    fn preset_installed_ref_wins() {
        let preset = "commit@7649ced6cd25865874d787ec1a9abbc67b068729";
        let asset = Step2UpdateAsset {
            installed_source_ref: Some(preset.to_string()),
            ..named_asset("mod-win.zip", "v19")
        };
        assert_eq!(
            extract_source_ref(&asset, Some(&github_source())).as_deref(),
            Some(preset)
        );
    }
}
