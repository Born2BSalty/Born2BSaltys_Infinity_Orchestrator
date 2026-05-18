// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `InstallScreenState` — per-screen UI state for the Install Modlist
// destination (SPEC §4). Lives on `OrchestratorApp::install_screen_state`.
// Persists across screen visits within a session; not written to disk (the
// pasted code / chosen destination are transient until an install starts —
// Phase 7).
//
// **Run 4 scope.** The four-stage machine is declared *whole*
// (`Paste | Preview | Downloading | InstallingStub`) so the dispatcher and
// the back-navigation are total. Run 3 implemented `Paste` +
// `InstallingStub`; Run 4 implements `Preview` (the parsed share-code
// preview — Overview Box + 6 tabs + the `allow_auto_install` gate +
// provenance display + `ForkInfoPopup`). `Downloading` still renders the
// Run-5 placeholder; the download/extract engines are NOT in Run 4.
//
// Run 4 grows this struct with the real preview state:
//   - `parsed_preview: Option<ModlistSharePreview>` — the result of
//     `preview_modlist_share_code`, cached so the parse runs once on
//     `Paste → Preview` (not per-frame). `None` while on Paste; `Some`
//     after a successful parse. Carries (via carve-out #5) the
//     `allow_auto_install` bit + the provenance trio
//     `name` / `author` / `forked_from`.
//   - `preview_parse_error: Option<String>` — set when the parse fails so
//     the preview can show the error instead of a blank box (the wireframe
//     assumes a valid code; a paste-stage parse failure is real and must be
//     surfaced, not silently swallowed).
//   - `active_preview_tab: PreviewTab` — the selected Content-Box tab.
//   - `fork_info_open: bool` — whether the `ForkInfoPopup` is open.
// `preview_cached` stays (the stage-4 stub's `← Back to preview` target,
// SPEC §4.4) and is now set `true` when the parse succeeds.
//
// **DestChoice → WeiDU flag mapping (SPEC §13.12 #1 + #6).** The radio
// labels are wireframe-verbatim (`screens.jsx:123-154`); the flag mapping is
// `DestChoice::to_flags` below — a pure function with unit tests. Run 3 only
// *records* the choice + exposes the mapping; the mapping is applied to the
// orchestrator-owned `WizardState.step1` at install start (Phase 7), so no
// BIO state is mutated this run.
//
// SPEC: §4.1 (paste stage), §4.2 (preview stage + `allow_auto_install`
//       gate + provenance), §4.4 (stage 4 stub), §13.3 (provenance fields),
//       §13.12 #1/#6 (flag policy), §1 (carve-out #5).

// rationale: per-screen UI state + the pure `DestChoice::to_flags` mapping —
// `DestFlags`'s 4 flags model 4 independent WeiDU policy bits (intentional,
// not a state-machine smell); `Self`/`const fn`/`#[must_use]` and the
// doc-paragraph-length lint are churn without behavior value (Cat 3).
#![allow(
    clippy::struct_excessive_bools,
    clippy::use_self,
    clippy::missing_const_for_fn,
    clippy::must_use_candidate,
    clippy::too_long_first_doc_paragraph
)]

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::stage_downloading::DownloadProgress;

/// The four Install Modlist stages (SPEC §4: paste → preview → downloading →
/// installing). The machine is whole so the dispatcher + back-navigation are
/// total; Run 3 implements `Paste` + `InstallingStub`, with `Preview` /
/// `Downloading` rendering Run-4 / Run-5 placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallStage {
    /// Stage 1 — destination + `DestinationNotEmptyWarning` + import-code
    /// textarea + footer (SPEC §4.1). Fully implemented this run.
    #[default]
    Paste,
    /// Stage 2 — parsed share-code preview (SPEC §4.2). Run 4. This run it
    /// renders a placeholder.
    Preview,
    /// Stage 3 — per-mod download/extract grid (SPEC §4.3). Run 5. This run
    /// it renders a placeholder.
    Downloading,
    /// Stage 4 — the install runtime (SPEC §4.4). Full runtime is Phase 7;
    /// this run renders the §4.4 stub.
    InstallingStub,
}

