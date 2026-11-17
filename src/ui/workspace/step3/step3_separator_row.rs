// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step3ItemState;
use crate::app::step3_history::{self, Step3HistoryEntry, Step3TouchedRows};
use crate::app::step3_separators::{SeparatorRename, sanitize_separator_name};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_accent, redesign_pill_text, redesign_with_alpha,
};
use crate::ui::shared::typography_global::strong;

use super::step3_list_body::paint_chevron_button;

const ROW_LEFT_PAD_PX: f32 = 6.0;
const GLYPH_GAP_PX: f32 = 4.0;
const COUNT_GAP_PX: f32 = 8.0;
const NAME_SIZE_STEP_PX: f32 = 4.0;
const RENAME_WIDTH_PX: f32 = 260.0;

pub(super) struct SeparatorRowView<'a> {
    pub(super) palette: ThemePalette,
    pub(super) name: &'a str,
    pub(super) collapsed: bool,
    pub(super) mod_count: usize,
    pub(super) right_x: f32,
    pub(super) band_height: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RenameOutcome {
    Editing,
    Commit(String),
    Cancel,
}

pub(super) struct SeparatorRowResponse {
    pub(super) row_rect: egui::Rect,
    pub(super) grab_rect: egui::Rect,
    pub(super) name_rect: egui::Rect,
    pub(super) chevron_clicked: bool,
    pub(super) rename: Option<RenameOutcome>,
}

pub(super) fn render_separator_row(
    ui: &mut egui::Ui,
    view: &SeparatorRowView<'_>,
    rename: Option<&mut SeparatorRename>,
) -> SeparatorRowResponse {
    let mut chevron_clicked = false;
    let mut rename_outcome = None;
    let mut name_rect = egui::Rect::NOTHING;
    let mut count_rect = egui::Rect::NOTHING;
    let band_left_top = ui.cursor().min;
    let band_shape = ui.painter().add(egui::Shape::Noop);
    let on_band = redesign_pill_text(view.palette);
    let name_size = egui::TextStyle::Body.resolve(ui.style()).size + NAME_SIZE_STEP_PX;
    let band_size = egui::vec2((view.right_x - band_left_top.x).max(0.0), view.band_height);
    let row = ui.allocate_ui_with_layout(
        band_size,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_height(view.band_height);
            ui.add_space(ROW_LEFT_PAD_PX);
            chevron_clicked = paint_chevron_button(ui, view.collapsed, on_band).clicked();
            ui.add_space(GLYPH_GAP_PX);
            if let Some(rename) = rename {
                let (response, outcome) = rename_box(ui, rename);
                name_rect = response.rect;
                rename_outcome = Some(outcome);
            } else {
                name_rect = ui
                    .add(
                        egui::Label::new(strong(view.name).size(name_size).color(on_band))
                            .selectable(false),
                    )
                    .rect;
            }
            ui.add_space(COUNT_GAP_PX);
            count_rect = ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(mod_count_text(view.mod_count))
                            .color(redesign_with_alpha(on_band, 3, 4)),
                    )
                    .selectable(false),
                )
                .rect;
        },
    );
    let row_top = band_left_top.y;
    let row_bottom = row_top + view.band_height.max(row.response.rect.height());
    let row_rect = egui::Rect::from_min_max(
        egui::pos2(band_left_top.x, row_top),
        egui::pos2(view.right_x.max(count_rect.right()), row_bottom),
    );
    ui.painter().set(
        band_shape,
        egui::Shape::rect_filled(
            row_rect,
            egui::CornerRadius::ZERO,
            redesign_accent(view.palette),
        ),
    );
    let grab_left = if rename_outcome.is_some() {
        count_rect.left()
    } else {
        name_rect.left()
    };
    let grab_rect = egui::Rect::from_min_max(
        egui::pos2(grab_left, row_top),
        egui::pos2(row_rect.right(), row_bottom),
    );
    SeparatorRowResponse {
        row_rect,
        grab_rect,
        name_rect,
        chevron_clicked,
        rename: rename_outcome,
    }
}

fn mod_count_text(mod_count: usize) -> String {
    if mod_count == 1 {
        "· 1 mod".to_string()
    } else {
        format!("· {mod_count} mods")
    }
}

