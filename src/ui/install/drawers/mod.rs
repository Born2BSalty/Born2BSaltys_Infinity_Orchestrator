// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod included_mods;
pub mod install_drawer;
pub mod text_drawer;

use eframe::egui;

use crate::app::mod_downloads;
use crate::registry::model::ModlistRegistry;
use crate::ui::install::stage_review;
use crate::ui::install::state_install::{DrawerKind, InstallScreenState};
use crate::ui::shared::redesign_tokens::ThemePalette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawerOutcome {
    Stay,
    BeginInstall,
    BeginImport,
}

pub(crate) const fn after_included_mods(
    outcome: &included_mods::IncludedModsOutcome,
    drawer: &mut crate::ui::install::state_install::DrawerState,
) {
    match outcome {
        included_mods::IncludedModsOutcome::Stay => {}
        included_mods::IncludedModsOutcome::Close => drawer.open = None,
        included_mods::IncludedModsOutcome::Install => drawer.open = Some(DrawerKind::Install),
    }
}

pub(crate) fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    state: &mut InstallScreenState,
    registry: &ModlistRegistry,
    pending_reinstall_id: Option<&str>,
    offer_install: bool,
) -> DrawerOutcome {
    let Some(kind) = state.drawer.open else {
        return DrawerOutcome::Stay;
    };

    let Some(preview) = state.parsed_preview.clone() else {
        state.drawer.open = None;
        return DrawerOutcome::Stay;
    };

    match kind {
        DrawerKind::IncludedMods => {
            let _ = state.inside_model(mod_downloads::load_source_tiers);
            let name = stage_review::display_name(&state.review.name, &preview);
            let InstallScreenState { drawer, inside, .. } = state;
            let Some(inside) = inside.as_ref() else {
                drawer.open = None;
                return DrawerOutcome::Stay;
            };
            let outcome = included_mods::render(ctx, palette, drawer, &name, inside, offer_install);
            after_included_mods(&outcome, drawer);
            DrawerOutcome::Stay
        }
        DrawerKind::WeiduLogs => close_text_drawer(
            ctx,
            palette,
            text_drawer::TextDrawer::WeiduLogs,
            state,
            &preview,
        ),
        DrawerKind::InstalledRefs => close_text_drawer(
            ctx,
            palette,
            text_drawer::TextDrawer::InstalledRefs,
            state,
            &preview,
        ),
        DrawerKind::DownloadSources => close_text_drawer(
            ctx,
            palette,
            text_drawer::TextDrawer::DownloadSources,
            state,
            &preview,
        ),
        DrawerKind::ConfigFiles => close_text_drawer(
            ctx,
            palette,
            text_drawer::TextDrawer::ConfigFiles,
            state,
            &preview,
        ),
        DrawerKind::Install => {
            match install_drawer::render(
                ctx,
                palette,
                state,
                &preview,
                registry,
                pending_reinstall_id,
            ) {
                install_drawer::InstallDrawerOutcome::Stay => DrawerOutcome::Stay,
                install_drawer::InstallDrawerOutcome::Close => {
                    state.drawer.open = None;
                    DrawerOutcome::Stay
                }
                install_drawer::InstallDrawerOutcome::BeginInstall => {
                    state.drawer.open = None;
                    DrawerOutcome::BeginInstall
                }
                install_drawer::InstallDrawerOutcome::BeginImport => {
                    state.drawer.open = None;
                    DrawerOutcome::BeginImport
                }
            }
        }
    }
}

fn close_text_drawer(
    ctx: &egui::Context,
    palette: ThemePalette,
    kind: text_drawer::TextDrawer,
    state: &mut InstallScreenState,
    preview: &crate::app::modlist_share::ModlistSharePreview,
) -> DrawerOutcome {
    if text_drawer::render(ctx, palette, kind, state, preview) {
        state.drawer.open = None;
    }
    DrawerOutcome::Stay
}
