use std::collections::BTreeSet;
use std::path::PathBuf;

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::state::VersionsSheet;
use crate::app::step2_action::Step2Action;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::icon_button::{self, ButtonIcon};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, redesign_btn_height};
use crate::ui::settings::state_settings::SettingsTab;
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_accent, redesign_border_soft, redesign_border_strong, redesign_error,
    redesign_shell_bg, redesign_success, redesign_text_muted, redesign_text_primary,
    redesign_warning,
};
use crate::ui::workspace::versions::{versions_form, versions_menus, versions_sheets};

use super::catalog_runtime::CatalogItemOutcome;
use super::state_catalog::{CatalogDestination, CatalogKind, CatalogRow, LocalPresence};

fn text(value: impl Into<String>, size: f32, palette: ThemePalette) -> egui::RichText {
    egui::RichText::new(value)
        .size(size)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(redesign_text_primary(palette))
}

fn panel(palette: ThemePalette) -> egui::Frame {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(1.0_f32, redesign_border_soft(palette)))
        .corner_radius(3)
        .inner_margin(16)
}

pub(crate) fn destination(app: &OrchestratorApp) -> CatalogDestination {
    CatalogDestination {
        mods_root: PathBuf::from(app.wizard_state.step1.global_mods_folder.trim()),
        archives: PathBuf::from(app.wizard_state.step1.mods_archive_folder.trim()),
        backup_root: PathBuf::from(app.wizard_state.step1.mods_backup_folder.trim()),
    }
}

pub(crate) fn render(ui: &mut egui::Ui, app: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = app.theme_palette;
    let destination = destination(app);
    app.catalog_runtime
        .ensure_loaded(&mut app.catalog_screen_state, &destination);
    app.catalog_runtime.poll(&mut app.catalog_screen_state);
    let rect = ui.max_rect();
    egui::Frame::default().show(ui, |ui| {
        ui.set_width(ui.available_width());
        render_header(ui, app, palette);
        let footer_height = catalog_footer_height(ui, app, palette, &destination);
        let body_height = (ui.available_height() - footer_height).max(100.0);
        if ui.available_width() >= 850.0 {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), body_height),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    let table_width = ui.available_width().mul_add(0.64, -8.0).max(240.0);
                    let table = ui.allocate_ui_with_layout(
                        egui::vec2(table_width, body_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| render_table(ui, app, palette, body_height),
                    );
                    ui.add_space(8.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), body_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            render_details(
                                ui,
                                app,
                                ctx,
                                palette,
                                &destination,
                                Some(table.inner.rect.height()),
                            );
                        },
                    );
                },
            );
        } else {
            egui::ScrollArea::vertical()
                .id_salt("catalog_narrow")
                .auto_shrink([false, false])
                .max_height(body_height)
                .show(ui, |ui| {
                    render_table(ui, app, palette, 240.0);
                    ui.add_space(12.0);
                    render_details(ui, app, ctx, palette, &destination, None);
                });
        }
        ui.add_space(6.0);
        let separator = ui.separator();
        let footer_content_height = ui.spacing().item_spacing.y.mul_add(-3.0, footer_height) - 12.0;
        let top_gap = ui.cursor().top() - separator.rect.center().y;
        let bottom_gap = ui.clip_rect().bottom() - ui.cursor().top() - footer_content_height;
        ui.add_space(((bottom_gap - top_gap) * 0.5).max(0.0));
        render_footer(ui, app, palette, &destination);
    });
    render_sheet(app, ctx, palette, rect);
    if app.catalog_runtime.running
        || app.catalog_runtime.scan_rx.is_some()
        || app.catalog_screen_state.editor_step2.forks_list.status
            == crate::app::github_forks_list::ForksStatus::Loading
        || app
            .catalog_screen_state
            .editor_step2
            .versions_ui
            .release_list
            .status
            == crate::app::github_release_list::ReleaseListStatus::Loading
    {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn render_header(ui: &mut egui::Ui, app: &mut OrchestratorApp, palette: ThemePalette) {
    ui.label(
        text("Mod catalog", 26.0, palette).family(egui::FontFamily::Name("poppins_medium".into())),
    );
    ui.label(
        text("Browse available mods and download sources.", 14.0, palette)
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(6.0);
    let kinds = [
        CatalogKind::All,
        CatalogKind::GitHub,
        CatalogKind::WeaselMods,
        CatalogKind::MorpheusMart,
        CatalogKind::DirectArchive,
    ];
    let labels = kinds.map(|kind| {
        format!(
            "{} ({})",
            kind.label(),
            app.catalog_screen_state.kind_count(kind)
        )
    });
    let tabs_width = ui.spacing().item_spacing.x.mul_add(
        4.0,
        labels
            .iter()
            .map(|label| catalog_text_width(ui, label, 14.0, "poppins_medium") + 32.0)
            .sum::<f32>(),
    );
    let inline = ui.available_width() >= tabs_width + 232.0;
    let height = redesign_btn_height(ui, false).max(36.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(
                if inline {
                    tabs_width
                } else {
                    ui.available_width()
                },
                height,
            ),
            egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
            |ui| {
                for (kind, label) in kinds.into_iter().zip(&labels) {
                    if redesign_btn(
                        ui,
                        palette,
                        label,
                        BtnOpts {
                            primary: app.catalog_screen_state.kind == kind,
                            ..Default::default()
                        },
                    )
                    .clicked()
                    {
                        app.catalog_screen_state.kind = kind;
                    }
                }
            },
        );
        if inline {
            render_search(ui, &mut app.catalog_screen_state.query, palette);
        }
    });
    if !inline {
        render_search(ui, &mut app.catalog_screen_state.query, palette);
    }
    ui.add_space(12.0);
    if let Some(error) = app.catalog_screen_state.error.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.label(text(error, 12.0, palette).color(redesign_error(palette)));
            if redesign_btn(
                ui,
                palette,
                "Retry",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                app.catalog_runtime.loaded = false;
            }
        });
    }
}

