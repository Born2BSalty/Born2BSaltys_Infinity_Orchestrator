// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::registry::model::ModlistRegistry;
use crate::registry::operations::{DestinationOwnership, classify_destination};
use crate::ui::install::state_install::{DestChoice, InstallScreenState, ReviewOrigin};
use crate::ui::install::{destination_field, destination_not_empty, destination_owned};
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, redesign_btn, redesign_section_header, redesign_text_input,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_soft, redesign_input_bg,
    redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

pub(crate) const FALLBACK_NAME: &str = "Shared modlist";

const MODIFY_QUESTION: &str = "Modify this modlist before installing?";
const MODIFY_NO: &str = "No, install as provided";
const MODIFY_YES: &str = "Yes, review and modify";
const HINT_INSTALL: &str = "downloads mods and installs the selected components in order";
const HINT_FORK: &str = "downloads mods, applies selection + order, then drops you on Step 2";
const NO_DISABLED_REASON: &str =
    "this share code was exported mid-install, so it can only be reviewed and modified";
const REINSTALL_ONLY_INSTALL_REASON: &str =
    "reinstall keeps this modlist as it is; to change it, open it from Home";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModifyAvailability {
    Both,
    OnlyInstall,
    OnlyModify,
}

#[must_use]
pub(crate) const fn modify_choice_available(
    origin: ReviewOrigin,
    allow_auto_install: bool,
) -> ModifyAvailability {
    if matches!(origin, ReviewOrigin::Reinstall) {
        ModifyAvailability::OnlyInstall
    } else if !allow_auto_install {
        ModifyAvailability::OnlyModify
    } else {
        ModifyAvailability::Both
    }
}

pub(crate) struct BeginGuards<'a> {
    pub(crate) name: &'a str,
    pub(crate) code: &'a str,
    pub(crate) destination_valid: bool,
    pub(crate) ownership_blocks: bool,
    pub(crate) destination_non_empty: bool,
    pub(crate) choice: Option<DestChoice>,
}

#[must_use]
pub(crate) fn ownership_banner_visible(
    classification: &DestinationOwnership,
    pending_reinstall_id: Option<&str>,
) -> bool {
    match classification {
        DestinationOwnership::Free => false,
        DestinationOwnership::ExactOwners(ids) => {
            pending_reinstall_id.is_none_or(|reinstall_id| !ids.iter().any(|id| id == reinstall_id))
        }
        DestinationOwnership::InsideOwner(_) | DestinationOwnership::ContainsOwners(_) => true,
    }
}

#[must_use]
pub(crate) fn begin_disabled(guards: &BeginGuards<'_>) -> bool {
    guards.name.trim().is_empty()
        || !guards.destination_valid
        || guards.ownership_blocks
        || (guards.destination_non_empty && guards.choice.is_none())
        || guards.code.trim().is_empty()
}

pub(crate) struct DestinationChecks {
    pub(crate) ownership: DestinationOwnership,
    pub(crate) ownership_blocks: bool,
    pub(crate) dest_valid: bool,
    pub(crate) dest_non_empty: bool,
}

#[must_use]
pub(crate) fn destination_checks(
    destination: &str,
    registry: &ModlistRegistry,
) -> DestinationChecks {
    let ownership = classify_destination(destination, registry);
    let ownership_blocks = matches!(
        &ownership,
        DestinationOwnership::InsideOwner(_) | DestinationOwnership::ContainsOwners(_)
    );
    let dest_valid = destination_is_valid(destination);
    let dest_non_empty = destination_is_non_empty(destination);
    DestinationChecks {
        ownership,
        ownership_blocks,
        dest_valid,
        dest_non_empty,
    }
}

#[must_use]
pub(crate) fn begin_disabled_for(state: &InstallScreenState, checks: &DestinationChecks) -> bool {
    begin_disabled(&BeginGuards {
        name: &state.review.name,
        code: &state.import_code,
        destination_valid: checks.dest_valid,
        ownership_blocks: checks.ownership_blocks,
        destination_non_empty: checks.dest_non_empty,
        choice: state.destination_choice,
    })
}

