// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::EmbeddedTerminal;

fn is_component_id_token(token: &str) -> bool {
    let t = token.trim_matches(|c: char| c == ',' || c == '.' || c == ':' || c == ';');
    t.starts_with('#') && t[1..].chars().all(|c| c.is_ascii_digit())
}

fn normalized_token(token: &str) -> String {
    token
        .trim_matches(|c: char| {
            !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '[' && c != ']'
        })
        .to_ascii_uppercase()
}

fn split_chunks_preserve_quotes(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    for ch in line.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            cur.push(ch);
            continue;
        }
        if ch.is_whitespace() && !in_quote {
            cur.push(ch);
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn token_color(token: &str) -> egui::Color32 {
    let t = token.trim();
    let n = normalized_token(t);
    let default = egui::Color32::from_rgb(210, 210, 210);
    let red = egui::Color32::from_rgb(230, 96, 96);
    let debug_blue = egui::Color32::from_rgb(70, 110, 180);
    let sent_blue = egui::Color32::from_rgb(110, 190, 255);
    let info_green = egui::Color32::from_rgb(168, 204, 98);
    let amber = egui::Color32::from_rgb(214, 168, 96);
    let sand = egui::Color32::from_rgb(214, 182, 146);
    let dim = egui::Color32::from_rgb(150, 150, 150);

    if n == "ERROR" || n == "FATAL" {
        return red;
    }
    if n == "WARN" || n == "WARNING" {
        return amber;
    }
    if n == "DEBUG" {
        return debug_blue;
    }
    if n == "INFO" {
        return info_green;
    }
    if n == "[SENT]" || n == "SENT" {
        return sent_blue;
    }

    // Keep component ids visually distinct in log-like lines.
    if is_component_id_token(t) {
        return sent_blue;
    }

    // Keep only WeiDU-style ~...~ path markers colored.
    if t.contains('~') {
        return sand;
    }
    if n.contains("MOD_INSTALLER::") || n.contains("WEIDU_PARSER") || n.contains("WEIDU]") {
        return dim;
    }
    default
}

fn render_styled_line(ui: &mut egui::Ui, line: &str) {
    let line_upper = line.to_ascii_uppercase();
    let success_line = line_upper.contains("SUCCESSFULLY INSTALLED");
    let success_green = egui::Color32::from_rgb(124, 196, 124);
    if line.is_empty() {
        ui.label(egui::RichText::new(" ").monospace().strong());
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for token in split_chunks_preserve_quotes(line) {
            let n = normalized_token(&token);
            let color = if success_line && (n == "SUCCESSFULLY" || n == "INSTALLED") {
                success_green
            } else {
                token_color(&token)
            };
            ui.label(
                egui::RichText::new(&token)
                    .monospace()
                    .strong()
                    .color(color),
            );
        }
    });
}

pub(super) fn render(term_state: &mut EmbeddedTerminal, ui: &mut egui::Ui, size: egui::Vec2) {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        term_state.request_focus = true;
    }
    term_state.active = response.has_focus();
    if term_state.request_focus && term_state.active {
        term_state.request_focus = false;
    }

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.scope(|ui| {
            let mut scroll = egui::style::ScrollStyle::solid();
            scroll.bar_width = 12.0;
            scroll.bar_inner_margin = 0.0;
            scroll.bar_outer_margin = 2.0;
            ui.style_mut().spacing.scroll = scroll;
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            let out = egui::ScrollArea::vertical()
                .id_salt("step5_console_scroll")
                .auto_shrink([false, false])
                .stick_to_bottom(term_state.stick_to_bottom)
                .show(ui, |ui| {
                    for line in term_state.display_text().split('\n') {
                        render_styled_line(ui, line);
                    }
                    if term_state.stick_to_bottom {
                        ui.add_space(0.0);
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });
            let _ = out;
        });
    });
}
