// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::controller::util::open_in_shell;
use crate::ui::pages::step2::Step2Action;
use crate::ui::step2::compat::{apply_step2_compat_rules, export_step2_compat_report};

use super::WizardApp;
use super::{step2_log, step2_scan};

pub(super) fn handle_step2_action(app: &mut WizardApp, action: Step2Action) {
    match action {
        Step2Action::StartScan => step2_scan::start_step2_scan(app),
        Step2Action::CancelScan => step2_scan::cancel_step2_scan(app),
        Step2Action::SelectBgeeViaLog => step2_log::apply_weidu_log_selection(app, true),
        Step2Action::SelectBg2eeViaLog => step2_log::apply_weidu_log_selection(app, false),
        Step2Action::RevalidateCompat => {
            reapply_step2_rules(app);
            app.revalidate_compat_step2_checked_order();
            app.state.step2.scan_status = format!(
                "Compatibility revalidated: {} errors, {} warnings",
                app.state.compat.error_count, app.state.compat.warning_count
            );
        }
        Step2Action::ExportCompatReport => match export_step2_compat_report(&app.state.step2, &app.state.compat) {
            Ok(path) => {
                app.state.step2.scan_status = format!("Compat report exported: {}", path.display());
            }
            Err(err) => {
                app.state.step2.scan_status = format!("Compat export failed: {err}");
            }
        },
        Step2Action::OpenSelectedReadme(path)
        | Step2Action::OpenSelectedTp2(path)
        | Step2Action::OpenSelectedWeb(path) => {
            if let Err(err) = open_in_shell(&path) {
                app.state.step2.scan_status = format!("Open failed: {err}");
            }
        }
        Step2Action::OpenCompatForComponent {
            game_tab,
            tp_file,
            component_id,
            component_key,
        } => {
            app.state.step2.selected = Some(crate::ui::state::Step2Selection::Component {
                game_tab,
                tp_file,
                component_id,
                component_key,
            });
            app.state.step2.compat_popup_open = true;
        }
    }
}

fn reapply_step2_rules(app: &mut WizardApp) {
    apply_step2_compat_rules(
        &app.state.step1,
        &mut app.state.step2.bgee_mods,
        &mut app.state.step2.bg2ee_mods,
    );
}
