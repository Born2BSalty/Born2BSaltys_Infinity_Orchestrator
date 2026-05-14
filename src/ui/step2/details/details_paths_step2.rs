// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_warning};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::Step2Details;

pub(crate) struct PathsGridLayout {
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
    palette: ThemePalette,
}

pub(crate) fn render_paths_grid(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    layout: PathsGridLayout,
) {
    let PathsGridLayout {
        label_w,
        value_w,
        row_h,
        value_chars,
        palette,
    } = layout;
    let layout = PathsGridLayout {
        label_w,
        value_w,
        row_h,
        value_chars,
        palette,
    };
    ui.label(crate::ui::shared::typography_global::small_strong(
        "Paths / Links",
    ));
    egui::Grid::new("step2_details_paths_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            render_open_only_row(
                ui,
                action,
                &layout,
                "TP2 Folder",
                details.tp2_folder.as_deref(),
                details
                    .tp2_folder
                    .clone()
                    .map(Step2Action::OpenSelectedTp2Folder),
            );
            render_path_row(
                ui,
                action,
                &layout,
                "TP2 Path",
                details.tp2_path.as_deref(),
                true,
                details.tp2_path.clone().map(Step2Action::OpenSelectedTp2),
            );
            render_path_row(
                ui,
                action,
                &layout,
                "INI Path",
                details.ini_path.as_deref(),
                true,
                details.ini_path.clone().map(Step2Action::OpenSelectedIni),
            );
            render_path_row(
                ui,
                action,
                &layout,
                "Readme",
                details.readme_path.as_deref(),
                true,
                details
                    .readme_path
                    .clone()
                    .map(Step2Action::OpenSelectedReadme),
            );
            if details.web_url.is_some() {
                render_path_row(
                    ui,
                    action,
                    &layout,
                    "Web",
                    details.web_url.as_deref(),
                    false,
                    details.web_url.clone().map(Step2Action::OpenSelectedWeb),
                );
            }
        });
    ui.add_space(6.0);
    render_package_grid(ui, details, action, &layout);
}

impl PathsGridLayout {
    pub(crate) fn new(
        label_w: f32,
        value_w: f32,
        row_h: f32,
        value_chars: usize,
        palette: ThemePalette,
    ) -> Self {
        Self {
            label_w,
            value_w,
            row_h,
            value_chars,
            palette,
        }
    }
}

fn render_package_grid(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    layout: &PathsGridLayout,
) {
    let Some(source_status) = details.package_source_status.as_deref() else {
        return;
    };
    ui.label(crate::ui::shared::typography_global::small_strong(
        "Package",
    ));
    let can_add_source = details.package_source_url.is_none();
    egui::Grid::new("step2_details_package_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            render_text_row(
                ui,
                layout,
                "Installed Source",
                details
                    .package_installed_source_name
                    .as_deref()
                    .unwrap_or("Unknown"),
                false,
            );
            if let Some(name) = details.package_source_name.as_deref() {
                let update_source = if source_status == "Selected" {
                    format!("{name} (selected)")
                } else {
                    format!("{name} (default)")
                };
                render_text_row(ui, layout, "Update Source", &update_source, false);
            }
            if let Some(version) = details.package_latest_version.as_deref() {
                render_text_row(ui, layout, "Latest Version", version, false);
            }
            if let Some(url) = details.package_source_url.as_deref() {
                render_path_row(
                    ui,
                    action,
                    layout,
                    "URL",
                    Some(url),
                    false,
                    Some(Step2Action::OpenSelectedWeb(url.to_string())),
                );
            }
            if let Some(github) = details.package_source_github.as_deref() {
                render_path_row(
                    ui,
                    action,
                    layout,
                    "GitHub",
                    Some(github),
                    false,
                    Some(Step2Action::OpenSelectedWeb(github_repo_url(github))),
                );
            }
        });
    if can_add_source {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .button("Add Source")
                .on_hover_text("Open mod_downloads_user.toml")
                .clicked()
            {
                *action = Some(Step2Action::OpenModDownloadsUserSource);
            }
            if ui
                .button("Reload Sources")
                .on_hover_text("Reload mod_downloads_default.toml and mod_downloads_user.toml")
                .clicked()
            {
                *action = Some(Step2Action::ReloadModDownloadSources);
            }
        });
    }
    if let Some(locked) = details.package_update_locked {
        ui.add_space(4.0);
        let label = if locked {
            "Unlock Updates"
        } else {
            "Lock Updates"
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    details.package_can_check_updates,
                    egui::Button::new("Check This Mod"),
                )
                .on_hover_text("Check updates only for the selected mod.")
                .clicked()
            {
                *action = Some(Step2Action::PreviewUpdateSelectedMod);
            }
            if ui
                .button(label)
                .on_hover_text("Skip or allow this mod when checking updates in this session.")
                .clicked()
            {
                *action = Some(Step2Action::SetSelectedModUpdateLocked(!locked));
            }
        });
    }
}

