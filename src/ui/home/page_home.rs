// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use chrono::{DateTime, Utc};
use eframe::egui;

use crate::app::state::WizardState;
use crate::registry::model::{ModlistEntry, ModlistRegistry, ModlistState};
use crate::ui::home::state_home::{HomeFilter, HomeScreenState};
use crate::ui::home::{
    add_a_modlist, confirm_delete, confirm_reinstall, filter_chip, first_launch_setup_card,
    game_installs_detected, modlist_card, rename_modlist,
};
use crate::ui::orchestrator::widgets::screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_BOX_LABEL_FONT_SIZE_PX,
    REDESIGN_BOX_LABEL_GAP_PX, REDESIGN_HOME_CARD_LIST_GAP_PX,
    REDESIGN_HOME_CHIP_ROW_BOTTOM_MARGIN_PX, REDESIGN_HOME_CHIP_ROW_GAP_PX,
    REDESIGN_HOME_GAME_BLOCK_TOP_MARGIN_PX, REDESIGN_HOME_GRID_BOTTOM_MARGIN_PX,
    REDESIGN_HOME_GRID_GAP_PX, REDESIGN_HOME_LEFT_COLUMN_WEIGHT,
    REDESIGN_HOME_PANEL_PADDING_MARGIN, REDESIGN_HOME_RIGHT_COLUMN_WEIGHT,
    REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_PAGE_PADDING_X_PX, REDESIGN_PAGE_PADDING_Y_PX,
    ThemePalette, redesign_border_strong, redesign_shell_bg, redesign_text_faint,
    redesign_text_muted,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeAction {
    OpenInstall,
    OpenCreate,
    OpenSettingsPaths,
    CardIntent {
        modlist_id: String,
        action: modlist_card::ModlistCardAction,
    },
    CancelDelete,
    ConfirmDeleteIntent,
    CancelReinstall,
    ConfirmReinstallIntent,
    CancelRename,
    ConfirmRenameIntent,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut HomeScreenState,
    registry: Option<&ModlistRegistry>,
    wizard_state: &WizardState,
) -> Option<HomeAction> {
    let entries = registry.map_or_else(|| [].as_slice(), |registry| registry.entries.as_slice());
    let installed_count = entries
        .iter()
        .filter(|entry| entry.state == ModlistState::Installed)
        .count();
    let in_progress_count = entries
        .iter()
        .filter(|entry| entry.state == ModlistState::InProgress)
        .count();
    state.ensure_valid_filter(installed_count, in_progress_count);

    let mut action = None;
    let subtitle = home_subtitle(installed_count, in_progress_count, entries);
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_PAGE_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_PAGE_PADDING_Y_PX),
        ))
        .show(ui, |ui| {
            screen_title::render(ui, palette, "Welcome back, adventurer", Some(&subtitle));

            let available_width = ui.available_width();
            let left_width = ((available_width - REDESIGN_HOME_GRID_GAP_PX)
                * REDESIGN_HOME_LEFT_COLUMN_WEIGHT
                / (REDESIGN_HOME_LEFT_COLUMN_WEIGHT + REDESIGN_HOME_RIGHT_COLUMN_WEIGHT))
                .max(f32::MIN_POSITIVE);
            let right_width =
                (available_width - left_width - REDESIGN_HOME_GRID_GAP_PX).max(f32::MIN_POSITIVE);

            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = REDESIGN_HOME_GRID_GAP_PX;
                ui.allocate_ui_with_layout(
                    egui::vec2(left_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        home_box(ui, palette, None, |ui| {
                            if installed_count == 0 && in_progress_count == 0 {
                                action = first_launch_setup_card::render(ui, palette)
                                    .or_else(|| action.take());
                            } else {
                                render_filter_row(
                                    ui,
                                    palette,
                                    state,
                                    installed_count,
                                    in_progress_count,
                                );
                                ui.add_space(REDESIGN_HOME_CHIP_ROW_BOTTOM_MARGIN_PX);
                                render_cards(ui, palette, state.filter, entries, &mut action);
                            }
                        });
                    },
                );

                ui.allocate_ui_with_layout(
                    egui::vec2(right_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        home_box(ui, palette, Some("add a modlist"), |ui| {
                            action = add_a_modlist::render(ui, palette).or_else(|| action.take());
                            ui.add_space(REDESIGN_HOME_GAME_BLOCK_TOP_MARGIN_PX);
                            game_installs_detected::render(
                                ui,
                                palette,
                                &wizard_state.step1,
                                wizard_state.step1_path_check.as_ref(),
                            );
                        });
                    },
                );
            });
            ui.add_space(REDESIGN_HOME_GRID_BOTTOM_MARGIN_PX);
        });

    if let Some(target_id) = state.delete_target.as_deref()
        && let Some(entry) = entries.iter().find(|entry| entry.id == target_id)
    {
        action = confirm_delete::render(ui.ctx(), palette, entry).or(action);
    }
    if let Some(target_id) = state.reinstall_target.as_deref()
        && let Some(entry) = entries.iter().find(|entry| entry.id == target_id)
    {
        action = confirm_reinstall::render(ui.ctx(), palette, entry).or(action);
    }
    if state.rename_target.is_some() {
        action = rename_modlist::render(ui.ctx(), palette, state).or(action);
    }

    action
}

