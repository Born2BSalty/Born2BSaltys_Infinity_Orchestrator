// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{ButtonIcon, clipboard, render_icon_button};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_text_primary,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::Step2Details;

#[derive(Clone, Copy)]
pub(crate) struct PathsGridLayout {
    pub(crate) palette: ThemePalette,
    pub(crate) label_w: f32,
    pub(crate) value_w: f32,
    pub(crate) action_w: f32,
    pub(crate) row_h: f32,
    pub(crate) value_chars: usize,
}

pub(crate) fn render_paths_grid(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    layout: PathsGridLayout,
) {
    ui.label(crate::ui::shared::typography_global::small_strong(
        "Paths / Links",
    ));
    egui::Grid::new("step2_details_paths_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            let tp2_folder_action = details
                .tp2_folder
                .clone()
                .map(Step2Action::OpenSelectedTp2Folder);
            render_open_only_row(
                ui,
                action,
                &layout,
                "TP2 Folder",
                details.tp2_folder.as_deref(),
                tp2_folder_action.as_ref(),
            );
            render_path_row(
                ui,
                action,
                &layout,
                "TP2 Path",
                details.tp2_path.as_deref(),
                true,
                details
                    .tp2_path
                    .clone()
                    .map(Step2Action::OpenSelectedTp2)
                    .as_ref(),
            );
            render_path_row(
                ui,
                action,
                &layout,
                "INI Path",
                details.ini_path.as_deref(),
                true,
                details
                    .ini_path
                    .clone()
                    .map(Step2Action::OpenSelectedIni)
                    .as_ref(),
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
                    .map(Step2Action::OpenSelectedReadme)
                    .as_ref(),
            );
            if details.web_url.is_some() {
                render_path_row(
                    ui,
                    action,
                    &layout,
                    "Web",
                    details.web_url.as_deref(),
                    false,
                    details
                        .web_url
                        .clone()
                        .map(Step2Action::OpenSelectedWeb)
                        .as_ref(),
                );
            }
        });
    ui.add_space(6.0);
    render_package_grid(ui, details, action, &layout);
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
            render_package_grid_rows(ui, details, action, layout, source_status);
        });
    if can_add_source {
        render_package_source_buttons(ui, action);
    }
    if let Some(locked) = details.package_update_locked {
        render_package_update_buttons(ui, details, action, locked);
    }
}

fn render_package_grid_rows(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    layout: &PathsGridLayout,
    source_status: &str,
) {
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
    render_package_link_rows(ui, details, action, layout);
}

fn render_package_link_rows(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    layout: &PathsGridLayout,
) {
    if let Some(url) = details.package_source_url.as_deref() {
        let open_action = Step2Action::OpenSelectedWeb(url.to_string());
        render_path_row(
            ui,
            action,
            layout,
            "URL",
            Some(url),
            false,
            Some(&open_action),
        );
    }
    if let Some(github) = details.package_source_github.as_deref() {
        let open_action = Step2Action::OpenSelectedWeb(github_repo_url(github));
        render_path_row(
            ui,
            action,
            layout,
            "GitHub",
            Some(github),
            false,
            Some(&open_action),
        );
    }
}

