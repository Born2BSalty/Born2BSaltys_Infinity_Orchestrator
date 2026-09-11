// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::{Game, ModlistRegistry};
use crate::registry::operations::{DestinationOwnership, classify_destination};
use crate::ui::create::state_create::CreateScreenState;
use crate::ui::install::sub_flow_footer::{self, PrimaryBtn};
use crate::ui::install::{destination_not_empty, destination_owned};
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, redesign_box, redesign_btn, redesign_text_input, render_screen_title,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong, redesign_error,
    redesign_input_bg, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChooseOutcome {
    #[default]
    Stay,
    StartScratch,
    OpenLoadDraft,
}

const GAME_OPTIONS: [Game; 4] = [Game::EET, Game::BGEE, Game::BG2EE, Game::IWDEE];

const FORM_ROW_H_PX: f32 = 30.0;

const RIGHT_COL_W_PX: f32 = 96.0;

const FORM_INPUT_MARGIN: egui::Margin = egui::Margin {
    left: 12,
    right: 12,
    top: 8,
    bottom: 8,
};

const FORM_ROW_GAP_PX: f32 = 8.0;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
    destination_prep_running: bool,
    registry: &ModlistRegistry,
    active_install_id: Option<&str>,
) -> ChooseOutcome {
    let mut outcome = ChooseOutcome::Stay;
    let mut ownership = DestinationOwnership::Free;

    let body_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                render_body(ui, palette, state, &mut outcome, registry, &mut ownership);
            });
    });

    let proceed_ok = destination_owned::proceed_allowed(&ownership, active_install_id);

    let footer = sub_flow_footer::render(
        ui,
        palette,
        None::<sub_flow_footer::BackBtn<'_>>,
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        None,
        None,
        PrimaryBtn {
            label: if destination_prep_running {
                "Preparing"
            } else {
                "Start"
            },
            disabled: destination_prep_running || !proceed_ok,
        },
    );
    if footer == sub_flow_footer::FooterClick::Primary {
        outcome = ChooseOutcome::StartScratch;
    }

    outcome
}

fn render_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
    outcome: &mut ChooseOutcome,
    registry: &ModlistRegistry,
    ownership: &mut DestinationOwnership,
) {
    render_title_row(ui, palette, outcome);
    render_setup_box(ui, palette, state, registry, ownership);
}

fn render_title_row(ui: &mut egui::Ui, palette: ThemePalette, outcome: &mut ChooseOutcome) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let title_w = (ui.available_width() - 120.0).max(200.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_w, 0.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                render_screen_title(
                    ui,
                    palette,
                    "Create your own modlist",
                    Some("name your modlist, set destination + mods paths"),
                );
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if redesign_btn(
                ui,
                palette,
                "load draft",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                *outcome = ChooseOutcome::OpenLoadDraft;
            }
        });
    });
}

fn render_setup_box(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
    registry: &ModlistRegistry,
    ownership: &mut DestinationOwnership,
) {
    redesign_box(ui, palette, None, |ui| {
        ui.spacing_mut().item_spacing.y = 14.0;

        let input_box_h = ui
            .horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = FORM_ROW_GAP_PX;

                let name_w = (ui.available_width() - RIGHT_COL_W_PX - FORM_ROW_GAP_PX).max(160.0);

                let input_box_h = ui
                    .allocate_ui_with_layout(
                        egui::vec2(name_w, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            field_label(ui, palette, "modlist name");
                            ui.add_space(4.0);
                            let resp = redesign_text_input(
                                ui,
                                palette,
                                InputOpts {
                                    edit: egui::TextEdit::singleline(&mut state.modlist_name)
                                        .font(egui::FontId::new(
                                            14.0,
                                            egui::FontFamily::Name("poppins_light".into()),
                                        ))
                                        .hint_text(
                                            egui::RichText::new("e.g. Tactical EET 2026")
                                                .family(egui::FontFamily::Name(
                                                    "poppins_light".into(),
                                                ))
                                                .color(redesign_text_faint(palette)),
                                        )
                                        .text_color(redesign_text_primary(palette))
                                        .background_color(redesign_input_bg(palette))
                                        .margin(FORM_INPUT_MARGIN),
                                    margin: FORM_INPUT_MARGIN,
                                    size: egui::vec2(ui.available_width(), FORM_ROW_H_PX),
                                    border: None,
                                },
                            );
                            resp.rect.height()
                                + f32::from(FORM_INPUT_MARGIN.top)
                                + f32::from(FORM_INPUT_MARGIN.bottom)
                        },
                    )
                    .inner;

                ui.allocate_ui_with_layout(
                    egui::vec2(RIGHT_COL_W_PX, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        field_label(ui, palette, "game");
                        ui.add_space(4.0);
                        game_combo(ui, palette, &mut state.game, input_box_h);
                    },
                );

                input_box_h
            })
            .inner;

        *ownership = classify_destination(&state.destination, registry);
        let hard_block = matches!(
            *ownership,
            DestinationOwnership::InsideOwner(_) | DestinationOwnership::ContainsOwners(_)
        );

        let dest_changed = folder_input(
            ui,
            palette,
            "destination folder",
            "D:\\BG2EE_install_test",
            &mut state.destination,
            input_box_h,
            hard_block,
        );
        if dest_changed {
            state.destination_choice = None;
        }

        if !matches!(*ownership, DestinationOwnership::Free) {
            destination_owned::render(ui, palette, ownership, registry);
        }

        if !hard_block
            && destination_is_non_empty(&state.destination)
            && let Some(picked) =
                destination_not_empty::render(ui, palette, state.destination_choice)
        {
            state.destination_choice = Some(picked);
        }
    });
}

