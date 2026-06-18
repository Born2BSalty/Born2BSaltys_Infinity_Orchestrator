// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

const POPPINS_LIGHT: &[u8] = include_bytes!("../../../assets/fonts/Poppins-Light.ttf");
const POPPINS_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Poppins-Medium.ttf");
const POPPINS_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Poppins-Bold.ttf");
const FIRACODE_NERD_LIGHT: &[u8] =
    include_bytes!("../../../assets/fonts/FiraCodeNerdFont-Light.ttf");

pub fn install_redesign_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    register_font(&mut fonts, "poppins_light", POPPINS_LIGHT);
    register_font(&mut fonts, "poppins_medium", POPPINS_MEDIUM);
    register_font(&mut fonts, "poppins_bold", POPPINS_BOLD);
    register_font(&mut fonts, "firacode_nerd", FIRACODE_NERD_LIGHT);

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

    fonts.families.insert(
        egui::FontFamily::Name("poppins_light".into()),
        vec!["poppins_light".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Name("poppins_medium".into()),
        vec!["poppins_medium".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Name("poppins_bold".into()),
        vec!["poppins_bold".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Name("firacode_nerd".into()),
        vec!["firacode_nerd".to_owned()],
    );

    ctx.set_fonts(fonts);
}

fn register_font(fonts: &mut egui::FontDefinitions, name: &'static str, bytes: &'static [u8]) {
    if bytes.is_empty() {
        return;
    }
    fonts
        .font_data
        .insert(name.to_owned(), egui::FontData::from_static(bytes).into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redesign_fonts_register() {
        let ctx = egui::Context::default();
        install_redesign_fonts(&ctx);
        let _ = ctx.run(egui::RawInput::default(), |_| {});

        ctx.fonts(|f| {
            let _light = f.glyph_width(
                &egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into())),
                'A',
            );
            let _medium = f.glyph_width(
                &egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into())),
                'A',
            );
            let _bold = f.glyph_width(
                &egui::FontId::new(12.0, egui::FontFamily::Name("poppins_bold".into())),
                'A',
            );
            let _mono = f.glyph_width(
                &egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into())),
                'A',
            );
        });
    }
}