/// The `DestinationNotEmptyWarning` radio choice (SPEC §4.1 / §13.12 #6).
/// Labels in the UI are wireframe-verbatim (`screens.jsx:123-154`):
///   - `Clear`    → "Clear contents"
///   - `Backup`   → "Backup contents then proceed"
///   - `Continue` → "Continue partial installation" (only when partial is
///     allowed)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestChoice {
    /// Wipe the target then reinstall.
    Clear,
    /// Move existing files to a backup folder, then install.
    Backup,
    /// Continue a partial install — skip the share-code requirement, pick up
    /// where a previous install left off.
    Continue,
}

/// The `WeiDU` / install-runner flags a `DestChoice` resolves to, per
/// SPEC §13.12. These mirror `bio::app::state::Step1State` fields
/// (`prepare_target_dirs_before_install`, `backup_targets_before_eet_copy`,
/// `skip_installed`, `check_last_installed`). Run 3 only computes this; the
/// values are written into the orchestrator-owned `WizardState.step1` at
/// install start (Phase 7) — no BIO state is touched this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestFlags {
    /// `prepare_target_dirs_before_install` — wipe/prepare the target dirs
    /// before `WeiDU` runs (SPEC §13.12 #6).
    pub prepare_target_dirs_before_install: bool,
    /// `backup_targets_before_eet_copy` — move existing files aside first
    /// (SPEC §13.12 #6).
    pub backup_targets_before_eet_copy: bool,
    /// `-s` (skip installed) — ON only in Continue Partial Install
    /// (SPEC §13.12 #1).
    pub skip_installed: bool,
    /// `-c` (check last installed) — ON only in Continue Partial Install
    /// (SPEC §13.12 #1).
    pub check_last_installed: bool,
}

impl DestChoice {
    /// SPEC §13.12 #1 + #6 — the canonical `DestChoice` → flag mapping.
    ///
    /// | choice     | prepare | backup | -s / -c |
    /// |------------|---------|--------|---------|
    /// | `Clear`    | true    | false  | off     |
    /// | `Backup`   | true    | true   | off     |
    /// | `Continue` | false   | false  | on      |
    ///
    /// - `Clear` / `Backup` are fresh installs: prepare the target dirs
    ///   (`prepare_target_dirs_before_install = true`); `Backup` additionally
    ///   moves existing files aside first (`backup_targets_before_eet_copy =
    ///   true`); skip/check-last stay OFF (§13.12 #6 + #1 "OFF for fresh
    ///   installs").
    /// - `Continue` is the Continue Partial Install workflow: do NOT prepare
    ///   the target dirs (`prepare_target_dirs_before_install = false`), no
    ///   backup, and turn `-s` (skip installed) + `-c` (check last installed)
    ///   ON (§13.12 #1 "automatically ON when the user enters the Continue
    ///   Partial Install workflow").
    pub fn to_flags(self) -> DestFlags {
        match self {
            DestChoice::Clear => DestFlags {
                prepare_target_dirs_before_install: true,
                backup_targets_before_eet_copy: false,
                skip_installed: false,
                check_last_installed: false,
            },
            DestChoice::Backup => DestFlags {
                prepare_target_dirs_before_install: true,
                backup_targets_before_eet_copy: true,
                skip_installed: false,
                check_last_installed: false,
            },
            DestChoice::Continue => DestFlags {
                prepare_target_dirs_before_install: false,
                backup_targets_before_eet_copy: false,
                skip_installed: true,
                check_last_installed: true,
            },
        }
    }
}

/// The six Content-Box tabs of the preview stage, in the file-folder strip
/// (SPEC §4.2; wireframe `screens.jsx::ImportPreviewTabs` line 470 — exact
/// order + labels). `display_label` is the wireframe-verbatim tab caption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewTab {
    /// BIO version / game / install mode / log counts / included flags /
    /// "what import will do" recap (default tab — wireframe `useState`
    /// initial is `"Summary"`).
    #[default]
    Summary,
    /// Verbatim BGEE `weidu.log` from the share code.
    BgeeWeidu,
    /// Verbatim BG2EE `weidu.log` from the share code.
    Bg2eeWeidu,
    /// `mod_downloads_user.toml` excerpt packaged in the share.
    UserDownloads,
    /// Pinned `[refs]` / `[sources]` TOML.
    InstalledRefs,
    /// `<mod> | <source> | <file>` list of restored mod-config files.
    ModConfigs,
}

