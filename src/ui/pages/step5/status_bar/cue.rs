// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io::{IsTerminal, Write};
#[cfg(all(unix, not(target_os = "macos")))]
use std::process::Command;

pub(super) fn play_prompt_required_sound_once() {
    if std::io::stdout().is_terminal() {
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(b"\x07");
        let _ = stdout.flush();
        return;
    }

    if play_platform_beep() {
        return;
    }

    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(b"\x07");
    let _ = stdout.flush();
}

#[cfg(target_os = "windows")]
fn play_platform_beep() -> bool {
    unsafe {
        if MessageBeep(0x0000_0040) == 0 {
            let _ = MessageBeep(0xFFFF_FFFF);
        }
    }
    true
}

#[cfg(target_os = "macos")]
fn play_platform_beep() -> bool {
    unsafe {
        NSBeep();
    }
    true
}

#[cfg(all(unix, not(target_os = "macos")))]
fn play_platform_beep() -> bool {
    try_command("canberra-gtk-play", &["-i", "bell"])
        || try_command("paplay", &["/usr/share/sounds/freedesktop/stereo/bell.oga"])
        || try_command("aplay", &["/usr/share/sounds/alsa/Front_Center.wav"])
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn play_platform_beep() -> bool {
    false
}

#[cfg(all(unix, not(target_os = "macos")))]
fn try_command(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    fn NSBeep();
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBeep(u_type: u32) -> i32;
}
