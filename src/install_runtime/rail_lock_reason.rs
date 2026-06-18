// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Instant;

#[derive(Debug, Clone)]
pub enum RailLockReason {
    InstallRunning {
        modlist_id: String,

        modlist_label: String,

        started_at: Instant,
    },
}

#[must_use]
pub fn rail_lock_tooltip(running_modlist_name: &str) -> String {
    format!(
        "An install is already running for {running_modlist_name}. \
         Wait for it to finish before starting another."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_fills_the_modlist_placeholder_verbatim() {
        assert_eq!(
            rail_lock_tooltip("Polished BG2EE"),
            "An install is already running for Polished BG2EE. \
             Wait for it to finish before starting another."
        );
    }

    #[test]
    fn reason_carries_modlist_label_and_start_instant() {
        let now = Instant::now();
        let r = RailLockReason::InstallRunning {
            modlist_id: "ABC0123".to_string(),
            modlist_label: "Polished BG2EE".to_string(),
            started_at: now,
        };
        match r {
            RailLockReason::InstallRunning {
                modlist_id,
                modlist_label,
                started_at,
            } => {
                assert_eq!(modlist_id, "ABC0123");
                assert_eq!(modlist_label, "Polished BG2EE");
                assert_eq!(started_at, now);
            }
        }
    }
}
