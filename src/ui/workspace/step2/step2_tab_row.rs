// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step2_tab_row` — the **net-new redesign Step-2 tab row** (P6.T2c). The
// direct analogue of how Step-4's C4 (`step4/`, P6.T2b) rebuilt its chrome:
// net-new redesign layout, reusing **only** BIO data/state reads + the
// `pub(crate)` action helpers BIO's own toolbar calls (directive
// decision-order step 1 — read-only reuse, no logic reimplementation).
//
// Mirrors the wireframe `SourcesPanel` tab row exactly
// (`wireframe-preview/screens.jsx:2807-2852`):
//
//   <div flex gap:4 position:relative zIndex:1>
//     {tabs.map(t => <GameTab active onClick=setGameTab>{t.label}</GameTab>)}
//     <div flex:1 flex gap:8 paddingLeft:12 paddingRight:4 height:30 wrap>
//       {!fork && <TopButton onClick=askWeiduImport>Select {tab} via WeiDU Log</TopButton>}
//       <TopButton onClick=setUpdatesOpen>Updates...</TopButton>
//       <Pill tone=danger onClick=compat>{tab} {cat} {count}</Pill>
//       <Pill tone=warn  onClick=prompt>PROMPT {n}</Pill>
//       <Label hand marginLeft:auto>{sel} / {total} on {tab}</Label>
//       <Kebab items=[ Show Details panel(✓), Clear All, Select Visible,
//                      Collapse All, Expand All, Jump to Selected ] />
//     </div>
//   </div>
//
// **The wireframe has NO BIO controls row** (no `render_controls` /
// `render_tabs`). The toolbar controls live in the Kebab + the Rescan
// button (in `step2_search`). This row reproduces the wireframe shape with
// redesign primitives; it never calls BIO's `content_step2::render_*`.
//
// ## Action / state mapping (read off `content_step2::render_controls` +
//    `render_tabs` — the authoritative source, reproduced read-only):
//
//   - GameTabs                → write `state.step2.active_game_tab`
//                               (`content_step2::draw_tab`, line 18-47).
//   - `Select <Tab> via WeiDU Log` → arms the orchestrator-owned
//                               `workspace_view.step2.pending_weidu_log_confirm`
//                               (the SPEC §6.10 / wireframe `askWeiduImport`
//                               destructive gate). It does NOT emit an action
//                               directly: `workspace_step2::render` shows the
//                               danger `ConfirmDialog`; only on **Confirm**
//                               does it dispatch `Step2Action::
//                               Select{Bgee,Bg2ee}ViaLog` → `step2_log_glue`
//                               sibling. Cancel = no-op.
//   - `Updates...`            → `Step2Action::OpenUpdatePopup`
//                               (`content_step2.rs:317`).
//   - compat `Pill`           → `toolbar_actions_step2::open_active_tab_issue`
//                               (`content_step2.rs:347`; `pub(crate)`). Count
//                               via `toolbar_compat_step2::
//                               active_tab_compat_summary` (`pub(crate)`,
//                               the exact source `render_tabs:258` uses).
//   - `PROMPT` `Pill`         → `toolbar_actions_step2::open_prompt_toolbar`
//                               (`content_step2.rs:354`; `pub(crate)`). Count
//                               via `prompt_popup_step2::
//                               collect_step2_prompt_toolbar_entries`
//                               (`pub(crate)`, the exact source
//                               `render_tabs:259-263` sums).
//   - `selected / total on <Tab>` → `state.step2.{selected_count,
//                               total_count}` (public fields, recomputed by
//                               `service_list_ops_step2::
//                               recompute_selection_counts` — same source
//                               BIO uses for the count text).
//   - Kebab `Show Details panel`  → toggles `workspace_view.step2.
//                               details_open` (orchestrator-owned; SPEC §6).
//   - Kebab `Clear All`       → `toolbar_actions_step2::
//                               clear_all_and_refresh_compat` (the exact fn
//                               `render_controls:174` calls; `pub(crate)`).
//   - Kebab `Select Visible`  → `toolbar_actions_step2::
//                               select_visible_and_refresh_compat`
//                               (`render_controls:184`; `pub(crate)`).
//   - Kebab `Collapse All`    → `toolbar_actions_step2::collapse_all`
//                               (`render_controls:194`; `pub(crate)`).
//   - Kebab `Expand All`      → `toolbar_actions_step2::expand_all`
//                               (`render_controls:204`; `pub(crate)`).
//   - Kebab `Jump to Selected`→ `state.step2.jump_to_selected_requested =
//                               true` (the exact mutation
//                               `render_controls:213` performs; public field).
//
// None of these reimplement BIO logic — every action delegates to the same
// `pub(crate)` helper / public-field write `content_step2::render_controls`
// / `render_tabs` perform (directive decision-order step 1: reuse BIO's
// public-at-crate API). The two log-picker variants flow back to the router
// as `Step2Action` (it owns the `step2_log_glue` sibling dispatch).
//
// SPEC: §6, §1 (decision order), §2.2; wireframe `screens.jsx:2807-2852`.

