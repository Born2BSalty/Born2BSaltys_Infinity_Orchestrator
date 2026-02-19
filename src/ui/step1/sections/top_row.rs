// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step1State;

pub fn top_row(ui: &mut egui::Ui, s: &mut Step1State, dev_mode: bool) {
    ui.columns(3, |cols| {
        cols[0].group(|ui| {
            ui.set_width(ui.available_width());
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            super::section_title(ui, "Game Selection");
            ui.horizontal(|ui| {
                ui.label("Game Install")
                    .on_hover_text("Select target mode: BGEE, BG2EE, or EET.");
                egui::ComboBox::from_id_salt("game_install")
                    .selected_text(&s.game_install)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut s.game_install, "BGEE".to_string(), "BGEE");
                        ui.selectable_value(&mut s.game_install, "BG2EE".to_string(), "BG2EE");
                        ui.selectable_value(&mut s.game_install, "EET".to_string(), "EET");
                    });
            });
        });
        cols[0].add_space(super::SECTION_GAP);
        cols[0].group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(142.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            super::section_title(ui, "Advanced Options");
            ui.horizontal(|ui| {
                ui.checkbox(&mut s.custom_scan_depth, "Custom scan depth")
                    .on_hover_text("Maximum folder depth under Mods Folder to search for TP2 files.");
                ui.add_enabled(
                    s.custom_scan_depth,
                    egui::DragValue::new(&mut s.depth).range(1..=10),
                );
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut s.timeout_per_mod_enabled, "Timeout per mod")
                    .on_hover_text("Abort a component if it runs longer than this many seconds.");
                ui.add_enabled(
                    s.timeout_per_mod_enabled,
                    egui::DragValue::new(&mut s.timeout).range(30..=86400),
                );
            });
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut s.auto_answer_initial_delay_enabled,
                    "Auto-answer initial delay (ms)",
                )
                .on_hover_text(
                    "Base wait before first auto-answer on a prompt. Increase this for large prompt lists so answers are not sent too early.",
                );
                ui.add_enabled(
                    s.auto_answer_initial_delay_enabled,
                    egui::DragValue::new(&mut s.auto_answer_initial_delay_ms).range(500..=15000),
                );
            });
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut s.auto_answer_post_send_delay_enabled,
                    "Auto-answer post-send delay (ms)",
                )
                .on_hover_text("Wait between auto-answer sends and fallback checks.");
                ui.add_enabled(
                    s.auto_answer_post_send_delay_enabled,
                    egui::DragValue::new(&mut s.auto_answer_post_send_delay_ms).range(500..=15000),
                );
            });
            if dev_mode {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.tick_dev_enabled, "Tick (dev)")
                        .on_hover_text("Polling interval in ms for installer output.");
                    ui.add_enabled(
                        s.tick_dev_enabled,
                        egui::DragValue::new(&mut s.tick).range(50..=5000),
                    );
                });
            }
        });

        cols[1].group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(204.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            super::section_title(ui, "Options");
            let prompt_help = "Copies: // @wlb-inputs:\n\
\n\
What it does\n\
Pre-fills answers for installer prompts from the same weidu.log line.\n\
\n\
How to use\n\
Add the tag at the end of a component line:\n\
... // @wlb-inputs: answer1,answer2,answer3\n\
\n\
Rules\n\
- Answers are used left-to-right.\n\
- Use ,, for a blank answer (press Enter).\n\
- Keep the colon exactly: // @wlb-inputs:\n\
\n\
Examples\n\
\n\
Yes/No prompt\n\
~EET\\EET.TP2~ #0 #0 // EET core (resource importation): v14.0 // @wlb-inputs: y\n\
\n\
Path prompt\n\
~EET\\EET.TP2~ #0 #0 // EET core (resource importation): v14.0 // @wlb-inputs: C:\\My Games\\BG2\n\
\n\
Two numeric prompts in sequence\n\
~VIENXAY\\VIENXAY.TP2~ #0 #0 // Vienxay NPC for BG1EE: 1.67 // @wlb-inputs: 1,2\n\
\n\
Multiple yes/no/accept/cancel prompts\n\
~SOMEMOD\\SETUP-SOMEMOD.TP2~ #0 #10 // Confirm install: v1.0 // @wlb-inputs: y,n,a,c\n\
\n\
Prompt sequence with blank input\n\
~ANOTHERMOD\\SETUP-ANOTHERMOD.TP2~ #0 #0 // Optional prompt: v2.0 // @wlb-inputs: 1,,y";
            let copy_resp = ui.small_button("Click Me To Copy Prompt Input Tag");
            copy_resp.clone().on_hover_ui(|ui| {
                ui.set_max_width(1400.0);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.label(prompt_help);
            });
            if copy_resp.clicked() {
                ui.ctx().copy_text("// @wlb-inputs:".to_string());
            }
            ui.checkbox(&mut s.have_weidu_logs, "Have WeiDU Logs?")
                .on_hover_text("Use existing weidu.log files as input source (skip scan/select/order).");
            ui.checkbox(&mut s.weidu_log_mode_enabled, "Enable WeiDU Logging Options (-u)")
                .on_hover_text("Enable/disable WeiDU logging flags (autolog/logapp/log-extern/log component).");

            ui.horizontal(|ui| {
                ui.checkbox(&mut s.lookback_enabled, "Prompt context lookback")
                    .on_hover_text("Keep this many prior output lines for prompt detection and context.");
                ui.add_enabled(
                    s.lookback_enabled,
                    egui::DragValue::new(&mut s.lookback).range(1..=5000),
                );
            });
            let show_backup_checkbox = if s.game_install == "EET" {
                s.new_pre_eet_dir_enabled || s.new_eet_dir_enabled
            } else {
                s.generate_directory_enabled
            };
            if show_backup_checkbox {
                ui.checkbox(
                    &mut s.backup_targets_before_eet_copy,
                    "Backup target dirs before -p/-n/-g run",
                )
                .on_hover_text(
                    "If target dir has files, move it to a timestamped backup folder and recreate an empty target before copy.",
                );
            } else {
                s.backup_targets_before_eet_copy = false;
            }
        });

        cols[2].group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(204.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            super::section_title(ui, "Flags");
            ui.checkbox(&mut s.skip_installed, "-s Skip installed")
                .on_hover_text("Skip components already present in WeiDU logs.");
            ui.checkbox(&mut s.check_last_installed, "-c Check last installed")
                .on_hover_text("Use strict last-installed validation.");
            if s.game_install == "EET" {
                ui.checkbox(&mut s.new_pre_eet_dir_enabled, "-p Clone BGEE -> Pre-EET target")
                    .on_hover_text("Copy from Source BGEE Folder into Pre-EET Directory.");
            } else {
                s.new_pre_eet_dir_enabled = false;
            }
            ui.checkbox(&mut s.abort_on_warnings, "-a Abort on warnings")
                .on_hover_text("Abort install when warnings are encountered.");
            ui.checkbox(&mut s.strict_matching, "-x Strict matching")
                .on_hover_text("Require strict component/version matching.");
            if s.game_install == "EET" {
                ui.checkbox(&mut s.new_eet_dir_enabled, "-n Clone BG2EE -> EET target")
                    .on_hover_text("Copy from Source BG2EE Folder into New EET Directory.");
            } else {
                s.new_eet_dir_enabled = false;
            }
            ui.checkbox(&mut s.download, "--download Missing mods")
                .on_hover_text("Prompt for download URI when a mod is missing.");
            ui.checkbox(&mut s.overwrite, "-o Overwrite mod folder")
                .on_hover_text("Force copy mod folders even when they already exist.");
            if s.game_install != "EET" {
                ui.checkbox(
                    &mut s.generate_directory_enabled,
                    "-g Clone source game -> target directory",
                )
                .on_hover_text("Copy from Source Game Folder into Generate Directory.");
            } else {
                s.generate_directory_enabled = false;
            }
        });
    });
}
