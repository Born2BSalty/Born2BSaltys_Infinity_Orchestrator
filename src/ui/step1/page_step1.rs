// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::layout::SECTION_GAP;
use crate::ui::step1::action_step1::Step1Action;
use crate::ui::step1::frame_step1::{render_bottom, render_top};
use crate::ui::step1::service_step1::{
    split_path_check_lines, sync_install_mode, sync_weidu_log_mode,
};
use crate::ui::step1::state_step1::clear_path_check_if_step1_changed;
use crate::ui::step5::service_diagnostics_support_step5::{
    export_diagnostics, restart_app_with_diagnostics,
};

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step1Action> {
    sync_install_mode(&mut state.step1);
    let before = state.step1.clone();
    sync_weidu_log_mode(&mut state.step1);
    let mut step1_action = None;
    let github_button_label = if state.github_auth_running {
        "GitHub: Waiting...".to_string()
    } else if state.github_auth_login.trim().is_empty() {
        "Connect GitHub".to_string()
    } else {
        format!("GitHub: {}", state.github_auth_login.trim())
    };

    ui.horizontal(|ui| {
        ui.heading("Step 1: Setup");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if dev_mode {
                if ui.button("Export diagnostics").clicked() {
                    match export_diagnostics(state, None, dev_mode, exe_fingerprint) {
                        Ok(path) => {
                            state.step5.last_status_text =
                                format!("Diagnostics exported: {}", path.display());
                        }
                        Err(err) => {
                            state.step5.last_status_text =
                                format!("Diagnostics export failed: {err}");
                        }
                    }
                }
            } else if ui.button("Restart App With Diagnostics").clicked() {
                restart_app_with_diagnostics(state);
            }
            let paths_checked = matches!(state.step1_path_check, Some((true, _)));
            let import_response =
                ui.add_enabled(paths_checked, egui::Button::new("Import Modlist…"));
            if !paths_checked {
                import_response.on_hover_text("Run Test Paths first.");
                ui.label(crate::ui::shared::typography_global::weak(
                    "Run Test Paths first.",
                ));
            } else if import_response.clicked() {
                state.modlist_import_window_open = true;
                state.modlist_import_preview_mode = false;
            }
        });
    });
    ui.label("Choose game mode, paths, and installer options.");
    if let Some((ok, msg)) = state.step1_path_check.clone() {
        ui.add_space(4.0);
        ui.group(|ui| {
            ui.label(crate::ui::shared::typography_global::strong("Path Check"));
            if ok {
                ui.label(
                    crate::ui::shared::typography_global::plain(format!("- {msg}"))
                        .color(crate::ui::shared::theme_global::success_bright()),
                );
            } else {
                for line in split_path_check_lines(&msg) {
                    ui.label(
                        crate::ui::shared::typography_global::plain(format!("- {}", line))
                            .color(crate::ui::shared::theme_global::error()),
                    );
                }
            }
        });
    }
    ui.add_space(SECTION_GAP);

    egui::ScrollArea::vertical().show(ui, |ui| {
        render_top(
            ui,
            &mut state.step1,
            dev_mode,
            github_button_label.as_str(),
            &mut step1_action,
        );
        ui.add_space(SECTION_GAP);
        render_bottom(ui, &mut state.step1);
    });
    render_modlist_import_popup(ui, state);

    let step1_changed = state.step1 != before;
    clear_path_check_if_step1_changed(state, step1_changed);
    if step1_action.is_some() {
        step1_action
    } else if step1_changed {
        Some(Step1Action::PathsChanged)
    } else {
        None
    }
}

