// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use anyhow::Result;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn parse_level(s: &str) -> Level {
    match s.to_ascii_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

pub fn init(log_level: &str) -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(parse_level(log_level))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
