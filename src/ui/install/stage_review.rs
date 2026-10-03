// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::compat_dlc_source::{SourceNotice, SourceNoticeSeverity, SourceRemedy};
use crate::app::modlist_share::ModlistSharePreview;
use crate::registry::destination_claim::{
    ClaimContext, DestinationClaim, resolve_destination_claim,
};
use crate::registry::model::ModlistRegistry;
use crate::ui::install::state_install::{DestChoice, InstallScreenState, ReviewOrigin};
use crate::ui::install::{destination_field, destination_not_empty, destination_owned};
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, redesign_btn, redesign_section_header, redesign_text_input,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_soft, redesign_input_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary, redesign_with_alpha,
};

pub(crate) const FALLBACK_NAME: &str = "Shared modlist";

const MODIFY_QUESTION: &str = "Modify this modlist before installing?";
const MODIFY_NO: &str = "No, install as provided";
const MODIFY_YES: &str = "Yes, review and modify";
const HINT_INSTALL: &str = "downloads mods and installs the selected components in order";
const HINT_FORK: &str = "downloads mods, applies selection + order, then drops you on Step 2";
const HINT_FORK_ADD_MODS: &str = "to add mods, extract them into the modlist's Mods folder and rescan; the ? on the Mods / Components page shows where";
const NO_DISABLED_REASON: &str =
    "this share code was exported mid-install, so it can only be reviewed and modified";
const REINSTALL_ONLY_INSTALL_REASON: &str =
    "reinstall keeps this modlist as it is; to change it, open it from Home";
const REINSTALL_FOLDER_MISSING: &str =
    "This folder no longer exists. Use Create to install the list somewhere else.";
const REINSTALL_MODE_FACT: &str =
    "As provided. A reinstall keeps the list's mods and order; use Create for a changed copy.";

#[must_use]
pub(crate) const fn reinstall_locked(origin: ReviewOrigin) -> bool {
    matches!(origin, ReviewOrigin::Reinstall)
}

#[must_use]
pub(crate) const fn reinstall_folder_note(dest_valid: bool) -> Option<&'static str> {
    if dest_valid {
        None
    } else {
        Some(REINSTALL_FOLDER_MISSING)
    }
}

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
pub(crate) fn begin_disabled(guards: &BeginGuards<'_>) -> bool {
    guards.name.trim().is_empty()
        || !guards.destination_valid
        || guards.ownership_blocks
        || (guards.destination_non_empty && guards.choice.is_none())
        || guards.code.trim().is_empty()
}

pub(crate) struct DestinationChecks {
    pub(crate) claim: DestinationClaim,
    pub(crate) ownership_blocks: bool,
    pub(crate) dest_valid: bool,
    pub(crate) dest_non_empty: bool,
}

#[must_use]
pub(crate) fn destination_checks(
    destination: &str,
    registry: &ModlistRegistry,
    held_id: Option<&str>,
    installing_id: Option<&str>,
) -> DestinationChecks {
    let claim = resolve_destination_claim(
        registry,
        &ClaimContext {
            destination,
            held_id,
            installing_id,
        },
    );
    let ownership_blocks = claim.blocks();
    let dest_valid = destination_is_valid(destination);
    let dest_non_empty = destination_is_non_empty(destination);
    DestinationChecks {
        claim,
        ownership_blocks,
        dest_valid,
        dest_non_empty,
    }
}

#[must_use]
pub(crate) fn source_notice_blocks(state: &InstallScreenState) -> bool {
    state
        .source_compat_issue
        .as_ref()
        .is_some_and(|notice| notice.severity == SourceNoticeSeverity::Warning)
}

#[must_use]
pub(crate) fn begin_disabled_for(state: &InstallScreenState, checks: &DestinationChecks) -> bool {
    state.missing_source_issue.is_some()
        || state.game_version_issue.is_some()
        || (source_notice_blocks(state) && !state.review.modify)
        || begin_disabled(&BeginGuards {
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
    pub(crate) ownership_banner: Option<&'a DestinationClaim>,
    pub(crate) registry: &'a ModlistRegistry,
    pub(crate) ownership_blocks: bool,
    pub(crate) dest_non_empty: bool,
    pub(crate) dest_valid: bool,
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

    let locked = reinstall_locked(state.review.origin);

    if locked {
        fact_row(ui, palette, "destination folder", state.destination.trim());
        if let Some(note) = reinstall_folder_note(ctx.dest_valid) {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(note)
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            );
        }
    } else if destination_field::render(ui, palette, &mut state.destination, ctx.ownership_blocks) {
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

    render_all_source_notices(ui, palette, state, locked);

    if locked {
        fact_row(ui, palette, "install mode", REINSTALL_MODE_FACT);
        return;
    }

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

    if state.review.modify {
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(HINT_FORK_ADD_MODS)
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_faint(palette)),
        );
    }
}

