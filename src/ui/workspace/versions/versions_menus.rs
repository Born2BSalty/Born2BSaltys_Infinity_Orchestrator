// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::mod_source_history;
use crate::app::state::VersionsSheet;
use crate::app::step2_action::{ModSourceEditDestination, Step2Action};
use crate::app::versions_view::{CardSourceOption, FetchOffer, SourceRowKind, VersionCard};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_hover_overlay, redesign_shell_bg, redesign_text_faint,
    redesign_text_primary,
};

use super::versions_card::fetch_label;
use super::versions_icons;
use super::versions_sheets::{self, NoteSeed};

const SOURCES_MENU_W: f32 = 620.0;
const SOURCES_MENU_CHROME_H: f32 = 110.0;
const KEBAB_MENU_W: f32 = 220.0;
const MENU_INSET: f32 = 8.0;
const ROW_H: f32 = 30.0;
const NOTE_COL_W: f32 = 18.0;
const WHO_COL_W: f32 = 160.0;
const RULE_COL_W: f32 = 208.0;

#[derive(Default)]
pub(crate) struct MenuOutcome {
    pub(crate) action: Option<Step2Action>,
    pub(crate) open_sheet: Option<VersionsSheet>,
    pub(crate) close: bool,
    pub(crate) note_seed: Option<NoteSeed>,
}

pub(crate) fn render_catalog_selector(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tp2: &str,
    sources: &[mod_downloads::ModDownloadSource],
    selected: &mod_downloads::ModDownloadSource,
    tiers: &mod_downloads::SourceTiers,
) -> Option<String> {
    ui.scope(|ui| {
        let visuals = ui.visuals_mut();
        for widget in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.bg_fill = redesign_shell_bg(palette);
            widget.weak_bg_fill = redesign_shell_bg(palette);
            widget.bg_stroke = egui::Stroke::new(1.0_f32, redesign_border_strong(palette));
            widget.corner_radius = egui::CornerRadius::same(3);
            widget.expansion = 0.0;
        }
        visuals.window_fill = redesign_shell_bg(palette);
        visuals.window_stroke = egui::Stroke::new(1.0_f32, redesign_border_strong(palette));
        visuals.menu_corner_radius = egui::CornerRadius::same(3);
        let repository = selected
            .github
            .as_deref()
            .map(str::trim)
            .filter(|repo| !repo.is_empty())
            .map_or_else(
                || {
                    let label = selected.source_label.trim();
                    if label.is_empty() {
                        selected.url.clone()
                    } else {
                        label.to_string()
                    }
                },
                |repo| format!("GitHub · {repo}"),
            );
        let rule_words = crate::app::versions_view::rule_words(selected);
        let rule = format!("Version: {rule_words}");
        let font = egui::FontId::new(14.0, egui::FontFamily::Name("poppins_light".into()));
        let spacing = ui.spacing();
        let width = (spacing.button_padding.x.mul_add(-2.0, ui.available_width())
            - spacing.icon_spacing
            - spacing.icon_width)
            .max(0.0);
        let shown_repository = versions_icons::elide(ui.painter(), &repository, &font, width);
        let shown_rule = versions_icons::elide(ui.painter(), &rule, &font, width);
        let truncated = shown_repository != repository || shown_rule != rule;
        let selected_text = ui.painter().layout_no_wrap(
            format!("{shown_repository}\n{shown_rule}"),
            font,
            redesign_text_primary(palette),
        );
        let response = catalog_source_button(ui, palette, tp2, selected_text);
        let picked = catalog_source_popup(ui, palette, tp2, sources, selected, tiers, &response);
        let popup_id = response.id.with("popup");
        if truncated && response.hovered() && !ui.memory(|memory| memory.is_popup_open(popup_id)) {
            egui::Area::new(response.id.with("catalog_source_caption"))
                .order(egui::Order::Tooltip)
                .fixed_pos(response.rect.left_bottom() + egui::vec2(0.0, 4.0))
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    popup_frame(palette).show(ui, |ui| {
                        ui.set_max_width(480.0);
                        ui.label(
                            egui::RichText::new(format!("{repository}\n{rule}"))
                                .size(14.0)
                                .color(redesign_text_primary(palette)),
                        );
                    });
                });
        }
        picked
    })
    .inner
}

