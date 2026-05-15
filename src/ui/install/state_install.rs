// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `InstallScreenState` — per-screen UI state for the Install Modlist
// destination (SPEC §4). Lives on `OrchestratorApp::install_screen_state`.
// Persists across screen visits within a session; not written to disk (the
// pasted code / chosen destination are transient until an install starts —
// Phase 7).
//
// **Run 3 scope.** The four-stage machine is declared *whole*
// (`Paste | Preview | Downloading | InstallingStub`) so the dispatcher and
// the back-navigation are total, but only `Paste` and `InstallingStub` are
// fully implemented this run. `Preview` / `Downloading` render minimal
// Run-4 / Run-5 placeholders (same chassis as the stage-4 stub) so the flow
// is navigable end to end. The share-code parse, the 6 preview tabs, the
// `allow_auto_install` gate, and the download/extract engines are explicitly
// NOT in Run 3 (Run 4 / Run 5 / no `modlist_share.rs` touch this run).
//
// The preview/download payload fields the full plan lists
// (`parsed_preview`, `active_preview_tab`, `download_progress`) are NOT
// declared yet — Run 3 keeps the state minimal and inert. Pulling in
// `bio::app::modlist_share::ModlistSharePreview` (or a tab enum / progress
// struct) now would be speculative abstraction for code that lands in
// Run 4 / Run 5; `preview_cached` is the single bit the stage-4 stub needs
// to decide its `← Back to preview` target (SPEC §4.4). Run 4 grows this
// struct with the real preview state when it implements the parse.
//
// **DestChoice → WeiDU flag mapping (SPEC §13.12 #1 + #6).** The radio
// labels are wireframe-verbatim (`screens.jsx:123-154`); the flag mapping is
// `DestChoice::to_flags` below — a pure function with unit tests. Run 3 only
// *records* the choice + exposes the mapping; the mapping is applied to the
// orchestrator-owned `WizardState.step1` at install start (Phase 7), so no
// BIO state is mutated this run.
//
// SPEC: §4.1 (paste stage), §4.4 (stage 4 stub), §13.12 #1/#6 (flag policy).

/// The four Install Modlist stages (SPEC §4: paste → preview → downloading →
/// installing). The machine is whole so the dispatcher + back-navigation are
/// total; Run 3 implements `Paste` + `InstallingStub`, with `Preview` /
/// `Downloading` rendering Run-4 / Run-5 placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallStage {
    /// Stage 1 — destination + DestinationNotEmptyWarning + import-code
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

/// The WeiDU / install-runner flags a `DestChoice` resolves to, per
/// SPEC §13.12. These mirror `bio::app::state::Step1State` fields
/// (`prepare_target_dirs_before_install`, `backup_targets_before_eet_copy`,
/// `skip_installed`, `check_last_installed`). Run 3 only computes this; the
/// values are written into the orchestrator-owned `WizardState.step1` at
/// install start (Phase 7) — no BIO state is touched this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestFlags {
    /// `prepare_target_dirs_before_install` — wipe/prepare the target dirs
    /// before WeiDU runs (SPEC §13.12 #6).
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
    /// SPEC §13.12 #1 + #6 — the canonical DestChoice → flag mapping.
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

/// Per-screen Install Modlist UI state. Minimal + whole for Run 3 (see the
/// module header for why preview/download payload fields are deferred).
#[derive(Debug, Clone, Default)]
pub struct InstallScreenState {
    /// Active stage. Defaults to `Paste` so a fresh entry from the rail / the
    /// Home `paste import code` CTA lands on stage 1.
    pub stage: InstallStage,
    /// Destination folder string (FolderInput value). The
    /// `DestinationNotEmptyWarning` shows only when this is set AND non-empty
    /// on disk.
    pub destination: String,
    /// The chosen `DestinationNotEmptyWarning` option, if any. `None` until
    /// the user picks one (no warning shown, or shown-but-unanswered).
    pub destination_choice: Option<DestChoice>,
    /// The pasted BIO-MODLIST-V1 share code. Empty disables the footer
    /// primary in non-partial mode (SPEC §4.1).
    pub import_code: String,
    /// Whether a parsed preview has been cached (drives the stage-4 stub's
    /// `← Back to preview` target — SPEC §4.4: preview if cached, else
    /// paste). Run 3 sets this to `false`; Run 4 flips it when the parse
    /// succeeds.
    pub preview_cached: bool,
}

impl InstallScreenState {
    /// `true` when the user picked the Continue-Partial option. In partial
    /// mode the import-code section disappears and the footer primary becomes
    /// `Continue Install →` (SPEC §4.1).
    pub fn is_partial(&self) -> bool {
        self.destination_choice == Some(DestChoice::Continue)
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
}
