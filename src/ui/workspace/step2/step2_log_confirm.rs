// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::ConfirmDialog;

const fn upper_tab(bgee: bool) -> &'static str {
    if bgee { "BGEE" } else { "BG2EE" }
}

#[must_use]
pub fn weidu_log_dialog_text(bgee: bool) -> (String, String) {
    let tab = upper_tab(bgee);
    let title = format!("Replace {tab} selections from a WeiDU log?");
    let body = format!(
        "This will overwrite every component selection on the {tab} bucket \
         with the contents of the chosen weidu.log. Make sure the log was \
         produced from the same mod versions you have downloaded — otherwise \
         components may resolve to the wrong rows or fail to install."
    );
    (title, body)
}

#[must_use]
pub const fn weidu_log_confirm<'a>(title: &'a str, body: &'a str) -> ConfirmDialog<'a> {
    ConfirmDialog {
        id_salt: "step2_select_via_weidu_log",
        title,
        body,
        confirm_label: "Pick a weidu.log...",
        danger: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_names_the_target_tab() {
        let (t, _) = weidu_log_dialog_text(true);
        assert_eq!(t, "Replace BGEE selections from a WeiDU log?");
        let (t2, _) = weidu_log_dialog_text(false);
        assert_eq!(t2, "Replace BG2EE selections from a WeiDU log?");
    }

    #[test]
    fn body_is_wireframe_verbatim() {
        let (_, b) = weidu_log_dialog_text(true);
        assert_eq!(
            b,
            "This will overwrite every component selection on the BGEE \
             bucket with the contents of the chosen weidu.log. Make sure \
             the log was produced from the same mod versions you have \
             downloaded — otherwise components may resolve to the wrong \
             rows or fail to install."
        );
        let (_, b2) = weidu_log_dialog_text(false);
        assert!(b2.contains("on the BG2EE bucket"));
    }

    #[test]
    fn confirm_descriptor_is_danger_with_wireframe_label() {
        let (t, b) = weidu_log_dialog_text(true);
        let d = weidu_log_confirm(&t, &b);
        assert!(d.danger);
        assert_eq!(d.confirm_label, "Pick a weidu.log...");
        assert_eq!(d.id_salt, "step2_select_via_weidu_log");
    }
}
