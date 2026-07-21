// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::ModlistRegistry;
use crate::registry::operations::DestinationOwnership;
use crate::ui::install::destination_not_empty::{
    WARN_BORDER, WARN_INK, paint_warning_triangle, warn_fill,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_error,
};

#[must_use]
pub(crate) fn proceed_allowed(
    ownership: &DestinationOwnership,
    active_install_id: Option<&str>,
) -> bool {
    match ownership {
        DestinationOwnership::Free => true,
        DestinationOwnership::ExactOwners(ids) => {
            active_install_id.is_none_or(|active| ids.iter().all(|id| id != active))
        }
        DestinationOwnership::InsideOwner(_) | DestinationOwnership::ContainsOwners(_) => false,
    }
}

pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    ownership: &DestinationOwnership,
    registry: &ModlistRegistry,
) {
    match ownership {
        DestinationOwnership::Free => {}
        DestinationOwnership::ExactOwners(ids) => render_exact_warning(ui, ids, registry),
        DestinationOwnership::InsideOwner(id) => render_inside_block(ui, palette, id, registry),
        DestinationOwnership::ContainsOwners(ids) => {
            render_contains_block(ui, palette, ids, registry);
        }
    }
}

#[derive(Clone, Copy)]
struct BannerStyle {
    border: egui::Color32,
    fill: egui::Color32,
    ink: egui::Color32,
}

fn warning_style() -> BannerStyle {
    BannerStyle {
        border: WARN_BORDER,
        fill: warn_fill(),
        ink: WARN_INK,
    }
}

fn danger_style(palette: ThemePalette) -> BannerStyle {
    let error = redesign_error(palette);
    BannerStyle {
        border: error,
        fill: egui::Color32::from_rgba_unmultiplied(error.r(), error.g(), error.b(), 46),
        ink: WARN_INK,
    }
}

fn banner_frame(style: BannerStyle) -> egui::Frame {
    egui::Frame::default()
        .fill(style.fill)
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, style.border))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
}

fn header_row(ui: &mut egui::Ui, title: &str, ink: egui::Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
        paint_warning_triangle(ui.painter(), icon_rect.center(), ink);
        ui.label(
            egui::RichText::new(title)
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(ink),
        );
    });
}

fn body_label(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(
        egui::RichText::new(text)
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(egui::Color32::from_rgba_unmultiplied(
                0xff, 0xff, 0xff, 0xcc,
            )),
    );
}

fn render_exact_warning(ui: &mut egui::Ui, ids: &[String], registry: &ModlistRegistry) {
    ui.add_space(12.0);
    let title = if ids.len() == 1 {
        "Folder claimed by another modlist"
    } else {
        "Folder claimed by other modlists"
    };
    let style = warning_style();
    banner_frame(style).show(ui, |ui| {
        ui.set_width(ui.available_width());
        header_row(ui, title, style.ink);
        ui.add_space(4.0);
        for id in ids {
            let name = registry
                .find(id)
                .map_or_else(|| id.as_str(), |e| e.name.as_str());
            body_label(ui, format!("\"{name}\" will be removed from your list"));
        }
    });
}

fn render_inside_block(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    id: &str,
    registry: &ModlistRegistry,
) {
    ui.add_space(12.0);
    let name = registry.find(id).map_or_else(|| id, |e| e.name.as_str());
    let style = danger_style(palette);
    banner_frame(style).show(ui, |ui| {
        ui.set_width(ui.available_width());
        header_row(ui, "Folder is inside another modlist's folder", style.ink);
        ui.add_space(4.0);
        body_label(
            ui,
            format!("This folder is inside \"{name}\"'s install folder \u{2014} pick a different folder."),
        );
    });
}

fn render_contains_block(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    ids: &[String],
    registry: &ModlistRegistry,
) {
    ui.add_space(12.0);
    let names: Vec<&str> = ids
        .iter()
        .map(|id| {
            registry
                .find(id)
                .map_or_else(|| id.as_str(), |e| e.name.as_str())
        })
        .collect();
    let names_list = names
        .iter()
        .map(|n| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let style = danger_style(palette);
    banner_frame(style).show(ui, |ui| {
        ui.set_width(ui.available_width());
        header_row(ui, "Folder contains other modlists", style.ink);
        ui.add_space(4.0);
        body_label(
            ui,
            format!("This folder contains: {names_list} \u{2014} pick a different folder."),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};

    fn reg_with(id: &str, name: &str, dest: &str) -> ModlistRegistry {
        let mut r = ModlistRegistry::default();
        r.entries.push(ModlistEntry {
            id: id.to_string(),
            name: name.to_string(),
            game: Game::EET,
            destination_folder: dest.to_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        r
    }

    #[test]
    fn proceed_allowed_free_always_true() {
        assert!(proceed_allowed(&DestinationOwnership::Free, None));
        assert!(proceed_allowed(&DestinationOwnership::Free, Some("X")));
    }

    #[test]
    fn proceed_allowed_exact_no_active_install() {
        let o = DestinationOwnership::ExactOwners(vec!["A".to_string()]);
        assert!(proceed_allowed(&o, None));
    }

    #[test]
    fn proceed_allowed_exact_active_install_not_owner() {
        let o = DestinationOwnership::ExactOwners(vec!["A".to_string()]);
        assert!(proceed_allowed(&o, Some("B")));
    }

    #[test]
    fn proceed_blocked_exact_active_install_is_owner() {
        let o = DestinationOwnership::ExactOwners(vec!["A".to_string()]);
        assert!(!proceed_allowed(&o, Some("A")));
    }

    #[test]
    fn proceed_blocked_inside_owner() {
        let o = DestinationOwnership::InsideOwner("A".to_string());
        assert!(!proceed_allowed(&o, None));
        assert!(!proceed_allowed(&o, Some("B")));
    }

    #[test]
    fn proceed_blocked_contains_owners() {
        let o = DestinationOwnership::ContainsOwners(vec!["A".to_string()]);
        assert!(!proceed_allowed(&o, None));
        assert!(!proceed_allowed(&o, Some("B")));
    }

    #[test]
    fn exact_block_with_multiple_owners_mid_install_blocks_on_any() {
        let o = DestinationOwnership::ExactOwners(vec!["A".to_string(), "B".to_string()]);
        assert!(!proceed_allowed(&o, Some("A")));
        assert!(!proceed_allowed(&o, Some("B")));
        assert!(proceed_allowed(&o, Some("C")));
        assert!(proceed_allowed(&o, None));
    }

    #[test]
    fn name_resolution_falls_back_to_id_when_not_in_registry() {
        let reg = ModlistRegistry::default();
        let ids = ["GHOST0000000".to_string()];
        let names: Vec<&str> = ids
            .iter()
            .map(|id| {
                reg.find(id)
                    .map_or_else(|| id.as_str(), |e| e.name.as_str())
            })
            .collect();
        assert_eq!(names, ["GHOST0000000"]);
    }

    #[test]
    fn name_resolution_uses_registry_name_when_present() {
        let reg = reg_with("MOD0000000AA", "My Modlist", "/dest");
        let id = "MOD0000000AA".to_string();
        let name = reg
            .find(&id)
            .map_or_else(|| id.as_str(), |e| e.name.as_str());
        assert_eq!(name, "My Modlist");
    }
}
