// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::mod_downloads::SourceTiers;
use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::gallery::filter::GalleryFilter;
use crate::ui::install::inside_model::{self, InsideModel};
use crate::ui::install::stage_downloading::{DownloadProgress, SkippedMod};
use crate::ui::install::whats_inside::{InsideClick, InsideCounts};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallStage {
    #[default]
    Gallery,
    Details,
    Paste,
    Downloading,
    InstallingStub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewOrigin {
    #[default]
    Details,
    Paste,
    Reinstall,
}

impl ReviewOrigin {
    #[must_use]
    pub const fn back_stage(self) -> InstallStage {
        match self {
            Self::Details | Self::Reinstall => InstallStage::Gallery,
            Self::Paste => InstallStage::Paste,
        }
    }
}

#[must_use]
pub const fn install_stage_is_idle(stage: InstallStage) -> bool {
    matches!(
        stage,
        InstallStage::Gallery | InstallStage::Details | InstallStage::Paste
    )
}

#[derive(Debug, Clone, Default)]
pub struct ReviewState {
    pub origin: ReviewOrigin,
    pub name: String,
    pub modify: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipelineKind {
    #[default]
    Install,
    Fork,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GalleryScreenState {
    pub(crate) filter: GalleryFilter,
    pub(crate) selected: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DestChoice {
    Clear,
    Backup,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestFlags {
    pub prepare_target_dirs_before_install: bool,
    pub backup_targets_before_eet_copy: bool,
    pub skip_installed: bool,
    pub check_last_installed: DestCheckFlag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DestCheckFlag(bool);

impl DestCheckFlag {
    const fn new(value: bool) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> bool {
        self.0
    }
}

impl std::ops::Not for DestCheckFlag {
    type Output = bool;

    fn not(self) -> Self::Output {
        !self.0
    }
}

impl DestChoice {
    #[must_use]
    pub const fn to_flags(self) -> DestFlags {
        match self {
            Self::Clear => DestFlags {
                prepare_target_dirs_before_install: true,
                backup_targets_before_eet_copy: false,
                skip_installed: false,
                check_last_installed: DestCheckFlag::new(false),
            },
            Self::Backup => DestFlags {
                prepare_target_dirs_before_install: true,
                backup_targets_before_eet_copy: true,
                skip_installed: false,
                check_last_installed: DestCheckFlag::new(false),
            },
            Self::Continue => DestFlags {
                prepare_target_dirs_before_install: false,
                backup_targets_before_eet_copy: false,
                skip_installed: true,
                check_last_installed: DestCheckFlag::new(true),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawerKind {
    IncludedMods,
    WeiduLogs,
    InstalledRefs,
    DownloadSources,
    ConfigFiles,
    Install,
}

impl DrawerKind {
    #[must_use]
    pub(crate) const fn from_click(click: InsideClick) -> Option<Self> {
        match click {
            InsideClick::None => None,
            InsideClick::IncludedMods => Some(Self::IncludedMods),
            InsideClick::WeiduLogs => Some(Self::WeiduLogs),
            InsideClick::InstalledRefs => Some(Self::InstalledRefs),
            InsideClick::DownloadSources => Some(Self::DownloadSources),
            InsideClick::ConfigFiles => Some(Self::ConfigFiles),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DrawerState {
    pub(crate) open: Option<DrawerKind>,
    pub(crate) logs_tab: usize,
    pub(crate) mods_tab: usize,
    pub(crate) mods_query: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstallPipelineFlags {
    bits: u16,
}

impl InstallPipelineFlags {
    const ARMED: u16 = 0b0000_0001;
    const ARCHIVES_STAGED: u16 = 0b0000_0010;
    const ARCHIVES_INGESTED: u16 = 0b0000_0100;
    const ARCHIVES_VERIFIED: u16 = 0b0000_1000;
    const DOWNLOAD_PHASE_STARTED: u16 = 0b0001_0000;
    const ARCHIVE_SKIP_COMPLETED: u16 = 0b0010_0000;
    const EXPLICIT_RESOLVE_STARTED: u16 = 0b0100_0000;
    const OVERRIDE_WARNINGS_TOASTED: u16 = 0b1000_0000;
    const AUTO_START_FIRED: u16 = 0b1_0000_0000;

    #[must_use]
    pub const fn armed(self) -> bool {
        self.bits & Self::ARMED != 0
    }

    pub const fn set_armed(&mut self, value: bool) {
        self.set_bit(Self::ARMED, value);
    }

    #[must_use]
    pub const fn archives_staged(self) -> bool {
        self.bits & Self::ARCHIVES_STAGED != 0
    }

    pub const fn set_archives_staged(&mut self, value: bool) {
        self.set_bit(Self::ARCHIVES_STAGED, value);
    }

    #[must_use]
    pub const fn archives_ingested(self) -> bool {
        self.bits & Self::ARCHIVES_INGESTED != 0
    }

    pub const fn set_archives_ingested(&mut self, value: bool) {
        self.set_bit(Self::ARCHIVES_INGESTED, value);
    }

    #[must_use]
    pub const fn archives_verified(self) -> bool {
        self.bits & Self::ARCHIVES_VERIFIED != 0
    }

    pub const fn set_archives_verified(&mut self, value: bool) {
        self.set_bit(Self::ARCHIVES_VERIFIED, value);
    }

    #[must_use]
    pub const fn download_phase_started(self) -> bool {
        self.bits & Self::DOWNLOAD_PHASE_STARTED != 0
    }

    pub const fn set_download_phase_started(&mut self, value: bool) {
        self.set_bit(Self::DOWNLOAD_PHASE_STARTED, value);
    }

    #[must_use]
    pub const fn archive_skip_completed(self) -> bool {
        self.bits & Self::ARCHIVE_SKIP_COMPLETED != 0
    }

    pub const fn set_archive_skip_completed(&mut self, value: bool) {
        self.set_bit(Self::ARCHIVE_SKIP_COMPLETED, value);
    }

    #[must_use]
    pub const fn explicit_resolve_started(self) -> bool {
        self.bits & Self::EXPLICIT_RESOLVE_STARTED != 0
    }

    pub const fn set_explicit_resolve_started(&mut self, value: bool) {
        self.set_bit(Self::EXPLICIT_RESOLVE_STARTED, value);
    }

    #[must_use]
    pub const fn override_warnings_toasted(self) -> bool {
        self.bits & Self::OVERRIDE_WARNINGS_TOASTED != 0
    }

    pub const fn set_override_warnings_toasted(&mut self, value: bool) {
        self.set_bit(Self::OVERRIDE_WARNINGS_TOASTED, value);
    }

    #[must_use]
    pub const fn auto_start_fired(self) -> bool {
        self.bits & Self::AUTO_START_FIRED != 0
    }

    pub const fn set_auto_start_fired(&mut self, value: bool) {
        self.set_bit(Self::AUTO_START_FIRED, value);
    }

    pub const fn reset(&mut self) {
        self.bits = 0;
    }

    const fn set_bit(&mut self, bit: u16, value: bool) {
        if value {
            self.bits |= bit;
        } else {
            self.bits &= !bit;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InstallScreenState {
    pub stage: InstallStage,
    pub pipeline_kind: PipelineKind,
    pub(crate) gallery: GalleryScreenState,
    pub(crate) drawer: DrawerState,
    pub review: ReviewState,
    pub destination: String,
    pub destination_choice: Option<DestChoice>,
    pub import_code: String,
    pub(crate) parsed_preview: Option<ModlistSharePreview>,
    pub preview_parse_error: Option<String>,
    pub(crate) inside: Option<InsideModel>,
    pub(crate) inside_counts: Option<InsideCounts>,
    pub fork_info_open: bool,
    pub preview_cached: bool,
    pub download_progress: DownloadProgress,
    pub pipeline_flags: InstallPipelineFlags,
    pub pipeline_arm_error: Option<String>,
    pub pre_step5_blocker_checked: bool,
    pub expected_archive_meta: Vec<crate::registry::share_export::ArchiveMeta>,
    pub pre_skip_assets: Vec<crate::app::state::Step2UpdateAsset>,
    pub skipped_mods: Vec<SkippedMod>,
    pub expected_archive_sizes: std::collections::BTreeMap<usize, u64>,
    pub skip_indices: std::collections::HashSet<usize>,
    pub hashed_indices: std::collections::HashSet<usize>,
}

impl InstallScreenState {
    #[must_use]
    pub fn has_route_context(&self) -> bool {
        self.stage != InstallStage::Gallery
            || !self.destination.trim().is_empty()
            || !self.import_code.trim().is_empty()
            || self.parsed_preview.is_some()
            || self.preview_parse_error.is_some()
            || self.preview_cached
            || !self.download_progress.rows.is_empty()
            || self.pipeline_flags.armed()
            || self.pipeline_flags.archives_staged()
            || self.pipeline_flags.archives_ingested()
            || self.pipeline_flags.archives_verified()
            || self.pipeline_flags.download_phase_started()
            || self.pipeline_flags.archive_skip_completed()
            || self.pipeline_flags.explicit_resolve_started()
            || self.pipeline_arm_error.is_some()
            || !self.expected_archive_meta.is_empty()
            || !self.pre_skip_assets.is_empty()
            || !self.skipped_mods.is_empty()
            || !self.expected_archive_sizes.is_empty()
            || !self.skip_indices.is_empty()
    }

    pub fn reset_to_gallery(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn inside_model(
        &mut self,
        load_tiers: impl FnOnce(&str) -> SourceTiers,
    ) -> Option<&InsideModel> {
        if self.inside.is_none() {
            let preview = self.parsed_preview.as_ref()?;
            let tiers = load_tiers(&preview.source_overrides_text);
            self.inside = Some(inside_model::build(preview, &tiers));
        }
        self.inside.as_ref()
    }

    pub(crate) fn inside_counts(&mut self) -> Option<&InsideCounts> {
        if self.inside_counts.is_none() {
            let preview = self.parsed_preview.as_ref()?;
            self.inside_counts = Some(InsideCounts::from_preview(preview));
        }
        self.inside_counts.as_ref()
    }

    pub fn clear_preview(&mut self) {
        self.parsed_preview = None;
        self.preview_parse_error = None;
        self.inside = None;
        self.inside_counts = None;
        self.fork_info_open = false;
        self.preview_cached = false;
        self.download_progress = DownloadProgress::default();
        self.pipeline_flags.reset();
        self.pipeline_arm_error = None;
        self.expected_archive_meta = Vec::new();
        self.pre_skip_assets = Vec::new();
        self.skipped_mods = Vec::new();
        self.expected_archive_sizes = std::collections::BTreeMap::new();
        self.skip_indices = std::collections::HashSet::new();
        self.hashed_indices = std::collections::HashSet::new();
        self.drawer = DrawerState::default();
    }

    #[must_use]
    pub const fn auto_start_fired(&self) -> bool {
        self.pipeline_flags.auto_start_fired()
    }

    pub const fn set_auto_start_fired(&mut self, value: bool) {
        self.pipeline_flags.set_auto_start_fired(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_maps_to_prepare_on_backup_off_no_skip() {
        let f = DestChoice::Clear.to_flags();
        assert!(f.prepare_target_dirs_before_install);
        assert!(!f.backup_targets_before_eet_copy);
        assert!(!f.skip_installed);
        assert!(!f.check_last_installed.get());
    }

    #[test]
    fn backup_maps_to_prepare_on_backup_on_no_skip() {
        let f = DestChoice::Backup.to_flags();
        assert!(f.prepare_target_dirs_before_install);
        assert!(f.backup_targets_before_eet_copy);
        assert!(!f.skip_installed);
        assert!(!f.check_last_installed.get());
    }

    #[test]
    fn continue_maps_to_prepare_off_backup_off_skip_on() {
        let f = DestChoice::Continue.to_flags();
        assert!(!f.prepare_target_dirs_before_install);
        assert!(!f.backup_targets_before_eet_copy);
        assert!(f.skip_installed);
        assert!(f.check_last_installed.get());
    }

    #[test]
    fn continue_stays_in_the_enum_for_persisted_workspaces() {
        let round_trip: DestChoice =
            serde_json::from_str(&serde_json::to_string(&DestChoice::Continue).expect("serialize"))
                .expect("a workspace.json that persisted Continue must still load");
        assert_eq!(round_trip, DestChoice::Continue);
    }

    #[test]
    fn default_pipeline_kind_is_install() {
        assert_eq!(PipelineKind::default(), PipelineKind::Install);
        assert_eq!(
            InstallScreenState::default().pipeline_kind,
            PipelineKind::Install
        );
    }

    #[test]
    fn review_back_stage_follows_the_origin() {
        assert_eq!(ReviewOrigin::Details.back_stage(), InstallStage::Gallery);
        assert_eq!(ReviewOrigin::Paste.back_stage(), InstallStage::Paste);
        assert_eq!(ReviewOrigin::Reinstall.back_stage(), InstallStage::Gallery);
    }

    #[test]
    fn default_review_state_is_an_unnamed_unmodified_details_review() {
        let review = InstallScreenState::default().review;
        assert_eq!(review.origin, ReviewOrigin::Details);
        assert!(review.name.is_empty());
        assert!(!review.modify);
    }

    #[test]
    fn default_stage_is_gallery() {
        assert_eq!(InstallScreenState::default().stage, InstallStage::Gallery);
    }

    #[test]
    fn default_gallery_has_no_filter_and_no_selection() {
        let st = InstallScreenState::default();
        assert_eq!(st.gallery.filter, GalleryFilter::default());
        assert!(st.gallery.selected.is_none());
    }

    #[test]
    fn clear_preview_resets_preview_state() {
        let mut st = InstallScreenState {
            preview_cached: true,
            fork_info_open: true,
            preview_parse_error: Some("boom".to_string()),
            pipeline_arm_error: Some("arm boom".to_string()),
            ..Default::default()
        };
        st.pipeline_flags.set_armed(true);
        st.pipeline_flags.set_archives_staged(true);
        st.pipeline_flags.set_archives_ingested(true);
        st.pipeline_flags.set_download_phase_started(true);
        st.pipeline_flags.set_archive_skip_completed(true);
        st.pipeline_flags.set_explicit_resolve_started(true);
        st.hashed_indices.insert(0);
        st.hashed_indices.insert(7);
        st.clear_preview();
        assert!(st.parsed_preview.is_none());
        assert!(st.preview_parse_error.is_none());
        assert!(!st.fork_info_open);
        assert!(!st.preview_cached);
        assert!(!st.pipeline_flags.armed());
        assert!(st.pipeline_arm_error.is_none());
        assert!(
            !st.pipeline_flags.archives_staged() && !st.pipeline_flags.archives_ingested(),
            "D1: a re-entry must re-stage/re-ingest from scratch"
        );
        assert!(
            !st.pipeline_flags.download_phase_started(),
            "a re-entry must re-kick the streamer"
        );
        assert!(
            !st.pipeline_flags.archive_skip_completed(),
            "a re-entry must re-run the async skip pass"
        );
        assert!(
            !st.pipeline_flags.explicit_resolve_started(),
            "a re-entry must re-run the explicit resolve"
        );
        assert!(
            st.hashed_indices.is_empty(),
            "a re-entry must start with no asset hash decisions carried in"
        );
    }

    #[test]
    fn clear_preview_resets_per_install_hash_and_extract_snapshots() {
        let mut st = InstallScreenState::default();
        st.download_progress.hash_progress = Some((51, 51));
        st.download_progress.extract_progress = Some((51, 51));
        st.clear_preview();
        assert!(
            st.download_progress.hash_progress.is_none(),
            "a fresh install must not inherit the previous install's hash 51/51"
        );
        assert!(
            st.download_progress.extract_progress.is_none(),
            "a fresh install must not flash the previous install's extract 51/51"
        );
    }

    #[test]
    fn idle_stages_are_the_three_pre_pipeline_stages() {
        assert!(install_stage_is_idle(InstallStage::Gallery));
        assert!(install_stage_is_idle(InstallStage::Details));
        assert!(install_stage_is_idle(InstallStage::Paste));
    }

    #[test]
    fn pipeline_stages_are_not_idle() {
        assert!(!install_stage_is_idle(InstallStage::Downloading));
        assert!(!install_stage_is_idle(InstallStage::InstallingStub));
    }

    #[test]
    fn clear_preview_closes_the_drawer_and_forgets_its_query() {
        let mut st = InstallScreenState {
            drawer: DrawerState {
                open: Some(DrawerKind::IncludedMods),
                logs_tab: 1,
                mods_tab: 1,
                mods_query: "fix".to_string(),
            },
            ..Default::default()
        };
        st.clear_preview();
        assert!(st.drawer.open.is_none());
        assert_eq!(st.drawer.logs_tab, 0);
        assert_eq!(st.drawer.mods_tab, 0);
        assert!(st.drawer.mods_query.is_empty());
    }

    #[test]
    fn drawer_kind_maps_every_inside_click_and_none_to_none() {
        assert_eq!(DrawerKind::from_click(InsideClick::None), None);
        assert_eq!(
            DrawerKind::from_click(InsideClick::IncludedMods),
            Some(DrawerKind::IncludedMods)
        );
        assert_eq!(
            DrawerKind::from_click(InsideClick::WeiduLogs),
            Some(DrawerKind::WeiduLogs)
        );
        assert_eq!(
            DrawerKind::from_click(InsideClick::InstalledRefs),
            Some(DrawerKind::InstalledRefs)
        );
        assert_eq!(
            DrawerKind::from_click(InsideClick::DownloadSources),
            Some(DrawerKind::DownloadSources)
        );
        assert_eq!(
            DrawerKind::from_click(InsideClick::ConfigFiles),
            Some(DrawerKind::ConfigFiles)
        );
    }

    #[test]
    fn route_context_tracks_non_default_install_flow_state() {
        let mut st = InstallScreenState::default();
        assert!(!st.has_route_context());

        st.stage = InstallStage::InstallingStub;
        assert!(st.has_route_context());

        st.reset_to_gallery();
        assert!(!st.has_route_context());

        st.import_code = "BIO:example".to_string();
        assert!(st.has_route_context());
        st.reset_to_gallery();
        assert!(st.import_code.is_empty());
        assert_eq!(st.stage, InstallStage::Gallery);
        assert!(!st.has_route_context());
    }

    #[test]
    fn inside_model_is_built_once_and_dropped_with_the_preview() {
        let preview = ModlistSharePreview {
            bio_version: "0.1.0-test".to_string(),
            game_install: "EET".to_string(),
            install_mode: "build_from_scanned_mods".to_string(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: String::new(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            forked_from: Vec::new(),
        };
        let mut st = InstallScreenState {
            parsed_preview: Some(preview),
            ..Default::default()
        };
        let calls = std::cell::Cell::new(0);
        let load = |_: &str| {
            calls.set(calls.get() + 1);
            SourceTiers::default()
        };
        assert!(st.inside_model(load).is_some());
        assert!(st.inside_model(load).is_some());
        assert_eq!(calls.get(), 1);

        st.clear_preview();
        assert!(st.inside.is_none());
    }

    #[test]
    fn inside_counts_are_built_once_and_dropped_with_the_preview() {
        use crate::app::modlist_share::preview_modlist_share_code;
        use crate::ui::install::gallery::catalog;

        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut st = InstallScreenState {
            parsed_preview: Some(preview),
            ..Default::default()
        };

        let first = st.inside_counts().cloned();
        assert!(first.is_some());
        let second = st.inside_counts().cloned();
        assert_eq!(first, second);

        if let Some(preview) = st.parsed_preview.as_mut() {
            preview.bgee_entries += 1000;
        }
        let third = st.inside_counts().cloned();
        assert_eq!(first, third, "the cached counts must not be rebuilt");

        st.clear_preview();
        assert!(st.inside_counts().is_none());
        assert!(st.inside_counts.is_none());
    }
}
