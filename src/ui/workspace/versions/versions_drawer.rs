// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::state::{
    VersionsChip, VersionsDrawerUi, VersionsMenu, VersionsSheet, exact_log_ready_to_install,
    update_pipeline_busy,
};
use crate::app::step2_action::Step2Action;
use crate::app::versions_view::{self, VersionCard, VersionsView};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::drawer::{self, DrawerSpec, DrawerWidth};
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, clipboard, redesign_btn, redesign_text_input,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_accent, redesign_border_strong,
    redesign_input_bg, redesign_pill_text, redesign_shell_bg, redesign_text_faint,
    redesign_text_muted, redesign_text_primary,
};
use crate::ui::step2::update_check_popup_report_step2::build_popup_report;

use super::{versions_card, versions_form, versions_icons, versions_menus, versions_sheets};

type FileStamp = (u64, u128);
type TiersCacheKey = (Option<PathBuf>, FileStamp, FileStamp);

#[derive(Clone)]
struct TiersCache {
    key: TiersCacheKey,
    tiers: Arc<mod_downloads::SourceTiers>,
}

fn file_stamp(path: &std::path::Path) -> FileStamp {
    let Ok(meta) = std::fs::metadata(path) else {
        return (0, 0);
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    (meta.len(), modified)
}

fn cached_source_tiers(ctx: &egui::Context) -> Arc<mod_downloads::SourceTiers> {
    let modlist_path = mod_downloads::active_modlist_downloads_path();
    let modlist_stamp = modlist_path.as_deref().map_or((0, 0), file_stamp);
    let user_stamp = file_stamp(&mod_downloads::mod_downloads_user_path());
    let key: TiersCacheKey = (modlist_path.clone(), modlist_stamp, user_stamp);
    let id = egui::Id::new("versions_drawer_source_tiers_cache");
    if let Some(cache) = ctx.data(|d| d.get_temp::<TiersCache>(id))
        && cache.key == key
    {
        return cache.tiers;
    }
    let modlist_text = modlist_path
        .map(|path| std::fs::read_to_string(path).unwrap_or_default())
        .unwrap_or_default();
    let tiers = Arc::new(mod_downloads::load_source_tiers(&modlist_text));
    ctx.data_mut(|d| {
        d.insert_temp(
            id,
            TiersCache {
                key,
                tiers: tiers.clone(),
            },
        );
    });
    tiers
}

struct HeaderCtx {
    report_text: String,
    busy: bool,
    fetching: bool,
    suppress_escape: bool,
    check_sources_label: &'static str,
    retry_count: usize,
}

fn build_header_ctx(orchestrator: &OrchestratorApp) -> HeaderCtx {
    let exact_log = orchestrator
        .wizard_state
        .step1
        .installs_exactly_from_weidu_logs();
    let good_to_go = exact_log_ready_to_install(&orchestrator.wizard_state);
    let report_text = build_popup_report(&orchestrator.wizard_state, exact_log, good_to_go);
    let busy = update_pipeline_busy(&orchestrator.wizard_state.step2);
    let fetching = orchestrator
        .wizard_state
        .step2
        .update_selected_download_running
        || orchestrator
            .wizard_state
            .step2
            .update_selected_extract_running;
    let suppress_escape = orchestrator.wizard_state.step2.versions_ui.menu.is_some()
        || orchestrator.wizard_state.step2.versions_ui.sheet.is_some();
    let retry_count = orchestrator
        .wizard_state
        .step2
        .update_selected_exact_version_retry_requests
        .len();
    HeaderCtx {
        report_text,
        busy,
        fetching,
        suppress_escape,
        check_sources_label: if busy {
            "Checking\u{2026}"
        } else {
            "Check sources"
        },
        retry_count,
    }
}

fn active_modlist_id(orchestrator: &OrchestratorApp) -> Option<String> {
    let id = orchestrator.workspace_view.modlist_id.trim();
    (!id.is_empty()).then(|| id.to_string())
}

fn active_modlist_display_name(orchestrator: &OrchestratorApp) -> String {
    let Some(id) = active_modlist_id(orchestrator) else {
        return orchestrator.workspace_view.modlist_name.clone();
    };
    orchestrator
        .registry
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .map_or_else(
            || orchestrator.workspace_view.modlist_name.clone(),
            |entry| entry.name.clone(),
        )
}

fn ensure_known_extras(orchestrator: &mut OrchestratorApp) {
    if orchestrator.wizard_state.step2.versions_ui.known.is_some() {
        return;
    }
    let active_id = active_modlist_id(orchestrator);
    let lists: Vec<(String, String)> = orchestrator
        .registry
        .entries
        .iter()
        .map(|entry| (entry.id.clone(), entry.name.clone()))
        .collect();
    let extras = versions_view::load_known_extras(active_id.as_deref(), &lists);
    orchestrator.wizard_state.step2.versions_ui.known = Some(extras);
}

fn note_who(orchestrator: &OrchestratorApp) -> String {
    if mod_downloads::active_modlist_downloads_path().is_some() {
        let name = orchestrator.workspace_view.modlist_name.trim();
        if name.is_empty() {
            "My default".to_string()
        } else {
            name.to_string()
        }
    } else {
        "My default".to_string()
    }
}

struct RenderPrep {
    tiers: Arc<mod_downloads::SourceTiers>,
    view: VersionsView,
    subtitle: String,
    header: HeaderCtx,
}

fn prepare_render(
    ctx: &egui::Context,
    orchestrator: &mut OrchestratorApp,
    action: &mut Option<Step2Action>,
) -> RenderPrep {
    if let Some(toast) = orchestrator
        .wizard_state
        .step2
        .versions_ui
        .pending_toast
        .take()
    {
        orchestrator.notification_manager.success(toast);
    }

    ensure_known_extras(orchestrator);
    let tiers = cached_source_tiers(ctx);
    let view = versions_view::build_versions_view(
        &orchestrator.wizard_state,
        &tiers,
        orchestrator.wizard_state.step2.versions_ui.known.as_ref(),
    );

    maybe_emit_auto_check(orchestrator, action);

    let modlist_name = orchestrator.workspace_view.modlist_name.clone();
    let subtitle = subtitle_text(orchestrator, &modlist_name, view.cards.len());
    let header = build_header_ctx(orchestrator);
    RenderPrep {
        tiers,
        view,
        subtitle,
        header,
    }
}

pub(crate) fn render(
    ctx: &egui::Context,
    orchestrator: &mut OrchestratorApp,
    action: &mut Option<Step2Action>,
    palette: ThemePalette,
) {
    if !orchestrator.wizard_state.step2.update_selected_popup_open {
        return;
    }

    let RenderPrep {
        tiers,
        view,
        subtitle,
        header,
    } = prepare_render(ctx, orchestrator, action);
    let busy = header.busy;

    let spec = DrawerSpec {
        id_salt: "versions_drawer",
        title: "Versions",
        subtitle: &subtitle,
        width: DrawerWidth::Wide,
        header_button: Some(drawer::HeaderButton {
            label: header.check_sources_label,
            enabled: !busy,
        }),
        suppress_escape: header.suppress_escape,
    };

    let mut body_action: Option<Step2Action> = None;
    let mut anchor_for_menu: Option<AnchorInfo> = None;
    let mut footer_action: Option<Step2Action> = None;
    let mut close_clicked = false;

    let response = drawer::render(
        ctx,
        palette,
        &spec,
        |ui| {
            if view.log_missing_count > 0 {
                render_log_strip(ui, palette, view.log_missing_count);
            }
            if header.retry_count > 0 {
                render_exact_version_miss_strip(
                    ui,
                    orchestrator,
                    palette,
                    busy,
                    header.retry_count,
                    &mut body_action,
                );
            }
            render_filter_row(ui, orchestrator, palette, &view);
            let list_env = CardListEnv {
                view: &view,
                tiers: &tiers,
                busy,
            };
            render_card_list(
                ui,
                orchestrator,
                palette,
                &list_env,
                &mut body_action,
                &mut anchor_for_menu,
            );
        },
        |ui| {
            let mut footer_ctx = FooterCtx {
                fetch_count: view.fetch_count,
                busy,
                fetching: header.fetching,
                report_text: &header.report_text,
                footer_action: &mut footer_action,
                close_clicked: &mut close_clicked,
            };
            render_footer(ui, palette, &mut footer_ctx);
        },
    );

    if response.header_clicked && !busy && body_action.is_none() {
        body_action = Some(handle_header_click(orchestrator, &view));
    }

    if let Some(a) = body_action.or(footer_action) {
        *action = Some(a);
    }

    if response.close_requested || close_clicked {
        close_drawer(orchestrator);
        return;
    }

    let list_env = CardListEnv {
        view: &view,
        tiers: &tiers,
        busy,
    };
    finish_render(
        ctx,
        orchestrator,
        palette,
        &list_env,
        action,
        anchor_for_menu,
    );
}

fn handle_header_click(orchestrator: &mut OrchestratorApp, view: &VersionsView) -> Step2Action {
    let repos: Vec<String> = view
        .cards
        .iter()
        .filter_map(|card| card.repo.clone())
        .collect();
    crate::app::github_release_list::drop_cached_release_lists(&repos);
    orchestrator.wizard_state.step2.versions_ui.release_list =
        crate::app::github_release_list::ReleaseListState::default();
    Step2Action::PreviewUpdateSelected
}

fn close_drawer(orchestrator: &mut OrchestratorApp) {
    orchestrator.wizard_state.step2.update_selected_popup_open = false;
    orchestrator.wizard_state.step2.versions_ui = VersionsDrawerUi::default();
    orchestrator
        .wizard_state
        .step2
        .update_selected_confirm_latest_fallback_open = false;
    versions_sheets::clear_editor_state(&mut orchestrator.wizard_state.step2);
}

struct CardListEnv<'a> {
    view: &'a VersionsView,
    tiers: &'a mod_downloads::SourceTiers,
    busy: bool,
}

