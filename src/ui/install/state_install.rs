// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::stage_downloading::{DownloadProgress, SkippedMod};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallStage {
    #[default]
    Paste,
    Preview,
    Downloading,
    InstallingStub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewTab {
    #[default]
    Summary,
    BgeeWeidu,
    Bg2eeWeidu,
    UserDownloads,
    InstalledRefs,
    ModConfigs,
}

impl PreviewTab {
    pub const ALL: [Self; 6] = [
        Self::Summary,
        Self::BgeeWeidu,
        Self::Bg2eeWeidu,
        Self::UserDownloads,
        Self::InstalledRefs,
        Self::ModConfigs,
    ];

    #[must_use]
    pub const fn display_label(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::BgeeWeidu => "BGEE WeiDU",
            Self::Bg2eeWeidu => "BG2EE WeiDU",
            Self::UserDownloads => "User Downloads",
            Self::InstalledRefs => "Installed Refs",
            Self::ModConfigs => "Mod Configs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstallPipelineFlags {
    bits: u8,
}

impl InstallPipelineFlags {
    const ARMED: u8 = 0b0_0001;
    const ARCHIVES_STAGED: u8 = 0b0_0010;
    const ARCHIVES_INGESTED: u8 = 0b0_0100;
    const ARCHIVES_VERIFIED: u8 = 0b0_1000;
    const DOWNLOAD_PHASE_STARTED: u8 = 0b1_0000;

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

    pub const fn reset(&mut self) {
        self.bits = 0;
    }

    const fn set_bit(&mut self, bit: u8, value: bool) {
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
    pub destination: String,
    pub destination_choice: Option<DestChoice>,
    pub import_code: String,
    pub(crate) parsed_preview: Option<ModlistSharePreview>,
    pub preview_parse_error: Option<String>,
    pub active_preview_tab: PreviewTab,
    pub fork_info_open: bool,
    pub preview_cached: bool,
    pub download_progress: DownloadProgress,
    pub pipeline_flags: InstallPipelineFlags,
    pub pipeline_arm_error: Option<String>,
    pub expected_archive_meta: Vec<crate::registry::share_export::ArchiveMeta>,
    pub pre_skip_assets: Vec<crate::app::state::Step2UpdateAsset>,
    pub skipped_mods: Vec<SkippedMod>,
    pub expected_archive_sizes: std::collections::BTreeMap<usize, u64>,
    pub skip_indices: std::collections::HashSet<usize>,
    pub download_phase_started: bool,
    pub archive_skip_completed: bool,
}

impl InstallScreenState {
    #[must_use]
    pub fn is_partial(&self) -> bool {
        self.destination_choice == Some(DestChoice::Continue)
    }

    pub fn clear_preview(&mut self) {
        self.parsed_preview = None;
        self.preview_parse_error = None;
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
        self.download_phase_started = false;
        self.archive_skip_completed = false;
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
    fn is_partial_only_for_continue() {
        let mut st = InstallScreenState::default();
        assert!(!st.is_partial());
        st.destination_choice = Some(DestChoice::Clear);
        assert!(!st.is_partial());
        st.destination_choice = Some(DestChoice::Backup);
        assert!(!st.is_partial());
        st.destination_choice = Some(DestChoice::Continue);
        assert!(st.is_partial());
    }

    #[test]
    fn default_stage_is_paste() {
        assert_eq!(InstallScreenState::default().stage, InstallStage::Paste);
    }

    #[test]
    fn preview_tab_labels_are_wireframe_verbatim() {
        let labels: Vec<&str> = PreviewTab::ALL.iter().map(|t| t.display_label()).collect();
        assert_eq!(
            labels,
            vec![
                "Summary",
                "BGEE WeiDU",
                "BG2EE WeiDU",
                "User Downloads",
                "Installed Refs",
                "Mod Configs",
            ]
        );
    }

    #[test]
    fn default_preview_tab_is_summary() {
        assert_eq!(PreviewTab::default(), PreviewTab::Summary);
        assert_eq!(
            InstallScreenState::default().active_preview_tab,
            PreviewTab::Summary
        );
    }

    #[test]
    fn clear_preview_resets_preview_state() {
        let mut st = InstallScreenState::default();
        st.preview_cached = true;
        st.fork_info_open = true;
        st.preview_parse_error = Some("boom".to_string());
        st.pipeline_armed = true;
        st.pipeline_arm_error = Some("arm boom".to_string());
        st.archives_staged = true;
        st.archives_ingested = true;
        st.download_phase_started = true;
        st.archive_skip_completed = true;
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
            !st.download_phase_started,
            "Fix 1e: a re-entry must re-kick the streamer"
        );
        assert!(
            !st.archive_skip_completed,
            "v3 Change B: a re-entry must re-run the async skip pass"
        );
    }
}