fn render_package_source_buttons(ui: &mut egui::Ui, action: &mut Option<Step2Action>) {
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

fn render_package_update_buttons(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    locked: bool,
) {
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
    ui.add_sized(
        [layout.value_w, layout.row_h],
        egui::Label::new(text).truncate(),
    )
    .on_hover_text(value);
    render_empty_action_cell(ui, layout);
    ui.end_row();
}

fn render_open_only_row(
    ui: &mut egui::Ui,
    action: &mut Option<Step2Action>,
    layout: &PathsGridLayout,
    label: &str,
    value: Option<&str>,
    open_action: Option<&Step2Action>,
) {
    let Some(raw) = value else {
        return;
    };
    let label_resp = ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    let display = ellipsize_end(raw, layout.value_chars);
    let value_resp = ui
        .add_sized(
            [layout.value_w, layout.row_h],
            egui::Label::new(crate::ui::shared::typography_global::monospace(display)).truncate(),
        )
        .on_hover_text(raw);
    let visible = row_action_visible(ui, label_resp.rect, value_resp.rect, layout.action_w);
    render_action_cell(ui, layout, None, open_action, action, visible);
    ui.end_row();
}

fn render_path_row(
    ui: &mut egui::Ui,
    action: &mut Option<Step2Action>,
    layout: &PathsGridLayout,
    label: &str,
    value: Option<&str>,
    missing_amber: bool,
    open_action: Option<&Step2Action>,
) {
    let label_resp = ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    let raw = value.unwrap_or("No data");
    let display = ellipsize_end(raw, layout.value_chars);
    let mut text = crate::ui::shared::typography_global::monospace(display);
    if value.is_none() && missing_amber {
        text = text.color(crate::ui::shared::redesign_tokens::redesign_pill_warn(
            layout.palette,
        ));
    }
    let value_resp = ui
        .add_sized(
            [layout.value_w, layout.row_h],
            egui::Label::new(text).truncate(),
        )
        .on_hover_text(raw);
    if let Some(copy_value) = value {
        let visible = row_action_visible(ui, label_resp.rect, value_resp.rect, layout.action_w);
        render_action_cell(ui, layout, Some(copy_value), open_action, action, visible);
    } else {
        render_empty_action_cell(ui, layout);
    }
    ui.end_row();
}

pub(crate) fn render_component_block(
    ui: &mut egui::Ui,
    details: &Step2Details,
    palette: ThemePalette,
) {
    let Some(block) = details.compat_component_block.as_deref() else {
        return;
    };
    let id = ui.make_persistent_id((
        "step2_component_block",
        details.tp_file.as_deref().unwrap_or_default(),
        details.component_id.as_deref().unwrap_or_default(),
    ));
    render_code_section(ui, palette, id, "Component Block", block, true);
}

pub(crate) fn render_raw_line(ui: &mut egui::Ui, details: &Step2Details, palette: ThemePalette) {
    let Some(raw) = details.raw_line.as_deref() else {
        return;
    };
    let id = ui.make_persistent_id((
        "step2_weidu_line",
        details.tp_file.as_deref().unwrap_or_default(),
        details.component_id.as_deref().unwrap_or_default(),
    ));
    render_code_section(ui, palette, id, "WeiDU Line", raw, false);
}

fn render_code_section(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    id: egui::Id,
    title: &str,
    value: &str,
    default_open: bool,
) {
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
        .show_header(ui, |ui| {
            ui.label(crate::ui::shared::typography_global::small_strong(title));
            let spare = (ui.available_width() - 24.0).max(0.0);
            ui.add_space(spare);
            if render_icon_button(
                ui,
                palette,
                ButtonIcon::Copy,
                crate::ui::shared::tooltip_global::COPY,
                true,
            )
            .clicked()
            {
                clipboard::copy(ui.ctx(), value.to_string());
            }
        })
        .body_unindented(|ui| {
            let frame = egui::Frame::default()
                .fill(redesign_input_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
                .inner_margin(egui::Margin::symmetric(10, 8));
            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(value)
                            .family(egui::FontFamily::Monospace)
                            .color(redesign_text_primary(palette)),
                    )
                    .wrap(),
                );
            });
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

fn render_action_cell(
    ui: &mut egui::Ui,
    layout: &PathsGridLayout,
    copy_value: Option<&str>,
    open_action: Option<&Step2Action>,
    action: &mut Option<Step2Action>,
    visible: bool,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(layout.action_w, layout.row_h),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            if let Some(next_action) = open_action
                && render_icon_button(
                    ui,
                    layout.palette,
                    ButtonIcon::Open,
                    crate::ui::shared::tooltip_global::OPEN,
                    visible,
                )
                .clicked()
            {
                *action = Some(next_action.clone());
            }
            if let Some(value) = copy_value
                && render_icon_button(
                    ui,
                    layout.palette,
                    ButtonIcon::Copy,
                    crate::ui::shared::tooltip_global::COPY,
                    visible,
                )
                .clicked()
            {
                clipboard::copy(ui.ctx(), value.to_string());
            }
        },
    );
}

fn render_empty_action_cell(ui: &mut egui::Ui, layout: &PathsGridLayout) {
    ui.allocate_ui(egui::vec2(layout.action_w, layout.row_h), |_| {});
}

fn row_action_visible(
    ui: &egui::Ui,
    label_rect: egui::Rect,
    value_rect: egui::Rect,
    action_w: f32,
) -> bool {
    let hover_rect = egui::Rect::from_min_max(
        label_rect.min,
        egui::pos2(value_rect.right() + action_w + 8.0, value_rect.bottom()),
    )
    .expand(2.0);
    ui.rect_contains_pointer(hover_rect)
}
