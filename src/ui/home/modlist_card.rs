// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fmt::Write as _;

use eframe::egui;

use crate::registry::model::{ModlistEntry, ModlistState};
use crate::ui::orchestrator::widgets::{
    BtnOpts, KebabItem, redesign_btn, redesign_btn_height, render_kebab,
};
use crate::ui::shared::format_relative::relative_time;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModlistCardActions {
    #[default]
    None,
    Resume,
    Open,
    ShareModlist,
    OpenInstallFolder,
    Reinstall,
    Delete,
    EditModlist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardMenu {
    Full,
    DraftPicker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardMenuEntry {
    pub label: &'static str,
    pub action: ModlistCardActions,
    pub danger: bool,
}

#[must_use]
pub fn menu_entries(state: ModlistState, menu: CardMenu) -> Vec<CardMenuEntry> {
    match (state, menu) {
        (ModlistState::InProgress, CardMenu::Full) => vec![
            CardMenuEntry {
                label: "Share this modlist",
                action: ModlistCardActions::ShareModlist,
                danger: false,
            },
            CardMenuEntry {
                label: "Edit modlist",
                action: ModlistCardActions::EditModlist,
                danger: false,
            },
            CardMenuEntry {
                label: "Delete",
                action: ModlistCardActions::Delete,
                danger: true,
            },
        ],
        (ModlistState::InProgress, CardMenu::DraftPicker) => vec![CardMenuEntry {
            label: "Delete",
            action: ModlistCardActions::Delete,
            danger: true,
        }],
        (ModlistState::Installed, CardMenu::Full) => vec![
            CardMenuEntry {
                label: "Share this modlist",
                action: ModlistCardActions::ShareModlist,
                danger: false,
            },
            CardMenuEntry {
                label: "Open install folder",
                action: ModlistCardActions::OpenInstallFolder,
                danger: false,
            },
            CardMenuEntry {
                label: "Edit modlist",
                action: ModlistCardActions::EditModlist,
                danger: false,
            },
            CardMenuEntry {
                label: "Reinstall",
                action: ModlistCardActions::Reinstall,
                danger: false,
            },
            CardMenuEntry {
                label: "Delete",
                action: ModlistCardActions::Delete,
                danger: true,
            },
        ],
        (ModlistState::Installed, CardMenu::DraftPicker) => vec![
            CardMenuEntry {
                label: "Open install folder",
                action: ModlistCardActions::OpenInstallFolder,
                danger: false,
            },
            CardMenuEntry {
                label: "Reinstall",
                action: ModlistCardActions::Reinstall,
                danger: false,
            },
            CardMenuEntry {
                label: "Delete",
                action: ModlistCardActions::Delete,
                danger: true,
            },
        ],
    }
}

#[must_use]
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &ModlistEntry,
    menu: CardMenu,
) -> ModlistCardActions {
    let mut action = ModlistCardActions::None;

    let chassis = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 12,
            right: 12,
            top: 10,
            bottom: 10,
        });

    chassis.show(ui, |ui| {
        ui.horizontal(|ui| {
            let full_w = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(full_w, 40.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        egui::RichText::new(&entry.name)
                            .size(13.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
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

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                action = render_action_cluster(ui, palette, entry, menu);
            });
        });
    });

    action
}

pub fn meta_line(entry: &ModlistEntry) -> String {
    match entry.state {
        ModlistState::InProgress => {
            let mut s = format!(
                "{} mods \u{00B7} {} components \u{00B7} last touched {}",
                entry.mod_count,
                entry.component_count,
                relative_time(entry.last_touched_date),
            );
            if let Some(step) = entry.paused_at_step {
                let _ = write!(s, " \u{00B7} paused at Step {step}");
            }
            s
        }
        ModlistState::Installed => {
            let size = entry
                .total_size_bytes
                .map_or_else(|| "\u{2014}".to_string(), human_size);
            let when = entry.install_date.unwrap_or(entry.last_touched_date);
            format!(
                "{} mods \u{00B7} {} \u{00B7} installed {}",
                entry.mod_count,
                size,
                relative_time(when),
            )
        }
    }
}

