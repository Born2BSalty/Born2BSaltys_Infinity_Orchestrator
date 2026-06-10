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
    redesign_input_bg, redesign_shell_bg, redesign_text_faint, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModlistCardActions {
    #[default]
    None,
    Resume,
    Open,
    CopyImportCode,
    OpenInstallFolder,
    Reinstall,
    Delete,
    /// Signals that the kebab "Rename" item was picked; the caller opens the editor.
    Rename,
    /// Signals that the inline rename editor should commit the buffered name.
    SaveRename,
    /// Signals that the inline rename editor was dismissed without saving.
    CancelRename,
}

/// Renders a single modlist card.
///
/// Pass `rename_buf` as `Some(&mut buf)` when this card is the active rename
/// target; the card's title row becomes an inline editor bound to `buf`.
/// Pass `None` for normal rendering.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &ModlistEntry,
    rename_buf: Option<&mut String>,
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
            if let Some(buf) = rename_buf {
                render_rename_row(ui, palette, entry, buf, &mut action);
            } else {
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
                    action = render_action_cluster(ui, palette, entry);
                });
            }
        });
    });

    action
}

/// Renders the title row in inline-rename editing mode.
///
/// When in rename mode the right action cluster (kebab + resume/open) is
/// suppressed to avoid crowding the row.
fn render_rename_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    entry: &ModlistEntry,
    buf: &mut String,
    action: &mut ModlistCardActions,
) {
    let item_spacing = 6.0_f32;
    let btn_pad_x = 10.0_f32;
    let btn_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));

    let save_text_w = ui
        .painter()
        .layout_no_wrap("save".to_string(), btn_font.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let cancel_text_w = ui
        .painter()
        .layout_no_wrap("cancel".to_string(), btn_font, egui::Color32::WHITE)
        .size()
        .x;

    let save_btn_w = btn_pad_x.mul_add(2.0, save_text_w);
    let cancel_btn_w = btn_pad_x.mul_add(2.0, cancel_text_w);

    let available_w = ui.available_width();
    let reserved = item_spacing.mul_add(2.0, save_btn_w + cancel_btn_w);
    let field_w = (available_w - reserved).max(0.0);

    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = item_spacing;

        let edit_id = egui::Id::new(("home_card_rename_edit",)).with(&entry.id);
        let response = ui.add(
            egui::TextEdit::singleline(buf)
                .id(edit_id)
                .desired_width(field_w)
                .font(egui::FontId::new(
                    13.0,
                    egui::FontFamily::Name("poppins_medium".into()),
                ))
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette))
                .margin(egui::Margin::symmetric(8, 4)),
        );

        let focus_marker = edit_id.with("focused_once");
        let already_focused = ui
            .memory(|m| m.data.get_temp::<bool>(focus_marker))
            .unwrap_or(false);
        if !already_focused {
            response.request_focus();
            ui.memory_mut(|m| m.data.insert_temp(focus_marker, true));
        }

        let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

        let save_clicked = redesign_btn(
            ui,
            palette,
            "save",
            BtnOpts {
                primary: true,
                small: true,
                ..Default::default()
            },
        )
        .clicked();
        let cancel_clicked = redesign_btn(
            ui,
            palette,
            "cancel",
            BtnOpts {
                small: true,
                ..Default::default()
            },
        )
        .clicked();

        if save_clicked || enter_pressed {
            *action = ModlistCardActions::SaveRename;
        } else if cancel_clicked || escape_pressed {
            *action = ModlistCardActions::CancelRename;
        }
    });
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
) -> ModlistCardActions {
    use std::cell::Cell;

    let picked: Cell<ModlistCardActions> = Cell::new(ModlistCardActions::None);
    ui.spacing_mut().item_spacing.x = 6.0;

    match entry.state {
        ModlistState::InProgress => {
            let mut items = vec![
                KebabItem::new("Copy import code", || {
                    picked.set(ModlistCardActions::CopyImportCode);
                }),
                KebabItem::new("Rename", || {
                    picked.set(ModlistCardActions::Rename);
                }),
                KebabItem::danger("Delete", || picked.set(ModlistCardActions::Delete)),
            ];
            let kebab_h = redesign_btn_height(ui, true);
            render_kebab(ui, palette, &entry.id, &mut items, kebab_h);
            drop(items);

            if redesign_btn(
                ui,
                palette,
                "resume",
                BtnOpts {
                    small: true,
                    primary: true,
                    no_shadow: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                picked.set(ModlistCardActions::Resume);
            }
        }
        ModlistState::Installed => {
            let mut items = vec![
                KebabItem::new("Copy import code", || {
                    picked.set(ModlistCardActions::CopyImportCode);
                }),
                KebabItem::new("Open install folder", || {
                    picked.set(ModlistCardActions::OpenInstallFolder);
                }),
                KebabItem::new("Rename", || {
                    picked.set(ModlistCardActions::Rename);
                }),
                KebabItem::new("Reinstall", || picked.set(ModlistCardActions::Reinstall)),
                KebabItem::danger("Delete", || picked.set(ModlistCardActions::Delete)),
            ];
            let kebab_h = redesign_btn_height(ui, true);
            render_kebab(ui, palette, &entry.id, &mut items, kebab_h);
            drop(items);

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
}