#[must_use]
pub(crate) const fn begin_label(modify: bool) -> &'static str {
    if modify {
        "Begin Import"
    } else {
        "Begin Install"
    }
}

pub(crate) const fn force_modify_for_availability(
    state: &mut InstallScreenState,
    preview: &ModlistSharePreview,
) {
    match modify_choice_available(state.review.origin, preview.allow_auto_install) {
        ModifyAvailability::OnlyModify => state.review.modify = true,
        ModifyAvailability::OnlyInstall => state.review.modify = false,
        ModifyAvailability::Both => {}
    }
}

pub(crate) struct RightColumnCtx<'a> {
    pub(crate) ownership_banner: Option<&'a DestinationOwnership>,
    pub(crate) registry: &'a ModlistRegistry,
    pub(crate) ownership_blocks: bool,
    pub(crate) dest_non_empty: bool,
}

pub(crate) fn render_install_settings(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
    preview: &ModlistSharePreview,
    ctx: &RightColumnCtx<'_>,
) {
    redesign_section_header(ui, palette, "Installation settings", None);
    ui.add_space(12.0);

    field_label(ui, palette, "modlist name");
    name_input(ui, palette, &mut state.review.name);
    ui.add_space(14.0);

    if destination_field::render(ui, palette, &mut state.destination, ctx.ownership_blocks) {
        state.destination_choice = None;
    }

    if let Some(ownership) = ctx.ownership_banner {
        destination_owned::render(ui, palette, ownership, ctx.registry);
    }

    if !ctx.ownership_blocks
        && ctx.dest_non_empty
        && let Some(picked) = destination_not_empty::render(ui, palette, state.destination_choice)
    {
        state.destination_choice = Some(picked);
    }

    ui.add_space(16.0);
    divider(ui, palette);
    ui.add_space(16.0);

    ui.label(
        egui::RichText::new(MODIFY_QUESTION)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );
    ui.add_space(8.0);
    modify_toggle(
        ui,
        palette,
        &mut state.review.modify,
        modify_choice_available(state.review.origin, preview.allow_auto_install),
    );

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(if state.review.modify {
            HINT_FORK
        } else {
            HINT_INSTALL
        })
        .size(13.0)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(redesign_text_faint(palette)),
    );
}

fn modify_toggle(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    modify: &mut bool,
    availability: ModifyAvailability,
) {
    let no_disabled = availability == ModifyAvailability::OnlyModify;
    let yes_disabled = availability == ModifyAvailability::OnlyInstall;
    let reason = match availability {
        ModifyAvailability::OnlyModify => Some(NO_DISABLED_REASON),
        ModifyAvailability::OnlyInstall => Some(REINSTALL_ONLY_INSTALL_REASON),
        ModifyAvailability::Both => None,
    };

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);

        let no_response = redesign_btn(
            ui,
            palette,
            MODIFY_NO,
            BtnOpts {
                small: true,
                primary: !*modify,
                disabled: no_disabled,
                ..Default::default()
            },
        );
        if no_disabled {
            no_response.on_hover_text(NO_DISABLED_REASON);
        } else if no_response.clicked() {
            *modify = false;
        }

        let yes_response = redesign_btn(
            ui,
            palette,
            MODIFY_YES,
            BtnOpts {
                small: true,
                primary: *modify,
                disabled: yes_disabled,
                ..Default::default()
            },
        );
        if yes_disabled {
            yes_response.on_hover_text(REINSTALL_ONLY_INSTALL_REASON);
        } else if yes_response.clicked() {
            *modify = true;
        }
    });

    if let Some(reason) = reason {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(reason)
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        );
    }
}

#[must_use]
pub(crate) fn display_name(review_name: &str, preview: &ModlistSharePreview) -> String {
    let trimmed = review_name.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_NAME)
        .to_string()
}

