// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "BIO")]
#[command(version)]
#[command(about = "Born2BSalty's Infinity Orchestrator")]
#[command(disable_help_flag = true, disable_version_flag = true)]
pub struct Cli {
    #[arg(long, action = ArgAction::Help)]
    pub help: Option<bool>,
    #[arg(long, action = ArgAction::Version)]
    pub version: Option<bool>,

    #[arg(long, default_value = "info")]
    pub log_level: String,

    #[arg(short = 'd', long, default_value_t = false)]
    pub dev_mode: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Launch 5-step GUI shell
    #[command(name = "gui")]
    Gui,
    /// Normal install for (BG1EE,BG2EE,IWDEE)
    #[command(name = "normal")]
    Normal(NormalArgs),
    /// EET install for (eet)
    #[command(name = "eet")]
    Eet(EetArgs),
    /// Scan utilities
    #[command(name = "scan")]
    Scan(ScanArgs),
}

#[derive(Args, Debug)]
#[command(disable_help_flag = true)]
pub struct NormalArgs {
    #[arg(long, action = ArgAction::Help)]
    pub help: Option<bool>,

    #[arg(long, env = "LOG_FILE")]
    pub log_file: String,
    #[arg(long, env = "GAME_DIRECTORY")]
    pub game_directory: String,
    #[arg(long, env = "GENERATE_DIRECTORY")]
    pub generate_directory: Option<String>,
    #[command(flatten)]
    pub options: CommonOptions,
}

#[derive(Args, Debug)]
#[command(disable_help_flag = true)]
pub struct EetArgs {
    #[arg(long, action = ArgAction::Help)]
    pub help: Option<bool>,

    #[arg(long, env = "BG1_GAME_DIRECTORY")]
    pub bg1_game_directory: String,
    #[arg(long, env = "BG1_LOG_FILE")]
    pub bg1_log_file: String,
    #[arg(long, env = "BG2_GAME_DIRECTORY")]
    pub bg2_game_directory: String,
    #[arg(long, env = "BG2_LOG_FILE")]
    pub bg2_log_file: String,
    #[arg(long, env = "NEW_PRE_EET_DIR")]
    pub new_pre_eet_dir: Option<String>,
    #[arg(long, env = "NEW_EET_DIR")]
    pub new_eet_dir: Option<String>,
    #[command(flatten)]
    pub options: CommonOptions,
}

#[derive(Args, Debug)]
pub struct CommonOptions {
    #[arg(long, env = "WEIDU_BINARY", default_value = "")]
    pub weidu_binary: String,
    #[arg(long, env = "MOD_DIRECTORIES", default_value = ".")]
    pub mod_directories: String,
    #[arg(long, default_value = "en_US")]
    pub language: String,
    #[arg(long, env = "DEPTH", default_value_t = 5)]
    pub depth: usize,
    #[arg(
        long,
        env = "SKIP_INSTALLED",
        default_value_t = true,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub skip_installed: bool,
    #[arg(
        long,
        env = "ABORT_ON_WARNINGS",
        default_value_t = false,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub abort_on_warnings: bool,
    #[arg(long, env = "TIMEOUT", default_value_t = 3600)]
    pub timeout: usize,
    #[arg(long, env = "WEIDU_LOG_MODE", default_value = "autolog,logapp,log-extern")]
    pub weidu_log_mode: String,
    #[arg(
        long,
        env = "STRICT_MATCHING",
        default_value_t = false,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub strict_matching: bool,
    #[arg(
        long,
        env = "DOWNLOAD",
        default_value_t = true,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub download: bool,
    #[arg(
        long,
        env = "OVERWRITE",
        default_value_t = false,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub overwrite: bool,
    #[arg(
        long,
        env = "CHECK_LAST_INSTALLED",
        default_value_t = true,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub check_last_installed: bool,
    #[arg(long, env = "TICK", default_value_t = 500)]
    pub tick: u64,
    #[arg(long, env = "LOOKBACK", default_value_t = 10)]
    pub lookback: usize,
    #[arg(
        long,
        env = "CASEFOLD",
        default_value_t = false,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub casefold: bool,
}

#[derive(Args, Debug)]
#[command(disable_help_flag = true)]
pub struct ScanArgs {
    #[arg(long, action = ArgAction::Help)]
    pub help: Option<bool>,

    #[command(subcommand)]
    pub command: ScanCommand,
}

#[derive(Subcommand, Debug)]
pub enum ScanCommand {
    /// Scan and print components
    #[command(name = "components")]
    Components(ScanComponentsArgs),
    /// Scan and print available languages
    #[command(name = "languages")]
    Languages(ScanLanguagesArgs),
}

#[derive(Args, Debug)]
#[command(disable_help_flag = true)]
pub struct ScanComponentsArgs {
    #[arg(long, action = ArgAction::Help)]
    pub help: Option<bool>,
    #[arg(long)]
    pub game_directory: String,
    #[arg(long, default_value = "english")]
    pub filter_by_selected_language: String,
    #[command(flatten)]
    pub options: CommonOptions,
}

#[derive(Args, Debug)]
#[command(disable_help_flag = true)]
pub struct ScanLanguagesArgs {
    #[arg(long, action = ArgAction::Help)]
    pub help: Option<bool>,
    #[arg(long, default_value = "english")]
    pub filter_by_selected_language: String,
    #[command(flatten)]
    pub options: CommonOptions,
}
