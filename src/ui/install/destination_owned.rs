// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::destination_claim::{ClaimRefusal, DestinationClaim};
use crate::registry::model::ModlistRegistry;
use crate::ui::install::destination_not_empty::{
    WARN_BORDER, WARN_INK, paint_warning_triangle, warn_fill,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_error,
};

pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    claim: &DestinationClaim,
    registry: &ModlistRegistry,
) {
    match claim {
        DestinationClaim::Free | DestinationClaim::Adopt(_) => {}
        DestinationClaim::Replace(ids) => render_exact_warning(ui, ids, registry),
        DestinationClaim::Refused(ClaimRefusal::InsideAnother(id)) => {
            render_inside_block(ui, palette, id, registry);
        }
        DestinationClaim::Refused(ClaimRefusal::ContainsOthers(ids)) => {
            render_contains_block(ui, palette, ids, registry);
        }
        DestinationClaim::Refused(ClaimRefusal::OwnerIsInstalling(id)) => {
            render_installing_block(ui, palette, id, registry);
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

fn render_installing_block(
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
        header_row(ui, "Folder in use", style.ink);
        ui.add_space(4.0);
        body_label(
            ui,
            format!(
                "\"{name}\" is installing into this folder right now \u{2014} wait for it to finish or pick a different folder."
            ),
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
