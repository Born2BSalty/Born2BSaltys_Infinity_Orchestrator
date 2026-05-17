// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_step4` — the **Step-4 C4 orchestrator-side renderer** (P6.T2b).
// The direct analogue of `step2::workspace_step2` (P6.T2c): net-new redesign
// chrome that **does not** call BIO's `bio::ui::step4::page_step4::render`
// (per C4 — calling it would render a *second* `Save weidu.log's` button,
// since BIO's `content_step4` paints its own). The legacy `BIO` binary's
// `WizardApp::update_loop` keeps invoking `page_step4::render` normally; the
// orchestrator simply doesn't.
//
// ## Signature / dispatch
//
// `pub fn render(ui, orchestrator) -> Option<Step4Action>`. The wrapper
// **returns** any `Step4Action` produced by button clicks (Save / Check Mod
// List) to the router; the router dispatches uniformly via
// `step_action_dispatch::dispatch_step4` (M11 — **all dispatch happens at
// the router layer for consistency**). The Save **error** popup is the one
// render-side concern: after the router has dispatched a `SaveWeiduLog`,
// `bio::app::app_step4_flow::handle_step4_action` writes the failure into
// `wizard_state.step5.last_status_text` (the exact field BIO's own save-error
// popup reads), so this wrapper surfaces that string inline as a small
// non-blocking error notice — it never bubbles back up as an action.
//
// ## Layout (top-down, per SPEC §8.1)
//
//   1. Save action row (`step4_save_row`).
//   2. Game tab strip — **EET dual-game only**; single-game modlists skip
//      it (the brief: "EET dual only; single-game skips it" / SPEC §8.1 "Tab
//      row (game tabs only…)"). Net-new redesign `GameTab` painter — the
//      wireframe `GameTab` (`screens.jsx:1609-1637`) is the **same**
//      component the wireframe Step-2 tab row uses, so this matches the
//      wireframe Step-4 tab look while staying consistent with the sibling
//      `step2_tab_row`'s established redesign `game_tab` chassis (the
//      codebase convention is each module paints its own tab/glyph chassis —
//      `workspace_nav_bar`, `sub_flow_footer`, `step2_tab_row`). Writes
//      `wizard_state.step3.active_game_tab`.
//   3. Body — branches on install mode:
//        - normal modes → `step4_review_list` over the active tab's Step-3
//          items (the §6.7 three-colour line-numbered list).
//        - `install_exactly_from_weidu_logs` → `step4_exact_log_viewer`
//          (read-only source-log viewer + `Check Mod List`) — SPEC §8.2 /
//          Appendix A.7.
//
// SPEC: §8.1, §8.2, §6.7, Appendix A.7, §2.2, §5.1 (game immutable
//       per-workspace — drives single vs dual tabs), §1 (decision order).

// rationale: f32→u8 corner-radius / pixel roundings of small positive layout
// constants — correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::app::state::{Step3ItemState, WizardState};
use crate::app::step4_action::Step4Action;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_hover_overlay, redesign_pill_danger, redesign_pill_text,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::workspace::step4::{step4_exact_log_viewer, step4_review_list, step4_save_row};

/// Tab height (wireframe `GameTab` action sub-row `height: 30`).
const TAB_H: f32 = 30.0;
/// Gap between GameTabs (wireframe outer row `gap: 4`).
const TAB_GAP: f32 = 4.0;
/// The tab row overlaps the body Box's top edge by 1.5px so the active
/// tab's shell-bg fill masks the box's top border (wireframe `GameTab`
/// `marginBottom: -1.5px`, `screens.jsx:1630`).
const TAB_TO_BODY_OVERLAP: f32 = 1.5;
/// Gap below the Save row (wireframe `OrderPanel` action row
/// `marginBottom: 10`).
const SAVE_ROW_GAP: f32 = 10.0;

/// True if this modlist's game family uses **two** game tabs (BGEE + BG2EE).
/// Only EET is dual-game; BGEE / BG2EE / IWDEE are single (matches the
/// wireframe `tabsForGame` and BIO's Step-2/3/4 single-vs-dual logic).
pub fn is_dual_game(state: &WizardState) -> bool {
    state.step1.game_install == "EET"
}

/// The active tab's upper-case label + its Step-3 ordered items, picked by
/// the modlist's game + `step3.active_game_tab` exactly as BIO's Step-4
/// (`content_step4::render_order_list` / `active_step4_game_tab`):
///   - BG2EE              → `("BG2EE", bg2ee_items)`
///   - EET + active BG2EE → `("BG2EE", bg2ee_items)`
///   - everything else    → `("BGEE",  bgee_items)`  (incl. IWDEE — BIO
///     routes IWDEE through the BGEE bucket/tab)
pub fn active_tab_items(state: &WizardState) -> (&'static str, &[Step3ItemState]) {
    match state.step1.game_install.as_str() {
        "BG2EE" => ("BG2EE", &state.step3.bg2ee_items),
        "EET" if state.step3.active_game_tab == "BG2EE" => ("BG2EE", &state.step3.bg2ee_items),
        _ => ("BGEE", &state.step3.bgee_items),
    }
}

