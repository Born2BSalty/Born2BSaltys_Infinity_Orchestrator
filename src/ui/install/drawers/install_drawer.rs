// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::registry::model::ModlistRegistry;
use crate::ui::install::stage_review::{self, RightColumnCtx};
use crate::ui::install::state_install::InstallScreenState;
use crate::ui::install::sub_flow_footer::{self, GlyphSide};
use crate::ui::install::whats_inside::{self, InsideCounts};
use crate::ui::orchestrator::widgets::drawer::{self, DrawerSpec, DrawerWidth};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_box, redesign_btn};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_muted};

const SUBTITLE: &str = "name it, pick a destination, then choose how to install";
const CANCEL_LABEL: &str = "Cancel";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallDrawerOutcome {
    Stay,
    Close,
    BeginInstall,
    BeginImport,
}

#[must_use]
pub(crate) fn scope_line(preview: &ModlistSharePreview, counts: &InsideCounts) -> String {
    let targets = if preview.game_install.eq_ignore_ascii_case("EET") {
        "BGEE + BG2EE (EET)".to_string()
    } else {
        preview.game_install.clone()
    };
    let base = format!(
        "Installs {} \u{00B7} {} into {targets}.",
        whats_inside::plural(counts.mods, "mod"),
        whats_inside::plural(counts.components, "component")
    );
    if let Some(last) = preview.forked_from.last() {
        format!("{base} Forked from {}.", last.name)
    } else if let Some(author) = preview
        .author
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        format!("{base} Credited to {author}.")
    } else {
        base
    }
}

pub(crate) fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    state: &mut InstallScreenState,
    preview: &ModlistSharePreview,
    registry: &ModlistRegistry,
    pending_reinstall_id: Option<&str>,
) -> InstallDrawerOutcome {
    stage_review::force_modify_for_availability(state, preview);
    let checks = stage_review::destination_checks(&state.destination, registry);
    let title = format!(
        "Install {}",
        stage_review::display_name(&state.review.name, preview)
    );
    let counts = InsideCounts::from_preview(preview);
    let scope = scope_line(preview, &counts);

    let spec = DrawerSpec {
        id_salt: "install_drawer",
        title: &title,
        subtitle: SUBTITLE,
        width: DrawerWidth::Form,
    };

    let disabled = stage_review::begin_disabled_for(state, &checks);
    let begin_label = stage_review::begin_label(state.review.modify);

    let mut cancel_clicked = false;
    let mut begin_clicked = false;
    let response = drawer::render(
        ctx,
        palette,
        &spec,
        |ui| {
            stage_review::render_install_settings(
                ui,
                palette,
                state,
                preview,
                &RightColumnCtx {
                    ownership_banner: stage_review::ownership_banner_visible(
                        &checks.ownership,
                        pending_reinstall_id,
                    )
                    .then_some(&checks.ownership),
                    registry,
                    ownership_blocks: checks.ownership_blocks,
                    dest_non_empty: checks.dest_non_empty,
                },
            );
            ui.add_space(22.0);
            redesign_box(ui, palette, None, |ui| {
                ui.label(
                    egui::RichText::new(&scope)
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_muted(palette)),
                );
            });
        },
        |ui| {
            if redesign_btn(
                ui,
                palette,
                CANCEL_LABEL,
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                cancel_clicked = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let resp = sub_flow_footer::glyph_btn(
                    ui,
                    palette,
                    GlyphSide::Trailing("\u{2192}"),
                    begin_label,
                    true,
                    disabled,
                );
                if !disabled && resp.clicked() {
                    begin_clicked = true;
                }
            });
        },
    );

    if cancel_clicked || response.close_requested {
        InstallDrawerOutcome::Close
    } else if begin_clicked {
        if state.review.modify {
            InstallDrawerOutcome::BeginImport
        } else {
            InstallDrawerOutcome::BeginInstall
        }
    } else {
        InstallDrawerOutcome::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eet_preview() -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "0.1.0-test".to_string(),
            game_install: "EET".to_string(),
            install_mode: "build_from_scanned_mods".to_string(),
            bgee_entries: 3,
            bg2ee_entries: 4,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: "~A/A.TP2~ #0 #0 // A".to_string(),
            bg2ee_log_text: "~B/B.TP2~ #0 #0 // B".to_string(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            forked_from: Vec::new(),
        }
    }

    #[test]
    fn scope_line_names_mods_components_targets_and_credit() {
        let mut preview = eet_preview();
        preview.author = Some("BIO Team".to_string());
        let counts = InsideCounts::from_preview(&preview);
        assert_eq!(
            scope_line(&preview, &counts),
            "Installs 2 mods \u{00B7} 7 components into BGEE + BG2EE (EET). Credited to BIO Team."
        );
    }

    #[test]
    fn scope_line_says_forked_from_the_last_ancestor() {
        let mut preview = eet_preview();
        preview.author = Some("@me".to_string());
        preview.forked_from = vec![
            crate::app::modlist_share::ForkAncestor {
                name: "Root".to_string(),
                author: "@root".to_string(),
            },
            crate::app::modlist_share::ForkAncestor {
                name: "Parent".to_string(),
                author: "@parent".to_string(),
            },
        ];
        let counts = InsideCounts::from_preview(&preview);
        assert_eq!(
            scope_line(&preview, &counts),
            "Installs 2 mods \u{00B7} 7 components into BGEE + BG2EE (EET). Forked from Parent."
        );
    }

    #[test]
    fn scope_line_omits_the_credit_when_the_code_has_no_author() {
        let preview = eet_preview();
        let counts = InsideCounts::from_preview(&preview);
        assert_eq!(
            scope_line(&preview, &counts),
            "Installs 2 mods \u{00B7} 7 components into BGEE + BG2EE (EET)."
        );
    }

    #[test]
    fn single_game_target_is_the_game_name() {
        let mut preview = eet_preview();
        preview.game_install = "BGEE".to_string();
        preview.bg2ee_log_text = String::new();
        preview.bg2ee_entries = 0;
        let counts = InsideCounts::from_preview(&preview);
        assert_eq!(
            scope_line(&preview, &counts),
            "Installs 1 mod \u{00B7} 3 components into BGEE."
        );
    }
}