fn render_search(ui: &mut egui::Ui, query: &mut String, palette: ThemePalette) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::hover());
    ui.painter().rect_stroke(
        rect,
        3,
        egui::Stroke::new(1.0_f32, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
    let input_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(36.0, 0.0),
        rect.max - egui::vec2(10.0, 0.0),
    );
    table_cell(ui, input_rect, "catalog_search").add(
        egui::TextEdit::singleline(query)
            .frame(false)
            .margin(egui::Margin::ZERO)
            .desired_width(input_rect.width())
            .hint_text("Search mods, source labels or TP2\u{2026}")
            .font(egui::FontId::new(
                14.0,
                egui::FontFamily::Name("poppins_light".into()),
            )),
    );
    let center = egui::pos2(rect.left() + 16.0, rect.center().y - 1.0);
    let stroke = egui::Stroke::new(1.5_f32, redesign_text_muted(palette));
    ui.painter().circle_stroke(center, 6.0, stroke);
    ui.painter().line_segment(
        [center + egui::vec2(4.0, 4.0), center + egui::vec2(9.0, 9.0)],
        stroke,
    );
}

#[derive(Clone, Copy)]
struct CatalogColumns {
    width: f32,
    name: f32,
    source: f32,
    status: f32,
}

impl CatalogColumns {
    fn new(width: f32) -> Self {
        let content = (width - 96.0).max(0.0);
        let source = (content * 0.28).min(132.0);
        let status = (content * 0.34).min(140.0);
        Self {
            width,
            name: content - source - status,
            source,
            status,
        }
    }

    fn cells(self, rect: egui::Rect) -> [egui::Rect; 5] {
        let mut x = rect.left() + 4.0;
        [24.0, self.name, self.source, self.status, 24.0].map(|width| {
            let cell = egui::Rect::from_min_size(
                egui::pos2(x, rect.top()),
                egui::vec2(width, rect.height()),
            );
            x += width + 10.0;
            cell
        })
    }
}

fn table_cell(ui: &mut egui::Ui, rect: egui::Rect, id: &str) -> egui::Ui {
    let mut cell = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id)
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    cell.set_clip_rect(ui.clip_rect().intersect(rect));
    cell
}

