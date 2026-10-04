// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::compat_dlc_source::unresolved_suffix;
use crate::app::modlist_biolist::{
    BIOLIST_EXTENSION, BIOLIST_FILTER_LABEL, suggested_file_name, write_biolist,
};
use crate::app::modlist_share::preview_modlist_share_code;
use crate::ui::orchestrator::widgets::clipboard;
use crate::ui::orchestrator::widgets::notification::NotificationManager;

#[must_use]
pub fn unresolved_mods_for_code(code: &str) -> Vec<String> {
    preview_modlist_share_code(code)
        .map(|preview| preview.unresolved_mods)
        .unwrap_or_default()
}

pub fn export_modlist_file(
    modlist_name: &str,
    code: &str,
    unresolved: &[String],
    notifications: &mut NotificationManager,
) {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(suggested_file_name(modlist_name))
        .add_filter(BIOLIST_FILTER_LABEL, &[BIOLIST_EXTENSION])
        .save_file()
    else {
        return;
    };

    let path = ensure_biolist_extension(path);

    match write_biolist(code, &path) {
        Ok(()) => {
            let suffix = unresolved_suffix(unresolved);
            let message = if suffix.is_empty() {
                format!("Saved {}", path.display())
            } else {
                format!("Saved {}.{suffix}", path.display())
            };
            notifications.success(message);
        }
        Err(err) => notifications.error(format!("Couldn't save the modlist file: {err}")),
    }
}

pub fn copy_share_code(ctx: &egui::Context, modlist_name: &str, code: &str, unresolved: &[String]) {
    let suffix = unresolved_suffix(unresolved);
    let message = if suffix.is_empty() {
        format!("Copied share code for \"{modlist_name}\"")
    } else {
        format!("Copied share code for \"{modlist_name}\".{suffix}")
    };
    clipboard::copy_with_message(ctx, code, message);
}

fn ensure_biolist_extension(path: std::path::PathBuf) -> std::path::PathBuf {
    let has_extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case(BIOLIST_EXTENSION));
    if has_extension {
        path
    } else {
        path.with_extension(BIOLIST_EXTENSION)
    }
}
