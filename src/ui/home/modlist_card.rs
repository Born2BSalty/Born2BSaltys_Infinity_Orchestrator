// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `modlist_card` — one card in the Home filtered list. Two card types
// (in-progress vs installed) share the same chassis, differing only in the
// meta line and the right-cluster action set.
//
// Mirrors `wireframe-preview/screens.jsx::HomeScreen` cards (line 318-346):
//   Box { display:flex; justify-content:space-between; align-items:center;
//         padding:"10px 12px" }
//     left:  <Label>{name}</Label>
//            <Label hand style={fontSize:14, color:var(--text-faint)}>{meta}</Label>
//     right (in-progress): <Btn small primary>resume</Btn> + <Kebab .../>
//            Kebab: Copy import code / Rename / Delete
//     right (installed):   <Btn small>play</Btn> + <Kebab .../>
//            Kebab: Copy import code / Open install folder / Rename /
//                   Reinstall / Delete
//
// Per SPEC §3.2 + HANDOFF M6 the installed action button is **`open`** (not
// the wireframe's `play`) for v1 alpha — there is no game launcher yet, so
// the label honestly reflects the behavior (opens the install folder).
//
// Meta lines (SPEC §3.2):
//   in-progress: `<N> mods · <C> components · last touched <rel>
//                  [· paused at Step <K>]`
//                — the `· paused at Step <K>` segment is OMITTED (not a
//                  placeholder) when `entry.paused_at_step` is `None`.
//   installed:   `<N> mods · <size> · installed <rel>`
//                — `<size>` renders `—` when `entry.total_size_bytes` is
//                  `None` (post-install size computation lands in Phase 7;
//                  Phase 5 / Run 1 has no size yet).
//
// **Run 1 nuance:** the card renders the Kebab with all its menu items
// present, but each item's `on_click` is a placeholder no-op (`|| {}`).
// Delete / Copy import code / Open install folder / Reinstall / Rename are
// wired in Run 2 (P5.T7 / T16 / T17 / T18). The primary action button
// (`resume` / `open`) returns a `ModlistCardActions` so the page can route;
// Run 1's page only acts on `Resume`.
//
// SPEC: §3.2 (cards shared shape), §3.1 (in-list cards + Kebab item sets).

use eframe::egui;

use crate::registry::model::{ModlistEntry, ModlistState};
use crate::ui::orchestrator::widgets::{
    BtnOpts, KebabItem, redesign_btn, render_kebab,
};
use crate::ui::shared::format_relative::relative_time;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary,
};

/// What the user did on a card this frame. Run 1 only the primary button
/// produces a meaningful variant; Kebab actions are inert placeholders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ModlistCardActions {
    /// Nothing was clicked.
    #[default]
    None,
    /// In-progress card's `resume` button — open the workspace at Step 2.
    Resume,
    /// Installed card's `open` button — open the install folder (Run 2 wires
    /// the actual folder-open via P5.T17; Run 1 surfaces the intent only).
    Open,
}

/// Render one modlist card. `id_salt` (the entry id) keeps each card's Kebab
/// popup state independent.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &ModlistEntry,
) -> ModlistCardActions {
    let mut action = ModlistCardActions::None;

    // Card chassis: sketchy Box, 10×12 padding, name/meta on the left, the
    // action cluster flush-right.
    let chassis = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin {
            left: 12,
            right: 12,
            top: 10,
            bottom: 10,
        });

    chassis.show(ui, |ui| {
        ui.horizontal(|ui| {
            // ── Left: name + meta stack (grows to fill). ──
            let full_w = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(full_w, 40.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        egui::RichText::new(&entry.name)
                            .size(13.0)
                            .family(egui::FontFamily::Name("poppins_bold".into()))
                            .color(redesign_text_primary(palette)),
                    );
                    ui.label(
                        egui::RichText::new(meta_line(entry))
                            .size(14.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_faint(palette)),
                    );
                },
            );

            // ── Right: action cluster (button + Kebab), flush-right. ──
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                action = render_action_cluster(ui, palette, entry);
            });
        });
    });

    action
}

/// Build the meta line for an entry per SPEC §3.2.
pub fn meta_line(entry: &ModlistEntry) -> String {
    match entry.state {
        ModlistState::InProgress => {
            let mut s = format!(
                "{} mods \u{00B7} {} components \u{00B7} last touched {}",
                entry.mod_count,
                entry.component_count,
                relative_time(entry.last_touched_date),
            );
            // Omit the segment entirely (not a placeholder) when unknown.
            if let Some(step) = entry.paused_at_step {
                s.push_str(&format!(" \u{00B7} paused at Step {step}"));
            }
            s
        }
        ModlistState::Installed => {
            let size = match entry.total_size_bytes {
                Some(bytes) => human_size(bytes),
                None => "\u{2014}".to_string(), // — (em dash) when size unknown
            };
            // "installed <rel>" tracks the install date; fall back to
            // last-touched if (defensively) the install date is missing.
            let when = entry
                .install_date
                .unwrap_or(entry.last_touched_date);
            format!(
                "{} mods \u{00B7} {} \u{00B7} installed {}",
                entry.mod_count,
                size,
                relative_time(when),
            )
        }
    }
}