impl PreviewTab {
    /// All six tabs in wireframe order (for the tab-strip render loop).
    pub const ALL: [PreviewTab; 6] = [
        PreviewTab::Summary,
        PreviewTab::BgeeWeidu,
        PreviewTab::Bg2eeWeidu,
        PreviewTab::UserDownloads,
        PreviewTab::InstalledRefs,
        PreviewTab::ModConfigs,
    ];

    /// Wireframe-verbatim tab caption (`screens.jsx:470`).
    pub fn display_label(self) -> &'static str {
        match self {
            PreviewTab::Summary => "Summary",
            PreviewTab::BgeeWeidu => "BGEE WeiDU",
            PreviewTab::Bg2eeWeidu => "BG2EE WeiDU",
            PreviewTab::UserDownloads => "User Downloads",
            PreviewTab::InstalledRefs => "Installed Refs",
            PreviewTab::ModConfigs => "Mod Configs",
        }
    }
}

/// Per-screen Install Modlist UI state. Run 4 grows the preview state (see
/// the module header); `destination` / `import_code` / the `DestChoice`
/// machinery stay from Run 3.
#[derive(Debug, Clone, Default)]
pub struct InstallScreenState {
    /// Active stage. Defaults to `Paste` so a fresh entry from the rail / the
    /// Home `paste import code` CTA lands on stage 1.
    pub stage: InstallStage,
    /// Destination folder string (`FolderInput` value). The
    /// `DestinationNotEmptyWarning` shows only when this is set AND non-empty
    /// on disk.
    pub destination: String,
    /// The chosen `DestinationNotEmptyWarning` option, if any. `None` until
    /// the user picks one (no warning shown, or shown-but-unanswered).
    pub destination_choice: Option<DestChoice>,
    /// The pasted BIO-MODLIST-V1 share code. Empty disables the footer
    /// primary in non-partial mode (SPEC §4.1).
    pub import_code: String,
    /// The parsed share-code preview (carve-out #5: carries
    /// `allow_auto_install` + the provenance trio). `None` until the parse
    /// runs on `Paste → Preview`; cached so the parse is one-shot, not
    /// per-frame. Cleared when the user goes back to Paste.
    ///
    /// `pub(crate)` (the rest of the struct is `pub`): `ModlistSharePreview`
    /// is BIO's `pub(crate)` type (carve-out #5 keeps it at its existing
    /// field visibility — not a redesign decision), so this field can't be
    /// more public than the type it holds. Every reader is in-crate.
    pub(crate) parsed_preview: Option<ModlistSharePreview>,
    /// Set when `preview_modlist_share_code` returned `Err` so the preview
    /// stage shows the failure instead of a blank box (mutually exclusive
    /// with `parsed_preview` — a parse either yields a preview or an error).
    pub preview_parse_error: Option<String>,
    /// The selected Content-Box tab (SPEC §4.2).
    pub active_preview_tab: PreviewTab,
    /// Whether the `ForkInfoPopup` (SPEC §10.9) is open. Toggled by the
    /// `⑂ fork info` button; the popup is Close-only + non-blocking.
    pub fork_info_open: bool,
    /// Whether a parsed preview has been cached (drives the stage-4 stub's
    /// `← Back to preview` target — SPEC §4.4: preview if cached, else
    /// paste). Set `true` when the parse succeeds.
    pub preview_cached: bool,
    /// Stage-3 per-mod download/extract progress model (SPEC §4.3).
    /// **Phase 7 P7.T17 feeds it live** each frame from BIO's auto-build
    /// state (`DownloadProgress::from_wizard_state`) while on the
    /// Downloading stage; the Phase-5 empty-grid chassis is the
    /// pre-arm / fork-download fallback. Cleared whenever the user leaves
    /// Downloading back to Preview (a re-parsed code must not inherit a
    /// stale grid).
    pub download_progress: DownloadProgress,
    /// **P7.T17 — pipeline-armed-once latch.** `false` until the
    /// Downloading stage has run the import + per-install-dir derivation +
    /// auto-build arm exactly once for this code; flipped `true` by
    /// `stage_downloading::render_live` on first entry so the import /
    /// `arm_auto_build` (which set the `pending_saved_log_*` flags) is not
    /// re-run every frame (that would reset the pipeline mid-flight).
    /// Reset to `false` whenever the user leaves Downloading back to
    /// Preview (alongside `download_progress`) so a re-entry re-arms with
    /// a possibly-changed code/destination. Not persisted (transient until
    /// an install starts — Phase 7).
    pub pipeline_armed: bool,
    /// **Non-masking arm-failure surface (the "it just sits there, no
    /// feedback" fix).** Set to the error string when
    /// `auto_build_driver::prepare_install_dirs_and_maybe_import` returns
    /// `Err` on the one-shot arm: the latch stays `true` (no per-frame
    /// re-import / I/O churn of a bad code — the original design intent),
    /// but the Downloading chrome renders this prominently instead of the
    /// failure being buried in the empty-grid-hidden `step2.scan_status`
    /// sub-text (the reported permanent inert "0 / 0 mods" mystery). Mirror
    /// of `preview_parse_error` (same `Option<String>` lifecycle). Cleared
    /// by `clear_preview()` alongside `pipeline_armed` so a re-entry from
    /// Preview re-arms cleanly. Not persisted.
    pub pipeline_arm_error: Option<String>,
}

