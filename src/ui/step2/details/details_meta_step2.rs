// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::selected_details::SelectedDetailsData;
use crate::ui::step2::state_step2::Step2Details;

pub(crate) fn map_selected_details(data: SelectedDetailsData) -> Step2Details {
    Step2Details {
        mod_name: data.mod_name,
        component_label: data.component_label,
        component_id: data.component_id,
        shown_component_count: data.shown_component_count,
        hidden_component_count: data.hidden_component_count,
        raw_component_count: data.raw_component_count,
        component_lang: data.component_lang,
        component_version: data.component_version,
        selected_order: data.selected_order,
        is_checked: data.is_checked,
        is_disabled: data.is_disabled,
        compat_kind: data.compat_kind,
        compat_role: data.compat_role,
        compat_code: data.compat_code,
        disabled_reason: data.disabled_reason,
        compat_source: data.compat_source,
        compat_related_target: data.compat_related_target,
        compat_graph: data.compat_graph,
        compat_evidence: data.compat_evidence,
        compat_component_block: data.compat_component_block,
        raw_line: data.raw_line,
        tp_file: data.tp_file,
        tp2_folder: data.tp2_folder,
        tp2_path: data.tp2_path,
        ini_path: data.ini_path,
        readme_path: data.readme_path,
        web_url: data.web_url,
        package_installed_source_name: data.package_installed_source_name,
        package_source_status: data.package_source_status,
        package_source_name: data.package_source_name,
        package_latest_version: data.package_latest_version,
        package_source_url: data.package_source_url,
        package_source_github: data.package_source_github,
        package_update_locked: data.package_update_locked,
        package_can_check_updates: data.package_can_check_updates,
    }
}
