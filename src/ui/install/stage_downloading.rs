// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::install_runtime::archive_store;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::{
    DestinationPrepFlow, PendingInstallDestinationPrep,
};
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_input_bg, redesign_pill_danger, redesign_shell_bg,
    redesign_success, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

const CHECK_STAGED: &str = "\u{2713}"; // ✓

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
    pub asset_bytes: std::collections::BTreeMap<usize, (u64, Option<u64>)>,
    pub extract_progress: Option<(usize, usize)>,
    pub hash_progress: Option<(usize, usize)>,
}

impl DownloadProgress {
    #[must_use]
    pub fn from_wizard_state_full(
        state: &WizardState,
        prior_bytes: &std::collections::BTreeMap<usize, (u64, Option<u64>)>,
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
                        ModDownloadStatus::Downloading
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
                    per_byte: prior_bytes.get(&i).copied(),
                    expected_size,
                }
            })
            .collect();

        rows.sort_by_key(|r| status_sort_key(r.status));

        Self {
            rows,
            skipped: Vec::new(),
            expected_sizes: prior_expected.clone(),
            asset_bytes: prior_bytes.clone(),
            extract_progress: None,
            hash_progress: None,
        }
    }

    #[must_use]
    pub fn from_wizard_state_with_bytes(
        state: &WizardState,
        prior_bytes: &std::collections::BTreeMap<usize, (u64, Option<u64>)>,
    ) -> Self {
        Self::from_wizard_state_full(
            state,
            prior_bytes,
            &[],
            &std::collections::BTreeMap::new(),
            None,
        )
    }

    #[must_use]
    pub fn from_wizard_state(state: &WizardState) -> Self {
        Self::from_wizard_state_full(
            state,
            &std::collections::BTreeMap::new(),
            &[],
            &std::collections::BTreeMap::new(),
            None,
        )
    }

    pub fn set_asset_bytes(&mut self, index: usize, bytes: u64, total: Option<u64>) {
        self.asset_bytes.insert(index, (bytes, total));
        if let Some(row) = self.rows.get_mut(index) {
            row.per_byte = Some((bytes, total));
        }
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
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy: DownloadScreenCopy,
    progress: &DownloadProgress,
) -> DownloadingOutcome {
    let back_clicked = render_chrome(ui, palette, copy, progress, None);
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
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    copy: DownloadScreenCopy,
) -> DownloadingOutcome {
    use crate::install_runtime::auto_build_driver;

    let palette = orchestrator.theme_palette;
    let inputs = LivePipelineInputs::from(orchestrator);

    arm_pipeline_once(orchestrator, &inputs);
    stage_and_kick_archive_skip_once(orchestrator, &inputs);
    kick_streaming_downloader_once(orchestrator);
    verify_downloaded_archives_once(orchestrator, &inputs.destination);
    ingest_downloaded_archives_once(orchestrator, &inputs.destination);

    let progress = build_and_hold_progress(orchestrator);
    let arm_error = orchestrator.install_screen_state.pipeline_arm_error.clone();
    let back_clicked = render_chrome(ui, palette, copy, &progress, arm_error.as_deref());

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
        let workflow = if orchestrator.install_screen_state.is_partial() {
            crate::install_runtime::flag_policies::InstallWorkflow::ContinuePartialInstall
        } else {
            crate::install_runtime::flag_policies::InstallWorkflow::PasteAndInstall
        };
        Self::from_workflow(orchestrator, workflow)
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

    let flow = install_destination_prep_flow(orchestrator);

    if orchestrator.install_screen_state.pipeline_flags.armed()
        || orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_some()
    {
        return;
    }

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

const fn install_destination_prep_flow(
    orchestrator: &crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
) -> DestinationPrepFlow {
    if matches!(orchestrator.nav, NavDestination::Create) {
        DestinationPrepFlow::CreateForkDownload
    } else {
        DestinationPrepFlow::InstallPipeline
    }
}

fn finish_pipeline_arm_after_destination_prep(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    inputs: &LivePipelineInputs,
) {
    use crate::install_runtime::auto_build_driver;

    orchestrator
        .install_screen_state
        .pipeline_flags
        .set_armed(true);

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
            crate::install_runtime::install_modlist_registration::register_and_write_install_start_artifacts(
                orchestrator,
            );
        }
        Err(err) => {
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

pub(crate) fn stage_and_kick_archive_skip_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    inputs: &LivePipelineInputs,
) {
    if !orchestrator
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
    {
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

pub(crate) fn kick_streaming_downloader_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
) {
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
        let skip_indices = orchestrator.install_screen_state.skip_indices.clone();
        if let Some(rx) = crate::install_runtime::stream_downloader::start_stream_download(
            &mut orchestrator.wizard_state,
            &skip_indices,
        ) {
            orchestrator.stream_download_rx = Some(rx);
            tracing::info!(
                target = "orchestrator",
                "parallel streaming downloader spawned for {} asset(s); \
                 bypasses {} skipped index/indices",
                orchestrator
                    .wizard_state
                    .step2
                    .update_selected_update_assets
                    .len(),
                skip_indices.len()
            );
        }
    }
}

pub(crate) fn verify_downloaded_archives_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
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
        && downloaded_sources > 0;
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
    let prior_bytes = orchestrator
        .install_screen_state
        .download_progress
        .asset_bytes
        .clone();
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
        &prior_bytes,
        &prior_skipped,
        &prior_expected,
        hashed,
    );
    progress.extract_progress = orchestrator.extract_progress.lock().ok().and_then(|g| *g);
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

