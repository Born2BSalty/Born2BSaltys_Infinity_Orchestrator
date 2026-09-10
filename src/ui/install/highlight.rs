// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_selection_highlight, redesign_text_primary,
};

pub(crate) fn highlight_job(
    text: &str,
    query_lower: &str,
    font: egui::FontId,
    color: egui::Color32,
    mark_fill: egui::Color32,
    mark_text: egui::Color32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let plain_format = egui::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };

    if query_lower.is_empty() {
        job.append(text, 0.0, plain_format);
        return job;
    }

    let lower = text.to_ascii_lowercase();
    if lower.len() != text.len() {
        job.append(text, 0.0, plain_format);
        return job;
    }

    let Some(start) = lower.find(query_lower) else {
        job.append(text, 0.0, plain_format);
        return job;
    };
    let end = start + query_lower.len();

    if start > 0 {
        job.append(&text[..start], 0.0, plain_format.clone());
    }
    job.append(
        &text[start..end],
        0.0,
        egui::TextFormat {
            font_id: font,
            color: mark_text,
            background: mark_fill,
            ..Default::default()
        },
    );
    if end < text.len() {
        job.append(&text[end..], 0.0, plain_format);
    }
    job
}

pub(crate) fn highlight_label(
    ui: &mut egui::Ui,
    text: &str,
    query_lower: &str,
    font: egui::FontId,
    color: egui::Color32,
    palette: ThemePalette,
) -> egui::Response {
    let job = highlight_job(
        text,
        query_lower,
        font,
        color,
        redesign_selection_highlight(palette),
        redesign_text_primary(palette),
    );
    ui.label(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> egui::FontId {
        egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()))
    }

    #[test]
    fn empty_query_yields_one_plain_section() {
        let job = highlight_job(
            "DLC Merger",
            "",
            font(),
            egui::Color32::WHITE,
            egui::Color32::BLACK,
            egui::Color32::RED,
        );
        assert_eq!(job.sections.len(), 1);
    }

    #[test]
    fn no_hit_yields_one_plain_section() {
        let job = highlight_job(
            "DLC Merger",
            "zzz",
            font(),
            egui::Color32::WHITE,
            egui::Color32::BLACK,
            egui::Color32::RED,
        );
        assert_eq!(job.sections.len(), 1);
    }

    #[test]
    fn first_hit_is_marked_case_insensitively() {
        let job = highlight_job(
            "DLC Merger Extra",
            "merger",
            font(),
            egui::Color32::WHITE,
            egui::Color32::BLACK,
            egui::Color32::RED,
        );
        assert_eq!(job.sections.len(), 3);
        assert_eq!(job.sections[1].format.color, egui::Color32::RED);
        assert_eq!(job.sections[1].format.background, egui::Color32::BLACK);
    }

    #[test]
    fn hit_at_the_start_yields_two_sections() {
        let job = highlight_job(
            "Merger DLC",
            "merger",
            font(),
            egui::Color32::WHITE,
            egui::Color32::BLACK,
            egui::Color32::RED,
        );
        assert_eq!(job.sections.len(), 2);
        assert_eq!(job.sections[0].format.color, egui::Color32::RED);
    }
}