fn catalog_source_popup(
    ui: &egui::Ui,
    palette: ThemePalette,
    tp2: &str,
    sources: &[mod_downloads::ModDownloadSource],
    selected: &mod_downloads::ModDownloadSource,
    tiers: &mod_downloads::SourceTiers,
    response: &egui::Response,
) -> Option<String> {
    let mut picked = None;
    let popup_id = response.id.with("popup");
    let context_id = egui::Id::new("catalog_source_popup_context");
    let frame = ui.ctx().cumulative_pass_nr();
    let previous = ui.ctx().data_mut(|data| {
        let previous = data.get_temp::<(egui::Id, u64)>(context_id);
        data.insert_temp(context_id, (popup_id, frame));
        previous
    });
    if let Some((previous, previous_frame)) = previous
        && (previous != popup_id || previous_frame.saturating_add(1) < frame)
        && ui.memory(|memory| memory.is_popup_open(previous))
    {
        ui.memory_mut(egui::Memory::close_popup);
    }
    if response.clicked() {
        ui.input_mut(|input| {
            input.consume_key(egui::Modifiers::NONE, egui::Key::Space);
            input.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
        });
        ui.memory_mut(|memory| memory.toggle_popup(popup_id));
        if ui.memory(|memory| memory.is_popup_open(popup_id)) {
            ui.memory_mut(|memory| memory.request_focus(catalog_source_row_id(selected)));
        }
    } else if ui.memory(|memory| memory.is_popup_open(popup_id)) {
        picked = catalog_source_keyboard(ui, sources, selected);
        if picked.is_some() {
            ui.memory_mut(egui::Memory::close_popup);
            response.request_focus();
        }
    }
    let (side, height, width) = catalog_source_popup_geometry(ui, popup_id, response, sources);
    egui::popup::popup_above_or_below_widget(
        ui,
        popup_id,
        response,
        side,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = 0.0;
            egui::ScrollArea::vertical()
                .id_salt(("catalog_source_rows", tp2))
                .max_height(height)
                .min_scrolled_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for source in sources {
                        if catalog_source_row(ui, palette, source, selected, tiers) {
                            picked = Some(source.source_id.clone());
                            ui.memory_mut(egui::Memory::close_popup);
                            response.request_focus();
                        }
                    }
                });
        },
    );
    picked
}

fn catalog_source_popup_geometry(
    ui: &egui::Ui,
    popup_id: egui::Id,
    response: &egui::Response,
    sources: &[mod_downloads::ModDownloadSource],
) -> (egui::AboveOrBelow, f32, f32) {
    let screen = ui.ctx().screen_rect().shrink(4.0);
    let row_height = ui
        .painter()
        .layout_no_wrap(
            "Source\nLocation\nVersion".to_string(),
            catalog_source_row_font(),
            egui::Color32::WHITE,
        )
        .size()
        .y
        + 16.0;
    let desired_height = sources
        .iter()
        .take(3)
        .fold(0.0, |height, _| height + row_height);
    let geometry = (response.rect, screen, desired_height);
    let previous_geometry = ui.ctx().data_mut(|data| {
        let previous = data.get_temp::<(egui::Rect, egui::Rect, f32)>(popup_id.with("geometry"));
        data.insert_temp(popup_id.with("geometry"), geometry);
        previous
    });
    if previous_geometry.is_some_and(|previous| previous != geometry)
        && ui.memory(|memory| memory.is_popup_open(popup_id))
    {
        ui.ctx()
            .request_discard("Catalog source popup anchor changed");
    }
    let frame_margin = egui::Frame::popup(ui.style()).total_margin().sum();
    let below = (screen.bottom() - response.rect.bottom()).max(0.0);
    let above = (response.rect.top() - screen.top()).max(0.0);
    let side = if below >= desired_height + frame_margin.y || below >= above {
        egui::AboveOrBelow::Below
    } else {
        egui::AboveOrBelow::Above
    };
    let height = (match side {
        egui::AboveOrBelow::Below => below,
        egui::AboveOrBelow::Above => above,
    } - frame_margin.y)
        .clamp(0.0, desired_height);
    let width = (response.rect.width().min(screen.width()) - frame_margin.x).max(0.0);
    (side, height, width)
}