fn finish_render(
    ctx: &egui::Context,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    env: &CardListEnv<'_>,
    action: &mut Option<Step2Action>,
    anchor_for_menu: Option<AnchorInfo>,
) {
    let drawer_rect = drawer_rect(ctx);
    let menu_open_before = orchestrator.wizard_state.step2.versions_ui.menu.is_some();
    render_open_menu(ctx, orchestrator, palette, env, action, anchor_for_menu);
    let escape_active_for_sheet =
        !menu_open_before && orchestrator.wizard_state.step2.versions_ui.menu.is_none();
    render_open_sheet(
        ctx,
        orchestrator,
        palette,
        drawer_rect,
        env.busy,
        escape_active_for_sheet,
        action,
    );
}

struct AnchorInfo {
    rect: egui::Rect,
    response: egui::Response,
}

fn render_exact_version_miss_strip(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    busy: bool,
    n: usize,
    body_action: &mut Option<Step2Action>,
) {
    let confirm_open = orchestrator
        .wizard_state
        .step2
        .update_selected_confirm_latest_fallback_open;
    ui.horizontal(|ui| {
        if confirm_open {
            render_latest_fallback_confirm_inline(ui, orchestrator, palette, busy, body_action);
        } else {
            render_latest_fallback_prompt(ui, orchestrator, palette, busy, n);
        }
    });
    ui.add_space(8.0);
}

