// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::github_release_list::{
    CachedAsset, CachedRelease, ReleaseListState, ReleaseListStatus,
};
use crate::app::source_form::{self, AssetPick, Follow, SourceForm, SourceKind};
use crate::app::step2_action::ModSourceEditDestination;
use crate::ui::orchestrator::widgets::{BtnOpts, InputOpts, redesign_btn, redesign_text_input};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_soft, redesign_border_strong, redesign_chrome_bg, redesign_hover_overlay,
    redesign_input_bg, redesign_pill_text, redesign_shell_bg, redesign_text_faint,
    redesign_text_muted, redesign_text_primary, redesign_warning,
};

use super::versions_icons;

const FIELD_H: f32 = 36.0;
const FIELD_MARGIN: egui::Margin = egui::Margin {
    left: 10,
    right: 10,
    top: 9,
    bottom: 9,
};
const SOURCE_KIND_W: f32 = 170.0;
const ROW_GAP: f32 = 16.0;
const TOGGLE_ROW_OFFSET: f32 = 24.0;

pub(crate) struct FormEnv<'a> {
    pub(crate) has_modlist_destination: bool,
    pub(crate) logged_in: bool,
    pub(crate) release_list: &'a ReleaseListState,
}

#[derive(Default)]
pub(crate) struct FormOutcome {
    pub(crate) request_release_list: Option<String>,
}

pub(crate) fn render(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    env: &FormEnv<'_>,
) -> FormOutcome {
    let mut outcome = FormOutcome::default();

    if env.has_modlist_destination {
        render_save_to(ui, palette, form);
        ui.add_space(ROW_GAP);
    }

    render_source_row(ctx, ui, palette, form);
    ui.add_space(ROW_GAP);

    if form.kind == SourceKind::GitHub {
        render_follow_pills(ui, palette, form);
        ui.add_space(ROW_GAP);
        render_follow_body(ctx, ui, palette, form, env, &mut outcome);
        if let Some(notice) = form.notice.clone() {
            ui.add_space(8.0);
            render_notice(ui, palette, &notice);
        }
        ui.add_space(ROW_GAP);
    }

    render_note_row(ui, palette);
    ui.add_space(ROW_GAP);
    render_advanced(ui, palette, form);
    ui.add_space(ROW_GAP);
    render_will_fetch(ui, palette, form);

    outcome
}

fn menu_id(card_key: &str, which: &str) -> egui::Id {
    egui::Id::new((
        "versions_form_menu",
        card_key.to_string(),
        which.to_string(),
    ))
}

pub(crate) fn reset_dropdown_state(ctx: &egui::Context, card_key: &str) {
    for which in ["kind", "release", "asset"] {
        set_menu_open(ctx, menu_id(card_key, which), false);
    }
}

pub(crate) fn any_popover_open(ctx: &egui::Context, card_key: &str) -> bool {
    ["kind", "release", "asset"]
        .into_iter()
        .any(|which| is_menu_open(ctx, menu_id(card_key, which)))
}

fn is_menu_open(ctx: &egui::Context, id: egui::Id) -> bool {
    ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false)
}

fn set_menu_open(ctx: &egui::Context, id: egui::Id, open: bool) {
    ctx.data_mut(|d| d.insert_temp(id, open));
}

fn popup_frame(palette: ThemePalette) -> egui::Frame {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(6))
}

fn section_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(6.0);
}

fn field_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(6.0);
}

fn chip_button(ui: &mut egui::Ui, palette: ThemePalette, label: &str, selected: bool) -> bool {
    redesign_btn(
        ui,
        palette,
        label,
        BtnOpts {
            small: true,
            primary: selected,
            ..Default::default()
        },
    )
    .clicked()
        && !selected
}

