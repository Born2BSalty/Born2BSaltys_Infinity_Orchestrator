// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_header` — the full workspace header row (P6.T5 + P6.T6),
// replacing Run-1's minimal `Editing <name>` placeholder.
//
// Mirrors the wireframe `WorkspaceView` header (`screens.jsx:3442-3553`):
//
//   <div flex justifyContent:space-between alignItems:flex-end>
//     <div>
//       <div flex alignItems:center gap:8 wrap>
//         {renaming
//           ? <span>Editing</span> <input/> <Btn primary>save</Btn>
//                                            <Btn>cancel</Btn>
//           : <h1>Editing {displayName}</h1> <span onClick=startRename>✎</span>}
//         {fork && <div ...sketchy accent badge>⑂ Fork</div>}
//       </div>
//       <div fontSize:16 color:text-muted marginTop:4>
//         {fork
//           ? <⑂ Forked from "{name}" by {author} · {m} mods · {c} preselected>
//           : `${source} · shared BIO workflow`}
//       </div>
//     </div>
//     <div flex gap:8>
//       {fork && <Btn small onClick=forkInfo>⑂ view fork details</Btn>}
//       {tab==="final" ? <Btn>Share import code</Btn>
//                      : <Btn>{draftSavedAt ? "✓ saved!" : "save draft"}</Btn>}
//     </div>
//   </div>
//
// ## P6.T5 — inline rename (SPEC §2.2)
//
// `Editing <name>` (Poppins 13/500, primary — wireframe `h1` line) + a `✎`
// affordance. `✎` U+270E is **cmap-ABSENT even in the full bundled FiraCode
// Nerd build** (verified this run via fontTools — U+270E is in the Dingbats
// block the HANDOFF caveat flags as uncovered) → **painted as a vector
// pencil** (the established precedent: `left_rail.rs` nav icons,
// `destination_not_empty.rs::paint_warning_triangle`,
// `fork_info_popup.rs::paint_fork_glyph`). Clicking it swaps the title for an
// inline `TextEdit` + primary `save` / `cancel` (the exact
// `settings/widgets/name_row` chassis). **Enter saves, Escape cancels**
// (wireframe `onKeyDown`). Save → `registry::operations_rename::
// rename_modlist` (**registry-entry rename ONLY — no on-disk folder rename**,
// SPEC §2.2) + `persistence_cycle.mark_registry_dirty(now)` so the write is
// **debounced** through the existing registry persistence cycle (SPEC §13.14;
// NOT `workspace_state_dirty`). The Home card reflects the new name on next
// visit (it reads `registry`); the install folder is unchanged.
//
// ## P6.T5 — fork badge + `⑂ view fork details`
//
// Shown **only when this modlist's `forked_from` is non-empty** (SPEC §2.2;
// `workspace_view.fork_meta` is `Some` iff so — populated from the registry
// entry by `page_router::render_workspace`). The badge (`⑂ Fork`, accent
// sketchy chip) + the fork sub-line (immediate parent = last `forked_from`)
// + `⑂ view fork details` all paint the **vector fork glyph** (`⑂` U+2442 is
// cmap-ABSENT — same precedent). `⑂ view fork details` opens the **reused
// Phase-5 `ForkInfoPopup`** (`orchestrator::widgets::dialogs::
// fork_info_popup` — the SAME widget the Install preview uses; no new
// popup), fed the entry's `forked_from` chain + the entry's own
// `name`/`author` as the current node (SPEC §10.9).
//
// ## P6.T6 — save draft (SPEC §10.1)
//
// Steps 2-4 only (Step 5 shows `Share import code` — Phase 7; this run
// renders it disabled, matching the wireframe's pre-install state). Click →
// `workspace_state_loader::extract_workspace_state_from_wizard(&wizard_state,
// <prior>)` → `WorkspaceStore::save` **immediately, synchronously** (the
// debounced cycle is Run 4 — this is the explicit immediate write). Flash
// `✓ saved!` for ~1.6s (the `✓` U+2713 IS cmap-present in `firacode_nerd`,
// rendered as a glyph), then revert to `save draft`. No dialog / file
// picker. **First caller of `extract_workspace_state_from_wizard`** (it
// shipped unit-tested but unwired in Run 1).
//
// SPEC: §2.2 (header + rename + fork badge), §10.9 (ForkInfoPopup),
//       §10.1 (Save Draft inline), §13.3 (Provenance), §13.14 (debounced
//       registry write), §1 (decision order — reuse Phase-5 popup + BIO
//       extract, net-new chrome).

