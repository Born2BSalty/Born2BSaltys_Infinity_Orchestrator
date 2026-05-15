// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_muted};
use crate::ui::step5::prompt_memory;

pub(crate) fn render_entry_row(
    ui: &mut egui::Ui,
    key: &str,
    entry: prompt_memory::PromptAnswerEntry,
    palette: ThemePalette,
) {
    let mut enabled = entry.enabled;
    if ui.checkbox(&mut enabled, "").changed() {
        prompt_memory::set_enabled(key, enabled);
    }

    let mut alias = entry.alias.clone();
    let alias_resp = ui.add_sized(
        [160.0, 24.0],
        egui::TextEdit::singleline(&mut alias).hint_text("optional alias"),
    );
    if alias_resp.changed() {
        prompt_memory::set_alias(key, &alias);
    }

    let preview = if entry.preview.trim().is_empty() {
        "(no preview)".to_string()
    } else {
        entry.preview.clone()
    };
    ui.label(
        crate::ui::shared::typography_global::weak(preview.clone())
            .color(redesign_text_muted(palette))
            .text_style(egui::TextStyle::Small),
    )
    .on_hover_text(format!("{preview}\n\nKey: {key}"));

    let mut answer = entry.answer;
    let answer_resp = ui.add_sized([170.0, 24.0], egui::TextEdit::singleline(&mut answer));
    if answer_resp.changed() {
        prompt_memory::set_answer(key, &answer);
    }

    if ui.button("Delete").clicked() {
        prompt_memory::delete_entry(key);
    }
    ui.end_row();
}
