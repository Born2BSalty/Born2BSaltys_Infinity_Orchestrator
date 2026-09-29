// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use crate::app::app_step2_update_download::{self, DownloadRefusal};
use crate::app::state::{
    DownloadOrigin, ManualDownloadReason, ManualDownloadRequest, Step2State, Step2UpdateAsset,
    WizardState,
};
use crate::install_runtime::archive_store;
use crate::install_runtime::manual_archive_probe::{self, ArchiveProbe, ProbeMatch, ProbeRefusal};
use crate::install_runtime::manual_download_watcher::{self, WatchEvent};
use crate::ui::install::manual_downloads_panel::{self, PanelAction};
use crate::ui::install::state_install::{
    ManualDownloadRow, ManualDownloadsState, ManualRowStatus, PipelineKind,
};
use crate::ui::install::sub_flow_footer::{self, BackBtn, LeftActionBtn, PrimaryBtn};
use crate::ui::orchestrator::orchestrator_app::{
    DestinationPrepFlow, OrchestratorApp, PendingInstallDestinationPrep,
};
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{
    self, ConfirmDialog, ConfirmOutcome,
};
use crate::ui::orchestrator::widgets::help_button::{self, HelpPage};
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_input_bg, redesign_pill_danger, redesign_shell_bg,
    redesign_success, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

const CHECK_STAGED: &str = "\u{2713}";

