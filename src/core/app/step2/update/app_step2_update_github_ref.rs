// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn format_branch_head_ref(branch: &str, sha: &str) -> String {
    format!("{}@{}", branch.trim(), sha.trim())
}

#[cfg(test)]
mod tests {
    use super::format_branch_head_ref;

    #[test]
    fn branch_head_ref_keeps_the_full_sha() {
        let ref_value =
            format_branch_head_ref("master", "7649ced6cd25865874d787ec1a9abbc67b068729");
        assert_eq!(ref_value, "master@7649ced6cd25865874d787ec1a9abbc67b068729");
    }
}
