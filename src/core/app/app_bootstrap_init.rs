// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::controller::step3_sync::scrub_dev_settings;
use crate::app::controller::util::current_exe_fingerprint;
use crate::app::state::Step1State;
use crate::settings::model::AppSettings;
use crate::settings::redesign_fields::RedesignSettings;
use crate::settings::store::SettingsStore;
use tracing::{info, warn};

pub(crate) struct AppBootstrap {
    pub(crate) settings_store: SettingsStore,
    pub(crate) exe_fingerprint: String,
    pub(crate) step1: Step1State,
    pub(crate) general: RedesignSettings,
    pub(crate) github_auth_login: String,
}

pub(crate) fn initialize(dev_mode: bool) -> AppBootstrap {
    if let Err(err) = crate::app::compat_rules::ensure_compat_rules_files() {
        warn!(target = "orchestrator", "compat rules init failed: {err}");
    }
    if let Err(err) = crate::app::mod_downloads::ensure_mod_downloads_files() {
        warn!(
            target = "orchestrator",
            "mod download sources init failed: {err}"
        );
    }

    let settings_store = SettingsStore::new_default();
    let exe_fingerprint = current_exe_fingerprint();
    let mut loaded = settings_store.load().unwrap_or_else(|err| {
        warn!(target = "orchestrator", "settings load failed: {err}");
        AppSettings::default()
    });
    if let Some((old_bgee, old_bg2ee)) = loaded.step1.clear_unreachable_eet_sources() {
        info!(
            target = "orchestrator",
            "ignoring unreachable EET source fields still present in settings: {:?}",
            (old_bgee, old_bg2ee)
        );
    }
    let general = loaded.general;
    let mut step1 = Step1State::from(loaded.step1);
    if step1.global_mods_folder.trim().is_empty() && !step1.mods_folder.trim().is_empty() {
        step1.global_mods_folder.clone_from(&step1.mods_folder);
    }
    if !dev_mode {
        scrub_dev_settings(&mut step1);
    }
    let github_auth_login =
        match crate::app::app_step1_github_oauth::load_github_login_from_stored_token() {
            Ok(Some(login)) => login,
            Ok(None) => String::new(),
            Err(err) => {
                warn!(target = "orchestrator", "github auth restore failed: {err}");
                String::new()
            }
        };
    AppBootstrap {
        settings_store,
        exe_fingerprint,
        step1,
        general,
        github_auth_login,
    }
}