use crate::ui::shared::numeric::{
    f32_from_f64, f64_from_u64, pct_from_fraction, ratio_u64, ratio_usize, unit_f32,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModDownloadStatus {
    #[default]
    Queued,
    Hashing,
    Downloading,
    Extracting,
    Staged,
    Skipped,
}

impl ModDownloadStatus {
    #[must_use]
    pub fn status_text(self) -> String {
        match self {
            Self::Queued => "queued".to_string(),
            Self::Hashing => "checking cache...".to_string(),
            Self::Downloading => "downloading".to_string(),
            Self::Extracting | Self::Staged | Self::Skipped => "downloaded".to_string(),
        }
    }

    #[must_use]
    pub const fn phase_fraction(self) -> f32 {
        match self {
            Self::Queued => 0.0,
            Self::Hashing => 0.1,
            Self::Downloading => 0.15,
            Self::Extracting | Self::Staged | Self::Skipped => 1.0,
        }
    }

    #[must_use]
    pub const fn is_done(self) -> bool {
        matches!(self, Self::Staged | Self::Skipped)
    }

    #[must_use]
    pub const fn download_complete(self) -> bool {
        matches!(self, Self::Extracting | Self::Staged | Self::Skipped)
    }

    #[must_use]
    pub const fn is_queued(self) -> bool {
        matches!(self, Self::Queued)
    }

    #[must_use]
    pub const fn is_hashing(self) -> bool {
        matches!(self, Self::Hashing)
    }

    #[must_use]
    pub const fn is_skipped(self) -> bool {
        matches!(self, Self::Skipped)
    }
}

const fn status_sort_key(s: ModDownloadStatus) -> u8 {
    match s {
        ModDownloadStatus::Hashing => 0,
        ModDownloadStatus::Downloading => 1,
        ModDownloadStatus::Queued => 2,
        ModDownloadStatus::Extracting | ModDownloadStatus::Staged | ModDownloadStatus::Skipped => 3,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDownloadRow {
    pub name: String,
    pub source: String,
    pub status: ModDownloadStatus,
    pub per_byte: Option<(u64, Option<u64>)>,
    pub expected_size: Option<u64>,
}

impl ModDownloadRow {
    #[must_use]
    pub fn bar_fraction(&self) -> f32 {
        if self.status == ModDownloadStatus::Downloading {
            let size = self
                .per_byte
                .and_then(|(_, t)| t)
                .filter(|&t| t > 0)
                .or_else(|| self.expected_size.filter(|&s| s > 0));
            if let Some(size) = size {
                let got = self.per_byte.map_or(0, |(b, _)| b);
                return ratio_u64(got, size);
            }
            return ModDownloadStatus::Downloading.phase_fraction();
        }
        self.status.phase_fraction()
    }

    #[must_use]
    pub fn is_indeterminate(&self) -> bool {
        if self.status != ModDownloadStatus::Downloading {
            return false;
        }
        let has_content_length = matches!(self.per_byte, Some((_, Some(t))) if t > 0);
        let has_baked_size = matches!(self.expected_size, Some(s) if s > 0);
        !has_content_length && !has_baked_size
    }

    #[must_use]
    pub fn download_bytes_pair(&self) -> Option<(u64, u64)> {
        let known_size = self
            .expected_size
            .or_else(|| self.per_byte.and_then(|(_, t)| t).filter(|&t| t > 0));
        match self.status {
            ModDownloadStatus::Skipped
            | ModDownloadStatus::Extracting
            | ModDownloadStatus::Staged => known_size.map(|s| (s, s)),
            ModDownloadStatus::Downloading
            | ModDownloadStatus::Queued
            | ModDownloadStatus::Hashing => {
                let size = known_size?;
                let got = self.per_byte.map_or(0, |(b, _)| b).min(size);
                Some((got, size))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallPhase {
    Hashing,
    #[default]
    Downloading,
    Extracting,
}

impl InstallPhase {
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::Hashing => "Checking cache",
            Self::Downloading => "Downloading",
            Self::Extracting => "Extracting",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkippedMod {
    pub name: String,
    pub source: String,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadProgress {
    pub rows: Vec<ModDownloadRow>,
    pub skipped: Vec<SkippedMod>,
    pub expected_sizes: std::collections::BTreeMap<usize, u64>,
    pub extract_progress: Option<(usize, usize)>,
    pub hash_progress: Option<(usize, usize)>,
}

impl DownloadProgress {
    #[must_use]
    pub fn from_wizard_state_full(
        state: &WizardState,
        prior_skipped: &[SkippedMod],
        prior_expected: &std::collections::BTreeMap<usize, u64>,
        hashed_indices: Option<&std::collections::HashSet<usize>>,
    ) -> Self {
        let s2 = &state.step2;

        let label_done = |list: &[String], label: &str| {
            list.iter().any(|e| {
                e.split(" -> ")
                    .next()
                    .map(str::trim)
                    .is_some_and(|l| l == label)
            })
        };
        let skipped_by_label: std::collections::HashMap<&str, &SkippedMod> =
            prior_skipped.iter().map(|s| (s.name.as_str(), s)).collect();

        let mut rows: Vec<ModDownloadRow> = s2
            .update_selected_update_assets
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let status = if skipped_by_label.contains_key(a.label.as_str()) {
                    ModDownloadStatus::Skipped
                } else {
                    let downloaded = label_done(&s2.update_selected_downloaded_sources, &a.label);
                    let extracted = label_done(&s2.update_selected_extracted_sources, &a.label);
                    if extracted {
                        ModDownloadStatus::Staged
                    } else if downloaded {
                        ModDownloadStatus::Extracting
                    } else if s2.update_selected_download_running {
                        if s2.update_selected_download_bytes.contains_key(&i)
                            && !s2.update_selected_download_done.contains(&i)
                        {
                            ModDownloadStatus::Downloading
                        } else {
                            ModDownloadStatus::Queued
                        }
                    } else if hashed_indices.is_some_and(|h| !h.contains(&i)) {
                        ModDownloadStatus::Hashing
                    } else {
                        ModDownloadStatus::Queued
                    }
                };
                let expected_size = prior_expected
                    .get(&i)
                    .copied()
                    .or_else(|| skipped_by_label.get(a.label.as_str()).and_then(|s| s.size));
                ModDownloadRow {
                    name: a.label.clone(),
                    source: a.source_id.clone(),
                    status,
                    per_byte: s2.update_selected_download_bytes.get(&i).copied(),
                    expected_size,
                }
            })
            .collect();

        rows.sort_by_key(|r| status_sort_key(r.status));

        Self {
            rows,
            skipped: Vec::new(),
            expected_sizes: prior_expected.clone(),
            extract_progress: None,
            hash_progress: None,
        }
    }

    #[must_use]
    pub fn from_wizard_state(state: &WizardState) -> Self {
        Self::from_wizard_state_full(state, &[], &std::collections::BTreeMap::new(), None)
    }
}

impl DownloadProgress {
    #[must_use]
    pub fn phase(&self) -> InstallPhase {
        let any_hashing = self
            .rows
            .iter()
            .any(|r| r.status == ModDownloadStatus::Hashing);
        if any_hashing {
            return InstallPhase::Hashing;
        }
        let any_work = !self.rows.is_empty() || !self.skipped.is_empty();
        let still_fetching = self.rows.iter().any(|r| {
            matches!(
                r.status,
                ModDownloadStatus::Downloading | ModDownloadStatus::Queued
            )
        });
        if any_work && !still_fetching {
            InstallPhase::Extracting
        } else {
            InstallPhase::Downloading
        }
    }

    #[must_use]
    pub fn is_preparing_install(&self) -> bool {
        if self.phase() != InstallPhase::Extracting {
            return false;
        }
        let (c, t) = self.extract_completed_total();
        if t == 0 || c != t {
            return false;
        }
        self.rows.iter().all(|r| {
            matches!(
                r.status,
                ModDownloadStatus::Skipped
                    | ModDownloadStatus::Extracting
                    | ModDownloadStatus::Staged
            )
        })
    }

    #[must_use]
    pub const fn total(&self) -> usize {
        self.rows.len() + self.skipped.len()
    }

    #[must_use]
    pub fn downloaded_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.status.download_complete())
            .count()
            + self.skipped.len()
    }

    #[must_use]
    pub fn extracted_count(&self) -> usize {
        self.rows.iter().filter(|r| r.status.is_done()).count()
    }

    const fn extract_total(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn completed(&self) -> usize {
        match self.phase() {
            InstallPhase::Hashing => self.hash_completed_total().0,
            InstallPhase::Downloading => self.downloaded_count(),
            InstallPhase::Extracting => self.extract_completed_total().0,
        }
    }

    #[must_use]
    pub fn download_overall_fraction(&self) -> f32 {
        if self.rows.is_empty() && self.skipped.is_empty() {
            return 0.0;
        }
        if self.any_row_lacks_known_size() {
            let denom = self.rows.len() + self.skipped.len();
            if denom == 0 {
                return 0.0;
            }
            let downloaded = self
                .rows
                .iter()
                .filter(|r| r.status.download_complete())
                .count()
                + self.skipped.len();
            return ratio_usize(downloaded, denom);
        }
        let mut num: f64 = 0.0;
        let mut den: f64 = 0.0;
        for r in &self.rows {
            if let Some((got, size)) = r.download_bytes_pair() {
                num += f64_from_u64(got);
                den += f64_from_u64(size);
            }
        }
        for s in &self.skipped {
            if let Some(sz) = s.size {
                num += f64_from_u64(sz);
                den += f64_from_u64(sz);
            }
        }
        if den <= 0.0 {
            return 0.0;
        }
        unit_f32(num / den)
    }

    #[must_use]
    pub fn any_row_lacks_known_size(&self) -> bool {
        self.rows.iter().any(|r| {
            let baked = r.expected_size.is_some_and(|s| s > 0);
            let live = r.per_byte.and_then(|(_, t)| t).is_some_and(|t| t > 0);
            !baked && !live
        })
    }

    #[must_use]
    pub fn download_overall_pct(&self) -> u32 {
        pct_from_fraction(self.download_overall_fraction())
    }

    #[must_use]
    pub fn extract_overall_fraction(&self) -> f32 {
        if self.phase() != InstallPhase::Extracting {
            return 0.0;
        }
        if let Some((completed, total)) = self.extract_progress {
            return ratio_usize(completed, total.max(1));
        }
        let to_extract = self.extract_total();
        if to_extract == 0 {
            return 1.0;
        }
        ratio_usize(self.extracted_count(), to_extract)
    }

    #[must_use]
    pub fn extract_overall_pct(&self) -> u32 {
        pct_from_fraction(self.extract_overall_fraction())
    }

    #[must_use]
    pub fn extract_completed_total(&self) -> (usize, usize) {
        if let Some((completed, total)) = self.extract_progress {
            return (completed, total);
        }
        (self.extracted_count(), self.extract_total())
    }

    #[must_use]
    pub fn hash_completed_total(&self) -> (usize, usize) {
        self.hash_progress.unwrap_or((0, 0))
    }

    #[must_use]
    pub fn hash_overall_pct(&self) -> u32 {
        let (n, t) = self.hash_completed_total();
        if t == 0 {
            return 0;
        }
        pct_from_fraction(ratio_usize(n, t))
    }

    #[must_use]
    pub fn all_staged(&self) -> bool {
        if self.rows.is_empty() && self.skipped.is_empty() {
            return false;
        }
        self.rows.iter().all(|r| r.status.is_done())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DownloadScreenCopy {
    pub title: &'static str,
    pub sub: &'static str,
    pub hint: Option<&'static str>,
}

impl DownloadScreenCopy {
    pub const INSTALL: Self = Self {
        title: "Downloading & extracting",
        sub: "fetching mod archives \u{2014} install starts automatically when ready",
        hint: Some("after download: install runs without further prompts (no review step)"),
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadingOutcome {
    #[default]
    Stay,
    Cancel,
    Advance,
    OpenWorkspace,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy: DownloadScreenCopy,
    progress: &DownloadProgress,
) -> DownloadingOutcome {
    let (back_clicked, _) = render_chrome(ui, palette, copy, progress, None, None);
    if back_clicked {
        DownloadingOutcome::Cancel
    } else if progress.all_staged() {
        DownloadingOutcome::Advance
    } else {
        DownloadingOutcome::Stay
    }
}

pub fn render_live(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    copy: DownloadScreenCopy,
) -> DownloadingOutcome {
    use crate::install_runtime::auto_build_driver;

    let palette = orchestrator.theme_palette;
    let inputs = LivePipelineInputs::from(orchestrator);

    arm_pipeline_once(orchestrator, &inputs);
    kick_explicit_resolve_once(orchestrator);
    enter_manual_hold_once(orchestrator, &inputs);
    stage_and_kick_archive_skip_once(orchestrator, &inputs);
    kick_streaming_downloader_once(orchestrator);
    verify_downloaded_archives_once(orchestrator, &inputs.destination);
    ingest_downloaded_archives_once(orchestrator, &inputs.destination);

    if orchestrator
        .install_screen_state
        .manual_downloads
        .continue_without
        && !orchestrator
            .install_screen_state
            .manual_downloads
            .extract_deferred
        && super::stage_fork_download::fork_extract_complete(orchestrator)
    {
        return DownloadingOutcome::OpenWorkspace;
    }
    if let Some(outcome) = install_empty_asset_clean_finish(orchestrator) {
        return outcome;
    }
    gate_pre_step5_blocker_once(orchestrator);
    toast_version_override_warnings_once(orchestrator);

    let progress = build_and_hold_progress(orchestrator);
    let arm_error = orchestrator.install_screen_state.pipeline_arm_error.clone();
    let (back_clicked, panel_action) = render_chrome(
        ui,
        palette,
        copy,
        &progress,
        arm_error.as_deref(),
        Some(&mut orchestrator.install_screen_state.manual_downloads),
    );

    match panel_action {
        PanelAction::OpenPage(index) => open_manual_page(orchestrator, index),
        PanelAction::PickFile(index) => pick_manual_file(orchestrator, index),
        PanelAction::None => {}
    }

    render_manual_confirm_dialog(ui, orchestrator, palette);

    if back_clicked {
        return DownloadingOutcome::Cancel;
    }
    if auto_build_driver::pipeline_reached_install(&orchestrator.wizard_state) {
        return DownloadingOutcome::Advance;
    }
    auto_build_driver::log_if_pipeline_stopped(&orchestrator.wizard_state);
    DownloadingOutcome::Stay
}

pub(crate) struct LivePipelineInputs {
    pub(crate) destination: String,
    pub(crate) game: crate::registry::model::Game,
    pub(crate) workflow: crate::install_runtime::flag_policies::InstallWorkflow,
    pub(crate) code: String,
}

impl LivePipelineInputs {
    pub(crate) fn from(
        orchestrator: &crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    ) -> Self {
        Self::from_workflow(
            orchestrator,
            crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
        )
    }

    pub(crate) fn from_workflow(
        orchestrator: &crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
        workflow: crate::install_runtime::flag_policies::InstallWorkflow,
    ) -> Self {
        let state = &orchestrator.install_screen_state;
        let destination = state.destination.trim().to_string();
        let game = state
            .parsed_preview
            .as_ref()
            .map(|p| crate::registry::model::Game::from_legacy_string(&p.game_install))
            .unwrap_or_default();
        let code = state.import_code.trim().to_string();
        Self {
            destination,
            game,
            workflow,
            code,
        }
    }
}

pub(crate) fn arm_pipeline_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    inputs: &LivePipelineInputs,
) {
    use crate::install_runtime::destination_prep;
    use std::sync::mpsc::TryRecvError;

    let flow = install_destination_prep_flow(orchestrator.install_screen_state.pipeline_kind);

    if orchestrator.install_screen_state.pipeline_flags.armed()
        || orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_some()
    {
        return;
    }

    orchestrator
        .wizard_state
        .step2
        .skipped_manual_downloads
        .clear();

    if let Some(pending) = orchestrator.install_destination_prep_rx.as_ref() {
        if !pending.matches_context(
            orchestrator.destination_prep_generation,
            flow,
            &inputs.destination,
            inputs.game,
            inputs.workflow,
            &inputs.code,
        ) {
            orchestrator.abandon_install_destination_prep();
            return;
        }

        match pending.worker.rx.try_recv() {
            Ok(Ok(report)) => {
                let Some(pending) = orchestrator.install_destination_prep_rx.take() else {
                    return;
                };
                if !pending.matches_context(
                    orchestrator.destination_prep_generation,
                    flow,
                    &inputs.destination,
                    inputs.game,
                    inputs.workflow,
                    &inputs.code,
                ) {
                    orchestrator.complete_destination_prep_worker(pending.worker);
                    return;
                }
                tracing::info!(
                    target = "orchestrator",
                    "Install screen destination prep finished: {report:?}"
                );
                let pending_inputs = LivePipelineInputs {
                    destination: pending.destination.clone(),
                    game: pending.game,
                    workflow: pending.workflow,
                    code: pending.code.clone(),
                };
                orchestrator.complete_destination_prep_worker(pending.worker);
                finish_pipeline_arm_after_destination_prep(orchestrator, &pending_inputs);
            }
            Ok(Err(msg)) => {
                if let Some(pending) = orchestrator.install_destination_prep_rx.take() {
                    orchestrator.complete_destination_prep_worker(pending.worker);
                }
                set_pipeline_arm_error(orchestrator, &msg);
            }
            Err(TryRecvError::Empty) => {
                orchestrator.wizard_state.step2.scan_status =
                    "Auto Build: preparing target destination".to_string();
            }
            Err(TryRecvError::Disconnected) => {
                if let Some(pending) = orchestrator.install_destination_prep_rx.take() {
                    orchestrator.complete_destination_prep_worker(pending.worker);
                }
                set_pipeline_arm_error(
                    orchestrator,
                    "destination prep failed: worker disconnected",
                );
            }
        }
        return;
    }

    let dest_path = std::path::PathBuf::from(inputs.destination.trim());
    let token = orchestrator.next_destination_prep_token(flow, &inputs.destination, None);
    orchestrator.install_destination_prep_rx = Some(PendingInstallDestinationPrep {
        token,
        destination: inputs.destination.clone(),
        game: inputs.game,
        workflow: inputs.workflow,
        code: inputs.code.clone(),
        worker: destination_prep::spawn_prepare_destination_worker(
            dest_path,
            orchestrator.install_screen_state.destination_choice,
        ),
    });
    orchestrator.wizard_state.step2.scan_status =
        "Auto Build: preparing target destination".to_string();
}

pub(crate) const fn install_destination_prep_flow(kind: PipelineKind) -> DestinationPrepFlow {
    match kind {
        PipelineKind::Fork => DestinationPrepFlow::CreateForkDownload,
        PipelineKind::Install => DestinationPrepFlow::InstallPipeline,
    }
}

fn finish_pipeline_arm_after_destination_prep(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    inputs: &LivePipelineInputs,
) {
    use crate::install_runtime::active_modlist_source_path;
    use crate::install_runtime::auto_build_driver;
    use crate::install_runtime::install_modlist_registration;

    orchestrator
        .install_screen_state
        .pipeline_flags
        .set_armed(true);

    let held_id = match orchestrator.install_screen_state.pipeline_kind {
        PipelineKind::Fork => orchestrator.active_install_modlist_id.clone(),
        PipelineKind::Install => orchestrator.pending_reinstall_id.clone(),
    };

    let early_mint_result = if auto_build_driver::is_share_code_consuming(inputs.workflow) {
        match install_modlist_registration::early_mint_modlist_id(
            orchestrator,
            &inputs.destination,
            held_id.as_deref(),
        ) {
            Ok(result) => result,
            Err(err) => {
                set_pipeline_arm_error(orchestrator, &err);
                return;
            }
        }
    } else {
        None
    };

    if let Some((ref id, _)) = early_mint_result {
        active_modlist_source_path::set_ambient_for_modlist(id);
    }

    match auto_build_driver::prepare_install_dirs_and_maybe_import(
        &mut orchestrator.wizard_state,
        &inputs.destination,
        inputs.game,
        inputs.workflow,
        &inputs.code,
    ) {
        Ok(_) => {
            let settings: crate::settings::model::Step1Settings =
                orchestrator.wizard_state.step1.clone().into();
            crate::install_runtime::flag_policies::apply_flags(
                &mut orchestrator.wizard_state.step1,
                inputs.workflow,
                &settings,
            );
            let mods_archive_folder = orchestrator
                .settings_store
                .load()
                .map(|settings| {
                    let from: crate::app::state::Step1State = settings.step1.into();
                    from.mods_archive_folder
                })
                .unwrap_or_default();
            auto_build_driver::arm_download_archive_policy(
                &mut orchestrator.wizard_state,
                &mods_archive_folder,
            );
            install_modlist_registration::register_and_write_install_start_artifacts(
                orchestrator,
                early_mint_result.as_ref().map(|(id, _)| id.as_str()),
            );
        }
        Err(err) => {
            if let Some((ref id, true)) = early_mint_result {
                install_modlist_registration::rollback_early_minted_entry(orchestrator, id);
            }
            set_pipeline_arm_error(orchestrator, &err);
            tracing::warn!(
                target = "orchestrator",
                "pipeline arm failed: {err} (Downloading stays navigable; surfaced on-screen)"
            );
        }
    }
}

fn set_pipeline_arm_error(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    msg: &str,
) {
    orchestrator.install_screen_state.pipeline_arm_error = Some(msg.to_string());
    orchestrator.wizard_state.step2.scan_status = format!("Auto Build could not start: {msg}");
    tracing::warn!(
        target = "orchestrator",
        "{msg} (Downloading stays navigable; surfaced on-screen)"
    );
}

pub(crate) fn kick_explicit_resolve_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
) {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    if !flags.armed()
        || !orchestrator.wizard_state.modlist_auto_build_active
        || orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_some()
        || flags.explicit_resolve_started()
    {
        return;
    }
    orchestrator
        .install_screen_state
        .pipeline_flags
        .set_explicit_resolve_started(true);
    crate::install_runtime::auto_build_driver::drive_explicit_resolve(
        &mut orchestrator.wizard_state,
        &mut orchestrator.step2_update_check_rx,
    );
    tracing::info!(
        target = "orchestrator",
        "explicit resolve fired — update-check worker started"
    );
}

fn manual_row_url_host(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let after_scheme = trimmed.split_once("://").map_or(trimmed, |(_, rest)| rest);
    let host = after_scheme.split(['/', '?']).next()?;
    let host = if host
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("www."))
    {
        &host[4..]
    } else {
        host
    };
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

fn manual_row_from_request(request: &ManualDownloadRequest) -> ManualDownloadRow {
    let from = match &request.reason {
        ManualDownloadReason::NoSourceEntry => "no source entry".to_string(),
        ManualDownloadReason::NotAutoResolvable | ManualDownloadReason::SourceCheckFailed(_) => {
            manual_row_url_host(&request.page_url)
                .unwrap_or_else(|| "source check failed".to_string())
        }
    };
    let display_name = request.display_name.trim();
    let label = if display_name.is_empty() {
        request.label.clone()
    } else {
        display_name.to_string()
    };
    ManualDownloadRow {
        label,
        from,
        page_url: request.page_url.clone(),
        status: ManualRowStatus::Waiting,
    }
}

const fn manual_hold_pending(orchestrator: &OrchestratorApp) -> bool {
    orchestrator
        .install_screen_state
        .manual_downloads
        .rows
        .is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_manual_downloads
            .is_empty()
}

const fn manual_hold_entry_ready(orchestrator: &OrchestratorApp) -> bool {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    flags.armed()
        && orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_none()
        && flags.explicit_resolve_started()
        && !flags.archives_staged()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_check_running
}

fn enter_manual_hold_once(orchestrator: &mut OrchestratorApp, inputs: &LivePipelineInputs) {
    enter_manual_hold_once_with_poll(orchestrator, inputs, Duration::from_secs(2));
}

fn enter_manual_hold_once_with_poll(
    orchestrator: &mut OrchestratorApp,
    inputs: &LivePipelineInputs,
    poll: Duration,
) -> Option<std::sync::Arc<()>> {
    if !manual_hold_entry_ready(orchestrator) || !manual_hold_pending(orchestrator) {
        return None;
    }
    let archive_dir = orchestrator
        .wizard_state
        .step1
        .mods_archive_folder
        .trim()
        .to_string();
    let rows: Vec<ManualDownloadRow> = orchestrator
        .wizard_state
        .step2
        .update_selected_manual_downloads
        .iter()
        .map(manual_row_from_request)
        .collect();
    orchestrator.install_screen_state.manual_downloads.rows = rows;
    orchestrator
        .install_screen_state
        .manual_downloads
        .watched_folder
        .clone_from(&archive_dir);
    let expected =
        crate::registry::share_export::decode_archive_meta(&inputs.code).unwrap_or_default();
    orchestrator
        .install_screen_state
        .expected_archive_meta
        .clone_from(&expected);
    let baseline: Vec<(PathBuf, u64, std::time::SystemTime)> = if archive_dir.is_empty() {
        Vec::new()
    } else {
        std::fs::read_dir(&archive_dir).map_or_else(
            |_| Vec::new(),
            |read_dir| {
                read_dir
                    .flatten()
                    .filter_map(|entry| {
                        let metadata = entry.metadata().ok()?;
                        if !metadata.is_file() {
                            return None;
                        }
                        let modified = metadata.modified().ok()?;
                        Some((entry.path(), metadata.len(), modified))
                    })
                    .collect()
            },
        )
    };
    orchestrator.install_screen_state.manual_downloads.baseline = baseline;
    tracing::info!(
        target = "orchestrator",
        "manual downloads hold entered for {} request(s); watching {archive_dir}",
        orchestrator
            .install_screen_state
            .manual_downloads
            .rows
            .len(),
    );
    if archive_dir.is_empty() {
        return None;
    }
    let wanted_sizes: HashSet<u64> = expected.iter().map(|meta| meta.size).collect();
    let ignored_names: HashSet<String> = orchestrator
        .wizard_state
        .step2
        .update_selected_update_assets
        .iter()
        .map(|asset| crate::app::app_step2_update_download::archive_file_name(asset).to_lowercase())
        .collect();
    let (rx, alive) = manual_download_watcher::start_watch(
        PathBuf::from(&archive_dir),
        wanted_sizes,
        ignored_names,
        poll,
    );
    orchestrator.manual_download_rx = Some(rx);
    Some(alive)
}

fn push_manual_unmatched(manual: &mut ManualDownloadsState, name: String) {
    if !manual.unmatched.contains(&name) {
        manual.unmatched.push(name);
    }
}

fn remove_manual_request_label(step2: &mut Step2State, request: &ManualDownloadRequest) {
    match &request.reason {
        ManualDownloadReason::NotAutoResolvable => {
            step2
                .update_selected_manual_sources
                .retain(|label| label != &request.label);
        }
        ManualDownloadReason::NoSourceEntry => {
            step2
                .update_selected_unknown_sources
                .retain(|label| label != &request.label);
        }
        ManualDownloadReason::SourceCheckFailed(_) => {
            let prefix = format!("{}:", request.label);
            step2
                .update_selected_failed_sources
                .retain(|entry| !entry.starts_with(&prefix));
        }
    }
}

fn decompose_manual_store_name(store_name: &str, tp2_prefix: &str) -> (String, String) {
    let ext = crate::app::app_step2_update_download::archive_extension(store_name);
    let stem = store_name.strip_suffix(ext.as_str()).unwrap_or(store_name);
    let rest = stem
        .strip_prefix(&format!("{tp2_prefix}__"))
        .unwrap_or(stem);
    rest.split_once("__").map_or_else(
        || ("manual".to_string(), rest.to_string()),
        |(source, tag)| (source.to_string(), tag.to_string()),
    )
}

fn apply_manual_match(
    orchestrator: &mut OrchestratorApp,
    index: usize,
    probe_path: &Path,
    archive_dir: &Path,
    result: ProbeMatch,
    expected: &[crate::registry::share_export::ArchiveMeta],
) -> bool {
    let store_name = match result {
        ProbeMatch::Meta { store_name } | ProbeMatch::Tp2 { store_name } => store_name,
    };
    let Some(request) = orchestrator
        .wizard_state
        .step2
        .update_selected_manual_downloads
        .get(index)
        .cloned()
    else {
        return false;
    };
    let target = archive_dir.join(&store_name);
    let placed = if probe_path != target && target.exists() {
        let wanted_sizes: HashSet<u64> = expected.iter().map(|meta| meta.size).collect();
        let verified = manual_archive_probe::probe_archive(&target, &wanted_sizes)
            .ok()
            .and_then(|target_probe| {
                manual_archive_probe::match_request(&target_probe, &request, expected).ok()
            });
        if verified.is_none() {
            let file_name = probe_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("file");
            set_manual_refusal(
                orchestrator,
                index,
                format!("{file_name}: a different file already sits at {store_name}"),
            );
            return false;
        }
        target
    } else {
        let already_handled = orchestrator
            .install_screen_state
            .manual_downloads
            .handled
            .iter()
            .any(|path| path == probe_path);
        let placement = if already_handled {
            manual_archive_probe::copy_into_store(probe_path, archive_dir, &store_name)
        } else {
            manual_archive_probe::place_into_store(probe_path, archive_dir, &store_name)
        };
        match placement {
            Ok(path) => path,
            Err(err) => {
                push_manual_unmatched(
                    &mut orchestrator.install_screen_state.manual_downloads,
                    format!("{store_name}: {err}"),
                );
                return false;
            }
        }
    };
    orchestrator
        .install_screen_state
        .manual_downloads
        .handled
        .push(placed);
    if let Some(row) = orchestrator
        .install_screen_state
        .manual_downloads
        .rows
        .get_mut(index)
    {
        row.status = ManualRowStatus::Found;
    }
    let tp2_prefix = crate::app::app_step2_update_download::tp2_archive_name(&request.tp_file);
    let (source_id, tag) = decompose_manual_store_name(&store_name, &tp2_prefix);
    let dest = archive_dir.join(&store_name);
    orchestrator
        .wizard_state
        .step2
        .update_selected_update_assets
        .push(Step2UpdateAsset {
            game_tab: request.game_tab.clone(),
            tp_file: request.tp_file.clone(),
            label: request.label.clone(),
            source_id,
            tag,
            asset_name: store_name,
            asset_url: String::new(),
            installed_source_ref: None,
        });
    push_downloaded_entry_once(
        &mut orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources,
        &request.label,
        &dest,
    );
    remove_manual_request_label(&mut orchestrator.wizard_state.step2, &request);
    true
}

fn find_manual_match(
    orchestrator: &OrchestratorApp,
    probe: &ArchiveProbe,
    requests: &[ManualDownloadRequest],
    expected: &[crate::registry::share_export::ArchiveMeta],
) -> Option<(usize, ProbeMatch)> {
    for (index, row) in orchestrator
        .install_screen_state
        .manual_downloads
        .rows
        .iter()
        .enumerate()
    {
        if !matches!(
            row.status,
            ManualRowStatus::Waiting | ManualRowStatus::Refused(_)
        ) {
            continue;
        }
        let request = requests.get(index)?;
        if let Ok(result) = manual_archive_probe::match_request(probe, request, expected) {
            return Some((index, result));
        }
    }
    None
}

fn handle_manual_candidate(orchestrator: &mut OrchestratorApp, probe: &ArchiveProbe) {
    if orchestrator
        .install_screen_state
        .manual_downloads
        .handled
        .contains(&probe.path)
    {
        return;
    }
    let requests = orchestrator
        .wizard_state
        .step2
        .update_selected_manual_downloads
        .clone();
    let expected = orchestrator
        .install_screen_state
        .expected_archive_meta
        .clone();
    let archive_dir = PathBuf::from(orchestrator.wizard_state.step1.mods_archive_folder.trim());
    let mut current = probe.clone();
    let mut matched_any = false;
    while let Some((index, result)) =
        find_manual_match(orchestrator, &current, &requests, &expected)
    {
        matched_any = true;
        let store_name = match &result {
            ProbeMatch::Meta { store_name } | ProbeMatch::Tp2 { store_name } => store_name.clone(),
        };
        let placed = apply_manual_match(
            orchestrator,
            index,
            &current.path,
            &archive_dir,
            result,
            &expected,
        );
        if !placed {
            break;
        }
        current.path = archive_dir.join(&store_name);
    }
    if !matched_any && !probe_matches_baseline(orchestrator, probe) {
        push_manual_unmatched(
            &mut orchestrator.install_screen_state.manual_downloads,
            probe.file_name.clone(),
        );
    }
}

fn probe_matches_baseline(orchestrator: &OrchestratorApp, probe: &ArchiveProbe) -> bool {
    let Ok(modified) = std::fs::metadata(&probe.path).and_then(|meta| meta.modified()) else {
        return false;
    };
    orchestrator
        .install_screen_state
        .manual_downloads
        .baseline
        .iter()
        .any(|(path, size, base_modified)| {
            path == &probe.path && *size == probe.size && *base_modified == modified
        })
}

pub(crate) fn drain_manual_download_events(orchestrator: &mut OrchestratorApp) {
    loop {
        let event = match orchestrator.manual_download_rx.as_ref() {
            Some(rx) => rx.try_recv(),
            None => return,
        };
        match event {
            Ok(WatchEvent::Tick) => {}
            Ok(WatchEvent::Candidate(probe)) => handle_manual_candidate(orchestrator, &probe),
            Ok(WatchEvent::Error(msg)) => {
                push_manual_unmatched(&mut orchestrator.install_screen_state.manual_downloads, msg);
            }
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                orchestrator.manual_download_rx = None;
                return;
            }
        }
        let rows = &orchestrator.install_screen_state.manual_downloads.rows;
        if !rows.is_empty() && rows.iter().all(|row| row.status == ManualRowStatus::Found) {
            orchestrator.manual_download_rx = None;
            return;
        }
    }
}

fn set_manual_refusal(orchestrator: &mut OrchestratorApp, index: usize, reason: String) {
    if let Some(row) = orchestrator
        .install_screen_state
        .manual_downloads
        .rows
        .get_mut(index)
    {
        row.status = ManualRowStatus::Refused(reason.clone());
    }
    orchestrator
        .install_screen_state
        .manual_downloads
        .last_refusal = Some(reason);
}

fn pick_manual_file(orchestrator: &mut OrchestratorApp, index: usize) {
    if orchestrator
        .wizard_state
        .step1
        .mods_archive_folder
        .trim()
        .is_empty()
    {
        set_manual_refusal(
            orchestrator,
            index,
            "set your Mods archive folder in Settings \u{2192} Paths first".to_string(),
        );
        return;
    }
    let Some(path) = rfd::FileDialog::new().pick_file() else {
        return;
    };
    let expected = orchestrator
        .install_screen_state
        .expected_archive_meta
        .clone();
    let wanted_sizes: HashSet<u64> = expected.iter().map(|meta| meta.size).collect();
    let probe = match manual_archive_probe::probe_archive(&path, &wanted_sizes) {
        Ok(probe) => probe,
        Err(err) => {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("file")
                .to_string();
            set_manual_refusal(orchestrator, index, format!("{name}: {err}"));
            return;
        }
    };
    let Some(request) = orchestrator
        .wizard_state
        .step2
        .update_selected_manual_downloads
        .get(index)
        .cloned()
    else {
        return;
    };
    match manual_archive_probe::match_request(&probe, &request, &expected) {
        Ok(result) => {
            let archive_dir =
                PathBuf::from(orchestrator.wizard_state.step1.mods_archive_folder.trim());
            apply_manual_match(
                orchestrator,
                index,
                &probe.path,
                &archive_dir,
                result,
                &expected,
            );
        }
        Err(refusal) => {
            let reason = match refusal {
                ProbeRefusal::Unsupported => {
                    format!("{}: not a zip, 7z, rar or tar.gz archive", probe.file_name)
                }
                ProbeRefusal::NoTp2 { wanted } => {
                    format!("{}: no {wanted} inside", probe.file_name)
                }
            };
            set_manual_refusal(orchestrator, index, reason);
        }
    }
}

fn open_manual_page(orchestrator: &mut OrchestratorApp, index: usize) {
    let Some(url) = orchestrator
        .install_screen_state
        .manual_downloads
        .rows
        .get(index)
        .map(|row| row.page_url.clone())
    else {
        return;
    };
    let target = manual_archive_probe::page_url_to_open(&url);
    if let Err(err) = crate::app::controller::util::open_in_shell(&target) {
        orchestrator
            .notification_manager
            .error(format!("Could not open the page: {err}"));
    }
}

fn confirm_continue_without(orchestrator: &mut OrchestratorApp) {
    let labels: Vec<String> = orchestrator
        .install_screen_state
        .manual_downloads
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.status != ManualRowStatus::Found)
        .map(|(index, row)| {
            orchestrator
                .wizard_state
                .step2
                .update_selected_manual_downloads
                .get(index)
                .map_or_else(|| row.label.clone(), |request| request.label.clone())
        })
        .collect();
    orchestrator
        .install_screen_state
        .manual_downloads
        .continue_without = true;
    for row in &mut orchestrator.install_screen_state.manual_downloads.rows {
        if row.status != ManualRowStatus::Found {
            row.status = ManualRowStatus::Skipped;
        }
    }
    orchestrator.wizard_state.step2.skipped_manual_downloads = labels;
    orchestrator
        .install_screen_state
        .manual_downloads
        .confirm_open = false;
    orchestrator.manual_download_rx = None;
}

fn manual_continue_without_title(pending: usize) -> String {
    if pending == 1 {
        "Continue without 1 mod?".to_string()
    } else {
        format!("Continue without {pending} mods?")
    }
}

fn render_manual_confirm_dialog(
    ui: &egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) {
    if !orchestrator
        .install_screen_state
        .manual_downloads
        .confirm_open
    {
        return;
    }
    let pending = orchestrator
        .install_screen_state
        .manual_downloads
        .pending_count();
    let labels: Vec<String> = orchestrator
        .install_screen_state
        .manual_downloads
        .rows
        .iter()
        .filter(|row| row.status != ManualRowStatus::Found)
        .map(|row| row.label.clone())
        .collect();
    let title = manual_continue_without_title(pending);
    let body = format!(
        "{} will be skipped. The install may fail on anything that depends on them. \
         You'll land on Mods / Components to review the list before installing.",
        labels.join(", ")
    );
    let dialog = ConfirmDialog {
        id_salt: "manual_downloads_continue_without",
        title: &title,
        body: &body,
        confirm_label: "Continue without them",
        cancel_label: "Keep waiting",
        danger: false,
    };
    match confirm_dialog::render(ui.ctx(), palette, &dialog) {
        ConfirmOutcome::Confirmed => confirm_continue_without(orchestrator),
        ConfirmOutcome::Cancelled => {
            orchestrator
                .install_screen_state
                .manual_downloads
                .confirm_open = false;
        }
        ConfirmOutcome::Pending => {}
    }
}

fn install_empty_asset_clean_finish(
    orchestrator: &mut OrchestratorApp,
) -> Option<DownloadingOutcome> {
    if orchestrator
        .install_screen_state
        .manual_downloads
        .manual_hold_active()
    {
        return None;
    }
    let flags = orchestrator.install_screen_state.pipeline_flags;
    if !flags.armed()
        || orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_some()
        || !orchestrator.wizard_state.modlist_auto_build_active
        || !flags.explicit_resolve_started()
        || orchestrator
            .wizard_state
            .step2
            .update_selected_check_running
        || !orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .is_empty()
    {
        return None;
    }
    let continue_without = orchestrator
        .install_screen_state
        .manual_downloads
        .continue_without;
    let skip_count = orchestrator.install_screen_state.skip_indices.len();
    let extracted_count = orchestrator
        .wizard_state
        .step2
        .update_selected_extracted_sources
        .len();
    let archives_observed = extracted_count + skip_count;
    let blocker = crate::app::app_step2_saved_log_flow::unresolved_required_mods_blocker(
        &orchestrator.wizard_state,
    );
    if !continue_without && let Some(reason) = blocker {
        orchestrator.install_screen_state.pipeline_arm_error = Some(reason.clone());
        orchestrator.wizard_state.modlist_auto_build_active = false;
        orchestrator
            .wizard_state
            .modlist_auto_build_waiting_for_install = false;
        tracing::warn!(target = "orchestrator", "{reason}");
        return None;
    }
    let assets_still_empty = orchestrator
        .wizard_state
        .step2
        .update_selected_update_assets
        .is_empty();
    if assets_still_empty && (archives_observed > 0 || !flags.archives_staged()) {
        if continue_without {
            let step2 = &orchestrator.wizard_state.step2;
            let post_extract_scan_running = step2.is_scanning
                || step2.pending_saved_log_apply
                || step2.pending_saved_log_update_preview
                || step2.update_selected_download_running
                || step2.update_selected_extract_running;
            if post_extract_scan_running
                || orchestrator
                    .install_screen_state
                    .manual_downloads
                    .extract_deferred
            {
                return None;
            }
            tracing::info!(
                target = "orchestrator",
                "install path: continue-without with zero assets — routing to workspace"
            );
            return Some(DownloadingOutcome::OpenWorkspace);
        }
        tracing::info!(
            target = "orchestrator",
            "install path: zero assets after resolve — routing to Step 5"
        );
        route_install_to_step5(&mut orchestrator.wizard_state);
    }
    None
}

fn gate_pre_step5_blocker_once(orchestrator: &mut OrchestratorApp) {
    if orchestrator
        .install_screen_state
        .manual_downloads
        .manual_hold_active()
    {
        return;
    }
    let flags = orchestrator.install_screen_state.pipeline_flags;
    if !flags.armed()
        || !flags.explicit_resolve_started()
        || orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_some()
        || !orchestrator.wizard_state.modlist_auto_build_active
        || orchestrator
            .wizard_state
            .step2
            .update_selected_check_running
        || !flags.archives_ingested()
        || orchestrator.install_screen_state.pre_step5_blocker_checked
    {
        return;
    }
    orchestrator.install_screen_state.pre_step5_blocker_checked = true;
    if orchestrator
        .install_screen_state
        .manual_downloads
        .continue_without
    {
        return;
    }
    if let Some(reason) = crate::app::app_step2_saved_log_flow::unresolved_required_mods_blocker(
        &orchestrator.wizard_state,
    ) {
        orchestrator.install_screen_state.pipeline_arm_error = Some(reason.clone());
        orchestrator.wizard_state.modlist_auto_build_active = false;
        orchestrator
            .wizard_state
            .modlist_auto_build_waiting_for_install = false;
        tracing::warn!(target = "orchestrator", "{reason}");
    }
}

fn toast_version_override_warnings_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
) {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    if !flags.explicit_resolve_started()
        || orchestrator
            .wizard_state
            .step2
            .update_selected_check_running
        || flags.override_warnings_toasted()
    {
        return;
    }
    orchestrator
        .install_screen_state
        .pipeline_flags
        .set_override_warnings_toasted(true);
    let warnings = &orchestrator
        .wizard_state
        .step2
        .update_selected_version_override_warnings;
    if warnings.is_empty() {
        return;
    }
    let message = build_version_override_toast(warnings);
    orchestrator.notification_manager.warn_persistent(message);
}

fn build_version_override_toast(warnings: &[String]) -> String {
    let header = "Pinned versions of the following mods not available. Latest will be installed:";
    format!("{header}\n- {}", warnings.join("\n- "))
}

fn route_install_to_step5(state: &mut crate::app::state::WizardState) {
    state.modlist_auto_build_active = false;
    state.modlist_auto_build_waiting_for_install = false;
    state.step2.update_selected_popup_open = false;
    state.step2.update_selected_confirm_latest_fallback_open = false;
    state.step2.mod_download_forks_popup_open = false;
    state.current_step = 4;
    state.step5.start_install_requested = false;
    state.step5.last_status_text = "Auto Build: ready to install".to_string();
}

const fn cache_check_ready(orchestrator: &OrchestratorApp, inputs: &LivePipelineInputs) -> bool {
    !orchestrator
        .install_screen_state
        .pipeline_flags
        .archives_staged()
        && orchestrator.install_screen_state.pipeline_flags.armed()
        && !inputs.destination.is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .is_empty()
}

pub(crate) fn stage_and_kick_archive_skip_once(
    orchestrator: &mut OrchestratorApp,
    inputs: &LivePipelineInputs,
) {
    if cache_check_ready(orchestrator, inputs) && !manual_hold_pending(orchestrator) {
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_archives_staged(true);
        archive_store::stage_known_archives(&mut orchestrator.wizard_state, &inputs.destination);

        let expected =
            crate::registry::share_export::decode_archive_meta(&inputs.code).unwrap_or_default();
        orchestrator.install_screen_state.pre_skip_assets = orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .clone();

        let by_name: std::collections::HashMap<&str, &crate::registry::share_export::ArchiveMeta> =
            expected.iter().map(|m| (m.name.as_str(), m)).collect();
        let expected_sizes: std::collections::BTreeMap<usize, u64> = orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                let name = crate::app::app_step2_update_download::archive_file_name(a);
                by_name.get(name.as_str()).map(|m| (i, m.size))
            })
            .collect();
        orchestrator.install_screen_state.skipped_mods = Vec::new();
        orchestrator
            .wizard_state
            .step2
            .update_selected_download_bytes
            .clear();
        orchestrator
            .wizard_state
            .step2
            .update_selected_download_done
            .clear();
        orchestrator
            .wizard_state
            .step2
            .update_selected_extract_progress = None;
        orchestrator.install_screen_state.expected_archive_sizes = expected_sizes;
        orchestrator.install_screen_state.skip_indices = std::collections::HashSet::new();
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_archive_skip_completed(false);
        orchestrator
            .install_screen_state
            .expected_archive_meta
            .clone_from(&expected);

        let archive_dir_pb =
            std::path::PathBuf::from(orchestrator.wizard_state.step1.mods_archive_folder.trim());
        let input = crate::install_runtime::archive_skip_async::AsyncSkipInput {
            archive_dir: archive_dir_pb,
            assets: orchestrator
                .wizard_state
                .step2
                .update_selected_update_assets
                .clone(),
        };
        let rx =
            crate::install_runtime::archive_skip_async::start_async_archive_skip(input, expected);
        orchestrator.archive_skip_rx = Some(rx);
        tracing::info!(
            target = "orchestrator",
            "async checksum-then-skip pool spawned for {} asset(s); {} \
             baked expected sizes carried forward",
            orchestrator
                .wizard_state
                .step2
                .update_selected_update_assets
                .len(),
            orchestrator
                .install_screen_state
                .expected_archive_sizes
                .len(),
        );
    }
}

pub(crate) fn kick_streaming_downloader_once(orchestrator: &mut OrchestratorApp) {
    use crate::install_runtime::auto_build_driver;

    if orchestrator.install_screen_state.pipeline_flags.armed()
        && orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_none()
        && auto_build_driver::download_gate_open(&orchestrator.wizard_state)
        && !orchestrator
            .install_screen_state
            .pipeline_flags
            .download_phase_started()
        && orchestrator
            .install_screen_state
            .pipeline_flags
            .archive_skip_completed()
    {
        orchestrator
            .wizard_state
            .modlist_auto_build_waiting_for_install = true;
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_download_phase_started(true);
        let mut skip_indices = orchestrator.install_screen_state.skip_indices.clone();
        mark_empty_url_assets_as_cache_hits(orchestrator, &mut skip_indices);
        match app_step2_update_download::start_step2_update_download_scoped(
            &mut orchestrator.wizard_state,
            &mut orchestrator.step2_update_download_rx,
            None,
            &skip_indices,
            DownloadOrigin::InstallPipeline,
        ) {
            Ok(()) => tracing::info!(
                target = "orchestrator",
                "parallel streaming downloader spawned for {} asset(s); \
                 bypasses {} skipped index/indices",
                orchestrator
                    .wizard_state
                    .step2
                    .update_selected_update_assets
                    .len(),
                skip_indices.len()
            ),
            Err(DownloadRefusal::NothingToDownload) => {
                tracing::info!(
                    target = "orchestrator",
                    "every asset is already on disk; finishing the download stage at once"
                );
                orchestrator.after_download_finished();
            }
            Err(refusal) => tracing::warn!(
                target = "orchestrator",
                ?refusal,
                "parallel streaming downloader not started"
            ),
        }
    }
}

fn downloaded_entry_already_recorded(sources: &[String], label: &str) -> bool {
    sources.iter().any(|entry| {
        entry
            .split_once(" -> ")
            .is_some_and(|(l, _)| l.trim() == label)
    })
}

fn push_downloaded_entry_once(sources: &mut Vec<String>, label: &str, dest: &Path) {
    if !downloaded_entry_already_recorded(sources, label) {
        sources.push(format!("{label} -> {}", dest.display()));
    }
}

fn mark_empty_url_assets_as_cache_hits(
    orchestrator: &mut OrchestratorApp,
    skip_indices: &mut HashSet<usize>,
) {
    let archive_dir = PathBuf::from(orchestrator.wizard_state.step1.mods_archive_folder.trim());
    for (index, asset) in orchestrator
        .wizard_state
        .step2
        .update_selected_update_assets
        .iter()
        .enumerate()
    {
        if asset.asset_url.trim().is_empty() {
            skip_indices.insert(index);
            let dest = archive_dir.join(crate::app::app_step2_update_download::archive_file_name(
                asset,
            ));
            push_downloaded_entry_once(
                &mut orchestrator
                    .wizard_state
                    .step2
                    .update_selected_downloaded_sources,
                &asset.label,
                &dest,
            );
        }
    }
}

pub(crate) fn verify_downloaded_archives_once(
    orchestrator: &mut OrchestratorApp,
    destination: &str,
) {
    if !orchestrator
        .install_screen_state
        .pipeline_flags
        .archives_verified()
        && !destination.is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        && orchestrator
            .install_screen_state
            .pipeline_flags
            .download_phase_started()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty()
    {
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_archives_verified(true);
        let expected = orchestrator
            .install_screen_state
            .expected_archive_meta
            .clone();
        let skip_indices = orchestrator.install_screen_state.skip_indices.clone();
        let pre_skip: Vec<_> = orchestrator
            .install_screen_state
            .pre_skip_assets
            .iter()
            .enumerate()
            .filter(|(i, _)| !skip_indices.contains(i))
            .map(|(_, a)| a.clone())
            .collect();
        let v = crate::install_runtime::archive_skip::verify_downloaded_archives(
            &mut orchestrator.wizard_state,
            &expected,
            &pre_skip,
        );
        tracing::info!(
            target = "orchestrator",
            "post-download verify: {} verified, {} hash-mismatched \
             (deleted + recorded failed, NOT installed), {} unverifiable",
            v.verified,
            v.mismatched,
            v.unverifiable
        );
    }
}

pub(crate) fn ingest_downloaded_archives_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    destination: &str,
) {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    let destination_empty = destination.is_empty();
    let download_running = orchestrator
        .wizard_state
        .step2
        .update_selected_download_running;
    let downloaded_sources = orchestrator
        .wizard_state
        .step2
        .update_selected_downloaded_sources
        .len();
    let should_ingest = !flags.archives_ingested()
        && !destination_empty
        && !download_running
        && flags.download_phase_started()
        && downloaded_sources > 0
        && !orchestrator
            .install_screen_state
            .manual_downloads
            .manual_hold_active();
    if flags.download_phase_started() && !flags.archives_ingested() && !download_running {
        tracing::info!(
            target = "orchestrator",
            destination_empty,
            download_running,
            download_phase_started = flags.download_phase_started(),
            downloaded_sources,
            should_ingest,
            "downloaded archive ingest gate"
        );
    }
    if should_ingest {
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_archives_ingested(true);
        let names: Vec<String> = orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .iter()
            .map(crate::app::app_step2_update_download::archive_file_name)
            .collect();
        archive_store::ingest_downloaded_archives(&orchestrator.wizard_state, destination, &names);
        tracing::info!(
            target = "orchestrator",
            archive_names = names.len(),
            "downloaded archive ingest latch set"
        );
    }
}

pub(crate) fn build_and_hold_progress(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
) -> DownloadProgress {
    let prior_skipped = orchestrator.install_screen_state.skipped_mods.clone();
    let prior_expected = orchestrator
        .install_screen_state
        .expected_archive_sizes
        .clone();
    let hash_pass_active = orchestrator.install_screen_state.pipeline_flags.armed()
        && !orchestrator
            .install_screen_state
            .pipeline_flags
            .archive_skip_completed();
    let hashed: Option<&std::collections::HashSet<usize>> = if hash_pass_active {
        Some(&orchestrator.install_screen_state.hashed_indices)
    } else {
        None
    };
    let mut progress = DownloadProgress::from_wizard_state_full(
        &orchestrator.wizard_state,
        &prior_skipped,
        &prior_expected,
        hashed,
    );
    let step2 = &orchestrator.wizard_state.step2;
    progress.extract_progress = (step2.update_selected_download_origin
        == DownloadOrigin::InstallPipeline)
        .then_some(step2.update_selected_extract_progress)
        .flatten();
    progress.hash_progress = orchestrator.hash_progress.lock().ok().and_then(|g| *g);

    let hold_prior_grid = progress.rows.is_empty()
        && !orchestrator
            .install_screen_state
            .download_progress
            .rows
            .is_empty()
        && orchestrator.install_screen_state.pipeline_flags.armed()
        && orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_none()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty();
    if hold_prior_grid {
        orchestrator.install_screen_state.download_progress.clone()
    } else {
        orchestrator.install_screen_state.download_progress = progress.clone();
        progress
    }
}

const fn downloading_middle_height(available_height: f32) -> f32 {
    (available_height - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0)
}

pub(crate) fn render_chrome(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy: DownloadScreenCopy,
    progress: &DownloadProgress,
    arm_error: Option<&str>,
    manual: Option<&mut ManualDownloadsState>,
) -> (bool, PanelAction) {
    let help_page = HelpPage::Downloading {
        manual_downloads: manual.is_some(),
    };
    ui.horizontal_top(|ui| {
        let title_width = (ui.available_width() - 30.0).max(0.0);
        ui.vertical(|ui| {
            ui.set_width(title_width);
            render_screen_title(ui, palette, copy.title, Some(copy.sub));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            help_button::render(ui, palette, help_page);
        });
    });
    ui.add_space(12.0);

    let mut panel_action = PanelAction::None;
    let mut hold_active = false;
    let mut left_action_label = String::new();
    let waiting: Option<String> = manual.as_deref().and_then(|state| {
        (state.manual_hold_active() && (state.extract_deferred || progress.rows.is_empty()))
            .then(|| state.waiting_label())
    });

    let body_h = downloading_middle_height(ui.available_height());
    ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(err) = arm_error {
                    render_arm_error_banner(ui, palette, err);
                    ui.add_space(14.0);
                }

                render_overall_progress(ui, palette, copy.hint, progress, waiting.as_deref());
                ui.add_space(14.0);

                (panel_action, hold_active, left_action_label) = manual.as_deref().map_or_else(
                    || (PanelAction::None, false, String::new()),
                    |state| {
                        let panel_action = if state.rows.is_empty() {
                            PanelAction::None
                        } else {
                            let action = manual_downloads_panel::render(
                                ui,
                                palette,
                                state,
                                box_frame(palette),
                                state.last_refusal.as_deref(),
                            );
                            ui.add_space(14.0);
                            action
                        };
                        (
                            panel_action,
                            state.manual_hold_active(),
                            state.continue_label(),
                        )
                    },
                );

                let grid_budget = (ui.available_height() - 8.0).max(140.0);
                render_mod_progress(ui, palette, progress, grid_budget);
            });
    });

    let left_action = if hold_active {
        Some(LeftActionBtn {
            label: &left_action_label,
        })
    } else {
        None
    };

    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Cancel" }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        None,
        left_action,
        PrimaryBtn {
            label: waiting.as_deref().unwrap_or("Waiting\u{2026}"),
            disabled: true,
        },
    );
    if footer == sub_flow_footer::FooterClick::LeftAction
        && let Some(state) = manual
    {
        state.confirm_open = true;
    }
    (footer == sub_flow_footer::FooterClick::Back, panel_action)
}

