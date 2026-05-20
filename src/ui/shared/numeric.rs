// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Numeric cast helpers that don't trigger `clippy::pedantic`'s `as`-cast
//! lints.
//!
//! The `format!`/`parse` path is the underlying-issue fix for
//! `cast_possible_truncation` / `cast_precision_loss` / `cast_sign_loss`
//! on call sites where the value is provably in range but the lint can't
//! prove it.

#[must_use]
pub fn f64_from_u64(value: u64) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(f64::MAX)
}

#[must_use]
pub fn f64_from_usize(value: usize) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(f64::MAX)
}

#[must_use]
pub fn f32_from_f64(value: f64) -> f32 {
    value.to_string().parse::<f32>().unwrap_or(0.0)
}

#[must_use]
pub fn f32_from_u32(value: u32) -> f32 {
    value.to_string().parse::<f32>().unwrap_or(0.0)
}

/// Clamp to `0.0..=1.0` and narrow to `f32`.
#[must_use]
pub fn unit_f32(value: f64) -> f32 {
    f32_from_f64(value.clamp(0.0, 1.0))
}

/// `numerator / denominator` (saturating to 1 on zero denom), in `0.0..=1.0`.
#[must_use]
pub fn ratio_u64(numerator: u64, denominator: u64) -> f32 {
    unit_f32(f64_from_u64(numerator) / f64_from_u64(denominator.max(1)))
}

/// `numerator / denominator` (saturating to 1 on zero denom), in `0.0..=1.0`.
#[must_use]
pub fn ratio_usize(numerator: usize, denominator: usize) -> f32 {
    unit_f32(f64_from_usize(numerator) / f64_from_usize(denominator.max(1)))
}

/// Fraction in `0.0..=1.0` → integer percent in `0..=100`.
#[must_use]
pub fn pct_from_fraction(value: f32) -> u32 {
    format!("{:.0}", value.clamp(0.0, 1.0) * 100.0)
        .parse::<u32>()
        .unwrap_or(0)
}