fn catalog_source_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tp2: &str,
    caption: std::sync::Arc<egui::Galley>,
) -> egui::Response {
    let padding = ui.spacing().button_padding;
    let size = egui::vec2(
        ui.available_width().max(0.0),
        padding
            .y
            .mul_add(2.0, caption.size().y)
            .max(ui.spacing().interact_size.y),
    );
    let (_, rect) = ui.allocate_space(size);
    let response = ui.interact(
        rect,
        egui::Id::new(("catalog_source", tp2)),
        egui::Sense::click(),
    );
    let visuals = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        3.0,
        redesign_shell_bg(palette),
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        egui::pos2(
            rect.left() + padding.x,
            caption.size().y.mul_add(-0.5, rect.center().y),
        ),
        caption,
        redesign_text_primary(palette),
    );
    let center = egui::pos2(
        ui.spacing()
            .icon_width
            .mul_add(-0.5, rect.right() - padding.x),
        rect.center().y,
    );
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            center + egui::vec2(-4.0, -2.0),
            center + egui::vec2(4.0, -2.0),
            center + egui::vec2(0.0, 3.0),
        ],
        redesign_text_faint(palette),
        egui::Stroke::NONE,
    ));
    response
}

fn catalog_source_row_id(source: &mod_downloads::ModDownloadSource) -> egui::Id {
    egui::Id::new(("catalog_source_choice", &source.tp2, &source.source_id))
}

fn catalog_source_keyboard(
    ui: &egui::Ui,
    sources: &[mod_downloads::ModDownloadSource],
    selected: &mod_downloads::ModDownloadSource,
) -> Option<String> {
    if sources.is_empty() {
        return None;
    }
    let focused = ui.memory(egui::Memory::focused);
    let index = sources
        .iter()
        .position(|source| Some(catalog_source_row_id(source)) == focused)
        .or_else(|| {
            sources
                .iter()
                .position(|source| source.source_id == selected.source_id)
        })
        .unwrap_or(0);
    let movement = ui.input_mut(|input| {
        if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
            Some((index + 1).min(sources.len() - 1))
        } else if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
            Some(index.saturating_sub(1))
        } else if input.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab) {
            Some(index.checked_sub(1).unwrap_or(sources.len() - 1))
        } else if input.consume_key(egui::Modifiers::NONE, egui::Key::Tab) {
            Some((index + 1) % sources.len())
        } else {
            None
        }
    });
    if let Some(index) = movement {
        ui.memory_mut(|memory| memory.request_focus(catalog_source_row_id(&sources[index])));
    } else if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
        return Some(sources[index].source_id.clone());
    }
    None
}

fn catalog_source_row_font() -> egui::FontId {
    egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()))
}

fn catalog_source_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    source: &mod_downloads::ModDownloadSource,
    selected: &mod_downloads::ModDownloadSource,
    tiers: &mod_downloads::SourceTiers,
) -> bool {
    let origin =
        crate::app::versions_view::layer_name(tiers.tier_of(&source.tp2, &source.source_id));
    let heading = format!(
        "{origin} \u{b7} {}",
        if source.source_label.trim().is_empty() {
            &source.source_id
        } else {
            &source.source_label
        }
    );
    let location = source.github.as_deref().map_or_else(
        || source.url.clone(),
        |repo| format!("GitHub \u{b7} {repo}"),
    );
    let rule = format!("Version: {}", crate::app::versions_view::rule_words(source));
    let lines = [heading, location, rule];
    let font = catalog_source_row_font();
    let text_width = (ui.available_width() - 36.0).max(0.0);
    let shown = lines
        .iter()
        .map(|line| versions_icons::elide(ui.painter(), line, &font, text_width))
        .collect::<Vec<_>>()
        .join("\n");
    let galley = ui
        .painter()
        .layout_no_wrap(shown, font, redesign_text_primary(palette));
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), galley.size().y + 16.0));
    let response = ui.interact(rect, catalog_source_row_id(source), egui::Sense::click());
    if response.gained_focus() {
        response.scroll_to_me(Some(egui::Align::Center));
    }
    let current = source.source_id == selected.source_id;
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    if current {
        painter.rect_filled(rect, 3.0, redesign_accent(palette).gamma_multiply(0.12));
    }
    if response.hovered() || response.has_focus() {
        painter.rect_filled(rect, 3.0, redesign_hover_overlay(palette));
    }
    if current {
        versions_icons::paint_check(
            &painter,
            egui::pos2(rect.left() + 14.0, rect.center().y),
            redesign_accent(palette),
        );
    }
    painter.galley(
        egui::pos2(rect.left() + 28.0, rect.top() + 8.0),
        galley,
        redesign_text_primary(palette),
    );
    painter.hline(
        rect.x_range(),
        rect.bottom(),
        egui::Stroke::new(1.0_f32, redesign_border_strong(palette).gamma_multiply(0.4)),
    );
    response.on_hover_text(lines.join("\n")).clicked()
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

