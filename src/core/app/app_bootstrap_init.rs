// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::controller::step3_sync::scrub_dev_settings;
use crate::app::controller::util::current_exe_fingerprint;
use crate::app::state::Step1State;
use crate::settings::store::SettingsStore;

pub(crate) struct AppBootstrap {
    pub(crate) settings_store: SettingsStore,
    pub(crate) exe_fingerprint: String,
    pub(crate) step1: Step1State,
    pub(crate) github_auth_login: String,
    pub(crate) startup_status: Option<String>,
}

pub(crate) fn initialize(dev_mode: bool) -> AppBootstrap {
    let mut startup_warnings = Vec::<String>::new();
    if let Err(err) = crate::app::compat_rules::ensure_compat_rules_files() {
        startup_warnings.push(format!("compat rules init failed: {err}"));
    }
    if let Err(err) = crate::app::mod_downloads::ensure_mod_downloads_files() {
        startup_warnings.push(format!("mod download sources init failed: {err}"));
    }

    let settings_store = SettingsStore::new_default();
    let exe_fingerprint = current_exe_fingerprint();
    let loaded = match settings_store.load() {
        Ok(value) => value,
        Err(err) => {
            startup_warnings.push(format!("settings load failed: {err}"));
            Default::default()
        }
    };
    let mut step1 = Step1State::from(loaded.step1);
    if !dev_mode {
        scrub_dev_settings(&mut step1);
    }
    let github_auth_login =
        match crate::app::app_step1_github_oauth::load_github_login_from_stored_token() {
            Ok(Some(login)) => login,
            Ok(None) => String::new(),
            Err(err) => {
                startup_warnings.push(format!("github auth restore failed: {err}"));
                String::new()
            }
        };
    AppBootstrap {
        settings_store,
        exe_fingerprint,
        step1,
        github_auth_login,
        startup_status: (!startup_warnings.is_empty())
            .then(|| format!("Startup warnings: {}", startup_warnings.join(" | "))),
    }
}