fn follow_pill(ui: &mut egui::Ui, palette: ThemePalette, label: &str, selected: bool) -> bool {
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()));
    let color = if selected {
        redesign_pill_text(palette)
    } else {
        redesign_text_primary(palette)
    };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), color);
    let size = egui::vec2(
        11.0_f32.mul_add(2.0, galley.size().x),
        5.0_f32.mul_add(2.0, galley.size().y),
    );
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let fill = if selected {
            redesign_accent(palette)
        } else {
            redesign_shell_bg(palette)
        };
        painter.rect_filled(rect, egui::CornerRadius::same(12), fill);
        if !selected {
            painter.rect_stroke(
                rect,
                egui::CornerRadius::same(12),
                egui::Stroke::new(1.5_f32, redesign_border_strong(palette)),
                egui::StrokeKind::Inside,
            );
        }
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            color,
        );
    }
    response.clicked()
}

fn menu_check_item(ui: &mut egui::Ui, palette: ThemePalette, label: &str, selected: bool) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        if selected {
            versions_icons::paint_check(
                painter,
                egui::pos2(rect.left() + 14.0, rect.center().y),
                redesign_accent(palette),
            );
        }
        let text_font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()));
        let text = versions_icons::elide(painter, label, &text_font, rect.width() - 36.0);
        painter.text(
            egui::pos2(rect.left() + 28.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            text_font,
            redesign_text_primary(palette),
        );
    }
    response.clicked()
}

fn dropdown_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    width: f32,
    label: &str,
    open: bool,
    monospace: bool,
) -> (egui::Rect, egui::Response) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, FIELD_H), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let border = if open || response.hovered() {
            redesign_accent(palette)
        } else {
            redesign_border_strong(palette)
        };
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            redesign_input_bg(palette),
        );
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border),
            egui::StrokeKind::Inside,
        );
        let caret_center = egui::pos2(rect.right() - 14.0, rect.center().y);
        versions_icons::paint_caret_down(painter, caret_center, redesign_text_faint(palette));
        let font = if monospace {
            egui::FontId::new(13.0, egui::FontFamily::Monospace)
        } else {
            egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()))
        };
        let max_w = (caret_center.x - 8.0 - (rect.left() + 10.0)).max(10.0);
        let text = versions_icons::elide(painter, label, &font, max_w);
        painter.text(
            egui::pos2(rect.left() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            font,
            redesign_text_primary(palette),
        );
    }
    (rect, response)
}

fn text_field(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    value: &mut String,
    placeholder: &str,
    monospace: bool,
    width: f32,
) -> egui::Response {
    let font = if monospace {
        egui::FontId::new(13.0, egui::FontFamily::Name("firacode_nerd".into()))
    } else {
        egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()))
    };
    redesign_text_input(
        ui,
        palette,
        InputOpts {
            edit: egui::TextEdit::singleline(value)
                .hint_text(placeholder)
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette))
                .margin(FIELD_MARGIN)
                .font(font)
                .vertical_align(egui::Align::Center),
            margin: FIELD_MARGIN,
            size: egui::vec2(width, FIELD_H),
            border: None,
        },
    )
}

fn labeled_text_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    value: &mut String,
    placeholder: &str,
) {
    field_label(ui, palette, label);
    let width = ui.available_width();
    text_field(ui, palette, value, placeholder, true, width);
}

fn labeled_text_field_inline(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    value: &mut String,
    placeholder: &str,
    width: f32,
) {
    ui.vertical(|ui| {
        ui.set_width(width);
        field_label(ui, palette, label);
        text_field(ui, palette, value, placeholder, true, width);
    });
}

fn render_save_to(ui: &mut egui::Ui, palette: ThemePalette, form: &mut SourceForm) {
    section_label(ui, palette, "Save to");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if chip_button(
            ui,
            palette,
            "This modlist",
            form.save_to == ModSourceEditDestination::ThisModlist,
        ) {
            form.save_to = ModSourceEditDestination::ThisModlist;
        }
        if chip_button(
            ui,
            palette,
            "My default",
            form.save_to == ModSourceEditDestination::GlobalDefault,
        ) {
            form.save_to = ModSourceEditDestination::GlobalDefault;
        }
    });
}