// rationale: f32→u8 corner-radius / pixel roundings of small positive
// layout constants — correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, KebabItem, redesign_btn, render_kebab};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_accent_deep, redesign_pill_danger, redesign_pill_text,
    redesign_pill_warn,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::prompt_popup_step2::collect_step2_prompt_toolbar_entries;
use crate::ui::step2::state_step2::{non_scan_controls_locked, review_edit_scan_complete};
use crate::ui::step2::toolbar_actions_step2;
use crate::ui::step2::toolbar_compat_step2::{
    active_tab_compat_summary, first_active_tab_issue_target,
};
use crate::ui::workspace::widgets::game_tab::game_tab;

/// Gap between GameTabs (wireframe outer row `gap: 4`). The tab geometry
/// itself (height, padding) lives in the shared `widgets::game_tab` widget.
const TAB_GAP: f32 = 4.0;
/// Gap between action-row items (wireframe inner row `gap: 8`).
const ITEM_GAP: f32 = 8.0;
/// Left pad before the action sub-row (wireframe inner `paddingLeft: 12`).
const ACTION_LEFT_PAD: f32 = 12.0;

/// The active-tab mods slice — the exact selector `content_step2`'s private
/// `active_mods_ref` performs (`content_step2.rs:398-404`), reproduced from
/// **public `Step2State` fields only** (not BIO logic; pure state read).
fn active_mods(state: &crate::app::state::WizardState) -> &Vec<crate::app::state::Step2ModState> {
    if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    }
}

