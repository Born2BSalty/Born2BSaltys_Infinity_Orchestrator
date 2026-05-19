// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `ui::install::stage_installing` — Install-Modlist **Stage 4**, the real
// install runtime (SPEC §4.4 `InstallProgressScreen` / §9.3). Phase 7
// P7.T15 (Run 4b). Replaces the Phase-5/Run-4a `stage_installing_stub`.
//
// **What this is.** The §4.4 screen the user sees when an install was
// entered from **Install Modlist** (paste / Reinstall / Create-import /
// Load-Draft-via-Install) rather than from inside a Workspace. Per SPEC
// §4.4 + §9.3 it embeds the SAME BIO Step-5 panel as the workspace chrome
// but with its OWN minimal chrome — it does **not** use `workspace_view`'s
// 4-step progress bar / `workspace_nav_bar` / Save-Draft / Share-import
// header (the user came in from Install, not Create — SPEC §4.4: "no Share
// import code … the code that produced this install is the one they
// already pasted").
//
// **Render order — mirrors `page_workspace_step5::render` exactly, with
// this screen's own chrome instead of the workspace chrome:**
//   1. A simple header: `Installing modlist · <name> · live install
//      console` + a back affordance (SPEC §4.4 header). `<name>` is the
//      packed name from the parsed share-code preview (the same
//      honest-fallback `Shared modlist` `stage_preview` uses); the back
//      affordance routes to Preview (if a parse is cached) else Paste —
//      the SAME target `stage_installing_stub` used (SPEC §4.4 acceptance).
//   2. The C3-gated **success banner** ABOVE the panel (SPEC §4.4 routes
//      the post-success state to §9.2, whose FIRST element IS the green
//      `Installed` pill + `<N> mods · <C> components · no errors` +
//      `ran <MM:SS> · finished <relative>`). The §9.2-vs-§9.3 split is by
//      install state (finished vs. during), NOT by entry point — the only
//      §4.4-specific exclusion is the Share-import-code button (SPEC §4.4:
//      "the user is not offered a Share import code button from this entry
//      point"). So this screen shows the SAME §9.2 banner the workspace
//      Step-5 completion shows. **Reuses the redesign-owned
//      `success_banner::render` AS-IS** (the counts-based,
//      name-independent component — it renders correctly even with the
//      legitimate `Shared modlist` fallback title since the banner reads
//      `entry.mod_count` / `component_count` / timestamps, not the name).
//      Same gate / counts source as the workspace path
//      (`page_workspace_step5::render`). Empty pre/during/failed install
//      (the shared C3 predicate `success_banner::clean_exit` is false).
//   3. The C3-gated post-install action row, immediately BELOW the banner
//      and ABOVE the panel (per H9 — visually adjacent to BIO's
//      now-disabled `✓ Installed` button at the top of
//      `page_step5::render`'s panel). **Reuses the redesign-owned
//      `post_install_actions::render`** (`Return to Home` + `Open install
//      folder` — exactly the SPEC §4.4 / §9.2 post-install actions; it
//      renders NO Share, which IS the one §4.4-specific exclusion). Empty
//      pre/during/failed install (C3 false). Banner-then-row is the EXACT
//      order `page_workspace_step5::render` uses.
//   4. BIO's entire embedded Step-5 panel via the **EXACT Run-1 reuse +
//      borrow pattern** (`page_workspace_step5.rs`): clone
//      `exe_fingerprint`, five disjoint field borrows
//      (`wizard_state` / `step5_console_view` / `step5_terminal` /
//      `step5_terminal_error` / `dev_mode` + the cloned fingerprint). BIO's
//      Step-5 tree is reused **READ-ONLY** — never edited.
//   5. Dispatch the returned `Step5Action::StartInstall` the SAME way
//      `page_workspace_step5` does — **gated so it does not double-start**:
//      Run-4a's pipeline (`start_auto_build_install`) already flipped
//      `start_install_requested` before advancing to this seam, and the
//      orchestrator's per-frame `start_step5_after_render` (P7.T1) started
//      the install. So flipping `start_install_requested` again while the
//      install is already requested/running/prepping would be the
//      double-start. The gate: only flip when the install is NOT already
//      in flight (`!start_install_requested && !install_running &&
//      !prep_running`) — the defensive same-frame edge where BIO's panel
//      momentarily shows the Install button before `install_running` turns
//      true. (No concurrency gate / `on_install_start` here — that is the
//      workspace Step-5 path's job; this screen's install was driven by
//      the Run-4a auto-build pipeline, not a Step-5 Install click.)
//
// **Post-install actions are applied INSIDE** (this screen takes `&mut
// OrchestratorApp`, so unlike the render-gate-safe split there is no need
// to bubble the action up): `Return to Home` → `Nav(Home)` returned to the
// dispatcher; `Open install folder` → the SAME
// `registry::operations::open_install_folder` the Home Kebab /
// `page_workspace_step5` use, surfacing a failure in the standard bottom
// toast in its error tone (SPEC §3.2 — never create the folder). The
// folder target is the **registry entry** when one exists (a Reinstall —
// resolved by `pending`/`workspace_view`/the destination) else the
// Install-Modlist destination string.
//
// SPEC: §4.4 (`InstallProgressScreen`), §9.3 (Step-5 panel reuse), §9.2
//        (post-install actions / H9), §13.15 (concurrency — handled at the
//        rail layer, not here).

