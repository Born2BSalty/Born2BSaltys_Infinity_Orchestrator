// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use crate::platform_defaults::app_config_dir;

fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;
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

#[must_use]
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
        let p = default_destination("definitely-not-created-by-this-test-xyz");
        assert!(
            !std::path::Path::new(&p).exists(),
            "default_destination must not create the directory"
        );
    }
}
