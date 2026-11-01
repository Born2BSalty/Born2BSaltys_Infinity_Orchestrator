use crate::app::mod_downloads::{self, ModDownloadSource, ModDownloadTp2Rename};
use crate::app::source_form::{self, AssetPick, Follow, SourceFormIdentity, SourceKind};
use crate::app::step2_action::ModSourceEditDestination;

fn github_source() -> ModDownloadSource {
    ModDownloadSource {
        name: "The Darkest Day".to_string(),
        tp2: "tdd".to_string(),
        source_id: "main".to_string(),
        source_label: "Maintainer".to_string(),
        url: "https://github.com/Fixture/The_Darkest_Day".to_string(),
        github: Some("Fixture/The_Darkest_Day".to_string()),
        aliases: vec!["old-tdd".to_string()],
        config_files: vec!["settings.ini".to_string()],
        tp2_rename: Some(ModDownloadTp2Rename {
            from: "old-tdd.tp2".to_string(),
            to: "setup-tdd.tp2".to_string(),
        }),
        exact_github: vec!["Alternate/The_Darkest_Day".to_string()],
        source_default: true,
        subdir_require: Some("package".to_string()),
        pkg_windows: Some("wzp,zip".to_string()),
        pkg_linux: Some("lin,zip".to_string()),
        pkg_macos: Some("mac,zip".to_string()),
        ..ModDownloadSource::default()
    }
}

#[test]
fn catalog_shared_source_form_preserves_capabilities() {
    let source = github_source();
    for (follow, asset, expected_field) in [
        (Follow::Commit, AssetPick::SourceZip, "commit"),
        (Follow::Tag, AssetPick::SourceZip, "tag"),
        (Follow::Branch, AssetPick::SourceZip, "branch"),
        (Follow::Release, AssetPick::SourceZip, "tag"),
        (Follow::Release, AssetPick::BestForOs, "release"),
        (
            Follow::Release,
            AssetPick::Named("fixture-win.zip".to_string()),
            "release",
        ),
    ] {
        let mut form = source_form::from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::GlobalDefault,
            "catalog:tdd:main",
        );
        form.follow = follow;
        form.asset = asset.clone();
        form.commit = "abc123".to_string();
        form.tag = "v2".to_string();
        form.branch = "testing".to_string();
        form.release = "v3".to_string();
        assert_eq!(
            source_form::to_source(&form).pkg_windows,
            source.pkg_windows
        );
        let text = source_form::source_form_save_text(&form);
        let load = mod_downloads::load_mod_download_sources_from_texts(&text, "", "");
        assert!(load.error.is_none(), "{:?}", load.error);
        assert_eq!(load.sources.len(), 1);
        let saved = &load.sources[0];
        let actual = match expected_field {
            "commit" => saved.commit.as_deref(),
            "branch" => saved.branch.as_deref(),
            "release" => saved.release.as_deref(),
            _ => saved.tag.as_deref(),
        };
        let expected = match follow {
            Follow::Commit => "abc123",
            Follow::Tag => "v2",
            Follow::Branch => "testing",
            Follow::Release => "v3",
        };
        assert_eq!(actual, Some(expected));
        assert_eq!(saved.aliases, source.aliases);
        assert_eq!(saved.config_files, source.config_files);
        assert_eq!(saved.tp2_rename, source.tp2_rename);
        assert_eq!(saved.exact_github, source.exact_github);
        assert_eq!(saved.subdir_require, source.subdir_require);
        if expected_field == "release" {
            assert_eq!(saved.pkg_windows, source.pkg_windows);
            assert_eq!(saved.pkg_linux, source.pkg_linux);
            assert_eq!(saved.pkg_macos, source.pkg_macos);
        } else {
            assert!(saved.pkg_windows.is_none());
            assert!(saved.pkg_linux.is_none());
            assert!(saved.pkg_macos.is_none());
        }
        if let AssetPick::Named(name) = asset {
            assert_eq!(saved.asset.as_deref(), Some(name.as_str()));
        }
    }
}

#[test]
fn catalog_shared_source_kind_and_prerelease_notices_roundtrip() {
    let source = github_source();
    for (url, kind) in [
        (
            "https://downloads.weaselmods.net/download/test",
            SourceKind::WeaselMods,
        ),
        (
            "https://www.morpheus-mart.com/test",
            SourceKind::MorpheusMart,
        ),
        (
            "https://example.invalid/fixture.zip",
            SourceKind::DirectLink,
        ),
    ] {
        let mut form = source_form::from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::GlobalDefault,
            "catalog:tdd:main",
        );
        form.kind = kind;
        form.link = url.to_string();
        let saved = source_form::to_source(&form);
        assert_eq!(saved.url, url);
        assert!(saved.github.is_none());
        assert_eq!(saved.aliases, source.aliases);
        assert_eq!(saved.config_files, source.config_files);
    }
    let mut prerelease = source;
    prerelease.channel = Some("preonly".to_string());
    prerelease.asset = Some("fixture-win.zip".to_string());
    let form = source_form::from_source(
        &prerelease,
        SourceFormIdentity::default(),
        ModSourceEditDestination::GlobalDefault,
        "catalog:tdd:main",
    );
    assert!(form.allow_pre);
    assert!(form.notice.is_some());
    let saved = source_form::to_source(&form);
    assert_eq!(saved.channel.as_deref(), Some("pre-release"));
    assert_eq!(saved.asset, prerelease.asset);
}

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct FixtureRoot(PathBuf);

