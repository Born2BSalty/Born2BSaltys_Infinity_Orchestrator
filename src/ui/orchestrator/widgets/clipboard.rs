// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

/// Maximum number of pending toast messages that can be queued.
///
/// Both `BIO` (orchestrator) and `BIO_legacy` call `copy`; only `BIO` drains the
/// queue each frame. The cap prevents unbounded growth in `BIO_legacy`, which has
/// no drain and no toast UI.
const TOAST_QUEUE_CAP: usize = 8;

/// Key under which the pending toast queue is stored in egui temp memory.
fn queue_id() -> egui::Id {
    egui::Id::new("clipboard_pending_toasts")
}

fn enqueue_toast(ctx: &egui::Context, message: String) {
    ctx.memory_mut(|m| {
        let queue = m.data.get_temp_mut_or_default::<Vec<String>>(queue_id());
        if queue.len() < TOAST_QUEUE_CAP {
            queue.push(message);
        }
    });
}

/// Writes `text` to the system clipboard and enqueues a default `"Copied to clipboard"` toast.
///
/// This is the shared chokepoint called by all clipboard write sites. The toast is
/// surfaced by the orchestrator's per-frame drain; in `BIO_legacy` (no drain) the
/// bounded queue absorbs it harmlessly.
pub fn copy(ctx: &egui::Context, text: impl Into<String>) {
    ctx.copy_text(text.into());
    enqueue_toast(ctx, "Copied to clipboard".to_string());
}

/// Writes `text` to the system clipboard without enqueuing a toast.
///
/// Use at sites that render their own inline copy confirmation.
pub fn copy_silent(ctx: &egui::Context, text: impl Into<String>) {
    ctx.copy_text(text.into());
}

/// Writes `text` to the system clipboard and enqueues a toast with the given `message`.
///
/// Use when the copy site needs a more specific confirmation than the default.
pub fn copy_with_message(ctx: &egui::Context, text: impl Into<String>, message: impl Into<String>) {
    ctx.copy_text(text.into());
    enqueue_toast(ctx, message.into());
}

/// Drains and returns all pending toast messages enqueued since the last call.
///
/// Call once per frame in the redesign app loop to forward messages to `NotificationManager`.
#[must_use]
pub fn take_pending_toasts(ctx: &egui::Context) -> Vec<String> {
    ctx.memory_mut(|m| {
        m.data
            .get_temp_mut_or_default::<Vec<String>>(queue_id())
            .drain(..)
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::{copy, copy_silent, copy_with_message, take_pending_toasts};

    fn has_copy_text(output: &egui::FullOutput, expected: &str) -> bool {
        output
            .platform_output
            .commands
            .iter()
            .any(|cmd| *cmd == egui::OutputCommand::CopyText(expected.to_string()))
    }

    #[test]
    fn copy_emits_copy_text_command() {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            copy(ctx, "SOME-TEXT");
        });
        assert!(
            has_copy_text(&output, "SOME-TEXT"),
            "expected CopyText(\"SOME-TEXT\") in platform output commands"
        );
    }

    #[test]
    fn copy_enqueues_default_toast() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            copy(ctx, "TEXT");
        });
        let toasts = take_pending_toasts(&ctx);
        assert_eq!(toasts.len(), 1, "copy must enqueue exactly one toast");
        assert_eq!(
            toasts[0], "Copied to clipboard",
            "copy must enqueue the default message"
        );
    }

    #[test]
    fn copy_silent_emits_copy_text_command() {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            copy_silent(ctx, "SILENT-TEXT");
        });
        assert!(
            has_copy_text(&output, "SILENT-TEXT"),
            "expected CopyText(\"SILENT-TEXT\") in platform output commands"
        );
    }

    #[test]
    fn copy_silent_enqueues_nothing() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            copy_silent(ctx, "TEXT");
        });
        let toasts = take_pending_toasts(&ctx);
        assert!(toasts.is_empty(), "copy_silent must not enqueue any toast");
    }

    #[test]
    fn copy_with_message_emits_copy_text_command() {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            copy_with_message(ctx, "MSG-TEXT", "Custom message");
        });
        assert!(
            has_copy_text(&output, "MSG-TEXT"),
            "expected CopyText(\"MSG-TEXT\") in platform output commands"
        );
    }

    #[test]
    fn copy_with_message_enqueues_given_message() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            copy_with_message(ctx, "TEXT", "Copied import code for \"My List\"");
        });
        let toasts = take_pending_toasts(&ctx);
        assert_eq!(
            toasts.len(),
            1,
            "copy_with_message must enqueue exactly one toast"
        );
        assert_eq!(
            toasts[0], "Copied import code for \"My List\"",
            "copy_with_message must enqueue the provided message"
        );
    }

    #[test]
    fn take_pending_toasts_drains_the_queue() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            copy(ctx, "A");
            copy(ctx, "B");
        });
        let first = take_pending_toasts(&ctx);
        assert_eq!(
            first.len(),
            2,
            "both toasts must be returned on first drain"
        );
        let second = take_pending_toasts(&ctx);
        assert!(second.is_empty(), "queue must be empty after first drain");
    }

    #[test]
    fn cap_bounds_queue_at_eight() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            for i in 0..12_usize {
                copy(ctx, format!("text-{i}"));
            }
        });
        let toasts = take_pending_toasts(&ctx);
        assert_eq!(
            toasts.len(),
            8,
            "queue must be capped at TOAST_QUEUE_CAP (8)"
        );
    }
}