// rationale: f32→u8 colour-channel / pixel roundings of small positive
// constants — correct by construction (Cat 2); the doc-paragraph-length lint
// is subjective on a faithfully-mirrored header (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines
)]

use std::time::{Duration, Instant};

use eframe::egui;

use crate::registry::operations_rename;
use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::fork_info_popup::{self, SelfNode};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_accent, redesign_accent_deep, redesign_border_strong, redesign_input_bg,
    redesign_shadow, redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::workspace_state_loader;
use tracing::warn;

/// How long the `✓ saved!` flash stays before reverting to `save draft`
/// (wireframe `setTimeout(() => setDraftSavedAt(0), 1600)`).
const SAVE_FLASH_MS: u64 = 1600;

/// Render the full workspace header. `ctx` is needed for the non-blocking
/// `ForkInfoPopup` overlay (rendered last so it floats above the shell).
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    ui.horizontal_top(|ui| {
        // ── Left column: title row + fork sub-line. ──
        ui.vertical(|ui| {
            render_title_row(ui, orchestrator, palette);
            render_fork_subline(ui, orchestrator, palette);
        });

        // ── Right cluster (wireframe `marginLeft:auto`, gap 8). ──
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            // On-screen order is `[⑂ view fork details] [save draft]`;
            // right-to-left lays the trailing one first.
            render_save_or_share_button(ui, orchestrator, palette);
            if orchestrator.workspace_view.fork_meta.is_some()
                && fork_details_button(ui, palette).clicked()
            {
                orchestrator.workspace_view.fork_info_open = true;
            }
        });
    });

    // ── ForkInfoPopup (SPEC §10.9) — the reused Phase-5 widget, rendered
    //    last so it floats above the header. Self identity = the registry
    //    entry's own name/author (NEVER from `forked_from`). ──
    if orchestrator.workspace_view.fork_info_open {
        render_fork_info_popup(orchestrator, palette, ctx);
    }
}

/// The title row: either `Editing <name>` + `✎`, or the inline rename
/// editor (`Editing` + TextEdit + save/cancel). Plus the `⑂ Fork` badge.
fn render_title_row(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, palette: ThemePalette) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if orchestrator.workspace_view.renaming {
            render_rename_editor(ui, orchestrator, palette);
        } else {
            let name = orchestrator.workspace_view.modlist_name.clone();
            let title = if name.trim().is_empty() {
                "Editing modlist".to_string()
            } else {
                format!("Editing {name}")
            };
            ui.label(
                egui::RichText::new(title)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            // `✎` rename affordance — PAINTED VECTOR (U+270E cmap-absent).
            if pencil_button(ui, palette).clicked() {
                orchestrator.workspace_view.rename_temp =
                    orchestrator.workspace_view.modlist_name.clone();
                orchestrator.workspace_view.renaming = true;
                // Clear the focus-once marker so the editor re-focuses each
                // time rename is (re-)opened.
                let m = egui::Id::new(("workspace_header_rename_edit",)).with("focused_once");
                ui.memory_mut(|mem| mem.data.remove::<bool>(m));
            }
        }

        // `⑂ Fork` badge — accent sketchy chip, shown only for a forked
        // build (wireframe `{fork && …}`).
        if orchestrator.workspace_view.fork_meta.is_some() {
            fork_badge(ui, palette);
        }
    });
}

