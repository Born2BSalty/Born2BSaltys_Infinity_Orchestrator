// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[cfg(target_os = "windows")]
pub(crate) fn prefers_windows_32bit_assets() -> bool {
    cfg!(target_os = "windows")
        && matches!(std::env::consts::ARCH, "x86" | "i386" | "i586" | "i686")
}

#[cfg(target_os = "windows")]
pub(crate) fn prefers_windows_64bit_assets() -> bool {
    cfg!(target_os = "windows") && matches!(std::env::consts::ARCH, "x86_64" | "amd64")
}

#[cfg(target_os = "macos")]
pub(crate) fn prefers_macos_apple_silicon_assets() -> bool {
    cfg!(target_os = "macos") && matches!(std::env::consts::ARCH, "aarch64" | "arm64")
}

#[cfg(target_os = "macos")]
pub(crate) fn prefers_macos_intel_assets() -> bool {
    cfg!(target_os = "macos") && matches!(std::env::consts::ARCH, "x86_64" | "amd64")
}