const fn source_kind_label(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::GitHub => "GitHub",
        SourceKind::WeaselMods => "Weasel Mods",
        SourceKind::MorpheusMart => "Morpheus Mart",
        SourceKind::DirectLink => "Direct link",
    }
}

const SOURCE_KINDS: [SourceKind; 4] = [
    SourceKind::GitHub,
    SourceKind::WeaselMods,
    SourceKind::MorpheusMart,
    SourceKind::DirectLink,
];

fn render_source_kind_dropdown(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
) {
    let id = menu_id(&form.card_key, "kind");
    let open = is_menu_open(ctx, id);
    let (rect, response) = dropdown_button(
        ui,
        palette,
        SOURCE_KIND_W,
        source_kind_label(form.kind),
        open,
        false,
    );
    if response.clicked() {
        set_menu_open(ctx, id, !open);
    }
    if !open {
        return;
    }
    let pos = egui::pos2(rect.left(), rect.bottom() + 2.0);
    let area = egui::Area::new(id.with("popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            popup_frame(palette).show(ui, |ui| {
                ui.set_width(SOURCE_KIND_W);
                for kind in SOURCE_KINDS {
                    if menu_check_item(ui, palette, source_kind_label(kind), form.kind == kind) {
                        form.kind = kind;
                        set_menu_open(ctx, id, false);
                    }
                }
            });
        });
    let should_close = ctx.input(|i| i.key_pressed(egui::Key::Escape))
        || (response.clicked_elsewhere() && area.response.clicked_elsewhere());
    if should_close {
        set_menu_open(ctx, id, false);
    }
}

fn render_source_row(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.vertical(|ui| {
            ui.set_width(SOURCE_KIND_W);
            field_label(ui, palette, "Source");
            render_source_kind_dropdown(ctx, ui, palette, form);
        });
        let width = (ui.available_width() - 12.0).max(120.0);
        ui.vertical(|ui| {
            ui.set_width(width);
            match form.kind {
                SourceKind::GitHub => {
                    field_label(ui, palette, "Repository");
                    if text_field(ui, palette, &mut form.repo, "owner/repository", true, width)
                        .changed()
                    {
                        form.error = None;
                    }
                }
                SourceKind::WeaselMods | SourceKind::MorpheusMart | SourceKind::DirectLink => {
                    field_label(ui, palette, "Link");
                    if text_field(ui, palette, &mut form.link, "https://\u{2026}", true, width)
                        .changed()
                    {
                        form.error = None;
                    }
                }
            }
        });
    });
}

const FOLLOWS: [(Follow, &str); 5] = [
    (Follow::Commit, "Commit"),
    (Follow::Tag, "Tag"),
    (Follow::Branch, "Branch"),
    (Follow::LatestCode, "Latest code"),
    (Follow::Release, "Release"),
];

fn render_follow_pills(ui: &mut egui::Ui, palette: ThemePalette, form: &mut SourceForm) {
    field_label(ui, palette, "Follow");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (follow, label) in FOLLOWS {
            if follow_pill(ui, palette, label, form.follow == follow) {
                form.follow = follow;
                form.notice = None;
            }
        }
    });
}

fn render_follow_body(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    env: &FormEnv<'_>,
    outcome: &mut FormOutcome,
) {
    match form.follow {
        Follow::Commit => labeled_text_row(ui, palette, "Commit", &mut form.commit, "commit hash"),
        Follow::Tag => labeled_text_row(ui, palette, "Tag", &mut form.tag, "tag name"),
        Follow::Branch => labeled_text_row(ui, palette, "Branch", &mut form.branch, "branch name"),
        Follow::LatestCode => render_latest_code_readonly(ui, palette),
        Follow::Release => render_release_body(ctx, ui, palette, form, env, outcome),
    }
}

