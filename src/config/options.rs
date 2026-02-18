// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use crate::cli::args::{Cli, Command, CommonOptions, ScanCommand};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CoreOptions {
    pub weidu_binary: PathBuf,
    pub mod_directories: PathBuf,
    pub language: String,
    pub depth: usize,
    pub skip_installed: bool,
    pub abort_on_warnings: bool,
    pub timeout: usize,
    pub weidu_log_mode: String,
    pub strict_matching: bool,
    pub download: bool,
    pub overwrite: bool,
    pub check_last_installed: bool,
    pub tick: u64,
    pub lookback: usize,
    pub casefold: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct NormalConfig {
    pub log_file: PathBuf,
    pub game_directory: PathBuf,
    pub generate_directory: Option<PathBuf>,
    pub options: CoreOptions,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EetConfig {
    pub bg1_game_directory: PathBuf,
    pub bg1_log_file: PathBuf,
    pub bg2_game_directory: PathBuf,
    pub bg2_log_file: PathBuf,
    pub new_pre_eet_dir: Option<PathBuf>,
    pub new_eet_dir: Option<PathBuf>,
    pub options: CoreOptions,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ScanConfig {
    Components {
        game_directory: PathBuf,
        filter_by_selected_language: String,
        options: CoreOptions,
    },
    Languages {
        filter_by_selected_language: String,
        options: CoreOptions,
    },
}

#[derive(Debug, Clone)]
pub enum AppCommandConfig {
    Gui { dev_mode: bool },
    Normal(NormalConfig),
    Eet(EetConfig),
    Scan(ScanConfig),
}

pub fn from_cli(cli: &Cli) -> Option<AppCommandConfig> {
    let command = cli.command.as_ref()?;
    match command {
        Command::Gui => Some(AppCommandConfig::Gui {
            dev_mode: cli.dev_mode,
        }),
        Command::Normal(args) => Some(AppCommandConfig::Normal(NormalConfig {
            log_file: PathBuf::from(&args.log_file),
            game_directory: PathBuf::from(&args.game_directory),
            generate_directory: args.generate_directory.as_deref().map(PathBuf::from),
            options: map_common(&args.options),
        })),
        Command::Eet(args) => Some(AppCommandConfig::Eet(EetConfig {
            bg1_game_directory: PathBuf::from(&args.bg1_game_directory),
            bg1_log_file: PathBuf::from(&args.bg1_log_file),
            bg2_game_directory: PathBuf::from(&args.bg2_game_directory),
            bg2_log_file: PathBuf::from(&args.bg2_log_file),
            new_pre_eet_dir: args.new_pre_eet_dir.as_deref().map(PathBuf::from),
            new_eet_dir: args.new_eet_dir.as_deref().map(PathBuf::from),
            options: map_common(&args.options),
        })),
        Command::Scan(scan) => match &scan.command {
            ScanCommand::Components(args) => Some(AppCommandConfig::Scan(ScanConfig::Components {
                game_directory: PathBuf::from(&args.game_directory),
                filter_by_selected_language: args.filter_by_selected_language.clone(),
                options: map_common(&args.options),
            })),
            ScanCommand::Languages(args) => Some(AppCommandConfig::Scan(ScanConfig::Languages {
                filter_by_selected_language: args.filter_by_selected_language.clone(),
                options: map_common(&args.options),
            })),
        },
    }
}

fn map_common(options: &CommonOptions) -> CoreOptions {
    CoreOptions {
        weidu_binary: PathBuf::from(&options.weidu_binary),
        mod_directories: PathBuf::from(&options.mod_directories),
        language: options.language.clone(),
        depth: options.depth,
        skip_installed: options.skip_installed,
        abort_on_warnings: options.abort_on_warnings,
        timeout: options.timeout,
        weidu_log_mode: options.weidu_log_mode.clone(),
        strict_matching: options.strict_matching,
        download: options.download,
        overwrite: options.overwrite,
        check_last_installed: options.check_last_installed,
        tick: options.tick,
        lookback: options.lookback,
        casefold: options.casefold,
    }
}