fn home_box<R>(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: Option<&str>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::NONE
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::same(REDESIGN_HOME_PANEL_PADDING_MARGIN))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(label) = label {
                ui.label(
                    egui::RichText::new(label)
                        .size(REDESIGN_BOX_LABEL_FONT_SIZE_PX)
                        .color(redesign_text_muted(palette)),
                );
                ui.add_space(REDESIGN_BOX_LABEL_GAP_PX);
            }
            body(ui)
        })
}

fn render_filter_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut HomeScreenState,
    installed_count: usize,
    in_progress_count: usize,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = REDESIGN_HOME_CHIP_ROW_GAP_PX;
        if filter_chip::render(
            ui,
            palette,
            "Installed",
            installed_count,
            state.filter == HomeFilter::Installed,
        )
        .clicked()
        {
            state.filter = HomeFilter::Installed;
        }
        if in_progress_count > 0
            && filter_chip::render(
                ui,
                palette,
                "In progress",
                in_progress_count,
                state.filter == HomeFilter::InProgress,
            )
            .clicked()
        {
            state.filter = HomeFilter::InProgress;
        }
        if filter_chip::render(
            ui,
            palette,
            "All",
            installed_count + in_progress_count,
            state.filter == HomeFilter::All,
        )
        .clicked()
        {
            state.filter = HomeFilter::All;
        }
    });
}

fn render_cards(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    filter: HomeFilter,
    entries: &[ModlistEntry],
    action: &mut Option<HomeAction>,
) {
    let visible_entries: Vec<&ModlistEntry> = entries
        .iter()
        .filter(|entry| match filter {
            HomeFilter::Installed => entry.state == ModlistState::Installed,
            HomeFilter::InProgress => entry.state == ModlistState::InProgress,
            HomeFilter::All => true,
        })
        .collect();

    if visible_entries.is_empty() {
        ui.label(
            egui::RichText::new(empty_filter_text(filter))
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_faint(palette)),
        );
        return;
    }

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = REDESIGN_HOME_CARD_LIST_GAP_PX;
        for entry in visible_entries {
            let meta_line = meta_line(entry);
            let view = modlist_card::ModlistCardView {
                name: &entry.name,
                meta_line: &meta_line,
                state: match entry.state {
                    ModlistState::InProgress => modlist_card::ModlistCardState::InProgress,
                    ModlistState::Installed => modlist_card::ModlistCardState::Installed,
                },
            };
            if let Some(card_action) = modlist_card::render(ui, palette, &view) {
                *action = Some(HomeAction::CardIntent {
                    modlist_id: entry.id.clone(),
                    action: card_action,
                });
            }
        }
    });
}

fn home_subtitle(
    installed_count: usize,
    in_progress_count: usize,
    entries: &[ModlistEntry],
) -> String {
    let mut parts = vec![format!("{installed_count} modlists installed")];
    if in_progress_count > 0 {
        parts.push(format!("{in_progress_count} in progress"));
    }
    if let Some(last_played) = entries
        .iter()
        .filter_map(|entry| entry.last_played_date)
        .max()
    {
        parts.push(format!("last played {}", relative_time(last_played)));
    }
    parts.join(" · ")
}

fn meta_line(entry: &ModlistEntry) -> String {
    match entry.state {
        ModlistState::InProgress => format!(
            "{} mods · {} components · last touched {} · paused at Step 2",
            entry.mod_count,
            entry.component_count,
            relative_time(entry.last_touched_date)
        ),
        ModlistState::Installed => format!(
            "{} mods · {} · installed {}",
            entry.mod_count,
            total_size_text(entry.total_size_bytes),
            relative_time(entry.install_date.unwrap_or(entry.last_touched_date))
        ),
    }
}

fn total_size_text(total_size_bytes: Option<u64>) -> String {
    total_size_bytes.map_or_else(
        || "—".to_string(),
        |bytes| {
            let tenths = bytes.saturating_mul(10).saturating_add(536_870_912) / 1_073_741_824;
            format!("{}.{:01} GB", tenths / 10, tenths % 10)
        },
    )
}

fn relative_time(timestamp: DateTime<Utc>) -> String {
    let elapsed = Utc::now().signed_duration_since(timestamp);
    if elapsed.num_days() >= 30 {
        "last month".to_string()
    } else if elapsed.num_days() >= 7 {
        "last week".to_string()
    } else if elapsed.num_days() >= 1 {
        format!("{} days ago", elapsed.num_days())
    } else if elapsed.num_hours() >= 1 {
        format!("{} hours ago", elapsed.num_hours())
    } else {
        "just now".to_string()
    }
}

const fn empty_filter_text(filter: HomeFilter) -> &'static str {
    match filter {
        HomeFilter::Installed => {
            "No installed modlists yet. Create one or paste an import code to add the first."
        }
        HomeFilter::InProgress => {
            "No in-progress builds. Start a new modlist from \"create your own\"."
        }
        HomeFilter::All => "No modlists yet.",
    }
}