fn render_latest_fallback_prompt(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    busy: bool,
    n: usize,
) {
    let word = if n == 1 { "mod" } else { "mods" };
    let verb = if n == 1 { "has" } else { "have" };
    ui.label(
        egui::RichText::new(format!(
            "{n} {word} {verb} no release matching the WeiDU log version"
        ))
        .size(12.0)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(redesign_text_muted(palette)),
    );
    if redesign_btn(
        ui,
        palette,
        "Use latest for these\u{2026}",
        BtnOpts {
            small: true,
            disabled: busy,
            ..Default::default()
        },
    )
    .clicked()
        && !busy
    {
        orchestrator
            .wizard_state
            .step2
            .update_selected_confirm_latest_fallback_open = true;
    }
}

fn render_latest_fallback_confirm_inline(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    busy: bool,
    body_action: &mut Option<Step2Action>,
) {
    ui.label(
        egui::RichText::new("Download the latest release instead for these mods?")
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    if redesign_btn(
        ui,
        palette,
        "Yes, use latest",
        BtnOpts {
            primary: true,
            small: true,
            disabled: busy,
            ..Default::default()
        },
    )
    .clicked()
        && !busy
    {
        orchestrator
            .wizard_state
            .step2
            .update_selected_confirm_latest_fallback_open = false;
        if body_action.is_none() {
            *body_action = Some(Step2Action::AcceptLatestForExactVersionMisses);
        }
    }
    if redesign_btn(
        ui,
        palette,
        "No",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .clicked()
    {
        orchestrator
            .wizard_state
            .step2
            .update_selected_confirm_latest_fallback_open = false;
    }
}

fn drawer_rect(ctx: &egui::Context) -> egui::Rect {
    let screen = ctx.screen_rect();
    let w = drawer::drawer_width(screen.width(), DrawerWidth::Wide);
    egui::Rect::from_min_max(
        egui::pos2(
            screen.right() - w,
            screen.top() + REDESIGN_TITLEBAR_HEIGHT_PX,
        ),
        screen.max,
    )
}

fn maybe_emit_auto_check(orchestrator: &mut OrchestratorApp, action: &mut Option<Step2Action>) {
    let step2 = &mut orchestrator.wizard_state.step2;
    if step2.versions_ui.auto_check_pending && !update_pipeline_busy(step2) {
        step2.versions_ui.auto_check_pending = false;
        if action.is_none() {
            *action = Some(Step2Action::PreviewUpdateSelected);
        }
    }
}

fn subtitle_text(orchestrator: &OrchestratorApp, modlist_name: &str, mod_count: usize) -> String {
    let step2 = &orchestrator.wizard_state.step2;
    let mods_word = if mod_count == 1 { "mod" } else { "mods" };
    let status = if step2.update_selected_check_running {
        format!(
            "checking {} of {}",
            step2.update_selected_check_done_count, step2.update_selected_check_total_count
        )
    } else if let Some(checked_at) = step2.update_selected_last_checked_at.as_deref() {
        format!("checked {checked_at}")
    } else {
        "not checked yet".to_string()
    };
    format!("{modlist_name} \u{00B7} {mod_count} {mods_word} \u{00B7} {status}")
}

fn render_log_strip(ui: &mut egui::Ui, palette: ThemePalette, n: usize) {
    let word = if n == 1 { "mod" } else { "mods" };
    let verb = if n == 1 { "is" } else { "are" };
    ui.label(
        egui::RichText::new(format!("{n} {word} from the WeiDU log {verb} not on disk"))
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(8.0);
}

fn filter_chip_width(ui: &egui::Ui, label: &str, count: usize) -> f32 {
    let text = format!("{label} ({count})");
    let pad_x = 11.0_f32;
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(text, font, egui::Color32::WHITE);
    pad_x.mul_add(2.0, galley.size().x)
}

fn render_filter_row(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    view: &VersionsView,
) {
    let chip = orchestrator.wizard_state.step2.versions_ui.chip;
    let total = view.cards.len();
    let item_spacing = 10.0_f32;
    let pills_width = filter_chip_width(ui, "Fetch", view.fetch_count)
        + filter_chip_width(ui, "Attention", view.attention_count)
        + filter_chip_width(ui, "Locked", view.locked_count)
        + filter_chip_width(ui, "All", total);
    let gaps = item_spacing * 4.0;
    let search_box_width = (ui.available_width() - pills_width - gaps).clamp(160.0, 280.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = item_spacing;
        render_search_box(
            ui,
            palette,
            &mut orchestrator.wizard_state.step2.versions_ui.search,
            search_box_width,
        );
        if render_filter_chip(
            ui,
            palette,
            "Fetch",
            view.fetch_count,
            chip == VersionsChip::Fetch,
        ) {
            orchestrator.wizard_state.step2.versions_ui.chip = VersionsChip::Fetch;
        }
        if render_filter_chip(
            ui,
            palette,
            "Attention",
            view.attention_count,
            chip == VersionsChip::Attention,
        ) {
            orchestrator.wizard_state.step2.versions_ui.chip = VersionsChip::Attention;
        }
        if render_filter_chip(
            ui,
            palette,
            "Locked",
            view.locked_count,
            chip == VersionsChip::Locked,
        ) {
            orchestrator.wizard_state.step2.versions_ui.chip = VersionsChip::Locked;
        }
        if render_filter_chip(ui, palette, "All", total, chip == VersionsChip::All) {
            orchestrator.wizard_state.step2.versions_ui.chip = VersionsChip::All;
        }
    });
    ui.add_space(12.0);
}

fn render_search_box(ui: &mut egui::Ui, palette: ThemePalette, search: &mut String, width: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 30.0), egui::Sense::hover());
        versions_icons::paint_search(
            ui.painter(),
            icon_rect.center(),
            redesign_text_faint(palette),
        );
        let margin = egui::Margin::symmetric(8, 4);
        redesign_text_input(
            ui,
            palette,
            InputOpts {
                edit: egui::TextEdit::singleline(search)
                    .hint_text("Search mods")
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .margin(margin)
                    .font(egui::FontId::new(
                        13.0,
                        egui::FontFamily::Name("poppins_light".into()),
                    )),
                margin,
                size: egui::vec2(width - 24.0, 30.0),
                border: None,
            },
        );
    });
}