fn destination_is_valid(path: &str) -> bool {
    let trimmed = path.trim();
    !trimmed.is_empty() && std::path::Path::new(trimmed).is_dir()
}

fn destination_is_non_empty(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    std::fs::read_dir(trimmed).is_ok_and(|mut entries| entries.next().is_some())
}

fn field_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);
}

fn name_input(ui: &mut egui::Ui, palette: ThemePalette, value: &mut String) {
    let margin = egui::Margin {
        left: 12,
        right: 12,
        top: 8,
        bottom: 8,
    };
    let box_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            14.0,
            egui::FontFamily::Name("poppins_light".into()),
        ))
    }) + f32::from(margin.top)
        + f32::from(margin.bottom);
    let width = (ui.available_width() - 4.0).max(120.0);

    redesign_text_input(
        ui,
        palette,
        InputOpts {
            edit: egui::TextEdit::singleline(value)
                .font(egui::FontId::new(
                    13.0,
                    egui::FontFamily::Name("poppins_light".into()),
                ))
                .hint_text(
                    egui::RichText::new("name this modlist")
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_faint(palette)),
                )
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette))
                .vertical_align(egui::Align::Center)
                .margin(margin),
            margin,
            size: egui::vec2(width, box_h),
            border: None,
        },
    );
}

