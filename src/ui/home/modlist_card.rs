// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::kebab;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_KEBAB_MENU_ITEM_FONT_SIZE_PX,
    REDESIGN_KEBAB_MENU_ITEM_PADDING_X_PX, REDESIGN_KEBAB_MENU_ITEM_PADDING_Y_PX,
    REDESIGN_KEBAB_MENU_OFFSET_PX, REDESIGN_KEBAB_MENU_PADDING_PX,
    REDESIGN_KEBAB_MENU_SHADOW_OFFSET_PX, REDESIGN_KEBAB_MENU_WIDTH_PX,
    REDESIGN_MODLIST_CARD_ACTION_GAP_PX, REDESIGN_MODLIST_CARD_ACTION_WIDTH_PX,
    REDESIGN_MODLIST_CARD_META_FONT_SIZE_PX, REDESIGN_MODLIST_CARD_NAME_FONT_SIZE_PX,
    REDESIGN_MODLIST_CARD_PADDING_X_PX, REDESIGN_MODLIST_CARD_PADDING_Y_PX,
    REDESIGN_MODLIST_CARD_TEXT_GAP_PX, ThemePalette, redesign_border_strong, redesign_font_bold,
    redesign_font_light, redesign_font_medium, redesign_hover_overlay, redesign_shadow,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModlistCardState {
    InProgress,
    Installed,
}

#[derive(Debug, Clone, Copy)]
pub struct ModlistCardView<'a> {
    pub name: &'a str,
    pub meta_line: &'a str,
    pub state: ModlistCardState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModlistCardAction {
    Resume,
    Open,
    CopyImportCode,
    OpenInstallFolder,
    Rename,
    Delete,
    Reinstall,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    view: &ModlistCardView<'_>,
) -> Option<ModlistCardAction> {
    let mut action = None;

    egui::Frame::NONE
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            REDESIGN_MODLIST_CARD_PADDING_X_PX as i8,
            REDESIGN_MODLIST_CARD_PADDING_Y_PX as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = REDESIGN_MODLIST_CARD_ACTION_GAP_PX;

                let left_width =
                    (ui.available_width() - REDESIGN_MODLIST_CARD_ACTION_WIDTH_PX).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(left_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = REDESIGN_MODLIST_CARD_TEXT_GAP_PX;
                        ui.label(
                            egui::RichText::new(view.name)
                                .family(redesign_font_bold())
                                .size(REDESIGN_MODLIST_CARD_NAME_FONT_SIZE_PX)
                                .color(redesign_text_primary(palette)),
                        );
                        ui.label(
                            egui::RichText::new(view.meta_line)
                                .family(redesign_font_light())
                                .size(REDESIGN_MODLIST_CARD_META_FONT_SIZE_PX)
                                .color(redesign_text_faint(palette)),
                        );
                    },
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let kebab_response = kebab::render(ui, palette);
                    if kebab_response.clicked() {
                        ui.memory_mut(|mem| mem.toggle_popup(kebab_popup_id(view.name)));
                    }
                    if let Some(menu_action) =
                        render_kebab_menu(ui, palette, view, kebab_response.rect)
                    {
                        action = Some(menu_action);
                    }

                    let (label, primary, clicked_action) = match view.state {
                        ModlistCardState::InProgress => ("resume", true, ModlistCardAction::Resume),
                        ModlistCardState::Installed => ("play", false, ModlistCardAction::Open),
                    };
                    if redesign_btn(
                        ui,
                        palette,
                        label,
                        BtnOpts {
                            primary,
                            small: true,
                            ..Default::default()
                        },
                    )
                    .clicked()
                    {
                        action = Some(clicked_action);
                    }
                });
            });
        });

    action
}

fn kebab_popup_id(name: &str) -> egui::Id {
    egui::Id::new(("home_modlist_card_kebab", name))
}

fn render_kebab_menu(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    view: &ModlistCardView<'_>,
    anchor_rect: egui::Rect,
) -> Option<ModlistCardAction> {
    let popup_id = kebab_popup_id(view.name);
    if !ui.memory(|mem| mem.is_popup_open(popup_id)) {
        return None;
    }

    let mut selected = None;
    let pos = egui::pos2(
        anchor_rect.right() - REDESIGN_KEBAB_MENU_WIDTH_PX,
        anchor_rect.bottom() + REDESIGN_KEBAB_MENU_OFFSET_PX,
    );
    let area_response = egui::Area::new(popup_id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(REDESIGN_BORDER_RADIUS_PX)
                .shadow(egui::Shadow {
                    offset: [
                        REDESIGN_KEBAB_MENU_SHADOW_OFFSET_PX as i8,
                        REDESIGN_KEBAB_MENU_SHADOW_OFFSET_PX as i8,
                    ],
                    blur: 0,
                    spread: 0,
                    color: redesign_shadow(palette),
                })
                .inner_margin(egui::Margin::same(REDESIGN_KEBAB_MENU_PADDING_PX as i8))
                .show(ui, |ui| {
                    ui.set_min_width(REDESIGN_KEBAB_MENU_WIDTH_PX);
                    for (label, action) in menu_items(view.state) {
                        if menu_row(ui, palette, label).clicked() {
                            selected = Some(*action);
                        }
                    }
                });
        })
        .response;

    let clicked_outside =
        ui.input(|input| input.pointer.any_pressed()) && !area_response.contains_pointer();
    if selected.is_some()
        || clicked_outside
        || ui.input(|input| input.key_pressed(egui::Key::Escape))
    {
        ui.memory_mut(|mem| mem.close_popup());
    }

    selected
}

fn menu_items(state: ModlistCardState) -> &'static [(&'static str, ModlistCardAction)] {
    match state {
        ModlistCardState::InProgress => &[
            ("Copy import code", ModlistCardAction::CopyImportCode),
            ("Rename", ModlistCardAction::Rename),
            ("Delete", ModlistCardAction::Delete),
        ],
        ModlistCardState::Installed => &[
            ("Copy import code", ModlistCardAction::CopyImportCode),
            ("Open install folder", ModlistCardAction::OpenInstallFolder),
            ("Rename", ModlistCardAction::Rename),
            ("Reinstall", ModlistCardAction::Reinstall),
            ("Delete", ModlistCardAction::Delete),
        ],
    }
}

fn menu_row(ui: &mut egui::Ui, palette: ThemePalette, label: &str) -> egui::Response {
    let height =
        REDESIGN_KEBAB_MENU_ITEM_FONT_SIZE_PX + REDESIGN_KEBAB_MENU_ITEM_PADDING_Y_PX * 2.0;
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            REDESIGN_BORDER_RADIUS_PX,
            redesign_hover_overlay(palette),
        );
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(REDESIGN_KEBAB_MENU_ITEM_PADDING_X_PX, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::new(
            REDESIGN_KEBAB_MENU_ITEM_FONT_SIZE_PX,
            redesign_font_medium(),
        ),
        redesign_text_primary(palette),
    );
    response
}