fn render_latest_code_readonly(ui: &mut egui::Ui, palette: ThemePalette) {
    field_label(ui, palette, "Default branch");
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), FIELD_H),
        egui::Sense::hover(),
    );
    if ui.is_rect_visible(rect) {
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            egui::pos2(rect.left() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "The repository's default branch",
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
            redesign_text_muted(palette),
        );
    }
}

fn release_control_label(form: &SourceForm) -> String {
    if form.release.trim().is_empty() {
        if form.allow_pre {
            "Newest (pre-releases included)".to_string()
        } else {
            "Newest".to_string()
        }
    } else {
        form.release.clone()
    }
}

fn maybe_request_release_list(
    form: &SourceForm,
    env: &FormEnv<'_>,
    outcome: &mut FormOutcome,
    retry_on_reopen: bool,
) {
    if !env.logged_in {
        return;
    }
    let repo = form.repo.trim();
    if repo.is_empty() {
        return;
    }
    let same_repo = env.release_list.repo == repo;
    let needs_request = !same_repo
        || matches!(env.release_list.status, ReleaseListStatus::Idle)
        || (retry_on_reopen && matches!(env.release_list.status, ReleaseListStatus::Failed(_)));
    if needs_request {
        outcome.request_release_list = Some(repo.to_string());
    }
}

fn render_release_body(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    env: &FormEnv<'_>,
    outcome: &mut FormOutcome,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        let toggle_w = 190.0;
        let width = (ui.available_width() - toggle_w).max(160.0);
        ui.vertical(|ui| {
            ui.set_width(width);
            field_label(ui, palette, "Release");
            if env.logged_in {
                render_release_control(ctx, ui, palette, form, env, outcome, width);
            } else {
                text_field(ui, palette, &mut form.release, "e.g. v35.17", true, width);
            }
        });
        ui.vertical(|ui| {
            ui.add_space(TOGGLE_ROW_OFFSET);
            render_toggle_track(ui, palette, &mut form.allow_pre, "Allow pre-releases");
        });
    });
    ui.add_space(ROW_GAP);
    if form.release.trim().is_empty() {
        render_packages_row(ui, palette, form);
    } else {
        render_asset_control(ctx, ui, palette, form, env);
    }
}

fn render_release_control(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    env: &FormEnv<'_>,
    outcome: &mut FormOutcome,
    width: f32,
) {
    let id = menu_id(&form.card_key, "release");
    let open = is_menu_open(ctx, id);
    let label = release_control_label(form);
    let monospace = !form.release.trim().is_empty();
    let (rect, response) = dropdown_button(ui, palette, width, &label, open, monospace);
    if response.clicked() {
        set_menu_open(ctx, id, !open);
    }
    let just_reopened = response.clicked() && !open;
    maybe_request_release_list(form, env, outcome, just_reopened);
    if !open {
        return;
    }
    let pos = egui::pos2(rect.left(), rect.bottom() + 2.0);
    let popup_w = width.max(280.0);
    let area = egui::Area::new(id.with("popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            popup_frame(palette)
                .show(ui, |ui| {
                    ui.set_width(popup_w);
                    render_release_list_body(ui, palette, form, env)
                })
                .inner
        });
    let selected = area.inner;
    let should_close = selected
        || ctx.input(|i| i.key_pressed(egui::Key::Escape))
        || (response.clicked_elsewhere() && area.response.clicked_elsewhere());
    if should_close {
        set_menu_open(ctx, id, false);
    }
}

fn render_release_search_box(ui: &mut egui::Ui, palette: ThemePalette, query: &mut String) {
    let width = ui.available_width();
    redesign_text_input(
        ui,
        palette,
        InputOpts {
            edit: egui::TextEdit::singleline(query)
                .hint_text("Search releases")
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette))
                .margin(egui::Margin::symmetric(8, 4))
                .font(egui::FontId::new(
                    13.0,
                    egui::FontFamily::Name("poppins_light".into()),
                )),
            margin: egui::Margin::symmetric(8, 4),
            size: egui::vec2(width, 30.0),
            border: None,
        },
    );
}

