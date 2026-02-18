// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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

pub fn open_in_shell(target: &str) -> std::io::Result<()> {
    let target = target.trim();
    if target.is_empty() {
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        if target.starts_with("http://") || target.starts_with("https://") {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", target])
                .spawn()?;
        } else {
            std::process::Command::new("explorer").arg(target).spawn()?;
        }
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        // Prefer xdg-open, then common desktop fallbacks.
        let candidates: [(&str, &[&str]); 3] = [
            ("xdg-open", &[target]),
            ("gio", &["open", target]),
            ("gnome-open", &[target]),
        ];
        return try_open_candidates(&candidates);
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(target).spawn()?;
        return Ok(());
    }
    #[allow(unreachable_code)]
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
                continue;
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("no opener command found")))
}

pub fn sort_mods_alphabetically(mods: &mut [crate::ui::state::Step2ModState]) {
    mods.sort_by(|a, b| {
        let an = a.name.to_ascii_lowercase();
        let bn = b.name.to_ascii_lowercase();
        an.cmp(&bn).then_with(|| a.tp_file.cmp(&b.tp_file))
    });
}