/// The right-cluster button + Kebab. Two action sets keyed on state. The
/// Kebab items are Run-1 placeholders (`|| {}`); the button drives the
/// returned action.
fn render_action_cluster(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &ModlistEntry,
) -> ModlistCardActions {
    let mut action = ModlistCardActions::None;
    ui.spacing_mut().item_spacing.x = 6.0;

    match entry.state {
        ModlistState::InProgress => {
            // `right_to_left` lays widgets out trailing-edge first, so add the
            // Kebab first to keep on-screen order [resume] [···].
            let mut items = vec![
                KebabItem::new("Copy import code", || {}),
                KebabItem::new("Rename", || {}),
                KebabItem::danger("Delete", || {}),
            ];
            render_kebab(ui, palette, &entry.id, &mut items);

            if redesign_btn(
                ui,
                palette,
                "resume",
                BtnOpts {
                    small: true,
                    primary: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                action = ModlistCardActions::Resume;
            }
        }
        ModlistState::Installed => {
            let mut items = vec![
                KebabItem::new("Copy import code", || {}),
                KebabItem::new("Open install folder", || {}),
                KebabItem::new("Rename", || {}),
                KebabItem::new("Reinstall", || {}),
                KebabItem::danger("Delete", || {}),
            ];
            render_kebab(ui, palette, &entry.id, &mut items);

            // Renamed from the wireframe's `play` → `open` for v1 alpha
            // (SPEC §3.2 / HANDOFF M6): neutral (non-primary) small button.
            if redesign_btn(
                ui,
                palette,
                "open",
                BtnOpts {
                    small: true,
                    primary: false,
                    ..Default::default()
                },
            )
            .clicked()
            {
                action = ModlistCardActions::Open;
            }
        }
    }

    action
}

/// Human-readable byte size (e.g. "2.3 GB"). Only used for installed cards
/// once Phase 7 populates `total_size_bytes`; included now so the meta-line
/// shape is final.
fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;
    let b = bytes as f64;
    if b >= TB {
        format!("{:.1} TB", b / TB)
    } else if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::Game;
    use chrono::Utc;

    fn base_entry() -> ModlistEntry {
        ModlistEntry {
            id: "ABCDEFGHIJKL".to_string(),
            name: "Tactical EET 2026".to_string(),
            game: Game::EET,
            ..Default::default()
        }
    }

    #[test]
    fn in_progress_meta_includes_paused_step_when_present() {
        let mut e = base_entry();
        e.state = ModlistState::InProgress;
        e.mod_count = 9;
        e.component_count = 136;
        e.paused_at_step = Some(3);
        let m = meta_line(&e);
        assert!(m.starts_with("9 mods \u{00B7} 136 components \u{00B7} last touched "));
        assert!(m.ends_with(" \u{00B7} paused at Step 3"), "got: {m}");
    }

    #[test]
    fn in_progress_meta_omits_paused_step_when_none() {
        let mut e = base_entry();
        e.state = ModlistState::InProgress;
        e.mod_count = 9;
        e.component_count = 136;
        e.paused_at_step = None;
        let m = meta_line(&e);
        assert!(!m.contains("paused at Step"), "got: {m}");
        assert!(m.contains("last touched "));
    }

    #[test]
    fn installed_meta_renders_em_dash_when_size_unknown() {
        let mut e = base_entry();
        e.state = ModlistState::Installed;
        e.mod_count = 47;
        e.total_size_bytes = None;
        e.install_date = Some(Utc::now());
        let m = meta_line(&e);
        assert!(
            m.starts_with("47 mods \u{00B7} \u{2014} \u{00B7} installed "),
            "got: {m}"
        );
    }

    #[test]
    fn installed_meta_renders_human_size_when_known() {
        let mut e = base_entry();
        e.state = ModlistState::Installed;
        e.mod_count = 47;
        e.total_size_bytes = Some((2.3 * 1024.0 * 1024.0 * 1024.0) as u64);
        e.install_date = Some(Utc::now());
        let m = meta_line(&e);
        assert!(m.contains("2.3 GB"), "got: {m}");
    }

    #[test]
    fn human_size_buckets() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }
}