/// Render the net-new redesign tab row into `rect`. Returns any
/// `Step2Action` produced (only the two log-picker variants — they route to
/// the `step2_log_glue` sibling via the router's `dispatch_step2`).
pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    rect: egui::Rect,
) -> Option<Step2Action> {
    let mut action: Option<Step2Action> = None;

    // ── Tab-set + count predicates (public-state reads only; the exact
    //    conditions `content_step2::render_tabs` uses, lines 240-279). ──
    let game = orchestrator.wizard_state.step1.game_install.clone();
    let show_bgee = matches!(game.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(game.as_str(), "BG2EE" | "EET");
    let bgee_scanned = !orchestrator.wizard_state.step2.bgee_mods.is_empty();
    let bg2_scanned = !orchestrator.wizard_state.step2.bg2ee_mods.is_empty();
    let has_completed_scan =
        bgee_scanned || bg2_scanned || review_edit_scan_complete(&orchestrator.wizard_state);
    let controls_locked = non_scan_controls_locked(&orchestrator.wizard_state);
    let exact_log_mode = orchestrator
        .wizard_state
        .step1
        .installs_exactly_from_weidu_logs();
    let bootstraps_from_log = orchestrator.wizard_state.step1.bootstraps_from_weidu_logs();
    let build_from_scanned = !orchestrator.wizard_state.step1.uses_source_weidu_logs();
    // `can_bootstrap_from_log` — `content_step2::render_tabs:264-270`.
    let can_bootstrap_from_log = if exact_log_mode {
        false
    } else if bootstraps_from_log {
        review_edit_scan_complete(&orchestrator.wizard_state)
    } else {
        has_completed_scan
    };
    let is_scanning = orchestrator.wizard_state.step2.is_scanning;
    let active_tab = orchestrator.wizard_state.step2.active_game_tab.clone();
    let active_is_bgee = active_tab == "BGEE";
    let active_is_bg2 = active_tab == "BG2EE";

    // Compat summary + first issue target (the exact `pub(crate)` helpers
    // `render_tabs:258,276-277` use — read-only, no logic copied).
    let issue_summary = active_tab_compat_summary(active_mods(&orchestrator.wizard_state));
    let target_filter: String = if orchestrator
        .wizard_state
        .step2
        .compat_popup_filter
        .eq_ignore_ascii_case("All")
    {
        issue_summary.dominant_filter.to_owned()
    } else {
        orchestrator.wizard_state.step2.compat_popup_filter.clone()
    };
    let issue_target =
        first_active_tab_issue_target(active_mods(&orchestrator.wizard_state), &target_filter);

    // Prompt count — the exact source `render_tabs:259-263` sums.
    let prompt_count: usize = collect_step2_prompt_toolbar_entries(&orchestrator.wizard_state)
        .iter()
        .map(|e| e.component_ids.len())
        .sum();

    let selected_count = orchestrator.wizard_state.step2.selected_count;
    let total_count = orchestrator.wizard_state.step2.total_count;
    let has_selection = orchestrator.wizard_state.step2.selected.is_some();
    // Effective "is this a fork build" flag — the wireframe hides
    // `Select <Tab> via WeiDU Log` for forks (`screens.jsx:2828` `!fork`).
    // The orchestrator surfaces forks via `workspace_view.fork_meta`.
    let is_fork = orchestrator.workspace_view.fork_meta.is_some();

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TAB_GAP;

            // ── GameTabs (wireframe `GameTab`, `screens.jsx:1609-1637`). ──
            if show_bgee {
                game_tab(ui, palette, "BGEE", &mut orchestrator.wizard_state.step2.active_game_tab);
            }
            if show_bg2ee {
                game_tab(ui, palette, "BG2EE", &mut orchestrator.wizard_state.step2.active_game_tab);
            }

            // ── Action sub-row (`flex:1`, the wireframe inner div). ──
            ui.add_space(ACTION_LEFT_PAD - TAB_GAP);
            ui.spacing_mut().item_spacing.x = ITEM_GAP;

            // `Select <Tab> via WeiDU Log` — hidden for forks + exact-log
            // mode (`render_tabs:282-307`). Active-tab specific.
            //
            // SPEC §6.10 + wireframe `askWeiduImport` (`screens.jsx:
            // 2778-2784`): Select-via-Log is **destructive** (it replaces
            // *every* selection on the tab), so the button does NOT dispatch
            // the picker. It arms the orchestrator-owned
            // `pending_weidu_log_confirm` with the target tab; the danger
            // `ConfirmDialog` (rendered by `workspace_step2::render`) then
            // gates it — only on Confirm does the
            // `Step2Action::Select{Bgee,Bg2ee}ViaLog` picker+apply path run;
            // on Cancel nothing changes.
            if !is_fork && !exact_log_mode {
                if active_is_bgee {
                    let enabled = bgee_scanned || can_bootstrap_from_log;
                    if redesign_btn(
                        ui,
                        palette,
                        "Select BGEE via WeiDU Log",
                        BtnOpts {
                            small: true,
                            disabled: !enabled,
                            ..Default::default()
                        },
                    )
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SELECT_BGEE_LOG)
                    .clicked()
                        && enabled
                    {
                        orchestrator.workspace_view.step2.pending_weidu_log_confirm = Some(true);
                    }
                } else if active_is_bg2 {
                    let enabled = bg2_scanned || can_bootstrap_from_log;
                    if redesign_btn(
                        ui,
                        palette,
                        "Select BG2EE via WeiDU Log",
                        BtnOpts {
                            small: true,
                            disabled: !enabled,
                            ..Default::default()
                        },
                    )
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SELECT_BG2EE_LOG)
                    .clicked()
                        && enabled
                    {
                        orchestrator.workspace_view.step2.pending_weidu_log_confirm = Some(false);
                    }
                }
            }

            // `Updates...` / `Mod List...` — the exact enable matrix
            // `render_tabs:308-339` uses (per install mode).
            let (updates_label, updates_enabled) = if build_from_scanned {
                ("Updates...", has_completed_scan && !is_scanning)
            } else if exact_log_mode {
                ("Mod List...", !is_scanning)
            } else {
                (
                    "Updates...",
                    review_edit_scan_complete(&orchestrator.wizard_state) && !is_scanning,
                )
            };
            if redesign_btn(
                ui,
                palette,
                updates_label,
                BtnOpts {
                    small: true,
                    disabled: !updates_enabled,
                    ..Default::default()
                },
            )
            .on_hover_text("Open the updates popup.")
            .clicked()
                && updates_enabled
            {
                action = Some(Step2Action::OpenUpdatePopup);
            }

            // compat `Pill` (tone=danger) — shown only when there are
            // issues (`draw_active_tab_issue_badge:110` returns false at 0).
            if issue_summary.total_count > 0 {
                let display_filter = if orchestrator
                    .wizard_state
                    .step2
                    .compat_popup_filter
                    .eq_ignore_ascii_case("All")
                {
                    issue_summary.dominant_filter
                } else {
                    orchestrator.wizard_state.step2.compat_popup_filter.as_str()
                };
                let display_count = if orchestrator
                    .wizard_state
                    .step2
                    .compat_popup_filter
                    .eq_ignore_ascii_case("All")
                {
                    issue_summary.dominant_count
                } else {
                    issue_summary.total_count
                };
                let issue_word = if issue_summary.total_count == 1 {
                    "issue"
                } else {
                    "issues"
                };
                let tip = format!(
                    "{} compatibility {} in the {} Step 2 tab. Active badge category: {} ({}). Dominant category: {} ({}).",
                    issue_summary.total_count,
                    issue_word,
                    active_tab,
                    display_filter,
                    display_count,
                    issue_summary.dominant_filter,
                    issue_summary.dominant_count
                );
                if clickable_pill(
                    ui,
                    palette,
                    &format!("{active_tab} {display_filter} {display_count}"),
                    redesign_pill_danger(palette),
                    &tip,
                )
                .clicked()
                {
                    toolbar_actions_step2::open_active_tab_issue(
                        &mut orchestrator.wizard_state,
                        &issue_summary,
                        issue_target.clone(),
                    );
                }
            }

            // `PROMPT` `Pill` (tone=warn) — shown only when count > 0
            // (`draw_prompt_toolbar_badge:87` returns false at 0).
            if prompt_count > 0
                && clickable_pill(
                    ui,
                    palette,
                    &format!("PROMPT {prompt_count}"),
                    redesign_pill_warn(palette),
                    crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS,
                )
                .clicked()
            {
                toolbar_actions_step2::open_prompt_toolbar(&mut orchestrator.wizard_state);
            }

            // `<Label hand marginLeft:auto>{sel} / {total} on {tab}</Label>`
            // + Kebab pinned to the right edge (wireframe `marginLeft:auto`
            // pushes the count + Kebab flush-right).
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Kebab is rightmost (it follows the count in the wireframe;
                // right-to-left lays it out first).
                let details_open = orchestrator.workspace_view.step2.details_open;
                let ui_locked = controls_locked || exact_log_mode;
                // Shared `Cell` (not `&mut`) so each Kebab item's `FnMut`
                // closure can record its intent — the same pattern
                // `home::modlist_card` uses for multi-item Kebabs.
                let picked: std::cell::Cell<Option<KebabIntent>> = std::cell::Cell::new(None);
                {
                    let mut items = [
                        KebabItem::new(details_label(details_open), || {
                            picked.set(Some(KebabIntent::ToggleDetails));
                        }),
                        KebabItem::new("Clear All", || {
                            picked.set(Some(KebabIntent::ClearAll));
                        }),
                        KebabItem::new("Select Visible", || {
                            picked.set(Some(KebabIntent::SelectVisible));
                        }),
                        KebabItem::new("Collapse All", || {
                            picked.set(Some(KebabIntent::CollapseAll));
                        }),
                        KebabItem::new("Expand All", || {
                            picked.set(Some(KebabIntent::ExpandAll));
                        }),
                        KebabItem::new("Jump to Selected", || {
                            picked.set(Some(KebabIntent::JumpToSelected));
                        }),
                    ];
                    render_kebab(ui, palette, "step2_tab_row_kebab", &mut items);
                }
                if let Some(intent) = picked.get() {
                    apply_kebab_intent(intent, orchestrator, ui_locked, has_completed_scan);
                }

                ui.add_space(ITEM_GAP);
                // The count Label (wireframe `<Label hand>`: accent-deep,
                // 12px per `screens.jsx:2840`).
                ui.label(
                    egui::RichText::new(format!(
                        "{selected_count} / {total_count} on {active_tab}"
                    ))
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_accent_deep(palette)),
                );
            });
            let _ = has_selection;
        });
    });

    action
}

