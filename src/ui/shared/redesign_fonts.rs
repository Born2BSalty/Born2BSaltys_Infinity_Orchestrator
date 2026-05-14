// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

pub fn install_redesign_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "poppins_light".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../assets/fonts/Poppins-Light.ttf"))
            .into(),
    );
    fonts.font_data.insert(
        "poppins_medium".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../assets/fonts/Poppins-Medium.ttf"))
            .into(),
    );
    fonts.font_data.insert(
        "poppins_bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../assets/fonts/Poppins-Bold.ttf"))
            .into(),
    );
    fonts.font_data.insert(
        "firacode_nerd".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/FiraCodeNerdFont-Light.ttf"
        ))
        .into(),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "poppins_medium".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "firacode_nerd".to_owned());

    ctx.set_fonts(fonts);
}