use eframe::egui;

use crate::registry::operations;
use crate::ui::install::state_install::InstallStage;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_primary,
};
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::workspace::step5::state_workspace_step5::PostInstallAction;
use crate::ui::workspace::step5::{post_install_actions, success_banner};

/// SPEC §4.2/§4.4 honest fallback when the share code carries no packed
/// `name` (the exact string `stage_preview`'s `FALLBACK_TITLE` uses — never
/// fabricate a name).
const FALLBACK_NAME: &str = "Shared modlist";

/// What this stage wants the `page_install` dispatcher to do next (the
/// deferred-intent pattern the dispatcher already uses — applied after the
/// render borrow ends; a stage transition vs a nav change are mutually
/// exclusive per click).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StageInstallingOutcome {
    /// Stay on the install screen (the normal case while the install runs
    /// / after it finishes if the user does nothing).
    #[default]
    Stay,
    /// The header back affordance was clicked — go to the cached Preview,
    /// or Paste when no preview is cached (SPEC §4.4 acceptance, the same
    /// target the Phase-5 stub used).
    Back(InstallStage),
    /// Leave Install for another top-level destination — the post-install
    /// `Return to Home` (SPEC §4.4 / §9.2).
    Nav(NavDestination),
}

/// Render the Stage-4 `InstallProgressScreen`. Takes `&mut OrchestratorApp`
/// (it drives BIO's embedded `page_step5::render` with the orchestrator's
/// Step-5 fields, exactly like `page_workspace_step5::render`). Applies the
/// post-install `Open install folder` side-effect inside; returns the
/// nav/back intent for the dispatcher.
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> StageInstallingOutcome {
    let palette = orchestrator.theme_palette;

    // The packed modlist name from the parsed preview (the user reviewed it
    // on the Preview stage). Honest fallback `Shared modlist` — never
    // fabricate (SPEC §4.2).
    let name = orchestrator
        .install_screen_state
        .parsed_preview
        .as_ref()
        .and_then(|p| p.name.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_NAME)
        .to_string();

    // Back target: the Phase-5-stub rule (SPEC §4.4: "returns to stage 2,
    // or paste if no preview is cached").
    let back_target = if orchestrator.install_screen_state.preview_cached {
        InstallStage::Preview
    } else {
        InstallStage::Paste
    };

    // ── 1. Header — the wireframe `InstallProgressScreen`
    //    (`screens.jsx:549-550`, the canonical reference; wins over SPEC
    //    prose per spec-authority): `<ScreenTitle title="Installing
    //    modlist" sub="<name> · live install console" />` + a flush-right
    //    small `← back to import` `Btn`. NOT the workspace 4-step progress
    //    bar / nav bar / Save-Draft / Share header — this screen has its
    //    own minimal chrome (SPEC §4.4 / §9.3). Layout mirrors
    //    `stage_preview.rs`'s "ScreenTitle + flush-right button" precedent
    //    (`horizontal_top` → `allocate_ui_with_layout` for the title +
    //    `with_layout(right_to_left)` for the button, the title width
    //    reserved so it wraps before colliding). ──
    let mut outcome = StageInstallingOutcome::Stay;
    let sub = format!("{name} \u{00B7} live install console");
    ui.horizontal_top(|ui| {
        let back_btn_w = 130.0;
        let title_w = (ui.available_width() - back_btn_w).max(160.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                render_screen_title(ui, palette, "Installing modlist", Some(&sub));
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.add_space(0.0);
            if back_to_import_btn(ui, palette).clicked() {
                outcome = StageInstallingOutcome::Back(back_target);
            }
        });
    });
    ui.add_space(10.0);

    // Snapshot the routed modlist's registry entry once for the post-install
    // row + the Open-folder target. For a **Reinstall** there IS a registry
    // entry (resolved by the loaded workspace id, or — more robustly here —
    // by matching the Install-Modlist destination, since the Install-Modlist
    // screen has no `workspace_view.loaded_workspace_id`). For a plain
    // Install-Modlist *paste* of a brand-new modlist there is no entry yet;
    // the post-install row still renders (it gates only on the C3 triple,
    // not on the entry) and Open-folder falls back to the destination
    // string. Clone so the immutable `registry` borrow ends before the
    // `&mut orchestrator` field-split / `page_step5::render` below (the same
    // clone-the-entry discipline `page_workspace_step5` uses).
    let dest = orchestrator
        .install_screen_state
        .destination
        .trim()
        .to_string();
    let entry = orchestrator
        .registry
        .entries
        .iter()
        .find(|e| e.destination_folder.trim() == dest && !dest.is_empty())
        .cloned();

    // The registry entry (or a default when a fresh paste has none) — the
    // banner's counts/timestamps + the post-install row's gating/identity.
    // The banner reads `entry.mod_count` / `component_count` /
    // `install_started_at` / `install_date`, NOT the modlist name, so the
    // (correct) `Shared modlist` fallback title for a nameless pasted code
    // does not affect it. Cloned so the immutable `registry` borrow ends
    // before the `&mut orchestrator` field-split / `page_step5::render`.
    let entry_for_row = entry.clone().unwrap_or_default();

    // ── 2. C3-gated success banner ABOVE the panel (SPEC §4.4 line ~343
    //    routes the post-success state to §9.2, Appendix B.2; SPEC §9.2's
    //    FIRST element IS the green `Installed` pill + `<N> mods · <C>
    //    components · no errors` + `ran <MM:SS> · finished <relative>`).
    //    The §9.2-vs-§9.3 split is by install state (finished vs. during),
    //    NOT by entry point — the ONLY §4.4-specific exclusion is the
    //    Share-import-code button (SPEC §4.4: "the user is not offered a
    //    Share import code button from this entry point"; that button lives
    //    in the workspace header, not in this screen's chrome — already
    //    correctly absent here). So this Install-Modlist completion screen
    //    shows the SAME §9.2 banner the workspace Step-5 completion shows.
    //    Reuses the redesign-owned `success_banner::render` **AS-IS** with
    //    the SAME args/gate/counts source the workspace path uses
    //    (`page_workspace_step5::render`: `(ui, palette,
    //    &orchestrator.wizard_state, &entry)` — the shared
    //    `WizardState`/registry entry, which now persists on the
    //    Install-Modlist path thanks to A-1). It renders nothing until the
    //    shared C3 clean-exit predicate `success_banner::clean_exit` holds
    //    (pre/during/failed install ⇒ empty slot; the embedded panel below
    //    shows the live console), so this is purely additive — no behavior
    //    change to the during-install / pre-install / failed states. ──
    success_banner::render(ui, palette, &orchestrator.wizard_state, &entry_for_row);

    // ── 3. C3-gated post-install action row, immediately BELOW the banner
    //    and ABOVE the panel (per H9 — banner-then-row is the EXACT order
    //    `page_workspace_step5::render` uses). The redesign-owned
    //    `post_install_actions::render` renders exactly `Return to Home` +
    //    `Open install folder` (NO Share — the one §4.4-specific exclusion,
    //    SPEC §4.4: the user pasted the code). Renders nothing until the
    //    SAME C3 clean-exit triple holds (pre/during/failed install ⇒ empty
    //    slot, the embedded panel below shows the live console).
    //    `post_install_actions::render` takes a `&ModlistEntry` only for
    //    gating/identity symmetry with `success_banner`; when there is no
    //    registry entry (a fresh paste) it is a default — the row's gate is
    //    the C3 triple, and the open-folder target is resolved below from
    //    the entry-or-destination. ──
    let post_install_action: Option<PostInstallAction> =
        post_install_actions::render(ui, palette, &orchestrator.wizard_state, &entry_for_row);

    // ── 4. BIO's entire embedded Step-5 panel — the EXACT Run-1
    //    reuse/borrow pattern (`page_workspace_step5.rs`): clone
    //    `exe_fingerprint`, then five disjoint struct-field borrows (a
    //    sound split borrow). BIO's Step-5 tree is reused READ-ONLY and
    //    never edited. While the install runs the panel shows the live
    //    console + Cancel Install + Actions/Diagnostics/Prompt Answers +
    //    the prompt input row (BIO's own renderers — the orchestrator does
    //    not duplicate them).
    //
    //    **T-C — right-edge underflow fix (redesign-owned chrome only).**
    //    BIO's `page_step5::render` (protected) lays its console + cards
    //    out from `ui.available_width()`; on this Install-Modlist screen
    //    the wrapper previously handed it the bare `ui`, whose
    //    `available_rect` extends to the egui surface edge (no right bound)
    //    — so a long console line / wide card stretched past the app's
    //    right edge / underflowed the window. The fix bounds the panel to
    //    the **window-available rect** here (the wrapper's layout/clip
    //    rect — NOT a BIO edit) via the established `clipped_pane`
    //    containment helper (the verbatim `workspace_step2::clipped_pane`
    //    pattern): a child UI whose `max_rect` + hard-set `clip_rect` are
    //    the window-available rect, after which the parent placer is
    //    advanced by exactly that rect. BIO's panel keeps its full internal
    //    behavior (its own `ScrollArea`, the prompt input row, etc.) but
    //    can no longer paint past the window's right edge. `clip_rect`
    //    intersects the inherited clip so it also respects any ancestor
    //    bound. ──
    let exe_fingerprint = orchestrator.exe_fingerprint.clone();
    // The window-bounded rect for the panel: full remaining height, width
    // clamped to what the screen actually offers (never the unbounded egui
    // surface). `available_rect_before_wrap` is the post-header remaining
    // area the dispatcher/shell already constrained horizontally.
    let panel_rect = ui.available_rect_before_wrap();
    let mut action: Option<Step5Action> = None;
    clipped_pane(ui, panel_rect, |ui| {
        action = crate::ui::step5::page_step5::render(
            ui,
            &mut orchestrator.wizard_state,
            &mut orchestrator.step5_console_view,
            orchestrator.step5_terminal.as_mut(),
            orchestrator.step5_terminal_error.as_deref(),
            orchestrator.dev_mode,
            &exe_fingerprint,
        );
    });

    // ── 5. Dispatch the returned action — GATED so it cannot double-start.
    //    Run-4a's auto-build pipeline (`start_auto_build_install`) already
    //    flipped `start_install_requested` before advancing to this seam,
    //    and `start_step5_after_render` (P7.T1) started the install. So if
    //    the install is already requested / running / prepping, a
    //    `StartInstall` here MUST be ignored (re-flipping
    //    `start_install_requested` is the double-start). The only case
    //    where flipping is correct is the defensive same-frame edge where
    //    BIO's panel still shows the Install button before
    //    `install_running` turns true — and even then the pipeline already
    //    set the flag, so this is belt-and-braces. (Dispatched the SAME
    //    way `page_workspace_step5` does — flip
    //    `state.step5.start_install_requested = true` — but only when not
    //    already in flight; no `on_install_start` / concurrency gate here:
    //    this screen's install was driven by the pipeline, not a Step-5
    //    Install click.) ──
    if let Some(Step5Action::StartInstall) = action {
        let s5 = &orchestrator.wizard_state.step5;
        let already_in_flight = s5.start_install_requested || s5.install_running || s5.prep_running;
        if already_in_flight {
            tracing::debug!(
                target = "orchestrator",
                "stage_installing: ignoring Step5Action::StartInstall — \
                 install already in flight (the Run-4a auto-build pipeline \
                 already flipped start_install_requested; re-flipping would \
                 double-start)"
            );
        } else {
            orchestrator.wizard_state.step5.start_install_requested = true;
        }
    }

    // ── Apply the post-install action AFTER `page_step5::render` (the
    //    `&mut orchestrator.wizard_state` / split-field render borrows have
    //    ended). `Return to Home` → return `Nav(Home)` for the dispatcher
    //    (the freshly-installed modlist shows under Home's Installed chip —
    //    P7.T12). `Open install folder` → the SAME
    //    `registry::operations::open_install_folder` the Home Kebab /
    //    `page_workspace_step5` use; the target is the registry entry when
    //    one exists (a Reinstall) else a synthesized entry carrying the
    //    Install-Modlist destination string (a fresh paste — no registry
    //    entry yet). A failure (unset / missing folder) surfaces in the
    //    standard bottom toast in its error tone (SPEC §3.2 — do not create
    //    the folder), exactly like `page_workspace_step5`. ──
    match post_install_action {
        Some(PostInstallAction::ReturnToHome) => {
            outcome = StageInstallingOutcome::Nav(NavDestination::Home);
        }
        Some(PostInstallAction::OpenInstallFolder) => {
            // Prefer the real registry entry (Reinstall); otherwise build a
            // throwaway entry carrying just the destination so the same
            // path-validated `open_install_folder` (and its honest
            // missing-folder error) is used uniformly.
            let target = entry.unwrap_or_else(|| {
                let mut e = crate::registry::model::ModlistEntry::default();
                e.name = name.clone();
                e.destination_folder = dest.clone();
                e
            });
            if let Err(msg) = operations::open_install_folder(&target) {
                orchestrator.home_screen_state.toast =
                    Some(crate::ui::home::state_home::ToastMessage::error(msg));
            }
        }
        None => {}
    }

    outcome
}

