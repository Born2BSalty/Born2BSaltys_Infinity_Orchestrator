// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{ConfirmDialog, ConfirmOutcome};

#[must_use]
pub const fn global_mods_scan_confirm<'a>() -> ConfirmDialog<'a> {
    ConfirmDialog {
        id_salt: "step2_global_mods_scan",
        title: "Rescan with a different mods source?",
        body: "Rescanning with a different mods source can drop selected components \
               not present in the new source.",
        confirm_label: "Scan anyway",
        danger: false,
    }
}

#[must_use]
pub const fn resolve_confirm_outcome(
    pending: Option<()>,
    outcome: ConfirmOutcome,
) -> (bool, Option<()>) {
    match (pending, outcome) {
        (Some(()), ConfirmOutcome::Confirmed) => (true, None),
        (Some(()), ConfirmOutcome::Cancelled) => (false, None),
        _ => (false, pending),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_confirmed_triggers_scan_and_clears_flag() {
        let (should_scan, new_pending) =
            resolve_confirm_outcome(Some(()), ConfirmOutcome::Confirmed);
        assert!(should_scan, "confirmed must trigger scan");
        assert!(new_pending.is_none(), "flag must be cleared on confirm");
    }

    #[test]
    fn pending_cancelled_clears_flag_no_scan() {
        let (should_scan, new_pending) =
            resolve_confirm_outcome(Some(()), ConfirmOutcome::Cancelled);
        assert!(!should_scan, "cancelled must not trigger scan");
        assert!(new_pending.is_none(), "flag must be cleared on cancel");
    }

    #[test]
    fn pending_outcome_leaves_state_unchanged() {
        let (should_scan, new_pending) = resolve_confirm_outcome(Some(()), ConfirmOutcome::Pending);
        assert!(!should_scan, "pending outcome must not trigger scan");
        assert!(
            new_pending.is_some(),
            "flag must remain set while outcome is pending"
        );
    }

    #[test]
    fn no_pending_flag_never_scans_regardless_of_outcome() {
        let (scan_confirmed, _) = resolve_confirm_outcome(None, ConfirmOutcome::Confirmed);
        let (scan_cancelled, _) = resolve_confirm_outcome(None, ConfirmOutcome::Cancelled);
        let (scan_pending, _) = resolve_confirm_outcome(None, ConfirmOutcome::Pending);
        assert!(!scan_confirmed);
        assert!(!scan_cancelled);
        assert!(!scan_pending);
    }
}
