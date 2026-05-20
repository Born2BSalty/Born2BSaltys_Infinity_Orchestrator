// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::install::stage_downloading::{self, DownloadProgress, DownloadScreenCopy};
use crate::ui::shared::redesign_tokens::ThemePalette;

const fn fork_download_copy() -> DownloadScreenCopy {
    DownloadScreenCopy {
        title: "Downloading fork",
        sub: "fetching the parent's mods \u{2014} Step 2 opens automatically when ready",
        hint: Some(
            "after download: components auto-selected \u{00B7} order applied \u{00B7} lands on Step 2",
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForkDownloadOutcome {
    #[default]
    Stay,
    Cancel,
    Import,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    progress: &DownloadProgress,
) -> ForkDownloadOutcome {
    match stage_downloading::render(ui, palette, fork_download_copy(), progress) {
        stage_downloading::DownloadingOutcome::Cancel => ForkDownloadOutcome::Cancel,
        stage_downloading::DownloadingOutcome::Advance => ForkDownloadOutcome::Import,
        stage_downloading::DownloadingOutcome::Stay => ForkDownloadOutcome::Stay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fork_copy_is_spec_5_3_verbatim() {
        let c = fork_download_copy();
        assert_eq!(c.title, "Downloading fork");
        assert_eq!(
            c.hint,
            Some(
                "after download: components auto-selected \u{00B7} order applied \u{00B7} lands on Step 2"
            )
        );
        assert!(c.sub.contains("Step 2 opens automatically"));
    }
}
