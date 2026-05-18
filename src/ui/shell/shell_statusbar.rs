// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Redesign shell statusbar (Infinity Orchestrator binary).
//
// Per Phase 1 P1.T5 — paints the 26px footer:
//   - 1.5px top border in `redesign_border_strong`,
//   - `redesign_chrome_bg` background,
//   - left-aligned segments `● connected · <N> modlists · <J> jobs running`
//     separated by ` · `,
//   - right-aligned `v<crate version>`,
//   - status dot is `8×8px` filled in `redesign_status_dot` with a 1px
//     `redesign_border_strong` ring.
//
// **P7.T14 — running-install readout (SPEC §13.15).** When an install is
// running, the jobs segment reads `1 job running` and is followed by the
// running modlist + elapsed: `1 job running · <modlist> · <elapsed>`
// (the SPEC §13.15 "faint right-aligned `<modlist A> · <elapsed>`" — laid
// out inline after the jobs segment in the same left-aligned run, the
// shell's single-line footer having no separate right region between the
// segments and the version). Zero installs ⇒ `0 jobs running` exactly as
// before. Only one install can run at a time (SPEC §13.15) so the counter
// is 0 or 1, never higher.
//
// SPEC: §1.2 (26px footer status bar always visible), §13.15 (running-
// install readout), wireframe `index.html:175-184`, `app.jsx:148-155`.

// rationale: `mul_add` would change float rounding of layout math, so the
// suboptimal-flops rewrite is not behavior-neutral — suppressed (Cat 3).
#![allow(clippy::suboptimal_flops)]

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_status_dot, redesign_text_muted,
};

/// The running-install readout for the statusbar (SPEC §13.15) — the
/// modlist's display name + how long it has been running. `None` ⇒ no
/// install running (`0 jobs running`). Built by the caller from
/// `install_runtime::install_concurrency::install_in_progress` + the
/// registry name; the elapsed is formatted via [`format_elapsed`].
#[derive(Debug, Clone)]
pub struct RunningInstallStatus {
    /// Registry display name of the running modlist.
    pub modlist_name: String,
    /// Wall-clock elapsed since the install started (this process run).
    pub elapsed: std::time::Duration,
}

/// `H:MM:SS` if ≥ 1h, else `MM:SS` (zero-padded). Mirrors the duration
/// convention the success banner will use (P7.T4's
/// `format_install_duration`); kept local + tiny so the statusbar has no
/// cross-module dependency for a 3-line format (and so this run touches
/// only the Phase-1 statusbar file, per scope).
#[must_use]
pub fn format_elapsed(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Paint the redesign statusbar inside the given `ui` (caller is expected to
/// have allocated a 26px-tall strip — e.g., via `egui::TopBottomPanel::bottom`).
///
/// `modlist_count` is caller-provided (Phase 3 wires the real registry
/// count). `running_install` is the **P7.T14** readout: `Some` ⇒ the jobs
/// segment reads `1 job running` followed by `· <modlist> · <elapsed>`;
/// `None` ⇒ `0 jobs running` (the pre-Phase-7 behavior). SPEC §13.15: only
/// one install runs at a time, so this is the whole jobs story.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    modlist_count: usize,
    running_install: Option<&RunningInstallStatus>,
) {
    let rect = ui.max_rect();
    let painter = ui.painter();

    // Background fill.
    painter.rect_filled(rect, 0.0, redesign_chrome_bg(palette));

    // 1.5px top border.
    let top_y = rect.top() + REDESIGN_BORDER_WIDTH_PX * 0.5;
    painter.line_segment(
        [
            egui::pos2(rect.left(), top_y),
            egui::pos2(rect.right(), top_y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    let text_color = redesign_text_muted(palette);
    let font = egui::FontId::new(10.0, egui::FontFamily::Proportional);

    // Status dot — 8×8px filled in `redesign_status_dot` with a 1px ring.
    let dot_x = rect.left() + 12.0 + 4.0;
    let dot_center = egui::pos2(dot_x, rect.center().y);
    painter.circle_filled(dot_center, 4.0, redesign_status_dot(palette));
    painter.circle_stroke(
        dot_center,
        4.0,
        egui::Stroke::new(1.0, redesign_border_strong(palette)),
    );

    // Left-aligned text segments after the dot. SPEC §13.15: `1 job
    // running · <modlist> · <elapsed>` when an install runs, else
    // `0 jobs running`. The running modlist + elapsed are appended as
    // their own ` · `-separated segments (the shell footer is a single
    // left-aligned run; the version is the only right-aligned element).
    let mut x = dot_center.x + 4.0 + 8.0;
    let mut segments = vec!["connected".to_string(), format!("{modlist_count} modlists")];
    if let Some(run) = running_install {
        // Exactly one install can run (SPEC §13.15) ⇒ "1 job running".
        segments.push("1 job running".to_string());
        segments.push(run.modlist_name.clone());
        segments.push(format_elapsed(run.elapsed));
    } else {
        segments.push("0 jobs running".to_string());
    }
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            let galley = painter.layout_no_wrap("·".to_string(), font.clone(), text_color);
            let pos = egui::pos2(x, rect.center().y - galley.size().y * 0.5);
            painter.galley(pos, galley.clone(), text_color);
            x += galley.size().x + 8.0;
        }
        let galley = painter.layout_no_wrap(seg.clone(), font.clone(), text_color);
        let pos = egui::pos2(x, rect.center().y - galley.size().y * 0.5);
        let w = galley.size().x;
        painter.galley(pos, galley, text_color);
        x += w + 8.0;
    }

    // Right-aligned crate version.
    let version_text = format!("v{}", env!("CARGO_PKG_VERSION"));
    let galley = painter.layout_no_wrap(version_text, font, text_color);
    let pos = egui::pos2(
        rect.right() - 12.0 - galley.size().x,
        rect.center().y - galley.size().y * 0.5,
    );
    painter.galley(pos, galley, text_color);
}

/// Convenience: the natural pixel height of the statusbar strip
/// (matches the wireframe `.sk-statusbar` height: 26px).
pub const HEIGHT_PX: f32 = REDESIGN_STATUSBAR_HEIGHT_PX;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn elapsed_under_an_hour_is_mm_ss_zero_padded() {
        assert_eq!(format_elapsed(Duration::from_secs(0)), "00:00");
        assert_eq!(format_elapsed(Duration::from_secs(9)), "00:09");
        assert_eq!(format_elapsed(Duration::from_secs(65)), "01:05");
        assert_eq!(format_elapsed(Duration::from_secs(59 * 60 + 59)), "59:59");
    }

    #[test]
    fn elapsed_over_an_hour_is_h_mm_ss() {
        assert_eq!(format_elapsed(Duration::from_secs(3600)), "1:00:00");
        assert_eq!(
            format_elapsed(Duration::from_secs(3600 + 23 * 60 + 7)),
            "1:23:07"
        );
        assert_eq!(
            format_elapsed(Duration::from_secs(10 * 3600 + 5)),
            "10:00:05"
        );
    }

    #[test]
    fn sub_second_truncates_to_zero() {
        // Wall-clock elapsed carries nanos; the readout is whole-second.
        assert_eq!(format_elapsed(Duration::from_millis(1500)), "00:01");
    }
}