fn render_table(
    ui: &mut egui::Ui,
    app: &mut OrchestratorApp,
    palette: ThemePalette,
    height: f32,
) -> egui::Response {
    panel(palette)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let scroll = ui.spacing().scroll;
            let scrollbar = scroll.bar_width + scroll.bar_inner_margin + scroll.bar_outer_margin;
            let columns = CatalogColumns::new((ui.available_width() - scrollbar).max(0.0));
            let visible = app.catalog_screen_state.visible_rows();
            let (header, _) =
                ui.allocate_exact_size(egui::vec2(columns.width, 24.0), egui::Sense::hover());
            let [checkbox, name, source, status, _] = columns.cells(header);
            let mut all = !visible.is_empty()
                && visible.iter().all(|&index| {
                    app.catalog_screen_state
                        .selected_tp2
                        .contains(&app.catalog_screen_state.rows[index].tp2)
                });
            if table_cell(ui, checkbox, "select_visible")
                .checkbox(&mut all, "")
                .on_hover_text("Select visible results")
                .changed()
            {
                app.catalog_screen_state.toggle_visible();
            }
            for (rect, label) in [(name, "Mod"), (source, "Source"), (status, "Local status")] {
                table_cell(ui, rect, label).add(
                    egui::Label::new(
                        text(label, 12.0, palette).color(redesign_text_muted(palette)),
                    )
                    .truncate(),
                );
            }
            ui.separator();
            if visible.is_empty() {
                ui.label(text(
                    if app.catalog_screen_state.rows.is_empty() {
                        "No configured download sources are available."
                    } else {
                        "No matching mods. Clear the search or choose All."
                    },
                    14.0,
                    palette,
                ));
                return;
            }
            egui::ScrollArea::vertical()
                .id_salt("catalog_rows")
                .auto_shrink([false, false])
                .max_height((height - 70.0).max(80.0))
                .show_rows(ui, 56.0, visible.len(), |ui, range| {
                    for index in range {
                        render_row(ui, app, palette, visible[index], columns);
                    }
                });
        })
        .response
}

fn render_row(
    ui: &mut egui::Ui,
    app: &mut OrchestratorApp,
    palette: ThemePalette,
    index: usize,
    columns: CatalogColumns,
) {
    let row = app.catalog_screen_state.rows[index].clone();
    let Some(source) = app.catalog_screen_state.source(&row.tp2).cloned() else {
        return;
    };
    ui.push_id(("catalog_row", &row.tp2), |ui| {
        let focused = app.catalog_screen_state.focused_tp2.as_deref() == Some(&row.tp2);
        let fill = if focused {
            redesign_accent(palette).gamma_multiply(0.15)
        } else {
            redesign_shell_bg(palette)
        };
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(columns.width, 52.0), egui::Sense::hover());
        let row_response = ui.interact(rect, ui.id().with("focus_row"), egui::Sense::click());
        ui.painter().rect_filled(rect, 0, fill);
        let [checkbox, name, kind, status, open] =
            columns.cells(rect.shrink2(egui::vec2(0.0, 6.0)));
        let mut selected = app.catalog_screen_state.selected_tp2.contains(&row.tp2);
        if table_cell(ui, checkbox, "select")
            .checkbox(&mut selected, "")
            .changed()
        {
            app.catalog_screen_state.toggle(&row.tp2);
        }
        let mut name_ui = table_cell(ui, name, "name");
        name_ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            render_row_name(ui, app, palette, &row, &source);
        });
        table_cell(ui, kind, "source").add(
            egui::Label::new(
                text(CatalogKind::of(&source).label(), 11.0, palette)
                    .color(redesign_text_muted(palette)),
            )
            .truncate(),
        );
        render_row_status(&mut table_cell(ui, status, "status"), app, palette, &row);
        if let Some(url) = mod_downloads::source_open_url(&source)
            && icon_button::render(
                &mut table_cell(ui, open, "open"),
                palette,
                ButtonIcon::Open,
                "Open source",
                true,
            )
            .clicked()
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
        if row_response.clicked() {
            app.catalog_screen_state.focus(&row.tp2);
        }
    });
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        egui::Stroke::new(1.0_f32, redesign_border_soft(palette)),
    );
}

fn render_row_name(
    ui: &mut egui::Ui,
    app: &mut OrchestratorApp,
    palette: ThemePalette,
    row: &CatalogRow,
    source: &mod_downloads::ModDownloadSource,
) {
    if ui
        .add(
            egui::Label::new(text(&row.name, 14.0, palette))
                .sense(egui::Sense::click())
                .truncate(),
        )
        .clicked()
    {
        app.catalog_screen_state.focus(&row.tp2);
    }
    let owner = source
        .github
        .as_deref()
        .and_then(|repo| repo.split('/').next())
        .unwrap_or(&source.source_label);
    if ui
        .add(
            egui::Label::new(text(owner, 11.0, palette).color(redesign_text_muted(palette)))
                .sense(egui::Sense::click())
                .truncate(),
        )
        .clicked()
    {
        app.catalog_screen_state.focus(&row.tp2);
    }
}

