// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use rfd::FileDialog;

use crate::app::state::Step1State;
use crate::ui::layout::{
    BROWSE_BUTTON_WIDTH, PATH_FIELD_MIN_WIDTH, PATH_INPUT_HEIGHT, PATH_LABEL_WIDTH,
    PATH_ROW_INNER_GAP,
};
use crate::ui::shared::tooltip_global as tt;
use crate::ui::shared::typography_global as typo;
use crate::ui::step1::action_step1::Step1Action;
use crate::ui::step1::service_step1::{sync_install_mode, sync_weidu_log_mode};

pub fn render_game_selection_content(ui: &mut egui::Ui, s: &mut Step1State) {
    section_title(ui, "Game Selection");
    ui.horizontal(|ui| {
        ui.label("Game Install")
            .on_hover_text(tt::STEP1_GAME_INSTALL);
        egui::ComboBox::from_id_salt("game_install")
            .selected_text(&s.game_install)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.game_install, "BGEE".to_string(), "BGEE");
                ui.selectable_value(&mut s.game_install, "BG2EE".to_string(), "BG2EE");
                ui.selectable_value(&mut s.game_install, "EET".to_string(), "EET");
            });

        ui.add_space(12.0);
        ui.label("Language");
        egui::ComboBox::from_id_salt("install_language")
            .selected_text(step1_language_label(&s.language))
            .show_ui(ui, |ui| {
                for (value, label) in step1_language_options() {
                    ui.selectable_value(&mut s.language, value.to_string(), *label);
                }
            });
    });
}

fn step1_language_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("en_US", "English"),
        ("de_DE", "German"),
        ("fr_FR", "French"),
        ("es_ES", "Spanish"),
        ("it_IT", "Italian"),
        ("pl_PL", "Polish"),
        ("pt_BR", "Portuguese"),
        ("cs_CZ", "Czech"),
        ("tr_TR", "Turkish"),
        ("uk_UA", "Ukrainian"),
        ("ru_RU", "Russian"),
    ]
}

fn step1_language_label(value: &str) -> String {
    step1_language_options()
        .iter()
        .find_map(|(locale, label)| locale.eq_ignore_ascii_case(value).then_some(*label))
        .unwrap_or(value)
        .to_string()
}

pub fn render_advanced_options_content(ui: &mut egui::Ui, s: &mut Step1State, dev_mode: bool) {
    section_title(ui, "Advanced Options");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.custom_scan_depth, "Custom scan depth")
            .on_hover_text(tt::STEP1_CUSTOM_SCAN_DEPTH);
        ui.add_enabled(
            s.custom_scan_depth,
            egui::DragValue::new(&mut s.depth).range(1..=10),
        );
    });
    ui.horizontal(|ui| {
        ui.checkbox(
            &mut s.timeout_per_mod_enabled,
            "Mod install timeout (off = 3600s default)",
        )
        .on_hover_text(tt::STEP1_TIMEOUT_PER_MOD);
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
        .on_hover_text(tt::STEP1_AUTO_ANSWER_INITIAL_DELAY);
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
        .on_hover_text(tt::STEP1_AUTO_ANSWER_POST_SEND_DELAY);
        ui.add_enabled(
            s.auto_answer_post_send_delay_enabled,
            egui::DragValue::new(&mut s.auto_answer_post_send_delay_ms).range(500..=15000),
        );
    });
    if dev_mode {
        ui.horizontal(|ui| {
            ui.checkbox(&mut s.tick_dev_enabled, "Tick (dev)")
                .on_hover_text(tt::STEP1_TICK_DEV);
            ui.add_enabled(
                s.tick_dev_enabled,
                egui::DragValue::new(&mut s.tick).range(50..=5000),
            );
        });
    }
}

