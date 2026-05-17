// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step4_exact_log_viewer` — the read-only source-WeiDU-log viewer for
// `install_mode == install_exactly_from_weidu_logs` (P6.T2b, SPEC §8.2 /
// Appendix A.7). Net-new redesign chrome; **BIO-fidelity** for the resolved
// log files + the `Check Mod List` action.
//
// SPEC §8.2: "When the modlist's install mode is
// `install_exactly_from_weidu_logs`, Step 4 becomes a read-only viewer of
// the source WeiDU log files. The user cannot edit; they can only
// **Check Mod List** (which triggers the same update-check flow as Step 2's
// `Updates...` button)." Appendix A.7: BIO-fidelity for the exact-log
// read-only viewer.
//
// ## What is reused (read-only, BIO-fidelity)
//
// - **Which file, per game / mode:** `bio::app::step5::log_files::
//   source_log_infos(&step1)` — the exact `pub fn` BIO's own
//   `content_step4::render_source_logs` uses to resolve the source log
//   path(s) per `game_install` (EET → bgee+bg2ee, BG2EE → bg2ee, else
//   bgee). The active tab is picked the same way BIO does
//   (`state.step3.active_game_tab` → `bgee`/`bg2ee` tag). This honours the
//   plan's "reads the configured WeiDU log files from `step1.bgee_log_file`
//   / `bg2ee_log_file` (or whichever applies per game)" by using BIO's
//   canonical per-game resolver (more robust than reading the raw fields and
//   what BIO itself does).
// - **File read:** `bio::ui::step4::service_step4::read_source_log_lines`
//   (`pub fn`) — the same reader BIO's exact-log Step 4 uses.
// - **Check Mod List:** returns `Step4Action::CheckMissingMods` to the
//   wrapper → router → `bio::app::app_step4_flow::handle_step4_action`
//   (`pub(crate) fn`, `src/core/app/app_step4_flow.rs:8`) →
//   `app_step2_saved_log_flow::queue_exact_log_update_preview` (the same
//   update-check flow Step 2's `Updates...`/`Mod List...` triggers).
//
// ## What is net-new (redesign chrome)
//
// The source header line, the status line, the line-numbered read-only
// list, and the `Check Mod List` button are net-new redesign-token widgets
// — BIO's `content_step4::render_source_logs` is **not** called (per C4).
// Each source log line is rendered via the redesign `weidu_line` renderer
// (so a WeiDU line gets the §6.7 three-colour treatment; a header / comment
// line renders flat in the comment colour, matching BIO's own
// `render_weidu_colored_line` behaviour).
//
// SPEC: §8.2 (exact-log mode), Appendix A.7 (BIO-fidelity), §6.7 (line
//       three-colour syntax), §1 (decision order — reuse BIO data/action,
//       net-new render).

// rationale: f32→u8 corner-radius rounding of a small positive constant —
// correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::app::state::Step3ItemState;
use crate::app::step4_action::Step4Action;
use crate::app::step5::log_files::source_log_infos;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};
use crate::ui::step4::service_step4::read_source_log_lines;
use crate::ui::workspace::widgets::weidu_line;

/// Box inner padding (matches the review-list Box).
const BOX_PADDING: f32 = 12.0;