fn render_row_status(
    ui: &mut egui::Ui,
    app: &OrchestratorApp,
    palette: ThemePalette,
    row: &CatalogRow,
) {
    let presence = app
        .catalog_screen_state
        .local_presence
        .get(&row.tp2)
        .cloned()
        .unwrap_or_default();
    let color = match &presence {
        LocalPresence::Found(_) => redesign_success(palette),
        LocalPresence::Scanning => redesign_warning(palette),
        LocalPresence::Error(_) => redesign_error(palette),
        LocalPresence::Unknown | LocalPresence::NotFound => redesign_text_muted(palette),
    };
    let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(dot.center(), 3.0, color);
    let response =
        ui.add(egui::Label::new(text(presence.label(), 11.0, palette).color(color)).truncate());
    if let LocalPresence::Error(error) = presence {
        response.on_hover_text(error);
    }
}

fn render_details(
    ui: &mut egui::Ui,
    app: &mut OrchestratorApp,
    ctx: &egui::Context,
    palette: ThemePalette,
    destination: &CatalogDestination,
    height: Option<f32>,
) {
    let frame = panel(palette);
    let margin = frame.total_margin().sum().y;
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        if let Some(height) = height {
            let inner_height = (height - margin).max(0.0);
            ui.set_height(inner_height);
            egui::ScrollArea::vertical()
                .id_salt("catalog_details")
                .auto_shrink([false, false])
                .max_height(inner_height)
                .show(ui, |ui| {
                    render_details_content(ui, app, ctx, palette, destination);
                });
        } else {
            render_details_content(ui, app, ctx, palette, destination);
        }
    });
}

fn render_details_content(
    ui: &mut egui::Ui,
    app: &mut OrchestratorApp,
    ctx: &egui::Context,
    palette: ThemePalette,
    destination: &CatalogDestination,
) {
    let Some(key) = app.catalog_screen_state.focused_tp2.clone() else {
        ui.label(text(
            "Select a mod to see its download sources.",
            14.0,
            palette,
        ));
        return;
    };
    let Some(row) = app
        .catalog_screen_state
        .rows
        .iter()
        .find(|row| row.tp2 == key)
        .cloned()
    else {
        return;
    };
    let Some(source) = app.catalog_screen_state.source(&key).cloned() else {
        return;
    };
    ui.label(
        text(&row.name, 20.0, palette).family(egui::FontFamily::Name("poppins_medium".into())),
    );
    ui.label(
        text(
            format!("{} download sources", row.sources.len()),
            12.0,
            palette,
        )
        .color(redesign_text_muted(palette)),
    );
    ui.separator();
    render_source_actions(ui, app, ctx, palette, &key, &row, &source);
    render_download_detail(ui, app, palette, destination, &key);
}

