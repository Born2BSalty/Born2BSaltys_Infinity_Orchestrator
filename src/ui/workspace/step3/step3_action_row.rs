// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step3_action_row` — the Step-3 top action row (P6.T2d, SPEC §7.1).
// Net-new redesign chrome.
//
// SPEC §7.1: "Action row above the tabs: right-aligned count "_N_
// components ready to install on _<tab>_ · across _M_ mods" (**no**
// `Save weidu.log's` button — that lives in Step 4, see §8.1)."
//
// The wireframe `ComponentsPanel` (`screens.jsx:3023-3056`) does not draw a
// count line for Step 3 (only `OrderPanel`/Step-4 does, `screens.jsx:
// 3182-3187`); SPEC §7.1 explicitly specifies it for Step 3 too, and SPEC
// prose wins where the wireframe is silent (spec-authority priority order).
// It is the **same** count string Step-4's `step4_save_row` renders. The
// active-tab → ordered-items resolver is reused from the shared **`pub`**
// `workspace_step4::active_tab_items` (the single source of truth for
// "which tab, which Step-3 bucket" — identical to BIO's Step-4
// `active_step4_game_tab`), so the Step-3 and Step-4 tabs can never resolve
// differently; the trivial leaf/unique-tp2 count derivation is net-new (it
// mirrors `step4_save_row::active_tab_counts` exactly, but that helper is
// private to `step4_save_row` and this run edits no file but its own
// net-new modules + the router). Unlike Step 4 there is **no Save button**
// on this row (SPEC §7.1 / §7.6 — Save lives only in Step 4).
//
// Rendered in the wireframe `<Label hand>` style (Poppins-medium 14px) but
// colour-overridden to `text-faint` per the wireframe's inline
// `color: var(--text-faint)` (the same treatment Step-4's count uses).
//
// SPEC: §7.1 (Step-3 action row + count), §7.6 (no Save on Step 3 — Step 4
//       only), §1 (decision order — net-new render, reuse the shared
//       active-tab resolver).

#![allow(clippy::module_name_repetitions)]

use eframe::egui;

use crate::app::state::{Step3ItemState, WizardState};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};
use crate::ui::workspace::step4::workspace_step4;

/// Render the Step-3 action row into the current `ui`: a single
/// right-aligned count Label, no buttons (SPEC §7.1).
pub fn render(ui: &mut egui::Ui, state: &WizardState, palette: ThemePalette) {
    let (active_tab, comp_count, mod_count) = active_tab_counts(state);

    ui.horizontal(|ui| {
        // `marginLeft: auto` → push the count Label flush-right (the
        // wireframe Step-4 action-row treatment SPEC §7.1 mirrors).
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
}

/// `(active_tab_label, component_count, unique_mod_count)` for the count
/// text. Resolves the active tab + its Step-3 ordered items via the shared
/// `pub` `workspace_step4::active_tab_items` (the single tab→bucket source
/// of truth — identical to BIO's Step-4 `active_step4_game_tab`), then
/// counts the installable **leaves** (`!is_parent` — synthetic Step-3
/// parent-header rows are not installable components, the wireframe's
/// `selected`) and the number of distinct `tp_file`s over them (the
/// wireframe's `new Set(selected.map(c => c.tp2)).size`). Mirrors
/// `step4_save_row::active_tab_counts` exactly so the Step-3 / Step-4 count
/// strings stay identical (SPEC §7.1 / §8.1 specify the same wording).
fn active_tab_counts(state: &WizardState) -> (&'static str, usize, usize) {
    let (tab_label, items) = workspace_step4::active_tab_items(state);
    let leaves: Vec<&Step3ItemState> = items.iter().filter(|i| !i.is_parent).collect();
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
    /// and unique TP2 files on the active tab — matching the wireframe
    /// `selected` / `new Set(...tp2).size` and Step-4's identical count.
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

    /// EET active-tab switch resolves the BG2EE bucket — proves the Step-3
    /// count tracks `step3.active_game_tab` exactly as Step-4 does (shared
    /// `active_tab_items` resolver).
    #[test]
    fn eet_active_tab_switch_resolves_bg2ee_bucket() {
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        s.step3.bgee_items = vec![leaf("A.TP2", "0")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0"), leaf("B.TP2", "1")];

        s.step3.active_game_tab = "BGEE".to_string();
        let (tab, comps, _mods) = active_tab_counts(&s);
        assert_eq!(tab, "BGEE");
        assert_eq!(comps, 1);

        s.step3.active_game_tab = "BG2EE".to_string();
        let (tab, comps, _mods) = active_tab_counts(&s);
        assert_eq!(tab, "BG2EE");
        assert_eq!(comps, 2);
    }
}