pub(crate) fn render_chrome(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy: DownloadScreenCopy,
    progress: &DownloadProgress,
    arm_error: Option<&str>,
) -> bool {
    render_screen_title(ui, palette, copy.title, Some(copy.sub));
    ui.add_space(12.0);

    if let Some(err) = arm_error {
        render_arm_error_banner(ui, palette, err);
        ui.add_space(14.0);
    }

    render_overall_progress(ui, palette, copy.hint, progress);
    ui.add_space(14.0);

    let footer_h = sub_flow_footer::FOOTER_HEIGHT_PX;
    let grid_budget = (ui.available_height() - footer_h - 8.0).max(140.0);
    render_mod_progress(ui, palette, progress, grid_budget);

    let spacer = (ui.available_height() - footer_h).max(0.0);
    if spacer > 0.0 {
        ui.add_space(spacer);
    }

    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Cancel" }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        None,
        PrimaryBtn {
            label: "Waiting\u{2026}",
            disabled: true,
        },
    );
    footer.back_clicked
}

fn render_overall_progress(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    hint: Option<&str>,
    progress: &DownloadProgress,
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
        let phase_line = if preparing {
            "Preparing to install \u{2026}".to_string()
        } else {
            let (verb, n, t, p) = match phase {
                InstallPhase::Hashing => (InstallPhase::Hashing.verb(), h_n, h_total, h_pct),
                InstallPhase::Downloading => {
                    (InstallPhase::Downloading.verb(), dl_n, dl_total, dl_pct)
                }
                InstallPhase::Extracting => {
                    (InstallPhase::Extracting.verb(), ex_n, ex_total, ex_pct)
                }
            };
            format!("{verb} \u{2026} {n} / {t} mods \u{00B7} {p}%")
        };
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
            "hash",
            h_n,
            h_total,
            h_pct,
            f64::from(h_pct) / 100.0,
            phase == InstallPhase::Hashing,
        );
        ui.add_space(8.0);
        phase_bar_row(
            ui,
            palette,
            "download",
            dl_n,
            dl_total,
            dl_pct,
            f64::from(dl_pct) / 100.0,
            phase == InstallPhase::Downloading,
        );
        ui.add_space(8.0);
        phase_bar_row(
            ui,
            palette,
            "extract",
            ex_n,
            ex_total,
            ex_pct,
            f64::from(ex_pct) / 100.0,
            phase == InstallPhase::Extracting && !preparing,
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

#[allow(clippy::too_many_arguments)]
fn phase_bar_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    verb: &str,
    n: usize,
    total: usize,
    pct: u32,
    frac: f64,
    active: bool,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let (label_rect, _) = ui.allocate_exact_size(egui::vec2(180.0, 18.0), egui::Sense::hover());
        let cap_color = if active {
            redesign_text_primary(palette)
        } else {
            redesign_text_faint(palette)
        };
        ui.painter().text(
            egui::pos2(label_rect.left(), label_rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{verb} {n} / {total} \u{00B7} {pct}%"),
            egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
            cap_color,
        );

        let bar_w = ui.available_width();
        let (track, _) = ui.allocate_exact_size(egui::vec2(bar_w, 14.0), egui::Sense::hover());
        paint_phase_bar(ui, palette, track, frac, active);
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

fn check_prose_cell(ui: &mut egui::Ui, w: f32, prose: &str, color: egui::Color32) {
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

fn sized_label(
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
                ), // indeterminate (no size) ⇒ any-lacks-known-size ⇒ pure count
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
            extract_progress: Some((7, 10)), // a stale value
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
    fn set_asset_bytes_persists_and_survives_per_frame_rebuild() {
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

        let mut p = DownloadProgress::from_wizard_state(&st);
        p.set_asset_bytes(0, 512, Some(2048));
        assert_eq!(p.asset_bytes.get(&0), Some(&(512, Some(2048))));
        assert_eq!(p.rows[0].per_byte, Some((512, Some(2048))));

        let mut expected = BTreeMap::new();
        expected.insert(0usize, 2048u64);
        let p2 = DownloadProgress::from_wizard_state_full(
            &st,
            &p.asset_bytes,
            &[skipped("CACHED", Some(4096))],
            &expected,
            None,
        );
        assert_eq!(
            p2.rows[0].per_byte,
            Some((512, Some(2048))),
            "the byte map survives the per-frame row rebuild"
        );
        assert_eq!(
            p2.rows[0].expected_size,
            Some(2048),
            "the share-code expected size is merged onto the row"
        );
        assert_eq!(p2.rows[1].per_byte, None, "asset 1 had no byte delta yet");
        assert!(
            p2.skipped.is_empty(),
            ": `skipped` is vestigial; not populated"
        );
        assert!((p2.rows[0].bar_fraction() - 0.25).abs() < 0.001);
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

        let sk = vec![skipped("ALREADY_HERE", Some(7777))];
        let p = DownloadProgress::from_wizard_state_full(
            &st,
            &BTreeMap::new(),
            &sk,
            &BTreeMap::new(),
            None,
        );
        assert_eq!(p.rows.len(), 4);
        let statuses: Vec<_> = p.rows.iter().map(|r| r.status).collect();
        assert_eq!(statuses[0], ModDownloadStatus::Downloading);
        assert_eq!(statuses[1], ModDownloadStatus::Downloading);
        assert!(statuses[2..].iter().all(|s| s.download_complete()));
        assert_eq!(
            p.skipped.len(),
            0,
            "v3: skipped is not populated by from_wizard_state_full"
        );
        assert_eq!(p.total(), 4, "4 rows (no phantom skipped row)");
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
        let p = DownloadProgress::from_wizard_state_full(
            &st,
            &BTreeMap::new(),
            &[],
            &BTreeMap::new(),
            Some(&hashed),
        );
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
        let p = DownloadProgress::from_wizard_state_full(
            &st,
            &BTreeMap::new(),
            &[],
            &BTreeMap::new(),
            None,
        );
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
}
