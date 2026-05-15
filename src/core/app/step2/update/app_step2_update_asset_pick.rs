// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[cfg(any(target_os = "windows", target_os = "macos"))]
use crate::app::platform_asset_target;
use std::path::Path;

pub(super) fn pick_release_asset_for_current_os(
    assets: &[(&str, &str)],
) -> Option<(String, String, String)> {
    let mut best: Option<(i32, String, String, String)> = None;
    for (asset_name, asset_url) in assets {
        if is_archive_asset(asset_name) {
            let score = asset_score(asset_name);
            if best
                .as_ref()
                .is_none_or(|(best_score, ..)| score > *best_score)
            {
                best = Some((
                    score,
                    String::new(),
                    (*asset_name).to_string(),
                    (*asset_url).to_string(),
                ));
            }
        }
    }
    best.map(|(_, tag, asset_name, url)| (tag, asset_name, url))
}

pub(super) fn pick_release_asset_for_pkg_list(
    assets: &[(&str, &str)],
    pkg_list: &str,
) -> Option<(String, String, String)> {
    let mut best: Option<(i32, usize, String, String, String)> = None;
    for (pkg_index, pkg) in pkg_list.split(',').enumerate() {
        let pkg = pkg.trim().to_ascii_lowercase();
        if pkg.is_empty() {
            continue;
        }
        for (asset_name, asset_url) in assets {
            if asset_matches_pkg(asset_name, &pkg) {
                let score = asset_score(asset_name);
                let replace = best
                    .as_ref()
                    .is_none_or(|(best_score, best_pkg_index, ..)| {
                        score > *best_score || (score == *best_score && pkg_index < *best_pkg_index)
                    });
                if replace {
                    best = Some((
                        score,
                        pkg_index,
                        String::new(),
                        (*asset_name).to_string(),
                        (*asset_url).to_string(),
                    ));
                }
            }
        }
    }
    best.map(|(_, _, tag, asset_name, asset_url)| (tag, asset_name, asset_url))
}

pub(super) fn pick_release_asset_by_name(
    assets: &[(&str, &str)],
    requested_name: &str,
) -> Option<(String, String, String)> {
    let requested_name = requested_name.trim();
    if requested_name.is_empty() {
        return None;
    }
    assets
        .iter()
        .find(|(asset_name, _)| asset_name.trim().eq_ignore_ascii_case(requested_name))
        .map(|(asset_name, asset_url)| {
            (
                String::new(),
                (*asset_name).to_string(),
                (*asset_url).to_string(),
            )
        })
}

