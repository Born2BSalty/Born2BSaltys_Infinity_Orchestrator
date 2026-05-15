// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod compat_popup_issue_text_explain {
    use crate::app::compat_issue_text::issue_summary as core_issue_summary;
    use crate::ui::step2::compat_types_step2::CompatIssueDisplay;

    pub(crate) fn issue_summary(issue: &CompatIssueDisplay, selected_mode: &str) -> String {
        core_issue_summary(
            &issue.code,
            &issue.related_mod,
            issue.related_component,
            &issue.reason,
            issue.raw_evidence.as_deref(),
            selected_mode,
        )
    }
}

pub mod compat_popup_issue_text_kind {
    pub(crate) use crate::app::compat_issue_text::human_kind;
}