fn render_overall_progress(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    hint: Option<&str>,
    progress: &DownloadProgress,
    waiting_headline: Option<&str>,
) {
    box_frame(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("overall progress")
                .size(11.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.add_space(6.0);

        let phase = progress.phase();
        let (h_n, h_total) = progress.hash_completed_total();
        let h_pct = progress.hash_overall_pct();
        let dl_total = progress.total();
        let dl_n = progress.downloaded_count();
        let dl_pct = progress.download_overall_pct();
        let (ex_n, ex_total) = progress.extract_completed_total();
        let ex_pct = progress.extract_overall_pct();

        let preparing = progress.is_preparing_install();
        let phase_line = waiting_headline.map_or_else(
            || {
                if preparing {
                    "Preparing to install \u{2026}".to_string()
                } else {
                    let (verb, n, t, p) = match phase {
                        InstallPhase::Hashing => {
                            (InstallPhase::Hashing.verb(), h_n, h_total, h_pct)
                        }
                        InstallPhase::Downloading => {
                            (InstallPhase::Downloading.verb(), dl_n, dl_total, dl_pct)
                        }
                        InstallPhase::Extracting => {
                            (InstallPhase::Extracting.verb(), ex_n, ex_total, ex_pct)
                        }
                    };
                    format!("{verb} \u{2026} {n} / {t} mods \u{00B7} {p}%")
                }
            },
            ToString::to_string,
        );
        ui.label(
            egui::RichText::new(phase_line)
                .size(15.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(8.0);

        phase_bar_row(
            ui,
            palette,
            PhaseBarParams {
                verb: "hash",
                n: h_n,
                total: h_total,
                pct: h_pct,
                frac: f64::from(h_pct) / 100.0,
                active: phase == InstallPhase::Hashing,
            },
        );
        ui.add_space(8.0);
        phase_bar_row(
            ui,
            palette,
            PhaseBarParams {
                verb: "download",
                n: dl_n,
                total: dl_total,
                pct: dl_pct,
                frac: f64::from(dl_pct) / 100.0,
                active: phase == InstallPhase::Downloading,
            },
        );
        ui.add_space(8.0);
        phase_bar_row(
            ui,
            palette,
            PhaseBarParams {
                verb: "extract",
                n: ex_n,
                total: ex_total,
                pct: ex_pct,
                frac: f64::from(ex_pct) / 100.0,
                active: phase == InstallPhase::Extracting && !preparing,
            },
        );

        if let Some(h) = hint {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(h)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
            );
        }
    });
}

#[derive(Clone, Copy)]
struct PhaseBarParams<'a> {
    verb: &'a str,
    n: usize,
    total: usize,
    pct: u32,
    frac: f64,
    active: bool,
}