impl FixtureRoot {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "catalog_fixture_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        crate::platform_defaults::set_config_dir_override(Some(root.join("config")));
        Self(root)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        crate::platform_defaults::clear_config_dir_override_if(&self.0.join("config"));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn source_text(sources: &[ModDownloadSource]) -> String {
    sources
        .iter()
        .map(|source| {
            let form = source_form::from_source(
                source,
                SourceFormIdentity::default(),
                ModSourceEditDestination::GlobalDefault,
                "catalog:fixture",
            );
            source_form::source_form_save_text(&form)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fixture_sources() -> Vec<ModDownloadSource> {
    let mut direct = github_source();
    direct.tp2 = "another".to_string();
    direct.name = "Another Adventure".to_string();
    direct.aliases = vec!["ancient-name".to_string()];
    direct.github = None;
    direct.url = "https://example.invalid/another.zip".to_string();
    direct.kind = Some("url".to_string());
    let mut page = direct.clone();
    page.tp2 = "weasel".to_string();
    page.name = "Weasel Quest".to_string();
    page.url = "https://downloads.weaselmods.net/download/fixture".to_string();
    page.kind = Some("page".to_string());
    let mut mart = page.clone();
    mart.tp2 = "mart".to_string();
    mart.name = "Morpheus Quest".to_string();
    mart.url = "https://www.morpheus-mart.com/fixture".to_string();
    vec![github_source(), direct, page, mart]
}

fn write_archive(path: &Path, files: &[(&str, &[u8])]) {
    use std::io::Write;
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    for (name, bytes) in files {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

fn fixture_fork() -> crate::app::state::Step2DiscoveredFork {
    crate::app::state::Step2DiscoveredFork {
        full_name: "Community/The_Darkest_Day".to_string(),
        html_url: "https://github.com/Community/The_Darkest_Day".to_string(),
        owner_login: "Community".to_string(),
        default_branch: "supported".to_string(),
        updated_at: "2026-10-01T00:00:00Z".to_string(),
    }
}

fn fixture_release() -> crate::app::github_release_list::CachedRelease {
    crate::app::github_release_list::CachedRelease {
        tag: "v3".to_string(),
        name: "Fixture release".to_string(),
        prerelease: true,
        published_at: "2026-10-01T00:00:00Z".to_string(),
        assets: vec![crate::app::github_release_list::CachedAsset {
            name: "fixture-win.zip".to_string(),
            size: 4096,
            download_url: "https://example.invalid/fixture-win.zip".to_string(),
        }],
    }
}

#[test]
fn catalog_shared_fork_seed_preserves_identity_without_saving() {
    let source_fork = fixture_fork();
    let form = crate::ui::workspace::versions::versions_sheets::seed_source_form_from_fork(
        "tdd",
        "The Darkest Day",
        &source_fork,
    );
    assert_eq!(form.tp2, "tdd");
    assert_eq!(form.name, "The Darkest Day");
    assert_eq!(form.repo, source_fork.full_name);
    assert_eq!(form.branch, source_fork.default_branch);
    assert_eq!(form.source_id, source_fork.owner_login);
    assert_eq!(form.follow, Follow::Branch);
    assert!(form.identity.may_change_id);
    assert!(!form.identity.is_new_mod);
    assert_eq!(form.save_to, ModSourceEditDestination::GlobalDefault);
    let saved = source_form::to_source(&form);
    assert_eq!(saved.github.as_deref(), Some("Community/The_Darkest_Day"));
    assert_eq!(saved.branch.as_deref(), Some("supported"));
}

use super::state_catalog::{CatalogDestination, CatalogKind, CatalogScreenState, LocalPresence};
use std::collections::{BTreeMap, BTreeSet};

fn fixture_state() -> CatalogScreenState {
    let mut state = CatalogScreenState::default();
    state.replace_from_texts(&source_text(&fixture_sources()), "");
    assert!(state.error.is_none(), "{:?}", state.error);
    state
}

fn destination(root: &FixtureRoot) -> CatalogDestination {
    CatalogDestination {
        mods_root: root.join("mods"),
        archives: root.join("archives"),
        backup_root: root.join("backups"),
    }
}

#[test]
fn catalog_browses_global_sources_without_modlist() {
    let defaults = source_text(&fixture_sources());
    let mut personal = github_source();
    personal.source_label = "Personal override".to_string();
    personal.branch = Some("personal".to_string());
    let mut fork = personal.clone();
    fork.source_id = "community".to_string();
    fork.source_label = "Community branch".to_string();
    fork.github = Some("Community/The_Darkest_Day".to_string());
    fork.source_default = false;
    let mut state = CatalogScreenState::default();
    state.replace_from_texts(&defaults, &source_text(&[personal, fork]));
    assert!(state.loaded);
    assert!(state.error.is_none(), "{:?}", state.error);
    assert_eq!(state.rows.len(), 4);
    let row = state.rows.iter().find(|row| row.tp2 == "tdd").unwrap();
    assert_eq!(row.sources.len(), 2);
    assert_eq!(
        state.source("setup-tdd.tp2").unwrap().source_label,
        "Personal override"
    );
    assert_eq!(
        state.source("tdd").unwrap().branch.as_deref(),
        Some("personal")
    );
    assert_eq!(
        state.tiers.tier_of("tdd", "main"),
        mod_downloads::SourceTier::User
    );
    for (kind, tp2) in [
        (CatalogKind::GitHub, "tdd"),
        (CatalogKind::WeaselMods, "weasel"),
        (CatalogKind::MorpheusMart, "mart"),
        (CatalogKind::DirectArchive, "another"),
    ] {
        state.kind = kind;
        let visible = state.visible_rows();
        assert_eq!(visible.len(), 1);
        assert_eq!(state.rows[visible[0]].tp2, tp2);
    }
}

#[test]
fn catalog_counts_unique_rows_across_mixed_alternatives_and_refresh() {
    let mut sources = fixture_sources();
    let mut duplicate = sources[0].clone();
    duplicate.source_id = "second-github".to_string();
    duplicate.source_default = false;
    let mut mixed = sources[1].clone();
    mixed.tp2 = "tdd".to_string();
    mixed.source_id = "direct-alternative".to_string();
    mixed.source_default = false;
    sources.extend([duplicate, mixed]);
    let mut state = CatalogScreenState::default();
    state.replace_from_texts(&source_text(&sources), "");
    assert!(state.error.is_none(), "{:?}", state.error);
    for (kind, expected) in [
        (CatalogKind::All, vec!["another", "mart", "tdd", "weasel"]),
        (CatalogKind::GitHub, vec!["tdd"]),
        (CatalogKind::WeaselMods, vec!["weasel"]),
        (CatalogKind::MorpheusMart, vec!["mart"]),
        (CatalogKind::DirectArchive, vec!["another", "tdd"]),
    ] {
        assert_eq!(state.kind_count(kind), expected.len());
        state.kind = kind;
        let mut actual: Vec<_> = state
            .visible_rows()
            .iter()
            .map(|&index| state.rows[index].tp2.as_str())
            .collect();
        actual.sort_unstable();
        assert_eq!(actual, expected);
        state.query = "no matching mod".to_string();
        assert_eq!(state.visible_rows(), Vec::<usize>::new());
        assert_eq!(state.kind_count(kind), expected.len());
        assert_eq!(state.kind_count(CatalogKind::All), 4);
        state.query.clear();
    }
    state.replace_from_texts(&source_text(&sources[1..2]), "");
    assert_eq!(state.kind_count(CatalogKind::All), 1);
    assert_eq!(state.kind_count(CatalogKind::GitHub), 0);
    assert_eq!(state.kind_count(CatalogKind::DirectArchive), 1);
    state.replace_from_texts("", "");
    for kind in [
        CatalogKind::All,
        CatalogKind::GitHub,
        CatalogKind::WeaselMods,
        CatalogKind::MorpheusMart,
        CatalogKind::DirectArchive,
    ] {
        assert_eq!(state.kind_count(kind), 0);
    }
}

#[test]
fn catalog_search_filters_keep_selection_and_focus() {
    let mut state = fixture_state();
    state.focus("setup-tdd.tp2");
    assert!(state.selected_tp2.is_empty());
    state.toggle("tdd");
    for query in ["DARKEST", "TDD", "old-tdd", "fixture", "maintainer"] {
        state.query = query.to_string();
        assert!(
            state
                .visible_rows()
                .iter()
                .any(|&index| state.rows[index].tp2 == "tdd")
        );
    }
    state.query = "Ancient-NAME".to_string();
    state.kind = CatalogKind::DirectArchive;
    assert_eq!(state.visible_rows().len(), 1);
    state.toggle_visible();
    assert_eq!(
        state.selected_tp2,
        BTreeSet::from(["tdd".to_string(), "another".to_string()])
    );
    assert_eq!(state.focused_tp2.as_deref(), Some("tdd"));
    state.kind = CatalogKind::GitHub;
    assert_eq!(state.visible_rows(), Vec::<usize>::new());
    assert_eq!(state.selected_tp2.len(), 2);
    state.kind = CatalogKind::DirectArchive;
    state.toggle_visible();
    assert_eq!(state.selected_tp2, BTreeSet::from(["tdd".to_string()]));
    state.toggle("missing");
    assert_eq!(state.selected_tp2.len(), 1);
    state.query.clear();
    state.kind = CatalogKind::All;
    state.replace_from_texts(&source_text(&fixture_sources()[1..]), "");
    assert!(state.focused_tp2.is_none());
    assert!(state.selected_tp2.is_empty());
}

#[test]
fn catalog_download_request_freezes_source_and_destination() {
    let root = FixtureRoot::new();
    let mut state = fixture_state();
    state.toggle("tdd");
    state.toggle("another");
    let frozen = state
        .snapshot(&state.selected_tp2, destination(&root), false)
        .unwrap();
    assert_eq!(frozen.sources.len(), 2);
    assert!(
        state
            .snapshot(&state.selected_tp2, destination(&root), true)
            .unwrap_err()
            .contains("Wait")
    );
    assert!(
        state
            .snapshot(&state.selected_tp2, CatalogDestination::default(), false)
            .is_err()
    );
    assert!(
        state
            .snapshot(&BTreeSet::new(), destination(&root), false)
            .is_err()
    );
    let mut duplicate = destination(&root);
    duplicate.archives = duplicate.mods_root.clone();
    assert!(
        state
            .snapshot(&state.selected_tp2, duplicate, false)
            .is_err()
    );
    let invalid = root.join("file");
    std::fs::write(&invalid, b"sentinel").unwrap();
    let mut invalid_destination = destination(&root);
    invalid_destination.mods_root = invalid;
    assert!(
        state
            .snapshot(&state.selected_tp2, invalid_destination, false)
            .is_err()
    );
    let original = frozen
        .sources
        .iter()
        .find(|source| source.tp2 == "tdd")
        .unwrap()
        .clone();
    let mut changed = github_source();
    changed.commit = Some("other-commit".to_string());
    state.replace_from_texts(&source_text(&[changed]), "");
    assert_eq!(
        frozen.sources.iter().find(|source| source.tp2 == "tdd"),
        Some(&original)
    );
    assert_eq!(frozen.destination, destination(&root));
    assert_ne!(frozen.generation, state.generation);
}

#[test]
fn catalog_forks_seed_shared_editor_and_ignore_stale_repo() {
    let _root = FixtureRoot::new();
    let mut state = fixture_state();
    state.open_editor("tdd");
    let form = state.editor_step2.versions_ui.source_form.as_ref().unwrap();
    assert_eq!(form.save_to, ModSourceEditDestination::GlobalDefault);
    assert!(form.card_key.starts_with("catalog:"));
    state.editor_step2.forks_list.repo = "Fixture/The_Darkest_Day".to_string();
    assert!(!state.accept_forks("stale/repo", Ok(vec![fixture_fork()])));
    assert_eq!(state.editor_step2.mod_download_forks, Vec::new());
    assert!(state.accept_forks("Fixture/The_Darkest_Day", Err("offline".to_string())));
    assert_eq!(
        state.editor_step2.mod_download_forks_popup_error.as_deref(),
        Some("offline")
    );
    assert!(state.accept_forks("Fixture/The_Darkest_Day", Ok(Vec::new())));
    assert!(state.editor_step2.mod_download_forks_popup_error.is_none());
    assert!(state.accept_forks("Fixture/The_Darkest_Day", Ok(vec![fixture_fork()])));
    assert_eq!(state.editor_step2.mod_download_forks, vec![fixture_fork()]);
}

#[test]
fn catalog_releases_ignore_stale_repo_and_keep_assets() {
    use crate::app::github_release_list::ReleaseListStatus;
    let _root = FixtureRoot::new();
    let mut state = fixture_state();
    state.open_editor("tdd");
    state.editor_step2.versions_ui.release_list.repo = "Fixture/The_Darkest_Day".to_string();
    assert!(!state.accept_releases("stale/repo", Ok(vec![fixture_release()])));
    assert_eq!(
        state.editor_step2.versions_ui.release_list.status,
        ReleaseListStatus::Idle
    );
    assert!(state.accept_releases("Fixture/The_Darkest_Day", Err("disconnected".to_string())));
    assert!(matches!(
        state.editor_step2.versions_ui.release_list.status,
        ReleaseListStatus::Failed(_)
    ));
    assert!(state.accept_releases("Fixture/The_Darkest_Day", Ok(vec![fixture_release()])));
    assert_eq!(
        state.editor_step2.versions_ui.release_list.status,
        ReleaseListStatus::Ready(vec![fixture_release()])
    );
    let form = state.editor_step2.versions_ui.source_form.as_mut().unwrap();
    form.follow = Follow::Release;
    form.release = "v3".to_string();
    form.asset = AssetPick::Named("fixture-win.zip".to_string());
    assert_eq!(
        source_form::to_source(form).asset.as_deref(),
        Some("fixture-win.zip")
    );
}

#[test]
fn catalog_scan_results_require_current_generation_and_destination() {
    let root = FixtureRoot::new();
    let mut state = fixture_state();
    state.scan_destination = root.join("mods");
    let result = BTreeMap::from([("tdd".to_string(), LocalPresence::NotFound)]);
    assert!(!state.accept_scan(
        state.generation.wrapping_sub(1),
        &state.scan_destination.clone(),
        result.clone()
    ));
    assert!(!state.accept_scan(state.generation, &root.join("other"), result.clone()));
    assert!(state.local_presence.is_empty());
    assert!(state.accept_scan(
        state.generation,
        &state.scan_destination.clone(),
        result.clone()
    ));
    assert_eq!(state.local_presence, result);
}

fn isolated_app(root: &FixtureRoot) -> crate::ui::orchestrator::orchestrator_app::OrchestratorApp {
    crate::platform_defaults::set_config_dir_override(Some(root.join("bootstrap")));
    crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test("catalog")
}

fn fixture_asset() -> crate::app::state::Step2UpdateAsset {
    crate::app::state::Step2UpdateAsset {
        game_tab: String::new(),
        tp_file: "tdd".to_string(),
        label: "The Darkest Day".to_string(),
        source_id: "main".to_string(),
        tag: "v3".to_string(),
        asset_name: "fixture.zip".to_string(),
        asset_url: "https://example.invalid/fixture.zip".to_string(),
        installed_source_ref: Some("v3".to_string()),
    }
}

#[test]
fn catalog_extracts_local_archive_without_modlist_side_effects() {
    use crate::app::app_step2_update_extract::{archive, plan};
    use crate::app::app_step2_update_source_refs;
    let root = FixtureRoot::new();
    let app = isolated_app(&root);
    let wizard = app.wizard_state.clone();
    let paths = destination(&root);
    std::fs::create_dir_all(&paths.archives).unwrap();
    std::fs::create_dir_all(&paths.mods_root).unwrap();
    let pending = crate::app::modlist_config_files::pending_mod_configs_path();
    std::fs::create_dir_all(pending.parent().unwrap()).unwrap();
    let pending_bytes = b"format_version = 1\n[[files]]\ntp2 = 'tdd'\nsource_id = 'main'\nrelative_path = 'settings.ini'\nbase64_data = 'dXNlcg'\n";
    std::fs::write(&pending, pending_bytes).unwrap();
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let active_list = root.join("active-list");
    std::fs::create_dir_all(&active_list).unwrap();
    mod_downloads::set_active_modlist_dir(Some(active_list.clone()));
    let list_refs = app_step2_update_source_refs::installed_source_refs_path();
    assert_eq!(list_refs.parent(), Some(active_list.as_path()));
    std::fs::write(&list_refs, b"list sentinel").unwrap();
    let source = github_source();
    let job = plan::build_global_extract_job(
        &fixture_asset(),
        &source,
        &paths.archives,
        &paths.mods_root,
        &paths.backup_root,
        None,
    );
    write_archive(
        &job.archive_path,
        &[
            ("wrapper/package/tdd/old-tdd.tp2", b"BACKUP ~unused~"),
            ("wrapper/package/tdd/readme.txt", b"fixture"),
        ],
    );
    let (extracted, record) = archive::extract_one_archive_global(&job).unwrap();
    assert!(extracted.starts_with(&paths.mods_root));
    assert!(extracted.join("setup-tdd.tp2").is_file());
    assert!(!extracted.join("old-tdd.tp2").exists());
    assert!(!extracted.join("settings.ini").exists());
    assert_eq!(std::fs::read(&pending).unwrap(), pending_bytes);
    let folder_refs = root.join("global-folder-refs.toml");
    app_step2_update_source_refs::save_folder_installed_records(
        std::slice::from_ref(&record),
        &folder_refs,
        &paths.mods_root.to_string_lossy(),
    )
    .unwrap();
    let stored = app_step2_update_source_refs::load_refs_file_at(&folder_refs);
    assert_eq!(stored.sources.get("tdd").map(String::as_str), Some("main"));
    assert_eq!(stored.refs.get("tdd").map(String::as_str), Some("v3"));
    assert_eq!(std::fs::read(&list_refs).unwrap(), b"list sentinel");
    assert!(
        app_step2_update_source_refs::save_folder_installed_records(
            &[record],
            &paths.mods_root,
            "invalid file target",
        )
        .is_err()
    );
    assert_eq!(app.wizard_state.step1, wizard.step1);
    assert_eq!(app.wizard_state.step2, wizard.step2);
}

#[test]
fn catalog_archive_failure_preserves_existing_destination() {
    use crate::app::app_step2_update_extract::{archive, plan};
    let root = FixtureRoot::new();
    let paths = destination(&root);
    std::fs::create_dir_all(&paths.archives).unwrap();
    std::fs::create_dir_all(paths.mods_root.join("existing")).unwrap();
    let sentinel = paths.mods_root.join("existing/sentinel.txt");
    std::fs::write(&sentinel, b"keep existing mod").unwrap();
    let job = plan::build_global_extract_job(
        &fixture_asset(),
        &github_source(),
        &paths.archives,
        &paths.mods_root,
        &paths.backup_root,
        None,
    );
    std::fs::write(&job.archive_path, b"not an archive").unwrap();
    assert!(archive::extract_one_archive_global(&job).is_err());
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep existing mod");
    write_archive(&job.archive_path, &[("../escape.txt", b"escape")]);
    assert!(archive::extract_one_archive_global(&job).is_err());
    assert!(!root.join("escape.txt").exists());
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep existing mod");
}

#[test]
fn catalog_global_save_preserves_siblings_notes_and_active_wizard() {
    use crate::app::app_step2_router;
    use crate::app::mod_source_history::{self, SourceNote};
    let root = FixtureRoot::new();
    let mut app = isolated_app(&root);
    app.wizard_state
        .step2
        .selected_source_ids
        .insert("wizard-only".to_string(), "original".to_string());
    app.wizard_state
        .step2
        .skipped_manual_downloads
        .push("wizard-only".to_string());
    let before = app.wizard_state.clone();
    let user_path = mod_downloads::mod_downloads_user_path();
    std::fs::create_dir_all(user_path.parent().unwrap()).unwrap();
    let sibling = fixture_sources()[1].clone();
    std::fs::write(&user_path, source_text(std::slice::from_ref(&sibling))).unwrap();
    let sibling_key = "another|sentinel".to_string();
    let note = SourceNote {
        text: "Keep this personal note".to_string(),
        who: "User".to_string(),
        date: "2026-10-01".to_string(),
    };
    let mut store = mod_source_history::load_store();
    store.notes.insert(sibling_key.clone(), note.clone());
    mod_source_history::save_store(&store).unwrap();
    let mut source = github_source();
    source.tp2 = "catalog-save-fixture".to_string();
    source.name = "Catalog Save Fixture".to_string();
    let mut form = source_form::from_source(
        &source,
        SourceFormIdentity::default(),
        ModSourceEditDestination::GlobalDefault,
        "catalog:save-fixture",
    );
    let mut adapter = crate::app::state::WizardState::default();
    app_step2_router::save_global_source_form(&mut adapter, &form).unwrap();
    let mut seeded_source =
        crate::ui::workspace::versions::versions_sheets::seed_source_form_from_fork(
            "catalog-save-fixture",
            "Catalog Save Fixture",
            &fixture_fork(),
        );
    seeded_source.card_key = "catalog:save-fixture".to_string();
    app_step2_router::save_global_source_form(&mut adapter, &seeded_source).unwrap();
    let text = std::fs::read_to_string(&user_path).unwrap();
    let saved = mod_downloads::load_mod_download_sources_from_texts("", &text, "");
    assert!(saved.error.is_none(), "{:?}", saved.error);
    assert_eq!(saved.find_sources("catalog-save-fixture").len(), 2);
    let retained = saved.find_sources("another");
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].url, sibling.url);
    assert_eq!(retained[0].aliases, sibling.aliases);
    assert_eq!(
        mod_source_history::load_store().notes.get(&sibling_key),
        Some(&note)
    );
    assert!(app_step2_router::save_global_source_form(&mut adapter, &seeded_source).is_err());
    let malformed = "[mods = invalid";
    std::fs::write(&user_path, malformed).unwrap();
    form.label = "Still editable".to_string();
    assert!(app_step2_router::save_global_source_form(&mut adapter, &form).is_err());
    assert_eq!(std::fs::read_to_string(&user_path).unwrap(), malformed);
    std::fs::remove_file(&user_path).unwrap();
    std::fs::create_dir(&user_path).unwrap();
    assert!(app_step2_router::save_global_source_form(&mut adapter, &form).is_err());
    assert!(user_path.is_dir());
    assert_eq!(app.wizard_state.step1, before.step1);
    assert_eq!(app.wizard_state.step2, before.step2);
    assert_eq!(format!("{:?}", app.wizard_state), format!("{before:?}"));
}

use super::catalog_runtime::{CatalogItemOutcome, CatalogRuntime, CatalogTransferEvent};

#[test]
fn catalog_local_presence_uses_global_folder_and_aliases() {
    let root = FixtureRoot::new();
    let paths = destination(&root);
    let nested = paths.mods_root.join("nested/legacy");
    std::fs::create_dir_all(&nested).unwrap();
    let tp2 = nested.join("SETUP-OLD-TDD.TP2");
    std::fs::write(&tp2, b"not parsed or executed").unwrap();
    let mut state = fixture_state();
    let rows = state
        .rows
        .iter()
        .map(|row| (row.tp2.clone(), row.sources.clone()))
        .collect::<Vec<_>>();
    let result = super::catalog_runtime::scan_local_sources(&paths.mods_root, &rows);
    assert_eq!(result.get("tdd"), Some(&LocalPresence::Found(tp2)));
    assert_eq!(result.get("another"), Some(&LocalPresence::NotFound));
    let missing = super::catalog_runtime::scan_local_sources(&root.join("missing"), &rows);
    assert!(matches!(missing.get("tdd"), Some(LocalPresence::Error(_))));
    let mut runtime = CatalogRuntime::default();
    runtime.start_scan(&mut state, Path::new(""));
    assert!(!runtime.pending());
    assert!(state.local_presence.is_empty());
    let (tx, rx) = std::sync::mpsc::channel();
    state.scan_destination = paths.mods_root;
    state
        .local_presence
        .insert("tdd".to_string(), LocalPresence::Scanning);
    runtime.scan_rx = Some(rx);
    drop(tx);
    runtime.poll(&mut state);
    assert!(matches!(
        state.local_presence.get("tdd"),
        Some(LocalPresence::Error(_))
    ));
    assert!(!runtime.pending());
}

#[test]
fn catalog_resolver_request_preserves_every_source_selector() {
    let mut source = github_source();
    source.channel = Some("pre-release".to_string());
    source.commit = Some("commit-value".to_string());
    source.tag = Some("tag-value".to_string());
    source.branch = Some("branch-value".to_string());
    source.release = Some("release-value".to_string());
    source.asset = Some("named.zip".to_string());
    let request = super::catalog_runtime::resolver_request(&source);
    assert_eq!(request.game_tab, "");
    assert_eq!(request.tp_file, source.tp2);
    assert_eq!(request.source_id, source.source_id);
    assert_eq!(request.repo, source.github.unwrap());
    assert_eq!(request.source_url, source.url);
    assert_eq!(request.exact_github, source.exact_github);
    assert_eq!(request.channel, source.channel);
    assert_eq!(request.commit, source.commit);
    assert_eq!(request.tag, source.tag);
    assert_eq!(request.branch, source.branch);
    assert_eq!(request.release, source.release);
    assert_eq!(request.asset, source.asset);
    let package = if cfg!(target_os = "windows") {
        "wzp,zip"
    } else if cfg!(target_os = "macos") {
        "mac,zip"
    } else {
        "lin,zip"
    };
    assert_eq!(request.pkg.as_deref(), Some(package));
    assert!(request.requested_version.is_none());
}

#[test]
fn catalog_download_outcomes_and_cancel_are_isolated() {
    let root = FixtureRoot::new();
    let app = isolated_app(&root);
    let before = format!("{:?}", app.wizard_state);
    let mut state = fixture_state();
    let mut runtime = CatalogRuntime::default();
    runtime.running = true;
    runtime.run_id = 12;
    runtime.cancel_token = Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
        false,
    )));
    let token = runtime.cancel_token.as_ref().unwrap().clone();
    assert!(runtime.pending());
    assert!(!runtime.accept_transfer(
        &mut state,
        11,
        CatalogTransferEvent::Progress("stale".to_string())
    ));
    assert_eq!(state.status, "");
    let (tx, rx) = std::sync::mpsc::channel();
    runtime.transfer_rx = Some(rx);
    tx.send((
        12,
        CatalogTransferEvent::Progress("2048 / 4096 bytes".to_string()),
    ))
    .unwrap();
    tx.send((
        12,
        CatalogTransferEvent::Item {
            tp2: "setup-tdd.tp2".to_string(),
            outcome: CatalogItemOutcome::Completed("finished output".to_string()),
        },
    ))
    .unwrap();
    runtime.poll(&mut state);
    assert_eq!(state.status, "2048 / 4096 bytes");
    runtime.cancel();
    assert!(token.load(Ordering::Relaxed));
    tx.send((
        12,
        CatalogTransferEvent::Item {
            tp2: "another".to_string(),
            outcome: CatalogItemOutcome::Failed("bad archive".to_string()),
        },
    ))
    .unwrap();
    tx.send((
        12,
        CatalogTransferEvent::Item {
            tp2: "mart".to_string(),
            outcome: CatalogItemOutcome::Canceled,
        },
    ))
    .unwrap();
    tx.send((12, CatalogTransferEvent::Finished)).unwrap();
    drop(tx);
    runtime.poll(&mut state);
    assert_eq!(
        runtime.outcomes.get("tdd"),
        Some(&CatalogItemOutcome::Completed(
            "finished output".to_string()
        ))
    );
    assert_eq!(runtime.outcomes.len(), 3);
    assert!(!runtime.running);
    assert!(runtime.refresh_scan);
    assert!(!runtime.pending());
    assert_eq!(state.status, "Finished: 1 completed, 1 failed, 1 canceled");
    assert!(!runtime.accept_transfer(
        &mut state,
        12,
        CatalogTransferEvent::Progress("late".to_string())
    ));
    assert_eq!(format!("{:?}", app.wizard_state), before);
}

#[test]
fn catalog_disconnected_transfer_settles_missing_items_and_keeps_completed() {
    let root = FixtureRoot::new();
    let mut state = fixture_state();
    state.toggle("tdd");
    state.toggle("another");
    let mut runtime = CatalogRuntime::default();
    runtime.running = true;
    runtime.run_id = 7;
    runtime.snapshot = Some(
        state
            .snapshot(&state.selected_tp2, destination(&root), false)
            .unwrap(),
    );
    runtime.accept_transfer(
        &mut state,
        7,
        CatalogTransferEvent::Item {
            tp2: "tdd".to_string(),
            outcome: CatalogItemOutcome::Completed("done".to_string()),
        },
    );
    let (tx, rx) = std::sync::mpsc::channel();
    runtime.transfer_rx = Some(rx);
    drop(tx);
    runtime.poll(&mut state);
    assert!(!runtime.running);
    assert_eq!(
        runtime.outcomes.get("tdd"),
        Some(&CatalogItemOutcome::Completed("done".to_string()))
    );
    assert!(
        matches!(runtime.outcomes.get("another"), Some(CatalogItemOutcome::Failed(error)) if error.contains("stopped"))
    );
    assert_eq!(state.status, "Finished: 1 completed, 1 failed, 0 canceled");
}

#[test]
fn catalog_precanceled_shared_download_dispatches_no_asset_io() {
    use crate::app::app_step2_update_download::{self, Step2UpdateDownloadEvent};
    let root = FixtureRoot::new();
    let mut asset = fixture_asset();
    asset.asset_url = "invalid-url-never-dispatched".to_string();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let rx = app_step2_update_download::spawn_global_download(
        root.join("archives"),
        vec![asset],
        cancel,
    );
    loop {
        match rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap() {
            Step2UpdateDownloadEvent::AssetProgress { .. } => panic!("canceled item dispatched"),
            Step2UpdateDownloadEvent::AssetDone { ok, error, .. } => {
                assert!(!ok);
                assert!(error.unwrap().to_lowercase().contains("cancel"));
            }
            Step2UpdateDownloadEvent::Finished(result) => {
                assert_eq!(result.downloaded, Vec::<String>::new());
                assert_eq!(result.failed.len(), 1);
                break;
            }
        }
    }
    assert_eq!(std::fs::read_dir(root.join("archives")).unwrap().count(), 0);
}

fn seed_existing_note(state: &CatalogScreenState) -> String {
    use crate::app::mod_source_history;
    let source = state.source("tdd").unwrap();
    let signature = mod_source_history::rule_signature(source);
    let note_key = mod_source_history::note_key("tdd", &signature);
    let mut store = mod_source_history::load_store();
    mod_source_history::set_note(
        &mut store,
        &note_key,
        "Keep existing note",
        "User",
        "2026-10-01",
    );
    mod_source_history::save_store(&store).unwrap();
    note_key
}

#[test]
fn catalog_personal_save_preserves_other_sources_and_wizard() {
    use crate::app::mod_source_history;
    let root = FixtureRoot::new();
    let mut app = isolated_app(&root);
    app.catalog_screen_state = fixture_state();
    let note_key = seed_existing_note(&app.catalog_screen_state);
    app.catalog_screen_state.open_editor("tdd");
    let form = app
        .catalog_screen_state
        .editor_step2
        .versions_ui
        .source_form
        .as_ref()
        .unwrap();
    assert_eq!(form.note, "Keep existing note");
    assert_eq!(form.note_seed, "Keep existing note");
    let before = format!("{:?}", app.wizard_state);
    let user_path = mod_downloads::mod_downloads_user_path();
    std::fs::create_dir_all(user_path.parent().unwrap()).unwrap();
    std::fs::write(&user_path, "invalid = [").unwrap();
    assert!(
        app.catalog_runtime
            .save_editor(&mut app.catalog_screen_state)
            .is_err()
    );
    let form = app
        .catalog_screen_state
        .editor_step2
        .versions_ui
        .source_form
        .as_ref()
        .unwrap();
    assert!(form.error.is_some());
    assert_eq!(form.note, "Keep existing note");
    assert_eq!(std::fs::read_to_string(&user_path).unwrap(), "invalid = [");
    app.catalog_runtime
        .dismiss_editor(&mut app.catalog_screen_state);
    assert!(
        app.catalog_screen_state
            .editor_step2
            .versions_ui
            .source_form
            .is_none()
    );
    assert!(
        app.catalog_screen_state
            .editor_step2
            .versions_ui
            .sheet
            .is_none()
    );
    app.catalog_runtime
        .open_forks(&mut app.catalog_screen_state, "tdd");
    app.catalog_screen_state
        .accept_forks("Fixture/The_Darkest_Day", Ok(vec![fixture_fork()]));
    app.catalog_runtime
        .use_fork(&mut app.catalog_screen_state, &fixture_fork());
    let form = app
        .catalog_screen_state
        .editor_step2
        .versions_ui
        .source_form
        .as_ref()
        .unwrap();
    assert_eq!(form.repo, "Community/The_Darkest_Day");
    assert!(form.identity.may_change_id);
    assert!(form.card_key.starts_with("catalog:"));
    assert_eq!(std::fs::read_to_string(&user_path).unwrap(), "invalid = [");
    let sibling = fixture_sources()[1].clone();
    std::fs::write(&user_path, source_text(std::slice::from_ref(&sibling))).unwrap();
    app.catalog_runtime
        .save_editor(&mut app.catalog_screen_state)
        .unwrap();
    assert!(
        app.catalog_screen_state
            .editor_step2
            .versions_ui
            .source_form
            .is_none()
    );
    assert!(!app.catalog_runtime.loaded);
    let user = std::fs::read_to_string(&user_path).unwrap();
    let load = mod_downloads::load_mod_download_sources_from_texts("", &user, "");
    assert_eq!(load.find_sources("another")[0].url, sibling.url);
    assert_eq!(
        load.find_sources("tdd")[0].github.as_deref(),
        Some("Community/The_Darkest_Day")
    );
    assert_eq!(
        mod_source_history::load_store()
            .notes
            .get(&note_key)
            .unwrap()
            .text,
        "Keep existing note"
    );
    assert_eq!(format!("{:?}", app.wizard_state), before);
}

use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::shared::redesign_tokens::ThemePalette;
use eframe::egui;
use egui_kittest::Harness;

struct CatalogScene {
    app: crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    urls: Vec<String>,
    copied_paths: Vec<String>,
    content_rect: egui::Rect,
    popup_open: bool,
    focused_widget: Option<egui::Id>,
    root: FixtureRoot,
}

fn catalog_scene(
    size: egui::Vec2,
    palette: ThemePalette,
    surface: &str,
) -> Harness<'static, CatalogScene> {
    let root = FixtureRoot::new();
    let mut app = isolated_app(&root);
    app.theme_palette = palette;
    app.nav = NavDestination::Catalog;
    app.last_rendered_nav = NavDestination::Catalog;
    app.catalog_screen_state = fixture_state();
    app.catalog_screen_state.focus("tdd");
    app.catalog_screen_state.toggle("tdd");
    app.catalog_screen_state.toggle("another");
    app.wizard_state.github_auth_login = "Fixture".to_string();
    app.wizard_state.step1.global_mods_folder = root.join("mods").to_string_lossy().into_owned();
    app.wizard_state.step1.mods_archive_folder =
        root.join("archives").to_string_lossy().into_owned();
    app.wizard_state.step1.mods_backup_folder = root.join("backups").to_string_lossy().into_owned();
    std::fs::create_dir_all(root.join("mods/tdd")).unwrap();
    std::fs::write(root.join("mods/tdd/setup-tdd.tp2"), b"fixture not executed").unwrap();
    app.catalog_screen_state.scan_destination = root.join("mods");
    app.catalog_screen_state.local_presence = BTreeMap::from([
        (
            "tdd".to_string(),
            LocalPresence::Found(root.join("mods/tdd/setup-tdd.tp2")),
        ),
        ("another".to_string(), LocalPresence::NotFound),
        ("weasel".to_string(), LocalPresence::NotFound),
        ("mart".to_string(), LocalPresence::NotFound),
    ]);
    app.catalog_runtime.mark_loaded();
    seed_catalog_surface(&mut app, surface);
    let mut fonts_ready = false;
    Harness::builder()
        .with_size(size)
        .with_step_dt(1.0 / 60.0)
        .with_max_steps(64)
        .build_state(
            move |ctx, scene| {
                if !fonts_ready {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                    fonts_ready = true;
                    ctx.request_repaint();
                    return;
                }
                seed_catalog_scene_releases(scene);
                ctx.set_visuals(crate::ui::shared::redesign_visuals::build_for(
                    scene.app.theme_palette,
                ));
                egui::SidePanel::left("catalog_test_rail")
                    .exact_width(crate::ui::shared::redesign_tokens::REDESIGN_NAV_WIDTH_PX)
                    .show(ctx, |ui| {
                        crate::ui::orchestrator::left_rail::render(
                            ui,
                            scene.app.theme_palette,
                            &mut scene.app.nav,
                            false,
                            &scene.app.path_validation,
                            None,
                        );
                    });
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::NONE
                            .fill(crate::ui::shared::redesign_tokens::redesign_page_bg(
                                scene.app.theme_palette,
                            ))
                            .inner_margin(egui::Margin {
                                left: 28,
                                right: 28,
                                top: 24,
                                bottom: 24,
                            }),
                    )
                    .show(ctx, |ui| {
                        scene.content_rect = ui.max_rect();
                        crate::ui::orchestrator::page_router::render(ui, &mut scene.app, ctx);
                    });
                scene.popup_open = ctx.memory(egui::Memory::any_popup_open);
                scene.focused_widget = ctx.memory(egui::Memory::focused);
                ctx.output(|output| {
                    for command in &output.commands {
                        match command {
                            egui::OutputCommand::OpenUrl(url) => scene.urls.push(url.url.clone()),
                            egui::OutputCommand::CopyText(text) => {
                                scene.copied_paths.push(text.clone());
                            }
                            egui::OutputCommand::CopyImage(_) => {}
                        }
                    }
                });
            },
            CatalogScene {
                app,
                urls: Vec::new(),
                copied_paths: Vec::new(),
                content_rect: egui::Rect::NOTHING,
                popup_open: false,
                focused_widget: None,
                root,
            },
        )
}

