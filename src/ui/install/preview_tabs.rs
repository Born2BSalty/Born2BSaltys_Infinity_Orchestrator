// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Install Modlist — the preview Content Box's 6-tab file-folder strip +
// per-tab content (SPEC §4.2, P5.T11).
//
// Mirrors `wireframe-preview/screens.jsx::ImportPreviewTabs` (line 469-510)
// for the strip and `::PreviewText` (line 512-529) for the per-tab body.
//
// **Tab strip** (`ImportPreviewTabs`, `merge` variant — the preview always
// uses `merge` so the active tab dissolves into the box below):
//   each tab: padding 6px 14px; 1.5px solid border-strong;
//     borderRadius 4px 4px 0 0; marginBottom -1.5px (overlap the box edge);
//     active  → background shell-bg, color text,        fontWeight 700,
//               borderBottom 1.5px solid shell-bg  (merge into the box)
//     inactive→ background chrome-bg, color text-muted, fontWeight 400,
//               borderBottom 1.5px solid border-strong
//   Poppins 14px; the strip itself has `borderBottom: 1.5px solid
//   border-strong` so the inactive tabs sit on a continuous baseline and
//   the active tab punches a shell-bg gap in it (the file-folder look).
//
// Because egui has no z-index/negative-margin model, the "active tab merges
// into the box" effect is reproduced by painting the strip's baseline rule
// and then over-painting a shell-bg segment exactly under the active tab
// (the same technique BIO's other file-folder strips will use; Settings'
// tab strip is the sibling precedent for this pattern in the redesign).
//
// **Tab content** (`PreviewText`): a single monospace pre-wrapped block,
// `FiraCode Nerd` 13px, lineHeight 1.35, color text. Each tab's text is
// built from the parsed `ModlistSharePreview` (NOT mock data):
//   - Summary        — composed recap (wireframe `PreviewText` "Summary"
//                       structure, populated from the real preview).
//   - BGEE WeiDU      — `preview.bgee_log_text` verbatim.
//   - BG2EE WeiDU     — `preview.bg2ee_log_text` verbatim.
//   - User Downloads  — `preview.source_overrides_text` verbatim.
//   - Installed Refs  — `preview.installed_refs_text` verbatim.
//   - Mod Configs     — `preview.mod_configs_text` verbatim.
// When a section is empty the body shows a faint "(none in this share
// code)" placeholder rather than an empty pane (the wireframe's mock always
// has content; a real share code legitimately may not — surface it
// honestly, like the rest of the redesign's empty states).
//
// `firacode_nerd` is the only registered monospace family (HANDOFF caveat);
// all glyphs used here are ASCII so coverage is not a concern.
//
// SPEC: §4.2 (preview tab contents), §6.4 (file-folder tab pattern).
// Wireframe: screens.jsx:469-529 (verbatim).

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// constants — correct by construction (Cat 2); the doc-paragraph-length and
// line-count lints are subjective style on a faithfully-mirrored widget
// (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines
)]

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::state_install::PreviewTab;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong, redesign_chrome_bg,
    redesign_shell_bg, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