fn render_filter_chip(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    count: usize,
    selected: bool,
) -> bool {
    let text = format!("{label} ({count})");
    let pad_x = 11.0_f32;
    let pad_y = 5.0_f32;
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into()));
    let color = if selected {
        redesign_pill_text(palette)
    } else {
        redesign_text_primary(palette)
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.clone(), font.clone(), color);
    let size = egui::vec2(
        pad_x.mul_add(2.0, galley.size().x),
        pad_y.mul_add(2.0, galley.size().y),
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
            text,
            font,
            color,
        );
    }
    response.clicked()
}

fn render_card_list(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    env: &CardListEnv<'_>,
    body_action: &mut Option<Step2Action>,
    anchor_for_menu: &mut Option<AnchorInfo>,
) {
    let view = env.view;
    let tiers = env.tiers;
    let busy = env.busy;
    let search = orchestrator
        .wizard_state
        .step2
        .versions_ui
        .search
        .trim()
        .to_ascii_lowercase();
    let chip = orchestrator.wizard_state.step2.versions_ui.chip;
    let open_menu = orchestrator.wizard_state.step2.versions_ui.menu.clone();
    let card_inner_width = (ui.available_width() - 32.0).max(0.0);
    let selector_width = (card_inner_width * 0.38).clamp(220.0, 380.0);

    let matches_chip = |card: &VersionCard| match chip {
        VersionsChip::All => true,
        VersionsChip::Fetch => matches!(card.status, versions_view::CardStatus::Fetch),
        VersionsChip::Attention => matches!(card.status, versions_view::CardStatus::Attention),
        VersionsChip::Locked => card.locked,
    };
    let pass = |card: &VersionCard| {
        (search.is_empty() || card.name.to_ascii_lowercase().contains(&search))
            && matches_chip(card)
    };

    let fetch_cards: Vec<&VersionCard> = view
        .cards
        .iter()
        .filter(|c| matches!(c.status, versions_view::CardStatus::Fetch) && pass(c))
        .collect();
    let attention_cards: Vec<&VersionCard> = view
        .cards
        .iter()
        .filter(|c| matches!(c.status, versions_view::CardStatus::Attention) && pass(c))
        .collect();
    let in_sync_cards: Vec<&VersionCard> = view
        .cards
        .iter()
        .filter(|c| matches!(c.status, versions_view::CardStatus::InSync) && pass(c))
        .collect();

    let any_shown =
        !fetch_cards.is_empty() || !attention_cards.is_empty() || !in_sync_cards.is_empty();

    let mut ctx = ListRenderCtx {
        palette,
        selector_width,
        tiers,
        busy,
        body_action,
        anchor_for_menu,
        open_menu: open_menu.as_ref(),
    };

    egui::ScrollArea::vertical()
        .id_salt("versions_card_list")
        .auto_shrink([false, false])
        .min_scrolled_height(200.0)
        .show(ui, |ui| {
            render_group(ui, orchestrator, "FETCH NEEDED", &fetch_cards, &mut ctx);
            render_group(
                ui,
                orchestrator,
                "NEEDS ATTENTION",
                &attention_cards,
                &mut ctx,
            );
            render_group(ui, orchestrator, "IN SYNC", &in_sync_cards, &mut ctx);
            if !any_shown {
                ui.label(
                    egui::RichText::new("No mods match")
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_muted(palette)),
                );
            }
        });
}