fn seed_catalog_surface(
    app: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    surface: &str,
) {
    if surface == "source-editor" {
        app.catalog_screen_state.open_editor("tdd");
        let form = app
            .catalog_screen_state
            .editor_step2
            .versions_ui
            .source_form
            .as_mut()
            .unwrap();
        form.follow = Follow::Release;
        form.release = "v3".to_string();
        form.asset = AssetPick::Named("fixture-win.zip".to_string());
        form.allow_pre = true;
        form.advanced_open = true;
        app.catalog_screen_state
            .editor_step2
            .versions_ui
            .release_list
            .repo = "Fixture/The_Darkest_Day".to_string();
        app.catalog_screen_state
            .accept_releases("Fixture/The_Darkest_Day", Ok(vec![fixture_release()]));
    } else if surface == "forks" {
        app.catalog_runtime
            .open_forks(&mut app.catalog_screen_state, "tdd");
        app.catalog_screen_state
            .accept_forks("Fixture/The_Darkest_Day", Ok(vec![fixture_fork()]));
    }
}

fn text_bounds(shape: &egui::Shape, label: &str) -> Option<egui::Rect> {
    match shape {
        egui::Shape::Text(text) if text.galley.text() == label => {
            Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
        }
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| text_bounds(shape, label)),
        _ => None,
    }
}

fn assert_visible_text(harness: &Harness<'_, CatalogScene>, label: &str, size: egui::Vec2) {
    let bounds = harness
        .output()
        .shapes
        .iter()
        .find_map(|shape| text_bounds(&shape.shape, label))
        .unwrap_or_else(|| panic!("missing painted text: {label}"));
    assert!(
        bounds.left() >= 0.0
            && bounds.top() >= 0.0
            && bounds.right() <= size.x
            && bounds.bottom() <= size.y,
        "{label}: {bounds:?} outside {size:?}"
    );
}

#[test]
fn catalog_production_main_source_editor_forks_render() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for (palette_name, palette) in [("dark", ThemePalette::Dark), ("light", ThemePalette::Light)] {
        for (width_name, size) in [
            ("normal", egui::vec2(1570.0, 1002.0)),
            ("narrow", egui::vec2(1100.0, 760.0)),
        ] {
            for surface in ["main", "source-editor", "forks"] {
                let mut harness = catalog_scene(size, palette, surface);
                match surface {
                    "source-editor" => {
                        assert_visible_text(&harness, "Save to My default", size);
                        assert_visible_text(&harness, "Cancel", size);
                    }
                    "forks" => {
                        assert_visible_text(&harness, "Use as new source", size);
                        assert_visible_text(&harness, "Community/The_Darkest_Day", size);
                    }
                    _ => {
                        assert_visible_text(&harness, "Mod catalog", size);
                        assert_visible_text(&harness, "Download selected (2)", size);
                        assert_visible_text(&harness, "All (4)", size);
                        assert_catalog_footer_fits(&harness, false);
                    }
                }
                if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
                    let directory = PathBuf::from(directory);
                    std::fs::create_dir_all(&directory).unwrap();
                    harness
                        .render()
                        .unwrap()
                        .save(
                            directory
                                .join(format!("catalog-{surface}-{palette_name}-{width_name}.png")),
                        )
                        .unwrap();
                }
            }
        }
    }
}

#[test]
fn catalog_navigation_roundtrip_preserves_wizard() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1570.0, 1002.0), ThemePalette::Dark, "main");
    harness.state_mut().app.nav = NavDestination::Create;
    harness.run();
    seed_active_wizard(&mut harness.state_mut().app.wizard_state);
    let before = format!("{:?}", harness.state().app.wizard_state);
    let workspaces = format!("{:?}", harness.state().app.workspace_state);
    harness.state_mut().app.nav = NavDestination::Catalog;
    harness.run();
    harness.state_mut().app.catalog_screen_state.query = "TDD".to_string();
    harness
        .state_mut()
        .app
        .catalog_screen_state
        .open_editor("tdd");
    {
        let state = &mut harness.state_mut().app.catalog_screen_state;
        state.editor_step2.versions_ui.release_list.repo = "Fixture/The_Darkest_Day".to_string();
        state.accept_releases("Fixture/The_Darkest_Day", Ok(vec![fixture_release()]));
    }
    harness.run();
    {
        let scene = harness.state_mut();
        scene
            .app
            .catalog_runtime
            .dismiss_editor(&mut scene.app.catalog_screen_state);
        scene.app.catalog_runtime.running = true;
        scene.app.catalog_runtime.run_id = 9;
        scene.app.catalog_runtime.accept_transfer(
            &mut scene.app.catalog_screen_state,
            9,
            CatalogTransferEvent::Item {
                tp2: "tdd".to_string(),
                outcome: CatalogItemOutcome::Completed("isolated fake output".to_string()),
            },
        );
        scene.app.catalog_runtime.accept_transfer(
            &mut scene.app.catalog_screen_state,
            9,
            CatalogTransferEvent::Finished,
        );
        assert!(scene.app.catalog_runtime.refresh_scan);
        scene.app.catalog_runtime.refresh_scan = false;
    }
    harness.state_mut().app.nav = NavDestination::Create;
    harness.run();
    assert_eq!(format!("{:?}", harness.state().app.wizard_state), before);
    assert_eq!(
        format!("{:?}", harness.state().app.workspace_state),
        workspaces
    );
    harness.state_mut().app.nav = NavDestination::Catalog;
    harness.run();
    assert_eq!(harness.state().app.catalog_screen_state.query, "TDD");
    assert_eq!(
        harness.state().app.catalog_screen_state.selected_tp2.len(),
        2
    );
}

struct AmbientGuard(Option<PathBuf>);

impl Drop for AmbientGuard {
    fn drop(&mut self) {
        mod_downloads::set_active_modlist_dir(self.0.take());
    }
}

#[test]
fn catalog_ui_fork_action_opens_global_editor_without_saving() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1570.0, 1002.0), ThemePalette::Dark, "forks");
    let before = format!("{:?}", harness.state().app.wizard_state);
    let user_path = mod_downloads::mod_downloads_user_path();
    let user_before = std::fs::read(&user_path).ok();
    let bounds = harness
        .output()
        .shapes
        .iter()
        .find_map(|shape| text_bounds(&shape.shape, "Use as new source"))
        .unwrap();
    let pos = bounds.center();
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    harness.run();
    let form = harness
        .state()
        .app
        .catalog_screen_state
        .editor_step2
        .versions_ui
        .source_form
        .as_ref()
        .unwrap();
    assert_eq!(form.repo, "Community/The_Darkest_Day");
    assert_eq!(form.save_to, ModSourceEditDestination::GlobalDefault);
    assert_eq!(std::fs::read(&user_path).ok(), user_before);
    assert_eq!(format!("{:?}", harness.state().app.wizard_state), before);
}

#[test]
fn catalog_note_write_warning_remains_visible_after_source_save() {
    let root = FixtureRoot::new();
    let mut app = isolated_app(&root);
    app.catalog_screen_state = fixture_state();
    app.catalog_screen_state.open_editor("tdd");
    let form = app
        .catalog_screen_state
        .editor_step2
        .versions_ui
        .source_form
        .as_mut()
        .unwrap();
    form.note = "Requested new personal note".to_string();
    let history_path =
        crate::platform_defaults::app_config_file("mod_source_history.json", "config");
    std::fs::create_dir_all(history_path.parent().unwrap()).unwrap();
    std::fs::write(&history_path, b"invalid history bytes").unwrap();
    app.catalog_runtime
        .save_editor(&mut app.catalog_screen_state)
        .unwrap();
    assert!(app.catalog_screen_state.status.contains("Source saved"));
    assert!(app.catalog_screen_state.status.contains("not saved"));
    assert_eq!(
        std::fs::read(&history_path).unwrap(),
        b"invalid history bytes"
    );
}

#[test]
fn catalog_transfer_folder_lease_blocks_conflicting_wizard_downloads() {
    use crate::app::app_step2_update_download::{self, DownloadRefusal};
    let root = FixtureRoot::new();
    let paths = destination(&root);
    let lease = app_step2_update_download::reserve_catalog_destination(
        &paths.mods_root,
        &paths.archives,
        &paths.backup_root,
    )
    .unwrap();
    for path in [&paths.mods_root, &paths.archives, &paths.backup_root] {
        assert!(app_step2_update_download::catalog_destination_busy(
            &path.to_string_lossy()
        ));
        let alias = path.join("child/..");
        assert!(app_step2_update_download::catalog_destination_busy(
            &alias.to_string_lossy()
        ));
        assert!(
            app_step2_update_download::reserve_catalog_destination(
                path,
                &root.join("other-archive"),
                &root.join("other-backup")
            )
            .is_err()
        );
    }
    for role in ["mods", "archive", "backup"] {
        let mut wizard = crate::app::state::WizardState::default();
        let value = match role {
            "mods" => &paths.mods_root,
            "archive" => &paths.archives,
            _ => &paths.backup_root,
        }
        .to_string_lossy()
        .into_owned();
        match role {
            "mods" => wizard.step1.mods_folder = value,
            "archive" => wizard.step1.mods_archive_folder = value,
            _ => wizard.step1.mods_backup_folder = value,
        }
        let mut receiver = None;
        assert_eq!(
            app_step2_update_download::start_step2_update_download(&mut wizard, &mut receiver),
            Err(DownloadRefusal::AlreadyRunning)
        );
        assert!(receiver.is_none());
        assert!(!wizard.step2.update_selected_download_running);
        assert!(wizard.step2.scan_status.contains("Catalog"));
    }
    drop(lease);
    assert!(!app_step2_update_download::catalog_destination_busy(
        &paths.mods_root.to_string_lossy()
    ));
    assert!(
        app_step2_update_download::reserve_catalog_destination(
            &paths.mods_root,
            &paths.archives,
            &paths.backup_root
        )
        .is_ok()
    );
}

fn seed_active_wizard(wizard: &mut crate::app::state::WizardState) {
    use crate::app::state::{
        InstallerAnswerDraft, InstallerAnswerTarget, InstallerAnswersEditor, Step3ItemState,
    };
    wizard.current_step = 3;
    wizard.step2.search_query = "Active wizard query".to_string();
    wizard
        .step2
        .selected_source_ids
        .insert("wizard-fixture".to_string(), "wizard-source".to_string());
    wizard.step2.next_selection_order = 7;
    wizard.step2.selected_count = 1;
    wizard.step2.update_selected_locked_update_assets = vec![fixture_asset()];
    wizard.step3.bgee_items = vec![Step3ItemState {
        tp_file: "wizard-fixture".to_string(),
        component_id: "100".to_string(),
        mod_name: "Wizard Fixture".to_string(),
        component_label: "Keep selected component".to_string(),
        raw_line: "~wizard-fixture.tp2~ #0 #100".to_string(),
        prompt_summary: Some("Keep answer".to_string()),
        prompt_events: Vec::new(),
        selected_order: 7,
        block_id: "wizard-block".to_string(),
        is_parent: false,
        parent_placeholder: false,
    }];
    wizard.step3.bgee_selected = vec![0];
    wizard.step3.bgee_locked_blocks = vec!["wizard-block".to_string()];
    wizard.step3.installer_answers = Some(InstallerAnswersEditor {
        targets: vec![InstallerAnswerTarget {
            game_tab: "BGEE".to_string(),
            package_path: "isolated/wizard-fixture".to_string(),
            row_index: 0,
            block_id: "wizard-block".to_string(),
            tp_file: "wizard-fixture".to_string(),
            component_id: "100".to_string(),
            language: 0,
            original_line: "~wizard-fixture.tp2~ #0 #100".to_string(),
            title: "Existing answer".to_string(),
            questions: "Choose value".to_string(),
            answers: vec![InstallerAnswerDraft {
                text: "preserved-answer".to_string(),
                blank_enter: false,
            }],
        }],
        ..InstallerAnswersEditor::default()
    });
}

#[test]
fn catalog_transfer_lease_distinguishes_descendants_and_sibling_prefixes() {
    use crate::app::app_step2_update_download;
    let root = FixtureRoot::new();
    let paths = destination(&root);
    let lease = app_step2_update_download::reserve_catalog_destination(
        &paths.mods_root,
        &paths.archives,
        &paths.backup_root,
    )
    .unwrap();
    let child = paths.mods_root.join("nested-mod");
    let sibling = root.join("mods2");
    assert!(app_step2_update_download::catalog_destination_busy(
        &child.to_string_lossy()
    ));
    assert!(app_step2_update_download::catalog_destination_busy(
        &root.0.to_string_lossy()
    ));
    assert!(!app_step2_update_download::catalog_destination_busy(
        &sibling.to_string_lossy()
    ));
    assert!(
        app_step2_update_download::reserve_catalog_destination(
            &child,
            &root.join("other-archives"),
            &root.join("other-backups")
        )
        .is_err()
    );
    drop(lease);
    assert!(!app_step2_update_download::catalog_destination_busy(
        &child.to_string_lossy()
    ));
}

#[test]
fn catalog_archive_collision_uses_windows_destination_identity() {
    use super::catalog_runtime::archive_destinations_conflict;
    use crate::app::app_step2_update_download::archive_file_name;
    let root = FixtureRoot::new();
    let mut first = fixture_asset();
    first.tp_file = "a b".to_string();
    first.tag = "v1".to_string();
    let mut alias = first.clone();
    alias.tp_file = "a-b".to_string();
    alias.tag = "V1".to_string();
    assert_ne!(archive_file_name(&first), archive_file_name(&alias));
    assert!(archive_destinations_conflict(
        &root.join("archives"),
        &[first.clone(), alias.clone()]
    ));
    alias.asset_url = "https://example.invalid/other.zip".to_string();
    assert!(archive_destinations_conflict(
        &root.join("archives"),
        &[first.clone(), alias.clone()]
    ));
    alias.tag = "v1".to_string();
    assert_eq!(archive_file_name(&first), archive_file_name(&alias));
    assert!(archive_destinations_conflict(
        &root.join("archives"),
        &[first.clone(), alias.clone()]
    ));
    alias.asset_url.clone_from(&first.asset_url);
    assert!(!archive_destinations_conflict(
        &root.join("archives"),
        &[first.clone(), alias.clone()]
    ));
    alias.tp_file = "completely-different".to_string();
    assert!(!archive_destinations_conflict(
        &root.join("archives"),
        &[first, alias]
    ));
}

fn catalog_shapes(shape: &egui::Shape, shapes: &mut Vec<egui::Shape>) {
    if let egui::Shape::Vec(children) = shape {
        for child in children {
            catalog_shapes(child, shapes);
        }
    } else {
        shapes.push(shape.clone());
    }
}