pub fn render_options_content(
    ui: &mut egui::Ui,
    s: &mut Step1State,
    github_button_label: &str,
    action: &mut Option<Step1Action>,
) {
    section_title(ui, "Options");
    sync_install_mode(s);
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
        ui.set_max_width(tt::DEFAULT_TOOLTIP_MAX_WIDTH);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.label(prompt_help);
    });
    if copy_resp.clicked() {
        ui.ctx().copy_text("// @wlb-inputs:".to_string());
    }
    if ui
        .button(github_button_label)
        .on_hover_text("Connect BIO to GitHub using browser-based authorization.")
        .clicked()
    {
        *action = Some(Step1Action::ConnectGitHub);
    }
    ui.horizontal(|ui| {
        ui.add_space(100.0);
        ui.label(typo::strong("Install Mode"));
    });
    egui::ComboBox::from_id_salt("install_mode")
        .selected_text(install_mode_label(&s.install_mode))
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut s.install_mode,
                Step1State::INSTALL_MODE_BUILD_FROM_SCANNED_MODS.to_string(),
                "Build from scanned mods",
            );
            ui.selectable_value(
                &mut s.install_mode,
                Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS.to_string(),
                "Install exactly from WeiDU logs",
            );
            ui.selectable_value(
                &mut s.install_mode,
                Step1State::INSTALL_MODE_WEIDU_LOGS_REVIEW_EDIT.to_string(),
                "Start from WeiDU logs, then review/edit",
            );
        });
    sync_install_mode(s);
    ui.checkbox(
        &mut s.weidu_log_mode_enabled,
        "Enable WeiDU Logging Options (-u)",
    )
    .on_hover_text(tt::STEP1_WEIDU_LOG_MODE);
    ui.checkbox(
        &mut s.prompt_required_sound_enabled,
        "Sound cue when prompt input is required",
    )
    .on_hover_text(tt::STEP1_PROMPT_REQUIRED_SOUND);
    ui.checkbox(&mut s.download_archive, "Download Missing Mods and Keep Archives")
        .on_hover_text(tt::STEP1_DOWNLOAD_ARCHIVE);

    ui.horizontal(|ui| {
        ui.checkbox(&mut s.lookback_enabled, "Prompt context lookback")
            .on_hover_text(tt::STEP1_PROMPT_CONTEXT_LOOKBACK);
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
            &mut s.prepare_target_dirs_before_install,
            "Prepare target dirs before -p/-n/-g install",
        )
        .on_hover_text(tt::STEP1_PREPARE_TARGET_DIRS);
        ui.add_enabled_ui(s.prepare_target_dirs_before_install, |ui| {
            ui.checkbox(
                &mut s.backup_targets_before_eet_copy,
                "Backup target dirs before -p/-n/-g run",
            )
            .on_hover_text(tt::STEP1_BACKUP_TARGET_DIRS);
        });
        if !s.prepare_target_dirs_before_install {
            s.backup_targets_before_eet_copy = false;
        }
    } else {
        s.prepare_target_dirs_before_install = false;
        s.backup_targets_before_eet_copy = false;
    }
}

pub fn render_flags_content(ui: &mut egui::Ui, s: &mut Step1State) {
    section_title(ui, "Flags");
    ui.checkbox(&mut s.skip_installed, "-s Skip installed")
        .on_hover_text(tt::STEP1_SKIP_INSTALLED);
    ui.checkbox(&mut s.check_last_installed, "-c Check last installed")
        .on_hover_text(tt::STEP1_CHECK_LAST_INSTALLED);
    if s.game_install == "EET" {
        ui.checkbox(
            &mut s.new_pre_eet_dir_enabled,
            "-p Clone BGEE -> Pre-EET target",
        )
        .on_hover_text(tt::STEP1_CLONE_BGEE_PRE_EET);
    } else {
        s.new_pre_eet_dir_enabled = false;
    }
    ui.checkbox(&mut s.abort_on_warnings, "-a Abort on warnings")
        .on_hover_text(tt::STEP1_ABORT_ON_WARNINGS);
    ui.checkbox(&mut s.strict_matching, "-x Strict matching")
        .on_hover_text(tt::STEP1_STRICT_MATCHING);
    if s.game_install == "EET" {
        ui.checkbox(&mut s.new_eet_dir_enabled, "-n Clone BG2EE -> EET target")
            .on_hover_text(tt::STEP1_CLONE_BG2EE_EET);
    } else {
        s.new_eet_dir_enabled = false;
    }
    ui.checkbox(&mut s.download, "--download Missing mods")
        .on_hover_text(tt::STEP1_DOWNLOAD_MISSING);
    ui.checkbox(&mut s.overwrite, "-o Overwrite mod folder")
        .on_hover_text(tt::STEP1_OVERWRITE_MOD_FOLDER);
    if s.game_install != "EET" {
        ui.checkbox(
            &mut s.generate_directory_enabled,
            "-g Clone source game -> target directory",
        )
        .on_hover_text(tt::STEP1_CLONE_SOURCE_TARGET);
    } else {
        s.generate_directory_enabled = false;
    }
}

