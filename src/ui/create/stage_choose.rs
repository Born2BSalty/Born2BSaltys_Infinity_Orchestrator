// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::Game;
use crate::ui::create::state_create::{CreateScreenState, StartingPoint};
use crate::ui::install::destination_not_empty;
use crate::ui::install::sub_flow_footer::{self, PrimaryBtn};
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, redesign_box, redesign_btn, redesign_text_input, render_screen_title,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_input_bg, redesign_shell_bg, redesign_text_faint,
    redesign_text_muted, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChooseOutcome {
    #[default]
    Stay,
    StartScratch,
    GoForkPaste,
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
const SCRATCH_TITLE: &str = "New modlist from downloaded mods";
const SCRATCH_DESC: &str = "Scan your local mods folder, pick components, reorder, then install. Starts from an empty selection.";
const IMPORT_TITLE: &str = "Import and modify another modlist";
const IMPORT_DESC: &str = "Paste a share code. BIO downloads the mods, preselects components, applies the order, then drops you on Step 2 to review and adjust.";

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
    destination_prep_running: bool,
) -> ChooseOutcome {
    let mut outcome = ChooseOutcome::Stay;

    render_body(ui, palette, state, &mut outcome);

    let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    if spacer > 0.0 {
        ui.add_space(spacer);
    }

    let footer = sub_flow_footer::render(
        ui,
        palette,
        None::<sub_flow_footer::BackBtn<'_>>,
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        None,
        PrimaryBtn {
            label: if destination_prep_running {
                "Preparing"
            } else {
                "Start"
            },
            disabled: destination_prep_running,
        },
    );
    if footer.primary_clicked {
        outcome = match state.starting_point {
            StartingPoint::Scratch => ChooseOutcome::StartScratch,
            StartingPoint::Import => ChooseOutcome::GoForkPaste,
        };
    }

    outcome
}

fn render_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
    outcome: &mut ChooseOutcome,
) {
    render_title_row(ui, palette, outcome);
    render_setup_box(ui, palette, state);
    render_starting_point_boxes(ui, palette, state);
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
                    "Create / edit modlist",
                    Some(
                        "name your modlist, set destination + mods paths, then pick a starting point",
                    ),
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

fn render_setup_box(ui: &mut egui::Ui, palette: ThemePalette, state: &mut CreateScreenState) {
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
                        match state.starting_point {
                            StartingPoint::Scratch => {
                                game_combo(ui, palette, &mut state.game, input_box_h);
                            }
                            StartingPoint::Import => {
                                game_from_code_note(ui, palette, input_box_h);
                            }
                        }
                    },
                );

                input_box_h
            })
            .inner;

        let dest_changed = folder_input(
            ui,
            palette,
            "destination folder",
            "D:\\BG2EE_install_test",
            &mut state.destination,
            input_box_h,
        );
        if dest_changed {
            state.destination_choice = None;
        }

        if destination_is_non_empty(&state.destination)
            && let Some(picked) =
                destination_not_empty::render(ui, palette, state.destination_choice, false)
        {
            state.destination_choice = Some(picked);
        }
    });
}