fn render_all_source_notices(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &InstallScreenState,
    locked: bool,
) {
    let action = source_warning_action(locked, state.review.modify);
    if let Some(msg) = state.missing_source_issue.clone() {
        render_source_notice(
            ui,
            palette,
            &SourceNotice {
                severity: SourceNoticeSeverity::Warning,
                text: msg,
                remedy: SourceRemedy::SetSourceFolder,
            },
            action,
        );
        ui.add_space(16.0);
    }

    if let Some(msg) = state.game_version_issue.clone() {
        render_source_notice(
            ui,
            palette,
            &SourceNotice {
                severity: SourceNoticeSeverity::Warning,
                text: msg,
                remedy: SourceRemedy::WrongGameVersion,
            },
            action,
        );
        ui.add_space(16.0);
    }

    if let Some(notice) = state.source_compat_issue.clone() {
        render_source_notice(ui, palette, &notice, action);
        ui.add_space(16.0);
    }

    if let Some(notice) = state.source_residue_issue.clone() {
        render_source_notice(ui, palette, &notice, action);
        ui.add_space(16.0);
    }

    if let Some(notice) = state.unresolved_sources_issue.clone() {
        render_source_notice(ui, palette, &notice, action);
        ui.add_space(16.0);
    }
}

fn fact_row(ui: &mut egui::Ui, palette: ThemePalette, label: &str, value: &str) {
    field_label(ui, palette, label);
    ui.label(
        egui::RichText::new(value)
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceWarningAction {
    Reinstall,
    ChooseModify,
    ModifyOn,
}

#[must_use]
pub(crate) const fn source_warning_action(locked: bool, modify_on: bool) -> SourceWarningAction {
    if locked {
        SourceWarningAction::Reinstall
    } else if modify_on {
        SourceWarningAction::ModifyOn
    } else {
        SourceWarningAction::ChooseModify
    }
}

const SOURCE_WARNING_TITLE_ORDER_MERGER: &str = "DLC Merger required";
const SOURCE_WARNING_TITLE_CHANGE_SOURCE: &str = "BGEE source mismatch";
const SOURCE_WARNING_TITLE_CLEAN_SOURCE: &str = "Modded game source";
const SOURCE_WARNING_TITLE_SET_SOURCE_FOLDER: &str = "Game folder not set";
const SOURCE_WARNING_TITLE_SET_GLOBAL_MODS_FOLDER: &str = "Global mods folder not set";
const SOURCE_WARNING_TITLE_WRONG_GAME_VERSION: &str = "Wrong game version";
const SOURCE_WARNING_ACTION_REINSTALL: &str = "This list cannot be reinstalled as provided. Use Create to make a modified copy with DLC Merger first in the BGEE installation order.";
const SOURCE_WARNING_ACTION_CHOOSE_MODIFY: &str = "Choose \"Yes, review and modify\" when you install, then put DLC Merger first in the BGEE installation order.";
const SOURCE_WARNING_ACTION_MODIFY_ON: &str =
    "Once the workspace opens, put DLC Merger first in the BGEE installation order.";
const SOURCE_WARNING_ACTION_REINSTALL_CHANGE_SOURCE: &str = "This list cannot be reinstalled against this BGEE source. Point Settings at a source that matches the list, then reinstall.";
const SOURCE_WARNING_ACTION_CHOOSE_MODIFY_CHANGE_SOURCE: &str = "Choose \"Yes, review and modify\" to remove DLC Merger, or point Settings at a BGEE source that matches the list.";
const SOURCE_WARNING_ACTION_MODIFY_ON_CHANGE_SOURCE: &str = "Once the workspace opens, remove DLC Merger, or point Settings at a BGEE source that matches the list.";
const SOURCE_WARNING_ACTION_CLEAN_SOURCE: &str = "Mods already in the source can conflict with this list. Point Settings at a clean install, or proceed anyway.";

#[must_use]
pub(crate) const fn source_warning_title(remedy: SourceRemedy) -> &'static str {
    match remedy {
        SourceRemedy::OrderMerger => SOURCE_WARNING_TITLE_ORDER_MERGER,
        SourceRemedy::ChangeSource => SOURCE_WARNING_TITLE_CHANGE_SOURCE,
        SourceRemedy::CleanSource => SOURCE_WARNING_TITLE_CLEAN_SOURCE,
        SourceRemedy::SetSourceFolder => SOURCE_WARNING_TITLE_SET_SOURCE_FOLDER,
        SourceRemedy::SetGlobalModsFolder => SOURCE_WARNING_TITLE_SET_GLOBAL_MODS_FOLDER,
        SourceRemedy::WrongGameVersion => SOURCE_WARNING_TITLE_WRONG_GAME_VERSION,
        SourceRemedy::None => "",
    }
}

#[must_use]
pub(crate) const fn source_warning_action_text(
    action: SourceWarningAction,
    remedy: SourceRemedy,
) -> &'static str {
    match (remedy, action) {
        (SourceRemedy::OrderMerger, SourceWarningAction::Reinstall) => {
            SOURCE_WARNING_ACTION_REINSTALL
        }
        (SourceRemedy::OrderMerger, SourceWarningAction::ChooseModify) => {
            SOURCE_WARNING_ACTION_CHOOSE_MODIFY
        }
        (SourceRemedy::OrderMerger, SourceWarningAction::ModifyOn) => {
            SOURCE_WARNING_ACTION_MODIFY_ON
        }
        (SourceRemedy::ChangeSource, SourceWarningAction::Reinstall) => {
            SOURCE_WARNING_ACTION_REINSTALL_CHANGE_SOURCE
        }
        (SourceRemedy::ChangeSource, SourceWarningAction::ChooseModify) => {
            SOURCE_WARNING_ACTION_CHOOSE_MODIFY_CHANGE_SOURCE
        }
        (SourceRemedy::ChangeSource, SourceWarningAction::ModifyOn) => {
            SOURCE_WARNING_ACTION_MODIFY_ON_CHANGE_SOURCE
        }
        (SourceRemedy::CleanSource, _) => SOURCE_WARNING_ACTION_CLEAN_SOURCE,
        (
            SourceRemedy::SetSourceFolder
            | SourceRemedy::SetGlobalModsFolder
            | SourceRemedy::WrongGameVersion
            | SourceRemedy::None,
            _,
        ) => "",
    }
}