#[test]
fn catalog_painted_columns_status_dots_and_detail_link_match() {
    use crate::ui::shared::redesign_tokens::redesign_accent;
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for (palette_name, palette) in [("dark", ThemePalette::Dark), ("light", ThemePalette::Light)] {
        for width in [1570_u16, 1280, 1100, 900] {
            let size = egui::vec2(f32::from(width), 1002.0);
            let mut harness = catalog_scene(size, palette, "main");
            add_catalog_long_rows(&mut harness);
            harness.run();
            let mut shapes = Vec::new();
            for shape in &harness.output().shapes {
                catalog_shapes(&shape.shape, &mut shapes);
            }
            let bounds = |label: &str| {
                shapes
                    .iter()
                    .find_map(|shape| text_bounds(shape, label))
                    .unwrap_or_else(|| panic!("{palette_name} {width}: missing {label}"))
            };
            let source = bounds("Source");
            let status = bounds("Local status");
            let name = bounds("Mod");
            assert_catalog_clipping(&harness, source, status);
            assert_catalog_details_bounds(&shapes, width, harness.state().content_rect);
            assert_catalog_footer_and_primary(
                &shapes,
                width,
                palette,
                harness.state().content_rect,
            );
            assert_catalog_search_and_filters(&shapes, width, harness.state().content_rect);
            assert_catalog_count_clips(&harness, width);
            assert_catalog_footer_fits(&harness, false);
            assert!(
                name.right() + 16.0 < source.left(),
                "{palette_name} {width}: name/source headers {name:?} {source:?}"
            );
            assert!(
                source.right() + 16.0 < status.left(),
                "{palette_name} {width}: source/status headers {source:?} {status:?}"
            );
            let (source_rows, found, missing) = assert_catalog_row_paint(
                &shapes,
                name,
                source,
                status,
                palette,
                palette_name,
                width,
            );
            let link = bounds("Open selected source\u{f08e}");
            let accent = redesign_accent(palette);
            let link_text = shapes
                .iter()
                .find_map(|shape| match shape {
                    egui::Shape::Text(text)
                        if text.galley.text() == "Open selected source\u{f08e}" =>
                    {
                        Some(text)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(
                link_text
                    .galley
                    .job
                    .sections
                    .iter()
                    .all(|section| section.format.color == accent)
            );
            assert_eq!(link_text.galley.job.sections.len(), 2);
            assert_eq!(
                link_text.galley.job.sections[1].format.font_id.family,
                egui::FontFamily::Name("firacode_nerd".into())
            );
            println!(
                "PAINT {palette_name} {width}: Mod={name:?} Source={source:?} Local status={status:?} aligned_source_rows={source_rows} green_dots={found} muted_dots={missing} link={link:?} accent={accent:?}"
            );
            if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
                let directory = PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                harness
                    .render()
                    .unwrap()
                    .save(directory.join(format!("catalog-columns-{palette_name}-{width}.png")))
                    .unwrap();
            }
        }
    }
}

fn seed_catalog_scene_releases(scene: &mut CatalogScene) {
    if let Some(form) = scene
        .app
        .catalog_screen_state
        .editor_step2
        .versions_ui
        .source_form
        .as_ref()
    {
        let repo = form.repo.clone();
        scene
            .app
            .catalog_screen_state
            .editor_step2
            .versions_ui
            .release_list = crate::app::github_release_list::ReleaseListState {
            repo,
            status: crate::app::github_release_list::ReleaseListStatus::Ready(vec![
                fixture_release(),
            ]),
        };
    }
}

fn assert_catalog_row_paint(
    shapes: &[egui::Shape],
    name: egui::Rect,
    source: egui::Rect,
    status: egui::Rect,
    palette: ThemePalette,
    palette_name: &str,
    width: u16,
) -> (usize, usize, usize) {
    use crate::ui::shared::redesign_tokens::{redesign_success, redesign_text_muted};
    let mut source_rows = 0;
    let mut found = 0;
    let mut missing = 0;
    for shape in shapes {
        let egui::Shape::Text(text) = shape else {
            continue;
        };
        let label = text.galley.text();
        if text.pos.y <= source.bottom() || text.pos.x < name.left() || text.pos.x > status.right()
        {
            continue;
        }
        if ["GitHub", "Direct archives", "Weaselmods", "Morpheus Mart"].contains(&label) {
            assert!(
                (text.pos.x - source.left()).abs() < 1.0,
                "{palette_name} {width}: source {label} {} != {}",
                text.pos.x,
                source.left()
            );
            assert!(text.pos.x + text.galley.size().x + 4.0 < status.left());
            source_rows += 1;
        }
        if ["Found locally", "Not found"].contains(&label) && text.pos.x >= status.left() {
            let color = if label == "Found locally" {
                redesign_success(palette)
            } else {
                redesign_text_muted(palette)
            };
            let circle = shapes
                .iter()
                .find_map(|shape| match shape {
                    egui::Shape::Circle(circle)
                        if circle.fill == color
                            && circle.radius >= 2.5
                            && circle.radius <= 4.0
                            && (circle.center.y - (text.pos.y + text.galley.size().y / 2.0))
                                .abs()
                                < 2.0
                            && circle.center.x < text.pos.x
                            && (circle.center.x - status.left() - 4.0).abs() < 1.0 =>
                    {
                        Some(circle)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    panic!("{palette_name} {width}: missing aligned {label} dot at {text:?}")
                });
            assert!((text.pos.x - circle.center.x - 12.0).abs() < 3.0);
            if label == "Found locally" {
                found += 1;
            } else {
                missing += 1;
            }
        }
    }
    assert!(
        source_rows >= 3,
        "{palette_name} {width}: only {source_rows} source rows"
    );
    assert!(
        found > 0 && missing > 0,
        "{palette_name} {width}: found {found}, missing {missing}"
    );
    (source_rows, found, missing)
}
#[test]
fn catalog_detail_text_and_glyph_share_external_target() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1570.0, 1002.0), ThemePalette::Dark, "main");
    let bounds = harness
        .output()
        .shapes
        .iter()
        .find_map(|shape| text_bounds(&shape.shape, "Open selected source\u{f08e}"))
        .unwrap();
    for pos in [
        bounds.left_center() + egui::vec2(8.0, 0.0),
        bounds.right_center() - egui::vec2(4.0, 0.0),
    ] {
        let count = harness.state().urls.len();
        harness.input_mut().events.extend([
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        harness.run();
        assert_eq!(harness.state().urls.len(), count + 1);
        assert_eq!(
            harness.state().urls.last().unwrap(),
            "https://github.com/Fixture/The_Darkest_Day"
        );
    }
}

fn assert_catalog_clipping(
    harness: &Harness<'_, CatalogScene>,
    source: egui::Rect,
    status: egui::Rect,
) {
    let mut long_names = 0;
    let mut source_cells = 0;
    let mut status_cells = 0;
    for clipped in &harness.output().shapes {
        let mut shapes = Vec::new();
        catalog_shapes(&clipped.shape, &mut shapes);
        for shape in shapes {
            let egui::Shape::Text(text) = shape else {
                continue;
            };
            let label = text.galley.text();
            if (label.starts_with("A very long catalog name")
                || label.starts_with("AnExtremelyLongMaintainerIdentity"))
                && text.pos.x < source.left()
                && !label.contains('\n')
            {
                assert!(clipped.clip_rect.right() <= source.left() - 9.0);
                assert!(text.pos.x < clipped.clip_rect.right());
                long_names += 1;
            }
            if text.pos.y > source.bottom() && (text.pos.x - source.left()).abs() < 1.0 {
                assert!(clipped.clip_rect.right() <= status.left() - 9.0);
                assert!(text.pos.x + text.galley.size().x <= clipped.clip_rect.right() + 1.0);
                source_cells += 1;
            }
            if ["Found locally", "Not found"].contains(&label)
                && text.pos.x >= status.left()
                && text.pos.x <= status.right()
            {
                assert!(text.pos.x + text.galley.size().x <= clipped.clip_rect.right() + 1.0);
                status_cells += 1;
            }
        }
    }
    assert!(long_names > 0 && source_cells > 0 && status_cells > 0);
}

fn add_catalog_long_rows(harness: &mut Harness<'_, CatalogScene>) {
    let focused = harness
        .state_mut()
        .app
        .catalog_screen_state
        .rows
        .iter_mut()
        .find(|row| row.tp2 == "tdd")
        .unwrap();
    focused.sources[0].github = Some("AnExtremelyLongMaintainerIdentityForClipping/AnExtremelyLongRepositoryIdentityForTheSelectedDetailsPicker".to_string());
    focused.sources[0].branch =
        Some("AnExtremelyLongTrackingBranchNameThatMustNotExpandTheDetailsCard".to_string());
    let first = &mut harness.state_mut().app.catalog_screen_state.rows[0];
    first.name =
        "A very long catalog name in the first visible row with an unmistakable ending".to_string();
    first.sources[0].github =
        Some("AnExtremelyLongMaintainerIdentityForClipping/Fixture".to_string());
    let mut template = harness.state().app.catalog_screen_state.rows[0].clone();
    template
        .sources
        .extend(fixture_sources().into_iter().skip(1));
    for index in 0..1267 {
        let mut row = template.clone();
        row.tp2 = format!("long-fixture-{index}");
        row.name = format!(
            "A very long catalog name with repeated words and an unmistakable ending {index}"
        );
        for source in &mut row.sources {
            source.tp2.clone_from(&row.tp2);
        }
        row.sources[0].github =
            Some("AnExtremelyLongMaintainerIdentityForClipping/Fixture".to_string());
        row.sources[0].source_label = "A deliberately very long maintainer identity".to_string();
        harness
            .state_mut()
            .app
            .catalog_screen_state
            .local_presence
            .insert(row.tp2.clone(), LocalPresence::NotFound);
        harness.state_mut().app.catalog_screen_state.rows.push(row);
    }
}

fn assert_catalog_details_bounds(shapes: &[egui::Shape], width: u16, content: egui::Rect) {
    let right = content.right();
    let selected = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text)
                if text.galley.text().starts_with("GitHub · AnExtremelyLong")
                    && text.galley.rows.len() == 2 =>
            {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .unwrap();
    assert!(
        selected.right() <= right - 16.0,
        "{width}: selected source {selected:?} exceeds details content right"
    );
    let link = shapes
        .iter()
        .find_map(|shape| text_bounds(shape, "Open selected source\u{f08e}"))
        .unwrap();
    let mut card = false;
    let mut picker = false;
    for shape in shapes {
        let egui::Shape::Rect(rect) = shape else {
            continue;
        };
        if rect.stroke.width > 0.0 && rect.rect.contains(link.center()) && rect.rect.width() > 200.0
        {
            assert!(
                rect.rect.right() <= right + 1.0,
                "{width}: card {:?} exceeds page margin",
                rect.rect
            );
            card = true;
        }
        if rect.stroke.width > 0.0
            && rect.rect.contains(selected.center())
            && rect.rect.height() >= 52.0
            && rect.rect.height() <= 64.0
        {
            assert!(
                rect.rect.right() <= right - 16.0,
                "{width}: picker {:?} exceeds content right",
                rect.rect
            );
            picker = true;
        }
    }
    assert!(card && picker, "{width}: missing actual card/picker paint");
    println!(
        "DETAILS {width}: selected source {selected:?}; card<= {} picker<= {}",
        right + 1.0,
        right - 16.0
    );
}

fn assert_catalog_footer_and_primary(
    shapes: &[egui::Shape],
    width: u16,
    palette: ThemePalette,
    content: egui::Rect,
) {
    use crate::ui::shared::redesign_tokens::redesign_accent;
    let accent = redesign_accent(palette);
    let text_rect = |label: &str| {
        shapes
            .iter()
            .find_map(|shape| text_bounds(shape, label))
            .unwrap_or_else(|| panic!("{width}: missing {label}"))
    };
    let button_rect = |text: egui::Rect| {
        shapes
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(rect)
                    if rect.fill == accent && rect.rect.contains(text.center()) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .unwrap()
    };
    let link = text_rect("Open selected source\u{f08e}");
    let primary = button_rect(text_rect("Download this mod"));
    assert!(
        (primary.left() - link.left()).abs() < 1.0
            && (primary.right() - (content.right() - 17.0)).abs() < 1.0,
        "{width}: details primary not full card content width {primary:?}"
    );
    let count = text_rect("2 mods selected");
    let selected = button_rect(text_rect("Download selected (2)"));
    assert!(
        (selected.right() - content.right()).abs() < 1.0,
        "{width}: footer primary not right aligned {selected:?}"
    );
    let archive = text_rect("Download");
    let path = text_rect("Extract");
    let change = text_rect("Change\u{2026}");
    let clear = text_rect("Clear selection");
    assert!(count.right() < clear.left());
    assert!(clear.right() < archive.left() || clear.bottom() < archive.top());
    assert!(archive.right() < path.left());
    assert!(path.right() < change.left() || path.top() > change.bottom());
    assert!(change.right() < selected.left());
    assert!((archive.center().y - path.center().y).abs() < 2.0);
    if width >= 1280 {
        for rect in [count, clear, archive, path, change] {
            assert!((rect.center().y - selected.center().y).abs() < 2.0);
        }
    }
    for label in ["Change\u{2026}", "Clear selection"] {
        assert!(shapes.iter().any(|shape| {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => text
                    .galley
                    .job
                    .sections
                    .iter()
                    .all(|section| section.format.color == accent),
                _ => false,
            }
        }));
    }
    println!(
        "ACTIONS {width}: full details {primary:?}, footer right {selected:?}, count {count:?}, archive {archive:?}, extract {path:?}, change {change:?}, clear {clear:?}"
    );
}
#[test]
fn catalog_footer_links_clear_selection_and_open_paths() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for size in [
        egui::vec2(1570.0, 1002.0),
        egui::vec2(1100.0, 760.0),
        egui::vec2(900.0, 760.0),
    ] {
        let mut harness = catalog_scene(size, ThemePalette::Dark, "main");
        for label in ["Clear selection", "Change\u{2026}"] {
            let pos = harness
                .output()
                .shapes
                .iter()
                .find_map(|shape| text_bounds(&shape.shape, label))
                .unwrap()
                .center();
            harness.input_mut().events.extend([
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            harness.run();
            assert!(
                harness
                    .state()
                    .app
                    .catalog_screen_state
                    .selected_tp2
                    .is_empty()
            );
        }
        assert_eq!(harness.state().app.nav, NavDestination::Settings);
        assert_eq!(
            harness.state().app.settings_screen_state.active_tab,
            crate::ui::settings::state_settings::SettingsTab::Paths
        );
    }
}

fn assert_catalog_search_and_filters(shapes: &[egui::Shape], width: u16, content: egui::Rect) {
    let placeholder = shapes
        .iter()
        .find_map(|shape| text_bounds(shape, "Search mods, source labels or TP2\u{2026}"))
        .unwrap();
    let title = shapes
        .iter()
        .find_map(|shape| text_bounds(shape, "Mod catalog"))
        .unwrap();
    let search = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if rect.stroke.width > 0.0
                    && rect.rect.contains(placeholder.center())
                    && rect.rect.height() >= 36.0
                    && rect.rect.height() <= 40.0 =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{width}: missing full-height native search border"));
    assert!(
        search.left() >= title.left() && search.right() < content.right(),
        "{width}: search border {search:?} does not align to right page edge"
    );
    assert!((title.left() - content.left()).abs() < 1.0);
    assert!((title.top() - content.top()).abs() < 1.0);
    let magnifier = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Circle(circle)
                if (circle.radius - 6.0).abs() < 0.1
                    && (circle.stroke.width - 1.5).abs() < 0.1
                    && circle.center.x < placeholder.left()
                    && circle.center.x > placeholder.left() - 32.0
                    && (circle.center.y - placeholder.center().y).abs() < 8.0 =>
            {
                Some(circle)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing native search magnifier beside {placeholder:?}"));
    assert!(shapes.iter().any(|shape| {
        match shape {
            egui::Shape::LineSegment { points, stroke } => {
                stroke.color == magnifier.stroke.color
                    && points
                        .iter()
                        .all(|point| point.distance(magnifier.center) < 18.0)
                    && points.iter().any(|point| {
                        point.x > magnifier.center.x + 6.0 && point.y > magnifier.center.y + 6.0
                    })
            }
            _ => false,
        }
    }));
    for label in [
        "All (1271)",
        "GitHub (1269)",
        "Weaselmods (1268)",
        "Morpheus Mart (1268)",
        "Direct archives (1267)",
    ] {
        let text = shapes
            .iter()
            .find_map(|shape| text_bounds(shape, label))
            .unwrap();
        let rect = shapes
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(rect)
                    if rect.rect.contains(text.center())
                        && rect.rect.height() < 60.0
                        && rect.rect.width() >= text.width() =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .unwrap();
        assert!(rect.height() >= 32.0, "{label}: filter too small {rect:?}");
        assert!(
            rect.contains_rect(text),
            "{label}: {text:?} outside {rect:?}"
        );
        assert!(rect.right() <= content.right() + 1.0);
        println!("FILTER {width}: {label} text={text:?} widget={rect:?}");
    }
}

fn assert_catalog_count_clips(harness: &Harness<'_, CatalogScene>, width: u16) {
    let mut found = 0;
    for clipped in &harness.output().shapes {
        let mut shapes = Vec::new();
        catalog_shapes(&clipped.shape, &mut shapes);
        for shape in shapes {
            let egui::Shape::Text(text) = shape else {
                continue;
            };
            if [
                "All (1271)",
                "GitHub (1269)",
                "Weaselmods (1268)",
                "Morpheus Mart (1268)",
                "Direct archives (1267)",
            ]
            .contains(&text.galley.text())
            {
                let bounds = egui::Rect::from_min_size(text.pos, text.galley.size());
                assert!(
                    clipped.clip_rect.contains_rect(bounds),
                    "{width}: caption {bounds:?} outside clip {:?}",
                    clipped.clip_rect
                );
                found += 1;
            }
        }
    }
    assert_eq!(found, 5);
}

#[test]
fn catalog_counted_filter_clicks_preserve_global_captions_under_search() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let size = egui::vec2(1570.0, 1002.0);
    let mut harness = catalog_scene(size, ThemePalette::Dark, "main");
    for (label, kind, expected) in [
        ("GitHub (1)", CatalogKind::GitHub, vec!["tdd"]),
        ("Weaselmods (1)", CatalogKind::WeaselMods, vec!["weasel"]),
        ("Morpheus Mart (1)", CatalogKind::MorpheusMart, vec!["mart"]),
        (
            "Direct archives (1)",
            CatalogKind::DirectArchive,
            vec!["another"],
        ),
        (
            "All (4)",
            CatalogKind::All,
            vec!["another", "mart", "tdd", "weasel"],
        ),
    ] {
        let pos = harness
            .output()
            .shapes
            .iter()
            .find_map(|shape| text_bounds(&shape.shape, label))
            .unwrap()
            .center();
        harness.input_mut().events.extend([
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        harness.run();
        let state = &harness.state().app.catalog_screen_state;
        assert_eq!(state.kind, kind);
        let mut actual: Vec<_> = state
            .visible_rows()
            .iter()
            .map(|&index| state.rows[index].tp2.as_str())
            .collect();
        actual.sort_unstable();
        assert_eq!(actual, expected);
        harness.state_mut().app.catalog_screen_state.query = "no matching mod".to_string();
        harness.run();
        assert_eq!(
            harness.state().app.catalog_screen_state.visible_rows(),
            Vec::<usize>::new()
        );
        for caption in [
            "All (4)",
            "GitHub (1)",
            "Weaselmods (1)",
            "Morpheus Mart (1)",
            "Direct archives (1)",
        ] {
            assert_visible_text(&harness, caption, size);
        }
        harness.state_mut().app.catalog_screen_state.query.clear();
        harness.run();
    }
}

fn catalog_pointer_click(harness: &mut Harness<'_, CatalogScene>, pos: egui::Pos2) {
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    harness.run();
}

fn catalog_painted_text(harness: &Harness<'_, CatalogScene>, prefix: &str) -> egui::Rect {
    let mut shapes = Vec::new();
    for clipped in &harness.output().shapes {
        catalog_shapes(&clipped.shape, &mut shapes);
    }
    shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text)
                if text.galley.text().starts_with(prefix)
                    && (!["Download", "Extract"].contains(&prefix)
                        || text.galley.text() == prefix) =>
            {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing text prefix {prefix}"))
}

fn catalog_hover_text(harness: &mut Harness<'_, CatalogScene>, pos: egui::Pos2, expected: &str) {
    let count_expected = |harness: &Harness<'_, CatalogScene>| {
        let mut shapes = Vec::new();
        for clipped in &harness.output().shapes {
            catalog_shapes(&clipped.shape, &mut shapes);
        }
        shapes
            .iter()
            .filter(
                |shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == expected),
            )
            .count()
    };
    let before = count_expected(harness);
    harness.run_steps(12);
    harness.ctx.style_mut(|style| {
        style.interaction.tooltip_delay = 0.0;
        style.interaction.show_tooltips_only_when_still = false;
    });
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(pos));
    harness.run();
    harness.run();
    assert!(
        count_expected(harness) > before,
        "missing additional full tooltip {expected}"
    );
}

#[test]
fn catalog_native_picker_popup_and_settings_paths_remain_usable() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for palette in [ThemePalette::Dark, ThemePalette::Light] {
        for width in [1570_u16, 1100, 900] {
            let mut harness = catalog_scene(egui::vec2(f32::from(width), 1002.0), palette, "main");
            let mut alternate = fixture_sources()[1].clone();
            alternate.tp2 = "tdd".to_string();
            alternate.source_id = "direct-mirror".to_string();
            alternate.source_label = "Mirror archive".to_string();
            alternate.source_default = false;
            harness
                .state_mut()
                .app
                .catalog_screen_state
                .rows
                .iter_mut()
                .find(|row| row.tp2 == "tdd")
                .unwrap()
                .sources
                .push(alternate);
            harness.run();
            let picker = catalog_assert_picker_choice(&mut harness, width);
            catalog_assert_source_and_path_tooltips(&mut harness, width);
            println!(
                "PICKER_AND_PATHS {width}: native picker {picker:?}; popup direct choice selected, forks disabled; full source/archive/extract tooltips painted"
            );
        }
    }
}

fn catalog_assert_picker_choice(harness: &mut Harness<'_, CatalogScene>, width: u16) -> egui::Rect {
    let content = harness.state().content_rect;
    let selected = catalog_painted_text(harness, "GitHub · Fixture/The_Darkest_Day");
    let mut shapes = Vec::new();
    for clipped in &harness.output().shapes {
        catalog_shapes(&clipped.shape, &mut shapes);
    }
    let picker = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if rect.stroke.width > 0.0
                    && rect.rect.contains(selected.center())
                    && (52.0..=64.0).contains(&rect.rect.height()) =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .unwrap();
    let link = catalog_painted_text(harness, "Open selected source");
    assert!((picker.left() - link.left()).abs() < 1.0);
    assert!((picker.right() - (content.right() - 17.0)).abs() < 1.0);
    assert!(picker.contains_rect(selected));
    assert_visible_text(
        harness,
        "Source provided by: BIO catalog",
        egui::vec2(f32::from(width), 1002.0),
    );
    let painted = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if text.pos == selected.min => Some(text),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        painted.galley.text(),
        "GitHub · Fixture/The_Darkest_Day\nVersion: Newest release"
    );
    assert_eq!(painted.galley.rows.len(), 2);
    assert!(painted.galley.rows[0].rect.bottom() <= painted.galley.rows[1].rect.top());

    assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.pos == selected.min && text.galley.job.sections.iter().all(|section| (section.format.font_id.size - 14.0).abs() < f32::EPSILON && section.format.font_id.family == egui::FontFamily::Name("poppins_light".into())))));
    catalog_pointer_click(harness, picker.center());
    let mut popup = Vec::new();
    for clipped in &harness.output().shapes {
        catalog_shapes(&clipped.shape, &mut popup);
    }
    let option = popup
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text)
                if text.galley.text().contains("Mirror archive")
                    && text.galley.text().starts_with("BIO default") =>
            {
                assert!(
                    text.galley
                        .job
                        .sections
                        .iter()
                        .all(|section| (12.0..=14.0).contains(&section.format.font_id.size))
                );
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .unwrap();
    catalog_pointer_click(harness, option.center());
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "direct-mirror"
    );
    let forks = catalog_painted_text(harness, "Find forks");
    catalog_pointer_click(harness, forks.center());
    assert!(
        harness
            .state()
            .app
            .catalog_screen_state
            .editor_step2
            .versions_ui
            .sheet
            .is_none()
    );

    picker
}

