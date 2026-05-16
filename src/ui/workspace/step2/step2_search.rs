// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step2_search` — the net-new Step-2 search row (P6.T2c).
//
// Mirrors the wireframe `screens.jsx` Step 2 search row (L2798-2802):
//   <div style={{ display:"flex", gap:10, alignItems:"center", marginBottom:10 }}>
//     <Input wide value="Search mods or components..." style={{ flex:1, ... }} />
//     <TopButton>Rescan Mods Folder</TopButton>
//   </div>
//
// The search input is **full-width** (`flex: 1`) — the structural fix the
// SPEC-CONFLICT resolution called out (BIO's `content_step2::render_header`
// caps the field at `STEP2_SEARCH_MAX_W`; the wireframe stretches it). It
// writes the **same** `state.step2.search_query` field BIO's
// `render_header` search writes (`src/ui/step2/content_step2.rs:105`), so
// BIO's reused tree filters identically with zero behavior drift.
//
// The wireframe's `<TopButton>Rescan Mods Folder</TopButton>` is rendered
// here as a redesign `redesign_btn` (small = the wireframe `TopButton` =
// `Btn small`).
//
// ## Production Rescan is INERT pre-Phase-7 (the #2 fix; SPEC §13.12a)
//
// BIO's scan worker reads `step1.mods_folder` (`bio::app::step2::scan::
// worker.rs:75`). The redesign has **no global mods folder** — Settings →
// Paths supplies game-source / Mods-*archive* / backup / tool paths only.
// The real scan target is the **per-install extracted-mods folder**
// produced at prep time by the Phase-7 P7.T17 pipeline (SPEC §13.12a).
// Pre-Phase-7 there is no valid target, so a production `StartScan` would
// fall through to whatever stale `mods_folder` Settings holds (a temp dir)
// — the observed defect. So the production **"Rescan Mods Folder"** button
// is **disabled** pre-Phase-7 with an explanatory tooltip. This is the
// SAME accepted §13.12a Phase-7 deferral pattern as the §4.3 Downloading
// chassis — a deliberate "lights up in Phase 7" stub, not a path-mapping
// hack. The **dev scan affordance is the functional scan path now.**
//
// ## Cancel Scan (the #7 recorded addition)
//
// The wireframe has no Cancel. While `state.step2.is_scanning`, the Rescan
// button is replaced **in place** by **"Cancel Scan"** (enabled), emitting
// `Step2Action::CancelScan` — already handled by `dispatch_step2` →
// `app_step2_router::handle_step2_action`. A necessary capability (a scan
// can be long / wrong-target); recorded as an intentional wireframe
// omission addition.
//
// The dev-only scan-folder affordance (P6.T2c — test enablement; absent in
// normal mode) sits to the right, behind `dev_mode`. It points BIO's scan
// at a developer-chosen folder regardless of the (absent) production
// target, so it is **always enabled in dev mode** (the noted-minor fix —
// it must not inherit a production `can_scan` gate).
//
// SPEC: §6 (Step 2 search + Scan), §13.12a (production Rescan deferral +
//       dev scan rationale), wireframe omission (Cancel Scan, recorded).

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_text_faint, redesign_text_primary,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step2::step2_dev_scan;

/// Height of the search input — wireframe `Input` row height (matches the
/// chrome's `SEARCH_INPUT_H` so the field aligns with the Rescan button).
const SEARCH_INPUT_H: f32 = 30.0;
/// Gap between row items (wireframe search row `gap: 10`).
const ROW_GAP: f32 = 10.0;

/// Tooltip on the **disabled** production Rescan button (the #2 fix). States
/// the §13.12a Phase-7 deferral plainly and points the developer at the dev
/// scan (the functional path pre-Phase-7).
const RESCAN_DISABLED_TIP: &str = "Available after install prep (Phase 7) \u{2014} \
     the mods folder is extracted per-install at prep time (SPEC \u{00A7}13.12a). \
     Use the dev scan in dev mode to exercise scanning before then.";

