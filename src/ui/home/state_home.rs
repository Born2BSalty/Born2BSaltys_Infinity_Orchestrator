// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HomeFilter {
    #[default]
    Installed,
    InProgress,
    All,
}

#[must_use]
pub const fn resolve_default_filter(
    installed_count: usize,
    in_progress_count: usize,
) -> HomeFilter {
    if installed_count > 0 {
        HomeFilter::Installed
    } else if in_progress_count > 0 {
        HomeFilter::InProgress
    } else {
        HomeFilter::All
    }
}

#[must_use]
pub const fn empty_filter_message(filter: HomeFilter) -> &'static str {
    match filter {
        HomeFilter::Installed => {
            "No installed modlists yet. Create one or paste an import code to add the first."
        }
        HomeFilter::InProgress => {
            "No in-progress builds. Start a new modlist from \"create your own\"."
        }
        HomeFilter::All => "No modlists yet.",
    }
}

#[derive(Debug, Clone, Default)]
pub struct HomeScreenState {
    pub filter: Option<HomeFilter>,

    pub delete_target: Option<String>,
    pub reinstall_target: Option<String>,

    /// Id of the card currently in inline-rename mode, if any.
    pub rename_target: Option<String>,
    /// Live buffer for the rename text field while `rename_target` is `Some`.
    pub rename_temp: String,
}

impl HomeScreenState {
    #[must_use]
    pub fn effective_filter(&self, installed_count: usize, in_progress_count: usize) -> HomeFilter {
        self.filter
            .unwrap_or_else(|| resolve_default_filter(installed_count, in_progress_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_filter_prefers_installed() {
        assert_eq!(resolve_default_filter(3, 2), HomeFilter::Installed);
        assert_eq!(resolve_default_filter(1, 0), HomeFilter::Installed);
    }

    #[test]
    fn default_filter_falls_back_to_in_progress() {
        assert_eq!(resolve_default_filter(0, 2), HomeFilter::InProgress);
    }

    #[test]
    fn default_filter_falls_back_to_all_when_empty() {
        assert_eq!(resolve_default_filter(0, 0), HomeFilter::All);
    }

    #[test]
    fn effective_filter_uses_explicit_choice_over_default() {
        let mut st = HomeScreenState::default();
        assert_eq!(st.effective_filter(2, 1), HomeFilter::Installed);
        st.filter = Some(HomeFilter::All);
        assert_eq!(st.effective_filter(2, 1), HomeFilter::All);
    }

    #[test]
    fn empty_messages_match_spec() {
        assert_eq!(
            empty_filter_message(HomeFilter::Installed),
            "No installed modlists yet. Create one or paste an import code to add the first."
        );
        assert_eq!(
            empty_filter_message(HomeFilter::InProgress),
            "No in-progress builds. Start a new modlist from \"create your own\"."
        );
        assert_eq!(empty_filter_message(HomeFilter::All), "No modlists yet.");
    }
}
