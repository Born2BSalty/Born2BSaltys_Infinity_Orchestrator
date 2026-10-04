// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step2ModState;

#[must_use]
pub fn current_exe_fingerprint() -> String {
    let Ok(path) = std::env::current_exe() else {
        return "unknown".to_string();
    };
    let Ok(meta) = std::fs::metadata(&path) else {
        return format!("path={}", path.display());
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!(
        "path={};len={};mtime={}",
        path.display(),
        meta.len(),
        modified
    )
}

#[cfg(target_os = "windows")]
pub fn open_in_shell(target: &str) -> std::io::Result<()> {
    let target = target.trim();
    if target.is_empty() {
        return Ok(());
    }
    if is_web_url(target) {
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", target])
            .spawn()?;
    } else {
        std::process::Command::new("explorer").arg(target).spawn()?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_web_url(target: &str) -> bool {
    ["http://", "https://"].iter().any(|scheme| {
        target
            .get(..scheme.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
    })
}

#[cfg(target_os = "linux")]
pub fn open_in_shell(target: &str) -> std::io::Result<()> {
    let target = target.trim();
    if target.is_empty() {
        return Ok(());
    }
    let candidates: [(&str, &[&str]); 3] = [
        ("xdg-open", &[target]),
        ("gio", &["open", target]),
        ("gnome-open", &[target]),
    ];
    try_open_candidates(&candidates)
}

#[cfg(target_os = "macos")]
pub fn open_in_shell(target: &str) -> std::io::Result<()> {
    let target = target.trim();
    if target.is_empty() {
        return Ok(());
    }
    std::process::Command::new("open").arg(target).spawn()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn open_in_shell(target: &str) -> std::io::Result<()> {
    let _ = target;
    Ok(())
}

#[cfg(target_os = "linux")]
fn try_open_candidates(candidates: &[(&str, &[&str])]) -> std::io::Result<()> {
    let mut last_err: Option<std::io::Error> = None;
    for (bin, args) in candidates {
        match std::process::Command::new(bin).args(*args).spawn() {
            Ok(_) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                last_err = Some(err);
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("no opener command found")))
}

pub fn sort_mods_alphabetically(mods: &mut [Step2ModState]) {
    mods.sort_by(|a, b| {
        let an = a.name.to_ascii_lowercase();
        let bn = b.name.to_ascii_lowercase();
        an.cmp(&bn).then_with(|| a.tp_file.cmp(&b.tp_file))
    });
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::is_web_url;

    #[test]
    fn web_urls_are_told_apart_from_paths() {
        assert!(is_web_url(
            "https://www.nexusmods.com/baldursgate2ee/mods/110?tab=files"
        ));
        assert!(is_web_url("HTTP://example.com"));
        assert!(!is_web_url("C:\\Games\\BIO\\mods"));
        assert!(!is_web_url("https"));
        assert!(!is_web_url("ftp://example.com/file.zip"));
    }
}
