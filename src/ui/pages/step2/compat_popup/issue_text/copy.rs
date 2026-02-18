// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

use super::helpers::format_issue_target;

pub(crate) fn format_issue_for_copy(issue: &CompatIssueDisplay) -> String {
    let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
    let related = format_issue_target(&issue.related_mod, issue.related_component);
    format!(
        "[{}] {} -> {}\\nBlocking: {}\\nReason: {}\\nSource: {}",
        issue.code,
        affected,
        related,
        if issue.is_blocking { "yes" } else { "no" },
        issue.reason,
        issue.source
    )
}