/// Render the Step-4 C4 chrome. Returns any `Step4Action` for the router.
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> Option<Step4Action> {
    let palette = orchestrator.theme_palette;
    let mut action: Option<Step4Action> = None;

    // For EET, keep `step3.active_game_tab` valid before any read (mirrors
    // BIO Step-2/4 normalisation: a freshly-loaded EET workspace whose
    // `active_game_tab` is unset / "BGEE" is fine; an out-of-set value would
    // pick the wrong bucket). Single-game modlists ignore the field.
    if is_dual_game(&orchestrator.wizard_state) {
        let t = &mut orchestrator.wizard_state.step3.active_game_tab;
        if t != "BGEE" && t != "BG2EE" {
            *t = "BGEE".to_string();
        }
    }

    // ── 1. Save action row (returns Some(SaveWeiduLog) on click). ──
    if let Some(a) = step4_save_row::render(ui, orchestrator, palette) {
        action = Some(a);
    }
    ui.add_space(SAVE_ROW_GAP);

    // ── 2. Game tab strip — EET dual-game only. ──
    if is_dual_game(&orchestrator.wizard_state) {
        render_game_tab_strip(
            ui,
            palette,
            &mut orchestrator.wizard_state.step3.active_game_tab,
        );
        // The body Box starts 1.5px ABOVE the tab row's bottom so the active
        // tab's shell-bg fill overlaps & masks the box's top border (the
        // wireframe "tab flows into the box" behaviour).
        ui.add_space(-TAB_TO_BODY_OVERLAP);
    }

    // ── 3. Body — branch on install mode. ──
    let exact_log_mode = orchestrator
        .wizard_state
        .step1
        .installs_exactly_from_weidu_logs();

    if exact_log_mode {
        // SPEC §8.2 / Appendix A.7 — read-only source-log viewer.
        if let Some(a) = step4_exact_log_viewer::render(ui, orchestrator, palette) {
            action = Some(a);
        }
    } else {
        // SPEC §8.1 — line-numbered three-colour review list over the
        // active tab's Step-3 order. Borrow the items immutably for the
        // render (no mutation of `wizard_state` here).
        let (active_tab, items) = active_tab_items(&orchestrator.wizard_state);
        // `items` borrows `orchestrator.wizard_state`; the renderer takes it
        // read-only so the borrow is sound.
        step4_review_list::render(ui, palette, items, active_tab);
    }

    // ── Render-side: the BIO save-error notice. After the router dispatches
    //    a `SaveWeiduLog`, `handle_step4_action` writes any failure into
    //    `wizard_state.step5.last_status_text` (the same field BIO's
    //    save-error popup reads). Surface it inline as a non-blocking notice
    //    (it never bubbles back as an action — M11). The success path writes
    //    "Saved weidu.log file(s)" to `step2.scan_status`, NOT this field, so
    //    a successful save shows nothing here. ──
    surface_save_error(ui, palette, orchestrator);

    action
}

/// Render the EET 2-tab strip (`BGEE` / `BG2EE`) as a redesign `GameTab`
/// row. `active` is `wizard_state.step3.active_game_tab` (written on click).
fn render_game_tab_strip(ui: &mut egui::Ui, palette: ThemePalette, active: &mut String) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TAB_GAP;
        game_tab(ui, palette, "BGEE", active);
        game_tab(ui, palette, "BG2EE", active);
    });
}

