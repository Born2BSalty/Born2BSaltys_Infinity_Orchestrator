// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `success_banner` — the post-install success banner row, rendered ABOVE
// BIO's embedded Step-5 panel (per H9, in the chrome row immediately above
// the panel; visually it replaces the empty pre-install banner slot).
//
// **Visibility gate — the C3 clean-exit triple.** Per the Phase-7 plan
// (the "Clean-exit definition (per C3)" section) the banner is visible
// **only** when:
//
//   state.step5.install_running == false
//       && state.step5.last_exit_code == Some(0)
//       && state.step5.last_install_failed == false
//
// (Each field is `pub` per BIO `state_step5.rs:17-19`; the triple
// supersedes every `errors_detected` reference — BIO's `Step5State` has no
// such field.) Pre-install and during-install the triple is false ⇒ the
// row is **empty** (the embedded panel below shows BIO's pre-install
// Command / Summary cards). When it holds, the banner is the wireframe
// `FinalPlanPanel` `installComplete` row (`screens.jsx:3231-3235`):
//
//   Box(borderColor: var(--success), padding: 10px 14px) {
//     Pill(background: var(--success), color: #0B1116) "Installed"
//     Label "<N> mods · <C> components · no errors"
//     Label(hand, marginLeft: auto, color: var(--text-faint))
//       "ran <MM:SS> · finished <relative>"
//   }
//
// **Counts source (premise-checked).** `<N> mods` / `<C> components` are
// read off the registry `ModlistEntry.mod_count` / `component_count` — the
// values `registry_transition::flip_to_installed` (P7.T6) wrote from
// `count_mods_and_components`, which mirrors BIO/the redesign's OWN Step-4
// count resolver (`step4_save_row::active_tab_counts`: installable leaves =
// non-parent `Step3ItemState`s, mods = distinct `tp_file`s). So the banner
// counts can never drift from what Step 4 displayed (NOT an invented
// count).
//
// **Duration source.** `ran <MM:SS>` = `format_install_duration(install_date
// − install_started_at)` (both wall-clock `DateTime<Utc>` on the entry —
// `install_started_at` stamped by `start_hooks::on_install_start`,
// `install_date` by `flip_to_installed`). `finished <relative>` =
// `relative_time(install_date)`. Both helpers live in
// `src/ui/shared/format_relative.rs` (L9 — `format_install_duration` added
// there this run, NOT duplicated from Run-2's `shell_statusbar
// ::format_elapsed`).
//
// SPEC: §9.2.

use eframe::egui;

use crate::app::state::WizardState;
use crate::registry::model::ModlistEntry;
use crate::ui::shared::format_relative::{format_install_duration, relative_time};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_success,
    redesign_text_faint, redesign_text_primary,
};

/// The C3 clean-exit triple (the Phase-7 plan's canonical success gate).
/// `true` iff an install has completed cleanly: not running, last exit
/// code 0, and BIO did not flag a likely failure on exit
/// (`step5_runtime_status::process_exit_event` sets `last_install_failed`
/// from `term.likely_failure_visible()` and `last_exit_code` on every
/// exit). The success banner, the post-install action row, and the P7.T6
/// registry transition all gate on this exact predicate — defined once
/// here so they can never drift.
#[must_use]
pub fn clean_exit(state: &WizardState) -> bool {
    !state.step5.install_running
        && state.step5.last_exit_code == Some(0)
        && !state.step5.last_install_failed
}

/// Render the success-banner row.
///
/// Returns immediately (renders nothing) unless the C3 triple holds — the
/// empty pre-install / during-install / failed-install slot (the embedded
/// `page_step5::render` panel below shows BIO's pre-install Command/Summary
/// cards or the live console). When `clean_exit` holds, paints the
/// wireframe `installComplete` banner (success-bordered Box: green
/// `Installed` pill + counts + right-aligned faint `ran … · finished …`).
///
/// `entry` is the routed modlist's registry entry — the source of the
/// counts (P7.T6-written, mirrors Step 4) and the duration timestamps
/// (`install_started_at` / `install_date`).
pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &WizardState, entry: &ModlistEntry) {
    if !clean_exit(state) {
        // Pre-install / during-install / failed install ⇒ empty slot. The
        // embedded `page_step5::render` panel below shows BIO's pre-install
        // Command/Summary cards (or the live console).
        return;
    }

    // ── Wireframe `installComplete` banner Box (screens.jsx:3231-3235):
    //    sketchy border in `var(--success)`, padding 10px 14px, single
    //    flex row gap 12, marginBottom 10. ──
    let pad_x = 14.0;
    let pad_y = 10.0;
    let gap = 12.0;
    let margin_bottom = 10.0;

    let success = redesign_success(palette);

    egui::Frame::default()
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, success))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin {
            left: pad_x as i8,
            right: pad_x as i8,
            top: pad_y as i8,
            bottom: pad_y as i8,
        })
        .show(ui, |ui| {
            // Full-width single row: [Installed pill] [counts] … [ran · finished]
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 0.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = gap;

                    // ── Green `Installed` pill. Wireframe overrides the
                    //    generic Pill's fill to `var(--success)` with the
                    //    fixed dark `#0B1116` text (theme-invariant — high
                    //    contrast on the green). The label `Installed` is
                    //    plain ASCII (renders in poppins; no symbol-glyph
                    //    concern). Painted inline (not the generic
                    //    `pill::render`, whose tones are danger/warn/info/
                    //    neutral — none is "success"); same chassis. ──
                    installed_pill(ui, success);

                    // ── `<N> mods · <C> components · no errors`. ──
                    ui.label(
                        egui::RichText::new(format!(
                            "{} mods \u{00B7} {} components \u{00B7} no errors",
                            entry.mod_count, entry.component_count
                        ))
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_primary(palette)),
                    );

                    // ── Right-aligned (marginLeft: auto) faint hand
                    //    `ran <MM:SS> · finished <relative>`. ──
                    let ran_finished = duration_line(entry);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(ran_finished)
                                .size(13.0)
                                .family(egui::FontFamily::Name("firacode_nerd".into()))
                                .color(redesign_text_faint(palette)),
                        );
                    });
                },
            );
        });

    ui.add_space(margin_bottom);
}