fn find_forks_action(card: &VersionCard) -> Step2Action {
    Step2Action::DiscoverModDownloadForks {
        tp2: card.tp2.clone(),
        label: card.name.clone(),
        repo: card.repo.clone().unwrap_or_default(),
    }
}

fn default_save_to() -> ModSourceEditDestination {
    if mod_downloads::active_modlist_downloads_path().is_some() {
        ModSourceEditDestination::ThisModlist
    } else {
        ModSourceEditDestination::GlobalDefault
    }
}

fn display_rule_words(option: &CardSourceOption) -> String {
    match option.version.as_deref() {
        Some(version) if !version.trim().is_empty() && !option.rule_words.contains(version) => {
            format!("{} \u{b7} {version}", option.rule_words)
        }
        _ => option.rule_words.clone(),
    }
}

fn format_date(iso: &str) -> String {
    chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d").map_or_else(
        |_| iso.to_string(),
        |date| date.format("%-d %b %Y").to_string(),
    )
}

fn who_label(option: &CardSourceOption) -> String {
    match option.kind {
        SourceRowKind::Bookmark => format!("Bookmark \u{b7} {}", format_date(&option.who)),
        SourceRowKind::Past => format!("saved {}", format_date(&option.who)),
        SourceRowKind::Fork => "fork".to_string(),
        SourceRowKind::Current | SourceRowKind::Layer | SourceRowKind::Pin => option.who.clone(),
    }
}

pub(crate) struct SourcesMenuEnv<'a> {
    pub(crate) card: &'a VersionCard,
    pub(crate) busy: bool,
    pub(crate) who: &'a str,
    pub(crate) bounds: egui::Rect,
}

fn sources_menu_id() -> egui::Id {
    egui::Id::new("versions_sources_menu")
}

fn sources_menu_pos(
    ctx: &egui::Context,
    palette: ThemePalette,
    anchor_rect: egui::Rect,
    bounds: egui::Rect,
) -> egui::Pos2 {
    let width = SOURCES_MENU_W + popup_frame(palette).total_margin().sum().x;
    let remembered = ctx.memory(|m| m.area_rect(sources_menu_id()));
    if remembered.is_none() {
        ctx.request_repaint();
    }
    let height = remembered.map_or(0.0, |rect| rect.height());
    let inner = bounds.shrink(MENU_INSET);
    let x = (anchor_rect.right() - width)
        .min(inner.right() - width)
        .max(inner.left());
    let y = (anchor_rect.bottom() + 4.0)
        .min(inner.bottom() - height)
        .max(inner.top());
    egui::pos2(x, y)
}

pub(crate) fn render_sources_menu(
    ctx: &egui::Context,
    palette: ThemePalette,
    anchor_rect: egui::Rect,
    anchor_response: &egui::Response,
    env: &SourcesMenuEnv<'_>,
) -> MenuOutcome {
    let mut outcome = MenuOutcome::default();
    let pos = sources_menu_pos(ctx, palette, anchor_rect, env.bounds);
    let response = egui::Area::new(sources_menu_id())
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            popup_frame(palette).show(ui, |ui| {
                ui.set_width(SOURCES_MENU_W);
                render_known_sources_body(ui, palette, env, &mut outcome);
            });
        });
    let should_close = anchor_response.clicked_elsewhere() && response.response.clicked_elsewhere();
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
        outcome.close = true;
    }
    outcome
}