fn render_source_actions(
    ui: &mut egui::Ui,
    app: &mut OrchestratorApp,
    ctx: &egui::Context,
    palette: ThemePalette,
    key: &str,
    row: &CatalogRow,
    source: &mod_downloads::ModDownloadSource,
) {
    ui.add_space(10.0);
    ui.label(text(format!("TP2     {}", source.tp2), 13.0, palette));
    ui.add_space(12.0);
    let tier = match app
        .catalog_screen_state
        .tiers
        .tier_of(&source.tp2, &source.source_id)
    {
        mod_downloads::SourceTier::Default => "BIO catalog",
        mod_downloads::SourceTier::User => "Your saved source",
        mod_downloads::SourceTier::Modlist => "This modlist",
    };
    ui.label(
        text(format!("Source provided by: {tier}"), 12.0, palette)
            .color(redesign_text_muted(palette)),
    );
    ui.label(text("Download from", 14.0, palette));
    let selector = ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 56.0;
        ui.spacing_mut().button_padding = egui::vec2(10.0, 8.0);
        versions_menus::render_catalog_selector(
            ui,
            palette,
            key,
            &row.sources,
            source,
            &app.catalog_screen_state.tiers,
        )
    });
    if let Some(picked) = selector.inner {
        app.catalog_screen_state.choose_source(key, &picked);
    }
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        if redesign_btn(
            ui,
            palette,
            "Edit source\u{2026}",
            BtnOpts {
                small: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            app.catalog_screen_state.open_editor(key);
            if let Some(form) = &app
                .catalog_screen_state
                .editor_step2
                .versions_ui
                .source_form
            {
                versions_form::reset_dropdown_state(ctx, &form.card_key);
            }
        }
        if redesign_btn(
            ui,
            palette,
            "Find forks\u{2026}",
            BtnOpts {
                small: true,
                disabled: source.github.is_none(),
                ..Default::default()
            },
        )
        .clicked()
        {
            app.catalog_runtime
                .open_forks(&mut app.catalog_screen_state, key);
        }
    });
    if row.sources.len() > 1 {
        ui.add_space(8.0);
        ui.label(
            text(
                format!(
                    "Also available: {}",
                    row.sources
                        .iter()
                        .filter(|s| s.source_id != source.source_id)
                        .map(|s| if s.source_label.trim().is_empty() {
                            s.source_id.clone()
                        } else {
                            s.source_label.clone()
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                12.0,
                palette,
            )
            .color(redesign_text_muted(palette)),
        );
    }
    render_source_link(ui, palette, source);
}

fn render_source_link(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    source: &mod_downloads::ModDownloadSource,
) {
    if let Some(url) = mod_downloads::source_open_url(source) {
        let color = redesign_accent(palette);
        let mut layout = egui::text::LayoutJob::default();
        layout.append(
            "Open selected source",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into())),
                color,
                underline: egui::Stroke::new(1.0_f32, color),
                ..Default::default()
            },
        );
        layout.append(
            "\u{F08E}",
            5.0,
            egui::TextFormat {
                font_id: egui::FontId::new(14.0, egui::FontFamily::Name("firacode_nerd".into())),
                color,
                ..Default::default()
            },
        );
        ui.scope(|ui| {
            ui.visuals_mut().hyperlink_color = color;
            ui.hyperlink_to(layout, url);
        });
    }
}

fn render_download_detail(
    ui: &mut egui::Ui,
    app: &mut OrchestratorApp,
    palette: ThemePalette,
    destination: &CatalogDestination,
    key: &str,
) {
    ui.separator();
    if redesign_btn(
        ui,
        palette,
        "Download this mod",
        BtnOpts {
            primary: true,
            block: true,
            disabled: destination.validate().is_err() || app.catalog_runtime.running,
            ..Default::default()
        },
    )
    .clicked()
    {
        start_download(app, &BTreeSet::from([key.to_string()]), destination.clone());
    }
    if let Some(outcome) = app.catalog_runtime.outcomes.get(key) {
        let message = match outcome {
            CatalogItemOutcome::Completed(path) => format!("Extracted to {path}"),
            CatalogItemOutcome::Failed(error) => error.clone(),
            CatalogItemOutcome::Canceled => "Canceled".to_string(),
        };
        ui.label(text(message, 12.0, palette).color(
            if matches!(outcome, CatalogItemOutcome::Failed(_)) {
                redesign_error(palette)
            } else {
                redesign_text_muted(palette)
            },
        ));
    }
}

fn start_download(
    app: &mut OrchestratorApp,
    keys: &BTreeSet<String>,
    destination: CatalogDestination,
) {
    let step2 = &app.wizard_state.step2;
    let busy = step2.update_selected_check_running
        || step2.update_selected_download_running
        || step2.update_selected_extract_running;
    if let Err(error) =
        app.catalog_runtime
            .start(&mut app.catalog_screen_state, keys, destination, busy)
    {
        app.catalog_screen_state.status = error;
    }
}

fn catalog_footer_height(
    ui: &egui::Ui,
    app: &OrchestratorApp,
    palette: ThemePalette,
    destination: &CatalogDestination,
) -> f32 {
    let spacing = ui.spacing().item_spacing.y;
    let label = format!(
        "Download selected ({})",
        app.catalog_screen_state.selected_tp2.len()
    );
    let widths = footer_widths(ui, app, &label);
    let mut height = spacing.mul_add(3.0, redesign_btn_height(ui, false) + 12.0);
    if footer_paths_wrap(ui, widths) {
        height += 24.0 + spacing;
    }
    if app.catalog_runtime.running {
        height += redesign_btn_height(ui, true) + spacing;
    }
    let error = destination.validate().err();
    for (message, size) in [
        (error.as_deref(), 11.0),
        (
            (!app.catalog_screen_state.status.is_empty())
                .then_some(app.catalog_screen_state.status.as_str()),
            12.0,
        ),
    ] {
        if let Some(message) = message {
            height += ui
                .painter()
                .layout(
                    message.to_string(),
                    egui::FontId::new(size, egui::FontFamily::Name("poppins_light".into())),
                    redesign_text_primary(palette),
                    ui.available_width(),
                )
                .size()
                .y
                + spacing;
        }
    }
    height
}

