// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::step5::state_step5::Step5ConsoleViewState;

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
    let default = crate::ui::shared::theme_global::terminal_default();
    let red = crate::ui::shared::theme_global::terminal_error();
    let debug_blue = crate::ui::shared::theme_global::terminal_debug();
    let sent_blue = crate::ui::shared::theme_global::terminal_sent();
    let info_green = crate::ui::shared::theme_global::terminal_info();
    let amber = crate::ui::shared::theme_global::terminal_amber();
    let sand = crate::ui::shared::theme_global::terminal_sand();
    let dim = crate::ui::shared::theme_global::terminal_dim();

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

    if is_component_id_token(t) {
        return sent_blue;
    }

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
    let success_green = crate::ui::shared::theme_global::success();
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

pub(crate) fn render_console_panel(
    ui: &mut egui::Ui,
    _state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
    terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(crate::ui::shared::typography_global::section_title(
            "Console",
        ));
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_SM);
        let console_w = ui.available_width();
        let reserved_for_input = 56.0;
        let console_h = (ui.available_height() - reserved_for_input).max(220.0);
        if let Some(term) = terminal {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(console_w, console_h), egui::Sense::click());
            if response.clicked() {
                console_view.request_input_focus = true;
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
                        .stick_to_bottom(console_view.auto_scroll)
                        .show(ui, |ui| {
                            for line in selected_console_text(term, console_view).split('\n') {
                                render_styled_line(ui, line);
                            }
                            if console_view.auto_scroll {
                                ui.add_space(0.0);
                                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                            }
                        });
                    let _ = out;
                });
            });
        } else {
            ui.add_sized(
                [console_w, console_h],
                egui::Label::new(terminal_error.unwrap_or("Initializing terminal...")),
            );
        }
    });
}

fn selected_console_text<'a>(
    terminal: &'a EmbeddedTerminal,
    console_view: &Step5ConsoleViewState,
) -> &'a str {
    if console_view.installed_only {
        terminal.installed_text()
    } else if console_view.important_only {
        terminal.important_text()
    } else {
        terminal.output_text()
    }
}