fn apply_row_click(
    card: &VersionCard,
    option: &CardSourceOption,
    who: &str,
    outcome: &mut MenuOutcome,
) {
    match option.kind {
        SourceRowKind::Current => {}
        SourceRowKind::Layer => {
            outcome.action = Some(Step2Action::SetModDownloadSource {
                tp2: card.tp2.clone(),
                source_id: option.source_id.clone(),
            });
        }
        SourceRowKind::Pin
        | SourceRowKind::Bookmark
        | SourceRowKind::Past
        | SourceRowKind::Fork => {
            outcome.action = Some(Step2Action::UseKnownSource {
                tp2: card.tp2.clone(),
                card_key: card.tp2.clone(),
                block: option.block.clone().unwrap_or_default(),
                save_to: default_save_to(),
                who: who.to_string(),
            });
        }
    }
    outcome.close = true;
}

fn render_known_sources_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    env: &SourcesMenuEnv<'_>,
    outcome: &mut MenuOutcome,
) {
    let card = env.card;
    let busy = env.busy;
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.label(
        egui::RichText::new("KNOWN SOURCES")
            .size(11.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_faint(palette)),
    );
    ui.add_space(4.0);
    let rows_max_height =
        (env.bounds.height() - SOURCES_MENU_CHROME_H - MENU_INSET - MENU_INSET).max(64.0);
    egui::ScrollArea::vertical()
        .id_salt(("versions_sources_menu_rows", &card.tp2))
        .max_height(rows_max_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (index, option) in card.sources.iter().enumerate() {
                if render_source_row(ui, palette, index, option, busy) {
                    apply_row_click(card, option, env.who, outcome);
                }
            }
        });
    ui.add_space(4.0);
    ui.separator();
    ui.horizontal(|ui| {
        if redesign_btn(
            ui,
            palette,
            "Edit source\u{2026}",
            BtnOpts {
                small: true,
                disabled: busy,
                ..Default::default()
            },
        )
        .clicked()
            && !busy
        {
            outcome.open_sheet = Some(VersionsSheet::EditSource);
            outcome.close = true;
        }
        if card.repo.is_some()
            && redesign_btn(
                ui,
                palette,
                "Find forks\u{2026}",
                BtnOpts {
                    small: true,
                    disabled: busy,
                    ..Default::default()
                },
            )
            .clicked()
            && !busy
        {
            outcome.action = Some(find_forks_action(card));
            outcome.open_sheet = Some(VersionsSheet::Forks);
            outcome.close = true;
        }
    });
}

struct RowSlot {
    rect: egui::Rect,
    x: f32,
    faint: egui::Color32,
}

fn render_row_who(
    ui: &egui::Ui,
    painter: &egui::Painter,
    palette: ThemePalette,
    index: usize,
    option: &CardSourceOption,
    slot: &RowSlot,
) {
    let RowSlot { rect, x, faint } = *slot;
    let who_rect = egui::Rect::from_min_size(
        egui::pos2(x, rect.top()),
        egui::vec2(WHO_COL_W, rect.height()),
    );
    let who_font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()));
    let who_text = versions_icons::elide(painter, &who_label(option), &who_font, WHO_COL_W - 6.0);
    painter.text(
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        who_text,
        who_font,
        faint,
    );
    if option.who_hover.len() > 1 {
        let names = option.who_hover.clone();
        ui.interact(
            who_rect,
            ui.id().with(("versions_who_hover", index)),
            egui::Sense::hover(),
        )
        .on_hover_ui(|ui| {
            ui.label(
                egui::RichText::new("Pinned in")
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_faint(palette)),
            );
            for name in &names {
                ui.label(egui::RichText::new(name).size(12.0));
            }
        });
    }
}