fn render_text_row(
    ui: &mut egui::Ui,
    layout: &PathsGridLayout,
    label: &str,
    value: &str,
    monospace: bool,
) {
    ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    let display = ellipsize_end(value, layout.value_chars);
    let text = if monospace {
        crate::ui::shared::typography_global::monospace(display)
    } else {
        crate::ui::shared::typography_global::plain(display)
    };
    ui.add_sized([layout.value_w, layout.row_h], egui::Label::new(text))
        .on_hover_text(value);
    ui.label("");
    ui.end_row();
}

fn render_open_only_row(
    ui: &mut egui::Ui,
    action: &mut Option<Step2Action>,
    layout: &PathsGridLayout,
    label: &str,
    value: Option<&str>,
    open_action: Option<Step2Action>,
) {
    let Some(raw) = value else {
        return;
    };
    ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    let display = ellipsize_end(raw, layout.value_chars);
    ui.add_sized(
        [layout.value_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::monospace(display)),
    )
    .on_hover_text(raw);
    if let Some(next_action) = open_action
        && ui
            .small_button("O")
            .on_hover_text(crate::ui::shared::tooltip_global::OPEN)
            .clicked()
    {
        *action = Some(next_action);
    }
    ui.end_row();
}

fn render_path_row(
    ui: &mut egui::Ui,
    action: &mut Option<Step2Action>,
    layout: &PathsGridLayout,
    label: &str,
    value: Option<&str>,
    missing_amber: bool,
    open_action: Option<Step2Action>,
) {
    ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    let raw = value.unwrap_or("No data");
    let display = ellipsize_end(raw, layout.value_chars);
    let mut text = crate::ui::shared::typography_global::monospace(display);
    if value.is_none() && missing_amber {
        text = text.color(redesign_warning(layout.palette));
    }
    ui.add_sized([layout.value_w, layout.row_h], egui::Label::new(text))
        .on_hover_text(raw);
    if let Some(copy_value) = value {
        ui.horizontal(|ui| {
            if ui
                .small_button("C")
                .on_hover_text(crate::ui::shared::tooltip_global::COPY)
                .clicked()
            {
                ui.ctx().copy_text(copy_value.to_string());
            }
            if ui
                .small_button("O")
                .on_hover_text(crate::ui::shared::tooltip_global::OPEN)
                .clicked()
                && let Some(a) = open_action.clone()
            {
                *action = Some(a);
            }
        });
    } else {
        ui.label("");
    }
    ui.end_row();
}

pub(crate) fn render_component_block(ui: &mut egui::Ui, details: &Step2Details) {
    let Some(block) = details.compat_component_block.as_deref() else {
        return;
    };
    egui::CollapsingHeader::new("Component Block")
        .id_salt((
            "step2_component_block",
            details.tp_file.as_deref().unwrap_or_default(),
            details.component_id.as_deref().unwrap_or_default(),
        ))
        .default_open(true)
        .show(ui, |ui| {
            if ui
                .small_button("C")
                .on_hover_text(crate::ui::shared::tooltip_global::COPY)
                .clicked()
            {
                ui.ctx().copy_text(block.to_string());
            }
            ui.add(egui::Label::new(crate::ui::shared::typography_global::monospace(block)).wrap());
        });
}

pub(crate) fn render_raw_line(ui: &mut egui::Ui, details: &Step2Details) {
    let Some(raw) = details.raw_line.as_deref() else {
        return;
    };
    egui::CollapsingHeader::new("WeiDU Line")
        .id_salt((
            "step2_weidu_line",
            details.tp_file.as_deref().unwrap_or_default(),
            details.component_id.as_deref().unwrap_or_default(),
        ))
        .default_open(false)
        .show(ui, |ui| {
            if ui
                .small_button("C")
                .on_hover_text(crate::ui::shared::tooltip_global::COPY)
                .clicked()
            {
                ui.ctx().copy_text(raw.to_string());
            }
            ui.add(egui::Label::new(crate::ui::shared::typography_global::monospace(raw)).wrap());
        });
}

fn ellipsize_end(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let prefix: String = value.chars().take(keep).collect();
    format!("{prefix}...")
}

fn github_repo_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://github.com/{}", trimmed.trim_start_matches('/'))
    }
}
