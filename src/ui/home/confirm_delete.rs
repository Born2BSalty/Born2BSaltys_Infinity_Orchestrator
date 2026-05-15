// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `confirm_delete` — the Home Delete confirm (P5.T7) and the sibling
// Reinstall confirm (P5.T18). Both delegate to the shared, non-blocking
// `ConfirmDialog` (`widgets/dialogs/confirm_dialog.rs`); this module owns
// only the title + body strings.
//
// ── Delete body — VERBATIM from the wireframe ──
// `wireframe-preview/screens.jsx:388-399` renders (as plain text):
//   This will permanently remove:
//     • the modlist's registry entry (it disappears from Home)
//     • the install folder on disk: <destination path>
//   This action cannot be undone.
// SPEC §3.1's Delete-semantics paragraph paraphrases this; the wireframe is
// canonical for copy (HANDOFF source-of-truth ordering), so the strings here
// track screens.jsx, not the SPEC prose.
//
// ── Reinstall body — SPEC §3.1 Reinstall semantics / wireframe :409-419 ──
//   This will erase the current install folder and re-run the entire install
//   from scratch. Your component selection and order are preserved; the
//   modlist moves back to in-progress while the install runs, then returns to
//   installed when complete.
//     • existing files at: <destination path> will be deleted
//   This action cannot be undone.
//
// The destination path shown in both bodies is the entry's real
// `destination_folder`. The wireframe fakes it via `sampleDestFor` (a slug)
// because it has no registry; we have the real value.
//
// SPEC: §3.1 (Delete semantics, Reinstall semantics), §10.1, §10.7.

use crate::registry::model::ModlistEntry;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::ConfirmDialog;

/// Build the Delete confirm's title + body for `entry`. Title:
/// `Delete "<name>"?`. Body: wireframe-verbatim (screens.jsx:388-399), with
/// the entry's real destination folder substituted for the wireframe's
/// faked slug path.
pub fn delete_dialog_text(entry: &ModlistEntry) -> (String, String) {
    let title = format!("Delete \"{}\"?", entry.name);
    let dest = destination_display(entry);
    // `\u{2022}` = • bullet. Rendered in Poppins (Latin-Extended range —
    // safe, unlike symbol glyphs that need firacode_nerd per the HANDOFF
    // caveat; • is U+2022 General Punctuation, present in the Poppins subset
    // the same way the `·` U+00B7 already used across Home cards is).
    let body = format!(
        "This will permanently remove:\n\
         \u{2022} the modlist's registry entry (it disappears from Home)\n\
         \u{2022} the install folder on disk: {dest}\n\
         \nThis action cannot be undone."
    );
    (title, body)
}

/// Build the Reinstall confirm's title + body for `entry`. Title:
/// `Reinstall "<name>"?`. Body per SPEC §3.1 Reinstall semantics /
/// wireframe screens.jsx:409-419.
pub fn reinstall_dialog_text(entry: &ModlistEntry) -> (String, String) {
    let title = format!("Reinstall \"{}\"?", entry.name);
    let dest = destination_display(entry);
    let body = format!(
        "This will erase the current install folder and re-run the entire \
         install from scratch. Your component selection and order are \
         preserved; the modlist moves back to in-progress while the install \
         runs, then returns to installed when complete.\n\
         \u{2022} existing files at: {dest} will be deleted\n\
         \nThis action cannot be undone."
    );
    (title, body)
}

/// The destination string to show in a confirm body. When the entry has no
/// destination yet (in-progress builds / dev-seed entries carry `""`), show
/// an explicit placeholder rather than an empty gap — keeps the bullet line
/// honest and matches the affordance-forward empty-state convention.
fn destination_display(entry: &ModlistEntry) -> String {
    let d = entry.destination_folder.trim();
    if d.is_empty() {
        "(no install folder set)".to_string()
    } else {
        d.to_string()
    }
}

/// Convenience: build a `ConfirmDialog` descriptor for the Delete confirm.
/// `title`/`body` must outlive the returned borrow (caller owns the
/// `(String, String)` from [`delete_dialog_text`]).
pub fn delete_confirm<'a>(id_salt: &'a str, title: &'a str, body: &'a str) -> ConfirmDialog<'a> {
    ConfirmDialog {
        id_salt,
        title,
        body,
        confirm_label: "Delete",
        danger: true,
    }
}

/// Convenience: build a `ConfirmDialog` descriptor for the Reinstall confirm.
pub fn reinstall_confirm<'a>(
    id_salt: &'a str,
    title: &'a str,
    body: &'a str,
) -> ConfirmDialog<'a> {
    ConfirmDialog {
        id_salt,
        title,
        body,
        confirm_label: "Reinstall",
        danger: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry};

    fn e(name: &str, dest: &str) -> ModlistEntry {
        ModlistEntry {
            id: "ABCDEFGHIJKL".to_string(),
            name: name.to_string(),
            game: Game::EET,
            destination_folder: dest.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn delete_title_quotes_name() {
        let (t, _) = delete_dialog_text(&e("Tactical EET 2026", ""));
        assert_eq!(t, "Delete \"Tactical EET 2026\"?");
    }

    #[test]
    fn delete_body_is_wireframe_verbatim_shape() {
        let (_, b) = delete_dialog_text(&e("X", "C:\\BIO\\modlists\\x"));
        assert!(b.starts_with("This will permanently remove:"));
        assert!(b.contains("\u{2022} the modlist's registry entry (it disappears from Home)"));
        assert!(b.contains("\u{2022} the install folder on disk: C:\\BIO\\modlists\\x"));
        assert!(b.trim_end().ends_with("This action cannot be undone."));
    }

    #[test]
    fn delete_body_shows_placeholder_when_dest_empty() {
        let (_, b) = delete_dialog_text(&e("X", ""));
        assert!(b.contains("the install folder on disk: (no install folder set)"));
    }

    #[test]
    fn reinstall_title_and_body() {
        let (t, b) = reinstall_dialog_text(&e("EET Mega", "/games/eet"));
        assert_eq!(t, "Reinstall \"EET Mega\"?");
        assert!(b.starts_with("This will erase the current install folder"));
        assert!(b.contains("moves back to in-progress"));
        assert!(b.contains("\u{2022} existing files at: /games/eet will be deleted"));
        assert!(b.trim_end().ends_with("This action cannot be undone."));
    }

    #[test]
    fn confirm_descriptors_are_danger() {
        let (t, b) = delete_dialog_text(&e("X", ""));
        let d = delete_confirm("salt", &t, &b);
        assert!(d.danger);
        assert_eq!(d.confirm_label, "Delete");
        let (t2, b2) = reinstall_dialog_text(&e("X", ""));
        let r = reinstall_confirm("salt", &t2, &b2);
        assert!(r.danger);
        assert_eq!(r.confirm_label, "Reinstall");
    }
}