/// The inline rename editor (wireframe `renaming` branch): `Editing` label +
/// `TextEdit` + primary `save` + `cancel`. Enter saves, Escape cancels.
fn render_rename_editor(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) {
    ui.label(
        egui::RichText::new("Editing")
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );

    let edit_id = egui::Id::new(("workspace_header_rename_edit",));
    let response = ui.add_sized(
        egui::vec2(240.0, 28.0),
        egui::TextEdit::singleline(&mut orchestrator.workspace_view.rename_temp)
            .id(edit_id)
            .font(egui::FontId::new(
                13.0,
                egui::FontFamily::Name("poppins_medium".into()),
            ))
            .text_color(redesign_text_primary(palette))
            .background_color(redesign_input_bg(palette))
            .margin(egui::Margin::symmetric(8, 4)),
    );
    ui.painter().rect_stroke(
        response.rect,
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Outside,
    );

    // Autofocus exactly once on the frame the editor opens (wireframe
    // `autoFocus`). A per-id "focused yet" flag in egui memory keeps this
    // from re-grabbing focus every frame (which would block the user
    // clicking save/cancel or typing-then-clicking-away).
    let focus_marker = edit_id.with("focused_once");
    let already_focused = ui
        .memory(|m| m.data.get_temp::<bool>(focus_marker))
        .unwrap_or(false);
    if !already_focused {
        response.request_focus();
        ui.memory_mut(|m| m.data.insert_temp(focus_marker, true));
    }

    // Keyboard (wireframe `onKeyDown`): Enter saves, Escape cancels. The
    // canonical egui idiom — `lost_focus()` fires the frame the TextEdit
    // yields focus (which Enter does), so `lost_focus && Enter` is the
    // reliable "Enter was pressed in this field" signal even though the
    // field has already blurred by the time we check.
    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));

    let mut do_save = enter;
    let mut do_cancel = escape;

    if redesign_btn(
        ui,
        palette,
        "save",
        BtnOpts {
            primary: true,
            small: true,
            ..Default::default()
        },
    )
    .clicked()
    {
        do_save = true;
    }
    if redesign_btn(
        ui,
        palette,
        "cancel",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .clicked()
    {
        do_cancel = true;
    }

    if do_save {
        commit_rename(orchestrator);
    } else if do_cancel {
        orchestrator.workspace_view.renaming = false;
        orchestrator.workspace_view.rename_temp.clear();
    }
}

/// Commit the inline rename: `operations_rename::rename_modlist` (registry-
/// entry only — SPEC §2.2) + anchor the **debounced** registry write
/// (SPEC §13.14). Mirrors the wireframe `saveRename`'s `if (tempName.trim())`
/// guard — an empty name is a no-op cancel (no rename, editor closes).
fn commit_rename(orchestrator: &mut OrchestratorApp) {
    let new_name = orchestrator.workspace_view.rename_temp.trim().to_string();
    orchestrator.workspace_view.renaming = false;

    if new_name.is_empty() {
        // Wireframe: empty trimmed name → don't rename, just close.
        orchestrator.workspace_view.rename_temp.clear();
        return;
    }

    let id = orchestrator.workspace_view.modlist_id.clone();
    match operations_rename::rename_modlist(&id, &new_name, &mut orchestrator.registry) {
        Ok(()) => {
            // Reflect immediately in the workspace header; the Home card
            // reads `registry` so it shows the new name on next visit.
            orchestrator.workspace_view.modlist_name = new_name;
            // SPEC §2.2 / §13.14 — the registry write is DEBOUNCED. Anchor
            // the debounce timer; `OrchestratorApp::tick_persistence` →
            // `persist_registry_if_needed` flushes it after the window
            // (a missing dirty mark would force an immediate write — that
            // is the delete path's contract, not rename's).
            orchestrator
                .persistence_cycle
                .mark_registry_dirty(Instant::now());
        }
        Err(err) => {
            // No IO is performed by `rename_modlist`; an error here is a
            // validation failure (empty / unknown id). The id is the loaded
            // workspace's, so `NotFound` is unexpected; log and leave the
            // name unchanged (the user can retry).
            warn!(target = "orchestrator", "rename_modlist failed: {err}");
        }
    }
    orchestrator.workspace_view.rename_temp.clear();
}