/// A wireframe `GameTab` (`screens.jsx:1609-1637`) — a tab (not a closed
/// button), rounded-top corners only, extending 1.5px past its bottom into
/// the seam above the body Box. Active = shell-bg fill + primary text + a
/// shell-bg bottom edge that masks the body Box's top border so the tab
/// "flows into" the box; idle = chrome-bg fill + muted text + border-strong
/// bottom. Same chassis the sibling `step2_tab_row::game_tab` paints (the
/// established redesign rendering of this shared wireframe component).
fn game_tab(ui: &mut egui::Ui, palette: ThemePalette, label: &str, current: &mut String) {
    let active = current == label;
    let font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        font.clone(),
        redesign_text_primary(palette),
    );
    // Wireframe `padding: 5px 14px` horizontal.
    let tab_w = galley.size().x + 14.0 * 2.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(tab_w, TAB_H), egui::Sense::click());
    // Rounded TOP corners only (wireframe `borderRadius: "4px 4px 0 0"`).
    let corner = egui::CornerRadius {
        nw: REDESIGN_BORDER_RADIUS_PX as u8,
        ne: REDESIGN_BORDER_RADIUS_PX as u8,
        sw: 0,
        se: 0,
    };
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let fill = if active {
            redesign_shell_bg(palette)
        } else {
            redesign_chrome_bg(palette)
        };
        // The tab box extends 1.5px past its bottom into the seam above the
        // body Box (wireframe `marginBottom: -1.5px`). For the ACTIVE tab
        // the shell-bg fill + shell-bg bottom edge (below) overlap & mask
        // the box's top border. For an IDLE tab the box's bottom edge lands
        // on the body Box's own top border (single line, no double rule).
        let box_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.max.x, rect.max.y + REDESIGN_BORDER_WIDTH_PX),
        );
        painter.rect_filled(box_rect, corner, fill);
        if !active && response.hovered() {
            painter.rect_filled(box_rect, corner, redesign_hover_overlay(palette));
        }
        let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
        painter.rect_stroke(box_rect, corner, stroke, egui::StrokeKind::Inside);
        if active {
            // Wireframe `borderBottom: active ? shell-bg : border-strong`
            // (`screens.jsx:1625`): over-paint the bottom edge in shell-bg
            // so — with the 1.5px fill overlap — it fully masks the body
            // Box's top border in the tab's x-range; the tab merges into
            // the box.
            let half = REDESIGN_BORDER_WIDTH_PX / 2.0;
            painter.line_segment(
                [
                    egui::pos2(box_rect.left(), box_rect.bottom() - half),
                    egui::pos2(box_rect.right(), box_rect.bottom() - half),
                ],
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, fill),
            );
        }
        let text_color = if active {
            redesign_text_primary(palette)
        } else {
            redesign_text_muted(palette)
        };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }
    if response.clicked() {
        *current = label.to_string();
    }
}

/// Inline, non-blocking save-error notice. After a `SaveWeiduLog` dispatch,
/// `auto_save_step4_weidu_logs` writes `Save weidu.log failed: …` into
/// `wizard_state.step5.last_status_text` on failure (the exact string BIO's
/// own Step-4 save-error popup shows). Surface it as a small pill-toned
/// notice above the nav bar. Cleared automatically the moment a later
/// successful save (which does **not** touch this field) makes it stale —
/// to avoid a stuck stale error, only show it while the most recent
/// scan-status is NOT the success string.
fn surface_save_error(ui: &mut egui::Ui, palette: ThemePalette, orchestrator: &OrchestratorApp) {
    let last = orchestrator.wizard_state.step5.last_status_text.trim();
    if !last.starts_with("Save weidu.log failed") {
        return;
    }
    // If the most recent scan-status is the success marker, a later save
    // succeeded — don't show the stale failure.
    if orchestrator
        .wizard_state
        .step2
        .scan_status
        .starts_with("Saved weidu.log file(s)")
    {
        return;
    }
    ui.add_space(6.0);
    let pad_x = 8.0;
    let pad_y = 4.0;
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let text_color = redesign_pill_text(palette);
    let galley = ui
        .painter()
        .layout_no_wrap(last.to_string(), font.clone(), text_color);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0),
        egui::Sense::hover(),
    );
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
            redesign_pill_danger(palette),
        );
        painter.text(
            egui::pos2(rect.left() + pad_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            last,
            font,
            text_color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(tp: &str, id: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: format!("c{id}"),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }
    }

    /// Only EET is dual-game (SPEC §5.1 / wireframe `tabsForGame`).
    #[test]
    fn dual_game_is_eet_only() {
        let mut s = WizardState::default();
        for (g, dual) in [
            ("EET", true),
            ("BGEE", false),
            ("BG2EE", false),
            ("IWDEE", false),
        ] {
            s.step1.game_install = g.to_string();
            assert_eq!(is_dual_game(&s), dual, "game {g}");
        }
    }

    /// Active-tab selection mirrors BIO's Step-4 `active_step4_game_tab`:
    /// BG2EE → bg2ee items; EET+BG2EE → bg2ee; else → bgee (incl. IWDEE).
    #[test]
    fn active_tab_items_match_bio_resolution() {
        let mut s = WizardState::default();
        s.step3.bgee_items = vec![leaf("A.TP2", "0")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0"), leaf("B.TP2", "1")];

        s.step1.game_install = "BGEE".to_string();
        let (t, it) = active_tab_items(&s);
        assert_eq!(t, "BGEE");
        assert_eq!(it.len(), 1);

        s.step1.game_install = "BG2EE".to_string();
        let (t, it) = active_tab_items(&s);
        assert_eq!(t, "BG2EE");
        assert_eq!(it.len(), 2);

        s.step1.game_install = "EET".to_string();
        s.step3.active_game_tab = "BG2EE".to_string();
        let (t, it) = active_tab_items(&s);
        assert_eq!(t, "BG2EE");
        assert_eq!(it.len(), 2);
        s.step3.active_game_tab = "BGEE".to_string();
        let (t, it) = active_tab_items(&s);
        assert_eq!(t, "BGEE");
        assert_eq!(it.len(), 1);

        // IWDEE routes through the BGEE bucket/tab.
        s.step1.game_install = "IWDEE".to_string();
        let (t, _it) = active_tab_items(&s);
        assert_eq!(t, "BGEE");
    }
}
