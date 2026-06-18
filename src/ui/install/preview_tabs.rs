// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::state_install::PreviewTab;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong, redesign_chrome_bg,
    redesign_shell_bg, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};
use crate::ui::shared::tab_open_seam::paint_active_tab_seam_cover;

pub(crate) fn render_tab_strip(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    active: &mut PreviewTab,
) -> Option<egui::Rect> {
    let mut active_tab_rect: Option<egui::Rect> = None;

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for tab in PreviewTab::ALL {
            let is_active = tab == *active;
            let rect = render_one_tab(ui, palette, tab, is_active);
            if is_active {
                active_tab_rect = Some(rect);
            }
            if tab_clicked(ui, rect) {
                *active = tab;
            }
        }
    });

    let strip_bottom = ui.cursor().top();
    active_tab_rect.filter(|r| {
        let item_gap = ui.spacing().item_spacing.y;
        r.bottom() + item_gap >= strip_bottom - 2.0
    })
}

pub(crate) fn paint_preview_seam_cover(
    painter: &egui::Painter,
    palette: ThemePalette,
    active_tab_rect: egui::Rect,
    panel_top_y: f32,
) {
    paint_active_tab_seam_cover(painter, palette, active_tab_rect, panel_top_y);
}

fn render_one_tab(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tab: PreviewTab,
    is_active: bool,
) -> egui::Rect {
    let pad_x = 14.0;
    let pad_y = 6.0;
    let font = egui::FontId::new(
        14.0,
        egui::FontFamily::Name(if is_active {
            "poppins_bold".into()
        } else {
            "poppins_light".into()
        }),
    );
    let text_color = if is_active {
        redesign_text_primary(palette)
    } else {
        redesign_text_muted(palette)
    };
    let label = tab.display_label();
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius {
            nw: 4,
            ne: 4,
            sw: 0,
            se: 0,
        };
        let fill = if is_active {
            redesign_shell_bg(palette)
        } else {
            redesign_chrome_bg(palette)
        };
        painter.rect_filled(rect, radius, fill);
        let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
        painter.line_segment([rect.left_top(), rect.right_top()], stroke);
        painter.line_segment([rect.left_top(), rect.left_bottom()], stroke);
        painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }

    rect
}

fn tab_clicked(ui: &egui::Ui, rect: egui::Rect) -> bool {
    ui.input(|i| {
        i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
    })
}

pub(crate) fn render_tab_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tab: PreviewTab,
    preview: &ModlistSharePreview,
) {
    let text = tab_text(tab, preview);
    let trimmed_empty = text.trim().is_empty();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if trimmed_empty {
                ui.label(
                    egui::RichText::new("(none in this share code)")
                        .size(13.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_text_faint(palette)),
                );
            } else {
                ui.label(
                    egui::RichText::new(text)
                        .size(13.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_text_primary(palette)),
                );
            }
        });
}

fn tab_text(tab: PreviewTab, p: &ModlistSharePreview) -> String {
    match tab {
        PreviewTab::Summary => summary_text(p),
        PreviewTab::BgeeWeidu => p.bgee_log_text.clone(),
        PreviewTab::Bg2eeWeidu => p.bg2ee_log_text.clone(),
        PreviewTab::UserDownloads => p.source_overrides_text.clone(),
        PreviewTab::InstalledRefs => p.installed_refs_text.clone(),
        PreviewTab::ModConfigs => p.mod_configs_text.clone(),
    }
}

fn summary_text(p: &ModlistSharePreview) -> String {
    let yn = |b: bool| if b { "Yes" } else { "No" };
    let modlist_name = p
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Shared modlist");
    format!(
        "BIO Modlist Import Preview\n\n\
         Modlist: {modlist_name}\n\
         BIO version: {bio}\n\
         Game install: {game}\n\
         Install mode: {mode}\n\n\
         WeiDU Logs\n\
         BGEE: {bgee} entries\n\
         BG2EE: {bg2ee} entries\n\n\
         Included Data\n\
         Source overrides: {src}\n\
         Installed refs / pins: {refs}\n\
         Mod config files: {cfg}\n\n\
         What Import Will Do\n\
         - Set game/install mode from this share code.\n\
         - Write imported WeiDU logs.\n\
         - Import source overrides if included.\n\
         - Import installed refs/pins if included.\n\
         - Store pending mod config files if included.\n\
         - Keep local game, mods, archive, and backup paths unchanged.",
        bio = p.bio_version,
        game = p.game_install,
        mode = p.install_mode,
        bgee = p.bgee_entries,
        bg2ee = p.bg2ee_entries,
        src = yn(p.has_source_overrides),
        refs = yn(p.has_installed_refs),
        cfg = p.mod_config_count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_preview() -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "0.1.0-test".to_string(),
            game_install: "EET".to_string(),
            install_mode: "start_from_weidu_logs_then_review_edit".to_string(),
            bgee_entries: 21,
            bg2ee_entries: 115,
            has_source_overrides: true,
            has_installed_refs: true,
            bgee_log_text: "// BGEE log\n~A\\A.TP2~ #0 #0 // X".to_string(),
            bg2ee_log_text: String::new(),
            source_overrides_text: "[[mods]]".to_string(),
            installed_refs_text: "[refs]".to_string(),
            mod_config_count: 4,
            mod_configs_text: "a | b | c".to_string(),
            allow_auto_install: true,
            name: None,
            author: None,
            forked_from: Vec::new(),
        }
    }

    #[test]
    fn summary_is_populated_from_preview() {
        let mut p = sample_preview();
        p.name = Some("Tactical EET 2026".to_string());
        p.author = Some("@hidden".to_string());
        p.forked_from = vec![crate::app::modlist_share::ForkAncestor {
            name: "Root".to_string(),
            author: "@root".to_string(),
        }];

        let s = summary_text(&p);
        assert!(s.starts_with("BIO Modlist Import Preview"));
        assert!(s.contains("Modlist: Tactical EET 2026"));
        assert!(s.contains("Game install: EET"));
        assert!(s.contains("BGEE: 21 entries"));
        assert!(s.contains("BG2EE: 115 entries"));
        assert!(s.contains("Source overrides: Yes"));
        assert!(s.contains("Mod config files: 4"));
        assert!(!s.contains("Step 1"));
        assert!(
            !s.contains("@hidden") && !s.contains("Root"),
            "summary shows the modlist name, not author/fork details"
        );
    }

    #[test]
    fn verbatim_tabs_pass_through_preview_sections() {
        let p = sample_preview();
        assert_eq!(tab_text(PreviewTab::BgeeWeidu, &p), p.bgee_log_text);
        assert_eq!(
            tab_text(PreviewTab::UserDownloads, &p),
            p.source_overrides_text
        );
        assert_eq!(
            tab_text(PreviewTab::InstalledRefs, &p),
            p.installed_refs_text
        );
        assert_eq!(tab_text(PreviewTab::ModConfigs, &p), p.mod_configs_text);
        assert!(tab_text(PreviewTab::Bg2eeWeidu, &p).is_empty());
    }
}