/// The fork sub-line (wireframe `fork ? … : "${source} · shared BIO
/// workflow"`). For a forked build: `⑂ Forked from "<parent>" by <author> ·
/// <m> mods · <c> components preselected` (the immediate parent = the last
/// `forked_from` ancestor, carried on `ForkMeta`). For a non-forked build
/// the wireframe shows a generic source line; the orchestrator has no
/// "source" string for a from-scratch build, so it renders nothing (the
/// SPEC-authoritative honest-absence rule — never invent a source).
fn render_fork_subline(ui: &mut egui::Ui, orchestrator: &OrchestratorApp, palette: ThemePalette) {
    let Some(meta) = orchestrator.workspace_view.fork_meta.as_ref() else {
        return;
    };
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // `⑂ Forked from` — vector fork + accent-deep bold prose.
        paint_inline_fork(ui, redesign_accent_deep(palette));
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Forked from ")
                .size(16.0)
                .family(egui::FontFamily::Name("poppins_bold".into()))
                .color(redesign_accent_deep(palette)),
        );
        ui.label(
            egui::RichText::new(format!("\"{}\"", meta.parent_name))
                .size(16.0)
                .family(egui::FontFamily::Name("poppins_bold".into()))
                .color(redesign_text_primary(palette)),
        );
        // `by <author>` — OMITTED when the parent author is empty
        // (SPEC §10.9 / §4.2 author-absent rule; never render `by —`).
        if !meta.parent_author.trim().is_empty() {
            ui.label(
                egui::RichText::new(format!(" by {}", meta.parent_author.trim()))
                    .size(16.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_muted(palette)),
            );
        }
        ui.label(
            egui::RichText::new(format!(
                " \u{00B7} {} mods \u{00B7} {} components preselected",
                meta.mods, meta.components
            ))
            .size(16.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
        );
    });
}

