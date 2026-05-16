// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step2_log_confirm` — the **Select-via-WeiDU-Log destructive confirm**
// (Phase 6 Run 1e, fix #1). The direct analogue of Home's `confirm_delete`:
// it owns only the title + body + button-label strings; the actual modal is
// the shared, non-blocking `ConfirmDialog`
// (`widgets/dialogs/confirm_dialog.rs`) already used by Home Delete /
// Reinstall — reused, not rebuilt.
//
// ## Why this gate exists
//
// SPEC §6.10: "Step 2 uses ConfirmDialog **only** for Select via WeiDU Log
// (since it's destructive — it replaces all selections on the tab)." The
// canonical wireframe `askWeiduImport` (`screens.jsx:2778-2784`) is the flow:
// click `Select <Tab> via WeiDU Log` → **danger ConfirmDialog** → on confirm
// → file picker → parse + apply; cancel (dialog OR picker) → abort, change
// nothing. Run 1d shipped the button wired straight to the picker with **no
// confirm** — this module + the `workspace_step2` render/clear glue closes
// that gap. (Picker-cancel non-destructiveness is `step2_log_glue`'s job.)
//
// ## Copy — VERBATIM from the wireframe
//
// `wireframe-preview/screens.jsx:2778-2784` renders:
//   title:        `Replace ${upperTab} selections from a WeiDU log?`
//   message:      `This will overwrite every component selection on the
//                  ${upperTab} bucket with the contents of the chosen
//                  weidu.log. Make sure the log was produced from the same
//                  mod versions you have downloaded — otherwise components
//                  may resolve to the wrong rows or fail to install.`
//   confirmLabel: `Pick a weidu.log...`
//   danger:       true
// `upperTab` is the active game tab upper-cased (`BGEE` / `BG2EE`). The
// wireframe is canonical for copy (HANDOFF source-of-truth ordering), so the
// strings here track `screens.jsx`.
//
// SPEC: §6.10 (ConfirmDialog only for Select via WeiDU Log — destructive),
//       §6.4 (the Select-via-Log button); wireframe `screens.jsx:2778-2784`.

// rationale: trivial dialog-text helpers — `const fn`/`#[must_use]` and the
// doc-paragraph-length lint are churn without behavior value (Cat 3).
#![allow(
    clippy::missing_const_for_fn,
    clippy::must_use_candidate,
    clippy::too_long_first_doc_paragraph
)]

use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::ConfirmDialog;

/// The upper-cased tab name for `bgee == true` (BGEE) / `false` (BG2EE) —
/// the wireframe `upperTab` substituted into both strings.
fn upper_tab(bgee: bool) -> &'static str {
    if bgee { "BGEE" } else { "BG2EE" }
}

/// Build the destructive-confirm's title + body for the target tab
/// (`bgee == true` → BGEE bucket; `false` → BG2EE). Both are
/// wireframe-verbatim (`screens.jsx:2778-2784`) with `upperTab`
/// substituted.
pub fn weidu_log_dialog_text(bgee: bool) -> (String, String) {
    let tab = upper_tab(bgee);
    let title = format!("Replace {tab} selections from a WeiDU log?");
    let body = format!(
        "This will overwrite every component selection on the {tab} bucket \
         with the contents of the chosen weidu.log. Make sure the log was \
         produced from the same mod versions you have downloaded — otherwise \
         components may resolve to the wrong rows or fail to install."
    );
    (title, body)
}

/// Convenience: build the `ConfirmDialog` descriptor for the Select-via-Log
/// confirm (danger-styled, primary label `Pick a weidu.log...` — the
/// wireframe `confirmLabel`). `title`/`body` must outlive the borrow (caller
/// owns the `(String, String)` from [`weidu_log_dialog_text`]).
pub fn weidu_log_confirm<'a>(title: &'a str, body: &'a str) -> ConfirmDialog<'a> {
    ConfirmDialog {
        id_salt: "step2_select_via_weidu_log",
        title,
        body,
        confirm_label: "Pick a weidu.log...",
        danger: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_names_the_target_tab() {
        let (t, _) = weidu_log_dialog_text(true);
        assert_eq!(t, "Replace BGEE selections from a WeiDU log?");
        let (t2, _) = weidu_log_dialog_text(false);
        assert_eq!(t2, "Replace BG2EE selections from a WeiDU log?");
    }

    #[test]
    fn body_is_wireframe_verbatim() {
        let (_, b) = weidu_log_dialog_text(true);
        // The exact `screens.jsx:2780` string with `${upperTab}` = BGEE.
        assert_eq!(
            b,
            "This will overwrite every component selection on the BGEE \
             bucket with the contents of the chosen weidu.log. Make sure \
             the log was produced from the same mod versions you have \
             downloaded — otherwise components may resolve to the wrong \
             rows or fail to install."
        );
        let (_, b2) = weidu_log_dialog_text(false);
        assert!(b2.contains("on the BG2EE bucket"));
    }

    #[test]
    fn confirm_descriptor_is_danger_with_wireframe_label() {
        let (t, b) = weidu_log_dialog_text(true);
        let d = weidu_log_confirm(&t, &b);
        assert!(d.danger);
        assert_eq!(d.confirm_label, "Pick a weidu.log...");
        assert_eq!(d.id_salt, "step2_select_via_weidu_log");
    }
}