fn render_footer(
    ui: &mut egui::Ui,
    app: &mut OrchestratorApp,
    palette: ThemePalette,
    destination: &CatalogDestination,
) {
    let label = format!(
        "Download selected ({})",
        app.catalog_screen_state.selected_tp2.len()
    );
    let widths = footer_widths(ui, app, &label);
    let wrapped = footer_paths_wrap(ui, widths);
    let height = redesign_btn_height(ui, false);
    let gap = ui.spacing().item_spacing.x;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let button = egui::Rect::from_min_size(
        egui::pos2(rect.right() - widths[3], rect.top()),
        egui::vec2(widths[3], height),
    );
    let count = egui::Rect::from_min_size(rect.min, egui::vec2(widths[0], height));
    table_cell(ui, count, "selection_summary").label(text(
        format!(
            "{} mods selected",
            app.catalog_screen_state.selected_tp2.len()
        ),
        12.0,
        palette,
    ));
    let clear = egui::Rect::from_min_size(
        egui::pos2(count.right() + gap, rect.top()),
        egui::vec2(widths[1], height),
    );
    if table_cell(ui, clear, "clear_selection")
        .link(text("Clear selection", 12.0, palette).color(redesign_accent(palette)))
        .clicked()
    {
        app.catalog_screen_state.selected_tp2.clear();
    }
    let change = egui::Rect::from_min_size(
        egui::pos2(button.left() - gap - widths[2], rect.top()),
        egui::vec2(widths[2], height),
    );
    if table_cell(ui, change, "change_destination")
        .link(text("Change\u{2026}", 12.0, palette).color(redesign_accent(palette)))
        .clicked()
    {
        app.settings_screen_state.active_tab = SettingsTab::Paths;
        app.nav = NavDestination::Settings;
    }
    if !wrapped {
        let paths = egui::Rect::from_min_max(
            egui::pos2(clear.right() + gap, rect.top()),
            egui::pos2(change.left() - gap, rect.bottom()),
        );
        render_destination_line(ui, palette, destination, paths);
    }
    if redesign_btn(
        &mut table_cell(ui, button, "download_selected"),
        palette,
        &label,
        BtnOpts {
            primary: true,
            disabled: destination.validate().is_err()
                || app.catalog_screen_state.selected_tp2.is_empty()
                || app.catalog_runtime.running,
            ..Default::default()
        },
    )
    .clicked()
    {
        start_download(
            app,
            &app.catalog_screen_state.selected_tp2.clone(),
            destination.clone(),
        );
    }
    if wrapped {
        let (paths, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
        render_destination_line(ui, palette, destination, paths);
    }
    if app.catalog_runtime.running
        && redesign_btn(
            ui,
            palette,
            "Cancel download",
            BtnOpts {
                small: true,
                ..Default::default()
            },
        )
        .clicked()
    {
        app.catalog_runtime.cancel();
        app.catalog_screen_state.status =
            "Canceling\u{2026} The current extraction may finish safely.".to_string();
    }
    if let Err(error) = destination.validate() {
        ui.label(text(error, 11.0, palette).color(redesign_text_muted(palette)));
    }
    if !app.catalog_screen_state.status.is_empty() {
        ui.label(text(&app.catalog_screen_state.status, 12.0, palette));
    }
}

fn catalog_text_width(ui: &egui::Ui, value: &str, size: f32, family: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(
            value.to_string(),
            egui::FontId::new(size, egui::FontFamily::Name(family.into())),
            egui::Color32::WHITE,
        )
        .size()
        .x
}

fn footer_widths(ui: &egui::Ui, app: &OrchestratorApp, label: &str) -> [f32; 4] {
    [
        catalog_text_width(
            ui,
            &format!(
                "{} mods selected",
                app.catalog_screen_state.selected_tp2.len()
            ),
            12.0,
            "poppins_light",
        ),
        catalog_text_width(ui, "Clear selection", 12.0, "poppins_light"),
        catalog_text_width(ui, "Change\u{2026}", 12.0, "poppins_light"),
        catalog_text_width(ui, label, 14.0, "poppins_medium") + 32.0,
    ]
}

fn footer_paths_wrap(ui: &egui::Ui, widths: [f32; 4]) -> bool {
    ui.spacing()
        .item_spacing
        .x
        .mul_add(-5.0, ui.available_width() - widths.iter().sum::<f32>())
        < 288.0
}

fn render_destination_line(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    destination: &CatalogDestination,
    rect: egui::Rect,
) {
    let width = (rect.width() - ui.spacing().item_spacing.x).max(0.0) * 0.5;
    for (index, (label, path, empty)) in [
        (
            "Download",
            &destination.archives,
            "Mods Archive folder not configured",
        ),
        (
            "Extract",
            &destination.mods_root,
            "Global Mods folder not configured",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let offset = if index == 0 {
            0.0
        } else {
            width + ui.spacing().item_spacing.x
        };
        let group = egui::Rect::from_min_size(
            rect.min + egui::vec2(offset, 0.0),
            egui::vec2(width, rect.height()),
        );
        let mut group_ui = table_cell(ui, group, label);
        group_ui.spacing_mut().item_spacing.x = 4.0;
        group_ui.label(text(label, 12.0, palette).color(redesign_text_muted(palette)));
        group_ui.label(text("\u{2192}", 12.0, palette).color(redesign_text_muted(palette)));
        let full = if path.as_os_str().is_empty() {
            empty.to_string()
        } else {
            path.display().to_string()
        };
        let response = group_ui
            .add_sized(
                egui::vec2(group_ui.available_width().max(0.0), 18.0),
                egui::Label::new(text(&full, 12.0, palette).color(redesign_text_muted(palette)))
                    .halign(egui::Align::Min)
                    .truncate(),
            )
            .on_hover_text(&full);
        response.context_menu(|ui| {
            if ui.button("Copy path").clicked() {
                ui.ctx().copy_text(path.display().to_string());
                ui.close_menu();
            }
        });
    }
}

fn render_sheet(
    app: &mut OrchestratorApp,
    ctx: &egui::Context,
    palette: ThemePalette,
    rect: egui::Rect,
) {
    let sheet = app.catalog_screen_state.editor_step2.versions_ui.sheet;
    let outcome = match sheet {
        Some(VersionsSheet::EditSource) => {
            let env = versions_sheets::EditSourceEnv {
                busy: false,
                logged_in: !app.wizard_state.github_auth_login.trim().is_empty(),
                release_list: app
                    .catalog_screen_state
                    .editor_step2
                    .versions_ui
                    .release_list
                    .clone(),
                has_modlist_destination: false,
                id_salt: "catalog_sheet_edit",
            };
            Some(versions_sheets::render_edit_source(
                ctx,
                palette,
                rect,
                &mut app.catalog_screen_state.editor_step2,
                &env,
                true,
            ))
        }
        Some(VersionsSheet::Forks) => Some(versions_sheets::render_forks(
            ctx,
            palette,
            rect,
            &app.catalog_screen_state.editor_step2,
            true,
        )),
        _ => None,
    };
    if let Some(outcome) = outcome {
        if outcome.close {
            app.catalog_runtime
                .dismiss_editor(&mut app.catalog_screen_state);
        }
        if let Some(form) = outcome.seed_form
            && let Some(fork) = app
                .catalog_screen_state
                .editor_step2
                .mod_download_forks
                .iter()
                .find(|fork| fork.full_name == form.repo)
                .cloned()
        {
            app.catalog_runtime
                .use_fork(&mut app.catalog_screen_state, &fork);
            if let Some(form) = &app
                .catalog_screen_state
                .editor_step2
                .versions_ui
                .source_form
            {
                versions_form::reset_dropdown_state(ctx, &form.card_key);
            }
        }
        match outcome.action {
            Some(Step2Action::RequestReleaseList { repo }) => app
                .catalog_runtime
                .request_releases(&mut app.catalog_screen_state, repo),
            Some(Step2Action::SaveSourceForm) => {
                let previous_status = app.catalog_screen_state.status.clone();
                if app
                    .catalog_runtime
                    .save_editor(&mut app.catalog_screen_state)
                    .is_ok()
                {
                    let notice =
                        std::mem::replace(&mut app.catalog_screen_state.status, previous_status);
                    if notice == "Saved to My default" {
                        app.notification_manager.success(notice);
                    } else {
                        app.notification_manager.warn(notice);
                    }
                }
            }
            _ => {}
        }
    }
}