struct ListRenderCtx<'a> {
    palette: ThemePalette,
    selector_width: f32,
    tiers: &'a mod_downloads::SourceTiers,
    busy: bool,
    body_action: &'a mut Option<Step2Action>,
    anchor_for_menu: &'a mut Option<AnchorInfo>,
    open_menu: Option<&'a VersionsMenu>,
}

fn render_group(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    label: &str,
    cards: &[&VersionCard],
    ctx: &mut ListRenderCtx<'_>,
) {
    if cards.is_empty() {
        return;
    }
    ui.add_space(18.0);
    ui.label(
        egui::RichText::new(format!("{label} \u{00B7} {}", cards.len()))
            .size(11.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_faint(ctx.palette)),
    );
    ui.add_space(6.0);
    for card in cards {
        render_one_card(ui, orchestrator, card, ctx);
    }
}

fn render_one_card(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    card: &VersionCard,
    ctx: &mut ListRenderCtx<'_>,
) {
    let versions_ui = &orchestrator.wizard_state.step2.versions_ui;
    let focused = versions_ui.focused_tp2.as_deref() == Some(card.tp2.as_str());
    let scroll_to_focus = focused && versions_ui.focus_scroll_pending;
    let event = versions_card::render(ui, ctx.palette, card, ctx.selector_width, ctx.busy, focused);
    if scroll_to_focus {
        ui.scroll_to_rect(event.rect, Some(egui::Align::Center));
        orchestrator
            .wizard_state
            .step2
            .versions_ui
            .focus_scroll_pending = false;
    }
    ui.add_space(6.0);

    if let Some(action) = event.action
        && ctx.body_action.is_none()
    {
        *ctx.body_action = Some(action);
    }

    let selector_clicked = event.selector_response.clicked();
    let kebab_clicked = event.kebab_response.clicked();

    if selector_clicked {
        if card.source_id.is_some() {
            orchestrator.wizard_state.step2.versions_ui.menu = Some(VersionsMenu::Sources {
                tp2: card.tp2.clone(),
            });
        } else {
            let this_modlist_name = active_modlist_display_name(orchestrator);
            let form = versions_sheets::seed_source_form_for_card(
                ctx.tiers,
                card,
                None,
                &this_modlist_name,
            );
            orchestrator.wizard_state.step2.versions_ui.source_form = Some(form);
            versions_form::reset_dropdown_state(ui.ctx(), &card.tp2);
            orchestrator
                .wizard_state
                .step2
                .versions_ui
                .open_sheet(VersionsSheet::EditSource, card.tp2.clone());
        }
    }
    if kebab_clicked {
        let bookmark_label = versions_view::bookmark_label(&orchestrator.wizard_state, &card.tp2);
        orchestrator.wizard_state.step2.versions_ui.menu = Some(VersionsMenu::Kebab {
            tp2: card.tp2.clone(),
            bookmark_label,
        });
    }

    let is_open_sources =
        matches!(ctx.open_menu, Some(VersionsMenu::Sources { tp2 }) if tp2 == &card.tp2);
    let is_open_kebab =
        matches!(ctx.open_menu, Some(VersionsMenu::Kebab { tp2, .. }) if tp2 == &card.tp2);

    if is_open_sources || (selector_clicked && card.source_id.is_some()) {
        *ctx.anchor_for_menu = Some(AnchorInfo {
            rect: event.selector_rect,
            response: event.selector_response.clone(),
        });
    }
    if is_open_kebab || kebab_clicked {
        *ctx.anchor_for_menu = Some(AnchorInfo {
            rect: event.kebab_rect,
            response: event.kebab_response.clone(),
        });
    }
}