/// Render the 6-tab strip. Mutates `active` in place when a tab is clicked.
/// Caller renders the Content Box immediately below so the active tab merges
/// into it. `pub(crate)` to stay consistent with `render_tab_body` (which
/// must be `pub(crate)` — it takes the BIO `pub(crate)` `ModlistSharePreview`).
pub(crate) fn render_tab_strip(ui: &mut egui::Ui, palette: ThemePalette, active: &mut PreviewTab) {
    // Wireframe: the strip is `display:flex; gap:4; flexWrap:wrap;
    // borderBottom: 1.5px solid border-strong`. Capture the strip origin so
    // we can punch a shell-bg gap under the active tab afterwards.
    let strip_left = ui.cursor().left();
    let strip_width = ui.available_width();

    let mut active_tab_rect: Option<egui::Rect> = None;

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for tab in PreviewTab::ALL {
            let is_active = tab == *active;
            let rect = render_one_tab(ui, palette, tab, is_active);
            if is_active {
                active_tab_rect = Some(rect);
            }
            if tab_clicked(ui, rect) {
                *active = tab;
            }
        }
    });

    // The strip's continuous 1.5px baseline rule (wireframe `borderBottom`).
    let baseline_y = ui.cursor().top() - 1.0;
    let painter = ui.painter();
    painter.line_segment(
        [
            egui::pos2(strip_left, baseline_y),
            egui::pos2(strip_left + strip_width, baseline_y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    // "Active tab merges into the box": over-paint a shell-bg segment of the
    // baseline exactly under the active tab so the border visually opens
    // into the Content Box below (the file-folder effect — wireframe
    // `borderBottom: 1.5px solid shell-bg` on the active tab).
    if let Some(r) = active_tab_rect {
        painter.line_segment(
            [
                egui::pos2(r.left() + REDESIGN_BORDER_WIDTH_PX, baseline_y),
                egui::pos2(r.right() - REDESIGN_BORDER_WIDTH_PX, baseline_y),
            ],
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX + 0.5, redesign_shell_bg(palette)),
        );
    }
}

/// Paint a single tab and return its rect (so the caller can punch the
/// baseline under the active one + hit-test the click).
fn render_one_tab(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tab: PreviewTab,
    is_active: bool,
) -> egui::Rect {
    let pad_x = 14.0;
    let pad_y = 6.0;
    let font = egui::FontId::new(
        14.0,
        egui::FontFamily::Name(if is_active {
            "poppins_bold".into()
        } else {
            "poppins_light".into()
        }),
    );
    let text_color = if is_active {
        redesign_text_primary(palette)
    } else {
        redesign_text_muted(palette)
    };
    let label = tab.display_label();
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius {
            nw: 4,
            ne: 4,
            sw: 0,
            se: 0,
        };
        let fill = if is_active {
            redesign_shell_bg(palette)
        } else {
            redesign_chrome_bg(palette)
        };
        painter.rect_filled(rect, radius, fill);
        // Wireframe: 1.5px border on top/left/right; the bottom edge is the
        // shell-bg merge for the active tab (handled by the baseline
        // over-paint in `render_tab_strip`) and border-strong for inactive
        // (the strip baseline rule already draws that). So stroke only the
        // three non-bottom sides here.
        let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
        painter.line_segment([rect.left_top(), rect.right_top()], stroke); // top
        painter.line_segment([rect.left_top(), rect.left_bottom()], stroke); // left
        painter.line_segment([rect.right_top(), rect.right_bottom()], stroke); // right
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }

    rect
}

/// Hit-test a tab rect for a primary click this frame.
fn tab_clicked(ui: &egui::Ui, rect: egui::Rect) -> bool {
    ui.input(|i| {
        i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
    })
}

/// Render the active tab's monospace body inside the Content Box
/// (`PreviewText`). Wireframe: `FiraCode Nerd` 13px, lineHeight 1.35,
/// whiteSpace pre-wrap, color text.
pub(crate) fn render_tab_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tab: PreviewTab,
    preview: &ModlistSharePreview,
) {
    let text = tab_text(tab, preview);
    let trimmed_empty = text.trim().is_empty();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if trimmed_empty {
                ui.label(
                    egui::RichText::new("(none in this share code)")
                        .size(13.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_text_faint(palette)),
                );
            } else {
                ui.label(
                    egui::RichText::new(text)
                        .size(13.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_text_primary(palette)),
                );
            }
        });
}

/// Build the per-tab text from the parsed preview. Summary is a composed
/// recap (wireframe `PreviewText` "Summary" structure); every other tab is
/// the verbatim packaged section.
fn tab_text(tab: PreviewTab, p: &ModlistSharePreview) -> String {
    match tab {
        PreviewTab::Summary => summary_text(p),
        PreviewTab::BgeeWeidu => p.bgee_log_text.clone(),
        PreviewTab::Bg2eeWeidu => p.bg2ee_log_text.clone(),
        PreviewTab::UserDownloads => p.source_overrides_text.clone(),
        PreviewTab::InstalledRefs => p.installed_refs_text.clone(),
        PreviewTab::ModConfigs => p.mod_configs_text.clone(),
    }
}