fn catalog_assert_source_and_path_tooltips(harness: &mut Harness<'_, CatalogScene>, _width: u16) {
    let content = harness.state().content_rect;
    harness
        .state_mut()
        .app
        .catalog_screen_state
        .choose_source("tdd", "main");
    add_catalog_long_rows(harness);
    harness.run();
    let source = harness
        .state()
        .app
        .catalog_screen_state
        .source("tdd")
        .unwrap();
    let full_caption = format!(
        "GitHub · {}\nVersion: {}",
        source.github.as_deref().unwrap(),
        crate::app::versions_view::rule_words(source)
    );
    let selected = catalog_painted_text(harness, "GitHub · AnExtremelyLong");
    let mut selected_shapes = Vec::new();
    for clipped in &harness.output().shapes {
        catalog_shapes(&clipped.shape, &mut selected_shapes);
    }
    let painted = selected_shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if text.pos == selected.min => Some(text),
            _ => None,
        })
        .unwrap();
    let lines: Vec<_> = painted.galley.text().split('\n').collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("GitHub · AnExtremelyLong"));
    assert!(lines[1].starts_with("Version: Branch AnExtremelyLong"));
    assert_eq!(painted.galley.rows.len(), 2);
    assert!(painted.galley.rows[0].rect.bottom() <= painted.galley.rows[1].rect.top());
    assert!(selected.right() <= content.right() - 16.0);

    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(egui::Pos2::ZERO));
    harness.run();
    for (prefix, expected) in [
        (
            "Download",
            harness
                .state()
                .app
                .wizard_state
                .step1
                .mods_archive_folder
                .clone(),
        ),
        (
            "Extract",
            harness
                .state()
                .app
                .wizard_state
                .step1
                .global_mods_folder
                .clone(),
        ),
    ] {
        let label = catalog_painted_text(harness, prefix);
        let rect = catalog_painted_text(harness, &expected);
        assert!(rect.left() >= label.right() && rect.left() - label.right() < 32.0);
        assert!(rect.right() <= content.right() + 1.0 && rect.bottom() <= content.bottom() + 24.0);
        catalog_hover_text(harness, rect.center(), &expected);
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(egui::Pos2::ZERO));
        harness.run();
    }
    catalog_hover_text(harness, selected.center(), &full_caption);
}

#[test]
fn catalog_footer_running_cancel_and_status_fit_short_windows() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for palette in [ThemePalette::Dark, ThemePalette::Light] {
        for width in [1570_u16, 1100, 900] {
            let size = egui::vec2(f32::from(width), 760.0);
            let mut harness = catalog_scene(size, palette, "main");
            let token = Arc::new(AtomicBool::new(false));
            harness.state_mut().app.catalog_runtime.running = true;
            harness.state_mut().app.catalog_runtime.cancel_token = Some(Arc::clone(&token));
            harness.state_mut().app.catalog_screen_state.status =
                "Fixture transfer in progress".to_string();
            harness.run();
            assert_visible_text(&harness, "Cancel download", size);
            assert_visible_text(&harness, "Fixture transfer in progress", size);
            assert_catalog_footer_fits(&harness, true);
            let archive = catalog_painted_text(&harness, "Download");
            let extract = catalog_painted_text(&harness, "Extract");
            let cancel = catalog_painted_text(&harness, "Cancel download");
            assert!(
                (archive.center().y - extract.center().y).abs() < 2.0
                    && extract.bottom() < cancel.top()
            );
            catalog_pointer_click(&mut harness, cancel.center());
            assert!(token.load(Ordering::Relaxed));
            assert!(
                harness
                    .state()
                    .app
                    .catalog_screen_state
                    .status
                    .starts_with("Canceling")
            );
            println!(
                "SHORT_FOOTER {width}: Archive={archive:?}, Extract={extract:?}, Cancel={cancel:?}; native cancel sets isolated token"
            );
        }
    }
}

fn assert_catalog_footer_fits(harness: &Harness<'_, CatalogScene>, running: bool) {
    let content = harness.state().content_rect;
    let mut labels = vec![
        "2 mods selected",
        "Download",
        "Extract",
        "Download selected (2)",
    ];
    if running {
        labels.extend(["Cancel download", "Fixture transfer in progress"]);
    }
    let mut bottom = content.top();
    for label in labels {
        let mut found = false;
        for clipped in &harness.output().shapes {
            let mut shapes = Vec::new();
            catalog_shapes(&clipped.shape, &mut shapes);
            for shape in shapes {
                let egui::Shape::Text(text) = shape else {
                    continue;
                };
                if !text.galley.text().starts_with(label) {
                    continue;
                }
                let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                assert!(
                    rect.left() >= content.left()
                        && rect.right() <= content.right()
                        && rect.top() >= content.top()
                        && rect.bottom() <= content.bottom() + 24.0,
                    "{label}: {rect:?} outside visible Catalog footer {content:?}"
                );
                assert!(
                    clipped.clip_rect.contains_rect(rect),
                    "{label}: {rect:?} clipped by {:?}",
                    clipped.clip_rect
                );
                bottom = bottom.max(rect.bottom());
                found = true;
            }
        }
        assert!(found, "missing footer paint {label}");
    }
    let gap = content.bottom() + 24.0 - bottom;
    assert!(
        (0.0..=50.0).contains(&gap),
        "unused footer gap {gap}: content={content:?}"
    );
    if content.width() >= 850.0 {
        let header = harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| text_bounds(&clipped.shape, "Mod"))
            .unwrap();
        let count = catalog_painted_text(harness, "2 mods selected");
        let mut table_bottom = None;
        for clipped in &harness.output().shapes {
            let mut shapes = Vec::new();
            catalog_shapes(&clipped.shape, &mut shapes);
            for shape in shapes {
                if let egui::Shape::Rect(rect) = shape
                    && rect.stroke.width > 0.0
                    && rect.rect.contains(header.center())
                    && rect.rect.height() > 200.0
                {
                    table_bottom = Some(rect.rect.bottom());
                }
            }
        }
        let table_bottom = table_bottom.expect("missing expanded native table card");
        assert!(table_bottom <= count.top());
        assert!(
            count.top() - table_bottom <= 44.0,
            "unused space between table {table_bottom} and footer {count:?}"
        );
        println!(
            "SHARED_CONTENT {content:?}: expanded table bottom {table_bottom}; footer last paint {bottom}; gap {gap}; running={running}"
        );
    } else {
        println!(
            "SHARED_CONTENT {content:?}: stacked scroll; footer last paint {bottom}; gap {gap}; running={running}"
        );
    }
}

fn catalog_painted_card(harness: &Harness<'_, CatalogScene>, label: &str) -> egui::Rect {
    let anchor = harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| text_bounds(&clipped.shape, label))
        .unwrap_or_else(|| panic!("missing card label {label}"));
    let mut cards = Vec::new();
    for clipped in &harness.output().shapes {
        let mut shapes = Vec::new();
        catalog_shapes(&clipped.shape, &mut shapes);
        for shape in shapes {
            if let egui::Shape::Rect(rect) = shape
                && rect.stroke.width > 0.0
                && rect.rect.width() > 200.0
                && rect.rect.contains(anchor.center())
            {
                cards.push(rect.rect);
            }
        }
    }
    cards
        .into_iter()
        .max_by(|a, b| a.area().total_cmp(&b.area()))
        .unwrap_or_else(|| panic!("missing painted card around {label}"))
}

#[test]
fn catalog_details_card_matches_table_height_and_keeps_stacked_natural_height() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for (theme, palette) in [("dark", ThemePalette::Dark), ("light", ThemePalette::Light)] {
        for width in [1570_u16, 1280, 900] {
            for height in [1002_u16, 760] {
                for empty in [false, true] {
                    let mut harness = catalog_scene(
                        egui::vec2(f32::from(width), f32::from(height)),
                        palette,
                        "main",
                    );
                    if empty {
                        harness.state_mut().app.catalog_screen_state.focused_tp2 = None;
                        harness.run();
                    }
                    let table = catalog_painted_card(&harness, "Mod");
                    let label = if empty {
                        "Select a mod to see its download sources."
                    } else {
                        "Source provided by: BIO catalog"
                    };
                    let details = if width == 900 && height == 760 {
                        let mut shapes = Vec::new();
                        for clipped in &harness.output().shapes {
                            catalog_shapes(&clipped.shape, &mut shapes);
                        }
                        shapes
                            .iter()
                            .find_map(|shape| match shape {
                                egui::Shape::Rect(rect)
                                    if rect.stroke.width > 0.0
                                        && rect.rect.top() > table.bottom()
                                        && (rect.rect.width() - table.width()).abs() < 1.0 =>
                                {
                                    Some(rect.rect)
                                }
                                _ => None,
                            })
                            .expect("missing stacked painted details frame")
                    } else {
                        catalog_painted_card(&harness, label)
                    };
                    if width >= 1280 {
                        assert!(
                            (table.top() - details.top()).abs() <= 1.0,
                            "{width}x{height} {theme} empty={empty}: top mismatch table={table:?} details={details:?}"
                        );
                        assert!(
                            (table.bottom() - details.bottom()).abs() <= 1.0,
                            "{width}x{height} {theme} empty={empty}: bottom mismatch table={table:?} details={details:?}"
                        );
                        assert!(
                            details.bottom()
                                <= catalog_painted_text(&harness, "2 mods selected").top()
                        );
                    } else {
                        assert!(details.top() > table.bottom());
                        if empty {
                            assert!(
                                details.height() < 100.0,
                                "stacked empty card stretched: {details:?}"
                            );
                        }
                    }
                    assert_catalog_footer_fits(&harness, false);
                    println!(
                        "CARD_HEIGHT {theme} {width}x{height} empty={empty}: table={table:?}; details={details:?}"
                    );
                    if ((width == 1570 && height == 1002)
                        || (width == 1280 && height == 760 && !empty)
                        || (width == 900 && height == 1002 && empty))
                        && let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR")
                    {
                        let directory = PathBuf::from(directory);
                        std::fs::create_dir_all(&directory).unwrap();
                        harness
                            .render()
                            .unwrap()
                            .save(directory.join(format!(
                                "catalog-details-height-{theme}-{width}-{height}-empty-{empty}.png"
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn catalog_details_internal_scroll_keeps_painted_frame_and_footer_fixed() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for palette in [ThemePalette::Dark, ThemePalette::Light] {
        let mut harness = catalog_scene(egui::vec2(1280.0, 760.0), palette, "main");
        harness.state_mut().app.catalog_runtime.outcomes.insert(
            "tdd".to_string(),
            CatalogItemOutcome::Failed("Tall fixture line\n".repeat(80)),
        );
        harness.run();
        let frame = catalog_painted_card(&harness, "Source provided by: BIO catalog");
        let table = catalog_painted_card(&harness, "Mod");
        let footer = catalog_painted_text(&harness, "2 mods selected");
        let before = catalog_painted_text(&harness, "Download this mod");
        harness.input_mut().events.extend([
            egui::Event::PointerMoved(frame.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -240.0),
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        harness.run();
        harness.run_steps(12);
        let after = catalog_painted_text(&harness, "Download this mod");
        assert!(
            after.top() < before.top() - 20.0,
            "bounded details did not scroll: {before:?} -> {after:?}"
        );
        assert_eq!(catalog_painted_card(&harness, "Mod"), table);
        assert_eq!(catalog_painted_text(&harness, "2 mods selected"), footer);
        for clipped in &harness.output().shapes {
            let mut shapes = Vec::new();
            catalog_shapes(&clipped.shape, &mut shapes);
            for shape in shapes {
                if let egui::Shape::Text(text) = shape
                    && text.galley.text().contains("Tall fixture line")
                {
                    assert!(clipped.clip_rect.bottom() <= frame.bottom());
                    assert!(clipped.clip_rect.top() >= frame.top());
                }
            }
        }
        assert_catalog_footer_fits(&harness, false);
        println!(
            "INTERNAL_SCROLL frame={frame:?} table={table:?} footer={footer:?} before={before:?} after={after:?}"
        );
    }
}

fn catalog_capture_compact(harness: &mut Harness<'_, CatalogScene>, name: &str) {
    if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
        let directory = PathBuf::from(directory);
        assert!(directory.is_absolute());
        std::fs::create_dir_all(&directory).unwrap();
        harness
            .render()
            .unwrap()
            .save(directory.join(format!("{name}.png")))
            .unwrap();
    }
}

fn catalog_path_group(harness: &Harness<'_, CatalogScene>, label: &str, path: &str) -> egui::Rect {
    let caption = catalog_painted_text(harness, label);
    for clipped in &harness.output().shapes {
        let mut shapes = Vec::new();
        catalog_shapes(&clipped.shape, &mut shapes);
        for shape in shapes {
            if let egui::Shape::Text(text) = shape
                && text.galley.text() == path
            {
                assert!(
                    text.pos.x >= caption.right() && text.pos.x - caption.right() <= 32.0,
                    "{label}: caption={caption:?} path={:?}",
                    text.pos
                );
                assert!((text.pos.y + text.galley.size().y / 2.0 - caption.center().y).abs() < 2.0);
                assert!(text.pos.x + text.galley.size().x <= clipped.clip_rect.right() + 1.0);
                return egui::Rect::from_min_max(
                    caption.min,
                    egui::pos2(clipped.clip_rect.right(), caption.bottom()),
                );
            }
        }
    }
    panic!("missing painted path {path}");
}

#[test]
fn catalog_compact_equal_groups_resize_and_short_source_hover() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1280.0, 813.0), ThemePalette::Dark, "main");
    catalog_assert_short_source_hover(&mut harness);
    let archive = format!(
        "{}/archives-with-a-deliberately-long-path-that-must-truncate-independently",
        harness.state().root.0.display()
    );
    let extract = format!(
        "{}/mods-with-a-different-long-path-that-also-must-truncate-independently",
        harness.state().root.0.display()
    );
    harness
        .state_mut()
        .app
        .wizard_state
        .step1
        .mods_archive_folder
        .clone_from(&archive);
    harness
        .state_mut()
        .app
        .wizard_state
        .step1
        .global_mods_folder
        .clone_from(&extract);
    harness
        .state_mut()
        .app
        .catalog_screen_state
        .scan_destination = PathBuf::from(&extract);
    let generation = harness.state().app.catalog_screen_state.generation;
    let presence = harness
        .state()
        .app
        .catalog_screen_state
        .local_presence
        .clone();
    for width in [1570_u16, 1280, 900, 1570] {
        harness.set_size(egui::vec2(f32::from(width), 813.0));
        harness.run();
        let left = catalog_path_group(&harness, "Download", &archive);
        let right = catalog_path_group(&harness, "Extract", &extract);
        let clear = catalog_painted_text(&harness, "Clear selection");
        let change = catalog_painted_text(&harness, "Change\u{2026}");
        let action = catalog_painted_text(&harness, "Download selected (2)");
        assert!(
            (left.width() - right.width()).abs() <= 1.0,
            "{width}: unequal groups {left:?} {right:?}"
        );
        assert!(left.right() <= right.left());
        assert!(right.right() <= change.left() || right.top() > change.bottom());
        assert!(action.right() <= harness.state().content_rect.right());
        if width >= 1280 {
            assert!(clear.right() < left.left() && change.right() < action.left());
            assert!((left.center().y - action.center().y).abs() < 2.0);
        }
        assert_eq!(
            harness.state().app.catalog_screen_state.generation,
            generation
        );
        assert_eq!(
            harness.state().app.catalog_screen_state.local_presence,
            presence
        );
        assert_catalog_footer_fits(&harness, false);
        println!(
            "EQUAL_GROUPS {width}: {left:?} {right:?}; actions {clear:?} {change:?} {action:?}; scan generation {generation}"
        );
        if width != 1570 || left.width() > 0.0 {
            catalog_capture_compact(&mut harness, &format!("dark-longpaths-{width}x813"));
        }
    }
    catalog_assert_copied_paths(&mut harness, [&archive, &extract]);
    harness.state_mut().app.theme_palette = ThemePalette::Light;
    harness.set_size(egui::vec2(1280.0, 813.0));
    harness.run();
    catalog_capture_compact(&mut harness, "light-longpaths-1280x813");
}

fn catalog_assert_short_source_hover(harness: &mut Harness<'_, CatalogScene>) {
    let caption = "GitHub · Fixture/The_Darkest_Day\nVersion: Newest release";
    let selected = catalog_painted_text(harness, caption);
    catalog_capture_compact(harness, "dark-shortrepo-idle-1280x813");
    harness.ctx.style_mut(|style| {
        style.interaction.tooltip_delay = 0.0;
        style.interaction.show_tooltips_only_when_still = false;
    });
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(selected.center()));
    harness.run_steps(16);
    let mut captions = 0;
    for clipped in &harness.output().shapes {
        let mut shapes = Vec::new();
        catalog_shapes(&clipped.shape, &mut shapes);
        captions += shapes
            .iter()
            .filter(
                |shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == caption),
            )
            .count();
    }
    assert_eq!(
        captions, 1,
        "short source must not duplicate a caption tooltip"
    );
    catalog_capture_compact(harness, "dark-shortrepo-hover-1280x813");
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(egui::Pos2::ZERO));
}

fn catalog_assert_copied_paths(harness: &mut Harness<'_, CatalogScene>, paths: [&String; 2]) {
    for expected in paths {
        let pos = catalog_painted_text(harness, expected).center();
        harness.input_mut().events.extend([
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        harness.run();
        let copy = catalog_painted_text(harness, "Copy path");
        catalog_pointer_click(harness, copy.center());
        assert_eq!(harness.state().copied_paths.last(), Some(expected));
    }
}

#[test]
fn catalog_search_grows_with_live_resize_and_preserves_query() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1280.0, 813.0), ThemePalette::Dark, "main");
    harness.state_mut().app.catalog_screen_state.query = "resize query".to_owned();
    let mut inline = Vec::new();
    for width in [1280_u16, 1570, 900, 1280] {
        harness.set_size(egui::vec2(f32::from(width), 813.0));
        harness.run();
        let query = catalog_painted_text(&harness, "resize query");
        let search = catalog_painted_card(&harness, "resize query");
        let labels = [
            "All (",
            "GitHub (",
            "Weaselmods (",
            "Morpheus Mart (",
            "Direct archives (",
        ];
        let tabs = labels.map(|label| catalog_painted_text(&harness, label));
        let content = harness.state().content_rect;
        let filter = catalog_painted_text(&harness, "Filter");
        assert!(filter.right() <= content.right());
        assert!(filter.left() > search.right());
        assert!((filter.center().y - search.center().y).abs() < 2.0);
        assert_eq!(
            harness.state().app.catalog_screen_state.query,
            "resize query"
        );
        assert!(search.contains(query.center()));
        if width >= 1280 {
            let last = tabs[4];
            assert!((search.center().y - last.center().y).abs() < 2.0);
            assert!(search.left() > last.right());
            assert!(search.left() - last.right() < 40.0);
            inline.push((search, tabs));
        } else {
            assert!((search.left() - content.left()).abs() < 1.0);
            assert!(search.width() > content.width() - 140.0);
            assert!(tabs.iter().all(|tab| tab.bottom() < search.top()));
        }
        println!("SEARCH_RESIZE {width}: search={search:?} tabs={tabs:?} content={content:?}");
    }
    let [(normal, tabs), (wide, wide_tabs), (restored, restored_tabs)] = inline.as_slice() else {
        panic!("missing inline resize measurements");
    };
    assert!((wide.width() - normal.width() - 290.0).abs() < 1.0);
    assert!((wide.left() - normal.left()).abs() < 1.0);
    assert_eq!(tabs, wide_tabs);
    assert_eq!(tabs, restored_tabs);
    assert_eq!(normal, restored);
}

#[test]
fn catalog_footer_paint_is_centered_above_visible_shell_bottom() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for width in [1280_u16, 1570, 900] {
        let size = egui::vec2(f32::from(width), 813.0);
        let mut harness = catalog_scene(size, ThemePalette::Dark, "main");
        harness.run();
        let primary = catalog_painted_text(&harness, "Download selected (2)");
        let content = harness.state().content_rect;
        let mut button = None;
        let mut separator = None;
        for clipped in &harness.output().shapes {
            let mut shapes = Vec::new();
            catalog_shapes(&clipped.shape, &mut shapes);
            for shape in shapes {
                match shape {
                    egui::Shape::Rect(rect)
                        if rect.rect.contains(primary.center())
                            && rect.rect.height() < 80.0
                            && rect.rect.width() > 100.0 =>
                    {
                        assert!(clipped.clip_rect.contains_rect(rect.rect));
                        println!("FOOTER_CLIP {width}: {:?}", clipped.clip_rect);
                        button = Some(rect.rect);
                    }
                    egui::Shape::LineSegment { points, .. }
                        if (points[0].y - points[1].y).abs() < 1.0
                            && (points[0].x - points[1].x).abs() > content.width() - 4.0
                            && points[0].y < primary.top() =>
                    {
                        separator =
                            Some(separator.map_or(points[0].y, |y: f32| y.max(points[0].y)));
                    }
                    _ => {}
                }
            }
        }
        let button = button.expect("missing primary button paint");
        let separator = separator.expect("missing footer separator paint");
        assert!(button.left() >= content.left() && button.right() <= content.right());
        assert!(button.bottom() <= size.y);
        let download = catalog_painted_text(&harness, "Download");
        let extract = catalog_painted_text(&harness, "Extract");
        let bottom = button.bottom().max(download.bottom()).max(extract.bottom());
        let above = button.top() - separator;
        let below = size.y - bottom;
        println!(
            "FOOTER_CENTER {width}: separator={separator}, button={button:?}, content={content:?}, visible_bottom={}, above={above}, below={below}",
            size.y
        );
        assert!((content.bottom() + 24.0 - size.y).abs() < 1.0);
        assert!(
            (above - below).abs() <= 3.0,
            "{width}: footer padding above={above}, below={below}"
        );
        if width >= 1280 {
            for label in [
                "2 mods selected",
                "Clear selection",
                "Download",
                "Extract",
                "Change",
            ] {
                let painted = catalog_painted_text(&harness, label);
                assert!(
                    (painted.center().y - button.center().y).abs() < 2.0,
                    "{width}: {label} not centered on primary button"
                );
            }
        }
        assert_catalog_footer_fits(&harness, false);
    }
}

