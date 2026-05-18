// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `input` — the shared redesign single-line text-input primitive.
//
// ## The bug this exists to fix (root-cause, once)
//
// Every redesign text input is the wireframe `<input>` with `sketchyBorder`
// (`screens.jsx::sketchyBorder` — 1.5px solid border-strong, 3px radius) and
// internal text padding (the wireframe `padding: 8px 12px` etc.). egui's
// `TextEdit` has no sketchy border, so each call site drew its own:
//
//   let resp = ui.add_sized(size, egui::TextEdit::singleline(v).margin(M)…);
//   ui.painter().rect_stroke(resp.rect, …);   // ← WRONG RECT
//
// `egui::TextEdit::show` (egui 0.31 `text_edit/builder.rs:432-436`) does:
//
//   let outer_rect = output.response.rect;
//   let inner_rect = outer_rect - margin;
//   output.response.rect = inner_rect;        // Response.rect := INNER rect
//
// i.e. the returned `Response::rect` is the **inner galley rect**, already
// shrunk by the `TextEdit`'s `margin` on every side. Stroking `resp.rect`
// therefore drew the border `margin` px inside the box the input actually
// occupies — so every input (modlist name, Settings rows, the destination
// `FolderInput`, the Step-2 search) looked **indented**: the visible field
// box was bigger than its border, with a dead inset all around.
//
// The fix is geometric and lives here once: re-expand the returned inner
// rect by the **same `margin`** to recover the outer (allocated) rect, and
// stroke *that*. The border then hugs the box the input occupies — the
// wireframe `sketchyBorder` look — on every call site that routes through
// this helper. (egui's own TODO at `builder.rs:432` is exactly this: "return
// full outer_rect"; until it does, this helper owns the correction.)
//
// ## Why a shared primitive (not N per-call patches)
//
// The stroke geometry was duplicated inline in `stage_choose` (name +
// destination), `settings/widgets/{value_row,path_row,name_row}`, and
// `step2_search`. Centralizing the *correct* outer-rect stroke here means
// the bug is fixed once and cannot regress per call site; each caller passes
// its existing font / colors / margin / border tone and gets the right
// border for free. This is the "fix the shared primitive" mandate.
//
// SPEC: §1.2 (sketchy borders — 1.5px solid, 3px radius), §12.1 (input-bg).

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// constants — correct by construction (Cat 2); the doc-paragraph-length
// lint is subjective style (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_long_first_doc_paragraph
)]

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
};

/// Options for [`redesign_text_input`]. All visual choices a call site
/// previously hand-wired stay caller-controlled (font, colors, margin,
/// border tone) so this primitive is a drop-in for every existing input
/// without changing any of their looks — only fixing the border rect.
pub struct InputOpts<'a> {
    /// The exact `egui::TextEdit` the caller built (font, hint, text/bg
    /// color, **margin**). The margin is read back off it to recover the
    /// outer rect, so the caller keeps full control of internal text padding.
    pub edit: egui::TextEdit<'a>,
    /// The `margin` the caller set on `edit` (egui's `TextEdit` does not
    /// expose its margin, so it is passed explicitly — it MUST equal the
    /// `.margin(..)` on `edit`, exactly as the call site already knew it).
    pub margin: egui::Margin,
    /// Allocated size handed to `add_sized` (the box the input occupies).
    pub size: egui::Vec2,
    /// Border stroke color. Defaults to `border_strong` when `None`
    /// (`path_row` passes a status-tone color here).
    pub border: Option<egui::Color32>,
}

/// Render a single-line redesign text input with the sketchy border drawn on
/// the **outer (allocated) rect** — the box-hugging border the wireframe
/// `sketchyBorder` specifies, fixing the app-wide indented-input bug (see the
/// module header).
///
/// Returns the `TextEdit`'s `egui::Response` unchanged (its `.rect` is still
/// egui's inner rect — callers that need the inner rect, e.g. to align a
/// sibling under the text column, get the same value they got before; this
/// helper only changes *where the border is painted*, not the response).
pub fn redesign_text_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    opts: InputOpts<'_>,
) -> egui::Response {
    let response = ui.add_sized(opts.size, opts.edit);

    // egui returned the INNER rect (`outer - margin`); re-expand by the same
    // margin to recover the OUTER (allocated) rect and stroke THAT — so the
    // border hugs the box the input occupies, not a margin-inset sub-rect.
    let outer_rect = response.rect + opts.margin;

    let color = opts
        .border
        .unwrap_or_else(|| redesign_border_strong(palette));
    ui.painter().rect_stroke(
        outer_rect,
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, color),
        // `Inside` so the 1.5px stroke is drawn just within the allocated
        // box edge (matching egui's frame-expansion model and the other
        // redesign chrome strokes); the rect is now the correct outer box.
        egui::StrokeKind::Inside,
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point: re-expanding the egui inner rect by the input's
    /// margin recovers the outer rect (`inner + margin == outer`). This is
    /// the inverse of egui `text_edit/builder.rs:435` (`inner = outer -
    /// margin`), so the stroked rect is the box the input occupies.
    #[test]
    fn outer_rect_is_inner_plus_margin() {
        let margin = egui::Margin::symmetric(12, 8);
        let inner = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 14.0));
        let outer = inner + margin;
        // egui's own relation, inverted: outer - margin == inner.
        assert_eq!(outer - margin, inner);
        // The border box is wider/taller than the galley by margin*2 on
        // each axis — i.e. it hugs the allocated box, not the text.
        assert_eq!(outer.width(), inner.width() + 24.0);
        assert_eq!(outer.height(), inner.height() + 16.0);
        // And it starts margin px up-left of the galley (no dead inset).
        assert_eq!(outer.left(), inner.left() - 12.0);
        assert_eq!(outer.top(), inner.top() - 8.0);
    }

    #[test]
    fn asymmetric_margin_recovers_each_side() {
        let margin = egui::Margin {
            left: 8,
            right: 4,
            top: 5,
            bottom: 5,
        };
        let inner = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(50.0, 20.0));
        let outer = inner + margin;
        assert_eq!(outer.left(), -8.0);
        assert_eq!(outer.right(), 54.0);
        assert_eq!(outer.top(), -5.0);
        assert_eq!(outer.bottom(), 25.0);
    }
}