fn release_status_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.add_space(4.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        )
        .truncate(),
    );
    ui.add_space(4.0);
}

fn render_release_list_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    env: &FormEnv<'_>,
) -> bool {
    render_release_search_box(ui, palette, &mut form.release_query);
    ui.add_space(4.0);
    let mut selected = false;
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .show(ui, |ui| {
            if menu_check_item(ui, palette, "Newest", form.release.trim().is_empty()) {
                form.release.clear();
                selected = true;
            }
            match &env.release_list.status {
                ReleaseListStatus::Loading => {
                    release_status_label(ui, palette, "Loading releases\u{2026}");
                }
                ReleaseListStatus::Failed(message) => {
                    release_status_label(
                        ui,
                        palette,
                        &format!("Could not load releases \u{b7} {message}"),
                    );
                }
                ReleaseListStatus::Ready(releases) if env.release_list.repo == form.repo.trim() => {
                    let query = form.release_query.trim().to_ascii_lowercase();
                    for release in releases
                        .iter()
                        .filter(|release| form.allow_pre || !release.prerelease)
                        .filter(|release| {
                            query.is_empty() || release.tag.to_ascii_lowercase().contains(&query)
                        })
                    {
                        if render_release_row(ui, palette, release, form.release == release.tag) {
                            form.release.clone_from(&release.tag);
                            form.asset = AssetPick::SourceZip;
                            selected = true;
                        }
                    }
                }
                ReleaseListStatus::Ready(_) | ReleaseListStatus::Idle => {}
            }
        });
    selected
}

fn render_release_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    release: &CachedRelease,
    selected: bool,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        let mut x = rect.left() + 8.0;
        if selected {
            versions_icons::paint_check(
                painter,
                egui::pos2(x + 6.0, rect.center().y),
                redesign_accent(palette),
            );
        }
        x += 20.0;
        let tag_x = x;
        let tag_font = egui::FontId::new(13.0, egui::FontFamily::Monospace);
        let pre_font = egui::FontId::new(11.0, egui::FontFamily::Name("poppins_light".into()));
        let date = release
            .published_at
            .split('T')
            .next()
            .unwrap_or(&release.published_at);
        let date_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()));
        let date_w = painter
            .layout_no_wrap(
                date.to_string(),
                date_font.clone(),
                redesign_text_faint(palette),
            )
            .size()
            .x;
        let badge_reserve = if release.prerelease {
            let badge_w = painter
                .layout_no_wrap(
                    "pre-release".to_string(),
                    pre_font.clone(),
                    redesign_text_faint(palette),
                )
                .size()
                .x;
            badge_w + 8.0
        } else {
            0.0
        };
        let available = (rect.right() - 16.0 - date_w - badge_reserve - tag_x).max(0.0);
        let tag_text = versions_icons::elide(painter, &release.tag, &tag_font, available);
        let tag_w = painter
            .layout_no_wrap(
                tag_text.clone(),
                tag_font.clone(),
                redesign_text_primary(palette),
            )
            .size()
            .x;
        painter.text(
            egui::pos2(tag_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &tag_text,
            tag_font,
            redesign_text_primary(palette),
        );
        if release.prerelease {
            painter.text(
                egui::pos2(tag_x + tag_w + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                "pre-release",
                pre_font,
                redesign_text_faint(palette),
            );
        }
        painter.text(
            egui::pos2(rect.right() - 8.0 - date_w, rect.center().y),
            egui::Align2::LEFT_CENTER,
            date,
            date_font,
            redesign_text_faint(palette),
        );
    }
    response.clicked()
}

