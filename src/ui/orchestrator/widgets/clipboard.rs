// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

/// Writes `text` to the system clipboard via egui.
///
/// The single chokepoint for all redesign clipboard writes. Callers are
/// responsible for showing their own visible confirmation after calling this.
pub fn copy(ctx: &egui::Context, text: impl Into<String>) {
    ctx.copy_text(text.into());
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::copy;

    #[test]
    fn copy_emits_copy_text_command() {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            copy(ctx, "SOME-TEXT");
        });
        let has_copy = output
            .platform_output
            .commands
            .iter()
            .any(|cmd| *cmd == egui::OutputCommand::CopyText("SOME-TEXT".to_string()));
        assert!(
            has_copy,
            "expected CopyText(\"SOME-TEXT\") in platform output commands"
        );
    }
}