/// Render the search row into `rect`. Returns `Some(Step2Action::CancelScan)`
/// when a scan is running and `Cancel Scan` is clicked (the #7 addition);
/// the production `Rescan Mods Folder` button is inert pre-Phase-7 (the #2
/// fix) so it never emits `StartScan`. `dev_mode` gates the (dev-only,
/// always-enabled) scan-folder affordance — when `false` only the field +
/// Rescan/Cancel are drawn (normal-mode behavior).
pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    rect: egui::Rect,
    dev_mode: bool,
) -> Option<Step2Action> {
    let is_scanning = orchestrator.wizard_state.step2.is_scanning;

    let mut action: Option<Step2Action> = None;

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;

            // Right-hand controls are measured first so the search field can
            // claim the remaining (full) width — the wireframe `flex: 1`.
            // While scanning the in-place button is "Cancel Scan" (#7);
            // otherwise the (disabled pre-Phase-7) "Rescan Mods Folder" (#2).
            let scan_btn_label = if is_scanning {
                "Cancel Scan"
            } else {
                "Rescan Mods Folder"
            };
            let scan_btn_w = small_btn_width(ui, scan_btn_label);
            let dev_label = "dev: scan a folder\u{2026}";
            let dev_w = if dev_mode {
                small_btn_width(ui, dev_label) + ROW_GAP
            } else {
                0.0
            };
            let search_w = (rect.width() - scan_btn_w - ROW_GAP - dev_w).max(80.0);

            let resp = ui.add_sized(
                [search_w, SEARCH_INPUT_H],
                egui::TextEdit::singleline(&mut orchestrator.wizard_state.step2.search_query)
                    .hint_text("Search mods or components...")
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .font(egui::FontId::new(
                        14.0,
                        egui::FontFamily::Name("poppins_medium".into()),
                    )),
            );
            // Sketchy 1.5px border around the input to match the redesign
            // `Input` chassis (egui's default frame is the wrong language).
            ui.painter().rect_stroke(
                resp.rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                egui::StrokeKind::Inside,
            );

            if is_scanning {
                // #7: "Cancel Scan" replaces Rescan in place while scanning.
                // Enabled; emits `CancelScan` (handled by `dispatch_step2` →
                // BIO's `app_step2_router::handle_step2_action`).
                if redesign_btn(
                    ui,
                    palette,
                    scan_btn_label,
                    BtnOpts {
                        small: true,
                        ..Default::default()
                    },
                )
                .on_hover_text("Stop the running scan and return to idle.")
                .clicked()
                {
                    action = Some(Step2Action::CancelScan);
                }
            } else {
                // #2: production "Rescan Mods Folder" is INERT pre-Phase-7
                // (no valid per-install extracted-mods target yet — SPEC
                // §13.12a). Disabled + explanatory tooltip; emits nothing.
                redesign_btn(
                    ui,
                    palette,
                    scan_btn_label,
                    BtnOpts {
                        small: true,
                        disabled: true,
                        ..Default::default()
                    },
                )
                .on_hover_text(RESCAN_DISABLED_TIP);
            }

            // Dev-only scan affordance — ALWAYS enabled in dev mode (the
            // noted-minor fix: it points BIO's scan at a chosen folder
            // regardless of the absent production target, so it must not
            // inherit any production `can_scan` gate).
            if dev_mode && dev_scan_button(ui, palette, dev_label).clicked() {
                step2_dev_scan::pick_folder_and_scan(orchestrator);
            }
        });
    });

    action
}

/// Measure a `redesign_btn` small-variant's width for the given label so the
/// search field can be sized to claim the remaining row width.
/// (`redesign_btn` small = 12px Poppins + 10px horizontal padding each side.)
fn small_btn_width(ui: &egui::Ui, label: &str) -> f32 {
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, egui::Color32::WHITE);
    galley.size().x + 10.0 * 2.0
}

/// The dev-only "scan a folder" button. Visually distinct (faint label) so
/// it reads as a dev affordance, not a primary control. Returns its
/// `Response` so the caller can react to `.clicked()`.
fn dev_scan_button(ui: &mut egui::Ui, palette: ThemePalette, label: &str) -> egui::Response {
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        font.clone(),
        redesign_text_faint(palette),
    );
    let pad_x = 10.0;
    let pad_y = 4.0;
    let size = egui::vec2(galley.size().x + pad_x * 2.0, SEARCH_INPUT_H);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(rect, radius, redesign_input_bg(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let _ = pad_y;
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            redesign_text_faint(palette),
        );
    }
    response.on_hover_text(
        "Dev only — pick an existing folder and run BIO's scan against it. \
         Pre-Phase-7 there is no per-install extracted-mods folder to scan \
         (SPEC §13.12a).",
    )
}