fn field_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
}

fn game_combo(ui: &mut egui::Ui, palette: ThemePalette, game: &mut Game, box_h: f32) {
    let mut selected = *game;

    let combo_w = ui.available_width();
    let combo_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let content_h = ui
        .fonts(|f| f.row_height(&combo_font))
        .max(ui.spacing().icon_width);
    let pad_y = ((box_h - content_h) / 2.0).max(0.0);

    let fill = redesign_input_bg(palette);
    let v = ui.visuals_mut();
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
    }
    ui.spacing_mut().button_padding = egui::vec2(10.0, pad_y);

    egui::ComboBox::from_id_salt("create_game_combo")
        .width(combo_w)
        .selected_text(
            egui::RichText::new(game_label(selected))
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        )
        .show_ui(ui, |ui| {
            for option in GAME_OPTIONS {
                ui.selectable_value(
                    &mut selected,
                    option,
                    egui::RichText::new(game_label(option))
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                );
            }
        });

    if selected != *game {
        *game = selected;
    }
}

const fn game_label(game: Game) -> &'static str {
    game.to_legacy_string()
}

fn destination_is_non_empty(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    std::fs::read_dir(trimmed).is_ok_and(|mut entries| entries.next().is_some())
}

fn folder_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    placeholder: &str,
    value: &mut String,
    box_h: f32,
    error: bool,
) -> bool {
    let mut changed = false;

    let border = if error {
        Some(redesign_error(palette))
    } else {
        None
    };

    ui.label(
        egui::RichText::new(label)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = FORM_ROW_GAP_PX;

        let reserved = RIGHT_COL_W_PX + FORM_ROW_GAP_PX;
        let edit_width = (ui.available_width() - reserved).max(120.0);

        let pre = value.clone();
        let response = redesign_text_input(
            ui,
            palette,
            InputOpts {
                edit: egui::TextEdit::singleline(value)
                    .font(egui::FontId::new(
                        12.0,
                        egui::FontFamily::Name("firacode_nerd".into()),
                    ))
                    .hint_text(
                        egui::RichText::new(placeholder)
                            .family(egui::FontFamily::Name("firacode_nerd".into()))
                            .color(redesign_text_faint(palette)),
                    )
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .vertical_align(egui::Align::Center)
                    .margin(FORM_INPUT_MARGIN),
                margin: FORM_INPUT_MARGIN,
                size: egui::vec2(edit_width, box_h),
                border,
            },
        );
        if response.changed() || *value != pre {
            changed = true;
        }

        if ui
            .add_sized(
                egui::vec2(RIGHT_COL_W_PX, box_h),
                egui::Button::new(
                    egui::RichText::new("browse\u{2026}")
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                )
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                )),
            )
            .clicked()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            let s = path.to_string_lossy().to_string();
            if s != *value {
                *value = s;
                changed = true;
            }
        }
    });

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_f32_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn game_options_are_wireframe_order_eet_first() {
        assert_eq!(
            GAME_OPTIONS,
            [Game::EET, Game::BGEE, Game::BG2EE, Game::IWDEE]
        );
        assert_eq!(GAME_OPTIONS[0], Game::EET);
    }

    #[test]
    fn game_labels_are_bare_enum_strings() {
        assert_eq!(game_label(Game::EET), "EET");
        assert_eq!(game_label(Game::BGEE), "BGEE");
        assert_eq!(game_label(Game::BG2EE), "BG2EE");
        assert_eq!(game_label(Game::IWDEE), "IWDEE");
    }

    #[test]
    fn non_empty_predicate_matches_stage_paste_semantics() {
        assert!(!destination_is_non_empty(""));
        assert!(!destination_is_non_empty("   "));
        let dir =
            std::env::temp_dir().join(format!("bio_create_choose_nonempty_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("f.txt"), b"x").unwrap();
        assert!(destination_is_non_empty(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn choose_outcome_default_is_stay() {
        assert_eq!(ChooseOutcome::default(), ChooseOutcome::Stay);
    }

    #[test]
    fn shared_form_chassis_constants_are_the_tuned_knob() {
        assert_f32_close(FORM_ROW_H_PX, 30.0);
        assert_f32_close(RIGHT_COL_W_PX, 96.0);
        assert_eq!(FORM_INPUT_MARGIN.left, 12);
        assert_eq!(FORM_INPUT_MARGIN.right, 12);
        assert_eq!(FORM_INPUT_MARGIN.top, 8);
        assert_eq!(FORM_INPUT_MARGIN.bottom, 8);
    }

    #[test]
    fn shared_form_row_gap_makes_inputs_equal_width() {
        assert_f32_close(FORM_ROW_GAP_PX, 8.0);
        let name_w = |avail: f32| (avail - RIGHT_COL_W_PX - FORM_ROW_GAP_PX).max(160.0);
        let dest_w = |avail: f32| (avail - (RIGHT_COL_W_PX + FORM_ROW_GAP_PX)).max(120.0);
        for avail in [400.0_f32, 600.0, 968.0, 1224.0] {
            assert_f32_close(name_w(avail), dest_w(avail));
        }
    }
}