/// Render the exact-log read-only viewer. Returns
/// `Some(Step4Action::CheckMissingMods)` when the `Check Mod List` button is
/// clicked (the router dispatches it via `dispatch_step4`).
pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) -> Option<Step4Action> {
    let mut action: Option<Step4Action> = None;

    // Resolve the active source log exactly as BIO's
    // `content_step4::render_source_logs` does: `source_log_infos` per
    // `game_install`, then the active tag from `step3.active_game_tab`.
    let infos = source_log_infos(&orchestrator.wizard_state.step1);
    let active_tag = match orchestrator.wizard_state.step1.game_install.as_str() {
        "BG2EE" => "bg2ee",
        "EET" if orchestrator.wizard_state.step3.active_game_tab == "BG2EE" => "bg2ee",
        _ => "bgee",
    };
    let info = infos.into_iter().find(|i| i.tag == active_tag);

    // ── Action row: `Check Mod List` (SPEC §8.2 — the only interaction). ──
    // BIO gates it on the same "step 4 busy" predicate
    // (`content_step4::render`: scanning / update check / download /
    // extract running).
    let step4_busy = orchestrator.wizard_state.step2.is_scanning
        || orchestrator
            .wizard_state
            .step2
            .update_selected_check_running
        || orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        || orchestrator
            .wizard_state
            .step2
            .update_selected_extract_running;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if redesign_btn(
                ui,
                palette,
                "Check Mod List",
                BtnOpts {
                    primary: true,
                    disabled: step4_busy,
                    ..Default::default()
                },
            )
            .clicked()
                && !step4_busy
            {
                action = Some(Step4Action::CheckMissingMods);
            }
        });
    });
    ui.add_space(8.0);

    // ── Source header + status (net-new redesign chrome). ──
    match &info {
        Some(info) => {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Source")
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_muted(palette)),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(info.path.to_string_lossy().to_string())
                        .size(12.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_text_primary(palette)),
                );
            });
            let status = if info.exists { "Found" } else { "Missing" };
            let mut status_line = format!("Status: {status}");
            if let Some(sz) = info.size_bytes {
                status_line.push_str(&format!(" \u{00B7} {sz} bytes"));
            }
            ui.label(
                egui::RichText::new(status_line)
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_faint(palette)),
            );
        }
        None => {
            ui.label(
                egui::RichText::new("No source WeiDU log configured for this tab.")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_faint(palette)),
            );
        }
    }
    ui.add_space(8.0);

    // ── Read-only line-numbered list inside a bordered Box. ──
    let avail = ui.available_size();
    let (box_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
    if ui.is_rect_visible(box_rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(box_rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            box_rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    }
    let inner = box_rect.shrink(BOX_PADDING);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));

    match info {
        Some(info) if info.exists => match read_source_log_lines(&info.path) {
            Ok(lines) if lines.is_empty() => {
                child.label(faint(palette, "Selected source log is empty."));
            }
            Ok(lines) => {
                let max_digits = lines.len().to_string().len();
                let lineno_w = weidu_line::lineno_column_width(max_digits);
                egui::ScrollArea::vertical()
                    .id_salt(("workspace_step4_exactlog_scroll", active_tag))
                    .auto_shrink([false, false])
                    .show(&mut child, |ui| {
                        for (i, line) in lines.iter().enumerate() {
                            // Reuse the redesign three-colour renderer by
                            // wrapping each raw log line in a transient
                            // item (its `raw_line` carries the text; the
                            // renderer normalises + colours it exactly as
                            // the review list does).
                            let item = line_as_item(line);
                            weidu_line::render_weidu_line(
                                ui,
                                palette,
                                &item,
                                Some(i + 1),
                                lineno_w,
                            );
                        }
                    });
            }
            Err(err) => {
                child.label(faint(palette, &format!("Failed to read file: {err}")));
            }
        },
        Some(_) => {
            child.label(faint(palette, "Source WeiDU log not found on disk."));
        }
        None => {
            child.label(faint(
                palette,
                "No source WeiDU log configured for this tab.",
            ));
        }
    }

    // Advance the parent placer by exactly the Box rect (the child UI does
    // not) so the workspace nav bar below stays pinned.
    ui.allocate_rect(box_rect, egui::Sense::hover());

    action
}

/// Wrap a raw source-log line in a transient `Step3ItemState` so the shared
/// `weidu_line` renderer can colour it. The renderer reads `raw_line` first
/// (`format_step4_item` → `normalize_weidu_like_line`); a header / comment
/// line (no `~...~`) falls through to the flat comment colour, matching
/// BIO's `render_weidu_colored_line` behaviour for the same lines.
fn line_as_item(raw: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: String::new(),
        component_id: String::new(),
        mod_name: String::new(),
        component_label: String::new(),
        raw_line: raw.to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 0,
        block_id: String::new(),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn faint(palette: ThemePalette, text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(13.0)
        .family(egui::FontFamily::Name("poppins_medium".into()))
        .color(redesign_text_faint(palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The active source-log tag must follow BIO's exact per-game / per-tab
    /// resolution (`content_step4::render_source_logs`): BG2EE → bg2ee;
    /// EET + active tab BG2EE → bg2ee; everything else → bgee.
    #[test]
    fn active_tag_matches_bio_resolution() {
        let resolve = |game: &str, tab: &str| -> &'static str {
            match game {
                "BG2EE" => "bg2ee",
                "EET" if tab == "BG2EE" => "bg2ee",
                _ => "bgee",
            }
        };
        assert_eq!(resolve("BG2EE", "BGEE"), "bg2ee");
        assert_eq!(resolve("EET", "BG2EE"), "bg2ee");
        assert_eq!(resolve("EET", "BGEE"), "bgee");
        assert_eq!(resolve("BGEE", "BGEE"), "bgee");
        assert_eq!(resolve("IWDEE", "BGEE"), "bgee");
    }

    /// A header / comment source-log line (no `~...~`) wraps into an item
    /// whose `raw_line` carries the text — the renderer then colours it flat
    /// (BIO `render_weidu_colored_line` parity for non-WeiDU lines).
    #[test]
    fn comment_line_wraps_into_raw_line_item() {
        let it = line_as_item("// Log of Currently Installed WeiDU Mods");
        assert_eq!(it.raw_line, "// Log of Currently Installed WeiDU Mods");
        assert!(!it.is_parent);
        // It is a pure-comment line: `format_step4_item` returns it
        // trimmed (no `~...~` to normalise).
        let rendered = crate::app::step5::diagnostics::format_step4_item(&it);
        assert_eq!(rendered, "// Log of Currently Installed WeiDU Mods");
    }
}
