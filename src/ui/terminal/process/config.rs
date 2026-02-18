// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::state::Step1State;

use super::super::EmbeddedTerminal;

impl EmbeddedTerminal {
    pub fn configure_from_step1(&mut self, step1: &Step1State) {
        self.child_env.clear();
        let rust_log = if step1.rust_log_trace {
            Some("trace")
        } else if step1.rust_log_debug {
            Some("debug")
        } else {
            None
        };
        if let Some(level) = rust_log {
            self.child_env
                .push(("RUST_LOG".to_string(), level.to_string()));
        }
        if step1.bio_full_debug {
            self.child_env
                .push(("BIO_FULL_DEBUG".to_string(), "1".to_string()));
        }
        self.raw_log_path = if step1.log_raw_output_dev {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Some(PathBuf::from("diagnostics").join(format!("raw_output_{ts}.log")))
        } else {
            None
        };
        self.bio_debug_log_path = if step1.bio_full_debug {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Some(PathBuf::from("diagnostics").join(format!("bio_full_debug_{ts}.log")))
        } else {
            None
        };
    }
}