/// The right-cluster `save draft` (Steps 2-4) / `Share import code` (Step 5)
/// button. P6.T6: Steps 2-4 = `save draft` (immediate synchronous write +
/// `✓ saved!` flash); Step 5 = `Share import code`, **disabled** here (it
/// enables post-install — Phase 7; wireframe shows it disabled pre-install).
fn render_save_or_share_button(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) {
    if orchestrator.workspace_view.current_step == WorkspaceStep::Step5 {
        // Step 5: `Share import code`, disabled until a successful install
        // (Phase 7 wires the dialog + the enable). Wireframe pre-install
        // state: `disabled`, secondary.
        let _ = redesign_btn(
            ui,
            palette,
            "Share import code",
            BtnOpts {
                small: true,
                disabled: true,
                ..Default::default()
            },
        )
        .on_hover_text("Available after a successful install");
        return;
    }

    // Steps 2-4: `save draft` / `✓ saved!`. Expire the flash if its window
    // elapsed.
    let now = Instant::now();
    let flashing = match orchestrator.workspace_view.save_draft_flash_until {
        Some(until) if now < until => true,
        Some(_) => {
            orchestrator.workspace_view.save_draft_flash_until = None;
            false
        }
        None => false,
    };

    if flashing {
        // `✓ saved!` — `✓` U+2713 IS cmap-present in firacode_nerd (verified)
        // so it renders as a glyph; the prose stays Poppins. Painted chassis
        // (glyph in firacode_nerd, prose in poppins) — the established
        // mixed-font pattern (`workspace_nav_bar`, `sub_flow_footer`).
        let _ = saved_flash_button(ui, palette);
        // Keep repainting so the flash reverts even without user input.
        ui.ctx().request_repaint_after(Duration::from_millis(120));
    } else if redesign_btn(
        ui,
        palette,
        "save draft",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .on_hover_text("Save this in-progress build so you can resume it from Home")
    .clicked()
    {
        save_draft(orchestrator);
    }
}

/// P6.T6 — persist the current workspace state **immediately, synchronously**
/// (the debounced cycle is Run 4; this is the explicit immediate write).
/// **First caller of `extract_workspace_state_from_wizard`** (it shipped
/// unit-tested but unwired in Run 1).
fn save_draft(orchestrator: &mut OrchestratorApp) {
    let id = orchestrator.workspace_view.modlist_id.clone();
    if id.is_empty() {
        return;
    }

    // `prior` = the workspace state currently in the orchestrator's map (or
    // default if not yet loaded). `extract` carries `prior`'s egui-side
    // fields (Step-2 expand map, prompt overrides, last_share_code) through
    // unchanged so an immediate save doesn't drop them.
    let prior = orchestrator
        .workspace_state
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let extracted = workspace_state_loader::extract_workspace_state_from_wizard(
        &orchestrator.wizard_state,
        &prior,
    );

    // Resolve / reuse the per-modlist store. (`render_workspace` inserts one
    // on open; a defensive `new_for_id` covers any edge where it isn't yet
    // in the map.)
    let store = orchestrator
        .workspace_stores
        .entry(id.clone())
        .or_insert_with(|| WorkspaceStore::new_for_id(&id));

    match store.save(&extracted) {
        Ok(()) => {
            // Keep the in-memory map + the persistence-cycle baseline in
            // sync so the Run-4 debounced cycle doesn't redundantly rewrite
            // the identical state immediately after.
            orchestrator
                .workspace_state
                .insert(id.clone(), extracted.clone());
            orchestrator
                .persistence_cycle
                .last_saved_workspaces
                .insert(id.clone(), extracted);
            // Flash `✓ saved!` for ~1.6s (wireframe).
            orchestrator.workspace_view.save_draft_flash_until =
                Some(Instant::now() + Duration::from_millis(SAVE_FLASH_MS));
        }
        Err(err) => {
            warn!(target = "orchestrator", "save draft for {id} failed: {err}");
        }
    }
}

/// Render the reused Phase-5 `ForkInfoPopup` for `⑂ view fork details`
/// (SPEC §10.9). Self identity = the **registry entry's** own name/author
/// (NEVER from `forked_from`); the lineage = the entry's `forked_from`. The
/// `id_salt` distinguishes this instance from the Install-preview one.
fn render_fork_info_popup(
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    ctx: &egui::Context,
) {
    // The current node's own identity comes from the registry entry (not
    // `fork_meta`, which holds the *parent*; not `forked_from`, which never
    // contains the modlist's own identity — SPEC §10.9 / §13.3 append rule).
    let id = orchestrator.workspace_view.modlist_id.clone();
    let (self_name, self_author, lineage) = match orchestrator.registry.find(&id) {
        Some(e) => (
            if e.name.trim().is_empty() {
                orchestrator.workspace_view.modlist_name.clone()
            } else {
                e.name.clone()
            },
            e.author.clone().unwrap_or_default(),
            e.forked_from.clone(),
        ),
        None => (
            orchestrator.workspace_view.modlist_name.clone(),
            String::new(),
            // Fall back to the cached lineage on `fork_meta` if the entry
            // vanished (stale state) — keeps the popup honest, not blank.
            orchestrator
                .workspace_view
                .fork_meta
                .as_ref()
                .map(|m| m.forked_from.clone())
                .unwrap_or_default(),
        ),
    };

    let outcome = fork_info_popup::render(
        ctx,
        palette,
        "workspace_header",
        &lineage,
        &SelfNode {
            name: &self_name,
            author: self_author.trim(),
        },
    );
    if outcome == fork_info_popup::ForkInfoOutcome::Closed {
        orchestrator.workspace_view.fork_info_open = false;
    }
}

// ───────────────────────── painted vector glyphs ─────────────────────────

/// The `✎` rename affordance as a small clickable button with a **painted
/// vector pencil** (U+270E is cmap-absent even in full FiraCode Nerd —
/// verified this run). Chassis = a borderless hover-highlighted hit target
/// (wireframe `<span onClick>` — not a boxed button, just a clickable icon).
fn pencil_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad = 4.0;
    let ink = 13.0; // ~ the wireframe `✎` 13px glyph box
    let desired = egui::vec2(ink + pad * 2.0, ink + pad * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let color = if response.hovered() {
        redesign_text_primary(palette)
    } else {
        redesign_text_muted(palette)
    };
    if ui.is_rect_visible(rect) {
        paint_pencil_glyph(ui.painter(), rect.center(), ink, color);
    }
    response.on_hover_text("Rename modlist")
}

/// Paint `✎` (pencil) as a vector: a slanted body (two parallel edges) with
/// a nib at the lower-left and an eraser/cap at the upper-right. Sized to fit
/// an `ink`-px box centered on `center`. Same vector-glyph convention as
/// `destination_not_empty.rs::paint_warning_triangle` /
/// `fork_info_popup.rs::paint_fork_glyph` (each widget paints its own).
fn paint_pencil_glyph(painter: &egui::Painter, center: egui::Pos2, ink: f32, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let h = ink / 2.0;
    // Pencil runs lower-left → upper-right at ~45°.
    let tip = egui::pos2(center.x - h, center.y + h); // writing nib
    let cap = egui::pos2(center.x + h, center.y - h); // eraser end
    // Body offset perpendicular to the tip→cap axis to give it width.
    let off = egui::vec2(2.4, 2.4);
    // Two long parallel edges.
    painter.line_segment([tip - off * 0.0 + perp(off), cap + perp(off)], stroke);
    painter.line_segment([tip - perp(off), cap - perp(off)], stroke);
    // Nib (the two short edges meeting at the writing point).
    painter.line_segment([tip + perp(off), tip - perp(off)], stroke);
    // The little ferrule line near the cap.
    let ferrule_a = cap + perp(off) - normalize(cap - tip) * 3.0;
    let ferrule_b = cap - perp(off) - normalize(cap - tip) * 3.0;
    painter.line_segment([ferrule_a, ferrule_b], stroke);
    // Eraser cap.
    painter.line_segment([cap + perp(off), cap - perp(off)], stroke);
}

/// Perpendicular of a 2-vector (rotate 90°).
fn perp(v: egui::Vec2) -> egui::Vec2 {
    egui::vec2(-v.y, v.x) * 0.5
}

/// Unit vector (safe for zero).
fn normalize(v: egui::Vec2) -> egui::Vec2 {
    let len = v.length();
    if len <= f32::EPSILON {
        egui::Vec2::ZERO
    } else {
        v / len
    }
}

/// The `⑂ Fork` badge (wireframe: sketchy border, accent fill, 2×2 shadow,
/// 10px uppercase Poppins, the `⑂` painted as a vector since U+2442 is
/// cmap-absent). Theme-invariant `#1a2638` ink on the teal accent (same as
/// the primary-button text token).
fn fork_badge(ui: &mut egui::Ui, palette: ThemePalette) {
    let pad_x = 12.0;
    let pad_y = 4.0;
    let font = egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into()));
    let ink = egui::Color32::from_rgb(0x1a, 0x26, 0x38);
    let label = "FORK";
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), ink);
    let fork_w = 9.0;
    let gap = 5.0;
    let content_w = fork_w + gap + galley.size().x;
    let desired = egui::vec2(
        content_w + pad_x * 2.0,
        galley.size().y.max(fork_w) + pad_y * 2.0,
    );
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        // 2×2 shadow (wireframe `boxShadow: 2px 2px 0 var(--shadow)`).
        painter.rect_filled(
            rect.translate(egui::vec2(
                REDESIGN_SHADOW_OFFSET_BTN_PX,
                REDESIGN_SHADOW_OFFSET_BTN_PX,
            )),
            radius,
            redesign_shadow(palette),
        );
        painter.rect_filled(rect, radius, redesign_accent(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        paint_fork_at(painter, egui::pos2(start_x + fork_w / 2.0, cy), ink);
        painter.text(
            egui::pos2(start_x + fork_w + gap, cy),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            ink,
        );
    }
}

