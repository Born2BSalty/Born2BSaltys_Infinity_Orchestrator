// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::install_runtime::archive_store;
use crate::install_runtime::flag_policies::InstallWorkflow;
use crate::registry::Game;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_input_bg, redesign_pill_danger, redesign_shell_bg,
    redesign_success, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

const CHECK_STAGED: &str = "\u{2713}";

fn f64_from_u64(value: u64) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(f64::MAX)
}

fn f64_from_usize(value: usize) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(f64::MAX)
}

fn f32_from_f64(value: f64) -> f32 {
    value.to_string().parse::<f32>().unwrap_or(0.0)
}

fn unit_f32(value: f64) -> f32 {
    f32_from_f64(value.clamp(0.0, 1.0))
}

fn ratio_u64(numerator: u64, denominator: u64) -> f32 {
    unit_f32(f64_from_u64(numerator) / f64_from_u64(denominator.max(1)))
}

fn ratio_usize(numerator: usize, denominator: usize) -> f32 {
    unit_f32(f64_from_usize(numerator) / f64_from_usize(denominator.max(1)))
}

fn pct_from_fraction(value: f32) -> u32 {
    format!("{:.0}", value.clamp(0.0, 1.0) * 100.0)
        .parse::<u32>()
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModDownloadStatus {
    #[default]
    Queued,
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
            Self::Downloading => "downloading".to_string(),
            Self::Extracting => "extracting...".to_string(),
            Self::Staged => "staged".to_string(),
            Self::Skipped => "already downloaded".to_string(),
        }
    }

    #[must_use]
    pub const fn phase_fraction(self) -> f32 {
        match self {
            Self::Queued => 0.0,
            Self::Downloading => 0.15,
            Self::Extracting => 0.65,
            Self::Staged | Self::Skipped => 1.0,
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
    pub const fn is_skipped(self) -> bool {
        matches!(self, Self::Skipped)
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
            ModDownloadStatus::Downloading | ModDownloadStatus::Queued => {
                let size = known_size?;
                let got = self.per_byte.map_or(0, |(b, _)| b).min(size);
                Some((got, size))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallPhase {
    #[default]
    Downloading,
    Extracting,
}

impl InstallPhase {
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
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
}

impl DownloadProgress {
    #[must_use]
    pub fn from_wizard_state_full(
        state: &WizardState,
        prior_bytes: &std::collections::BTreeMap<usize, (u64, Option<u64>)>,
        prior_skipped: &[SkippedMod],
        prior_expected: &std::collections::BTreeMap<usize, u64>,
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
        let skipped_labels: std::collections::HashSet<&str> =
            prior_skipped.iter().map(|s| s.name.as_str()).collect();

        let rows = s2
            .update_selected_update_assets
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                if skipped_labels.contains(a.label.as_str()) {
                    return None;
                }
                let downloaded = label_done(&s2.update_selected_downloaded_sources, &a.label);
                let extracted = label_done(&s2.update_selected_extracted_sources, &a.label);
                let status = if extracted {
                    ModDownloadStatus::Staged
                } else if downloaded {
                    ModDownloadStatus::Extracting
                } else if s2.update_selected_download_running {
                    ModDownloadStatus::Downloading
                } else {
                    ModDownloadStatus::Queued
                };
                Some(ModDownloadRow {
                    name: a.label.clone(),
                    source: a.source_id.clone(),
                    status,
                    per_byte: prior_bytes.get(&i).copied(),
                    expected_size: prior_expected.get(&i).copied(),
                })
            })
            .collect();

        Self {
            rows,
            skipped: prior_skipped.to_vec(),
            expected_sizes: prior_expected.clone(),
            asset_bytes: prior_bytes.clone(),
            extract_progress: None,
        }
    }

    #[must_use]
    pub fn from_wizard_state_with_bytes(
        state: &WizardState,
        prior_bytes: &std::collections::BTreeMap<usize, (u64, Option<u64>)>,
    ) -> Self {
        Self::from_wizard_state_full(state, prior_bytes, &[], &std::collections::BTreeMap::new())
    }

    #[must_use]
    pub fn from_wizard_state(state: &WizardState) -> Self {
        Self::from_wizard_state_full(
            state,
            &std::collections::BTreeMap::new(),
            &[],
            &std::collections::BTreeMap::new(),
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
            return ratio_usize(completed, total);
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
    orchestrator: &mut OrchestratorApp,
    copy: DownloadScreenCopy,
) -> DownloadingOutcome {
    use crate::install_runtime::auto_build_driver;

    let palette = orchestrator.theme_palette;

    let destination = install_destination(orchestrator);
    let game = install_game(orchestrator);
    let workflow = install_workflow(orchestrator);
    let code = install_code(orchestrator);

    arm_pipeline_if_needed(orchestrator, &destination, game, workflow, &code);

    stage_archives_if_ready(orchestrator, &destination, &code);

    start_download_if_ready(orchestrator);

    verify_archives_if_ready(orchestrator, &destination);

    ingest_archives_if_ready(orchestrator, &destination);

    let progress = build_live_progress(orchestrator);

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

fn install_destination(orchestrator: &OrchestratorApp) -> String {
    orchestrator
        .install_screen_state
        .destination
        .trim()
        .to_string()
}

fn install_game(orchestrator: &OrchestratorApp) -> Game {
    orchestrator
        .install_screen_state
        .parsed_preview
        .as_ref()
        .map(|p| Game::from_legacy_string(&p.game_install))
        .unwrap_or_default()
}

fn install_workflow(orchestrator: &OrchestratorApp) -> InstallWorkflow {
    if orchestrator.install_screen_state.is_partial() {
        InstallWorkflow::ContinuePartialInstall
    } else {
        InstallWorkflow::ShareCodeConsuming
    }
}

fn install_code(orchestrator: &OrchestratorApp) -> String {
    orchestrator
        .install_screen_state
        .import_code
        .trim()
        .to_string()
}

fn arm_pipeline_if_needed(
    orchestrator: &mut OrchestratorApp,
    destination: &str,
    game: Game,
    workflow: InstallWorkflow,
    code: &str,
) {
    use crate::install_runtime::auto_build_driver;

    if orchestrator.install_screen_state.pipeline_flags.armed() {
        return;
    }
    orchestrator
        .install_screen_state
        .pipeline_flags
        .set_armed(true);

    match auto_build_driver::prepare_install_dirs_and_maybe_import(
        &mut orchestrator.wizard_state,
        destination,
        game,
        workflow,
        code,
    ) {
        Ok(_) => {
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

            crate::install_runtime::install_modlist_registration
                ::register_and_write_install_start_artifacts(orchestrator);
        }
        Err(err) => {
            orchestrator.install_screen_state.pipeline_arm_error = Some(err.clone());
            orchestrator.wizard_state.step2.scan_status =
                format!("Auto Build could not start: {err}");
            tracing::warn!(
                target = "orchestrator",
                "P7.T17 pipeline arm failed: {err} (Downloading stays navigable; \
                 surfaced on-screen)"
            );
        }
    }
}

fn stage_archives_if_ready(orchestrator: &mut OrchestratorApp, destination: &str, code: &str) {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    if flags.archives_staged()
        || !flags.armed()
        || destination.is_empty()
        || orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .is_empty()
    {
        return;
    }

    orchestrator
        .install_screen_state
        .pipeline_flags
        .set_archives_staged(true);
    archive_store::stage_known_archives(&mut orchestrator.wizard_state, destination);

    let expected = crate::registry::share_export::decode_archive_meta(code).unwrap_or_default();
    orchestrator.install_screen_state.pre_skip_assets = orchestrator
        .wizard_state
        .step2
        .update_selected_update_assets
        .clone();
    let skip = crate::install_runtime::archive_skip::skip_present_archives(
        &mut orchestrator.wizard_state,
        &expected,
    );

    let by_name: std::collections::HashMap<&str, &crate::registry::share_export::ArchiveMeta> =
        expected.iter().map(|m| (m.name.as_str(), m)).collect();
    let skipped_mods: Vec<SkippedMod> = skip
        .skipped_assets
        .iter()
        .map(|a| {
            let name = crate::app::app_step2_update_download::archive_file_name(a);
            SkippedMod {
                name: a.label.clone(),
                source: a.source_id.clone(),
                size: by_name.get(name.as_str()).map(|m| m.size),
            }
        })
        .collect();
    let skip_indices = collect_skip_indices(orchestrator, &skip.skipped_assets);
    let expected_sizes = expected_archive_sizes(orchestrator, &by_name);
    orchestrator.install_screen_state.skipped_mods = skipped_mods;
    orchestrator.install_screen_state.expected_archive_sizes = expected_sizes;
    orchestrator.install_screen_state.skip_indices = skip_indices;

    orchestrator.install_screen_state.expected_archive_meta = expected;
    tracing::info!(
        target = "orchestrator",
        "checksum-then-skip: {} already-present (kept in list for \
         extract, streamer bypasses by index), {} missing (will fetch), \
         {} no-expected-hash, {} candidates hashed ({} persistent-cache \
         hits); DL-Run-2 captured {} skipped-mod rows + {} expected-size \
         denominators + {} skip indices",
        skip.skipped_present,
        skip.missing_on_disk,
        skip.no_expected_hash,
        skip.hashed_candidates,
        skip.cache_hits,
        orchestrator.install_screen_state.skipped_mods.len(),
        orchestrator
            .install_screen_state
            .expected_archive_sizes
            .len(),
        orchestrator.install_screen_state.skip_indices.len(),
    );
}

fn collect_skip_indices(
    orchestrator: &OrchestratorApp,
    skipped_assets: &[crate::app::state::Step2UpdateAsset],
) -> std::collections::HashSet<usize> {
    let mut skip_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut consumed: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for skipped_asset in skipped_assets {
        let name = crate::app::app_step2_update_download::archive_file_name(skipped_asset);
        for (i, asset) in orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .iter()
            .enumerate()
        {
            if consumed.contains(&i) {
                continue;
            }
            if asset.label == skipped_asset.label
                && crate::app::app_step2_update_download::archive_file_name(asset) == name
            {
                skip_indices.insert(i);
                consumed.insert(i);
                break;
            }
        }
    }
    skip_indices
}

fn expected_archive_sizes(
    orchestrator: &OrchestratorApp,
    by_name: &std::collections::HashMap<&str, &crate::registry::share_export::ArchiveMeta>,
) -> std::collections::BTreeMap<usize, u64> {
    orchestrator
        .wizard_state
        .step2
        .update_selected_update_assets
        .iter()
        .enumerate()
        .filter_map(|(i, asset)| {
            let name = crate::app::app_step2_update_download::archive_file_name(asset);
            by_name.get(name.as_str()).map(|meta| (i, meta.size))
        })
        .collect()
}

fn start_download_if_ready(orchestrator: &mut OrchestratorApp) {
    use crate::install_runtime::auto_build_driver;

    let flags = orchestrator.install_screen_state.pipeline_flags;
    if !flags.armed()
        || orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_some()
        || !auto_build_driver::download_gate_open(&orchestrator.wizard_state)
        || flags.download_phase_started()
    {
        return;
    }

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
            "Fix 1e — parallel streaming downloader spawned for {} \
             asset(s); streamer bypasses {} skipped index/indices",
            orchestrator
                .wizard_state
                .step2
                .update_selected_update_assets
                .len(),
            skip_indices.len()
        );
    }
}

fn verify_archives_if_ready(orchestrator: &mut OrchestratorApp, destination: &str) {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    if flags.archives_verified()
        || destination.is_empty()
        || orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        || !flags.download_phase_started()
        || orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty()
    {
        return;
    }

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
        .filter_map(|(i, asset)| {
            if skip_indices.contains(&i) {
                None
            } else {
                Some(asset.clone())
            }
        })
        .collect();
    let result = crate::install_runtime::archive_skip::verify_downloaded_archives(
        &mut orchestrator.wizard_state,
        &expected,
        &pre_skip,
    );
    tracing::info!(
        target = "orchestrator",
        "post-download verify: {} verified, {} hash-mismatched \
         (deleted + recorded failed, NOT installed), {} unverifiable",
        result.verified,
        result.mismatched,
        result.unverifiable
    );
}

fn ingest_archives_if_ready(orchestrator: &mut OrchestratorApp, destination: &str) {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    if flags.archives_ingested()
        || destination.is_empty()
        || orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        || !flags.download_phase_started()
        || orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty()
    {
        return;
    }

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
}

fn build_live_progress(orchestrator: &mut OrchestratorApp) -> DownloadProgress {
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
    let mut progress = DownloadProgress::from_wizard_state_full(
        &orchestrator.wizard_state,
        &prior_bytes,
        &prior_skipped,
        &prior_expected,
    );
    progress.extract_progress = orchestrator.extract_progress.lock().ok().and_then(|g| *g);

    if progress.rows.is_empty()
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
            .is_empty()
    {
        orchestrator.install_screen_state.download_progress.clone()
    } else {
        orchestrator.install_screen_state.download_progress = progress.clone();
        progress
    }
}

fn render_chrome(
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
        let dl_total = progress.total();
        let dl_n = progress.downloaded_count();
        let dl_pct = progress.download_overall_pct();
        let (ex_n, ex_total) = progress.extract_completed_total();
        let ex_pct = progress.extract_overall_pct();

        let (verb, n, t, p) = match phase {
            InstallPhase::Downloading => (InstallPhase::Downloading.verb(), dl_n, dl_total, dl_pct),
            InstallPhase::Extracting => (InstallPhase::Extracting.verb(), ex_n, ex_total, ex_pct),
        };
        ui.label(
            egui::RichText::new(format!("{verb} \u{2026} {n} / {t} mods \u{00B7} {p}%"))
                .size(15.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(8.0);

        phase_bar_row(
            ui,
            palette,
            PhaseBarRow {
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
            PhaseBarRow {
                verb: "extract",
                n: ex_n,
                total: ex_total,
                pct: ex_pct,
                frac: f64::from(ex_pct) / 100.0,
                active: phase == InstallPhase::Extracting,
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
struct PhaseBarRow<'a> {
    verb: &'a str,
    n: usize,
    total: usize,
    pct: u32,
    frac: f64,
    active: bool,
}

fn phase_bar_row(ui: &mut egui::Ui, palette: ThemePalette, row: PhaseBarRow<'_>) {
    let PhaseBarRow {
        verb,
        n,
        total,
        pct,
        frac,
        active,
    } = row;
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
            format!("{verb}  {n} / {total} \u{00B7} {pct}%"),
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

        if progress.rows.is_empty() && progress.skipped.is_empty() {
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
                        for s in &progress.skipped {
                            render_skipped_row(ui, palette, s, mod_w, src_w, status_w, prog_w);
                            ui.end_row();
                        }
                        for row in &progress.rows {
                            render_grid_row(ui, palette, row, mod_w, src_w, status_w, prog_w);
                            ui.end_row();
                        }
                    });
            });
    });
}

fn render_skipped_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    s: &SkippedMod,
    mod_w: f32,
    src_w: f32,
    status_w: f32,
    prog_w: f32,
) {
    sized_label(
        ui,
        mod_w,
        &s.name,
        14.0,
        "poppins_medium",
        redesign_text_primary(palette),
    );
    sized_label(
        ui,
        src_w,
        &s.source,
        13.0,
        "poppins_light",
        redesign_text_faint(palette),
    );
    check_prose_cell(
        ui,
        status_w,
        "already downloaded",
        redesign_success(palette),
    );
    let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(prog_w, 14.0), egui::Sense::hover());
    paint_bar(ui, palette, bar_rect, 1.0, true);
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

    let status_color = if row.status.is_done() {
        redesign_success(palette)
    } else if row.status.is_queued() {
        redesign_text_faint(palette)
    } else {
        redesign_text_primary(palette)
    };
    if row.status.is_done() {
        let prose = if row.status.is_skipped() {
            "already downloaded"
        } else {
            "staged"
        };
        check_prose_cell(ui, status_w, prose, status_color);
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
    let phase = (t / 1.6).fract();
    let tri = if phase < 0.5 {
        phase * 2.0
    } else {
        2.0 - phase * 2.0
    };
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

    fn assert_f32_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn status_text_has_no_fabricated_pct_and_skipped_reads_already_downloaded() {
        assert_eq!(ModDownloadStatus::Queued.status_text(), "queued");
        assert_eq!(ModDownloadStatus::Downloading.status_text(), "downloading");
        assert_eq!(ModDownloadStatus::Extracting.status_text(), "extracting...");
        assert_eq!(ModDownloadStatus::Staged.status_text(), "staged");
        assert_eq!(
            ModDownloadStatus::Skipped.status_text(),
            "already downloaded"
        );
        for s in [
            ModDownloadStatus::Queued,
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
    fn phase_fraction_is_strictly_monotonic_with_satisfied_terminals_full() {
        let queued = ModDownloadStatus::Queued.phase_fraction();
        let downloading = ModDownloadStatus::Downloading.phase_fraction();
        let extracting = ModDownloadStatus::Extracting.phase_fraction();
        let staged = ModDownloadStatus::Staged.phase_fraction();
        let skipped = ModDownloadStatus::Skipped.phase_fraction();
        assert!(
            queued < downloading && downloading < extracting && extracting < staged,
            "strictly increasing queued<downloading<extracting<staged"
        );
        assert!((queued - 0.0).abs() < f32::EPSILON);
        assert!((staged - 1.0).abs() < f32::EPSILON);
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
        assert_f32_close(
            nub.bar_fraction(),
            ModDownloadStatus::Downloading.phase_fraction(),
        );
        assert!(nub.is_indeterminate(), "no Content-Length ⇒ indeterminate");
        let zero = row_b(ModDownloadStatus::Downloading, Some((10, Some(0))));
        assert_f32_close(
            zero.bar_fraction(),
            ModDownloadStatus::Downloading.phase_fraction(),
        );
        assert!(zero.is_indeterminate());
        assert!(!row_b(ModDownloadStatus::Downloading, Some((5, Some(10)))).is_indeterminate());
        assert!(!row_b(ModDownloadStatus::Queued, Some((0, None))).is_indeterminate());
    }

    #[test]
    fn bar_fraction_is_strictly_monotonic_from_zero_no_reverse_nub_jerk() {
        let sz = Some(600_000u64);
        let none_yet = row_sz(ModDownloadStatus::Downloading, None, sz);
        assert_f32_close(none_yet.bar_fraction(), 0.0);
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
        assert_f32_close(seq[0], 0.0);
        assert!((seq[seq.len() - 1] - 1.0).abs() < 1e-6);
        let cl_only = row_sz(ModDownloadStatus::Downloading, Some((0, Some(1000))), None);
        assert_f32_close(cl_only.bar_fraction(), 0.0);
        assert!(!cl_only.is_indeterminate());
    }

    #[test]
    fn bar_fraction_falls_back_to_phase_when_no_byte_signal() {
        for s in [
            ModDownloadStatus::Queued,
            ModDownloadStatus::Downloading,
            ModDownloadStatus::Extracting,
            ModDownloadStatus::Staged,
            ModDownloadStatus::Skipped,
        ] {
            assert_f32_close(row_b(s, None).bar_fraction(), s.phase_fraction());
        }
        assert_f32_close(
            row_b(ModDownloadStatus::Extracting, Some((100, Some(100)))).bar_fraction(),
            ModDownloadStatus::Extracting.phase_fraction(),
        );
        assert_f32_close(
            row_b(ModDownloadStatus::Staged, Some((100, Some(100)))).bar_fraction(),
            1.0,
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
        assert_f32_close(only_unknown_skipped.download_overall_fraction(), 0.0);
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
            (f - (48.0_f32 / 51.0)).abs() < 0.001,
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
        assert_f32_close(downloading.extract_overall_fraction(), 0.0);
        assert_eq!(downloading.extract_overall_pct(), 0);

        let extracting = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Extracting),
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
                row("a", ModDownloadStatus::Extracting),
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
        assert_eq!(p2.skipped.len(), 1, "skipped mods carried through");
        assert!((p2.rows[0].bar_fraction() - 0.25).abs() < 0.001);
    }

    #[test]
    fn from_wizard_state_full_classifies_lifecycle_and_carries_skipped() {
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
        let p =
            DownloadProgress::from_wizard_state_full(&st, &BTreeMap::new(), &sk, &BTreeMap::new());
        assert_eq!(p.rows[0].status, ModDownloadStatus::Staged);
        assert_eq!(p.rows[1].status, ModDownloadStatus::Extracting);
        assert_eq!(p.rows[2].status, ModDownloadStatus::Downloading);
        assert_eq!(p.rows[3].status, ModDownloadStatus::Downloading);
        assert_eq!(p.skipped.len(), 1);
        assert_eq!(p.total(), 5, "4 rows + 1 skipped");
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