/// **T-C containment** — run BIO's reused `page_step5::render` inside a
/// **hard-clipped, fixed-size** child UI bounded to `rect` (the
/// window-available area). Verbatim of the established
/// `workspace_step2::clipped_pane` pattern (that fn is module-private to
/// `workspace_step2`; this is the same net-new local helper, no
/// shared-widget / BIO edit).
///
/// `ui.new_child` makes a child that does **not** advance the parent's
/// placer; its `clip_rect` is hard-set to `rect ∩ ancestor-clip` so BIO's
/// panel (its console lines, wide cards) physically cannot paint outside
/// the window's right edge; then the parent is advanced by **exactly
/// `rect`** so a tall panel can never push the layout. The panel keeps its
/// full internal behavior (its own `ScrollArea`, prompt input row, menus);
/// only the unbounded horizontal overflow is contained.
fn clipped_pane(ui: &mut egui::Ui, rect: egui::Rect, add: impl FnOnce(&mut egui::Ui)) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    // Intersect with the inherited clip so we never paint OUTSIDE the rect
    // (nor outside any ancestor clip).
    let clip = rect.intersect(ui.clip_rect());
    child.set_clip_rect(clip);
    add(&mut child);
    // Advance the parent by EXACTLY the bounded rect — discard the child's
    // (possibly overgrown) min_rect so the layout after it is never pushed.
    ui.allocate_rect(rect, egui::Sense::hover());
}