/// Kebab item intents — collected inside the (borrow-restricted) Kebab
/// closures, then applied after the widget returns (the closures can't hold
/// `&mut orchestrator` while the Kebab also borrows `ui`).
#[derive(Clone, Copy)]
enum KebabIntent {
    ToggleDetails,
    ClearAll,
    SelectVisible,
    CollapseAll,
    ExpandAll,
    JumpToSelected,
}

/// Apply a Kebab intent by delegating to the **exact** `pub(crate)` helper /
/// public-field write BIO's `content_step2::render_controls` performs for
/// the same control (directive decision-order step 1 — reuse, not
/// reimplement). The enable predicates mirror `render_controls`' (lines
/// 165-213) so disabled actions are no-ops, matching BIO behavior.
fn apply_kebab_intent(
    intent: KebabIntent,
    orchestrator: &mut OrchestratorApp,
    ui_locked: bool,
    has_completed_scan: bool,
) {
    let state = &mut orchestrator.wizard_state;
    match intent {
        // Orchestrator-owned (SPEC §6 — Details hidden by default; the
        // wireframe surfaces this toggle in the Kebab).
        KebabIntent::ToggleDetails => {
            orchestrator.workspace_view.step2.details_open =
                !orchestrator.workspace_view.step2.details_open;
        }
        // `render_controls:165-175` — Clear All needs a completed scan,
        // not locked, and at least one checked item.
        KebabIntent::ClearAll => {
            let has_any_checked = active_mods(state)
                .iter()
                .any(|m| m.checked || m.components.iter().any(|c| c.checked));
            if has_completed_scan && !ui_locked && has_any_checked {
                toolbar_actions_step2::clear_all_and_refresh_compat(state);
            }
        }
        // `render_controls:176-185`.
        KebabIntent::SelectVisible => {
            if has_completed_scan && !ui_locked {
                toolbar_actions_step2::select_visible_and_refresh_compat(state);
            }
        }
        // `render_controls:186-195`.
        KebabIntent::CollapseAll => {
            if has_completed_scan && !ui_locked {
                toolbar_actions_step2::collapse_all(state);
            }
        }
        // `render_controls:196-205`.
        KebabIntent::ExpandAll => {
            if has_completed_scan && !ui_locked {
                toolbar_actions_step2::expand_all(state);
            }
        }
        // `render_controls:206-214` — Jump needs a selection + not locked;
        // the exact mutation is `jump_to_selected_requested = true`.
        KebabIntent::JumpToSelected => {
            if state.step2.selected.is_some() && !ui_locked {
                state.step2.jump_to_selected_requested = true;
            }
        }
    }
}