/// The wireframe's success-tinted `Installed` pill (`screens.jsx:3232` —
/// `Pill` with `background: var(--success)`, `color: #0B1116`). The generic
/// `widgets::pill` has only danger/warn/info/neutral tones, so paint the
/// same compact rounded chip here with the success fill + the fixed dark
/// foreground (theme-invariant — matches the wireframe's hard-coded
/// `#0B1116`).
fn installed_pill(ui: &mut egui::Ui, success: egui::Color32) {
    let pad_x = 8.0;
    let pad_y = 2.0;
    // Wireframe hard-codes the pill text to `#0B1116` (the same dark the
    // theme uses for `--bg`); fixed, high-contrast on the green fill.
    let text_color = egui::Color32::from_rgb(0x0B, 0x11, 0x16);
    let font = egui::FontId::new(11.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap("Installed".to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
            success,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Installed",
            font,
            text_color,
        );
    }
}

/// `ran <MM:SS> · finished <relative>` (wireframe `screens.jsx:3234`).
///
/// `ran` = `install_date − install_started_at` formatted via
/// `format_install_duration` (`<MM:SS>` < 60 min else `<H:MM:SS>`).
/// `finished` = `relative_time(install_date)` (the same humanizer the Home
/// cards use). Both timestamps are wall-clock `DateTime<Utc>` on the entry.
/// If either timestamp is missing (an older entry, or a clock-skew where
/// `install_date < install_started_at`) the duration falls back to `—:—`
/// for `ran` (honest "unknown" — never a negative clock) while still
/// showing `finished`.
fn duration_line(entry: &ModlistEntry) -> String {
    let ran = match (entry.install_started_at, entry.install_date) {
        (Some(start), Some(end)) => {
            let secs = end.signed_duration_since(start).num_seconds();
            if secs < 0 {
                // Clock skew / missing — never render a negative run.
                "\u{2014}:\u{2014}".to_string()
            } else {
                format_install_duration(std::time::Duration::from_secs(secs as u64))
            }
        }
        // `install_started_at` absent (pre-P7.T3 entry) — honest unknown.
        _ => "\u{2014}:\u{2014}".to_string(),
    };
    let finished = entry
        .install_date
        .map_or_else(|| "just now".to_string(), relative_time);
    format!("ran {ran} \u{00B7} finished {finished}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry};
    use chrono::{Duration as ChronoDuration, Utc};

    #[test]
    fn fresh_state_is_not_clean_exit() {
        // A fresh `Step5State` has `install_running == false` but
        // `last_exit_code == None` — the C3 triple must be false (the
        // empty-banner-pre-install property).
        let s = WizardState::default();
        assert!(!clean_exit(&s), "no install has run ⇒ the banner is hidden");
    }

    #[test]
    fn running_install_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = true;
        s.step5.last_exit_code = Some(0);
        assert!(
            !clean_exit(&s),
            "an install still running is not a clean exit"
        );
    }

    #[test]
    fn nonzero_exit_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(1);
        assert!(!clean_exit(&s), "a nonzero exit code is not clean");
    }

    #[test]
    fn flagged_failure_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(0);
        s.step5.last_install_failed = true;
        assert!(
            !clean_exit(&s),
            "a BIO-flagged likely-failure is not a clean exit even at \
             exit code 0"
        );
    }

    #[test]
    fn clean_triple_holds() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(0);
        s.step5.last_install_failed = false;
        assert!(
            clean_exit(&s),
            "not running + exit 0 + not flagged ⇒ clean exit (the C3 \
             triple the banner/post-install/registry-flip gate on)"
        );
    }

    #[test]
    fn duration_line_formats_mm_ss_and_relative() {
        let start = Utc::now() - ChronoDuration::seconds(4 * 60 + 12);
        let end = start + ChronoDuration::seconds(4 * 60 + 12);
        let entry = ModlistEntry {
            id: "B".to_string(),
            name: "n".to_string(),
            game: Game::EET,
            install_started_at: Some(start),
            install_date: Some(end),
            ..Default::default()
        };
        let line = duration_line(&entry);
        assert!(
            line.starts_with("ran 4:12 \u{00B7} finished "),
            "expected `ran 4:12 · finished …`, got: {line}"
        );
    }

    #[test]
    fn duration_line_handles_missing_start_and_skew() {
        // No `install_started_at` (older entry) ⇒ `ran —:—`, still shows
        // `finished`.
        let end = Utc::now();
        let entry = ModlistEntry {
            install_started_at: None,
            install_date: Some(end),
            ..Default::default()
        };
        let line = duration_line(&entry);
        assert!(line.starts_with("ran \u{2014}:\u{2014} \u{00B7} finished "));

        // Clock skew: end < start ⇒ never a negative run.
        let now = Utc::now();
        let skew = ModlistEntry {
            install_started_at: Some(now),
            install_date: Some(now - ChronoDuration::seconds(30)),
            ..Default::default()
        };
        assert!(duration_line(&skew).starts_with("ran \u{2014}:\u{2014}"));
    }
}