impl InstallScreenState {
    /// `true` when the user picked the Continue-Partial option. In partial
    /// mode the import-code section disappears and the footer primary becomes
    /// `Continue Install →` (SPEC §4.1).
    pub fn is_partial(&self) -> bool {
        self.destination_choice == Some(DestChoice::Continue)
    }

    /// Drop the cached preview + any parse error and close the fork popup.
    /// Called when the user goes back to Paste (the pasted code may change,
    /// so a stale preview must not survive) and when a fresh parse is about
    /// to run. Also drops the live Downloading grid + the P7.T17
    /// pipeline-armed latch so a re-parsed code re-arms the import →
    /// auto-build pipeline from scratch (never inherits the prior code's
    /// in-flight pipeline state).
    pub fn clear_preview(&mut self) {
        self.parsed_preview = None;
        self.preview_parse_error = None;
        self.fork_info_open = false;
        self.preview_cached = false;
        self.download_progress = DownloadProgress::default();
        self.pipeline_armed = false;
        self.pipeline_arm_error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_maps_to_prepare_on_backup_off_no_skip() {
        // SPEC §13.12 #6: `clear` = prepare ON, backup OFF.
        // SPEC §13.12 #1: -s/-c OFF for fresh installs.
        let f = DestChoice::Clear.to_flags();
        assert!(f.prepare_target_dirs_before_install);
        assert!(!f.backup_targets_before_eet_copy);
        assert!(!f.skip_installed);
        assert!(!f.check_last_installed);
    }

    #[test]
    fn backup_maps_to_prepare_on_backup_on_no_skip() {
        // SPEC §13.12 #6: `backup` = prepare ON, backup ON.
        let f = DestChoice::Backup.to_flags();
        assert!(f.prepare_target_dirs_before_install);
        assert!(f.backup_targets_before_eet_copy);
        assert!(!f.skip_installed);
        assert!(!f.check_last_installed);
    }

    #[test]
    fn continue_maps_to_prepare_off_backup_off_skip_on() {
        // SPEC §13.12 #6: `continue` = prepare OFF, backup OFF.
        // SPEC §13.12 #1: -s/-c ON in Continue Partial Install.
        let f = DestChoice::Continue.to_flags();
        assert!(!f.prepare_target_dirs_before_install);
        assert!(!f.backup_targets_before_eet_copy);
        assert!(f.skip_installed);
        assert!(f.check_last_installed);
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
        // screens.jsx:470 — exact caption strings, exact order.
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
        // Wireframe `useState("Summary")` initial.
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
        st.clear_preview();
        assert!(st.parsed_preview.is_none());
        assert!(st.preview_parse_error.is_none());
        assert!(!st.fork_info_open);
        assert!(!st.preview_cached);
    }
}
