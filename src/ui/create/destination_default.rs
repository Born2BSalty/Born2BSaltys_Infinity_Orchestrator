// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `destination_default` — computes a sensible default destination folder for
// a new modlist when the user has not explicitly browsed to one.
//
// Per the phase-06 file inventory:
//   "Computes a default destination folder for new modlists:
//    `<config_dir>/modlists/installs/<slug-of-name>` (or honors a
//    user-configured base path from Settings → Paths)."
//
// **Settings → Paths has no configurable install-base path.** Per SPEC §11.2
// / §13.12a (and the orchestrator skill's standing note), Settings → Paths
// supplies game-source / Mods-*archive* / backup / tool paths **only** —
// there is *no* global install-base or "mods" folder to honor. So the
// "or honors a user-configured base path" clause has no Settings field to
// read in v1; the computed default is unconditionally
// `<config_dir>/modlists/installs/<slug-of-name>`. (`<config_dir>` is the
// same platform config dir the registry + per-modlist workspace files live
// in — `bio::platform_defaults::app_config_dir`.) This is recorded so a
// future reviewer does not treat the missing Settings read as a gap: the
// field genuinely does not exist (SPEC §11.2), it is not an omission.
//
// **Pure, side-effect-free.** This function computes a path string; it does
// **not** create any directory (directory creation at install time is the
// Phase-7 `prepare_target_dirs` / §13.12a P7.T17 concern). The Create screen
// follows the canonical wireframe (`screens.jsx::CreateScreen`): the
// `FolderInput` starts **empty** and the user `browse`s to a real folder
// (the affordance-forward empty-state pattern). This helper is the computed
// fallback the create path uses when the destination is left blank, and is
// available should a caller want to pre-fill — it never silently overrides a
// user-entered path.
//
// SPEC: §5.1 (Create destination), §11.2 (Settings → Paths has no
//       install-base field), §13.12a (per-install dirs are Phase-7).

// rationale: `#[must_use]` on a trivial pure path helper is churn (Cat 3).
#![allow(clippy::must_use_candidate)]

use std::path::PathBuf;

use crate::platform_defaults::app_config_dir;

/// Slugify a modlist name into a filesystem-safe directory component.
///
/// Lowercases, replaces every run of non-`[a-z0-9]` characters with a single
/// `-`, and trims leading/trailing `-`. An empty / all-symbol name slugs to
/// `"modlist"` so the path is always a valid directory component.
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true; // suppress a leading dash
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "modlist".to_string()
    } else {
        out
    }
}

/// Compute the default destination folder for a new modlist named `name`:
/// `<config_dir>/modlists/installs/<slug-of-name>`.
///
/// Falls back to a relative `modlists/installs/<slug>` (under the process
/// CWD) only if the platform config dir is unavailable — the same
/// `unwrap_or_else(PathBuf::from("."))` fallback `WorkspaceStore::new_for_id`
/// uses. Returns a `String` (the `FolderInput` value is a `String`).
///
/// Pure: creates nothing on disk.
pub fn default_destination(name: &str) -> String {
    let base = app_config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("modlists")
        .join("installs")
        .join(slugify(name))
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Tactical EET 2026"), "tactical-eet-2026");
    }

    #[test]
    fn slugify_collapses_and_trims_separators() {
        assert_eq!(slugify("  --Foo!!  Bar__baz--  "), "foo-bar-baz");
        assert_eq!(slugify("a@@@b"), "a-b");
    }

    #[test]
    fn slugify_empty_or_all_symbols_falls_back() {
        assert_eq!(slugify(""), "modlist");
        assert_eq!(slugify("   "), "modlist");
        assert_eq!(slugify("!!!"), "modlist");
    }

    #[test]
    fn slugify_is_lowercase_alnum_dash_only() {
        let s = slugify("MixedCase 123 ~weird~ Name");
        for c in s.chars() {
            assert!(
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-',
                "slug `{s}` contains illegal char `{c}`"
            );
        }
        assert!(!s.starts_with('-') && !s.ends_with('-'));
    }

    #[test]
    fn default_destination_ends_with_modlists_installs_slug() {
        let p = default_destination("My Build");
        // Path separators differ per-OS; check the tail components.
        let pb = PathBuf::from(&p);
        let comps: Vec<String> = pb
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let n = comps.len();
        assert!(n >= 3, "path `{p}` has too few components");
        assert_eq!(comps[n - 3], "modlists");
        assert_eq!(comps[n - 2], "installs");
        assert_eq!(comps[n - 1], "my-build");
    }

    #[test]
    fn default_destination_creates_nothing_on_disk() {
        // Pure: the computed path must NOT exist just because we computed it.
        let p = default_destination("definitely-not-created-by-this-test-xyz");
        assert!(
            !std::path::Path::new(&p).exists(),
            "default_destination must not create the directory"
        );
    }
}