fn catalog_picker_saved_fixture() -> (FixtureRoot, CatalogScreenState, CatalogRuntime) {
    let root = FixtureRoot::new();
    mod_downloads::set_active_modlist_dir(None);
    let path = mod_downloads::mod_downloads_user_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, source_text(&fixture_sources())).unwrap();
    let mut state = fixture_state();
    state.choose_source("tdd", "main");
    (root, state, CatalogRuntime::default())
}

#[test]
fn catalog_picker_saved_fork_reload_drives_next_request_and_manual_choice() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let (root, mut state, mut runtime) = catalog_picker_saved_fixture();
    runtime.open_forks(&mut state, "tdd");
    state.accept_forks("Fixture/The_Darkest_Day", Ok(vec![fixture_fork()]));
    runtime.use_fork(&mut state, &fixture_fork());
    let form = state.editor_step2.versions_ui.source_form.as_mut().unwrap();
    form.source_id = "MiXeD-Community".to_owned();
    form.follow = Follow::Release;
    form.release = "v7-offline".to_owned();
    form.asset = AssetPick::Named("community-win.zip".to_owned());
    runtime.save_editor(&mut state).unwrap();
    runtime.ensure_loaded(&mut state, &destination(&root));
    let saved = state.source("tdd").unwrap();
    assert_eq!(saved.github.as_deref(), Some("Community/The_Darkest_Day"));
    assert_eq!(
        mod_downloads::normalize_source_id(&saved.source_id),
        "mixed-community"
    );
    assert_eq!(
        state.tiers.tier_of("tdd", &saved.source_id),
        mod_downloads::SourceTier::User
    );
    for keys in [
        BTreeSet::from(["tdd".to_owned()]),
        BTreeSet::from(["tdd".to_owned(), "another".to_owned()]),
    ] {
        let snapshot = state.snapshot(&keys, destination(&root), false).unwrap();
        let source = snapshot
            .sources
            .iter()
            .find(|source| source.tp2 == "tdd")
            .unwrap();
        let request = super::catalog_runtime::resolver_request(source);
        assert_eq!(request.repo, "Community/The_Darkest_Day");
        assert_eq!(request.release.as_deref(), Some("v7-offline"));
        assert_eq!(request.asset.as_deref(), Some("community-win.zip"));
    }
    state.choose_source("tdd", "main");
    runtime.loaded = false;
    runtime.ensure_loaded(&mut state, &destination(&root));
    assert_eq!(state.source("tdd").unwrap().source_id, "main");
    assert!(
        state
            .rows
            .iter()
            .find(|row| row.tp2 == "tdd")
            .unwrap()
            .sources
            .iter()
            .any(
                |source| mod_downloads::normalize_source_id(&source.source_id) == "mixed-community"
            )
    );
}

#[test]
fn catalog_picker_failed_save_and_cancel_keep_selection_and_bytes() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let (_root, mut state, mut runtime) = catalog_picker_saved_fixture();
    let path = mod_downloads::mod_downloads_user_path();
    let bytes = std::fs::read(&path).unwrap();
    runtime.open_forks(&mut state, "tdd");
    runtime.use_fork(&mut state, &fixture_fork());
    runtime.dismiss_editor(&mut state);
    assert_eq!(state.source("tdd").unwrap().source_id, "main");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    runtime.open_forks(&mut state, "tdd");
    runtime.use_fork(&mut state, &fixture_fork());
    std::fs::write(&path, "invalid = [").unwrap();
    assert!(runtime.save_editor(&mut state).is_err());
    assert_eq!(state.source("tdd").unwrap().source_id, "main");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid = [");
    assert!(
        state
            .editor_step2
            .versions_ui
            .source_form
            .as_ref()
            .unwrap()
            .error
            .is_some()
    );
}

fn catalog_picker_field(harness: &Harness<'_, CatalogScene>) -> egui::Rect {
    let mut shapes = Vec::new();
    for clipped in &harness.output().shapes {
        catalog_shapes(&clipped.shape, &mut shapes);
    }
    let caption = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if text.galley.text().contains("\nVersion:") => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .expect("closed two-line source caption");
    shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if rect.stroke.width > 0.0
                    && rect.rect.contains(caption.center())
                    && (52.0..=64.0).contains(&rect.rect.height()) =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .expect("source field frame")
}

fn catalog_picker_seed_options(harness: &mut Harness<'_, CatalogScene>, count: usize) {
    let row = harness
        .state_mut()
        .app
        .catalog_screen_state
        .rows
        .iter_mut()
        .find(|row| row.tp2 == "tdd")
        .unwrap();
    let mut base = github_source();
    base.github = Some("AchievementsMaintainerWithAnExtremelyLongName/AchievementsRepositoryWithAnExtremelyLongNameAndUnicode世界".to_owned());
    base.url = format!("https://github.com/{}", base.github.as_deref().unwrap());
    base.branch =
        Some("supported-branch-with-an-extremely-long-reference-and-世界-ending".to_owned());
    base.source_label = "Achievements original maintainer 世界".to_owned();
    row.sources = (0..count).map(|index| {
        let mut source = base.clone();
        if index > 0 {
            source.source_id = format!("choice-{index}");
            source.source_default = false;
            source.source_label = format!("Achievements alternative {index} 世界");
            if index % 2 == 1 {
                source.github = None;
                source.url = format!("https://example.invalid/achievements/a-very-long-direct-download-location/world-世界/choice-{index}.zip");
                source.kind = Some("url".to_owned());
            }
        }
        source
    }).collect();
    harness.run();
}

fn catalog_picker_assert_open_bounds(
    harness: &Harness<'_, CatalogScene>,
    field: egui::Rect,
    size: egui::Vec2,
) {
    let mut count = 0;
    for clipped in &harness.output().shapes {
        let mut shapes = Vec::new();
        catalog_shapes(&clipped.shape, &mut shapes);
        for shape in shapes {
            let egui::Shape::Text(text) = shape else {
                continue;
            };
            if !text.galley.text().contains("BIO default")
                && !text.galley.text().contains("My default")
            {
                continue;
            }
            let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
            if rect.intersects(field) || !clipped.clip_rect.intersects(rect) {
                continue;
            }
            assert!(
                rect.left() >= field.left() - 8.0 && rect.right() <= field.right() + 8.0,
                "popup text {rect:?} exceeds source field {field:?}: {}",
                text.galley.text()
            );
            assert!(
                egui::Rect::from_min_size(egui::Pos2::ZERO, size)
                    .contains_rect(rect.intersect(clipped.clip_rect)),
                "visible popup text outside viewport {rect:?}"
            );
            if count == 0 {
                assert!(
                    clipped.clip_rect.contains_rect(rect),
                    "popup first row cropped: {rect:?} clip {:?}",
                    clipped.clip_rect
                );
            }
            assert!(text.galley.rows.len() <= 3, "unbounded popup row lines");
            count += 1;
        }
    }
    assert!(count > 0, "no visible native popup origin text");
}

#[test]
fn catalog_picker_open_two_long_sources_bounds_red() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let size = egui::vec2(1100.0, 813.0);
    let mut harness = catalog_scene(size, ThemePalette::Dark, "main");
    catalog_picker_seed_options(&mut harness, 2);
    let field = catalog_picker_field(&harness);
    catalog_pointer_click(&mut harness, field.center());
    catalog_picker_assert_open_bounds(&harness, field, size);
}

#[test]
fn catalog_picker_existing_edit_blank_id_and_transient_reload() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for new_fork in [false, true] {
        let (root, mut state, mut runtime) = catalog_picker_saved_fixture();
        if new_fork {
            runtime.open_forks(&mut state, "tdd");
            runtime.use_fork(&mut state, &fixture_fork());
        } else {
            state.open_editor("tdd");
        }
        let form = state.editor_step2.versions_ui.source_form.as_mut().unwrap();
        form.source_id = if new_fork {
            String::new()
        } else {
            " MAIN ".to_owned()
        };
        form.repo = "Saved/EditedSource".to_owned();
        form.follow = Follow::Branch;
        form.branch = "edited-rule".to_owned();
        runtime.save_editor(&mut state).unwrap();
        let path = mod_downloads::mod_downloads_user_path();
        let parked = root.join("parked-personal.toml");
        std::fs::rename(&path, &parked).unwrap();
        std::fs::create_dir(&path).unwrap();
        runtime.ensure_loaded(&mut state, &destination(&root));
        assert!(state.error.is_some());
        std::fs::remove_dir(&path).unwrap();
        std::fs::rename(&parked, &path).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        runtime.ensure_loaded(&mut state, &destination(&root));
        assert!(state.error.is_none(), "{:?}", state.error);
        let source = state.source("tdd").unwrap();
        assert_eq!(source.github.as_deref(), Some("Saved/EditedSource"));
        assert_eq!(source.branch.as_deref(), Some("edited-rule"));
        assert_eq!(
            mod_downloads::normalize_source_id(&source.source_id),
            if new_fork { "primary" } else { "main" }
        );
    }
}

fn catalog_picker_capture(harness: &mut Harness<'_, CatalogScene>, name: &str) {
    if let Some(directory) = std::env::var_os("BIO_PICKER_SCREENSHOT_DIR") {
        let directory = PathBuf::from(directory);
        assert!(directory.is_absolute());
        std::fs::create_dir_all(&directory).unwrap();
        harness
            .render()
            .unwrap()
            .save(directory.join(format!("picker-{name}.png")))
            .unwrap();
    }
}

fn catalog_picker_popup_text(
    harness: &Harness<'_, CatalogScene>,
    needle: &str,
) -> Option<egui::Rect> {
    for clipped in &harness.output().shapes {
        let mut shapes = Vec::new();
        catalog_shapes(&clipped.shape, &mut shapes);
        for shape in shapes {
            if let egui::Shape::Text(text) = shape
                && text.galley.text().contains(needle)
                && (text.galley.text().contains("BIO default")
                    || text.galley.text().contains("My default"))
            {
                let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                if clipped.clip_rect.contains_rect(rect) {
                    return Some(rect);
                }
            }
        }
    }
    None
}

fn catalog_picker_wheel(harness: &mut Harness<'_, CatalogScene>, pos: egui::Pos2, delta: f32) {
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, delta),
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    harness.run();
    harness.run_steps(12);
}

#[test]
fn catalog_picker_native_open_resize_themes_and_achievements() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1280.0, 813.0), ThemePalette::Dark, "main");
    let mut defaults = fixture_sources();
    defaults[0].github = Some("Argent77/A7-Achievements".to_owned());
    defaults[0].url = "https://github.com/Argent77/A7-Achievements".to_owned();
    let mut personal = defaults[0].clone();
    personal.source_id = "gwendolyne".to_owned();
    personal.source_label = "GwendolyneFreddy".to_owned();
    personal.github = Some("GwendolyneFreddy/A7-Achievements".to_owned());
    personal.url = "https://github.com/GwendolyneFreddy/A7-Achievements".to_owned();
    personal.source_default = false;
    harness
        .state_mut()
        .app
        .catalog_screen_state
        .replace_from_texts(&source_text(&defaults), &source_text(&[personal]));
    harness.run();
    let field = catalog_picker_field(&harness);
    catalog_pointer_click(&mut harness, field.center());
    for (width, height, name) in [
        (1280_u16, 813_u16, "achievements-default-dark"),
        (1570, 1002, "achievements-wide-dark"),
        (1100, 813, "achievements-resize-stacked-dark"),
        (900, 1002, "achievements-narrow-dark"),
    ] {
        let size = egui::vec2(f32::from(width), f32::from(height));
        harness.set_size(size);
        harness.run();
        assert!(
            harness.state().popup_open,
            "popup closed on resize to {width}x{height}"
        );
        catalog_picker_assert_open_bounds(&harness, catalog_picker_field(&harness), size);
        assert!(
            catalog_picker_popup_text(&harness, "BIO default").is_some(),
            "complete original row required"
        );
        assert!(
            catalog_picker_popup_text(&harness, "My default").is_some(),
            "complete personal row required"
        );
        catalog_picker_capture(&mut harness, name);
    }
    harness.state_mut().app.theme_palette = ThemePalette::Light;
    let size = egui::vec2(1280.0, 760.0);
    harness.set_size(size);
    harness.run();
    catalog_picker_assert_open_bounds(&harness, catalog_picker_field(&harness), size);
    assert!(catalog_picker_popup_text(&harness, "BIO default").is_some());
    assert!(catalog_picker_popup_text(&harness, "My default").is_some());
    catalog_picker_capture(&mut harness, "achievements-short-light");
    let option = catalog_picker_popup_text(&harness, "GwendolyneFreddy").unwrap();
    catalog_pointer_click(&mut harness, option.center());
    assert!(!harness.state().popup_open);
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "gwendolyne"
    );
}

#[test]
fn catalog_picker_native_many_scroll_dismiss_and_stale_target() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let size = egui::vec2(1280.0, 813.0);
    let mut harness = catalog_scene(size, ThemePalette::Dark, "main");
    catalog_picker_seed_options(&mut harness, 24);
    let original = harness
        .state()
        .app
        .catalog_screen_state
        .rows
        .iter()
        .find(|row| row.tp2 == "tdd")
        .unwrap()
        .sources
        .clone();
    let selected = harness
        .state()
        .app
        .catalog_screen_state
        .selected_tp2
        .clone();
    let table = catalog_painted_card(&harness, "Mod");
    let field = catalog_picker_field(&harness);
    catalog_pointer_click(&mut harness, field.center());
    catalog_picker_assert_open_bounds(&harness, field, size);
    catalog_picker_capture(&mut harness, "many-first-dark");
    let full = format!(
        "BIO default · {}\nGitHub · {}\nVersion: {}",
        original[0].source_label,
        original[0].github.as_deref().unwrap(),
        crate::app::versions_view::rule_words(&original[0])
    );
    let hovered = catalog_picker_popup_text(&harness, "BIO default").unwrap();
    catalog_hover_text(&mut harness, hovered.center(), &full);
    let origin = catalog_picker_popup_text(&harness, "BIO default")
        .unwrap()
        .center();
    catalog_picker_wheel(&mut harness, origin, -4000.0);
    let last = catalog_picker_popup_text(&harness, "Achievements alternative 23")
        .expect("last source reached by native wheel");
    assert_eq!(catalog_painted_card(&harness, "Mod"), table);
    assert_eq!(
        harness.state().app.catalog_screen_state.selected_tp2,
        selected
    );
    catalog_picker_capture(&mut harness, "many-last-dark");
    catalog_picker_wheel(&mut harness, last.center(), 4000.0);
    assert!(
        catalog_picker_popup_text(&harness, "Achievements original").is_some(),
        "first row unreachable after scrolling back"
    );
    let first = catalog_picker_popup_text(&harness, "Achievements original").unwrap();
    catalog_picker_wheel(&mut harness, first.center(), -4000.0);
    let last = catalog_picker_popup_text(&harness, "Achievements alternative 23").unwrap();
    catalog_pointer_click(&mut harness, last.center());
    assert!(!harness.state().popup_open);
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "choice-23"
    );
    catalog_picker_assert_dismiss_and_target(&mut harness, &original);
}

#[test]
fn catalog_picker_native_cardinality_keyboard_and_disappearance() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1280.0, 813.0), ThemePalette::Dark, "main");
    catalog_picker_seed_options(&mut harness, 1);
    let field = catalog_picker_field(&harness);
    catalog_pointer_click(&mut harness, field.center());
    assert!(harness.state().popup_open);
    let current = catalog_picker_popup_text(&harness, "BIO default").unwrap();
    catalog_pointer_click(&mut harness, current.center());
    assert!(!harness.state().popup_open);
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "main"
    );
    catalog_picker_assert_keyboard_reopen(&mut harness);
    catalog_pointer_click(&mut harness, field.center());
    harness.state_mut().app.catalog_screen_state.focused_tp2 = None;
    harness.run();
    harness.state_mut().app.catalog_screen_state.focus("tdd");
    harness.run();
    assert!(
        !harness.state().popup_open,
        "hidden source field resurrected an old open menu"
    );
    catalog_picker_seed_options(&mut harness, 0);
    harness.run();
    assert!(!harness.state().popup_open);
    assert!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .is_none()
    );
    let mut page = fixture_sources()[2].clone();
    page.tp2 = "tdd".to_owned();
    harness
        .state_mut()
        .app
        .catalog_screen_state
        .rows
        .iter_mut()
        .find(|row| row.tp2 == "tdd")
        .unwrap()
        .sources
        .push(page);
    harness.run();
    let page_field = catalog_picker_field(&harness);
    catalog_pointer_click(&mut harness, page_field.center());
    assert!(harness.state().popup_open);
    let page_choice = catalog_picker_popup_text(&harness, "BIO default").unwrap();
    catalog_pointer_click(&mut harness, page_choice.center());
    assert!(!harness.state().popup_open);
    assert!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .github
            .is_none()
    );
    let forks = catalog_painted_text(&harness, "Find forks");
    catalog_pointer_click(&mut harness, forks.center());
    assert!(
        harness
            .state()
            .app
            .catalog_screen_state
            .editor_step2
            .versions_ui
            .sheet
            .is_none()
    );
    assert!(!harness.state().app.catalog_runtime.running);
    assert_eq!(harness.state().urls, Vec::<String>::new());
}

fn catalog_picker_assert_dismiss_and_target(
    harness: &mut Harness<'_, CatalogScene>,
    original: &[ModDownloadSource],
) {
    let field = catalog_picker_field(harness);
    catalog_pointer_click(harness, field.center());
    harness.press_key(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().popup_open);
    catalog_pointer_click(harness, field.center());
    catalog_pointer_click(harness, egui::pos2(210.0, 25.0));
    assert!(!harness.state().popup_open);
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "choice-23"
    );
    catalog_pointer_click(harness, field.center());
    harness
        .state_mut()
        .app
        .catalog_screen_state
        .focus("another");
    harness.run();
    let stale = catalog_picker_popup_text(harness, "Achievements alternative 23");
    assert!(
        stale.is_none(),
        "old mod popup still interactable after focus switch"
    );
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("another")
            .unwrap()
            .source_id,
        "main"
    );
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .rows
            .iter()
            .find(|row| row.tp2 == "tdd")
            .unwrap()
            .sources,
        original
    );
    assert_eq!(harness.state().urls, Vec::<String>::new());
    assert!(!harness.state().app.catalog_runtime.running);
}

fn catalog_picker_assert_keyboard_reopen(harness: &mut Harness<'_, CatalogScene>) {
    harness.press_key(egui::Key::Space);
    harness.run();
    assert!(
        harness.state().popup_open,
        "Space must activate focused native source field"
    );
    println!(
        "KEYBOARD before arrow focus={:?}",
        harness.state().focused_widget
    );
    harness.press_key(egui::Key::ArrowDown);
    harness.run();
    harness.press_key(egui::Key::Enter);
    harness.run();
    println!(
        "NATIVE_KEYBOARD single-option ArrowDown+Enter popup_open={} focus={:?}",
        harness.state().popup_open,
        harness.state().focused_widget
    );
    if harness.state().popup_open {
        harness.press_key(egui::Key::Tab);
        harness.run();
        harness.press_key(egui::Key::Enter);
        harness.run();
    }
    println!(
        "NATIVE_KEYBOARD Tab+Enter popup_open={} focus={:?}",
        harness.state().popup_open,
        harness.state().focused_widget
    );
    assert!(
        !harness.state().popup_open,
        "native keyboard selection must close popup"
    );
}

