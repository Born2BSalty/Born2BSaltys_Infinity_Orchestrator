// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

fn parse_lang_and_component_ids(raw_line: &str, fallback_component_id: &str) -> (String, String) {
    let mut parts = raw_line.split_whitespace();
    let lang = parts
        .find(|p| p.starts_with('#'))
        .map(|p| p.trim_start_matches('#').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0".to_string());
    let component = parts
        .find(|p| p.starts_with('#'))
        .map(|p| p.trim_start_matches('#').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_component_id.to_string());
    (lang, component)
}

fn display_from_raw_line(raw_line: &str, fallback_label: &str) -> String {
    raw_line
        .split_once("//")
        .map(|(_, right)| right.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_label.to_string())
}

pub(crate) fn format_component_row_label(
    raw_line: &str,
    fallback_component_id: &str,
    label: &str,
) -> String {
    let (lang, component) = parse_lang_and_component_ids(raw_line, fallback_component_id);
    let display = display_from_raw_line(raw_line, label);
    format!("#{lang} #{component} // {display}")
}

pub(crate) fn colored_component_widget_text(ui: &egui::Ui, text: &str) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let default_color = ui.visuals().text_color();
    let nums_color = crate::ui::shared::theme_global::accent_numbers();
    let comment_color = crate::ui::shared::theme_global::accent_comment();

    if let Some(split_at) = text.find("//") {
        let left = text[..split_at].trim_end();
        let right = text[split_at + 2..].trim_start();
        let left_is_component_prefix = left.trim_start().starts_with('#');
        let left_color = if left_is_component_prefix {
            nums_color
        } else {
            comment_color
        };
        let right_color = if left_is_component_prefix {
            comment_color
        } else {
            nums_color
        };
        append_text(&mut job, left, &mono, left_color);
        append_text(&mut job, " ", &mono, default_color);
        append_text(&mut job, "//", &mono, comment_color);
        append_text(&mut job, " ", &mono, default_color);
        append_text(&mut job, right, &mono, right_color);
        return egui::WidgetText::from(job);
    }

    append_text(&mut job, text, &mono, default_color);
    egui::WidgetText::from(job)
}

fn append_text(
    job: &mut egui::text::LayoutJob,
    text: &str,
    font_id: &egui::FontId,
    color: egui::Color32,
) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font_id.clone(),
            color,
            ..Default::default()
        },
    );
}
