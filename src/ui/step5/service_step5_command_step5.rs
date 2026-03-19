// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod common_args {
    use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};
    use crate::ui::state::Step1State;

    pub(crate) fn installer_program(step1: &Step1State) -> String {
        resolve_mod_installer_binary(&step1.mod_installer_binary)
    }

    pub(crate) fn append_common_args(step1: &Step1State, args: &mut Vec<String>) {
        let bool_s = |v: bool| if v { "true" } else { "false" }.to_string();
        args.push("--language".to_string());
        args.push(step1.language.clone());
        args.push("--mod-directories".to_string());
        args.push(step1.mods_folder.clone());
        args.push("--weidu-binary".to_string());
        args.push(resolve_weidu_binary(&step1.weidu_binary));
        if step1.weidu_log_mode_enabled {
            args.push("--weidu-log-mode".to_string());
            args.push(step1.weidu_log_mode.clone());
        }
        args.push("--skip-installed".to_string());
        args.push(bool_s(step1.skip_installed));
        args.push("--strict-matching".to_string());
        args.push(bool_s(step1.strict_matching));
        args.push("--download".to_string());
        args.push(bool_s(step1.download));
        args.push("--overwrite".to_string());
        args.push(bool_s(step1.overwrite));
        args.push("--abort-on-warnings".to_string());
        args.push(bool_s(step1.abort_on_warnings));
        args.push("--check-last-installed".to_string());
        args.push(bool_s(step1.check_last_installed));
        if step1.custom_scan_depth {
            args.push("--depth".to_string());
            args.push(step1.depth.to_string());
        }
        if step1.timeout_per_mod_enabled {
            args.push("--timeout".to_string());
            args.push(step1.timeout.to_string());
        }
        if step1.lookback_enabled {
            args.push("--lookback".to_string());
            args.push(step1.lookback.to_string());
        }
        if step1.tick_dev_enabled {
            args.push("--tick".to_string());
            args.push(step1.tick.to_string());
        }
        if step1.casefold {
            args.push("--casefold".to_string());
            args.push("true".to_string());
        }
    }
}

mod log_paths {
    use crate::platform_defaults::compose_weidu_log_path;
    use crate::ui::state::Step1State;

    pub(crate) fn resolve_bgee_log_file(step1: &Step1State) -> String {
        if step1.have_weidu_logs && !step1.bgee_log_file.trim().is_empty() {
            return step1.bgee_log_file.trim().to_string();
        }
        let folder = if step1.game_install == "EET" {
            step1.eet_bgee_log_folder.trim()
        } else {
            step1.bgee_log_folder.trim()
        };
        compose_weidu_log_path(folder)
    }

    pub(crate) fn resolve_bg2_log_file(step1: &Step1State) -> String {
        if step1.have_weidu_logs && !step1.bg2ee_log_file.trim().is_empty() {
            return step1.bg2ee_log_file.trim().to_string();
        }
        let folder = if step1.game_install == "EET" {
            step1.eet_bg2ee_log_folder.trim()
        } else {
            step1.bg2ee_log_folder.trim()
        };
        compose_weidu_log_path(folder)
    }
}

mod builders_install {
    use crate::ui::state::Step1State;

    use super::common_args::{append_common_args, installer_program};
    use super::log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};

    pub fn build_install_invocation(step1: &Step1State) -> (String, Vec<String>) {
        let mut args: Vec<String> = Vec::new();
        let installer = installer_program(step1);
        if step1.game_install == "EET" {
            let bg1_source =
                if step1.new_pre_eet_dir_enabled && !step1.bgee_game_folder.trim().is_empty() {
                    step1.bgee_game_folder.trim()
                } else {
                    step1.eet_bgee_game_folder.trim()
                };
            let bg2_source =
                if step1.new_eet_dir_enabled && !step1.bg2ee_game_folder.trim().is_empty() {
                    step1.bg2ee_game_folder.trim()
                } else {
                    step1.eet_bg2ee_game_folder.trim()
                };
            args.push("eet".to_string());
            args.push("--bg1-game-directory".to_string());
            args.push(bg1_source.to_string());
            args.push("--bg1-log-file".to_string());
            args.push(resolve_bgee_log_file(step1));
            args.push("--bg2-game-directory".to_string());
            args.push(bg2_source.to_string());
            args.push("--bg2-log-file".to_string());
            args.push(resolve_bg2_log_file(step1));
            if step1.new_pre_eet_dir_enabled && !step1.eet_pre_dir.trim().is_empty() {
                args.push("--new-pre-eet-dir".to_string());
                args.push(step1.eet_pre_dir.trim().to_string());
            }
            if step1.new_eet_dir_enabled && !step1.eet_new_dir.trim().is_empty() {
                args.push("--new-eet-dir".to_string());
                args.push(step1.eet_new_dir.trim().to_string());
            }
        } else {
            args.push("normal".to_string());
            args.push("--game-directory".to_string());
            let game_dir = if step1.game_install == "BG2EE" {
                &step1.bg2ee_game_folder
            } else {
                &step1.bgee_game_folder
            };
            args.push(game_dir.to_string());
            args.push("--log-file".to_string());
            let log_file = if step1.game_install == "BG2EE" {
                resolve_bg2_log_file(step1)
            } else {
                resolve_bgee_log_file(step1)
            };
            args.push(log_file);
            if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
                args.push("--generate-directory".to_string());
                args.push(step1.generate_directory.trim().to_string());
            }
        }
        append_common_args(step1, &mut args);
        (installer, args)
    }
}

