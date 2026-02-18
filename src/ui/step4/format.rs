// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::path::Path;

use crate::ui::state::Step3ItemState;

pub fn render_weidu_colored_line(ui: &mut egui::Ui, text: &str) {
    let mut job = egui::text::LayoutJob::default();
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let default_color = ui.visuals().text_color();
    let path_color = egui::Color32::from_rgb(224, 196, 156);
    let nums_color = egui::Color32::from_rgb(136, 176, 255);
    let comment_color = egui::Color32::from_rgb(124, 196, 124);

    if text.trim_start().starts_with("//") {
        append_text(&mut job, text, &mono, comment_color);
        ui.label(egui::WidgetText::from(job));
        return;
    }

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
    } else {
        append_text(&mut job, text, &mono, default_color);
    }

    ui.label(egui::WidgetText::from(job));
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

pub fn format_step4_item(item: &Step3ItemState) -> String {
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

pub fn build_weidu_export_lines(items: &[Step3ItemState]) -> Vec<String> {
    items
        .iter()
        .filter(|i| !i.is_parent)
        .map(format_step4_item)
        .collect()
}

fn normalize_weidu_like_line(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with('~') {
        return trimmed.to_string();
    }
    let Some(end) = trimmed[1..].find('~').map(|i| i + 1) else {
        return trimmed.to_string();
    };
    let path_part = &trimmed[1..end];
    let suffix = &trimmed[end + 1..];
    let p = Path::new(path_part);
    let file = p
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| item_fallback_file(path_part));
    let folder = p
        .parent()
        .and_then(|v| v.file_name())
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| item_fallback_folder(path_part));
    format!("~{}\\{}~{}", folder, file, suffix)
}

fn item_fallback_file(path_part: &str) -> String {
    path_part
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(path_part)
        .to_string()
}

fn item_fallback_folder(path_part: &str) -> String {
    let mut parts = path_part.rsplit(['\\', '/']);
    let _ = parts.next();
    parts.next().unwrap_or("MOD").to_string()
}
