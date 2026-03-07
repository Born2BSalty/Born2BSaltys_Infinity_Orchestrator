// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::settings::store::SettingsStore;
use crate::ui::controller::step3_sync::scrub_dev_settings;
use crate::ui::controller::util::current_exe_fingerprint;
use crate::ui::state::Step1State;

pub(super) struct AppBootstrap {
    pub settings_store: SettingsStore,
    pub exe_fingerprint: String,
    pub step1: Step1State,
}

pub(super) fn initialize(dev_mode: bool) -> AppBootstrap {
    let settings_store = SettingsStore::new_default();
    let exe_fingerprint = current_exe_fingerprint();
    let loaded = settings_store.load().unwrap_or_default();
    let mut step1 = Step1State::from(loaded.step1);
    if !dev_mode {
        scrub_dev_settings(&mut step1);
    }
    AppBootstrap {
        settings_store,
        exe_fingerprint,
        step1,
    }
}
