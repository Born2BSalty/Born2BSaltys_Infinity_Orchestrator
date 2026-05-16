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
// `Btn small`). It emits `Step2Action::StartScan` — the **same** action
// BIO's `content_step2::render_controls` "Scan Mods Folder" button emits
// (`src/ui/step2/content_step2.rs:152`); the orchestrator's
// `step_action_dispatch::dispatch_step2` routes it through BIO's public
// `app_step2_router::handle_step2_action` → `app_step2_scan::
// start_step2_scan`. The enable predicate replicates `render_controls`'
// `can_scan_mods_folder` (`content_step2.rs:137-141`) — public-state reads
// only, no BIO logic reimplemented.
//
// The dev-only scan-folder affordance (P6.T2c — test enablement; absent in
// normal mode) sits to the right of Rescan, behind `dev_mode`. Pre-Phase-7
// there is no per-install extracted-mods folder to scan (SPEC §13.12a), so
// a developer needs to point the scan at an arbitrary folder to exercise
// the chrome.
//
// SPEC: §6 (Step 2 search + Scan), §13.12a (dev scan rationale).

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

/// Render the search row into `rect`. Returns `Some(Step2Action::StartScan)`
/// when the `Rescan Mods Folder` button is clicked (the router dispatches it
/// via `step_action_dispatch::dispatch_step2`). `dev_mode` gates the
/// (dev-only) scan-folder affordance — when `false` only the field + Rescan
/// are drawn (normal-mode behavior).
pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    rect: egui::Rect,
    dev_mode: bool,
) -> Option<Step2Action> {
    // Replicates `content_step2::render_controls`' `can_scan_mods_folder`
    // (`src/ui/step2/content_step2.rs:137-141`) — public-state reads only.
    let can_scan = if orchestrator.wizard_state.step1.bootstraps_from_weidu_logs() {
        !orchestrator.wizard_state.step2.is_scanning
    } else {
        !orchestrator.wizard_state.step2.is_scanning
            && orchestrator.wizard_state.step1_mods_folder_has_tp2 != Some(false)
    };

    let mut action: Option<Step2Action> = None;

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;

            // Right-hand controls are measured first so the search field can
            // claim the remaining (full) width — the wireframe `flex: 1`.
            let rescan_label = "Rescan Mods Folder";
            let rescan_w = small_btn_width(ui, rescan_label);
            let dev_label = "dev: scan a folder\u{2026}";
            let dev_w = if dev_mode {
                small_btn_width(ui, dev_label) + ROW_GAP
            } else {
                0.0
            };
            let search_w = (rect.width() - rescan_w - ROW_GAP - dev_w).max(80.0);

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

            // The wireframe `<TopButton>Rescan Mods Folder</TopButton>` —
            // redesign `redesign_btn` small (= `Btn small` = `TopButton`).
            if redesign_btn(
                ui,
                palette,
                rescan_label,
                BtnOpts {
                    small: true,
                    disabled: !can_scan,
                    ..Default::default()
                },
            )
            .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SCAN)
            .clicked()
                && can_scan
            {
                action = Some(Step2Action::StartScan);
            }

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