struct FooterCtx<'a> {
    fetch_count: usize,
    busy: bool,
    fetching: bool,
    report_text: &'a str,
    footer_action: &'a mut Option<Step2Action>,
    close_clicked: &'a mut bool,
}

fn render_footer(ui: &mut egui::Ui, palette: ThemePalette, footer: &mut FooterCtx<'_>) {
    let fetch_count = footer.fetch_count;
    let primary_label = if footer.fetching {
        "Fetching\u{2026}".to_string()
    } else if fetch_count == 0 {
        "Nothing to fetch".to_string()
    } else {
        format!(
            "Fetch {fetch_count} mod{}",
            if fetch_count == 1 { "" } else { "s" }
        )
    };
    let primary_disabled = fetch_count == 0 || footer.busy;
    if redesign_btn(
        ui,
        palette,
        &primary_label,
        BtnOpts {
            primary: true,
            small: true,
            disabled: primary_disabled,
            ..Default::default()
        },
    )
    .clicked()
        && !primary_disabled
    {
        *footer.footer_action = Some(Step2Action::DownloadUpdates);
    }
    if redesign_btn(
        ui,
        palette,
        "Copy report",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .clicked()
    {
        clipboard::copy(ui.ctx(), footer.report_text.to_string());
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if redesign_btn(
            ui,
            palette,
            "Close",
            BtnOpts {
                small: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            *footer.close_clicked = true;
        }
    });
}

fn render_open_menu(
    ctx: &egui::Context,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    env: &CardListEnv<'_>,
    action: &mut Option<Step2Action>,
    anchor: Option<AnchorInfo>,
) {
    let Some(menu) = orchestrator.wizard_state.step2.versions_ui.menu.clone() else {
        return;
    };
    let Some(anchor) = anchor else {
        orchestrator.wizard_state.step2.versions_ui.menu = None;
        return;
    };
    let tp2 = match &menu {
        VersionsMenu::Sources { tp2 } | VersionsMenu::Kebab { tp2, .. } => tp2.clone(),
    };
    let Some(card) = env.view.cards.iter().find(|c| c.tp2 == tp2) else {
        orchestrator.wizard_state.step2.versions_ui.menu = None;
        return;
    };
    let mut outcome = match menu {
        VersionsMenu::Sources { .. } => {
            let who = note_who(orchestrator);
            let sources_env = versions_menus::SourcesMenuEnv {
                card,
                busy: env.busy,
                who: &who,
                bounds: drawer_rect(ctx),
            };
            versions_menus::render_sources_menu(
                ctx,
                palette,
                anchor.rect,
                &anchor.response,
                &sources_env,
            )
        }
        VersionsMenu::Kebab { bookmark_label, .. } => {
            let kebab_env = versions_menus::KebabEnv {
                card,
                tiers: env.tiers,
                bookmark_label: bookmark_label.as_deref(),
            };
            versions_menus::render_kebab_menu(
                ctx,
                palette,
                anchor.rect,
                &anchor.response,
                &kebab_env,
                env.busy,
            )
        }
    };
    if outcome.action.is_some() && action.is_none() {
        *action = outcome.action;
    }
    if outcome.open_sheet == Some(VersionsSheet::EditSource) {
        let this_modlist_name = active_modlist_display_name(orchestrator);
        let known = orchestrator.wizard_state.step2.versions_ui.known.as_ref();
        let form =
            versions_sheets::seed_source_form_for_card(env.tiers, card, known, &this_modlist_name);
        orchestrator.wizard_state.step2.versions_ui.source_form = Some(form);
    }
    if outcome.open_sheet == Some(VersionsSheet::Note)
        && let Some(seed) = outcome.note_seed.take()
    {
        let who = note_who(orchestrator);
        versions_sheets::seed_note_sheet(ctx, &tp2, seed, who);
    }
    if let Some(sheet) = outcome.open_sheet {
        versions_form::reset_dropdown_state(ctx, &tp2);
        orchestrator
            .wizard_state
            .step2
            .versions_ui
            .open_sheet(sheet, tp2);
    }
    if outcome.close {
        orchestrator.wizard_state.step2.versions_ui.menu = None;
    }
}

fn render_open_sheet(
    ctx: &egui::Context,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    drawer_rect: egui::Rect,
    busy: bool,
    escape_active: bool,
    action: &mut Option<Step2Action>,
) {
    let Some(sheet) = orchestrator.wizard_state.step2.versions_ui.sheet else {
        versions_sheets::clear_note_sheet(ctx);
        return;
    };
    let just_opened = std::mem::take(
        &mut orchestrator
            .wizard_state
            .step2
            .versions_ui
            .sheet_just_opened,
    );
    let current_tp2 = orchestrator
        .wizard_state
        .step2
        .versions_ui
        .sheet_tp2
        .clone()
        .unwrap_or_default();
    let outcome = match sheet {
        VersionsSheet::EditSource => {
            let ready = versions_sheets::editor_ready_for(&orchestrator.wizard_state.step2);
            if ready {
                let release_list =
                    std::mem::take(&mut orchestrator.wizard_state.step2.versions_ui.release_list);
                let env = versions_sheets::EditSourceEnv {
                    busy,
                    logged_in: !orchestrator
                        .wizard_state
                        .github_auth_login
                        .trim()
                        .is_empty(),
                    release_list,
                };
                let result = versions_sheets::render_edit_source(
                    ctx,
                    palette,
                    drawer_rect,
                    &mut orchestrator.wizard_state.step2,
                    &env,
                    escape_active,
                );
                orchestrator.wizard_state.step2.versions_ui.release_list = env.release_list;
                result
            } else if just_opened {
                return;
            } else {
                versions_sheets::render_edit_source_unready(
                    ctx,
                    palette,
                    drawer_rect,
                    &mut orchestrator.wizard_state.step2,
                    escape_active,
                )
            }
        }
        VersionsSheet::Forks => versions_sheets::render_forks(
            ctx,
            palette,
            drawer_rect,
            &orchestrator.wizard_state.step2,
            escape_active,
        ),
        VersionsSheet::Note => {
            let note_env = versions_sheets::NoteEnv {
                sheet_error: orchestrator
                    .wizard_state
                    .step2
                    .versions_ui
                    .sheet_error
                    .as_deref(),
            };
            versions_sheets::render_note(ctx, palette, drawer_rect, escape_active, &note_env)
        }
    };
    if outcome.action.is_some() && action.is_none() {
        *action = outcome.action;
    }
    if let Some(form) = outcome.seed_form {
        orchestrator.wizard_state.step2.versions_ui.source_form = Some(form);
    }
    if let Some(new_sheet) = outcome.open_sheet {
        versions_form::reset_dropdown_state(ctx, &current_tp2);
        orchestrator
            .wizard_state
            .step2
            .versions_ui
            .open_sheet(new_sheet, current_tp2);
    } else if outcome.close {
        orchestrator.wizard_state.step2.versions_ui.sheet = None;
        orchestrator.wizard_state.step2.versions_ui.sheet_tp2 = None;
    }
}