fn render_row_note(
    ui: &egui::Ui,
    painter: &egui::Painter,
    palette: ThemePalette,
    index: usize,
    option: &CardSourceOption,
    note_rect: egui::Rect,
    faint: egui::Color32,
) {
    let Some((text, who)) = option.note.as_ref() else {
        return;
    };
    versions_icons::paint_note(painter, note_rect.center(), faint);
    let text = text.clone();
    let who = who.clone();
    ui.interact(
        note_rect,
        ui.id().with(("versions_note_hover", index)),
        egui::Sense::hover(),
    )
    .on_hover_ui(|ui| {
        ui.label(egui::RichText::new(&text).size(12.0));
        ui.label(
            egui::RichText::new(&who)
                .size(11.0)
                .color(redesign_text_faint(palette)),
        );
    });
}

fn render_source_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    index: usize,
    option: &CardSourceOption,
    disabled: bool,
) -> bool {
    let sense = if disabled {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter().clone();
        if response.hovered() {
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        let mut x = rect.left() + 8.0;
        let faint = if disabled {
            redesign_text_faint(palette).gamma_multiply(0.6)
        } else {
            redesign_text_faint(palette)
        };
        if option.current {
            let check_color = if disabled {
                redesign_accent(palette).gamma_multiply(0.5)
            } else {
                redesign_accent(palette)
            };
            versions_icons::paint_check(
                &painter,
                egui::pos2(x + 6.0, rect.center().y),
                check_color,
            );
        }
        x += 18.0;
        render_row_who(
            ui,
            &painter,
            palette,
            index,
            option,
            &RowSlot { rect, x, faint },
        );
        x += WHO_COL_W;
        let rule_font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()));
        let rule_text = versions_icons::elide(
            &painter,
            &display_rule_words(option),
            &rule_font,
            RULE_COL_W - 10.0,
        );
        let primary = if disabled {
            redesign_text_primary(palette).gamma_multiply(0.6)
        } else {
            redesign_text_primary(palette)
        };
        painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            rule_text,
            rule_font,
            primary,
        );
        x += RULE_COL_W;
        let loc_font = egui::FontId::new(12.0, egui::FontFamily::Monospace);
        let note_left = rect.right() - 8.0 - NOTE_COL_W;
        let max_w = (note_left - 6.0 - x).max(20.0);
        let loc_text = versions_icons::elide(&painter, &option.location, &loc_font, max_w);
        painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            loc_text,
            loc_font,
            faint,
        );
        let note_rect = egui::Rect::from_min_size(
            egui::pos2(note_left, rect.top()),
            egui::vec2(NOTE_COL_W, rect.height()),
        );
        render_row_note(ui, &painter, palette, index, option, note_rect, faint);
    }
    !disabled && response.clicked()
}

pub(crate) struct KebabEnv<'a> {
    pub(crate) card: &'a VersionCard,
    pub(crate) tiers: &'a mod_downloads::SourceTiers,
    pub(crate) bookmark_label: Option<&'a str>,
}

fn render_fetch_kebab_item(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    card: &VersionCard,
    busy: bool,
    outcome: &mut MenuOutcome,
) -> bool {
    if card.offer == FetchOffer::None {
        return false;
    }
    let label = fetch_label(card);
    if menu_item(ui, palette, &label, busy) {
        outcome.action = Some(Step2Action::DownloadUpdateFor {
            tp2: card.tp2.clone(),
        });
        outcome.close = true;
    }
    true
}

fn render_bookmark_kebab_item(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    card: &VersionCard,
    label: Option<&str>,
    busy: bool,
    outcome: &mut MenuOutcome,
) -> bool {
    let Some(label) = label else {
        return false;
    };
    if menu_item(ui, palette, label, busy) {
        outcome.action = Some(Step2Action::BookmarkOnDisk {
            tp2: card.tp2.clone(),
            card_key: card.tp2.clone(),
        });
        outcome.close = true;
    }
    true
}

fn current_option(card: &VersionCard) -> Option<&CardSourceOption> {
    card.sources
        .iter()
        .find(|option| option.kind == SourceRowKind::Current)
}

