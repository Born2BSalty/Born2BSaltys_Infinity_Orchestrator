// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn format_branch_head_ref(branch: &str, sha: &str) -> String {
    format!("{}@{}", branch.trim(), short_sha(sha))
}

fn short_sha(value: &str) -> &str {
    let trimmed = value.trim();
    let end = trimmed
        .char_indices()
        .nth(12)
        .map_or(trimmed.len(), |(idx, _)| idx);
    &trimmed[..end]
}