fn rename_box(ui: &mut egui::Ui, rename: &mut SeparatorRename) -> (egui::Response, RenameOutcome) {
    let id = egui::Id::new(("step3_separator_rename", rename.tab, &rename.block_id));
    let response = ui.add(
        egui::TextEdit::singleline(&mut rename.text)
            .id(id)
            .desired_width(RENAME_WIDTH_PX),
    );
    if rename.focus_pending {
        response.request_focus();
        response.scroll_to_me(Some(egui::Align::Center));
        select_whole_text(ui.ctx(), id, &rename.text);
        rename.focus_pending = false;
    }
    let outcome = if response.lost_focus() {
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            RenameOutcome::Cancel
        } else {
            RenameOutcome::Commit(rename.text.clone())
        }
    } else {
        RenameOutcome::Editing
    };
    (response, outcome)
}

fn select_whole_text(ctx: &egui::Context, id: egui::Id, text: &str) {
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let whole_text = egui::text::CCursorRange::two(
        egui::text::CCursor::new(0),
        egui::text::CCursor::new(text.chars().count()),
    );
    state.cursor.set_char_range(Some(whole_text));
    state.store(ctx, id);
}

pub(super) fn commit_separator_rename(
    items: &mut [Step3ItemState],
    idx: usize,
    text: &str,
    undo_stack: &mut Vec<Step3HistoryEntry>,
    redo_stack: &mut Vec<Step3HistoryEntry>,
) -> bool {
    let Some(name) = sanitize_separator_name(text) else {
        return false;
    };
    if !items
        .get(idx)
        .is_some_and(|item| item.is_separator() && item.mod_name != name)
    {
        return false;
    }
    step3_history::push_undo_snapshot(items, Step3TouchedRows::default(), undo_stack, redo_stack);
    items[idx].component_label.clone_from(&name);
    items[idx].mod_name = name;
    true
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::{RenameOutcome, commit_separator_rename, mod_count_text, rename_box};
    use crate::app::game_authority::GameSlot;
    use crate::app::step3_separators::{SeparatorRename, separator_row};

    fn run_rename_frame(
        ctx: &egui::Context,
        rename: &mut SeparatorRename,
        events: Vec<egui::Event>,
    ) -> RenameOutcome {
        let mut outcome = RenameOutcome::Editing;
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                outcome = rename_box(ui, rename).1;
            });
        });
        outcome
    }

    #[test]
    fn typing_into_a_freshly_opened_rename_box_replaces_the_old_name() {
        let ctx = egui::Context::default();
        let mut rename = SeparatorRename {
            tab: GameSlot::First,
            block_id: "SEPARATOR::1".to_string(),
            text: "New section".to_string(),
            focus_pending: true,
        };

        run_rename_frame(&ctx, &mut rename, Vec::new());
        assert!(!rename.focus_pending);
        run_rename_frame(
            &ctx,
            &mut rename,
            vec![egui::Event::Text("Quests".to_string())],
        );
        let outcome = run_rename_frame(
            &ctx,
            &mut rename,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
        );

        assert_eq!(outcome, RenameOutcome::Commit("Quests".to_string()));
    }

    #[test]
    fn mod_count_reads_singular_for_one_mod() {
        assert_eq!(mod_count_text(1), "· 1 mod");
        assert_eq!(mod_count_text(0), "· 0 mods");
        assert_eq!(mod_count_text(3), "· 3 mods");
    }

    #[test]
    fn a_rename_is_cleaned_and_undoable_and_a_blank_one_keeps_the_name() {
        let mut items = vec![separator_row("Fixes", "SEPARATOR::1".to_string())];
        let mut undo_stack = Vec::new();
        let mut redo_stack = Vec::new();

        assert!(!commit_separator_rename(
            &mut items,
            0,
            "  ##  ",
            &mut undo_stack,
            &mut redo_stack
        ));
        assert_eq!(items[0].mod_name, "Fixes");
        assert_eq!(undo_stack.len(), 0);

        assert!(commit_separator_rename(
            &mut items,
            0,
            "## NPCs @home ##",
            &mut undo_stack,
            &mut redo_stack
        ));
        assert_eq!(items[0].mod_name, "NPCs home");
        assert_eq!(items[0].component_label, "NPCs home");
        assert_eq!(undo_stack.len(), 1);
        assert_eq!(undo_stack[0].items[0].mod_name, "Fixes");
    }
}