mod builders_resume {
    use crate::ui::state::{ResumeTargets, Step1State};

    use super::common_args::{append_common_args, installer_program};
    use super::log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};

    pub fn capture_resume_targets(step1: &Step1State) -> ResumeTargets {
        if step1.game_install == "EET" {
            ResumeTargets {
                bg1_game_dir: Some(
                    if step1.new_pre_eet_dir_enabled && !step1.eet_pre_dir.trim().is_empty() {
                        step1.eet_pre_dir.trim().to_string()
                    } else {
                        step1.eet_bgee_game_folder.trim().to_string()
                    },
                ),
                bg2_game_dir: Some(
                    if step1.new_eet_dir_enabled && !step1.eet_new_dir.trim().is_empty() {
                        step1.eet_new_dir.trim().to_string()
                    } else {
                        step1.eet_bg2ee_game_folder.trim().to_string()
                    },
                ),
                game_dir: None,
            }
        } else {
            ResumeTargets {
                bg1_game_dir: None,
                bg2_game_dir: None,
                game_dir: Some(
                    if step1.generate_directory_enabled
                        && !step1.generate_directory.trim().is_empty()
                    {
                        step1.generate_directory.trim().to_string()
                    } else if step1.game_install == "BG2EE" {
                        step1.bg2ee_game_folder.trim().to_string()
                    } else {
                        step1.bgee_game_folder.trim().to_string()
                    },
                ),
            }
        }
    }

    pub fn build_resume_invocation(
        step1: &Step1State,
        resume_targets: &ResumeTargets,
    ) -> (String, Vec<String>) {
        let mut args: Vec<String> = Vec::new();
        let installer = installer_program(step1);
        if step1.game_install == "EET" {
            let bg1_dir = resume_targets
                .bg1_game_dir
                .as_deref()
                .unwrap_or(step1.eet_bgee_game_folder.trim());
            let bg2_dir = resume_targets
                .bg2_game_dir
                .as_deref()
                .unwrap_or(step1.eet_bg2ee_game_folder.trim());
            args.push("eet".to_string());
            args.push("--bg1-game-directory".to_string());
            args.push(bg1_dir.to_string());
            args.push("--bg1-log-file".to_string());
            args.push(resolve_bgee_log_file(step1));
            args.push("--bg2-game-directory".to_string());
            args.push(bg2_dir.to_string());
            args.push("--bg2-log-file".to_string());
            args.push(resolve_bg2_log_file(step1));
        } else {
            args.push("normal".to_string());
            args.push("--game-directory".to_string());
            let game_dir = resume_targets.game_dir.as_deref().unwrap_or_else(|| {
                if step1.game_install == "BG2EE" {
                    step1.bg2ee_game_folder.trim()
                } else {
                    step1.bgee_game_folder.trim()
                }
            });
            args.push(game_dir.to_string());
            args.push("--log-file".to_string());
            let log_file = if step1.game_install == "BG2EE" {
                resolve_bg2_log_file(step1)
            } else {
                resolve_bgee_log_file(step1)
            };
            args.push(log_file);
        }
        append_common_args(step1, &mut args);
        force_skip_installed_on(&mut args);
        (installer, args)
    }

    fn force_skip_installed_on(args: &mut [String]) {
        if let Some(idx) = args.iter().position(|a| a == "--skip-installed")
            && let Some(value) = args.get_mut(idx + 1)
        {
            *value = "true".to_string();
        }
    }
}

mod builders_text {
    use crate::ui::state::Step1State;

    use super::builders_install::build_install_invocation;

    pub fn build_install_command(step1: &Step1State) -> String {
        let (program, args) = build_install_invocation(step1);
        let q = |v: &str| format!("\"{}\"", v);
        let mut parts: Vec<String> = vec![q(&program)];
        parts.extend(
            args.into_iter()
                .map(|a| if a.contains(' ') { q(&a) } else { a }),
        );
        parts.join(" ")
    }
}

mod display {
    use crate::ui::state::Step1State;

    use super::builders_text::build_install_command;

    pub fn build_command_preview_lines(step1: &Step1State) -> Vec<String> {
        build_install_command(step1)
            .split(" --")
            .enumerate()
            .map(|(i, part)| {
                if i == 0 {
                    part.to_string()
                } else {
                    format!("--{part}")
                }
            })
            .collect()
    }

    pub fn wrap_display_line(line: &str, max_cols: usize) -> Vec<String> {
        if line.chars().count() <= max_cols {
            return vec![line.to_string()];
        }
        let mut out = Vec::new();
        let mut current = String::new();
        for token in line.split_whitespace() {
            if current.is_empty() {
                current.push_str(token);
                continue;
            }
            if current.chars().count() + 1 + token.chars().count() <= max_cols {
                current.push(' ');
                current.push_str(token);
            } else {
                out.push(current);
                current = token.to_string();
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
        out
    }
}

pub(crate) use builders_install::build_install_invocation;
pub(crate) use builders_resume::{build_resume_invocation, capture_resume_targets};
pub(crate) use builders_text::build_install_command;
pub(crate) use display::{build_command_preview_lines, wrap_display_line};