fn is_archive_asset(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    [
        ".zip", ".7z", ".rar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar.xz", ".txz",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

fn asset_matches_pkg(name: &str, pkg: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    match pkg {
        "lin" => lower.starts_with("lin-"),
        "osx" | "mac" => lower.starts_with("osx-") || lower.starts_with("mac-"),
        "win" => has_extension(name, "exe"),
        "wzp" => lower.starts_with("win-") && has_extension(name, "zip"),
        "w32zip" => {
            has_extension(name, "zip")
                && (lower.contains("win32")
                    || lower.contains("32-bit")
                    || lower.contains("32bit")
                    || (lower.contains("x86") && !lower.contains("x86_64")))
        }
        "w64zip" => {
            has_extension(name, "zip")
                && (lower.contains("win64")
                    || lower.contains("x64")
                    || lower.contains("x86_64")
                    || lower.contains("amd64")
                    || lower.contains("64-bit")
                    || lower.contains("64bit"))
        }
        "macarmzip" => {
            has_extension(name, "zip")
                && contains_any(&lower, &["arm64", "aarch64", "apple-silicon"])
        }
        "macx64zip" => {
            has_extension(name, "zip")
                && (contains_any(&lower, &["x64", "x86_64", "intel"])
                    || ((lower.starts_with("mac-") || lower.starts_with("osx-"))
                        && !contains_any(&lower, &["arm64", "aarch64", "apple-silicon"])))
        }
        "zip" => {
            has_extension(name, "zip")
                && !lower.starts_with("win-")
                && !lower.starts_with("osx-")
                && !lower.starts_with("mac-")
                && !lower.starts_with("lin-")
                && !lower.starts_with("wzp-")
                && !lower.contains("source code")
        }
        "rar" => has_extension(name, "rar"),
        "iemod" => has_extension(name, "iemod"),
        _ => false,
    }
}

fn asset_score(name: &str) -> i32 {
    let lower = name.trim().to_ascii_lowercase();
    let mut score = 0;
    if has_extension(name, "zip") || has_extension(name, "7z") || has_extension(name, "rar") {
        score += 5;
    }
    if lower.contains("source code") {
        score -= 30;
    }
    #[cfg(target_os = "windows")]
    {
        score += platform_score(
            &lower,
            &[
                "windows", "win64", "win32", "-win", "_win", ".win", "win-", "windows-", "_windows",
            ],
            &[
                "linux", "-linux", "_linux", "linux-", "lin-", "_lin", ".lin", "appimage", ".deb",
                ".rpm", "darwin", "macos", "osx", ".dmg",
            ],
        );
        if platform_asset_target::prefers_windows_64bit_assets() {
            score += arch_score(
                &lower,
                &["win64", "x64", "x86_64", "amd64", "64-bit", "64bit"],
                &["win32", "32-bit", "32bit"],
            );
        }
        if platform_asset_target::prefers_windows_32bit_assets() {
            score += arch_score(
                &lower,
                &["win32", "32-bit", "32bit"],
                &["win64", "x64", "x86_64", "amd64", "64-bit", "64bit"],
            );
        }
    }
    #[cfg(target_os = "linux")]
    {
        score += platform_score(
            &lower,
            &[
                "linux", "-linux", "_linux", "linux-", "lin-", "_lin", ".lin", "appimage", ".deb",
                ".rpm",
            ],
            &[
                "windows", "-windows", "_windows", "windows-", "win64", "win32", "win-", "_win",
                ".win", "darwin", "macos", "osx", ".dmg",
            ],
        );
    }
    #[cfg(target_os = "macos")]
    {
        score += platform_score(
            &lower,
            &[
                "darwin", "macos", "osx", ".dmg", "-mac", "_mac", "mac-", "-osx", "_osx",
            ],
            &[
                "windows", "-windows", "_windows", "windows-", "win64", "win32", "win-", "_win",
                ".win", "linux", "-linux", "_linux", "linux-", "lin-", "_lin", ".lin", "appimage",
                ".deb", ".rpm",
            ],
        );
        if platform_asset_target::prefers_macos_apple_silicon_assets() {
            score += arch_score(
                &lower,
                &["arm64", "aarch64", "apple-silicon"],
                &["x64", "x86_64", "intel"],
            );
        }
        if platform_asset_target::prefers_macos_intel_assets() {
            score += arch_score(
                &lower,
                &["x64", "x86_64", "intel"],
                &["arm64", "aarch64", "apple-silicon"],
            );
        }
    }
    score
}

fn has_extension(name: &str, extension: &str) -> bool {
    Path::new(name.trim())
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn platform_score(name: &str, preferred: &[&str], avoided: &[&str]) -> i32 {
    let mut score = 0;
    if preferred.iter().any(|token| name.contains(token)) {
        score += 20;
    }
    if avoided.iter().any(|token| name.contains(token)) {
        score -= 40;
    }
    score
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn arch_score(name: &str, preferred: &[&str], avoided: &[&str]) -> i32 {
    let mut score = 0;
    if contains_any(name, preferred) {
        score += 15;
    }
    if contains_any(name, avoided) {
        score -= 15;
    }
    score
}

fn contains_any(name: &str, tokens: &[&str]) -> bool {
    tokens.iter().any(|token| name.contains(token))
}
