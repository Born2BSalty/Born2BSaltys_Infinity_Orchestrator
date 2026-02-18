// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::controller::step3_sync::{build_step3_items, collect_parent_block_ids};

use super::WizardApp;

pub(super) fn sync_step3_from_step2(app: &mut WizardApp) {
    app.state.step3.bgee_items = build_step3_items(&app.state.step2.bgee_mods);
    app.state.step3.bg2ee_items = build_step3_items(&app.state.step2.bg2ee_mods);
    app.state.step3.bgee_collapsed_blocks = collect_parent_block_ids(&app.state.step3.bgee_items);
    app.state.step3.bg2ee_collapsed_blocks = collect_parent_block_ids(&app.state.step3.bg2ee_items);
    app.state.step3.bgee_clone_seq = 1;
    app.state.step3.bg2ee_clone_seq = 1;
    app.state.step3.bgee_selected.clear();
    app.state.step3.bg2ee_selected.clear();
    app.state.step3.bgee_drag_from = None;
    app.state.step3.bg2ee_drag_from = None;
    app.state.step3.bgee_drag_over = None;
    app.state.step3.bg2ee_drag_over = None;
    app.state.step3.bgee_drag_indices.clear();
    app.state.step3.bg2ee_drag_indices.clear();
    app.state.step3.bgee_anchor = None;
    app.state.step3.bg2ee_anchor = None;
    app.state.step3.jump_to_selected_requested = false;
    app.state.step3.compat_modal_open = false;
    let show_bgee = matches!(app.state.step1.game_install.as_str(), "BGEE" | "EET");
    app.state.step3.active_game_tab = if show_bgee {
        "BGEE".to_string()
    } else {
        "BG2EE".to_string()
    };

    super::tp2_metadata::refresh_validator_tp2_metadata(app);
    app.revalidate_compat();
}