pub fn render_mods_folder_content(ui: &mut egui::Ui, s: &mut Step1State) {
    section_title(ui, "Mods Folder");
    path_row_dir(ui, "Your Mods Folder", &mut s.mods_folder);
}

pub fn render_tools_content(ui: &mut egui::Ui, s: &mut Step1State) {
    section_title(ui, "Tools");
    path_row_file(ui, "WeiDU Binary", &mut s.weidu_binary);
    path_row_file(ui, "mod_installer Binary", &mut s.mod_installer_binary);
}

pub fn render_mods_archive_content(ui: &mut egui::Ui, s: &mut Step1State) {
    section_title(ui, "Mods Archive");
    path_row_dir(ui, "Mods Archive", &mut s.mods_archive_folder);
}

pub fn render_install_paths_content(ui: &mut egui::Ui, s: &mut Step1State, _max_height: f32) {
    let title = match s.game_install.as_str() {
        "BG2EE" => "Install Paths BG2EE:",
        "EET" => "Install Paths EET:",
        _ => "Install Paths BGEE:",
    };
    section_title(ui, title);
    ui.add_space(10.0);
    match s.game_install.as_str() {
        "BG2EE" => render_bg2ee_paths(ui, s),
        "EET" => render_eet_paths(ui, s),
        _ => render_bgee_paths(ui, s),
    }
}

pub fn render_weidu_log_mode_content(ui: &mut egui::Ui, s: &mut Step1State, _max_height: f32) {
    section_title(ui, "WeiDU Log Mode");
    ui.horizontal(|ui| {
        ui.add_space(180.0);
        ui.scope(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            ui.checkbox(&mut s.weidu_log_autolog, "autolog");
            ui.checkbox(&mut s.weidu_log_logapp, "logapp");
            ui.checkbox(&mut s.weidu_log_logextern, "log-extern");
            ui.checkbox(&mut s.weidu_log_log_component, "log (per-component)");
        });
    });
    ui.add_space(4.0);
    path_row_dir(ui, "Per-component folder", &mut s.weidu_log_folder);
    sync_weidu_log_mode(s);
}

fn render_bg2ee_paths(ui: &mut egui::Ui, s: &mut Step1State) {
    if s.generate_directory_enabled {
        ui.label(typo::weak("Using -g: source + generated target."));
    }
    path_row_dir(ui, "BG2EE Game Folder", &mut s.bg2ee_game_folder);
    if s.installs_exactly_from_weidu_logs() {
        path_row_file(ui, "BG2EE WeiDU Log File", &mut s.bg2ee_log_file);
    } else if !s.bootstraps_from_weidu_logs() {
        path_row_dir(ui, "BG2EE WeiDU Log Folder", &mut s.bg2ee_log_folder);
    }
    if s.generate_directory_enabled {
        path_row_dir(ui, "Generate Directory (-g)", &mut s.generate_directory);
    }
}