/// The Summary recap, structured exactly like the wireframe's `PreviewText`
/// "Summary" string (screens.jsx:521) but filled from the real
/// `ModlistSharePreview`. The redesign has no "Step 1", so the
/// "What Import Will Do" bullets use the wireframe's Step-1-free wording
/// (the wireframe is canonical over BIO's `format_modlist_import_preview`,
/// which still references Step 1 — that copy belongs to the legacy wizard).
fn summary_text(p: &ModlistSharePreview) -> String {
    let yn = |b: bool| if b { "Yes" } else { "No" };
    format!(
        "BIO Modlist Import Preview\n\n\
         Modlist\n\
         BIO version: {bio}\n\
         Game install: {game}\n\
         Install mode: {mode}\n\n\
         WeiDU Logs\n\
         BGEE: {bgee} entries\n\
         BG2EE: {bg2ee} entries\n\n\
         Included Data\n\
         Source overrides: {src}\n\
         Installed refs / pins: {refs}\n\
         Mod config files: {cfg}\n\n\
         What Import Will Do\n\
         - Set game/install mode from this share code.\n\
         - Write imported WeiDU logs.\n\
         - Import source overrides if included.\n\
         - Import installed refs/pins if included.\n\
         - Store pending mod config files if included.\n\
         - Keep local game, mods, archive, and backup paths unchanged.",
        bio = p.bio_version,
        game = p.game_install,
        mode = p.install_mode,
        bgee = p.bgee_entries,
        bg2ee = p.bg2ee_entries,
        src = yn(p.has_source_overrides),
        refs = yn(p.has_installed_refs),
        cfg = p.mod_config_count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_preview() -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "0.1.0-test".to_string(),
            game_install: "EET".to_string(),
            install_mode: "start_from_weidu_logs_then_review_edit".to_string(),
            bgee_entries: 21,
            bg2ee_entries: 115,
            has_source_overrides: true,
            has_installed_refs: true,
            bgee_log_text: "// BGEE log\n~A\\A.TP2~ #0 #0 // X".to_string(),
            bg2ee_log_text: String::new(),
            source_overrides_text: "[[mods]]".to_string(),
            installed_refs_text: "[refs]".to_string(),
            mod_config_count: 4,
            mod_configs_text: "a | b | c".to_string(),
            allow_auto_install: true,
            name: None,
            author: None,
            forked_from: Vec::new(),
        }
    }

    #[test]
    fn summary_is_populated_from_preview() {
        let s = summary_text(&sample_preview());
        assert!(s.starts_with("BIO Modlist Import Preview"));
        assert!(s.contains("Game install: EET"));
        assert!(s.contains("BGEE: 21 entries"));
        assert!(s.contains("BG2EE: 115 entries"));
        assert!(s.contains("Source overrides: Yes"));
        assert!(s.contains("Mod config files: 4"));
        // Redesign copy — no Step 1 references (wireframe is canonical).
        assert!(!s.contains("Step 1"));
    }

    #[test]
    fn verbatim_tabs_pass_through_preview_sections() {
        let p = sample_preview();
        assert_eq!(tab_text(PreviewTab::BgeeWeidu, &p), p.bgee_log_text);
        assert_eq!(
            tab_text(PreviewTab::UserDownloads, &p),
            p.source_overrides_text
        );
        assert_eq!(
            tab_text(PreviewTab::InstalledRefs, &p),
            p.installed_refs_text
        );
        assert_eq!(tab_text(PreviewTab::ModConfigs, &p), p.mod_configs_text);
        // An empty section yields an empty string (the renderer shows the
        // faint "(none …)" placeholder for that case).
        assert!(tab_text(PreviewTab::Bg2eeWeidu, &p).is_empty());
    }
}