fn phase_bar_row(ui: &mut egui::Ui, palette: ThemePalette, p: PhaseBarParams<'_>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let (label_rect, _) = ui.allocate_exact_size(egui::vec2(180.0, 18.0), egui::Sense::hover());
        let cap_color = if p.active {
            redesign_text_primary(palette)
        } else {
            redesign_text_faint(palette)
        };
        ui.painter().text(
            egui::pos2(label_rect.left(), label_rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{} {} / {} \u{00B7} {}%", p.verb, p.n, p.total, p.pct),
            egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
            cap_color,
        );

        let bar_w = ui.available_width();
        let (track, _) = ui.allocate_exact_size(egui::vec2(bar_w, 14.0), egui::Sense::hover());
        paint_phase_bar(ui, palette, track, p.frac, p.active);
    });
}

fn paint_phase_bar(
    ui: &egui::Ui,
    palette: ThemePalette,
    track: egui::Rect,
    frac: f64,
    active: bool,
) {
    if !ui.is_rect_visible(track) {
        return;
    }
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));
    let frac = unit_f32(frac);
    if frac > 0.0 {
        let fill = if active {
            redesign_accent(palette)
        } else {
            redesign_text_faint(palette)
        };
        let fill_rect =
            egui::Rect::from_min_size(track.min, egui::vec2(track.width() * frac, track.height()));
        painter.rect_filled(fill_rect, radius, fill);
    }
    painter.rect_stroke(
        track,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

fn render_mod_progress(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    progress: &DownloadProgress,
    max_h: f32,
) {
    box_frame(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("mod progress")
                .size(11.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.add_space(8.0);

        let col_gap = 12.0;
        let status_w = 170.0;
        let prog_w = 130.0;
        let flex_total = (ui.available_width() - status_w - prog_w - col_gap * 3.0).max(120.0);
        let mod_w = flex_total * (1.8 / 2.8);
        let src_w = flex_total * (1.0 / 2.8);

        egui::Grid::new("stage_downloading_mod_grid_header")
            .num_columns(4)
            .spacing(egui::vec2(col_gap, 6.0))
            .min_col_width(0.0)
            .show(ui, |ui| {
                grid_header(ui, palette, "mod", mod_w);
                grid_header(ui, palette, "source", src_w);
                grid_header(ui, palette, "status", status_w);
                grid_header(ui, palette, "progress", prog_w);
                ui.end_row();
            });

        if progress.rows.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("no mods queued")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
            );
            return;
        }

        let scroll_h = (max_h - 64.0).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("stage_downloading_mod_scroll")
            .max_height(scroll_h)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("stage_downloading_mod_grid")
                    .num_columns(4)
                    .spacing(egui::vec2(col_gap, 6.0))
                    .min_col_width(0.0)
                    .show(ui, |ui| {
                        for row in &progress.rows {
                            render_grid_row(ui, palette, row, mod_w, src_w, status_w, prog_w);
                            ui.end_row();
                        }
                    });
            });
    });
}

fn render_grid_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    row: &ModDownloadRow,
    mod_w: f32,
    src_w: f32,
    status_w: f32,
    prog_w: f32,
) {
    let name_color = if row.status.is_queued() {
        redesign_text_faint(palette)
    } else {
        redesign_text_primary(palette)
    };
    sized_label(ui, mod_w, &row.name, 14.0, "poppins_medium", name_color);

    sized_label(
        ui,
        src_w,
        &row.source,
        13.0,
        "poppins_light",
        redesign_text_faint(palette),
    );

    let status_color = if row.status.download_complete() {
        redesign_success(palette)
    } else if row.status.is_queued() {
        redesign_text_faint(palette)
    } else {
        redesign_text_primary(palette)
    };
    if row.status.download_complete() {
        check_prose_cell(ui, status_w, "downloaded", status_color);
    } else {
        sized_label(
            ui,
            status_w,
            &row.status.status_text(),
            14.0,
            "poppins_medium",
            status_color,
        );
    }

    let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(prog_w, 14.0), egui::Sense::hover());
    if row.is_indeterminate() {
        paint_indeterminate_bar(ui, palette, bar_rect);
    } else {
        paint_bar(
            ui,
            palette,
            bar_rect,
            f64::from(row.bar_fraction()),
            !row.status.is_queued(),
        );
    }
}

pub(super) fn check_prose_cell(ui: &mut egui::Ui, w: f32, prose: &str, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let glyph_font = egui::FontId::new(14.0, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(14.0, egui::FontFamily::Name("poppins_medium".into()));
    let glyph_galley = painter.layout_no_wrap(CHECK_STAGED.to_string(), glyph_font.clone(), color);
    let gap = 5.0;
    let cy = rect.center().y;
    painter.text(
        egui::pos2(rect.left(), cy),
        egui::Align2::LEFT_CENTER,
        CHECK_STAGED,
        glyph_font,
        color,
    );
    painter.text(
        egui::pos2(rect.left() + glyph_galley.size().x + gap, cy),
        egui::Align2::LEFT_CENTER,
        prose,
        prose_font,
        color,
    );
}

fn grid_header(ui: &mut egui::Ui, palette: ThemePalette, text: &str, w: f32) {
    sized_label(
        ui,
        w,
        text,
        14.0,
        "poppins_light",
        redesign_text_muted(palette),
    );
}

pub(super) fn sized_label(
    ui: &mut egui::Ui,
    w: f32,
    text: &str,
    size: f32,
    family: &'static str,
    color: egui::Color32,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            egui::pos2(rect.left(), rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::new(size, egui::FontFamily::Name(family.into())),
            color,
        );
    }
}