fn render_eet_paths(ui: &mut egui::Ui, s: &mut Step1State) {
    if s.new_pre_eet_dir_enabled {
        ui.label(typo::weak("Using -p: source BGEE -> Pre-EET target."));
        path_row_dir(ui, "Source BGEE Folder (-p)", &mut s.bgee_game_folder);
        path_row_dir(ui, "Pre-EET Directory", &mut s.eet_pre_dir);
    } else {
        path_row_dir(ui, "BGEE Game Folder", &mut s.eet_bgee_game_folder);
    }
    if s.installs_exactly_from_weidu_logs() {
        path_row_file(ui, "BGEE WeiDU Log File", &mut s.bgee_log_file);
    } else if !s.bootstraps_from_weidu_logs() {
        path_row_dir(ui, "BGEE WeiDU Log Folder", &mut s.eet_bgee_log_folder);
    }
    if s.new_eet_dir_enabled {
        ui.label(typo::weak("Using -n: source BG2EE -> New EET target."));
        path_row_dir(ui, "Source BG2EE Folder (-n)", &mut s.bg2ee_game_folder);
        path_row_dir(ui, "New EET Directory", &mut s.eet_new_dir);
    } else {
        path_row_dir(ui, "BG2EE Game Folder", &mut s.eet_bg2ee_game_folder);
    }
    if s.installs_exactly_from_weidu_logs() {
        path_row_file(ui, "BG2EE WeiDU Log File", &mut s.bg2ee_log_file);
    } else if !s.bootstraps_from_weidu_logs() {
        path_row_dir(ui, "BG2EE WeiDU Log Folder", &mut s.eet_bg2ee_log_folder);
    }
}

fn render_bgee_paths(ui: &mut egui::Ui, s: &mut Step1State) {
    if s.generate_directory_enabled {
        ui.label(typo::weak("Using -g: source + generated target."));
    }
    path_row_dir(ui, "BGEE Game Folder", &mut s.bgee_game_folder);
    if s.installs_exactly_from_weidu_logs() {
        path_row_file(ui, "BGEE WeiDU Log File", &mut s.bgee_log_file);
    } else if !s.bootstraps_from_weidu_logs() {
        path_row_dir(ui, "BGEE WeiDU Log Folder", &mut s.bgee_log_folder);
    }
    if s.generate_directory_enabled {
        path_row_dir(ui, "Generate Directory (-g)", &mut s.generate_directory);
    }
}

fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(crate::ui::shared::typography_global::section_title(text));
}

fn install_mode_label(value: &str) -> &'static str {
    match value {
        Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS => "Install exactly from WeiDU logs",
        Step1State::INSTALL_MODE_WEIDU_LOGS_REVIEW_EDIT => {
            "Start from WeiDU logs, then review/edit"
        }
        _ => "Build from scanned mods",
    }
}

fn path_row_dir(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        right_label(ui, label);
        let text_width = (ui.available_width() - BROWSE_BUTTON_WIDTH - PATH_ROW_INNER_GAP)
            .max(PATH_FIELD_MIN_WIDTH);
        ui.add_sized(
            [text_width, PATH_INPUT_HEIGHT],
            egui::TextEdit::singleline(value).clip_text(true),
        );
        if ui
            .add_sized(
                [BROWSE_BUTTON_WIDTH, PATH_INPUT_HEIGHT],
                egui::Button::new("Browse"),
            )
            .clicked()
            && let Some(path) = FileDialog::new().pick_folder()
        {
            *value = path.display().to_string();
        }
    });
}

fn path_row_file(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        right_label(ui, label);
        let text_width = (ui.available_width() - BROWSE_BUTTON_WIDTH - PATH_ROW_INNER_GAP)
            .max(PATH_FIELD_MIN_WIDTH);
        ui.add_sized(
            [text_width, PATH_INPUT_HEIGHT],
            egui::TextEdit::singleline(value).clip_text(true),
        );
        if ui
            .add_sized(
                [BROWSE_BUTTON_WIDTH, PATH_INPUT_HEIGHT],
                egui::Button::new("Browse"),
            )
            .clicked()
            && let Some(path) = FileDialog::new().pick_file()
        {
            *value = path.display().to_string();
        }
    });
}

fn right_label(ui: &mut egui::Ui, label: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(PATH_LABEL_WIDTH, PATH_INPUT_HEIGHT),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.label(typo::strong(format!("{label}:")));
        },
    );
}