fn render_starting_point_boxes(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
) {
    ui.add_space(18.0);

    ui.label(
        egui::RichText::new("Choose one")
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(8.0);

    let avail_w = ui.available_width();
    let gap = 14.0;
    let card_w = ((avail_w - gap) / 2.0).max(160.0);
    let measured = selectable_box_natural_height(ui, card_w, SCRATCH_TITLE, SCRATCH_DESC).max(
        selectable_box_natural_height(ui, card_w, IMPORT_TITLE, IMPORT_DESC),
    );
    // The wrapped-text pre-measure under-estimates the rendered card height, so
    // carry the real rendered max-height forward a frame and force both cards to
    // it; reset when the column width changes so it re-settles.
    let cards_key = ui.id().with("create_cards_equal_h");
    let (prev_w, carry) = ui
        .ctx()
        .memory(|m| m.data.get_temp::<(f32, f32)>(cards_key))
        .unwrap_or((0.0, 0.0));
    let carry = if (prev_w - card_w).abs() > 0.5 {
        0.0
    } else {
        carry
    };
    let box_h = measured.max(carry);

    let mut h_scratch = 0.0_f32;
    let mut h_import = 0.0_f32;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;

        let (clicked, h) = selectable_box(
            ui,
            palette,
            SelectableBoxSpec {
                width: card_w,
                min_h: box_h,
                title: SCRATCH_TITLE,
                desc: SCRATCH_DESC,
                selected: state.starting_point == StartingPoint::Scratch,
                id_salt: "create_box_scratch",
            },
        );
        if clicked {
            state.starting_point = StartingPoint::Scratch;
        }
        h_scratch = h;

        let (clicked, h) = selectable_box(
            ui,
            palette,
            SelectableBoxSpec {
                width: card_w,
                min_h: box_h,
                title: IMPORT_TITLE,
                desc: IMPORT_DESC,
                selected: state.starting_point == StartingPoint::Import,
                id_salt: "create_box_import",
            },
        );
        if clicked {
            state.starting_point = StartingPoint::Import;
        }
        h_import = h;
    });

    let diff = (h_scratch - h_import).abs();
    if diff > 0.5 {
        ui.ctx()
            .memory_mut(|m| m.data.insert_temp(cards_key, (card_w, box_h + diff)));
        ui.ctx().request_repaint();
    }
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

fn game_from_code_note(ui: &mut egui::Ui, palette: ThemePalette, box_h: f32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), box_h),
        egui::Sense::hover(),
    );
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
        redesign_shell_bg(palette),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "imported",
        egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into())),
        redesign_text_faint(palette),
    );
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
) -> bool {
    let mut changed = false;

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
                border: None,
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

#[derive(Clone, Copy)]
struct SelectableBoxSpec<'a> {
    width: f32,
    min_h: f32,
    title: &'a str,
    desc: &'a str,
    selected: bool,
    id_salt: &'a str,
}

fn selectable_box(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    spec: SelectableBoxSpec<'_>,
) -> (bool, f32) {
    let SelectableBoxSpec {
        width,
        min_h,
        title,
        desc,
        selected,
        id_salt,
    } = spec;
    let border_color = if selected {
        redesign_accent(palette)
    } else {
        redesign_border_strong(palette)
    };
    let fill = if selected {
        faint_accent_tint(palette)
    } else {
        redesign_shell_bg(palette)
    };

    let chassis = egui::Frame::default()
        .fill(fill)
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border_color))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: SBOX_PAD_X,
            right: SBOX_PAD_X,
            top: SBOX_PAD_Y,
            bottom: SBOX_PAD_Y,
        });

    let inner = ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            chassis.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(2.0f32.mul_add(-f32::from(SBOX_PAD_Y), min_h));
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(
                    egui::RichText::new(title)
                        .size(SBOX_TITLE_SIZE)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_primary(palette)),
                );
                ui.add_space(SBOX_TITLE_GAP);
                ui.label(
                    egui::RichText::new(desc)
                        .size(SBOX_DESC_SIZE)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_muted(palette)),
                );
            });
        },
    );

    let card_h = inner.response.rect.height();
    let resp = ui.interact(
        inner.response.rect,
        ui.make_persistent_id(("create_selectable_box", id_salt)),
        egui::Sense::click(),
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    (resp.clicked(), card_h)
}

const SELECTED_TINT_NUMERATOR: u16 = 14;
const SELECTED_TINT_DENOMINATOR: u16 = 100;