#[test]
fn catalog_picker_native_keyboard_alternatives_and_bounds() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1280.0, 813.0), ThemePalette::Dark, "main");
    catalog_picker_seed_options(&mut harness, 3);
    let field = catalog_picker_field(&harness);
    catalog_pointer_click(&mut harness, field.center());
    harness.press_key(egui::Key::Tab);
    harness.run();
    harness.press_key(egui::Key::Enter);
    harness.run();
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "choice-1"
    );
    catalog_pointer_click(&mut harness, field.center());
    harness.press_key_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    harness.run();
    harness.press_key(egui::Key::Enter);
    harness.run();
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "main"
    );
    catalog_pointer_click(&mut harness, field.center());
    for key in [
        egui::Key::ArrowUp,
        egui::Key::ArrowDown,
        egui::Key::ArrowDown,
        egui::Key::ArrowDown,
        egui::Key::Enter,
    ] {
        harness.press_key(key);
        harness.run();
    }
    assert!(!harness.state().popup_open);
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "choice-2"
    );
    assert_eq!(harness.state().urls, Vec::<String>::new());
    assert!(!harness.state().app.catalog_runtime.running);
}

fn catalog_picker_complete_rows(harness: &Harness<'_, CatalogScene>, count: usize) -> usize {
    (0..count)
        .filter(|index| {
            let needle = if *index == 0 {
                "Achievements original".to_owned()
            } else {
                format!("Achievements alternative {index} ")
            };
            catalog_picker_popup_text(harness, &needle).is_some()
        })
        .count()
}

#[test]
fn catalog_popup_cached_cardinality_complete_rows() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1280.0, 1002.0), ThemePalette::Dark, "main");
    for count in [1, 2, 3, 4, 24, 1] {
        catalog_picker_seed_options(&mut harness, count);
        let field = catalog_picker_field(&harness);
        catalog_pointer_click(&mut harness, field.center());
        assert!(harness.state().popup_open);
        assert_eq!(
            catalog_picker_complete_rows(&harness, count),
            count.min(3),
            "complete visible rows after cached cardinality changes to {count}"
        );
        if count == 4 {
            catalog_picker_capture(&mut harness, "four-sources-three-visible");
        }
        if count == 24 {
            catalog_picker_capture(&mut harness, "rows-capped-three");
            let first = catalog_picker_popup_text(&harness, "Achievements original").unwrap();
            catalog_picker_wheel(&mut harness, first.center(), -4000.0);
            assert!(catalog_picker_popup_text(&harness, "Achievements alternative 23").is_some());
        }
        harness.press_key(egui::Key::Escape);
        harness.run();
        assert!(!harness.state().popup_open);
    }
}

fn catalog_ui_editor(harness: &mut Harness<'_, CatalogScene>) {
    mod_downloads::set_active_modlist_dir(None);
    let path = mod_downloads::mod_downloads_user_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, source_text(&fixture_sources())).unwrap();
    let edit = catalog_painted_text(harness, "Edit source");
    catalog_pointer_click(harness, edit.center());
    assert!(
        harness
            .state()
            .app
            .catalog_screen_state
            .editor_step2
            .versions_ui
            .source_form
            .is_some()
    );
}

#[test]
fn catalog_ui_save_one_toast_preserves_footer() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for status in ["", "Finished: 1 completed, 1 failed, 0 canceled"] {
        let mut harness = catalog_scene(egui::vec2(1280.0, 813.0), ThemePalette::Dark, "main");
        harness.state_mut().app.catalog_screen_state.status = status.to_owned();
        harness.run();
        let footer = catalog_painted_text(&harness, "2 mods selected");
        let history = harness.state().app.notification_manager.history().len();
        catalog_ui_editor(&mut harness);
        let save = catalog_painted_text(&harness, "Save to My default");
        catalog_pointer_click(&mut harness, save.center());
        let app = &harness.state().app;
        assert!(
            app.catalog_screen_state
                .editor_step2
                .versions_ui
                .source_form
                .is_none()
        );
        assert_eq!(app.catalog_screen_state.status, status);
        assert_eq!(app.notification_manager.history().len(), history + 1);
        let record = app.notification_manager.history().back().unwrap();
        assert_eq!(record.text, "Saved to My default");
        assert!(matches!(record.kind, egui_toast::ToastKind::Success));
        assert_eq!(catalog_painted_text(&harness, "2 mods selected"), footer);
        harness.run_steps(12);
        assert_eq!(
            harness.state().app.notification_manager.history().len(),
            history + 1
        );
        assert_eq!(harness.state().app.catalog_screen_state.status, status);
        assert_eq!(catalog_painted_text(&harness, "2 mods selected"), footer);
    }
}

#[test]
fn catalog_ui_save_cancel_failure_and_note_warning() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for outcome in ["cancel", "failure", "warning"] {
        let mut harness = catalog_scene(egui::vec2(1280.0, 813.0), ThemePalette::Dark, "main");
        harness.state_mut().app.catalog_screen_state.status =
            "Fixture transfer progress".to_owned();
        harness.run();
        let history = harness.state().app.notification_manager.history().len();
        catalog_ui_editor(&mut harness);
        if outcome == "failure" {
            let path = mod_downloads::mod_downloads_user_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "invalid = [").unwrap();
        } else if outcome == "warning" {
            harness
                .state_mut()
                .app
                .catalog_screen_state
                .editor_step2
                .versions_ui
                .source_form
                .as_mut()
                .unwrap()
                .note = "New note must not disappear".to_owned();
            let path =
                crate::platform_defaults::app_config_file("mod_source_history.json", "config");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"invalid history bytes").unwrap();
        }
        let button = catalog_painted_text(
            &harness,
            if outcome == "cancel" {
                "Cancel"
            } else {
                "Save to My default"
            },
        );
        catalog_pointer_click(&mut harness, button.center());
        let app = &harness.state().app;
        assert_eq!(app.catalog_screen_state.status, "Fixture transfer progress");
        assert_eq!(
            app.notification_manager.history().len(),
            history + usize::from(outcome == "warning")
        );
        if outcome == "warning" {
            let record = app.notification_manager.history().back().unwrap();
            assert!(matches!(record.kind, egui_toast::ToastKind::Warning));
            assert!(record.text.contains("Source saved") && record.text.contains("not saved"));
        } else if outcome == "failure" {
            assert!(
                app.catalog_screen_state
                    .editor_step2
                    .versions_ui
                    .source_form
                    .as_ref()
                    .unwrap()
                    .error
                    .is_some()
            );
        } else {
            assert!(
                app.catalog_screen_state
                    .editor_step2
                    .versions_ui
                    .source_form
                    .is_none()
            );
        }
    }
}

#[test]
fn catalog_popup_live_resize_complete_rows_and_last_selection() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    let mut harness = catalog_scene(egui::vec2(1280.0, 1002.0), ThemePalette::Dark, "main");
    catalog_picker_seed_options(&mut harness, 24);
    let field = catalog_picker_field(&harness);
    catalog_pointer_click(&mut harness, field.center());
    for height in [150_u16, 1002, 150, 1002] {
        let size = egui::vec2(1280.0, f32::from(height));
        harness.set_size(size);
        harness.run();
        assert!(harness.state().popup_open);
        catalog_picker_assert_open_bounds(&harness, catalog_picker_field(&harness), size);
        let visible = catalog_picker_complete_rows(&harness, 24);
        if height == 1002 {
            assert_eq!(visible, 3);
        } else {
            assert!(
                (1..3).contains(&visible),
                "short viewport complete rows: {visible}"
            );
        }
    }
    let first = catalog_picker_popup_text(&harness, "Achievements original").unwrap();
    catalog_picker_wheel(&mut harness, first.center(), -4000.0);
    let last = catalog_picker_popup_text(&harness, "Achievements alternative 23")
        .expect("last complete source remains reachable after live resize");
    catalog_picker_capture(&mut harness, "resized-last-source");
    catalog_pointer_click(&mut harness, last.center());
    assert!(!harness.state().popup_open);
    assert_eq!(
        harness
            .state()
            .app
            .catalog_screen_state
            .source("tdd")
            .unwrap()
            .source_id,
        "choice-23"
    );
}

fn catalog_categories_text(categories: &str) -> String {
    format!(
        "[[mods]]\nname = \"Fixture Adventure\"\ntp2 = \"fixture\"\n{categories}\n\
         [[mods.sources]]\nid = \"main\"\ndefault = true\nlabel = \"Maintainer\"\n\
         type = \"github\"\nrepo = \"Fixture/Adventure\"\nbranch = \"stable\"\n\
         [[mods.sources]]\nid = \"mirror\"\nlabel = \"Mirror\"\ntype = \"url\"\n\
         url = \"https://example.invalid/adventure.zip\"\n"
    )
}

fn catalog_categories_matches(sources: Vec<ModDownloadSource>, query: &str) -> usize {
    let mut state = CatalogScreenState::default();
    state.replace_sources(sources, None);
    state.query = query.to_string();
    state.visible_rows().len()
}

#[test]
fn catalog_categories_search_is_case_insensitive_grouped_and_kind_filtered() {
    let text = catalog_categories_text(
        "categories = [\"Unusual Encounters\", \"NPC's \\\"Journeys\\\"\"]",
    );
    let mut state = CatalogScreenState::default();
    state.replace_from_texts(&text, "");
    assert!(state.error.is_none(), "{:?}", state.error);
    assert_eq!(state.rows.len(), 1);
    assert_eq!(state.rows[0].sources.len(), 2);
    state.query = "  uNuSuAl EnC  ".to_string();
    assert_eq!(state.visible_rows(), vec![0]);
    assert_eq!(state.kind_count(CatalogKind::All), 1);
    for kind in [CatalogKind::GitHub, CatalogKind::DirectArchive] {
        state.kind = kind;
        assert_eq!(state.visible_rows(), vec![0]);
    }
    state.kind = CatalogKind::WeaselMods;
    assert_eq!(state.visible_rows().len(), 0);
    state.kind = CatalogKind::All;
    state.query = "npc's \"journeys\"".to_string();
    assert_eq!(state.visible_rows(), vec![0]);
}

#[test]
fn catalog_categories_legacy_empty_and_tp2_identity_remain_compatible() {
    for categories in [
        "",
        "categories = []",
        "categories = [\"Unusual Encounters\"]",
    ] {
        let load = mod_downloads::load_mod_download_sources_from_texts(
            &catalog_categories_text(categories),
            "",
            "",
        );
        assert!(load.error.is_none(), "{:?}", load.error);
        assert_eq!(load.find_sources("setup-fixture.tp2").len(), 2);
        assert_eq!(load.find_sources("Unusual Encounters").len(), 0);
        assert_eq!(load.default_source("fixture").unwrap().source_id, "main");
        for query in [
            "fixture adventure",
            "fixture",
            "maintainer",
            "mirror",
            "fixture/adventure",
        ] {
            assert_eq!(catalog_categories_matches(load.sources.clone(), query), 1);
        }
        if categories != "categories = [\"Unusual Encounters\"]" {
            assert_eq!(catalog_categories_matches(load.sources, "unusual"), 0);
        }
    }
}

#[test]
fn catalog_categories_tiers_inherit_replace_and_clear_every_sibling() {
    let defaults = catalog_categories_text("categories = [\"Lower Category\"]");
    let replacement = "[[mods]]\ntp2 = \"SETUP-FIXTURE.TP2\"\n[[mods.sources]]\nid = \"main\"\ntype = \"github\"\nrepo = \"Personal/Adventure\"\ntag = \"v2\"\n";
    for (user_metadata, modlist, expected, rejected) in [
        ("", "", "lower category", "upper category"),
        (
            "categories = [\"Upper Category\"]",
            "",
            "upper category",
            "lower category",
        ),
        (
            "categories = [\"Upper Category\"]",
            "[[mods]]\ntp2 = \"fixture\"\ncategories = [\"Final Category\"]",
            "final category",
            "upper category",
        ),
        ("categories = []", "", "", "lower category"),
        (
            "",
            "[[mods]]\ntp2 = \"fixture\"\ncategories = []",
            "",
            "lower category",
        ),
    ] {
        let user = replacement.replace(
            "[[mods.sources]]",
            &format!("{user_metadata}\n[[mods.sources]]"),
        );
        let load = mod_downloads::load_mod_download_sources_from_texts(&defaults, &user, modlist);
        assert!(load.error.is_none(), "{:?}", load.error);
        assert_eq!(load.sources.len(), 2);
        let main = load
            .sources
            .iter()
            .find(|source| source.source_id == "main")
            .unwrap();
        assert_eq!(main.tag.as_deref(), Some("v2"));
        assert!(main.branch.is_none());
        for source in load.sources {
            if !expected.is_empty() {
                assert_eq!(
                    catalog_categories_matches(vec![source.clone()], expected),
                    1,
                    "{user_metadata} {modlist} {}",
                    source.source_id
                );
            }
            assert_eq!(catalog_categories_matches(vec![source], rejected), 0);
        }
    }
}

#[test]
fn catalog_categories_metadata_only_parent_and_added_source_inherit() {
    let defaults = catalog_categories_text("categories = [\"Lower Category\"]");
    let user = "[[mods]]\ntp2 = \"fixture\"\ncategories = [\"Upper Category\"]";
    let modlist = "[[mods]]\ntp2 = \"fixture\"\n[[mods.sources]]\nid = \"third\"\ntype = \"url\"\nurl = \"https://example.invalid/third.zip\"";
    let load = mod_downloads::load_mod_download_sources_from_texts(&defaults, user, modlist);
    assert!(load.error.is_none(), "{:?}", load.error);
    assert_eq!(load.sources.len(), 3);
    for source in load.sources {
        assert_eq!(
            catalog_categories_matches(vec![source.clone()], "upper category"),
            1,
            "{}",
            source.source_id
        );
        assert_eq!(
            catalog_categories_matches(vec![source], "lower category"),
            0
        );
    }
}

#[test]
fn catalog_categories_invalid_metadata_is_rejected() {
    for categories in [
        "categories = \"text\"",
        "categories = [1]",
        "categories = [\"text\", false]",
    ] {
        let load = mod_downloads::load_mod_download_sources_from_texts(
            &catalog_categories_text(categories),
            "",
            "",
        );
        assert!(load.error.is_some(), "accepted {categories}");
    }
}

#[test]
fn catalog_categories_form_roundtrip_keeps_parent_metadata_out_of_source_block() {
    let text =
        catalog_categories_text("categories = [\"NPC's \\\"Journeys\\\"\", \"Quest, story\"]");
    let load = mod_downloads::load_mod_download_sources_from_texts(&text, "", "");
    assert!(load.error.is_none(), "{:?}", load.error);
    for source in load.sources {
        let mut form = source_form::from_source(
            &source,
            SourceFormIdentity::default(),
            ModSourceEditDestination::GlobalDefault,
            "catalog:categories",
        );
        form.tag = "v3".to_string();
        let saved = source_form::source_form_save_text(&form);
        let parsed: toml::Value = toml::from_str(&saved).unwrap();
        let parent = &parsed["mods"][0];
        assert_eq!(
            parent.get("categories"),
            Some(&toml::Value::Array(vec![
                toml::Value::String("NPC's \"Journeys\"".to_string()),
                toml::Value::String("Quest, story".to_string())
            ]))
        );
        assert!(parent["sources"][0].get("categories").is_none());
        assert!(!mod_downloads::complete_source_block(&source).contains("categories"));
        let roundtrip = mod_downloads::load_mod_download_sources_from_texts(&saved, "", "");
        assert!(roundtrip.error.is_none(), "{:?}", roundtrip.error);
        assert_eq!(
            catalog_categories_matches(roundtrip.sources, "npc's \"journeys\""),
            1
        );
    }
}

#[test]
fn catalog_categories_save_preserves_destination_siblings_and_explicit_clear() {
    let _lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = FixtureRoot::new();
    for categories in ["categories = [\"Destination Category\"]", "categories = []"] {
        let path = root.join("mod_downloads_user.toml");
        let original = catalog_categories_text(categories);
        std::fs::write(&path, &original).unwrap();
        let incoming = catalog_categories_text("categories = [\"Placeholder Category\"]");
        mod_downloads::save_user_mod_download_source_block(
            "fixture",
            "Fixture Adventure",
            "main",
            false,
            &incoming,
            Some(&path),
        )
        .unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        let before: toml::Value = toml::from_str(&original).unwrap();
        let after: toml::Value = toml::from_str(&saved).unwrap();
        assert_eq!(
            after["mods"][0].get("categories"),
            before["mods"][0].get("categories")
        );
        let load = mod_downloads::load_mod_download_sources_from_texts(&saved, "", "");
        assert!(load.error.is_none(), "{:?}", load.error);
        assert_eq!(load.sources.len(), 2);
        assert!(
            load.sources
                .iter()
                .any(|source| source.source_id == "mirror")
        );
    }
}

#[test]
fn catalog_categories_effective_lists_and_template_are_exact() {
    let expected = vec!["NPC's \"Journeys\"".to_string(), "Quest, story".to_string()];
    let defaults =
        catalog_categories_text("categories = [\"NPC's \\\"Journeys\\\"\", \"Quest, story\"]");
    for (user, wanted) in [
        ("", expected),
        ("[[mods]]\ntp2 = \"fixture\"\ncategories = []", Vec::new()),
    ] {
        let loaded = mod_downloads::load_mod_download_sources_from_texts(&defaults, user, "");
        assert!(loaded.error.is_none());
        assert_eq!(loaded.sources.len(), 2);
        for source in loaded.sources {
            assert_eq!(source.categories, wanted);
        }
    }
    let header: toml::Value =
        toml::from_str(&mod_downloads::template_mod_header("newmod", "New Mod")).unwrap();
    assert_eq!(header["mods"][0]["categories"].as_array().unwrap().len(), 0);
}

#[test]
fn catalog_categories_save_inherited_override_add_remove_and_unrelated() {
    let _lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = FixtureRoot::new();
    let global = mod_downloads::mod_downloads_user_path();
    std::fs::create_dir_all(global.parent().unwrap()).unwrap();
    let defaults = catalog_categories_text("categories = [\"Inherited Category\"]");
    std::fs::write(&global, &defaults).unwrap();
    let target = root.join("override.toml");
    let unrelated = "[[mods]]\nname = \"Other\"\ntp2 = \"other\"\ncategories = [\"Other Category\"]\n[[mods.sources]]\nid = \"main\"\ntype = \"url\"\nurl = \"https://example.invalid/other.zip\"\n";
    std::fs::write(&target, unrelated).unwrap();
    let loaded = mod_downloads::load_mod_download_sources_from_texts(&defaults, "", "");
    let mut source = loaded.default_source("fixture").unwrap();
    source.tag = Some("v9".to_string());
    source.branch = None;
    let block = mod_downloads::complete_source_block(&source);
    mod_downloads::save_user_mod_download_source_block(
        "fixture",
        "Fixture Adventure",
        "main",
        false,
        &block,
        Some(&target),
    )
    .unwrap();
    let saved = std::fs::read_to_string(&target).unwrap();
    let parsed: toml::Value = toml::from_str(&saved).unwrap();
    let original: toml::Value = toml::from_str(unrelated).unwrap();
    assert_eq!(parsed["mods"][0], original["mods"][0]);
    assert_eq!(
        parsed["mods"][1]["categories"][0].as_str(),
        Some("Inherited Category")
    );
    source.source_id = "third".to_string();
    mod_downloads::save_user_mod_download_source_block(
        "fixture",
        "Fixture Adventure",
        "third",
        true,
        &mod_downloads::complete_source_block(&source),
        Some(&target),
    )
    .unwrap();
    mod_downloads::save_user_mod_download_source_block(
        "fixture",
        "Fixture Adventure",
        "main",
        false,
        "",
        Some(&target),
    )
    .unwrap();
    let saved = std::fs::read_to_string(&target).unwrap();
    let loaded = mod_downloads::load_mod_download_sources_from_texts(&defaults, "", &saved);
    assert!(loaded.error.is_none(), "{:?}", loaded.error);
    assert_eq!(loaded.find_sources("fixture").len(), 3);
    for source in loaded.find_sources("fixture") {
        assert_eq!(source.categories, vec!["Inherited Category"]);
    }
    let parsed: toml::Value = toml::from_str(&saved).unwrap();
    assert_eq!(parsed["mods"][0], original["mods"][0]);
    assert_eq!(parsed["mods"][1]["sources"].as_array().unwrap().len(), 1);
    assert_eq!(
        parsed["mods"][1]["sources"][0]["id"].as_str(),
        Some("third")
    );
}

