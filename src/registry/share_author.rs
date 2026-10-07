// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::registry::model::ModlistEntry;
use crate::registry::share_export::{ShareMeta, set_packed_identity};
use crate::registry::workspace_model::ModlistWorkspaceState;

pub const USER_NAME_RULE_HINT: &str = "At least 2 characters, including a letter.";

#[must_use]
pub fn user_name_is_valid(name: &str) -> bool {
    let trimmed = name.trim();
    (2..=80).contains(&trimmed.chars().count()) && trimmed.chars().any(char::is_alphabetic)
}

#[must_use]
pub fn workspace_marks_own_list(workspace: &ModlistWorkspaceState) -> bool {
    workspace
        .scratch_mods_folder
        .as_deref()
        .is_some_and(|folder| !folder.trim().is_empty())
}

pub fn stamp_current_author(entry: &mut ModlistEntry, user_name: &str) -> bool {
    let name = user_name.trim();
    if name.is_empty() {
        return false;
    }
    let author_changed = entry.author.as_deref() != Some(name);
    if author_changed {
        entry.author = Some(name.to_string());
    }
    let code_changed = if let Some(code) = entry.latest_share_code.clone()
        && let Ok(stamped) = set_packed_identity(&code, &ShareMeta::from_entry(entry, false))
        && stamped != code
    {
        entry.set_latest_share_code(stamped);
        true
    } else {
        false
    };
    author_changed || code_changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modlist_share::{decode_share_payload, encode_share_payload_text};

    fn code_by(author: &str) -> String {
        encode_share_payload_text(&format!(
            r#"{{
                "format_version": 1,
                "game_install": "BGEE",
                "install_mode": "start_from_scratch",
                "weidu_logs": {{ "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" }},
                "allow_auto_install": false,
                "name": "My BGEE run",
                "author": "{author}"
            }}"#
        ))
        .expect("mint code")
    }

    fn entry_with(author: Option<&str>, code: Option<String>) -> ModlistEntry {
        ModlistEntry {
            id: "OWN000000001".to_string(),
            name: "My BGEE run".to_string(),
            author: author.map(str::to_string),
            latest_share_code: code,
            ..Default::default()
        }
    }

    #[test]
    fn user_name_rule_accepts_two_chars_with_a_letter() {
        let eighty = "a".repeat(80);
        for name in ["A7", "Jo", "Ян", "A@", " Jo ", eighty.as_str()] {
            assert!(user_name_is_valid(name), "{name:?} must pass");
        }
    }

    #[test]
    fn user_name_rule_rejects_short_letterless_or_long() {
        let eighty_one = "a".repeat(81);
        for name in ["", "@", "77", "-", " a ", "@@", eighty_one.as_str()] {
            assert!(!user_name_is_valid(name), "{name:?} must fail");
        }
    }

    #[test]
    fn own_list_needs_a_non_blank_mods_folder() {
        let mut workspace = ModlistWorkspaceState {
            scratch_mods_folder: Some("D:\\run\\mods".to_string()),
            ..Default::default()
        };
        assert!(workspace_marks_own_list(&workspace));
        workspace.scratch_mods_folder = None;
        assert!(!workspace_marks_own_list(&workspace));
        workspace.scratch_mods_folder = Some("  ".to_string());
        assert!(!workspace_marks_own_list(&workspace));
    }

    #[test]
    fn stamp_writes_the_current_name_and_restamps_the_code() {
        let before = code_by("@old");
        let mut entry = entry_with(Some("@old"), Some(before.clone()));

        assert!(stamp_current_author(&mut entry, "  @new  "));

        assert_eq!(entry.author.as_deref(), Some("@new"));
        let after = entry.latest_share_code.clone().expect("code kept");
        let before_payload = decode_share_payload(&before).expect("decode before");
        let after_payload = decode_share_payload(&after).expect("decode after");
        assert_eq!(after_payload.author.as_deref(), Some("@new"));
        assert_eq!(
            after_payload.weidu_logs.bgee,
            before_payload.weidu_logs.bgee
        );
        assert!(!after_payload.allow_auto_install);
        assert_eq!(
            after_payload.allow_auto_install,
            before_payload.allow_auto_install
        );
    }

    #[test]
    fn stamp_with_a_blank_name_changes_nothing() {
        let code = code_by("@old");
        let mut entry = entry_with(Some("@old"), Some(code.clone()));

        assert!(!stamp_current_author(&mut entry, "   "));

        assert_eq!(entry.author.as_deref(), Some("@old"));
        assert_eq!(entry.latest_share_code.as_deref(), Some(code.as_str()));
    }

    #[test]
    fn stamp_with_the_same_name_and_code_reports_no_change() {
        let mut entry = entry_with(Some("@me"), Some(code_by("@me")));
        stamp_current_author(&mut entry, "@me");
        let settled = entry.latest_share_code.clone();

        assert!(!stamp_current_author(&mut entry, "@me"));

        assert_eq!(entry.author.as_deref(), Some("@me"));
        assert_eq!(entry.latest_share_code, settled);
    }

    #[test]
    fn stamp_keeps_a_non_bio_code() {
        let mut entry = entry_with(None, Some("not a share code".to_string()));

        assert!(stamp_current_author(&mut entry, "@me"));

        assert_eq!(entry.author.as_deref(), Some("@me"));
        assert_eq!(entry.latest_share_code.as_deref(), Some("not a share code"));
    }
}