/// The Details-toggle Kebab label. The wireframe prefixes `"✓ "` when open
/// (`screens.jsx:2844`); see the module-level SPEC-CONFLICT note in
/// `workspace_step2.rs` — the shared `kebab` widget renders labels
/// poppins-only (no `firacode_nerd` fallback) and is out of this run's
/// editable scope, so the open-state indicator is carried in the label text
/// (ASCII the Poppins subset covers) pending the user's decision.
fn details_label(open: bool) -> &'static str {
    if open {
        "Hide Details panel"
    } else {
        "Show Details panel"
    }
}

// The Step-2 GameTab is the ONE shared
// `crate::ui::workspace::widgets::game_tab::game_tab` widget (imported
// above; called at the GameTabs row). No per-step duplicate painter, and it
// has **no bottom bar in any state** — the prior all-four-sides stroke +
// active-bottom-overpaint / idle-coincides-with-pane scheme (the source of
// the reported bottom bar under idle tabs) is gone. Step 2 / 3 / 4 now
// render this one widget identically.

/// A clickable wireframe `Pill` (`screens.jsx:759-788` with `onClick`):
/// rounded chip, tinted fill, fixed dark `pill_text`, `cursor: pointer`.
///
/// The shared `pill::render` widget is hover-only (`Sense::hover()`); the
/// wireframe's compat / prompt pills are clickable. Rather than edit the
/// out-of-scope shared widget, this paints the identical pill chassis
/// (same fill / radius / `pill_text` tokens) with `Sense::click()` — a
/// net-new redesign-token-styled control, the C4 "rebuild chrome, reuse
/// data" approach.
fn clickable_pill(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    fill: egui::Color32,
    tooltip: &str,
) -> egui::Response {
    let pad_x = 6.0;
    let pad_y = 1.0;
    let text_color = redesign_pill_text(palette);
    let font = egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let size = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        // Wireframe `Pill` `borderRadius: 7`.
        painter.rect_filled(rect, egui::CornerRadius::same(7), fill);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }
    response.on_hover_text(tooltip.to_owned())
}