fn render_modlist_import_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    let mut open = state.modlist_import_window_open;
    if !open {
        return;
    }
    egui::Window::new("Import Modlist Share Code")
        .open(&mut open)
        .resizable(true)
        .default_size(egui::vec2(720.0, 420.0))
        .show(ui.ctx(), |ui| {
            if state.modlist_import_preview_mode {
                ui.label(crate::ui::shared::typography_global::strong(
                    "BIO Modlist Import Preview",
                ));
                ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_XS);
                render_modlist_import_tabs(ui, state);
                ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_XS);
                let preview_height = (ui.available_height() - 48.0).max(160.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(preview_height)
                    .show(ui, |ui| {
                        ui.label(crate::ui::shared::typography_global::monospace(
                            modlist_import_tab_text(state),
                        ));
                    });
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        state.modlist_import_preview_mode = false;
                    }
                    if ui
                        .add_enabled(
                            state.modlist_import_ready,
                            egui::Button::new("Import Modlist"),
                        )
                        .clicked()
                    {
                        let code = state.modlist_import_code.clone();
                        match crate::app::modlist_share::import_modlist_share_code(state, &code) {
                            Ok(preview) => {
                                state.modlist_import_preview =
                                    format_modlist_import_preview(&preview);
                                state.modlist_import_error.clear();
                                state.modlist_import_window_open = false;
                                state.modlist_import_preview_mode = false;
                                start_modlist_auto_build(state);
                            }
                            Err(err) => {
                                state.modlist_import_error = err;
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        state.modlist_import_window_open = false;
                        state.modlist_import_preview_mode = false;
                    }
                });
            } else {
                ui.label("Paste a BIO-MODLIST-V1 share code.");
                ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_XS);
                let text_height = (ui.available_height() - 110.0).max(180.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(text_height)
                    .show(ui, |ui| {
                        ui.add_sized(
                            [ui.available_width(), text_height],
                            egui::TextEdit::multiline(&mut state.modlist_import_code).code_editor(),
                        );
                    });
                if !state.modlist_import_error.trim().is_empty() {
                    ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_XS);
                    ui.label(
                        crate::ui::shared::typography_global::plain(&state.modlist_import_error)
                            .color(crate::ui::shared::theme_global::error()),
                    );
                }
                ui.horizontal(|ui| {
                    if ui.button("Preview").clicked() {
                        match crate::app::modlist_share::preview_modlist_share_code(
                            &state.modlist_import_code,
                        ) {
                            Ok(preview) => {
                                state.step1.game_install = preview.game_install.clone();
                                state.step1.install_mode = preview.install_mode.clone();
                                state.step1.sync_install_mode_flags();
                                state.step1_path_check = None;
                                state.modlist_import_preview =
                                    format_modlist_import_preview(&preview);
                                state.modlist_import_preview_bgee_log =
                                    preview.bgee_log_text.clone();
                                state.modlist_import_preview_bg2ee_log =
                                    preview.bg2ee_log_text.clone();
                                state.modlist_import_preview_source_overrides =
                                    preview.source_overrides_text.clone();
                                state.modlist_import_preview_installed_refs =
                                    preview.installed_refs_text.clone();
                                state.modlist_import_preview_mod_configs =
                                    preview.mod_configs_text.clone();
                                state.modlist_import_error.clear();
                                state.modlist_import_ready = true;
                                state.modlist_import_preview_mode = true;
                                state.modlist_import_preview_tab = "Summary".to_string();
                            }
                            Err(err) => {
                                state.modlist_import_preview.clear();
                                state.modlist_import_preview_bgee_log.clear();
                                state.modlist_import_preview_bg2ee_log.clear();
                                state.modlist_import_preview_source_overrides.clear();
                                state.modlist_import_preview_installed_refs.clear();
                                state.modlist_import_preview_mod_configs.clear();
                                state.modlist_import_error = err;
                                state.modlist_import_ready = false;
                                state.modlist_import_preview_mode = false;
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        state.modlist_import_window_open = false;
                    }
                });
            }
        });
    state.modlist_import_window_open = open && state.modlist_import_window_open;
    if !state.modlist_import_window_open {
        state.modlist_import_preview_mode = false;
    }
}

fn render_modlist_import_tabs(ui: &mut egui::Ui, state: &mut WizardState) {
    ui.horizontal_wrapped(|ui| {
        for tab in [
            "Summary",
            "BGEE WeiDU",
            "BG2EE WeiDU",
            "User Downloads",
            "Installed Refs",
            "Mod Configs",
        ] {
            let selected = state.modlist_import_preview_tab == tab;
            if ui.selectable_label(selected, tab).clicked() {
                state.modlist_import_preview_tab = tab.to_string();
            }
        }
    });
}

fn start_modlist_auto_build(state: &mut WizardState) {
    state.modlist_auto_build_active = true;
    state.modlist_auto_build_waiting_for_install = false;
    state.current_step = 1;
    state.step2.active_game_tab = if state.step1.game_install == "BGEE" {
        "BGEE".to_string()
    } else {
        "BG2EE".to_string()
    };
    state.step2.pending_saved_log_apply = true;
    state.step2.pending_saved_log_update_preview = true;
    state.step2.pending_saved_log_download = true;
    state.step2.update_selected_popup_open = true;
    state.step2.scan_status = "Auto Build: preparing imported modlist".to_string();
    state.step5.last_status_text = "Auto Build: preparing imported modlist".to_string();
}

fn modlist_import_tab_text(state: &WizardState) -> String {
    match state.modlist_import_preview_tab.as_str() {
        "BGEE WeiDU" => empty_tab_fallback(
            &state.modlist_import_preview_bgee_log,
            "No BGEE WeiDU log included.",
        ),
        "BG2EE WeiDU" => empty_tab_fallback(
            &state.modlist_import_preview_bg2ee_log,
            "No BG2EE WeiDU log included.",
        ),
        "User Downloads" => empty_tab_fallback(
            &state.modlist_import_preview_source_overrides,
            "No user download overrides included.",
        ),
        "Installed Refs" => empty_tab_fallback(
            &state.modlist_import_preview_installed_refs,
            "No installed refs included.",
        ),
        "Mod Configs" => empty_tab_fallback(
            &state.modlist_import_preview_mod_configs,
            "No mod config files included.",
        ),
        _ => state.modlist_import_preview.clone(),
    }
}

fn empty_tab_fallback(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn format_modlist_import_preview(
    preview: &crate::app::modlist_share::ModlistSharePreview,
) -> String {
    format!(
        "Modlist\nBIO version: {}\nGame install: {}\nInstall mode: {}\n\nWeiDU Logs\nBGEE: {} entries\nBG2EE: {} entries\n\nIncluded Data\nSource overrides: {}\nInstalled refs / pins: {}\nMod config files: {}\n\nWhat Import Will Do\n- Set Step 1 game/install mode from this share code.\n- Write imported WeiDU logs to the Step 1 WeiDU log paths.\n- Import source overrides if included.\n- Import installed refs/pins if included.\n- Store pending mod config files if included.\n- Keep your local game, mods, archive, and backup paths unchanged.\n\nAfter Import\n- Click Next.\n- Review the imported WeiDU order.\n- Run Check Updates.\n- Download/extract missing mods.",
        preview.bio_version,
        preview.game_install,
        preview.install_mode,
        preview.bgee_entries,
        preview.bg2ee_entries,
        if preview.has_source_overrides {
            "Yes"
        } else {
            "No"
        },
        if preview.has_installed_refs {
            "Yes"
        } else {
            "No"
        },
        preview.mod_config_count,
    )
}