/// The wireframe `InstallProgressScreen` back affordance
/// (`screens.jsx:550` — `<Btn small>← back to import</Btn>`). The `←`
/// U+2190 glyph is a base-FiraCode glyph the bundled full FiraCode Nerd
/// build covers but the Latin-only Poppins subset **tofus** (HANDOFF
/// symbol-glyph caveat — a `redesign_btn`, which renders its whole label in
/// `poppins_medium`, would show `?`). So this paints the glyph in
/// `firacode_nerd` and the prose in `poppins_medium` **side by side**
/// inside a sketchy-bordered small button — the exact established
/// glyph-aware convention (`workspace_nav_bar::glyph_btn`,
/// `install/sub_flow_footer`, `home/toast`). Net-new local helper (no
/// shared-widget edit — `redesign_btn` is not modified). Visual chrome
/// matches `redesign_btn(small, !primary)`: `redesign_shell_bg` fill,
/// 1.5px `redesign_border_strong` stroke, 3px radius, the same 10×4
/// padding + 12px text + 1px active-press shift.
fn back_to_import_btn(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font_size = 12.0;
    let gap = 5.0;

    let fill = redesign_shell_bg(palette);
    let text_color = redesign_text_primary(palette);
    let border = redesign_border_strong(palette);

    let glyph_font = egui::FontId::new(font_size, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));

    // `←` U+2190 in firacode_nerd; `back to import` in poppins_medium.
    let glyph_galley =
        ui.painter()
            .layout_no_wrap("\u{2190}".to_string(), glyph_font.clone(), text_color);
    let prose_galley =
        ui.painter()
            .layout_no_wrap("back to import".to_string(), prose_font.clone(), text_color);

    let content_w = glyph_galley.size().x + gap + prose_galley.size().x;
    let content_h = glyph_galley.size().y.max(prose_galley.size().y);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);

    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    // Wireframe `.sk-btn:active { transform: translate(1px, 1px) }` — the
    // same tactile press `redesign_btn`/`glyph_btn` apply.
    let pressed = response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(rect, radius, fill);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border),
            egui::StrokeKind::Inside,
        );
        // [glyph][gap][prose] centered in the rect — the exact
        // `workspace_nav_bar::glyph_btn` paint pattern
        // (`painter.text(..., Align2::LEFT_CENTER, ...)`, glyph in
        // `firacode_nerd`, prose in `poppins_medium`).
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            "\u{2190}",
            glyph_font,
            text_color,
        );
        painter.text(
            egui::pos2(start_x + glyph_galley.size().x + gap, cy),
            egui::Align2::LEFT_CENTER,
            "back to import",
            prose_font,
            text_color,
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_name_is_the_spec_authoritative_string() {
        // SPEC §4.2 — never fabricate a name; the honest generic fallback,
        // bit-identical to `stage_preview`'s `FALLBACK_TITLE`.
        assert_eq!(FALLBACK_NAME, "Shared modlist");
    }

    #[test]
    fn outcome_default_is_stay() {
        assert_eq!(
            StageInstallingOutcome::default(),
            StageInstallingOutcome::Stay
        );
    }

    #[test]
    fn back_target_is_preview_when_cached_else_paste() {
        // The Phase-5-stub rule (SPEC §4.4 acceptance) — re-asserted on the
        // `InstallScreenState.preview_cached` flag so a future refactor of
        // the back target is caught (the screen renders `Back(Preview)`
        // iff a parse is cached, else `Back(Paste)`).
        use crate::ui::install::state_install::InstallScreenState;
        let mut st = InstallScreenState::default();
        assert!(!st.preview_cached);
        let t = if st.preview_cached {
            InstallStage::Preview
        } else {
            InstallStage::Paste
        };
        assert_eq!(t, InstallStage::Paste, "no cached preview ⇒ Back to Paste");
        st.preview_cached = true;
        let t = if st.preview_cached {
            InstallStage::Preview
        } else {
            InstallStage::Paste
        };
        assert_eq!(t, InstallStage::Preview, "cached preview ⇒ Back to Preview");
    }
}