/// The `⑂ view fork details` button (wireframe `<Btn small>`). `⑂` U+2442 is
/// cmap-absent → painted vector + prose, on the small `redesign_btn` chassis
/// (sketchy border, no accent fill, active-press transform). Same approach as
/// `install/stage_preview::fork_info_button`.
fn fork_details_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let color = redesign_text_primary(palette);
    let label = "view fork details";
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), color);
    let fork_w = 9.0;
    let gap = 5.0;
    let content_w = fork_w + gap + galley.size().x;
    let content_h = galley.size().y.max(fork_w);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let pressed = response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        paint_fork_at(painter, egui::pos2(start_x + fork_w / 2.0, cy), color);
        painter.text(
            egui::pos2(start_x + fork_w + gap, cy),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            color,
        );
    }
    response
}

/// The `✓ saved!` flash chassis (small secondary button look; non-clickable
/// — it auto-reverts). `✓` U+2713 IS cmap-present in firacode_nerd (verified)
/// so it renders as a glyph; the prose is Poppins.
fn saved_flash_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let glyph_font = egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let color = redesign_text_primary(palette);
    let glyph = "\u{2713}"; // ✓ — cmap-present in firacode_nerd
    let prose = " saved!";
    let g = ui
        .painter()
        .layout_no_wrap(glyph.to_string(), glyph_font.clone(), color);
    let p = ui
        .painter()
        .layout_no_wrap(prose.to_string(), prose_font.clone(), color);
    let content_w = g.size().x + p.size().x;
    let content_h = g.size().y.max(p.size().y);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            glyph,
            glyph_font,
            color,
        );
        painter.text(
            egui::pos2(start_x + g.size().x, cy),
            egui::Align2::LEFT_CENTER,
            prose,
            prose_font,
            color,
        );
    }
    response
}

