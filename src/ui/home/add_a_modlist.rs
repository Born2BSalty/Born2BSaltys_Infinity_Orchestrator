// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `add_a_modlist` — the Home right-column Box.
//
// Mirrors `wireframe-preview/screens.jsx::HomeScreen` right column
// (line 350-364):
//   <Box label="add a modlist" style={padding:16}>
//     <div flex column gap:10>
//       {!v1 && <Btn>Browse community modlists</Btn>}   // omitted in v1 alpha
//       <Btn primary>paste import code</Btn>            // → navigate("install")
//       <Btn>create your own</Btn>                      // → navigate("create")
//     </div>
//     <div marginTop:20>
//       <Label hand>game installs detected</Label>
//       …detection lines…                                // game_installs_detected
//     </div>
//   </Box>
//
// `Browse community modlists` is intentionally NOT rendered — SPEC §2.1:
// "Explore is intentionally omitted from v1 alpha". Labels are intentionally
// lowercase (SPEC §3.3 — "read as fluent verb phrases").
//
// The game-installs-detected block is part of this same Box per the
// wireframe; it is delegated to `game_installs_detected::render`.
//
// SPEC: §3.3 ("Add a modlist" section).

use eframe::egui;

use crate::ui::home::game_installs_detected;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_box, redesign_btn};
use crate::ui::shared::redesign_tokens::redesign_accent_deep;

/// Which CTA the user clicked this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddAModlistAction {
    #[default]
    None,
    /// `paste import code` → Install destination.
    PasteImportCode,
    /// `create your own` → Create destination.
    CreateYourOwn,
}

/// Render the right-column `add a modlist` Box (CTAs + game-installs block).
pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &OrchestratorApp,
) -> AddAModlistAction {
    let palette = orchestrator.theme_palette;
    let mut action = AddAModlistAction::None;

    redesign_box(ui, palette, Some("add a modlist"), |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 10.0;

            if redesign_btn(
                ui,
                palette,
                "paste import code",
                BtnOpts {
                    primary: true,
                    block: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                action = AddAModlistAction::PasteImportCode;
            }

            if redesign_btn(
                ui,
                palette,
                "create your own",
                BtnOpts {
                    block: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                action = AddAModlistAction::CreateYourOwn;
            }
        });

        // 20px gap, then the detected-games block (wireframe `marginTop: 20`).
        ui.add_space(20.0);
        ui.label(
            // Wireframe `<Label hand>` — 14px accent-deep, weight 400 (not
            // bold). poppins_light is how we map the wireframe's hand-style
            // labels elsewhere (e.g. the card meta line).
            egui::RichText::new("game installs detected")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_accent_deep(palette)),
        );
        ui.add_space(6.0);
        game_installs_detected::render(ui, palette, orchestrator);
    });

    action
}
