// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

const fn source(size_px: u32) -> &'static [u8] {
    if size_px <= 64 {
        include_bytes!("../../../../assets/64.png")
    } else if size_px <= 128 {
        include_bytes!("../../../../assets/128.png")
    } else {
        include_bytes!("../../../../assets/256.png")
    }
}

#[must_use]
pub(crate) const fn pixels_for(target: f32) -> u32 {
    if target <= 50.0 {
        50
    } else if target <= 75.0 {
        75
    } else if target <= 100.0 {
        100
    } else if target <= 150.0 {
        150
    } else {
        200
    }
}

pub(crate) fn texture(ctx: &egui::Context, size_px: u32) -> egui::TextureHandle {
    let cache_id = egui::Id::new(("bio_brand_mark_texture", size_px));
    let cached: Option<egui::TextureHandle> = ctx.memory(|m| m.data.get_temp(cache_id));
    if let Some(tex) = cached {
        return tex;
    }
    let decoded = image::load_from_memory(source(size_px)).expect("brand-mark PNG must be valid");
    let scaled = decoded.resize_exact(size_px, size_px, image::imageops::FilterType::Lanczos3);
    let rgba = scaled.to_rgba8();
    let img = egui::ColorImage::from_rgba_unmultiplied(
        [size_px as usize, size_px as usize],
        rgba.as_raw(),
    );
    let tex = ctx.load_texture("bio_brand_mark", img, egui::TextureOptions::LINEAR);
    ctx.memory_mut(|m| m.data.insert_temp(cache_id, tex.clone()));
    tex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_buckets_step_up_with_target_size() {
        assert_eq!(pixels_for(20.0), 50);
        assert_eq!(pixels_for(50.0), 50);
        assert_eq!(pixels_for(51.0), 75);
        assert_eq!(pixels_for(100.0), 100);
        assert_eq!(pixels_for(150.0), 150);
        assert_eq!(pixels_for(400.0), 200);
    }

    #[test]
    fn source_picks_the_smallest_asset_that_covers_the_bucket() {
        assert_eq!(source(50).len(), source(64).len());
        assert_ne!(source(64).len(), source(100).len());
        assert_ne!(source(128).len(), source(200).len());
    }
}