#[test]
fn catalog_categories_migration_preserves_inherit_replace_and_clear() {
    use crate::app::mod_downloads_migrate::{MigrateResult, MigrateTier, migrate_source_text};
    let defaults = catalog_categories_text("categories = [\"Inherited Category\"]");
    for (metadata, expected) in [
        ("", vec!["Inherited Category"]),
        ("categories = []", vec![]),
        ("categories = [\"User Category\"]", vec!["User Category"]),
    ] {
        let user = format!(
            "[[mods]]\nname = \"Fixture Adventure\"\ntp2 = \"fixture\"\n{metadata}\n[[mods.sources]]\nid = \"main\"\nbranch = \"custom\"\n"
        );
        let MigrateResult::Rewritten(migrated) =
            migrate_source_text(&user, &defaults, "", MigrateTier::User)
        else {
            panic!("expected a migration rewrite");
        };
        let loaded = mod_downloads::load_mod_download_sources_from_texts(&defaults, &migrated, "");
        assert!(loaded.error.is_none(), "{:?}", loaded.error);
        for source in loaded.sources {
            assert_eq!(source.categories, expected);
            if source.source_id == "main" {
                assert_eq!(source.branch.as_deref(), Some("custom"));
                assert_eq!(source.github.as_deref(), Some("Fixture/Adventure"));
            }
        }
    }
}

#[test]
fn catalog_categories_history_preserves_metadata_without_identity_changes() {
    use crate::app::mod_source_history;
    let text =
        catalog_categories_text("categories = [\"NPC's \\\"Journeys\\\"\", \"Quest, story\"]");
    let loaded = mod_downloads::load_mod_download_sources_from_texts(&text, "", "");
    for source in loaded.sources {
        let normalized = mod_source_history::normalize_source("fixture", &source);
        assert_eq!(normalized.categories, source.categories);
        assert_eq!(normalized.source_id, source.source_id);
        let mut other = source.clone();
        other.categories = vec!["Different Category".to_string()];
        assert_eq!(
            mod_source_history::rule_signature(&source),
            mod_source_history::rule_signature(&other)
        );
        let (bookmarked, _) =
            mod_source_history::bookmark_block(&source, Some(&source.source_id), Some("v4"))
                .unwrap();
        assert_eq!(bookmarked.categories, source.categories);
        assert!(!mod_downloads::complete_source_block(&bookmarked).contains("categories"));
    }
}

#[test]
fn catalog_categories_metadata_only_lower_tiers_survive_first_source() {
    let defaults = "[[mods]]\ntp2 = \"fixture\"\ncategories = [\"Default Category\"]";
    let modlist = "[[mods]]\ntp2 = \"fixture\"\n[[mods.sources]]\nid = \"main\"\ntype = \"url\"\nurl = \"https://example.invalid/first.zip\"";
    for (user, expected) in [
        ("", vec!["Default Category"]),
        (
            "[[mods]]\ntp2 = \"fixture\"\ncategories = [\"User Category\"]",
            vec!["User Category"],
        ),
        ("[[mods]]\ntp2 = \"fixture\"\ncategories = []", vec![]),
    ] {
        let loaded = mod_downloads::load_mod_download_sources_from_texts(defaults, user, modlist);
        assert!(loaded.error.is_none(), "{:?}", loaded.error);
        assert_eq!(loaded.sources.len(), 1);
        assert_eq!(loaded.sources[0].categories, expected);
    }
}

#[test]
fn catalog_categories_duplicate_parent_collapse_keeps_effective_metadata() {
    let _lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = FixtureRoot::new();
    let path = root.join("duplicates.toml");
    let original = format!(
        "{}\n[[mods]]\ntp2 = \"fixture\"\ncategories = [\"Effective Category\"]\n",
        catalog_categories_text("categories = []")
    );
    std::fs::write(&path, &original).unwrap();
    let loaded = mod_downloads::load_mod_download_sources_from_texts(&original, "", "");
    assert!(loaded.error.is_none(), "{:?}", loaded.error);
    let mut source = loaded.default_source("fixture").unwrap();
    assert_eq!(source.categories, vec!["Effective Category"]);
    source.github = Some("Edited/Adventure".to_string());
    source.url = "https://github.com/Edited/Adventure".to_string();
    let form = source_form::from_source(
        &source,
        SourceFormIdentity::default(),
        ModSourceEditDestination::GlobalDefault,
        "catalog:duplicates",
    );
    mod_downloads::save_user_mod_download_source_block(
        "fixture",
        "Fixture Adventure",
        "main",
        false,
        &source_form::source_form_save_text(&form),
        Some(&path),
    )
    .unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    let loaded = mod_downloads::load_mod_download_sources_from_texts(&saved, "", "");
    assert!(loaded.error.is_none(), "{:?}", loaded.error);
    assert_eq!(loaded.sources.len(), 2);
    for source in loaded.sources {
        assert_eq!(source.categories, vec!["Effective Category"]);
    }
}

#[test]
fn catalog_categories_migration_metadata_only_duplicates_keep_last_declaration() {
    use crate::app::mod_downloads_migrate::{MigrateResult, MigrateTier, migrate_source_text};
    for first in ["", "categories = []"] {
        let text = format!(
            "[[mods]]\ntp2 = \"fixture\"\n{first}\n[[mods]]\ntp2 = \"fixture\"\ncategories = [\"Effective Category\"]\n"
        );
        let MigrateResult::Rewritten(migrated) =
            migrate_source_text(&text, "", "", MigrateTier::User)
        else {
            panic!("expected a migration rewrite");
        };
        let parsed: toml::Value = toml::from_str(&migrated).unwrap();
        let parents = parsed.get("mods").and_then(toml::Value::as_array).unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0]["tp2"].as_str(), Some("fixture"));
        assert_eq!(
            parents[0]["categories"].as_array().unwrap(),
            &vec![toml::Value::String("Effective Category".to_string())]
        );
        assert!(parents[0].get("sources").is_none());
    }
}

fn catalog_type_sources() -> Vec<ModDownloadSource> {
    let mut sources = fixture_sources();
    sources[0].categories = vec![" Quests ".to_owned(), "BGEE".to_owned()];
    sources[1].categories = vec!["NPC".to_owned()];
    sources[2].categories = vec!["Quests".to_owned()];
    let mut sibling = sources[0].clone();
    sibling.source_id = "mirror".to_owned();
    sibling.source_default = false;
    sibling.categories = vec!["NPC".to_owned(), "quests".to_owned()];
    sources.push(sibling);
    sources
}

#[test]
fn catalog_type_options_all_existing_custom_dedup_and_games_searchable() {
    let types = [
        "AI",
        "Animations",
        "Audio",
        "Banter",
        "Campaign",
        "Classes",
        "Content Restoration",
        "Difficulty",
        "Encounters",
        "Fixes",
        "Graphics",
        "Items",
        "Kits",
        "Modding Resources",
        "Music",
        "NPC",
        "NPC Expansion",
        "Portraits",
        "Quests",
        "Romance",
        "Rules",
        "Soundsets",
        "Spells",
        "Tools",
        "Total Conversion",
        "Translations",
        "Tweaks",
        "UI",
        "Worldmap",
    ];
    let games = [
        "BG1", "BG2", "BGEE", "SoD", "BG2EE", "EET", "Tutu", "BGT", "IWD", "IWDEE", "IWD2", "PST",
        "PSTEE",
    ];
    let mut source = github_source();
    source.categories = types
        .iter()
        .chain(games.iter())
        .map(|s| (*s).to_owned())
        .collect();
    source
        .categories
        .extend([" quests ".to_owned(), "Future Custom".to_owned()]);
    let mut state = CatalogScreenState::default();
    state.replace_sources(vec![source], None);
    assert_eq!(state.category_options.len(), 30);
    for category in types.into_iter().chain(["Future Custom"]) {
        assert!(
            state
                .category_options
                .contains_key(&category.to_lowercase())
        );
        assert!(
            state.rows[0]
                .category_keys
                .contains(&category.to_lowercase())
        );
    }
    for game in games {
        assert!(!state.category_options.contains_key(&game.to_lowercase()));
        assert!(!state.rows[0].category_keys.contains(&game.to_lowercase()));
        state.query = game.to_owned();
        assert_eq!(state.visible_rows(), vec![0]);
    }
}

#[test]
fn catalog_type_or_and_query_kind_refresh_and_selection_contract() {
    let mut state = CatalogScreenState::default();
    let sources = catalog_type_sources();
    state.replace_sources(sources.clone(), None);
    assert_eq!(state.category_options.len(), 2);
    assert_eq!(state.visible_rows().len(), 4);
    state.focus("tdd");
    state.toggle("tdd");
    state.choose_source("tdd", "mirror");
    let picks = state.source_picks.clone();
    state.selected_types.insert("npc".to_owned());
    assert_eq!(state.visible_rows().len(), 2);
    state.selected_types.insert("quests".to_owned());
    assert_eq!(state.visible_rows().len(), 3);
    state.query = "darkest".to_owned();
    state.kind = CatalogKind::GitHub;
    assert_eq!(state.visible_rows().len(), 1);
    state.kind = CatalogKind::DirectArchive;
    assert_eq!(state.visible_rows(), Vec::<usize>::new());
    assert_eq!(state.kind_count(CatalogKind::All), 4);
    assert_eq!(state.kind_count(CatalogKind::GitHub), 1);
    assert_eq!(state.selected_tp2, BTreeSet::from(["tdd".to_owned()]));
    assert_eq!(state.focused_tp2.as_deref(), Some("tdd"));
    assert_eq!(state.source_picks, picks);
    state.query.clear();
    state.kind = CatalogKind::All;
    state.replace_sources(sources, None);
    assert_eq!(state.selected_types.len(), 2);
    assert_eq!(state.source_picks, picks);
    state.replace_sources(vec![fixture_sources()[3].clone()], None);
    assert_eq!(state.selected_types.len(), 2);
    assert!(state.category_options.is_empty());
    assert_eq!(state.visible_rows(), Vec::<usize>::new());
    state.selected_types.clear();
    assert_eq!(state.visible_rows().len(), 1);
}

fn catalog_type_capture(harness: &mut Harness<'_, CatalogScene>, name: &str) {
    if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
        let directory = PathBuf::from(directory);
        assert!(directory.is_absolute());
        std::fs::create_dir_all(&directory).unwrap();
        harness
            .render()
            .unwrap()
            .save(directory.join(format!("filter-{name}.png")))
            .unwrap();
    }
}

#[test]
fn catalog_type_popup_click_clear_escape_outside_and_live_resize() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for (palette, name) in [(ThemePalette::Dark, "dark"), (ThemePalette::Light, "light")] {
        let mut harness = catalog_scene(egui::vec2(1570.0, 813.0), palette, "main");
        harness
            .state_mut()
            .app
            .catalog_screen_state
            .replace_sources(catalog_type_sources(), None);
        harness.run();
        let selected = harness
            .state()
            .app
            .catalog_screen_state
            .selected_tp2
            .clone();
        catalog_type_capture(&mut harness, &format!("closed-{name}"));
        let filter = catalog_painted_text(&harness, "Filter");
        catalog_pointer_click(&mut harness, filter.center());
        assert!(harness.state().popup_open);
        catalog_type_capture(&mut harness, &format!("open-{name}"));
        let npc = catalog_painted_text(&harness, "NPC");
        catalog_pointer_click(&mut harness, npc.center());
        assert!(harness.state().popup_open);
        assert_eq!(
            harness.state().app.catalog_screen_state.selected_types,
            BTreeSet::from(["npc".to_owned()])
        );
        catalog_painted_text(&harness, "Filter (1)");
        catalog_type_capture(&mut harness, &format!("selected-{name}"));
        for width in [1280_u16, 1100, 900, 1570] {
            harness.set_size(egui::vec2(f32::from(width), 813.0));
            harness.run();
            assert!(harness.state().popup_open);
            for label in ["NPC", "Quests", "Clear filters"] {
                let rect = catalog_painted_text(&harness, label);
                assert!(
                    egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(f32::from(width), 813.0)
                    )
                    .contains_rect(rect)
                );
            }
            catalog_type_capture(&mut harness, &format!("resize-{width}-{name}"));
        }
        let clear = catalog_painted_text(&harness, "Clear filters");
        catalog_pointer_click(&mut harness, clear.center());
        assert!(harness.state().popup_open);
        assert!(
            harness
                .state()
                .app
                .catalog_screen_state
                .selected_types
                .is_empty()
        );
        assert_eq!(
            harness.state().app.catalog_screen_state.selected_tp2,
            selected
        );
        assert_eq!(
            harness
                .state()
                .app
                .catalog_screen_state
                .focused_tp2
                .as_deref(),
            Some("tdd")
        );
        catalog_type_capture(&mut harness, &format!("cleared-{name}"));
        harness.press_key(egui::Key::Escape);
        harness.run();
        assert!(!harness.state().popup_open);
        let filter = catalog_painted_text(&harness, "Filter");
        catalog_pointer_click(&mut harness, filter.center());
        assert!(harness.state().popup_open);
        let outside = catalog_painted_text(&harness, "Mod catalog");
        catalog_pointer_click(&mut harness, outside.center());
        assert!(!harness.state().popup_open);
    }
}

fn catalog_type_visible_text(
    harness: &Harness<'_, CatalogScene>,
    label: &str,
) -> Option<egui::Rect> {
    harness.output().shapes.iter().find_map(|clipped| {
        let mut shapes = Vec::new();
        catalog_shapes(&clipped.shape, &mut shapes);
        shapes.iter().find_map(|shape| {
            if let egui::Shape::Text(text) = shape
                && text.galley.text() == label
            {
                let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                return clipped.clip_rect.contains_rect(rect).then_some(rect);
            }
            None
        })
    })
}

#[test]
fn catalog_type_many_options_scroll_last_checkbox_and_clear_reachable() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for (palette, name) in [(ThemePalette::Dark, "dark"), (ThemePalette::Light, "light")] {
        let mut harness = catalog_scene(egui::vec2(900.0, 620.0), palette, "main");
        let mut source = github_source();
        source.categories = (0..60).map(|i| format!("Type {i:02}")).collect();
        harness
            .state_mut()
            .app
            .catalog_screen_state
            .replace_sources(vec![source], None);
        harness.run();
        let filter = catalog_painted_text(&harness, "Filter");
        catalog_pointer_click(&mut harness, filter.center());
        assert!(harness.state().popup_open);
        let first = catalog_type_visible_text(&harness, "Type 00").unwrap();
        catalog_picker_wheel(&mut harness, first.center(), -5000.0);
        let last = catalog_type_visible_text(&harness, "Type 59")
            .expect("final checkbox must be reachable by wheel");
        catalog_pointer_click(&mut harness, last.center());
        assert!(harness.state().popup_open);
        assert!(
            harness
                .state()
                .app
                .catalog_screen_state
                .selected_types
                .contains("type 59")
        );
        catalog_type_capture(&mut harness, &format!("last-checkbox-{name}"));
        let clear = catalog_type_visible_text(&harness, "Clear filters")
            .expect("Clear filters must remain reachable");
        catalog_pointer_click(&mut harness, clear.center());
        assert!(harness.state().popup_open);
        assert!(
            harness
                .state()
                .app
                .catalog_screen_state
                .selected_types
                .is_empty()
        );
    }
}

fn catalog_full_toolbar_geometry(
    harness: &Harness<'_, CatalogScene>,
    width: u16,
    caption: &str,
) -> egui::Rect {
    let search = catalog_toolbar_widget(harness, "Search mods, source labels or TP2\u{2026}");
    let filter = catalog_toolbar_widget(harness, caption);
    let content = harness.state().content_rect;
    let tabs = [
        "All (",
        "GitHub (",
        "Weaselmods (",
        "Morpheus Mart (",
        "Direct archives (",
    ]
    .map(|label| catalog_painted_text(harness, label));
    assert!(content.contains_rect(search));
    assert!(content.contains_rect(filter));
    assert!(
        search.width() >= 140.0,
        "{width}: unusable search {search:?}"
    );
    assert!(search.right() < filter.left());
    assert!((search.center().y - filter.center().y).abs() < 2.0);
    for pair in tabs.windows(2) {
        assert!(!pair[0].intersects(pair[1]));
    }
    for tab in tabs {
        assert!(content.contains_rect(tab));
        if width >= 1280 {
            assert!(
                (tab.center().y - search.center().y).abs() < 2.0,
                "{width}: normal toolbar wrapped: tab={tab:?} search={search:?} filter={filter:?}"
            );
            assert!(tab.right() < search.left());
        } else {
            assert!(tab.bottom() < search.top());
        }
    }
    println!("FULL_TOOLBAR {width} {caption}: search={search:?} filter={filter:?}");
    search
}

fn catalog_full_toolbar_states(harness: &mut Harness<'_, CatalogScene>, theme: &str) {
    for count in [0, 2, 29] {
        let state = &mut harness.state_mut().app.catalog_screen_state;
        state.selected_types = state.category_options.keys().take(count).cloned().collect();
        harness.run();
        let caption = if count == 0 {
            "Filter".to_owned()
        } else {
            format!("Filter ({count})")
        };
        let normal = catalog_full_toolbar_geometry(harness, 1280, &caption);
        catalog_type_capture(harness, &format!("full-1280-closed-{count}-{theme}"));
        let filter = catalog_painted_text(harness, &caption);
        catalog_pointer_click(harness, filter.center());
        assert!(harness.state().popup_open);
        catalog_full_toolbar_geometry(harness, 1280, &caption);
        catalog_type_capture(harness, &format!("full-1280-open-{count}-{theme}"));
        for width in [1570_u16, 900, 1280] {
            harness.set_size(egui::vec2(f32::from(width), 813.0));
            harness.run();
            assert!(harness.state().popup_open);
            let search = catalog_full_toolbar_geometry(harness, width, &caption);
            if width == 1570 {
                assert!((search.width() - normal.width() - 290.0).abs() < 1.0);
            } else if width == 1280 {
                assert_eq!(search, normal);
            }
            catalog_type_capture(harness, &format!("full-{width}-open-{count}-{theme}"));
        }
        let clear = catalog_type_visible_text(harness, "Clear filters").unwrap();
        catalog_picker_wheel(harness, clear.center() - egui::vec2(0.0, 60.0), 5000.0);
        let first = catalog_type_visible_text(harness, "AI").unwrap();
        catalog_picker_wheel(harness, first.center(), -5000.0);
        assert!(catalog_type_visible_text(harness, "Worldmap").is_some());
        assert!(catalog_type_visible_text(harness, "Clear filters").is_some());
        catalog_type_capture(harness, &format!("full-1280-bottom-{count}-{theme}"));
        harness.press_key(egui::Key::Escape);
        harness.run();
        assert!(!harness.state().popup_open);
    }
}

#[test]
fn catalog_full_default_normal_1280_toolbar_stays_inline() {
    let _ambient_lock = mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ambient_guard = AmbientGuard(mod_downloads::active_modlist_dir());
    for (palette, theme) in [(ThemePalette::Dark, "dark"), (ThemePalette::Light, "light")] {
        let mut harness = catalog_scene(egui::vec2(1280.0, 813.0), palette, "main");
        let state = &mut harness.state_mut().app.catalog_screen_state;
        state.replace_from_texts(mod_downloads::default_mod_downloads_content(), "");
        assert_eq!(state.rows.len(), 1268);
        assert_eq!(
            state
                .rows
                .iter()
                .map(|row| row.sources.len())
                .sum::<usize>(),
            1292
        );
        assert_eq!(state.category_options.len(), 29);
        catalog_full_toolbar_states(&mut harness, theme);
        harness.state_mut().app.catalog_screen_state.query = "baldur".to_owned();
        harness.run();
        assert_eq!(harness.state().app.catalog_screen_state.query, "baldur");
        let search = catalog_toolbar_widget(&harness, "baldur");
        assert!(search.contains_rect(catalog_painted_text(&harness, "baldur")));
    }
}

fn catalog_toolbar_widget(harness: &Harness<'_, CatalogScene>, label: &str) -> egui::Rect {
    let anchor = catalog_painted_text(harness, label);
    let mut shapes = Vec::new();
    for clipped in &harness.output().shapes {
        catalog_shapes(&clipped.shape, &mut shapes);
    }
    shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if rect.rect.contains(anchor.left_center())
                    && (36.0..=40.0).contains(&rect.rect.height()) =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing toolbar widget {label}"))
}