fn paint_bar(ui: &egui::Ui, palette: ThemePalette, track: egui::Rect, frac: f64, filled: bool) {
    if !ui.is_rect_visible(track) {
        return;
    }
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));
    if filled {
        let frac = unit_f32(frac);
        if frac > 0.0 {
            let fill_rect = egui::Rect::from_min_size(
                track.min,
                egui::vec2(track.width() * frac, track.height()),
            );
            painter.rect_filled(fill_rect, radius, redesign_accent(palette));
        }
    }
    painter.rect_stroke(
        track,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

fn paint_indeterminate_bar(ui: &egui::Ui, palette: ThemePalette, track: egui::Rect) {
    if !ui.is_rect_visible(track) {
        return;
    }
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));

    let t = f32_from_f64(ui.input(|i| i.time));
    let block = (track.width() * 0.28).max(8.0);
    let travel = (track.width() - block).max(0.0);
    let phase = (t / 1.6).fract(); // 0..1
    let tri = if phase < 0.5 {
        phase * 2.0
    } else {
        2.0 - phase * 2.0
    }; // 0→1→0
    let x = track.left() + travel * tri;
    let block_rect = egui::Rect::from_min_size(
        egui::pos2(x, track.top()),
        egui::vec2(block, track.height()),
    );
    painter.rect_filled(block_rect, radius, redesign_accent(palette));

    painter.rect_stroke(
        track,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

fn render_arm_error_banner(ui: &mut egui::Ui, palette: ThemePalette, err: &str) {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_pill_danger(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new("could not start the download")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_bold".into()))
                    .color(redesign_pill_danger(palette)),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(err)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Click Cancel, fix the import code or destination, and try again.",
                )
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
            );
        });
}