/// Paint `⑂` (fork) as a vector — a stem that splits into two tines. Same
/// geometry as `fork_info_popup.rs::paint_fork_glyph` /
/// `stage_preview.rs::paint_fork_glyph` (each widget paints its own per the
/// codebase convention). Sized ~9px ink centered on `center`.
fn paint_fork_at(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let half_h = 4.5;
    let split_y = center.y - 0.5;
    let tine_dx = 3.0;
    painter.line_segment(
        [
            egui::pos2(center.x, center.y + half_h),
            egui::pos2(center.x, split_y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x - tine_dx, center.y - half_h),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x + tine_dx, center.y - half_h),
        ],
        stroke,
    );
}

/// Inline fork glyph used in the sub-line text (allocates a small inline ink
/// box so it flows with the surrounding label run).
fn paint_inline_fork(ui: &mut egui::Ui, color: egui::Color32) {
    let w = 9.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 16.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_fork_at(ui.painter(), rect.center(), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};

    fn orch_with_entry(name: &str) -> OrchestratorApp {
        let mut app = OrchestratorApp::new(false);
        app.registry = ModlistRegistry::default();
        app.registry.entries.push(ModlistEntry {
            id: "HDRTEST00000".to_string(),
            name: name.to_string(),
            game: Game::EET,
            state: ModlistState::InProgress,
            ..Default::default()
        });
        app.workspace_view.modlist_id = "HDRTEST00000".to_string();
        app.workspace_view.modlist_name = name.to_string();
        app
    }

    /// Committing a rename writes the registry entry's `name` (registry-only,
    /// SPEC §2.2) and reflects it in the header — and anchors the debounced
    /// registry write (does NOT mark `workspace_state_dirty`).
    #[test]
    fn commit_rename_updates_registry_and_header_only() {
        let mut app = orch_with_entry("Old Name");
        app.workspace_view.renaming = true;
        app.workspace_view.rename_temp = "Brand New Name".to_string();

        commit_rename(&mut app);

        assert!(!app.workspace_view.renaming);
        assert_eq!(app.workspace_view.modlist_name, "Brand New Name");
        assert_eq!(
            app.registry.find("HDRTEST00000").unwrap().name,
            "Brand New Name"
        );
        // Rename rides the registry persistence cycle, NOT the workspace
        // dirty bit (SPEC §2.2 / §13.14 — debounced via the registry path).
        assert!(
            !app.workspace_state_dirty,
            "rename must not mark workspace_state_dirty"
        );
    }

    /// An empty trimmed rename is a no-op cancel (wireframe `saveRename`'s
    /// `if (tempName.trim())`): no rename, the editor closes, the name is
    /// unchanged.
    #[test]
    fn empty_rename_is_noop_cancel() {
        let mut app = orch_with_entry("Keep Me");
        app.workspace_view.renaming = true;
        app.workspace_view.rename_temp = "   ".to_string();

        commit_rename(&mut app);

        assert!(!app.workspace_view.renaming);
        assert_eq!(app.workspace_view.modlist_name, "Keep Me");
        assert_eq!(app.registry.find("HDRTEST00000").unwrap().name, "Keep Me");
    }
}
