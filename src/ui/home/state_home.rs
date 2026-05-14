// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HomeFilter {
    Installed,
    InProgress,
    #[default]
    All,
}

#[derive(Debug, Clone, Default)]
pub struct HomeScreenState {
    pub filter: HomeFilter,
    pub delete_target: Option<String>,
    pub reinstall_target: Option<String>,
    pub rename_target: Option<String>,
    pub rename_value: String,
}

impl HomeScreenState {
    pub fn ensure_valid_filter(&mut self, installed_count: usize, in_progress_count: usize) {
        if installed_count == 0 && in_progress_count == 0 {
            self.filter = HomeFilter::All;
            return;
        }

        let current_has_items = match self.filter {
            HomeFilter::Installed => installed_count > 0,
            HomeFilter::InProgress => in_progress_count > 0,
            HomeFilter::All => installed_count + in_progress_count > 0,
        };
        if current_has_items {
            return;
        }

        self.filter = if installed_count > 0 {
            HomeFilter::Installed
        } else if in_progress_count > 0 {
            HomeFilter::InProgress
        } else {
            HomeFilter::All
        };
    }
}