pub(crate) fn render_source_warning(
    ui: &mut egui::Ui,
    issue: &str,
    action: SourceWarningAction,
    remedy: SourceRemedy,
) {
    let body_color = egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 0xCC);
    let title = source_warning_title(remedy);
    let action_text = source_warning_action_text(action, remedy);
    egui::Frame::default()
        .fill(destination_not_empty::warn_fill())
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            destination_not_empty::WARN_BORDER,
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            if !title.is_empty() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let (icon_rect, _) =
                        ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
                    destination_not_empty::paint_warning_triangle(
                        ui.painter(),
                        icon_rect.center(),
                        destination_not_empty::WARN_INK,
                    );
                    ui.label(
                        egui::RichText::new(title)
                            .size(13.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(destination_not_empty::WARN_INK),
                    );
                });
                ui.add_space(4.0);
            }

            ui.add(
                egui::Label::new(
                    egui::RichText::new(issue)
                        .size(14.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(body_color),
                )
                .wrap(),
            );

            if !action_text.is_empty() {
                ui.add_space(6.0);

                ui.add(
                    egui::Label::new(
                        egui::RichText::new(action_text)
                            .size(14.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(body_color),
                    )
                    .wrap(),
                );
            }
        });
}

pub(crate) fn render_source_notice(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    notice: &SourceNotice,
    action: SourceWarningAction,
) {
    if notice.severity == SourceNoticeSeverity::Warning {
        render_source_warning(ui, &notice.text, action, notice.remedy);
        return;
    }
    egui::Frame::default()
        .fill(redesign_with_alpha(redesign_accent(palette), 1, 8))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_accent(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(notice.text.clone())
                        .size(14.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_primary(palette)),
                )
                .wrap(),
            );
        });
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

    #[test]
    fn source_issue_blocks_direct_install_but_allows_modify() {
        let checks = DestinationChecks {
            claim: DestinationClaim::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let mut state = InstallScreenState::default();
        state.review.name = "Test".to_string();
        state.import_code = "code".to_string();
        assert!(!begin_disabled_for(&state, &checks));
        state.source_compat_issue = Some(SourceNotice {
            severity: SourceNoticeSeverity::Warning,
            text: "DLC Merger required".to_string(),
            remedy: SourceRemedy::OrderMerger,
        });
        assert!(begin_disabled_for(&state, &checks));
        state.review.modify = true;
        assert!(!begin_disabled_for(&state, &checks));
        assert!(state.source_compat_issue.is_some());
        state.clear_preview();
        assert!(state.source_compat_issue.is_none());
        state.source_compat_issue = Some(SourceNotice {
            severity: SourceNoticeSeverity::Info,
            text: "Needs Siege of Dragonspear".to_string(),
            remedy: SourceRemedy::OrderMerger,
        });
        state.review.modify = false;
        assert!(!begin_disabled_for(&state, &checks));
    }

    #[test]
    fn residue_notice_never_blocks_begin() {
        let checks = DestinationChecks {
            claim: DestinationClaim::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let state = InstallScreenState {
            review: crate::ui::install::state_install::ReviewState {
                name: "Tactical EET".to_string(),
                ..Default::default()
            },
            import_code: "BIO-MODLIST-V1:CODE".to_string(),
            source_residue_issue: Some(SourceNotice {
                severity: SourceNoticeSeverity::Warning,
                text: "Your BG2EE source at C:\\Games\\test is not a clean install: WeiDU.log."
                    .to_string(),
                remedy: SourceRemedy::CleanSource,
            }),
            ..Default::default()
        };
        assert!(!begin_disabled_for(&state, &checks));
    }

    #[test]
    fn unresolved_sources_notice_never_blocks_begin() {
        let checks = DestinationChecks {
            claim: DestinationClaim::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let state = InstallScreenState {
            review: crate::ui::install::state_install::ReviewState {
                name: "Tactical EET".to_string(),
                ..Default::default()
            },
            import_code: "BIO-MODLIST-V1:CODE".to_string(),
            unresolved_sources_issue: Some(SourceNotice {
                severity: SourceNoticeSeverity::Warning,
                text:
                    "2 mods have no download source: X, Y. BIO will not be able to download them."
                        .to_string(),
                remedy: SourceRemedy::None,
            }),
            ..Default::default()
        };
        assert!(!begin_disabled_for(&state, &checks));
    }

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
            game_version: None,
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
            description: None,
            forked_from: Vec::new(),
            unresolved_mods: Vec::new(),
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

    fn reg_with(id: &str, name: &str, dest: &str) -> ModlistRegistry {
        use crate::registry::model::{Game, ModlistEntry, ModlistState};
        let mut r = ModlistRegistry::default();
        r.entries.push(ModlistEntry {
            id: id.to_string(),
            name: name.to_string(),
            game: Game::EET,
            destination_folder: dest.to_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        r
    }

    #[test]
    fn checks_adopt_reinstalled_list_without_banner() {
        let reg = reg_with("REINSTALL0001", "Tactical EET", "D:\\dest");
        let checks = destination_checks("D:\\dest", &reg, Some("REINSTALL0001"), None);
        assert_eq!(
            checks.claim,
            DestinationClaim::Adopt("REINSTALL0001".to_string())
        );
        assert!(!checks.ownership_blocks);
    }

    #[test]
    fn checks_replace_shows_banner_and_allows_begin() {
        let reg = reg_with("OTHER0000001", "Other List", "D:\\dest");
        let checks = destination_checks("D:\\dest", &reg, None, None);
        assert_eq!(
            checks.claim,
            DestinationClaim::Replace(vec!["OTHER0000001".to_string()])
        );
        assert!(!checks.ownership_blocks);
    }

    #[test]
    fn checks_refused_blocks_begin() {
        let reg = reg_with("BUSY0000001", "Busy List", "D:\\dest");
        let checks = destination_checks("D:\\dest", &reg, None, Some("BUSY0000001"));
        assert!(checks.ownership_blocks);
        assert!(matches!(checks.claim, DestinationClaim::Refused(_)));
    }

    #[test]
    fn reinstall_folder_note_appears_only_when_the_folder_is_gone() {
        assert_eq!(reinstall_folder_note(true), None);
        assert_eq!(reinstall_folder_note(false), Some(REINSTALL_FOLDER_MISSING));
    }

    #[test]
    fn reinstall_locks_everything_but_name_and_folder_choice() {
        assert!(reinstall_locked(ReviewOrigin::Reinstall));
        assert!(!reinstall_locked(ReviewOrigin::Details));
        assert!(!reinstall_locked(ReviewOrigin::Paste));
    }

    #[test]
    fn source_warning_action_follows_the_drawer_state() {
        assert_eq!(
            source_warning_action(true, false),
            SourceWarningAction::Reinstall
        );
        assert_eq!(
            source_warning_action(true, true),
            SourceWarningAction::Reinstall
        );
        assert_eq!(
            source_warning_action(false, true),
            SourceWarningAction::ModifyOn
        );
        assert_eq!(
            source_warning_action(false, false),
            SourceWarningAction::ChooseModify
        );
    }

    #[test]
    fn source_warning_action_text_follows_the_remedy() {
        assert_eq!(
            source_warning_action_text(SourceWarningAction::Reinstall, SourceRemedy::OrderMerger),
            SOURCE_WARNING_ACTION_REINSTALL
        );
        assert_eq!(
            source_warning_action_text(
                SourceWarningAction::ChooseModify,
                SourceRemedy::OrderMerger
            ),
            SOURCE_WARNING_ACTION_CHOOSE_MODIFY
        );
        assert_eq!(
            source_warning_action_text(SourceWarningAction::ModifyOn, SourceRemedy::OrderMerger),
            SOURCE_WARNING_ACTION_MODIFY_ON
        );
        assert_eq!(
            source_warning_action_text(SourceWarningAction::Reinstall, SourceRemedy::ChangeSource),
            "This list cannot be reinstalled against this BGEE source. Point Settings at a source that matches the list, then reinstall."
        );
        assert_eq!(
            source_warning_action_text(
                SourceWarningAction::ChooseModify,
                SourceRemedy::ChangeSource
            ),
            "Choose \"Yes, review and modify\" to remove DLC Merger, or point Settings at a BGEE source that matches the list."
        );
        assert_eq!(
            source_warning_action_text(SourceWarningAction::ModifyOn, SourceRemedy::ChangeSource),
            "Once the workspace opens, remove DLC Merger, or point Settings at a BGEE source that matches the list."
        );
        for action in [
            SourceWarningAction::Reinstall,
            SourceWarningAction::ChooseModify,
            SourceWarningAction::ModifyOn,
        ] {
            assert_eq!(
                source_warning_action_text(action, SourceRemedy::CleanSource),
                "Mods already in the source can conflict with this list. Point Settings at a clean install, or proceed anyway."
            );
            assert_eq!(source_warning_action_text(action, SourceRemedy::None), "");
        }
    }

    #[test]
    fn source_warning_title_follows_the_remedy() {
        assert_eq!(
            source_warning_title(SourceRemedy::OrderMerger),
            "DLC Merger required"
        );
        assert_eq!(
            source_warning_title(SourceRemedy::ChangeSource),
            "BGEE source mismatch"
        );
        assert_eq!(
            source_warning_title(SourceRemedy::CleanSource),
            "Modded game source"
        );
        assert_eq!(source_warning_title(SourceRemedy::None), "");
    }

    #[test]
    fn begin_disabled_for_mirrors_begin_disabled() {
        let good_checks = DestinationChecks {
            claim: DestinationClaim::Free,
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
            claim: DestinationClaim::Free,
            ownership_blocks: true,
            dest_valid: true,
            dest_non_empty: false,
        };
        assert!(begin_disabled_for(&good_state, &blocked_checks));
    }

    #[test]
    fn missing_source_folder_holds_begin_even_with_modify() {
        let checks = DestinationChecks {
            claim: DestinationClaim::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let mut state = InstallScreenState {
            review: crate::ui::install::state_install::ReviewState {
                name: "Tactical EET".to_string(),
                ..Default::default()
            },
            import_code: "BIO-MODLIST-V1:CODE".to_string(),
            missing_source_issue: Some(
                "The IWDEE game folder is not set. Set it in Settings \u{2192} Paths.".to_string(),
            ),
            ..Default::default()
        };
        assert!(begin_disabled_for(&state, &checks));
        state.review.modify = true;
        assert!(begin_disabled_for(&state, &checks));
        state.review.modify = false;
        assert!(begin_disabled_for(&state, &checks));
    }

    #[test]
    fn missing_source_folder_holds_begin_on_reinstall() {
        let checks = DestinationChecks {
            claim: DestinationClaim::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let state = InstallScreenState {
            review: crate::ui::install::state_install::ReviewState {
                name: "Tactical EET".to_string(),
                origin: ReviewOrigin::Reinstall,
                ..Default::default()
            },
            import_code: "BIO-MODLIST-V1:CODE".to_string(),
            missing_source_issue: Some(
                "The IWDEE game folder is not set. Set it in Settings \u{2192} Paths.".to_string(),
            ),
            ..Default::default()
        };
        assert!(begin_disabled_for(&state, &checks));
    }

    #[test]
    fn begin_is_free_once_the_folder_is_set() {
        let checks = DestinationChecks {
            claim: DestinationClaim::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let state = InstallScreenState {
            review: crate::ui::install::state_install::ReviewState {
                name: "Tactical EET".to_string(),
                ..Default::default()
            },
            import_code: "BIO-MODLIST-V1:CODE".to_string(),
            missing_source_issue: None,
            ..Default::default()
        };
        assert!(!begin_disabled_for(&state, &checks));
    }

    #[test]
    fn version_issue_holds_begin_even_with_modify() {
        let checks = DestinationChecks {
            claim: DestinationClaim::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let mut state = InstallScreenState {
            review: crate::ui::install::state_install::ReviewState {
                name: "Tactical EET".to_string(),
                ..Default::default()
            },
            import_code: "BIO-MODLIST-V1:CODE".to_string(),
            missing_source_issue: None,
            game_version_issue: Some(
                "This modlist needs game version 2.6. The BG2EE game folder is version 2.7. Point Settings \u{2192} Paths at a game folder on version 2.6.".to_string(),
            ),
            ..Default::default()
        };
        assert!(begin_disabled_for(&state, &checks));
        state.review.modify = true;
        assert!(begin_disabled_for(&state, &checks));
        state.review.modify = false;
        assert!(begin_disabled_for(&state, &checks));
    }

    #[test]
    fn version_issue_holds_begin_on_reinstall() {
        let checks = DestinationChecks {
            claim: DestinationClaim::Free,
            ownership_blocks: false,
            dest_valid: true,
            dest_non_empty: false,
        };
        let state = InstallScreenState {
            review: crate::ui::install::state_install::ReviewState {
                name: "Tactical EET".to_string(),
                origin: ReviewOrigin::Reinstall,
                ..Default::default()
            },
            import_code: "BIO-MODLIST-V1:CODE".to_string(),
            missing_source_issue: None,
            game_version_issue: Some(
                "This modlist needs game version 2.6. The BG2EE game folder is version 2.7. Point Settings \u{2192} Paths at a game folder on version 2.6.".to_string(),
            ),
            ..Default::default()
        };
        assert!(begin_disabled_for(&state, &checks));
    }

    #[test]
    fn wrong_game_version_title_and_empty_action() {
        assert_eq!(
            source_warning_title(SourceRemedy::WrongGameVersion),
            "Wrong game version"
        );
        for action in [
            SourceWarningAction::Reinstall,
            SourceWarningAction::ChooseModify,
            SourceWarningAction::ModifyOn,
        ] {
            assert_eq!(
                source_warning_action_text(action, SourceRemedy::WrongGameVersion),
                ""
            );
        }
    }

    #[test]
    fn set_source_folder_remedy_has_a_title_and_no_action_line() {
        assert_eq!(
            source_warning_title(SourceRemedy::SetSourceFolder),
            "Game folder not set"
        );
        for action in [
            SourceWarningAction::Reinstall,
            SourceWarningAction::ChooseModify,
            SourceWarningAction::ModifyOn,
        ] {
            assert_eq!(
                source_warning_action_text(action, SourceRemedy::SetSourceFolder),
                ""
            );
        }
    }

    #[test]
    fn set_global_mods_folder_remedy_has_a_title_and_no_action_text() {
        assert_eq!(
            source_warning_title(SourceRemedy::SetGlobalModsFolder),
            "Global mods folder not set"
        );
        for action in [
            SourceWarningAction::Reinstall,
            SourceWarningAction::ChooseModify,
            SourceWarningAction::ModifyOn,
        ] {
            assert_eq!(
                source_warning_action_text(action, SourceRemedy::SetGlobalModsFolder),
                ""
            );
        }
    }
}