fn render_toggle_track(ui: &mut egui::Ui, palette: ThemePalette, on: &mut bool, label: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 18.0), egui::Sense::click());
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let radius = egui::CornerRadius::same(9);
            let fill = if *on {
                redesign_accent(palette)
            } else {
                redesign_chrome_bg(palette)
            };
            painter.rect_filled(rect, radius, fill);
            painter.rect_stroke(
                rect,
                radius,
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                egui::StrokeKind::Inside,
            );
            let knob_x = if *on {
                rect.right() - 9.0
            } else {
                rect.left() + 9.0
            };
            painter.circle_filled(
                egui::pos2(knob_x, rect.center().y),
                7.0,
                redesign_text_primary(palette),
            );
        }
        if response.clicked() {
            *on = !*on;
        }
        ui.add(
            egui::Label::new(
                egui::RichText::new(label)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_primary(palette)),
            )
            .truncate(),
        );
    });
}

fn render_packages_row(ui: &mut egui::Ui, palette: ThemePalette, form: &mut SourceForm) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(
            egui::RichText::new("Packages")
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.label(
            egui::RichText::new("\u{b7} first match wins")
                .size(11.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_faint(palette)),
        );
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let width = ((ui.available_width() - 24.0) / 3.0).max(80.0);
        labeled_text_field_inline(ui, palette, "Windows", &mut form.pkg_windows, "", width);
        labeled_text_field_inline(ui, palette, "Linux", &mut form.pkg_linux, "", width);
        labeled_text_field_inline(ui, palette, "macOS", &mut form.pkg_macos, "", width);
    });
}

fn asset_label(asset: &AssetPick) -> String {
    match asset {
        AssetPick::SourceZip => "Source code (zip)".to_string(),
        AssetPick::BestForOs => "Best for this OS".to_string(),
        AssetPick::Named(name) => name.clone(),
    }
}

fn current_release_assets<'a>(form: &SourceForm, env: &FormEnv<'a>) -> &'a [CachedAsset] {
    if let ReleaseListStatus::Ready(releases) = &env.release_list.status
        && env.release_list.repo == form.repo.trim()
        && let Some(release) = releases.iter().find(|release| release.tag == form.release)
    {
        return &release.assets;
    }
    &[]
}

fn render_asset_control(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    env: &FormEnv<'_>,
) {
    field_label(ui, palette, "Asset");
    if env.logged_in {
        render_asset_dropdown(ctx, ui, palette, form, env);
    } else {
        render_asset_logged_out(ui, palette, form);
    }
}

fn render_asset_dropdown(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    env: &FormEnv<'_>,
) {
    let id = menu_id(&form.card_key, "asset");
    let open = is_menu_open(ctx, id);
    let label = asset_label(&form.asset);
    let width = ui.available_width();
    let monospace = !matches!(form.asset, AssetPick::SourceZip | AssetPick::BestForOs);
    let (rect, response) = dropdown_button(ui, palette, width, &label, open, monospace);
    if response.clicked() {
        set_menu_open(ctx, id, !open);
    }
    if !open {
        return;
    }
    let assets = current_release_assets(form, env).to_vec();
    let pos = egui::pos2(rect.left(), rect.bottom() + 2.0);
    let area = egui::Area::new(id.with("popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            popup_frame(palette).show(ui, |ui| {
                ui.set_width(width);
                if menu_check_item(
                    ui,
                    palette,
                    "Source code (zip)",
                    matches!(form.asset, AssetPick::SourceZip),
                ) {
                    form.asset = AssetPick::SourceZip;
                    set_menu_open(ctx, id, false);
                }
                if menu_check_item(
                    ui,
                    palette,
                    "Best for this OS",
                    matches!(form.asset, AssetPick::BestForOs),
                ) {
                    form.asset = AssetPick::BestForOs;
                    set_menu_open(ctx, id, false);
                }
                for asset in &assets {
                    let selected =
                        matches!(&form.asset, AssetPick::Named(name) if name == &asset.name);
                    if menu_check_item(ui, palette, &asset.name, selected) {
                        form.asset = AssetPick::Named(asset.name.clone());
                        set_menu_open(ctx, id, false);
                    }
                }
            });
        });
    let should_close = ctx.input(|i| i.key_pressed(egui::Key::Escape))
        || (response.clicked_elsewhere() && area.response.clicked_elsewhere());
    if should_close {
        set_menu_open(ctx, id, false);
    }
}

