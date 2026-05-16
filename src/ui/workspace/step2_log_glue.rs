// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step2_log_glue` — the orchestrator-side sibling for the two
// `Step2Action::SelectBgeeViaLog` / `SelectBg2eeViaLog` log-picker variants.
//
// Per the Phase-6 C2 audit + SPEC §1 decision order: BIO's UI-layer wrapper
// `bio::ui::app_step2_log::apply_weidu_log_selection(app: &mut WizardApp, ..)`
// couples an `rfd::FileDialog` + `app.save_settings_best_effort()` to a
// `&mut WizardApp`, which is outside carve-out #4's state-only scope. This is
// a **simple workflow** (a file-picker wrapper + a state mutation + a
// settings-persistence trigger), so per the decision order it gets a net-new
// orchestrator sibling — no carve-out. The sibling:
//   (a) opens the *same* `rfd::FileDialog` BIO opens (same filter / title /
//       start-directory logic — replicated, not invented),
//   (b) writes `wizard_state.step1.{bgee,bg2ee}_log_file`,
//   (c) triggers the orchestrator's settings-persistence cycle (the
//       debounced `bio_settings.json` write — see note below),
//   (d) calls the underlying `bio::app::app_step2_log::
//       apply_weidu_log_selection_from_path` (`pub(crate)`, same-crate
//       reachable) for the in-memory state mutation.
//
// **Settings-persistence trigger.** BIO's wrapper calls
// `app.save_settings_best_effort()`. The orchestrator equivalent is its
// debounced `bio_settings.json` cycle: `OrchestratorApp::tick_bio_settings`
// already auto-detects drift between the live `wizard_state.step1`-derived
// snapshot and the on-disk one every frame and starts its own debounce, so
// writing the field is sufficient to schedule the write — there is no
// separate "mark dirty" call (this matches how Settings → Paths edits
// persist; see `OrchestratorApp::bio_settings_snapshot`). We additionally
// arm `bio_settings_last_dirty_at` so the debounce window starts from the
// pick rather than the next drift-detecting frame (promptness parity with
// BIO's immediate best-effort save).
//
// **Confirmed-destructive opt-in (SPEC §6.10 + wireframe `askWeiduImport`).**
// Select-via-WeiDU-Log replaces *every* selection on the tab, so it is gated
// by the danger `ConfirmDialog` (rendered by `workspace_step2`, owned via
// `WorkspaceStep2State::pending_weidu_log_confirm`). The action — and hence
// this sibling — only runs *after* the user accepted that dialog. Therefore
// the picker itself is the final destructive gesture: cancelling it backs
// out the whole flow, so this sibling **applies nothing and mutates nothing
// on picker-cancel** — the user's current Step-2 selection is preserved
// intact. (No "fall back to the resolved default log" — that legacy BIO
// parity would silently wipe the selection, the data loss the redesign's
// cancel-is-a-no-op model forbids.)
//
// SPEC: §6.4 (Select <TAB> via WeiDU Log — BIO-fidelity reuse), §6.10
//       (ConfirmDialog only for Select via WeiDU Log — destructive), §1
//       (decision order: sibling for simple workflows).

use std::path::PathBuf;
use std::time::Instant;

use rfd::FileDialog;

use crate::app::app_step2_log::{
    apply_weidu_log_selection_from_path, resolve_bg2_weidu_log_path, resolve_bgee_weidu_log_path,
};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

/// Orchestrator equivalent of BIO's `apply_weidu_log_selection(app, bgee)`.
/// `bgee == true` → BGEE log; `false` → BG2EE log.
pub fn apply_weidu_log_selection_for_orchestrator(orchestrator: &mut OrchestratorApp, bgee: bool) {
    let current = if bgee {
        resolve_bgee_weidu_log_path(&orchestrator.wizard_state.step1)
    } else {
        resolve_bg2_weidu_log_path(&orchestrator.wizard_state.step1)
    };

    let picked = pick_weidu_log_file(current, bgee);

    // Redesign behavior (SPEC §6.10 + wireframe `askWeiduImport`): this flow
    // is a **confirmed destructive opt-in** — the danger ConfirmDialog has
    // already been accepted before we get here, and the only remaining
    // gesture is the file pick. If the user cancels the picker they have
    // backed out, so we **abort**: apply nothing, mutate nothing, leave the
    // current Step-2 selection exactly as it was. (This deliberately drops
    // BIO's legacy "fall back to the resolved default log on cancel" parity:
    // applying the default would silently wipe the user's selection — the
    // exact data-loss the redesign's cancel-is-a-no-op model forbids.)
    let Some(path) = picked else {
        return;
    };

    let picked_str = path.to_string_lossy().to_string();
    if bgee {
        orchestrator.wizard_state.step1.bgee_log_file = picked_str;
    } else {
        orchestrator.wizard_state.step1.bg2ee_log_file = picked_str;
    }
    // (c) Trigger the orchestrator's settings-persistence cycle. The
    // tick already auto-detects the drift; arming the debounce start
    // matches BIO's immediate-best-effort-save promptness.
    orchestrator
        .bio_settings_last_dirty_at
        .get_or_insert_with(Instant::now);

    // (d) Underlying BIO state mutation (parse + apply the log selection) —
    // only ever reached with a user-picked path (cancel returned above).
    apply_weidu_log_selection_from_path(&mut orchestrator.wizard_state, bgee, Some(path));
}

/// Replicates BIO's `pick_weidu_log_file` (`src/ui/app_step2_log.rs:36-50`)
/// verbatim: a `WeiDU Log` `.log` filter, a tab-specific title, and the
/// current log's parent as the start directory.
fn pick_weidu_log_file(current: Option<PathBuf>, bgee: bool) -> Option<PathBuf> {
    let mut dialog = FileDialog::new()
        .add_filter("WeiDU Log", &["log"])
        .set_title(if bgee {
            "Select BGEE WeiDU log"
        } else {
            "Select BG2EE WeiDU log"
        });
    if let Some(cur) = &current
        && let Some(dir) = cur.parent()
    {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file()
}