fn box_frame(palette: ThemePalette) -> egui::Frame {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(14))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn row(name: &str, status: ModDownloadStatus) -> ModDownloadRow {
        ModDownloadRow {
            name: name.to_string(),
            source: "src".to_string(),
            status,
            per_byte: None,
            expected_size: None,
        }
    }

    fn row_b(status: ModDownloadStatus, per_byte: Option<(u64, Option<u64>)>) -> ModDownloadRow {
        ModDownloadRow {
            name: "m".to_string(),
            source: "src".to_string(),
            status,
            per_byte,
            expected_size: None,
        }
    }

    fn row_sz(
        status: ModDownloadStatus,
        per_byte: Option<(u64, Option<u64>)>,
        expected_size: Option<u64>,
    ) -> ModDownloadRow {
        ModDownloadRow {
            name: "m".to_string(),
            source: "src".to_string(),
            status,
            per_byte,
            expected_size,
        }
    }

    fn skipped(name: &str, size: Option<u64>) -> SkippedMod {
        SkippedMod {
            name: name.to_string(),
            source: "github".to_string(),
            size,
        }
    }

    #[test]
    fn downloading_middle_height_leaves_room_for_footer() {
        for available in [700.0_f32, 820.0, 900.0] {
            let middle = downloading_middle_height(available);
            assert!((available - middle - sub_flow_footer::FOOTER_HEIGHT_PX).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn status_text_has_no_fabricated_pct_and_unified_downloaded_caption_v3() {
        assert_eq!(ModDownloadStatus::Queued.status_text(), "queued");
        assert_eq!(
            ModDownloadStatus::Hashing.status_text(),
            "checking cache..."
        );
        assert_eq!(ModDownloadStatus::Downloading.status_text(), "downloading");
        assert_eq!(ModDownloadStatus::Extracting.status_text(), "downloaded");
        assert_eq!(ModDownloadStatus::Staged.status_text(), "downloaded");
        assert_eq!(ModDownloadStatus::Skipped.status_text(), "downloaded");
        for s in [
            ModDownloadStatus::Queued,
            ModDownloadStatus::Hashing,
            ModDownloadStatus::Downloading,
            ModDownloadStatus::Extracting,
            ModDownloadStatus::Staged,
            ModDownloadStatus::Skipped,
        ] {
            assert!(
                !s.status_text().contains('%'),
                "no fabricated per-row % in any status caption ({s:?})"
            );
        }
    }

    #[test]
    fn is_done_is_queued_is_skipped_download_complete_are_correct() {
        assert!(ModDownloadStatus::Queued.is_queued());
        assert!(!ModDownloadStatus::Queued.is_done());
        assert!(!ModDownloadStatus::Queued.download_complete());

        assert!(ModDownloadStatus::Staged.is_done());
        assert!(ModDownloadStatus::Staged.download_complete());
        assert!(!ModDownloadStatus::Staged.is_skipped());

        assert!(ModDownloadStatus::Skipped.is_done());
        assert!(ModDownloadStatus::Skipped.download_complete());
        assert!(ModDownloadStatus::Skipped.is_skipped());
        assert!(!ModDownloadStatus::Skipped.is_queued());

        assert!(ModDownloadStatus::Extracting.download_complete());
        assert!(!ModDownloadStatus::Extracting.is_done());
        assert!(!ModDownloadStatus::Downloading.download_complete());
    }

    #[test]
    fn phase_fraction_is_monotonic_with_v3_collapsed_terminals() {
        let queued = ModDownloadStatus::Queued.phase_fraction();
        let hashing = ModDownloadStatus::Hashing.phase_fraction();
        let downloading = ModDownloadStatus::Downloading.phase_fraction();
        let extracting = ModDownloadStatus::Extracting.phase_fraction();
        let staged = ModDownloadStatus::Staged.phase_fraction();
        let skipped = ModDownloadStatus::Skipped.phase_fraction();
        assert!(
            queued < hashing && hashing < downloading,
            "strictly increasing queued < hashing < downloading"
        );
        assert!(
            downloading < extracting,
            "Downloading < Extracting (collapsed terminal)"
        );
        assert!((queued - 0.0).abs() < f32::EPSILON);
        assert!(
            (extracting - 1.0).abs() < f32::EPSILON,
            "Extracting collapses to 1.0"
        );
        assert!(
            (staged - 1.0).abs() < f32::EPSILON,
            "Staged is the fully-satisfied terminal (1.0)"
        );
        assert!(
            (skipped - 1.0).abs() < f32::EPSILON,
            "Skipped is a fully-satisfied terminal (1.0)"
        );
    }

    #[test]
    fn bar_fraction_is_the_whole_byte_fraction_no_band_clamp() {
        let half = row_b(ModDownloadStatus::Downloading, Some((50, Some(100))));
        assert!((half.bar_fraction() - 0.5).abs() < 0.001, "50/100 ⇒ 0.5");

        let almost = row_b(ModDownloadStatus::Downloading, Some((999, Some(1000))));
        assert!(
            almost.bar_fraction() > 0.98,
            "byte-near-complete ⇒ a near-full bar (no 0.64 band-clamp), got {}",
            almost.bar_fraction()
        );

        let full = row_b(ModDownloadStatus::Downloading, Some((1000, Some(1000))));
        assert!((full.bar_fraction() - 1.0).abs() < f32::EPSILON);
        let over = row_b(ModDownloadStatus::Downloading, Some((1200, Some(1000))));
        assert!((over.bar_fraction() - 1.0).abs() < f32::EPSILON);

        let a = row_b(ModDownloadStatus::Downloading, Some((10, Some(100)))).bar_fraction();
        let b = row_b(ModDownloadStatus::Downloading, Some((60, Some(100)))).bar_fraction();
        assert!(b >= a);
    }

    #[test]
    fn bar_fraction_no_content_length_is_indeterminate_not_a_fake_pct() {
        let nub = row_b(ModDownloadStatus::Downloading, Some((123_456, None)));
        assert!(
            (nub.bar_fraction() - ModDownloadStatus::Downloading.phase_fraction()).abs()
                < f32::EPSILON
        );
        assert!(nub.is_indeterminate(), "no Content-Length ⇒ indeterminate");
        let zero = row_b(ModDownloadStatus::Downloading, Some((10, Some(0))));
        assert!(
            (zero.bar_fraction() - ModDownloadStatus::Downloading.phase_fraction()).abs()
                < f32::EPSILON
        );
        assert!(zero.is_indeterminate());
        assert!(!row_b(ModDownloadStatus::Downloading, Some((5, Some(10)))).is_indeterminate());
        assert!(!row_b(ModDownloadStatus::Queued, Some((0, None))).is_indeterminate());
    }

    #[test]
    fn bar_fraction_is_strictly_monotonic_from_zero_no_reverse_nub_jerk() {
        let sz = Some(600_000u64);
        let none_yet = row_sz(ModDownloadStatus::Downloading, None, sz);
        assert!(
            none_yet.bar_fraction().abs() < f32::EPSILON,
            "no bytes yet ⇒ empty bar, NOT a 0.15 nub"
        );
        assert!(
            !none_yet.is_indeterminate(),
            "a baked size ⇒ determinate (a real bytes/size bar), never the marquee"
        );
        let seq: Vec<f32> = [0u64, 65_536, 131_072, 300_000, 599_999, 600_000]
            .iter()
            .map(|&b| {
                row_sz(ModDownloadStatus::Downloading, Some((b, Some(600_000))), sz).bar_fraction()
            })
            .collect();
        for w in seq.windows(2) {
            assert!(w[1] >= w[0], "strictly monotonic from 0: {seq:?}");
        }
        assert!(seq[0].abs() < f32::EPSILON);
        assert!((seq[seq.len() - 1] - 1.0).abs() < 1e-6);
        let cl_only = row_sz(ModDownloadStatus::Downloading, Some((0, Some(1000))), None);
        assert!(cl_only.bar_fraction().abs() < f32::EPSILON);
        assert!(!cl_only.is_indeterminate());
    }

    #[test]
    fn bar_fraction_falls_back_to_phase_when_no_byte_signal() {
        for status in [
            ModDownloadStatus::Queued,
            ModDownloadStatus::Downloading,
            ModDownloadStatus::Extracting,
            ModDownloadStatus::Staged,
            ModDownloadStatus::Skipped,
        ] {
            assert!(
                (row_b(status, None).bar_fraction() - status.phase_fraction()).abs() < f32::EPSILON
            );
        }
        assert!(
            (row_b(ModDownloadStatus::Extracting, Some((100, Some(100)))).bar_fraction()
                - ModDownloadStatus::Extracting.phase_fraction())
            .abs()
                < f32::EPSILON
        );
        assert!(
            (row_b(ModDownloadStatus::Staged, Some((100, Some(100)))).bar_fraction() - 1.0).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn download_bytes_pair_uses_baked_size_then_content_length() {
        let r = row_sz(
            ModDownloadStatus::Downloading,
            Some((30, Some(999))),
            Some(100),
        );
        assert_eq!(r.download_bytes_pair(), Some((30, 100)));
        let r2 = row_sz(ModDownloadStatus::Downloading, Some((30, Some(120))), None);
        assert_eq!(r2.download_bytes_pair(), Some((30, 120)));
        let r3 = row_sz(ModDownloadStatus::Downloading, Some((30, None)), None);
        assert_eq!(r3.download_bytes_pair(), None);
        let ex = row_sz(ModDownloadStatus::Extracting, None, Some(500));
        assert_eq!(ex.download_bytes_pair(), Some((500, 500)));
        let st = row_sz(ModDownloadStatus::Staged, None, Some(500));
        assert_eq!(st.download_bytes_pair(), Some((500, 500)));
        let sk = row_sz(ModDownloadStatus::Skipped, None, Some(500));
        assert_eq!(sk.download_bytes_pair(), Some((500, 500)));
        let over = row_sz(
            ModDownloadStatus::Downloading,
            Some((700, Some(999))),
            Some(500),
        );
        assert_eq!(over.download_bytes_pair(), Some((500, 500)));
    }

    #[test]
    fn phase_is_downloading_until_all_fetched_then_extracting() {
        let p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Downloading),
            ],
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Downloading);
        let p2 = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Extracting),
            ],
            ..Default::default()
        };
        assert_eq!(p2.phase(), InstallPhase::Extracting);
        let p3 = DownloadProgress {
            skipped: vec![skipped("x", Some(10))],
            ..Default::default()
        };
        assert_eq!(p3.phase(), InstallPhase::Extracting);
        assert_eq!(
            DownloadProgress::default().phase(),
            InstallPhase::Downloading
        );
    }

    #[test]
    fn download_overall_is_a_true_byte_aggregate_not_n_over_m() {
        let p = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Extracting, None, Some(100)),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((50, Some(100))),
                    Some(100),
                ),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((0, Some(100))),
                    Some(100),
                ),
            ],
            ..Default::default()
        };
        let f = p.download_overall_fraction();
        assert!(
            (f - 0.5).abs() < 0.001,
            "Σbytes÷Σexpected = 150/300 = 0.5, got {f}"
        );
        assert_eq!(p.download_overall_pct(), 50);
    }

    #[test]
    fn fix_1a_pure_count_fallback_when_any_row_lacks_known_size() {
        let p = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Staged, None, Some(1000)),
                row_sz(ModDownloadStatus::Staged, None, Some(1000)),
                row_sz(ModDownloadStatus::Staged, None, Some(1000)),
                row_sz(ModDownloadStatus::Downloading, Some((100, None)), None),
            ],
            skipped: vec![skipped("c1", None)],
            ..Default::default()
        };
        assert!(p.any_row_lacks_known_size());
        let f = p.download_overall_fraction();
        assert!(
            (f - 0.8).abs() < 1e-4,
            "pure count = (3 complete + 1 skipped) / (4 rows + 1 skipped) = 0.8, got {f}"
        );
    }

    #[test]
    fn fix_1a_homogeneous_known_size_uses_byte_aggregate() {
        let p = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Extracting, None, Some(100)),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((50, Some(100))),
                    Some(100),
                ),
                row_sz(ModDownloadStatus::Downloading, Some((25, Some(100))), None),
            ],
            ..Default::default()
        };
        assert!(!p.any_row_lacks_known_size(), "every row has a known size");
        let f = p.download_overall_fraction();
        assert!(
            (f - (175.0_f32 / 300.0)).abs() < 1e-4,
            "byte aggregate = 175/300, got {f}"
        );
    }

    #[test]
    fn fix_1a_all_skipped_is_full_in_both_modes() {
        let p = DownloadProgress {
            skipped: vec![skipped("a", Some(10)), skipped("b", Some(20))],
            ..Default::default()
        };
        assert!(!p.any_row_lacks_known_size());
        assert!((p.download_overall_fraction() - 1.0).abs() < 1e-6);

        let only_unknown_skipped = DownloadProgress {
            skipped: vec![skipped("a", None)],
            ..Default::default()
        };
        assert!(
            only_unknown_skipped.download_overall_fraction().abs() < f32::EPSILON,
            "no rows + skipped-with-no-size ⇒ no determinate bytes (chrome \
 still flips via all_staged / extract-complete)"
        );
    }

    #[test]
    fn fix_1a_partial_skipped_plus_unknown_to_fetch_counts_skipped_complete() {
        let p = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Downloading, Some((50, None)), None),
                row_sz(ModDownloadStatus::Queued, None, None),
            ],
            skipped: vec![skipped("c1", Some(1000)), skipped("c2", None)],
            ..Default::default()
        };
        assert!(p.any_row_lacks_known_size());
        let f = p.download_overall_fraction();
        assert!(
            (f - 0.5).abs() < 1e-4,
            "pure count = (0 row-complete + 2 skipped) / (2 + 2) = 0.5, got {f}"
        );
        let p_done = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Extracting, Some((50, None)), None),
                row_sz(ModDownloadStatus::Staged, None, None),
            ],
            skipped: vec![skipped("c1", Some(1000)), skipped("c2", None)],
            ..Default::default()
        };
        assert!(p_done.any_row_lacks_known_size());
        assert!((p_done.download_overall_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn download_overall_climbs_smoothly_with_bytes_and_is_monotonic() {
        let mk = |b0: u64, b1: u64| DownloadProgress {
            rows: vec![
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((b0, Some(1000))),
                    Some(1000),
                ),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((b1, Some(1000))),
                    Some(1000),
                ),
            ],
            ..Default::default()
        };
        let f0 = mk(0, 0).download_overall_fraction();
        let f1 = mk(100, 50).download_overall_fraction();
        let f2 = mk(400, 300).download_overall_fraction();
        let f3 = mk(1000, 1000).download_overall_fraction();
        assert!((f0 - 0.0).abs() < 1e-6);
        assert!(f1 > f0 && f2 > f1 && f3 > f2, "strictly climbing");
        assert!((f3 - 1.0).abs() < 1e-6, "byte-complete ⇒ 1.0");
        assert!(f1 < 0.10 && f2 < 0.40);
    }

    #[test]
    fn download_overall_counts_skipped_mods_complete_so_cached_install_is_honest() {
        let skipped48: Vec<SkippedMod> = (0..48)
            .map(|i| skipped(&format!("c{i}"), Some(1000)))
            .collect();
        let p = DownloadProgress {
            rows: vec![
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((0, Some(1000))),
                    Some(1000),
                ),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((0, Some(1000))),
                    Some(1000),
                ),
                row_sz(ModDownloadStatus::Queued, None, Some(1000)),
            ],
            skipped: skipped48,
            ..Default::default()
        };
        let f = p.download_overall_fraction();
        assert!(
            (f - (48.0_f32 / 51.0_f32)).abs() < 0.001,
            "48 of 51 cached ⇒ ~0.941 (honest, not lurched), got {f}"
        );
        assert_eq!(p.total(), 51);
        assert_eq!(p.downloaded_count(), 48, "the 48 skipped are past download");
    }

    #[test]
    fn download_overall_indeterminate_rows_get_a_count_share_so_it_reaches_one() {
        let mk = |complete: bool| DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Staged, None, Some(100)),
                row_sz(
                    if complete {
                        ModDownloadStatus::Staged
                    } else {
                        ModDownloadStatus::Downloading
                    },
                    Some((9999, None)),
                    None,
                ),
            ],
            ..Default::default()
        };
        let mid = mk(false).download_overall_fraction();
        assert!(
            (mid - 0.5).abs() < 1e-6,
            "any-lacks-known-size ⇒ pure count = 1/2 = 0.5, got {mid}"
        );
        assert!((mk(true).download_overall_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn extract_overall_is_separate_zero_until_extract_begins_never_inherits_download() {
        let downloading = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Extracting, None, Some(100)),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((50, Some(100))),
                    Some(100),
                ),
            ],
            ..Default::default()
        };
        assert!(
            downloading.download_overall_fraction() > 0.0,
            "download is in progress"
        );
        assert!(
            downloading.extract_overall_fraction().abs() < f32::EPSILON,
            "Extract is 0 until the extract phase begins (never inherits Download)"
        );
        assert_eq!(downloading.extract_overall_pct(), 0);

        let extracting = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),     // extracted
                row("b", ModDownloadStatus::Extracting), // not yet
            ],
            ..Default::default()
        };
        assert_eq!(extracting.phase(), InstallPhase::Extracting);
        assert!(
            (extracting.extract_overall_fraction() - 0.5).abs() < 0.001,
            "1 of 2 extracted ⇒ 0.5 (its OWN 0→100, count-granular)"
        );
        let done = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Staged),
            ],
            ..Default::default()
        };
        assert!((done.extract_overall_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fix_1c_extract_overall_uses_live_snapshot_when_present() {
        let mut p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Extracting),
                row("b", ModDownloadStatus::Extracting),
            ],
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Extracting);
        assert_eq!(p.extract_overall_pct(), 0);
        p.extract_progress = Some((3, 10));
        assert_eq!(
            p.extract_overall_pct(),
            30,
            "live snapshot (3/10) ⇒ 30%, not the count fallback (0/2)"
        );
        assert_eq!(p.extract_completed_total(), (3, 10));
        assert_eq!(p.completed(), 3, "chrome N tracks the live snapshot");
        p.extract_progress = None;
        assert_eq!(p.extract_completed_total(), (0, 2));
        assert_eq!(p.extract_overall_pct(), 0);
    }

    #[test]
    fn fix_1c_extract_snapshot_only_drives_bar_during_extract_phase() {
        let p = DownloadProgress {
            rows: vec![row_sz(
                ModDownloadStatus::Downloading,
                Some((50, Some(100))),
                Some(100),
            )],
            extract_progress: Some((7, 10)),
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Downloading);
        assert_eq!(
            p.extract_overall_pct(),
            0,
            "Extract is 0 during Download phase, even with a snapshot present"
        );
    }

    #[test]
    fn extract_starts_at_exactly_zero_even_with_skipped_mods() {
        let p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Extracting), // fetched, not yet extracted
                row("b", ModDownloadStatus::Extracting),
            ],
            skipped: vec![skipped("c1", Some(1000)), skipped("c2", Some(2000))],
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Extracting);
        assert_eq!(
            p.extract_overall_pct(),
            0,
            "extract MUST start at exactly 0% (skipped mods don't pre-fill it)"
        );
        assert_eq!(p.download_overall_pct(), 100);
        let mut p2 = p;
        p2.rows[0].status = ModDownloadStatus::Staged;
        assert!(
            (p2.extract_overall_fraction() - 0.5).abs() < 0.001,
            "1 of 2 to-fetch extracted ⇒ 0.5 (skipped NOT in the extract denominator)"
        );
        p2.rows[1].status = ModDownloadStatus::Staged;
        assert!((p2.extract_overall_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fully_cached_install_extract_phase_is_complete_not_a_stuck_zero() {
        let p = DownloadProgress {
            skipped: vec![skipped("a", Some(10)), skipped("b", Some(20))],
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Extracting, "nothing to fetch");
        assert_eq!(p.download_overall_pct(), 100, "all cached ⇒ download done");
        assert_eq!(
            p.extract_overall_pct(),
            100,
            "no extract work ⇒ extract complete (not a stuck 0)"
        );
        assert!(p.all_staged(), "fully-cached ⇒ auto-advance");
    }

    #[test]
    fn extract_never_shows_70_at_51_of_51_downloaded() {
        let p = DownloadProgress {
            rows: (0..51)
                .map(|_| row("m", ModDownloadStatus::Extracting))
                .collect(),
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Extracting);
        assert_eq!(
            p.extract_overall_pct(),
            0,
            "0 extracted ⇒ Extract is 0%, never a conflated 70%"
        );
        assert_eq!(p.downloaded_count(), 51);
    }

    #[test]
    fn completed_tracks_the_live_phase_count() {
        let dl = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Extracting),
                row("b", ModDownloadStatus::Downloading),
            ],
            skipped: vec![skipped("s", Some(1))],
            ..Default::default()
        };
        assert_eq!(dl.phase(), InstallPhase::Downloading);
        assert_eq!(
            dl.completed(),
            dl.downloaded_count(),
            "while downloading the N is the download count"
        );
        assert_eq!(dl.downloaded_count(), 2, "1 Extracting + 1 skipped");
        let ex = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Extracting),
            ],
            ..Default::default()
        };
        assert_eq!(ex.phase(), InstallPhase::Extracting);
        assert_eq!(ex.completed(), ex.extracted_count());
    }

    #[test]
    fn total_counts_rows_plus_skipped() {
        let p = DownloadProgress {
            rows: vec![row("a", ModDownloadStatus::Queued)],
            skipped: vec![skipped("s1", Some(1)), skipped("s2", None)],
            ..Default::default()
        };
        assert_eq!(p.total(), 3, "1 to-fetch + 2 skipped");
    }

    #[test]
    fn all_staged_only_when_every_fetch_row_truly_staged() {
        let mut p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Staged),
            ],
            ..Default::default()
        };
        assert!(p.all_staged());
        p.rows[1].status = ModDownloadStatus::Extracting;
        assert!(
            !p.all_staged(),
            "an Extracting row must NOT auto-advance the stage"
        );
        let cached = DownloadProgress {
            skipped: vec![skipped("s", Some(1))],
            ..Default::default()
        };
        assert!(
            cached.all_staged(),
            "fully-cached (no fetch rows) ⇒ all_staged (auto-advance)"
        );
        assert!(!DownloadProgress::default().all_staged());
    }

    #[test]
    fn empty_progress_is_zero_and_not_complete() {
        let p = DownloadProgress::default();
        assert_eq!(p.completed(), 0);
        assert_eq!(p.total(), 0);
        assert_eq!(p.download_overall_pct(), 0);
        assert_eq!(p.extract_overall_pct(), 0);
        assert!(!p.all_staged());
    }

    #[test]
    fn state_bytes_drive_rows_across_rebuild() {
        use crate::app::state::Step2UpdateAsset;
        let mut st = WizardState::default();
        let mk = |label: &str, src: &str| Step2UpdateAsset {
            game_tab: "BGEE".into(),
            tp_file: format!("{label}/{label}.TP2"),
            label: label.into(),
            source_id: src.into(),
            tag: "v1".into(),
            asset_name: format!("{label}.zip"),
            asset_url: format!("http://x/{label}"),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets = vec![mk("A", "github"), mk("B", "weasel")];
        st.step2.update_selected_download_running = true;
        st.step2
            .update_selected_download_bytes
            .insert(0, (512, Some(2048)));

        let p = DownloadProgress::from_wizard_state(&st);
        assert_eq!(p.rows[0].name, "A");
        assert_eq!(p.rows[0].per_byte, Some((512, Some(2048))));

        let mut expected = BTreeMap::new();
        expected.insert(0usize, 2048u64);
        let p2 = DownloadProgress::from_wizard_state_full(
            &st,
            &[skipped("CACHED", Some(4096))],
            &expected,
            None,
        );
        assert_eq!(
            p2.rows[0].per_byte,
            Some((512, Some(2048))),
            "the state byte map drives every per-frame row rebuild"
        );
        assert_eq!(
            p2.rows[0].expected_size,
            Some(2048),
            "the share-code expected size is merged onto the row"
        );
        assert_eq!(p2.rows[1].per_byte, None, "asset 1 had no byte event yet");
        assert!(
            p2.skipped.is_empty(),
            ": `skipped` is vestigial; not populated"
        );
        assert!((p2.rows[0].bar_fraction() - 0.25).abs() < 0.001);

        st.step2
            .update_selected_download_bytes
            .insert(0, (1024, Some(2048)));
        let p3 = DownloadProgress::from_wizard_state_full(&st, &[], &expected, None);
        assert!((p3.rows[0].bar_fraction() - 0.5).abs() < 0.001);
    }

    #[test]
    fn from_wizard_state_full_v3_classifies_lifecycle_and_sorts_rows() {
        let mut st = WizardState::default();
        let asset = |label: &str, src: &str| crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/{label}.TP2"),
            label: label.to_string(),
            source_id: src.to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{label}.zip"),
            asset_url: format!("https://x/{label}.zip"),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets = vec![
            asset("EET", "github:eet"),
            asset("cdtweaks", "github:cdt"),
            asset("stratagems", "github:scs"),
            asset("spell_rev", "weasel:sr"),
        ];
        st.step2.update_selected_downloaded_sources = vec![
            "EET -> C:/a/EET.zip".to_string(),
            "cdtweaks -> C:/a/cdt.zip".to_string(),
        ];
        st.step2.update_selected_extracted_sources = vec!["EET -> C:/m/EET".to_string()];
        st.step2.update_selected_download_running = true;
        st.step2
            .update_selected_download_bytes
            .insert(3, (10, Some(100)));

        let sk = vec![skipped("ALREADY_HERE", Some(7777))];
        let p = DownloadProgress::from_wizard_state_full(&st, &sk, &BTreeMap::new(), None);
        assert_eq!(p.rows.len(), 4);
        let statuses: Vec<_> = p.rows.iter().map(|r| r.status).collect();
        let names: Vec<_> = p.rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(statuses[0], ModDownloadStatus::Downloading);
        assert_eq!(names[0], "spell_rev", "the in-flight row sits on top");
        assert_eq!(statuses[1], ModDownloadStatus::Queued);
        assert_eq!(names[1], "stratagems", "a row with no bytes yet is queued");
        assert!(statuses[2..].iter().all(|s| s.download_complete()));
        assert_eq!(
            p.skipped.len(),
            0,
            "v3: skipped is not populated by from_wizard_state_full"
        );
        assert_eq!(p.total(), 4, "4 rows (no phantom skipped row)");
    }

    #[test]
    fn failed_row_leaves_downloading_the_frame_its_done_lands() {
        let mut st = WizardState::default();
        let asset = |label: &str| crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/{label}.TP2"),
            label: label.to_string(),
            source_id: "github".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{label}.zip"),
            asset_url: format!("https://x/{label}.zip"),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets = vec![asset("failed"), asset("streaming")];
        st.step2.update_selected_download_running = true;
        st.step2
            .update_selected_download_bytes
            .insert(0, (100, Some(100)));
        st.step2.update_selected_download_done.insert(0);
        st.step2
            .update_selected_download_failed_sources
            .push("failed: HTTP 404".to_string());
        st.step2
            .update_selected_download_bytes
            .insert(1, (10, Some(100)));

        let p = DownloadProgress::from_wizard_state_full(&st, &[], &BTreeMap::new(), None);
        let rows: Vec<_> = p.rows.iter().map(|r| (r.name.as_str(), r.status)).collect();
        assert_eq!(
            rows,
            vec![
                ("streaming", ModDownloadStatus::Downloading),
                ("failed", ModDownloadStatus::Queued),
            ]
        );
    }

    #[test]
    fn hashing_classification_drives_hashing_status_while_hash_pass_alive() {
        let mut st = WizardState::default();
        let asset = |label: &str| crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/{label}.TP2"),
            label: label.to_string(),
            source_id: "github".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{label}.zip"),
            asset_url: format!("https://x/{label}.zip"),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets =
            vec![asset("A"), asset("B"), asset("C"), asset("D")];
        st.step2.update_selected_download_running = false;

        let mut hashed = std::collections::HashSet::new();
        hashed.insert(0usize);
        hashed.insert(2usize);
        let p = DownloadProgress::from_wizard_state_full(&st, &[], &BTreeMap::new(), Some(&hashed));
        assert_eq!(p.rows.len(), 4);
        let hashing_labels: Vec<&str> = p
            .rows
            .iter()
            .filter(|r| r.status == ModDownloadStatus::Hashing)
            .map(|r| r.name.as_str())
            .collect();
        let queued_labels: Vec<&str> = p
            .rows
            .iter()
            .filter(|r| r.status == ModDownloadStatus::Queued)
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(
            hashing_labels,
            vec!["B", "D"],
            "indices NOT in hashed_indices are in-flight Hashing"
        );
        assert_eq!(
            queued_labels,
            vec!["A", "C"],
            "indices in hashed_indices that are not yet downloaded show as Queued"
        );
        assert_eq!(p.phase(), InstallPhase::Hashing);
    }

    #[test]
    fn hashing_classification_off_after_pass_completes_no_more_hashing_rows() {
        let mut st = WizardState::default();
        let asset = |label: &str| crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/{label}.TP2"),
            label: label.to_string(),
            source_id: "github".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{label}.zip"),
            asset_url: format!("https://x/{label}.zip"),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets = vec![asset("A"), asset("B")];
        st.step2.update_selected_download_running = false;
        let p = DownloadProgress::from_wizard_state_full(&st, &[], &BTreeMap::new(), None);
        assert!(
            p.rows
                .iter()
                .all(|r| r.status != ModDownloadStatus::Hashing),
            "after the hash pass finishes the classifier must not produce Hashing rows"
        );
    }

    #[test]
    fn install_copy_is_spec_4_3_verbatim() {
        let c = DownloadScreenCopy::INSTALL;
        assert_eq!(c.title, "Downloading & extracting");
        assert_eq!(
            c.sub,
            "fetching mod archives \u{2014} install starts automatically when ready"
        );
        assert_eq!(
            c.hint,
            Some("after download: install runs without further prompts (no review step)")
        );
    }

    #[test]
    fn render_outcome_chassis_stay_until_all_staged() {
        let mut st = WizardState::default();
        let asset = crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".into(),
            tp_file: "A/A.TP2".into(),
            label: "A".into(),
            source_id: "gh".into(),
            tag: "v1".into(),
            asset_name: "A.zip".into(),
            asset_url: "http://x/A".into(),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets = vec![asset];
        let p = DownloadProgress::from_wizard_state(&st);
        assert!(!p.all_staged());
    }

    #[test]
    fn install_phase_verbs() {
        assert_eq!(InstallPhase::Downloading.verb(), "Downloading");
        assert_eq!(InstallPhase::Extracting.verb(), "Extracting");
        assert_eq!(InstallPhase::default(), InstallPhase::Downloading);
    }

    #[test]
    fn build_version_override_toast_single_entry() {
        let warnings = vec!["ISNF (6.5.5 -> 6.5.6)".to_string()];
        let msg = build_version_override_toast(&warnings);
        assert_eq!(
            msg,
            "Pinned versions of the following mods not available. Latest will be installed:\n- ISNF (6.5.5 -> 6.5.6)"
        );
    }

    #[test]
    fn pipeline_kind_picks_the_destination_prep_flow() {
        assert_eq!(
            install_destination_prep_flow(PipelineKind::Install),
            DestinationPrepFlow::InstallPipeline
        );
        assert_eq!(
            install_destination_prep_flow(PipelineKind::Fork),
            DestinationPrepFlow::CreateForkDownload
        );
    }

    #[test]
    fn build_version_override_toast_multiple_entries() {
        let warnings = vec![
            "ISNF (6.5.5 -> 6.5.6)".to_string(),
            "OtherMod (v0.3 -> v1.0)".to_string(),
        ];
        let msg = build_version_override_toast(&warnings);
        assert_eq!(
            msg,
            "Pinned versions of the following mods not available. Latest will be installed:\n- ISNF (6.5.5 -> 6.5.6)\n- OtherMod (v0.3 -> v1.0)"
        );
    }

    struct ClaimRouteDestGuard(std::path::PathBuf);

    impl ClaimRouteDestGuard {
        fn new(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("bio_claim_route_{tag}_{}", std::process::id()));
            Self(path)
        }

        fn as_string(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
    }

    impl Drop for ClaimRouteDestGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn claim_route_share_payload(name: &str) -> String {
        format!(
            r#"{{
                "format_version": 1,
                "game_install": "BGEE",
                "install_mode": "start_from_scratch",
                "weidu_logs": {{ "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" }},
                "name": "{name}"
            }}"#
        )
    }

    fn claim_route_share_code(name: &str) -> String {
        crate::app::modlist_share::encode_share_payload_text(&claim_route_share_payload(name))
            .expect("mint code")
    }

    fn entry_at(id: &str, name: &str, dest: &str) -> crate::registry::model::ModlistEntry {
        crate::registry::model::ModlistEntry {
            id: id.to_string(),
            name: name.to_string(),
            game: crate::registry::model::Game::EET,
            destination_folder: dest.to_string(),
            state: crate::registry::model::ModlistState::Installed,
            ..Default::default()
        }
    }

    fn arm_with_code(
        app: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
        dest: &str,
        name: &str,
        workflow: crate::install_runtime::flag_policies::InstallWorkflow,
    ) {
        let code = claim_route_share_code(name);
        let preview = crate::app::modlist_share::preview_modlist_share_code(&code)
            .expect("share code decodes");
        app.install_screen_state.destination = dest.to_string();
        app.install_screen_state.import_code = code;
        app.install_screen_state.parsed_preview = Some(preview);
        app.install_screen_state.preview_cached = true;
        app.install_screen_state.destination_choice =
            Some(crate::ui::install::state_install::DestChoice::Clear);
        let inputs = LivePipelineInputs::from_workflow(app, workflow);
        finish_pipeline_arm_after_destination_prep(app, &inputs);
    }

    #[test]
    fn gallery_install_into_an_owned_folder_replaces_that_list() {
        let dest = ClaimRouteDestGuard::new("gallery-replace");
        let dest_s = dest.as_string();
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "claim-gallery-replace",
            );
        assert!(app.wizard_state.step1.bgee_game_folder.is_empty());
        assert!(app.wizard_state.step1.bg2ee_game_folder.is_empty());
        assert!(app.wizard_state.step1.eet_pre_dir.is_empty());
        assert!(app.wizard_state.step1.eet_new_dir.is_empty());
        assert!(app.wizard_state.step1.mods_folder.is_empty());

        app.registry
            .entries
            .push(entry_at("OLD", "OLD name", &dest_s));
        app.install_screen_state.pipeline_kind = PipelineKind::Install;

        let old_data_dir = crate::registry::store_workspace::modlist_data_dir("OLD");
        std::fs::create_dir_all(&old_data_dir).expect("seed OLD data dir");
        assert!(old_data_dir.is_dir());

        arm_with_code(
            &mut app,
            &dest_s,
            "New name",
            crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
        );

        assert_eq!(app.install_screen_state.pipeline_arm_error, None);
        let at_dest: Vec<_> = app
            .registry
            .entries
            .iter()
            .filter(|e| e.destination_folder.trim() == dest_s)
            .collect();
        assert_eq!(
            at_dest.len(),
            1,
            "exactly one entry now owns the destination"
        );
        assert_eq!(at_dest[0].name, "New name");
        assert_ne!(at_dest[0].id, "OLD");
        assert!(app.registry.find("OLD").is_none());
        assert!(app.pending_replaced_entry.is_none());
        assert_eq!(
            app.active_install_modlist_id.as_deref(),
            Some(at_dest[0].id.as_str())
        );
        assert!(!old_data_dir.is_dir(), "OLD's data dir is gone");
    }

    #[test]
    fn fork_route_adopts_the_fork_it_minted() {
        let dest = ClaimRouteDestGuard::new("fork-adopt");
        let dest_s = dest.as_string();
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "claim-fork-adopt",
            );
        assert!(app.wizard_state.step1.bgee_game_folder.is_empty());
        assert!(app.wizard_state.step1.bg2ee_game_folder.is_empty());
        assert!(app.wizard_state.step1.eet_pre_dir.is_empty());
        assert!(app.wizard_state.step1.eet_new_dir.is_empty());
        assert!(app.wizard_state.step1.mods_folder.is_empty());

        app.registry
            .entries
            .push(entry_at("FORK", "My fork", &dest_s));
        app.active_install_modlist_id = Some("FORK".to_string());
        app.install_screen_state.pipeline_kind = PipelineKind::Fork;

        arm_with_code(
            &mut app,
            &dest_s,
            "Parent",
            crate::install_runtime::flag_policies::InstallWorkflow::ForkAndModify,
        );

        assert_eq!(app.install_screen_state.pipeline_arm_error, None);
        assert_eq!(app.registry.entries.len(), 1);
        assert_eq!(
            app.registry.find("FORK").expect("FORK still present").name,
            "My fork"
        );
        assert_eq!(app.active_install_modlist_id.as_deref(), Some("FORK"));
        assert!(app.pending_replaced_entry.is_none());
    }

    #[test]
    fn reinstall_route_adopts_the_reinstalled_list() {
        let dest = ClaimRouteDestGuard::new("reinstall-adopt");
        let dest_s = dest.as_string();
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "claim-reinstall-adopt",
            );
        assert!(app.wizard_state.step1.bgee_game_folder.is_empty());
        assert!(app.wizard_state.step1.bg2ee_game_folder.is_empty());
        assert!(app.wizard_state.step1.eet_pre_dir.is_empty());
        assert!(app.wizard_state.step1.eet_new_dir.is_empty());
        assert!(app.wizard_state.step1.mods_folder.is_empty());

        app.registry
            .entries
            .push(entry_at("RE", "RE name", &dest_s));
        app.pending_reinstall_id = Some("RE".to_string());
        app.install_screen_state.pipeline_kind = PipelineKind::Install;

        arm_with_code(
            &mut app,
            &dest_s,
            "Ignored",
            crate::install_runtime::flag_policies::InstallWorkflow::Reinstall,
        );

        assert_eq!(app.install_screen_state.pipeline_arm_error, None);
        assert_eq!(app.registry.entries.len(), 1);
        assert_eq!(
            app.registry.find("RE").expect("RE still present").name,
            "RE name"
        );
        assert_eq!(app.active_install_modlist_id.as_deref(), Some("RE"));
    }

    #[test]
    fn arming_into_a_folder_another_install_is_using_is_refused() {
        let dest = ClaimRouteDestGuard::new("busy-refused");
        let dest_s = dest.as_string();
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "claim-busy-refused",
            );
        assert!(app.wizard_state.step1.bgee_game_folder.is_empty());
        assert!(app.wizard_state.step1.bg2ee_game_folder.is_empty());
        assert!(app.wizard_state.step1.eet_pre_dir.is_empty());
        assert!(app.wizard_state.step1.eet_new_dir.is_empty());
        assert!(app.wizard_state.step1.mods_folder.is_empty());

        app.registry
            .entries
            .push(entry_at("BUSY", "Busy List", &dest_s));
        app.active_install_modlist_id = Some("BUSY".to_string());
        app.install_screen_state.pipeline_kind = PipelineKind::Install;
        app.pending_reinstall_id = None;

        arm_with_code(
            &mut app,
            &dest_s,
            "New name",
            crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
        );

        let err = app
            .install_screen_state
            .pipeline_arm_error
            .as_deref()
            .expect("arm error");
        assert!(
            err.contains("is installing into this folder right now"),
            "got {err:?}"
        );
        assert_eq!(app.registry.entries.len(), 1);
        assert!(app.registry.find("BUSY").is_some());
        assert!(app.pending_replaced_entry.is_none());
    }

    #[test]
    fn arming_with_no_preview_on_an_owned_folder_is_refused_and_both_lists_survive() {
        let dest = ClaimRouteDestGuard::new("noprev-refused");
        let dest_s = dest.as_string();
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "claim-noprev-refused",
            );
        assert!(app.wizard_state.step1.bgee_game_folder.is_empty());
        assert!(app.wizard_state.step1.bg2ee_game_folder.is_empty());
        assert!(app.wizard_state.step1.eet_pre_dir.is_empty());
        assert!(app.wizard_state.step1.eet_new_dir.is_empty());
        assert!(app.wizard_state.step1.mods_folder.is_empty());

        app.registry
            .entries
            .push(entry_at("OLD", "OLD name", &dest_s));
        app.install_screen_state.destination = dest_s.clone();
        app.install_screen_state.parsed_preview = None;
        app.install_screen_state.destination_choice =
            Some(crate::ui::install::state_install::DestChoice::Clear);
        app.install_screen_state.pipeline_kind = PipelineKind::Install;

        let inputs = LivePipelineInputs::from_workflow(
            &app,
            crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
        );
        finish_pipeline_arm_after_destination_prep(&mut app, &inputs);

        let err = app
            .install_screen_state
            .pipeline_arm_error
            .as_deref()
            .expect("arm error");
        assert!(err.contains("could not be replaced"), "got {err:?}");
        let old = app.registry.find("OLD").expect("OLD still present");
        assert_eq!(old.destination_folder, dest_s);
    }

    struct ManualDlTempRoot {
        path: std::path::PathBuf,
    }

    impl ManualDlTempRoot {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "bio_manualdl_{}_{}_{tag}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for ManualDlTempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn test_asset(label: &str, url: &str) -> Step2UpdateAsset {
        Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/setup-{label}.tp2"),
            label: label.to_string(),
            source_id: "github".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{label}.zip"),
            asset_url: url.to_string(),
            installed_source_ref: None,
        }
    }

    fn manual_request(label: &str) -> ManualDownloadRequest {
        ManualDownloadRequest {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/setup-{label}.tp2"),
            label: label.to_string(),
            source_id: String::new(),
            page_url: "https://www.nexusmods.com/baldursgate2ee/mods/1".to_string(),
            reason: ManualDownloadReason::NotAutoResolvable,
            aliases: Vec::new(),
            display_name: String::new(),
        }
    }

    #[test]
    fn cache_check_fires_while_manual_row_waits() {
        let root = ManualDlTempRoot::new("cache-beside-hold");
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-cache-beside-hold",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.wizard_state.step2.update_selected_check_running = false;
        app.wizard_state.step1.mods_archive_folder =
            root.path.join("archives").to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_update_assets =
            vec![test_asset("ModA", "https://example.com/a.zip")];
        app.wizard_state.step2.update_selected_manual_downloads = vec![manual_request("Ascension")];
        let inputs = LivePipelineInputs {
            destination: "C:/dest".to_string(),
            game: crate::registry::model::Game::BGEE,
            workflow: crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
            code: String::new(),
        };

        let alive = enter_manual_hold_once_with_poll(&mut app, &inputs, Duration::from_millis(20));
        stage_and_kick_archive_skip_once(&mut app, &inputs);

        assert!(
            app.install_screen_state
                .manual_downloads
                .manual_hold_active(),
            "the manual hold is still active"
        );
        assert!(
            app.install_screen_state.pipeline_flags.archives_staged(),
            "the cache check fires on the same frame a manual row is still waiting"
        );
        app.manual_download_rx = None;
        if let Some(alive) = alive {
            manual_download_watcher::wait_for_thread_exit(&alive);
        }
    }

    #[test]
    fn panel_reads_the_extract_counter_only_for_a_pipeline_origin_run() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-panel-extract-origin",
            );
        app.wizard_state.step2.update_selected_update_assets =
            vec![test_asset("ModA", "https://example.com/a.zip")];
        app.wizard_state.step2.update_selected_extract_progress = Some((11, 11));

        app.wizard_state.step2.update_selected_download_origin = DownloadOrigin::Workspace;
        assert_eq!(
            build_and_hold_progress(&mut app).extract_progress,
            None,
            "a drawer fetch's counter never reaches the install panel"
        );

        app.wizard_state.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        assert_eq!(
            build_and_hold_progress(&mut app).extract_progress,
            Some((11, 11))
        );
    }

    #[test]
    fn cache_check_kick_clears_stale_download_bytes_from_a_previous_run() {
        let root = ManualDlTempRoot::new("cache-clears-stale-bytes");
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-cache-clears-stale-bytes",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.wizard_state.step1.mods_archive_folder =
            root.path.join("archives").to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_update_assets =
            vec![test_asset("ModA", "https://example.com/a.zip")];
        app.wizard_state
            .step2
            .update_selected_download_bytes
            .insert(0, (50, Some(50)));
        app.wizard_state
            .step2
            .update_selected_download_done
            .insert(0);
        app.wizard_state.step2.update_selected_extract_progress = Some((11, 11));
        let inputs = LivePipelineInputs {
            destination: "C:/dest".to_string(),
            game: crate::registry::model::Game::BGEE,
            workflow: crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
            code: String::new(),
        };

        stage_and_kick_archive_skip_once(&mut app, &inputs);

        assert_eq!(
            app.wizard_state.step2.update_selected_extract_progress,
            None
        );
        assert_eq!(build_and_hold_progress(&mut app).extract_progress, None);
        assert!(
            app.wizard_state
                .step2
                .update_selected_download_bytes
                .is_empty(),
            "a re-armed run starts with no bytes from the previous run"
        );
        assert!(
            app.wizard_state
                .step2
                .update_selected_download_done
                .is_empty()
        );
        let rows = DownloadProgress::from_wizard_state(&app.wizard_state).rows;
        assert_eq!(rows[0].per_byte, None);
        app.archive_skip_rx = None;
    }

    #[test]
    fn no_requests_never_enters_hold() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-no-requests",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.wizard_state.step2.update_selected_update_assets =
            vec![test_asset("ModA", "https://example.com/a.zip")];
        let inputs = LivePipelineInputs {
            destination: "C:/dest".to_string(),
            game: crate::registry::model::Game::BGEE,
            workflow: crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
            code: String::new(),
        };

        enter_manual_hold_once(&mut app, &inputs);
        assert!(app.install_screen_state.manual_downloads.rows.is_empty());
        assert!(app.manual_download_rx.is_none());

        stage_and_kick_archive_skip_once(&mut app, &inputs);
        assert!(
            app.install_screen_state.pipeline_flags.archives_staged(),
            "the cache-check kick still fires on the same frame when there are no manual requests"
        );
    }

    #[test]
    fn all_manual_list_enters_hold_without_assets() {
        let root = ManualDlTempRoot::new("all-manual");
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-all-manual",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.wizard_state.step2.update_selected_check_running = false;
        app.wizard_state.step1.mods_archive_folder =
            root.path.join("archives").to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_manual_downloads = vec![manual_request("Ascension")];
        let inputs = LivePipelineInputs {
            destination: "C:/dest".to_string(),
            game: crate::registry::model::Game::BGEE,
            workflow: crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
            code: String::new(),
        };

        let alive = enter_manual_hold_once_with_poll(&mut app, &inputs, Duration::from_millis(20));
        assert_eq!(app.install_screen_state.manual_downloads.rows.len(), 1);
        assert!(
            app.install_screen_state
                .manual_downloads
                .manual_hold_active()
        );
        assert!(app.manual_download_rx.is_some());
        assert!(install_empty_asset_clean_finish(&mut app).is_none());
        assert!(app.install_screen_state.pipeline_arm_error.is_none());
        app.manual_download_rx = None;
        if let Some(alive) = alive {
            manual_download_watcher::wait_for_thread_exit(&alive);
        }
    }

    #[test]
    fn baseline_files_are_matched_but_never_listed_unmatched() {
        let root = ManualDlTempRoot::new("baseline");
        let archive_dir = root.path.join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();
        let preexisting = archive_dir.join("setup-ascension__manual__2.1.zip");
        std::fs::write(&preexisting, b"content").unwrap();

        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-baseline",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.wizard_state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_manual_downloads = vec![manual_request("Ascension")];
        let inputs = LivePipelineInputs {
            destination: "C:/dest".to_string(),
            game: crate::registry::model::Game::BGEE,
            workflow: crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
            code: String::new(),
        };

        let alive = enter_manual_hold_once_with_poll(&mut app, &inputs, Duration::from_millis(20));
        assert!(
            app.install_screen_state
                .manual_downloads
                .baseline
                .iter()
                .any(|(path, _, _)| path == &preexisting),
            "the archive present before the hold started is recorded as baseline"
        );
        app.manual_download_rx = None;
        if let Some(alive) = alive {
            manual_download_watcher::wait_for_thread_exit(&alive);
        }

        let probe = ArchiveProbe {
            path: preexisting,
            file_name: "notes.txt".to_string(),
            size: 7,
            hash: None,
            tp2_names: Vec::new(),
            format: manual_archive_probe::ProbeFormat::Unsupported,
        };
        handle_manual_candidate(&mut app, &probe);

        assert!(
            app.install_screen_state
                .manual_downloads
                .unmatched
                .is_empty(),
            "a pre-existing archive that matches nothing must not flood the unmatched list"
        );
    }

    #[test]
    fn file_overwritten_over_baseline_is_reported() {
        let root = ManualDlTempRoot::new("baseline-overwritten");
        let archive_dir = root.path.join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();
        let preexisting = archive_dir.join("setup-ascension__manual__2.1.zip");
        std::fs::write(&preexisting, b"content").unwrap();

        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-baseline-overwritten",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.wizard_state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_manual_downloads = vec![manual_request("Ascension")];
        let inputs = LivePipelineInputs {
            destination: "C:/dest".to_string(),
            game: crate::registry::model::Game::BGEE,
            workflow: crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall,
            code: String::new(),
        };

        let alive = enter_manual_hold_once_with_poll(&mut app, &inputs, Duration::from_millis(20));
        app.manual_download_rx = None;
        if let Some(alive) = alive {
            manual_download_watcher::wait_for_thread_exit(&alive);
        }

        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(&preexisting, b"different content, longer").unwrap();
        let metadata = std::fs::metadata(&preexisting).unwrap();

        let probe = ArchiveProbe {
            path: preexisting,
            file_name: "notes.txt".to_string(),
            size: metadata.len(),
            hash: None,
            tp2_names: Vec::new(),
            format: manual_archive_probe::ProbeFormat::Unsupported,
        };
        handle_manual_candidate(&mut app, &probe);

        assert_eq!(
            app.install_screen_state.manual_downloads.unmatched,
            vec!["notes.txt".to_string()],
            "a file saved over a pre-existing baseline name must be reported once it changes"
        );
    }

    #[test]
    fn one_archive_satisfies_both_eet_rows() {
        let root = ManualDlTempRoot::new("eet-dup");
        let dropped = root.path.join("Ascension-v2.1.zip");
        std::fs::write(&dropped, b"content").unwrap();
        let archive_dir = root.path.join("archives");

        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-eet-dup",
            );
        let mut first_tab_request = manual_request("Ascension");
        first_tab_request.game_tab = "BGEE".to_string();
        let mut second_tab_request = manual_request("Ascension");
        second_tab_request.game_tab = "BG2EE".to_string();
        app.wizard_state.step2.update_selected_manual_downloads =
            vec![first_tab_request, second_tab_request];
        app.wizard_state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        app.install_screen_state.manual_downloads.rows = vec![
            ManualDownloadRow {
                label: "Ascension".to_string(),
                from: "nexusmods.com".to_string(),
                page_url: String::new(),
                status: ManualRowStatus::Waiting,
            },
            ManualDownloadRow {
                label: "Ascension".to_string(),
                from: "nexusmods.com".to_string(),
                page_url: String::new(),
                status: ManualRowStatus::Waiting,
            },
        ];

        let probe = ArchiveProbe {
            path: dropped.clone(),
            file_name: "Ascension-v2.1.zip".to_string(),
            size: std::fs::metadata(&dropped).unwrap().len(),
            hash: None,
            tp2_names: vec!["setup-ascension.tp2".to_string()],
            format: manual_archive_probe::ProbeFormat::Zip,
        };
        handle_manual_candidate(&mut app, &probe);

        assert_eq!(
            app.install_screen_state.manual_downloads.rows[0].status,
            ManualRowStatus::Found
        );
        assert_eq!(
            app.install_screen_state.manual_downloads.rows[1].status,
            ManualRowStatus::Found
        );
        assert_eq!(
            app.wizard_state.step2.update_selected_update_assets.len(),
            2,
            "one landed archive satisfies both EET-duplicate rows"
        );
        assert!(!dropped.exists());
        assert!(
            archive_dir
                .join("setup-ascension__manual__2.1.zip")
                .exists()
        );
    }

    #[test]
    fn second_store_name_copies_instead_of_renaming() {
        let root = ManualDlTempRoot::new("second-store");
        let dropped = root.path.join("EETpack.zip");
        std::fs::write(&dropped, b"content").unwrap();
        let archive_dir = root.path.join("archives");

        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-second-store",
            );
        app.wizard_state.step2.update_selected_manual_downloads =
            vec![manual_request("eet"), manual_request("eet_end")];
        app.wizard_state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        app.install_screen_state.manual_downloads.rows = vec![
            ManualDownloadRow {
                label: "eet".to_string(),
                from: "nexusmods.com".to_string(),
                page_url: String::new(),
                status: ManualRowStatus::Waiting,
            },
            ManualDownloadRow {
                label: "eet_end".to_string(),
                from: "nexusmods.com".to_string(),
                page_url: String::new(),
                status: ManualRowStatus::Waiting,
            },
        ];

        let probe = ArchiveProbe {
            path: dropped.clone(),
            file_name: "EETpack.zip".to_string(),
            size: std::fs::metadata(&dropped).unwrap().len(),
            hash: None,
            tp2_names: vec!["setup-eet.tp2".to_string(), "setup-eet_end.tp2".to_string()],
            format: manual_archive_probe::ProbeFormat::Zip,
        };
        handle_manual_candidate(&mut app, &probe);

        assert_eq!(
            app.install_screen_state.manual_downloads.rows[0].status,
            ManualRowStatus::Found
        );
        assert_eq!(
            app.install_screen_state.manual_downloads.rows[1].status,
            ManualRowStatus::Found
        );
        let first_target = archive_dir.join("setup-eet__manual__manual.zip");
        let second_target = archive_dir.join("setup-eet_end__manual__manual.zip");
        assert!(
            first_target.exists(),
            "the first row's placed file must still exist after a second row claims a different store name"
        );
        assert!(second_target.exists());
    }

    #[test]
    fn existing_store_file_is_verified_before_acceptance() {
        let root = ManualDlTempRoot::new("verify-existing");
        let archive_dir = root.path.join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();
        let store_name = "setup-ascension__manual__manual.zip";
        let existing_target = archive_dir.join(store_name);
        std::fs::write(&existing_target, b"not a zip").unwrap();
        let landed = root.path.join("Ascension.zip");
        std::fs::write(&landed, b"also not a zip").unwrap();

        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-verify-existing",
            );
        app.wizard_state.step2.update_selected_manual_downloads = vec![manual_request("Ascension")];
        app.install_screen_state.manual_downloads.rows = vec![ManualDownloadRow {
            label: "Ascension".to_string(),
            from: "nexusmods.com".to_string(),
            page_url: "https://www.nexusmods.com/baldursgate2ee/mods/1".to_string(),
            status: ManualRowStatus::Waiting,
        }];

        let placed = apply_manual_match(
            &mut app,
            0,
            &landed,
            &archive_dir,
            ProbeMatch::Tp2 {
                store_name: store_name.to_string(),
            },
            &[],
        );

        assert!(!placed);
        match &app.install_screen_state.manual_downloads.rows[0].status {
            ManualRowStatus::Refused(reason) => {
                assert!(
                    reason.contains("a different file already sits at"),
                    "got {reason:?}"
                );
            }
            other => panic!("expected Refused, got {other:?}"),
        }
        assert!(landed.exists(), "the landed file must be left untouched");
        assert_eq!(
            std::fs::read(&existing_target).unwrap(),
            b"not a zip",
            "the existing store file must be left untouched"
        );
    }

    #[test]
    fn blank_archive_folder_refuses_pick() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-blank-folder",
            );
        app.wizard_state.step1.mods_archive_folder = "   ".to_string();
        app.install_screen_state.manual_downloads.rows = vec![ManualDownloadRow {
            label: "Ascension".to_string(),
            from: "nexusmods.com".to_string(),
            page_url: "https://www.nexusmods.com/baldursgate2ee/mods/1".to_string(),
            status: ManualRowStatus::Waiting,
        }];

        pick_manual_file(&mut app, 0);

        match &app.install_screen_state.manual_downloads.rows[0].status {
            ManualRowStatus::Refused(reason) => {
                assert!(
                    reason.contains("set your Mods archive folder in Settings"),
                    "got {reason:?}"
                );
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn found_row_appends_empty_url_asset_and_clears_bucket() {
        let root = ManualDlTempRoot::new("found");
        let dropped = root.path.join("Ascension-v2.1.zip");
        std::fs::write(&dropped, b"content").unwrap();
        let archive_dir = root.path.join("archives");

        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-found",
            );
        let request = manual_request("Ascension");
        app.wizard_state.step2.update_selected_manual_downloads = vec![request];
        app.wizard_state.step2.update_selected_manual_sources = vec!["Ascension".to_string()];
        app.install_screen_state.manual_downloads.rows = vec![ManualDownloadRow {
            label: "Ascension".to_string(),
            from: "nexusmods.com".to_string(),
            page_url: "https://www.nexusmods.com/baldursgate2ee/mods/1".to_string(),
            status: ManualRowStatus::Waiting,
        }];

        apply_manual_match(
            &mut app,
            0,
            &dropped,
            &archive_dir,
            ProbeMatch::Tp2 {
                store_name: "setup-ascension__manual__2.1.zip".to_string(),
            },
            &[],
        );

        assert_eq!(
            app.install_screen_state.manual_downloads.rows[0].status,
            ManualRowStatus::Found
        );
        assert_eq!(
            app.wizard_state.step2.update_selected_update_assets.len(),
            1
        );
        let asset = &app.wizard_state.step2.update_selected_update_assets[0];
        assert_eq!(asset.asset_url, "");
        assert_eq!(asset.asset_name, "setup-ascension__manual__2.1.zip");
        assert_eq!(asset.source_id, "manual");
        assert_eq!(asset.tag, "2.1");
        assert!(
            !app.wizard_state
                .step2
                .update_selected_manual_sources
                .contains(&"Ascension".to_string())
        );
        assert!(
            app.install_screen_state
                .manual_downloads
                .handled
                .contains(&archive_dir.join("setup-ascension__manual__2.1.zip"))
        );
    }

    #[test]
    fn continue_without_records_the_lists_own_label() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-skip-label",
            );
        let mut request = manual_request("WL_HOUSERULES");
        request.display_name = "House Rules".to_string();
        app.install_screen_state.manual_downloads.rows = vec![manual_row_from_request(&request)];
        app.wizard_state.step2.update_selected_manual_downloads = vec![request];

        confirm_continue_without(&mut app);

        assert_eq!(
            app.install_screen_state.manual_downloads.rows[0].label,
            "House Rules"
        );
        assert_eq!(
            app.wizard_state.step2.skipped_manual_downloads,
            vec!["WL_HOUSERULES".to_string()],
            "the workspace toast names the mod the way the list does"
        );
    }

    #[test]
    fn found_row_records_download_once() {
        let root = ManualDlTempRoot::new("found-once");
        let dropped = root.path.join("Ascension-v2.1.zip");
        std::fs::write(&dropped, b"content").unwrap();
        let archive_dir = root.path.join("archives");

        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-found-once",
            );
        let request = manual_request("Ascension");
        app.wizard_state.step2.update_selected_manual_downloads = vec![request];
        app.wizard_state.step2.update_selected_manual_sources = vec!["Ascension".to_string()];
        app.install_screen_state.manual_downloads.rows = vec![ManualDownloadRow {
            label: "Ascension".to_string(),
            from: "nexusmods.com".to_string(),
            page_url: "https://www.nexusmods.com/baldursgate2ee/mods/1".to_string(),
            status: ManualRowStatus::Waiting,
        }];

        apply_manual_match(
            &mut app,
            0,
            &dropped,
            &archive_dir,
            ProbeMatch::Tp2 {
                store_name: "setup-ascension__manual__2.1.zip".to_string(),
            },
            &[],
        );

        assert_eq!(
            app.wizard_state
                .step2
                .update_selected_downloaded_sources
                .len(),
            1,
            "the found archive is recorded as downloaded exactly once"
        );
        assert!(
            app.wizard_state.step2.update_selected_downloaded_sources[0]
                .starts_with("Ascension -> ")
        );

        let mut skip_indices = std::collections::HashSet::new();
        mark_empty_url_assets_as_cache_hits(&mut app, &mut skip_indices);

        assert_eq!(
            app.wizard_state
                .step2
                .update_selected_downloaded_sources
                .len(),
            1,
            "the empty-URL pass does not record the same label twice"
        );
        assert!(
            skip_indices.contains(&0),
            "the found asset's index is skipped"
        );
    }

    #[test]
    fn empty_url_assets_join_skip_indices() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-skip-indices",
            );
        app.wizard_state.step2.update_selected_update_assets = vec![
            test_asset("ModA", "https://example.com/a.zip"),
            test_asset("ModB", ""),
        ];
        let mut skip_indices = std::collections::HashSet::new();

        mark_empty_url_assets_as_cache_hits(&mut app, &mut skip_indices);

        assert!(!skip_indices.contains(&0));
        assert!(skip_indices.contains(&1));
        assert_eq!(
            app.wizard_state
                .step2
                .update_selected_downloaded_sources
                .len(),
            1
        );
        assert!(
            app.wizard_state.step2.update_selected_downloaded_sources[0].starts_with("ModB -> ")
        );
    }

    #[test]
    fn continue_without_bypasses_blockers() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-continue-without",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.install_screen_state
            .pipeline_flags
            .set_archives_ingested(true);
        app.wizard_state.modlist_auto_build_active = true;
        app.wizard_state.step2.update_selected_manual_sources = vec!["Ascension".to_string()];
        app.install_screen_state.manual_downloads.continue_without = true;

        gate_pre_step5_blocker_once(&mut app);

        assert!(app.install_screen_state.pipeline_arm_error.is_none());
        assert!(app.install_screen_state.pre_step5_blocker_checked);
        assert!(app.wizard_state.modlist_auto_build_active);
    }

    #[test]
    fn open_workspace_outcome_when_continue_without_and_no_assets() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-open-workspace",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.wizard_state.modlist_auto_build_active = true;
        app.install_screen_state.manual_downloads.continue_without = true;

        let outcome = install_empty_asset_clean_finish(&mut app);

        assert_eq!(outcome, Some(DownloadingOutcome::OpenWorkspace));
    }

    #[test]
    fn open_workspace_outcome_waits_out_the_post_extract_scan() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-open-workspace-scanning",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.wizard_state.modlist_auto_build_active = true;
        app.install_screen_state.manual_downloads.continue_without = true;
        app.wizard_state.step2.is_scanning = true;

        let outcome = install_empty_asset_clean_finish(&mut app);

        assert_eq!(
            outcome, None,
            "the workspace must not open while the post-extract scan is still running"
        );
    }

    #[test]
    fn ingest_waits_for_manual_hold() {
        let root = ManualDlTempRoot::new("ingest-waits");
        let destination = root.path.join("dest").to_string_lossy().into_owned();
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-ingest-waits",
            );
        app.install_screen_state
            .pipeline_flags
            .set_download_phase_started(true);
        app.wizard_state.step2.update_selected_downloaded_sources =
            vec!["Ascension -> C:/archives/Ascension.zip".to_string()];
        app.install_screen_state.manual_downloads.rows = vec![ManualDownloadRow {
            label: "Ascension".to_string(),
            from: "nexusmods.com".to_string(),
            page_url: String::new(),
            status: ManualRowStatus::Waiting,
        }];

        ingest_downloaded_archives_once(&mut app, &destination);
        assert!(
            !app.install_screen_state.pipeline_flags.archives_ingested(),
            "the store-and-lock step waits for the manual hold"
        );

        app.install_screen_state.manual_downloads.rows[0].status = ManualRowStatus::Found;
        ingest_downloaded_archives_once(&mut app, &destination);
        assert!(
            app.install_screen_state.pipeline_flags.archives_ingested(),
            "ingest proceeds once every manual row is found"
        );
    }

    #[test]
    fn continue_without_waits_for_deferred_extract() {
        let mut app =
            crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
                "manualdl-continue-without-deferred",
            );
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.wizard_state.modlist_auto_build_active = true;
        app.install_screen_state.manual_downloads.continue_without = true;
        app.install_screen_state.manual_downloads.extract_deferred = true;

        let outcome = install_empty_asset_clean_finish(&mut app);

        assert_eq!(
            outcome, None,
            "the workspace must not open while an extract is still deferred"
        );
    }

    #[test]
    fn manual_row_prefers_display_name_and_strips_www() {
        let mut request = manual_request("Ascension");
        request.display_name = "House Rules".to_string();
        request.page_url = "https://www.nexusmods.com/baldursgate2ee/mods/1".to_string();
        let row = manual_row_from_request(&request);
        assert_eq!(row.label, "House Rules");
        assert_eq!(row.from, "nexusmods.com");

        let mut fallback = manual_request("Ascension");
        fallback.display_name = String::new();
        let fallback_row = manual_row_from_request(&fallback);
        assert_eq!(fallback_row.label, "Ascension");
    }
}
