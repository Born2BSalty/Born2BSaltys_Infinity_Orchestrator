// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::Game;
use crate::ui::create::state_create::{CreateAction, CreateScreenState, CreateStage};
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::settings::widgets::path_row;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_BUTTON_SMALL_PADDING_X_PX,
    REDESIGN_BUTTON_SMALL_PADDING_Y_PX, REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_HOME_CARD_LIST_GAP_PX,
    REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_SETTINGS_ROW_GAP_PX, REDESIGN_SUBFLOW_SECTION_GAP_PX,
    ThemePalette, redesign_border_strong, redesign_font_bold, redesign_input_bg,
    redesign_text_muted, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
) -> Option<CreateAction> {
    let mut action = None;

    redesign_box(ui, palette, Some("setup"), |ui| {
        render_name_and_game(ui, palette, state);
        ui.add_space(REDESIGN_HOME_CARD_LIST_GAP_PX);
        let _ = path_row::render(
            ui,
            palette,
            "Destination folder",
            &mut state.destination,
            None,
        );
    });

    ui.add_space(REDESIGN_SUBFLOW_SECTION_GAP_PX);
    ui.columns(2, |columns| {
        action = render_starting_card(
            &mut columns[0],
            palette,
            "New modlist from downloaded mods",
            "Scan your local mods folder, pick components, reorder, then install. Starts from an empty selection.",
            "start →",
            CreateAction::StartNewModlist,
        )
        .or(action);

        if render_starting_card(
            &mut columns[1],
            palette,
            "Import and modify another modlist",
            "Paste a share code. BIO downloads the mods, preselects components, applies the order, then drops you on Step 2 to review and adjust.",
            "paste share code →",
            CreateAction::PasteShareCode,
        )
        .is_some()
        {
            state.stage = CreateStage::ForkPaste;
        }
    });

    action
}

fn render_name_and_game(ui: &mut egui::Ui, palette: ThemePalette, state: &mut CreateScreenState) {
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            ui.label(
                egui::RichText::new("modlist name")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_text_primary(palette)),
            );
            egui::Frame::NONE
                .fill(redesign_input_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(REDESIGN_BORDER_RADIUS_PX)
                .inner_margin(egui::Margin::symmetric(
                    crate::ui::shared::redesign_tokens::redesign_i8_px(
                        REDESIGN_BUTTON_SMALL_PADDING_X_PX,
                    ),
                    crate::ui::shared::redesign_tokens::redesign_i8_px(
                        REDESIGN_BUTTON_SMALL_PADDING_Y_PX,
                    ),
                ))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.modlist_name)
                            .hint_text("e.g. Tactical EET 2026")
                            .text_color(redesign_text_primary(palette))
                            .frame(false),
                    );
                });
        });

        columns[1].vertical(|ui| {
            ui.label(
                egui::RichText::new("game")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_text_primary(palette)),
            );
            render_game_combo(ui, palette, &mut state.game);
        });
    });
}

fn render_game_combo(ui: &mut egui::Ui, palette: ThemePalette, game: &mut Game) {
    egui::Frame::NONE
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_BUTTON_SMALL_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_BUTTON_SMALL_PADDING_Y_PX),
        ))
        .show(ui, |ui| {
            egui::ComboBox::from_id_salt("create_game_selector")
                .selected_text(
                    egui::RichText::new(game_label(game)).color(redesign_text_primary(palette)),
                )
                .show_ui(ui, |ui| {
                    for option in [Game::EET, Game::BGEE, Game::BG2EE, Game::IWDEE] {
                        let label = game_label(&option);
                        ui.selectable_value(game, option, label);
                    }
                });
        });
}

fn render_starting_card(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    title: &str,
    description: &str,
    button: &str,
    button_action: CreateAction,
) -> Option<CreateAction> {
    let mut action = None;

    redesign_box(ui, palette, None, |ui| {
        ui.spacing_mut().item_spacing.y = REDESIGN_SETTINGS_ROW_GAP_PX;
        ui.label(
            egui::RichText::new(title)
                .family(redesign_font_bold())
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_primary(palette)),
        );
        ui.label(
            egui::RichText::new(description)
                .size(REDESIGN_HINT_FONT_SIZE_PX)
                .color(redesign_text_muted(palette)),
        );
        if redesign_btn(
            ui,
            palette,
            button,
            BtnOpts {
                primary: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            action = Some(button_action);
        }
    });

    action
}

const fn game_label(game: &Game) -> &'static str {
    match game {
        Game::BGEE => "BGEE",
        Game::BG2EE => "BG2EE",
        Game::IWDEE => "IWDEE",
        Game::EET => "EET",
    }
}
