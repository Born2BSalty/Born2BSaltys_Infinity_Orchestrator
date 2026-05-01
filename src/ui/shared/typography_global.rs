// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::fs;
use std::path::Path;

pub const SIZE_SECTION_TITLE: f32 = 14.0;
pub const SIZE_PILL_TEXT: f32 = 11.0;

pub fn section_title(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().size(SIZE_SECTION_TITLE)
}

pub fn strong<T: Into<String>>(text: T) -> egui::RichText {
    egui::RichText::new(text.into()).strong()
}

pub fn plain<T: Into<String>>(text: T) -> egui::RichText {
    egui::RichText::new(text.into())
}

pub fn weak<T: Into<String>>(text: T) -> egui::RichText {
    egui::RichText::new(text.into()).weak()
}

pub fn monospace<T: Into<String>>(text: T) -> egui::RichText {
    egui::RichText::new(text.into()).monospace()
}

pub fn mono_weak<T: Into<String>>(text: T) -> egui::RichText {
    egui::RichText::new(text.into()).monospace().weak()
}

pub fn small_strong<T: Into<String>>(text: T) -> egui::RichText {
    egui::RichText::new(text.into()).small().strong()
}

pub fn configure_typography(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let ui_candidates = [
        "C:/Windows/Fonts/FiraCodeNerdFont-Regular.ttf",
        "C:/Windows/Fonts/FiraCodeNerdFontMono-Regular.ttf",
        "C:/Windows/Fonts/FiraCodeNerdFont.ttf",
        "/usr/share/fonts/truetype/firacode/FiraCodeNerdFont-Regular.ttf",
        "/usr/share/fonts/TTF/FiraCodeNerdFont-Regular.ttf",
    ];
    let terminal_candidates = [
        "C:/Windows/Fonts/FiraCode-Regular.ttf",
        "C:/Windows/Fonts/FiraCode-Medium.ttf",
        "/usr/share/fonts/truetype/firacode/FiraCode-Regular.ttf",
        "/usr/share/fonts/TTF/FiraCode-Regular.ttf",
    ];
    let ui_bytes = ui_candidates
        .iter()
        .find_map(|p| fs::read(Path::new(p)).ok());
    let terminal_bytes = terminal_candidates
        .iter()
        .find_map(|p| fs::read(Path::new(p)).ok());

    if let Some(bytes) = ui_bytes {
        let name = "FiraCode Nerd Font".to_string();
        fonts
            .font_data
            .insert(name.clone(), egui::FontData::from_owned(bytes).into());
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            fam.insert(0, name.clone());
        }
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            fam.insert(0, name);
        }
    }

    let terminal_font_name = if let Some(bytes) = terminal_bytes {
        let name = "FiraCode Terminal".to_string();
        fonts
            .font_data
            .insert(name.clone(), egui::FontData::from_owned(bytes).into());
        name
    } else {
        "Hack".to_string()
    };
    fonts.families.insert(
        egui::FontFamily::Name("TerminalMono".into()),
        vec![terminal_font_name, "Hack".to_string()],
    );

    ctx.set_fonts(fonts);
}