fn render_action_cluster(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &ModlistEntry,
    menu: CardMenu,
) -> ModlistCardActions {
    use std::cell::Cell;

    let picked: Cell<ModlistCardActions> = Cell::new(ModlistCardActions::None);
    ui.spacing_mut().item_spacing.x = 6.0;

    let entries = menu_entries(entry.state, menu);
    let picked_ref = &picked;
    let mut items: Vec<KebabItem<'_>> = entries
        .into_iter()
        .map(|menu_entry| {
            let action = menu_entry.action;
            if menu_entry.danger {
                KebabItem::danger(menu_entry.label, move || picked_ref.set(action))
            } else {
                KebabItem::new(menu_entry.label, move || picked_ref.set(action))
            }
        })
        .collect();
    let kebab_h = redesign_btn_height(ui, true);
    render_kebab(ui, palette, &entry.id, &mut items, kebab_h);
    drop(items);

    match entry.state {
        ModlistState::InProgress => {
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
                picked.set(ModlistCardActions::Resume);
            }
        }
        ModlistState::Installed => {
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
                picked.set(ModlistCardActions::Open);
            }
        }
    }

    picked.into_inner()
}

fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    if bytes >= TB {
        scaled_size(bytes, TB, "TB")
    } else if bytes >= GB {
        scaled_size(bytes, GB, "GB")
    } else if bytes >= MB {
        scaled_size(bytes, MB, "MB")
    } else if bytes >= KB {
        scaled_size(bytes, KB, "KB")
    } else {
        format!("{bytes} B")
    }
}

fn scaled_size(bytes: u64, unit: u64, suffix: &str) -> String {
    let tenths = (u128::from(bytes) * 10 + u128::from(unit) / 2) / u128::from(unit);
    let whole = tenths / 10;
    let decimal = tenths % 10;
    format!("{whole}.{decimal} {suffix}")
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
        e.total_size_bytes = Some(2_469_396_070);
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

    #[test]
    fn full_menu_in_progress_keeps_share_edit_delete() {
        let entries = menu_entries(ModlistState::InProgress, CardMenu::Full);
        let labels: Vec<&str> = entries.iter().map(|e| e.label).collect();
        assert_eq!(labels, ["Share this modlist", "Edit modlist", "Delete"]);
        assert!(entries.last().unwrap().danger);
        assert_eq!(entries[0].action, ModlistCardActions::ShareModlist);
        assert_eq!(entries[1].action, ModlistCardActions::EditModlist);
        assert_eq!(entries[2].action, ModlistCardActions::Delete);
    }

    #[test]
    fn full_menu_installed_keeps_all_five() {
        let entries = menu_entries(ModlistState::Installed, CardMenu::Full);
        let labels: Vec<&str> = entries.iter().map(|e| e.label).collect();
        assert_eq!(
            labels,
            [
                "Share this modlist",
                "Open install folder",
                "Edit modlist",
                "Reinstall",
                "Delete"
            ]
        );
    }

    #[test]
    fn draft_picker_in_progress_is_delete_only() {
        let entries = menu_entries(ModlistState::InProgress, CardMenu::DraftPicker);
        let labels: Vec<&str> = entries.iter().map(|e| e.label).collect();
        assert_eq!(labels, ["Delete"]);
        assert!(entries[0].danger);
    }

    #[test]
    fn draft_picker_installed_hides_share_and_edit() {
        let entries = menu_entries(ModlistState::Installed, CardMenu::DraftPicker);
        let labels: Vec<&str> = entries.iter().map(|e| e.label).collect();
        assert_eq!(labels, ["Open install folder", "Reinstall", "Delete"]);
    }
}