fn render_note_kebab_item(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    card: &VersionCard,
    tiers: &mod_downloads::SourceTiers,
    busy: bool,
    outcome: &mut MenuOutcome,
) {
    if card.source_id.is_none() {
        return;
    }
    let current_note = current_option(card).and_then(|option| option.note.clone());
    let note_label = if current_note.is_some() {
        "Edit note\u{2026}"
    } else {
        "Add a note\u{2026}"
    };
    if !menu_item(ui, palette, note_label, busy) {
        return;
    }
    let signature = versions_sheets::resolve_current_source(tiers, card)
        .map(|source| mod_source_history::rule_signature(&source))
        .unwrap_or_default();
    let (note_text, _) = current_note.unwrap_or_default();
    let location = current_option(card)
        .map(|option| option.location.clone())
        .unwrap_or_default();
    let rule_words = current_option(card)
        .map(|option| option.rule_words.clone())
        .unwrap_or_default();
    outcome.note_seed = Some(NoteSeed {
        mod_name: card.name.clone(),
        rule_words,
        location,
        note_text,
        signature,
    });
    outcome.open_sheet = Some(VersionsSheet::Note);
    outcome.close = true;
}

fn render_kebab_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    env: &KebabEnv<'_>,
    busy: bool,
    outcome: &mut MenuOutcome,
) {
    ui.set_width(KEBAB_MENU_W);
    ui.spacing_mut().item_spacing.y = 0.0;
    let fetch_shown = render_fetch_kebab_item(ui, palette, env.card, busy, outcome);
    let bookmark_shown =
        render_bookmark_kebab_item(ui, palette, env.card, env.bookmark_label, busy, outcome);
    if fetch_shown || bookmark_shown {
        ui.separator();
    }
    let edit_label = if env.card.source_id.is_some() {
        "Edit source\u{2026}"
    } else {
        "Add source\u{2026}"
    };
    if menu_item(ui, palette, edit_label, busy) {
        outcome.open_sheet = Some(VersionsSheet::EditSource);
        outcome.close = true;
    }
    if env.card.repo.is_some() && menu_item(ui, palette, "Find forks\u{2026}", busy) {
        outcome.action = Some(find_forks_action(env.card));
        outcome.open_sheet = Some(VersionsSheet::Forks);
        outcome.close = true;
    }
    render_note_kebab_item(ui, palette, env.card, env.tiers, busy, outcome);
}

pub(crate) fn render_kebab_menu(
    ctx: &egui::Context,
    palette: ThemePalette,
    anchor_rect: egui::Rect,
    anchor_response: &egui::Response,
    env: &KebabEnv<'_>,
    busy: bool,
) -> MenuOutcome {
    let mut outcome = MenuOutcome::default();
    let pos = egui::pos2(
        anchor_rect.right() - KEBAB_MENU_W,
        anchor_rect.bottom() + 4.0,
    );
    let response = egui::Area::new(egui::Id::new("versions_kebab_menu"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            popup_frame(palette).show(ui, |ui| {
                render_kebab_body(ui, palette, env, busy, &mut outcome);
            });
        });
    let should_close = anchor_response.clicked_elsewhere() && response.response.clicked_elsewhere();
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
        outcome.close = true;
    }
    outcome
}

fn menu_item(ui: &mut egui::Ui, palette: ThemePalette, label: &str, disabled: bool) -> bool {
    let font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into()));
    let text_color = if disabled {
        redesign_text_primary(palette).gamma_multiply(0.5)
    } else {
        redesign_text_primary(palette)
    };
    let pad_x = 10.0_f32;
    let pad_y = 6.0;
    let row_width = ui.available_width();
    let shown = versions_icons::elide(
        ui.painter(),
        label,
        &font,
        pad_x.mul_add(-2.0, row_width).max(0.0),
    );
    let galley = ui
        .painter()
        .layout_no_wrap(shown.clone(), font.clone(), text_color);
    let row_height = f32::mul_add(pad_y, 2.0, galley.size().y);
    let sense = if disabled {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(row_width, row_height), sense);
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + pad_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            shown,
            font,
            text_color,
        );
    }
    !disabled && response.clicked()
}