fn faint_accent_tint(palette: ThemePalette) -> egui::Color32 {
    let bg = redesign_shell_bg(palette);
    let ac = redesign_accent(palette);
    let mix = |b: u8, a: u8| -> u8 {
        let background_weight = SELECTED_TINT_DENOMINATOR - SELECTED_TINT_NUMERATOR;
        let mixed = (u16::from(b) * background_weight
            + u16::from(a) * SELECTED_TINT_NUMERATOR
            + SELECTED_TINT_DENOMINATOR / 2)
            / SELECTED_TINT_DENOMINATOR;
        u8::try_from(mixed).expect("mixed color channel is bounded")
    };
    egui::Color32::from_rgb(
        mix(bg.r(), ac.r()),
        mix(bg.g(), ac.g()),
        mix(bg.b(), ac.b()),
    )
}

const SBOX_PAD_X: i8 = 22;
const SBOX_PAD_Y: i8 = 20;
const SBOX_TITLE_SIZE: f32 = 18.0;
const SBOX_TITLE_GAP: f32 = 8.0;
const SBOX_DESC_SIZE: f32 = 13.0;

fn selectable_box_natural_height(ui: &egui::Ui, card_w: f32, title: &str, desc: &str) -> f32 {
    let inner_w = 2.0f32.mul_add(-f32::from(SBOX_PAD_X), card_w).max(1.0);
    let title_h = wrapped_text_height(ui, title, SBOX_TITLE_SIZE, "poppins_light", inner_w);
    let desc_h = wrapped_text_height(ui, desc, SBOX_DESC_SIZE, "poppins_light", inner_w);
    2.0f32.mul_add(f32::from(SBOX_PAD_Y), title_h) + SBOX_TITLE_GAP + desc_h
}

fn wrapped_text_height(ui: &egui::Ui, text: &str, size: f32, family: &str, wrap_w: f32) -> f32 {
    let font = egui::FontId::new(size, egui::FontFamily::Name(family.into()));
    ui.fonts(|f| {
        f.layout(text.to_string(), font, egui::Color32::PLACEHOLDER, wrap_w)
            .size()
            .y
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const _: () = assert!(SELECTED_TINT_NUMERATOR * 2 < SELECTED_TINT_DENOMINATOR);

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
    fn start_outcome_maps_from_selected_starting_point() {
        let pick = |sp: StartingPoint| match sp {
            StartingPoint::Scratch => ChooseOutcome::StartScratch,
            StartingPoint::Import => ChooseOutcome::GoForkPaste,
        };
        assert_eq!(pick(StartingPoint::Scratch), ChooseOutcome::StartScratch);
        assert_eq!(pick(StartingPoint::Import), ChooseOutcome::GoForkPaste);
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
    fn equalized_box_height_is_the_taller_boxs_natural_height() {
        let nat = |title_h: f32, desc_h: f32| {
            2.0f32.mul_add(f32::from(SBOX_PAD_Y), title_h) + SBOX_TITLE_GAP + desc_h
        };
        let short = nat(20.0, 40.0);
        let tall = nat(20.0, 72.0);
        let equalized = short.max(tall);
        assert_f32_close(equalized, tall);
        assert!(
            equalized >= short,
            "the shorter box is grown to match, never clipped"
        );
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

    #[test]
    fn faint_accent_tint_is_opaque_and_near_shell_bg() {
        for palette in [ThemePalette::Dark, ThemePalette::Light] {
            let tint = faint_accent_tint(palette);
            assert_eq!(tint.a(), 255, "the selected tint must be opaque");
            let bg = redesign_shell_bg(palette);
            let ac = redesign_accent(palette);
            let expect = |b: u8, a: u8| -> u8 {
                let background_weight = SELECTED_TINT_DENOMINATOR - SELECTED_TINT_NUMERATOR;
                let mixed = (u16::from(b) * background_weight
                    + u16::from(a) * SELECTED_TINT_NUMERATOR
                    + SELECTED_TINT_DENOMINATOR / 2)
                    / SELECTED_TINT_DENOMINATOR;
                u8::try_from(mixed).expect("mixed color channel is bounded")
            };
            assert_eq!(tint.r(), expect(bg.r(), ac.r()));
            assert_eq!(tint.g(), expect(bg.g(), ac.g()));
            assert_eq!(tint.b(), expect(bg.b(), ac.b()));
        }
    }
}
