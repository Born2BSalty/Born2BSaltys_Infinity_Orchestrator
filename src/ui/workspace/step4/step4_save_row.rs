// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step4_save_row` — the Step-4 top action row (P6.T2b, SPEC §8.1). Net-new
// redesign chrome; the Save **action** reuses BIO's `pub(crate)` save flow.
//
// Mirrors the wireframe `OrderPanel` action sub-row (`screens.jsx:3182-3187`):
//
//   <div flex gap:8 marginBottom:10 alignItems:center>
//     <Btn>Save weidu.log's</Btn>
//     <Label hand marginLeft:auto color:var(--text-faint)>
//       {selected.length} components ready to install on {upperTab}
//       · across {new Set(selected.map(c => c.tp2)).size} mods
//     </Label>
//   </div>
//
// - Button label: `Save weidu.log's` for a dual-game (EET) modlist,
//   `Save weidu.log` for single-game — the exact label switch BIO's
//   `content_step4::render` uses (`"EET" => "Save weidu.log's"`).
// - Click → returns `Some(Step4Action::SaveWeiduLog)` to the wrapper, which
//   returns it to the router; the router dispatches it via
//   `step_action_dispatch::dispatch_step4`. **All dispatch happens at the
//   router layer for consistency** (M11) — the Save row does NOT call the
//   BIO save fn itself, so the save-error popup is surfaced by the wrapper
//   (`workspace_step4`) from `wizard_state.step5.last_status_text`, not here.
// - Count: `<N> components ready to install on <TAB> · across <M> mods` —
//   `N` = non-parent Step-3 items on the active tab; `M` = unique `tp_file`
//   over them (the wireframe's `new Set(selected.map(c => c.tp2)).size`).
//   Rendered in the wireframe `<Label hand>` style (accent-deep, 14px) but
//   colour-overridden to `text-faint` per the wireframe's inline
//   `color: var(--text-faint)`.
//
// SPEC: §8.1 (Step-4 action row + count), §1 (decision order — net-new
//       render, reuse BIO's save action via the router).

// rationale: f32→u8 roundings are not present here; the module is layout-only.
#![allow(clippy::module_name_repetitions)]

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::step4_action::Step4Action;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};
use crate::ui::workspace::step4::workspace_step4;

/// Render the Step-4 action row into the current `ui`. Returns
/// `Some(Step4Action::SaveWeiduLog)` if the Save button was clicked (the
/// router dispatches it; per M11 all dispatch is at the router layer).
pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) -> Option<Step4Action> {
    let mut action: Option<Step4Action> = None;

    let is_dual = workspace_step4::is_dual_game(&orchestrator.wizard_state);
    // The exact label switch BIO's `content_step4::render` performs
    // (`"EET" => "Save weidu.log's"`, `_ => "Save weidu.log"`); the
    // orchestrator's modlist is dual-game iff EET.
    let save_label = if is_dual {
        "Save weidu.log's"
    } else {
        "Save weidu.log"
    };

    let (active_tab, comp_count, mod_count) = active_tab_counts(&orchestrator.wizard_state);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if redesign_btn(
            ui,
            palette,
            save_label,
            BtnOpts {
                primary: true,
                ..Default::default()
            },
        )
        .on_hover_text(crate::ui::shared::tooltip_global::STEP4_SAVE_WEIDU_LOG)
        .clicked()
        {
            // M11 — return the action; the router dispatches via
            // `dispatch_step4` → `bio::app::app_step4_flow::
            // handle_step4_action` (which routes to
            // `auto_save_step4_weidu_logs`). The save-error popup is a
            // render-side concern surfaced by the wrapper afterwards.
            action = Some(Step4Action::SaveWeiduLog);
        }

        // `marginLeft: auto` → push the count Label flush-right.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Wireframe `<Label hand>` is 14px Poppins, but its inline style
            // overrides the colour to `var(--text-faint)`.
            ui.label(
                egui::RichText::new(format!(
                    "{comp_count} components ready to install on {active_tab} \u{00B7} across {mod_count} mods"
                ))
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
            );
        });
    });

    action
}

/// `(active_tab_label, component_count, unique_mod_count)` for the count
/// text. The component count is the number of **non-parent** Step-3 items on
/// the active tab (the installable leaves — the wireframe's `selected`); the
/// mod count is the number of distinct `tp_file`s over them (the wireframe's
/// `new Set(selected.map(c => c.tp2)).size`).
fn active_tab_counts(state: &WizardState) -> (&'static str, usize, usize) {
    let (tab_label, items) = workspace_step4::active_tab_items(state);
    let leaves: Vec<&crate::app::state::Step3ItemState> =
        items.iter().filter(|i| !i.is_parent).collect();
    let comp_count = leaves.len();
    let mut seen: Vec<&str> = Vec::new();
    for it in &leaves {
        if !seen.iter().any(|t| t.eq_ignore_ascii_case(&it.tp_file)) {
            seen.push(&it.tp_file);
        }
    }
    (tab_label, comp_count, seen.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step3ItemState;

    fn leaf(tp: &str, id: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: format!("comp {id}"),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }
    }
    fn parent(tp: &str) -> Step3ItemState {
        let mut p = leaf(tp, "__PARENT__");
        p.is_parent = true;
        p
    }

    /// The count text counts installable leaves (not synthetic parent rows)
    /// and unique TP2 files — matching the wireframe `selected` /
    /// `new Set(...tp2).size`.
    #[test]
    fn counts_leaves_and_unique_mods_skipping_parents() {
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        s.step3.active_game_tab = "BGEE".to_string();
        s.step3.bgee_items = vec![
            parent("EEFIXPACK.TP2"),
            leaf("EEFIXPACK.TP2", "0"),
            leaf("EEFIXPACK.TP2", "2"),
            parent("BG1UB.TP2"),
            leaf("BG1UB.TP2", "0"),
        ];
        let (tab, comps, mods) = active_tab_counts(&s);
        assert_eq!(tab, "BGEE");
        assert_eq!(comps, 3, "3 leaves, parents excluded");
        assert_eq!(mods, 2, "2 distinct tp_files");
    }

    /// EET → dual-game → `Save weidu.log's`; everything else → single →
    /// `Save weidu.log` (BIO's exact label switch).
    #[test]
    fn save_label_switches_on_eet() {
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        assert!(workspace_step4::is_dual_game(&s));
        s.step1.game_install = "BGEE".to_string();
        assert!(!workspace_step4::is_dual_game(&s));
        s.step1.game_install = "BG2EE".to_string();
        assert!(!workspace_step4::is_dual_game(&s));
        s.step1.game_install = "IWDEE".to_string();
        assert!(!workspace_step4::is_dual_game(&s));
    }
}