fn divider(ui: &mut egui::Ui, palette: ThemePalette) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), REDESIGN_BORDER_WIDTH_PX),
        egui::Sense::hover(),
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.center().y),
            egui::pos2(rect.right(), rect.center().y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_good<'a>() -> BeginGuards<'a> {
        BeginGuards {
            name: "Tactical EET",
            code: "BIO-MODLIST-V1:CODE",
            destination_valid: true,
            ownership_blocks: false,
            destination_non_empty: false,
            choice: None,
        }
    }

    #[test]
    fn modify_choice_is_both_for_a_verified_code() {
        assert_eq!(
            modify_choice_available(ReviewOrigin::Details, true),
            ModifyAvailability::Both
        );
        assert_eq!(
            modify_choice_available(ReviewOrigin::Paste, true),
            ModifyAvailability::Both
        );
    }

    #[test]
    fn modify_choice_is_only_modify_when_auto_install_is_forbidden() {
        assert_eq!(
            modify_choice_available(ReviewOrigin::Details, false),
            ModifyAvailability::OnlyModify
        );
        assert_eq!(
            modify_choice_available(ReviewOrigin::Paste, false),
            ModifyAvailability::OnlyModify
        );
    }

    #[test]
    fn modify_choice_is_only_install_on_reinstall_even_for_a_verified_code() {
        assert_eq!(
            modify_choice_available(ReviewOrigin::Reinstall, true),
            ModifyAvailability::OnlyInstall
        );
    }

    #[test]
    fn modify_choice_is_only_install_on_reinstall_when_auto_install_is_forbidden() {
        assert_eq!(
            modify_choice_available(ReviewOrigin::Reinstall, false),
            ModifyAvailability::OnlyInstall
        );
    }

    #[test]
    fn begin_is_enabled_when_every_guard_is_satisfied() {
        assert!(!begin_disabled(&all_good()));
    }

    #[test]
    fn begin_is_disabled_without_a_name() {
        let mut guards = all_good();
        guards.name = "   ";
        assert!(begin_disabled(&guards));
    }

    #[test]
    fn begin_is_disabled_when_the_destination_is_not_a_real_folder() {
        let mut guards = all_good();
        guards.destination_valid = false;
        assert!(begin_disabled(&guards));
    }

    #[test]
    fn begin_is_disabled_while_another_modlist_owns_the_folder() {
        let mut guards = all_good();
        guards.ownership_blocks = true;
        assert!(begin_disabled(&guards));
    }

    #[test]
    fn begin_is_disabled_for_a_non_empty_folder_until_a_choice_is_made() {
        let mut guards = all_good();
        guards.destination_non_empty = true;
        assert!(begin_disabled(&guards));

        guards.choice = Some(DestChoice::Clear);
        assert!(!begin_disabled(&guards));

        guards.choice = Some(DestChoice::Backup);
        assert!(!begin_disabled(&guards));
    }

    #[test]
    fn begin_is_disabled_without_a_share_code() {
        let mut guards = all_good();
        guards.code = "  ";
        assert!(begin_disabled(&guards));
    }

    #[test]
    fn display_name_prefers_the_typed_name_then_the_code_then_the_fallback() {
        let mut preview = crate::app::modlist_share::ModlistSharePreview {
            bio_version: String::new(),
            game_install: "EET".to_string(),
            install_mode: String::new(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: String::new(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: Some("From the code".to_string()),
            author: None,
            forked_from: Vec::new(),
        };
        assert_eq!(display_name("  Typed  ", &preview), "Typed");
        assert_eq!(display_name("   ", &preview), "From the code");
        preview.name = None;
        assert_eq!(display_name("", &preview), FALLBACK_NAME);
    }

    #[test]
    fn hint_copy_matches_the_two_pipelines() {
        assert_eq!(
            HINT_INSTALL,
            "downloads mods and installs the selected components in order"
        );
        assert_eq!(
            HINT_FORK,
            "downloads mods, applies selection + order, then drops you on Step 2"
        );
        assert_eq!(MODIFY_NO, "No, install as provided");
        assert_eq!(MODIFY_YES, "Yes, review and modify");
    }

    #[test]
    fn ownership_banner_hidden_for_free() {
        assert!(!ownership_banner_visible(&DestinationOwnership::Free, None));
        assert!(!ownership_banner_visible(
            &DestinationOwnership::Free,
            Some("REINSTALL0001")
        ));
    }

    #[test]
    fn ownership_banner_hidden_when_reinstalling_the_exact_owner() {
        let classification = DestinationOwnership::ExactOwners(vec!["REINSTALL0001".to_string()]);
        assert!(!ownership_banner_visible(
            &classification,
            Some("REINSTALL0001")
        ));
    }

    #[test]
    fn ownership_banner_shown_for_exact_owner_that_is_not_the_reinstall() {
        let classification = DestinationOwnership::ExactOwners(vec!["OTHER0000001".to_string()]);
        assert!(ownership_banner_visible(
            &classification,
            Some("REINSTALL0001")
        ));
        assert!(ownership_banner_visible(&classification, None));
    }

    #[test]
    fn ownership_banner_shown_for_inside_owner_even_when_reinstalling() {
        let classification = DestinationOwnership::InsideOwner("REINSTALL0001".to_string());
        assert!(ownership_banner_visible(
            &classification,
            Some("REINSTALL0001")
        ));
    }

    #[test]
    fn ownership_banner_shown_for_contains_owners_even_when_reinstalling() {
        let classification =
            DestinationOwnership::ContainsOwners(vec!["REINSTALL0001".to_string()]);
        assert!(ownership_banner_visible(
            &classification,
            Some("REINSTALL0001")
        ));
    }

    #[test]
    fn begin_disabled_for_mirrors_begin_disabled() {
        let good_checks = DestinationChecks {
            ownership: DestinationOwnership::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let good_state = InstallScreenState {
            review: crate::ui::install::state_install::ReviewState {
                name: "Tactical EET".to_string(),
                ..Default::default()
            },
            import_code: "BIO-MODLIST-V1:CODE".to_string(),
            ..Default::default()
        };
        assert!(!begin_disabled_for(&good_state, &good_checks));

        let blocked_checks = DestinationChecks {
            ownership: DestinationOwnership::Free,
            ownership_blocks: true,
            dest_valid: true,
            dest_non_empty: false,
        };
        assert!(begin_disabled_for(&good_state, &blocked_checks));
    }
}
