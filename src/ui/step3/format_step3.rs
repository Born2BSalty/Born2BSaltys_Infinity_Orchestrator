// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step3ItemState;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_accent_comment, redesign_accent_numbers,
    redesign_accent_path, redesign_font_mono, redesign_text_primary,
};

pub(crate) fn weidu_colored_widget_text(
    _ui: &egui::Ui,
    text: &str,
    palette: ThemePalette,
) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    let mono = egui::FontId::new(REDESIGN_LABEL_FONT_SIZE_PX, redesign_font_mono());
    let default_color = redesign_text_primary(palette);
    let path_color = redesign_accent_path(palette);
    let nums_color = redesign_accent_numbers(palette);
    let comment_color = redesign_accent_comment(palette);

    if let Some(path_start) = text.find('~')
        && let Some(path_end_rel) = text[path_start + 1..].find('~')
    {
        let path_end = path_start + path_end_rel + 2;
        let comment_start = text[path_end..].find("//").map(|idx| path_end + idx);

        append_text(&mut job, &text[..path_start], &mono, default_color);
        append_text(&mut job, &text[path_start..path_end], &mono, path_color);

        if let Some(comment_start) = comment_start {
            append_text(&mut job, &text[path_end..comment_start], &mono, nums_color);
            append_text(&mut job, &text[comment_start..], &mono, comment_color);
        } else {
            append_text(&mut job, &text[path_end..], &mono, nums_color);
        }

        return egui::WidgetText::from(job);
    }

    append_text(&mut job, text, &mono, default_color);
    egui::WidgetText::from(job)
}

pub(crate) fn format_step3_item(item: &Step3ItemState) -> String {
    if !item.raw_line.trim().is_empty() {
        normalize_weidu_like_line(&item.raw_line)
    } else {
        let folder = item.mod_name.replace('/', "\\");
        format!(
            "~{}\\{}~ #0 #{} // {}",
            folder, item.tp_file, item.component_id, item.component_label
        )
    }
}

fn append_text(
    job: &mut egui::text::LayoutJob,
    text: &str,
    font: &egui::FontId,
    color: egui::Color32,
) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        },
    );
}

fn normalize_weidu_like_line(raw: &str) -> String {
    crate::platform_defaults::normalize_weidu_like_line(raw)
}