fn render_asset_logged_out(ui: &mut egui::Ui, palette: ThemePalette, form: &mut SourceForm) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if follow_pill(
            ui,
            palette,
            "Source code (zip)",
            matches!(form.asset, AssetPick::SourceZip),
        ) {
            form.asset = AssetPick::SourceZip;
        }
        if follow_pill(
            ui,
            palette,
            "Best for this OS",
            matches!(form.asset, AssetPick::BestForOs),
        ) {
            form.asset = AssetPick::BestForOs;
        }
    });
    ui.add_space(8.0);
    field_label(ui, palette, "File name");
    let width = ui.available_width();
    let mut name = if let AssetPick::Named(name) = &form.asset {
        name.clone()
    } else {
        String::new()
    };
    if text_field(ui, palette, &mut name, "exact file name", true, width).changed() {
        form.asset = if name.trim().is_empty() {
            AssetPick::SourceZip
        } else {
            AssetPick::Named(name)
        };
    }
}

fn render_note_row(ui: &mut egui::Ui, palette: ThemePalette) {
    field_label(ui, palette, "Note");
    let width = ui.available_width();
    ui.add_enabled_ui(false, |ui| {
        let mut text = String::new();
        text_field(
            ui,
            palette,
            &mut text,
            "notes arrive in a later build",
            false,
            width,
        );
    });
}

fn render_notice(ui: &mut egui::Ui, palette: ThemePalette, notice: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(notice)
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_warning(palette)),
        )
        .truncate(),
    );
}

fn advanced_header(ui: &mut egui::Ui, palette: ThemePalette, open: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.hline(
            rect.x_range(),
            rect.top(),
            egui::Stroke::new(1.0_f32, redesign_border_soft(palette)),
        );
        painter.hline(
            rect.x_range(),
            rect.bottom(),
            egui::Stroke::new(1.0_f32, redesign_border_soft(palette)),
        );
        let chevron_center = egui::pos2(rect.left() + 8.0, rect.center().y);
        if open {
            versions_icons::paint_caret_down(
                painter,
                chevron_center,
                redesign_text_primary(palette),
            );
        } else {
            versions_icons::paint_chevron_right(
                painter,
                chevron_center,
                redesign_text_primary(palette),
            );
        }
        painter.text(
            egui::pos2(rect.left() + 22.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Advanced",
            egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
            redesign_text_primary(palette),
        );
    }
    response
}

fn render_advanced(ui: &mut egui::Ui, palette: ThemePalette, form: &mut SourceForm) {
    let response = advanced_header(ui, palette, form.advanced_open);
    if response.clicked() {
        form.advanced_open = !form.advanced_open;
    }
    if !form.advanced_open {
        return;
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let width = ((ui.available_width() - 12.0) / 2.0).max(120.0);
        labeled_text_field_inline(
            ui,
            palette,
            "Other TP2 names",
            &mut form.aliases_text,
            "none",
            width,
        );
        labeled_text_field_inline(
            ui,
            palette,
            "Required subfolder",
            &mut form.subdir_require,
            "none",
            width,
        );
    });
}

fn render_will_fetch(ui: &mut egui::Ui, palette: ThemePalette, form: &SourceForm) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(
            egui::RichText::new("WILL FETCH")
                .size(11.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
        );
        let sentence = source_form::will_fetch(form);
        ui.add(
            egui::Label::new(
                egui::RichText::new(sentence)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            )
            .truncate(),
        );
    });
}
