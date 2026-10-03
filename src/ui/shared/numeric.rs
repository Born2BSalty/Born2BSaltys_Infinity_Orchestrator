// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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

#[must_use]
pub fn unit_f32(value: f64) -> f32 {
    f32_from_f64(value.clamp(0.0, 1.0))
}

#[must_use]
pub fn ratio_u64(numerator: u64, denominator: u64) -> f32 {
    unit_f32(f64_from_u64(numerator) / f64_from_u64(denominator.max(1)))
}

#[must_use]
pub fn ratio_usize(numerator: usize, denominator: usize) -> f32 {
    unit_f32(f64_from_usize(numerator) / f64_from_usize(denominator.max(1)))
}

#[must_use]
pub fn pct_from_fraction(value: f32) -> u32 {
    format!("{:.0}", value.clamp(0.0, 1.0) * 100.0)
        .parse::<u32>()
        .unwrap_or(0)
}

#[must_use]
pub fn u16_from_f32(value: f32) -> Option<u16> {
    if value.is_nan() {
        return None;
    }
    let rounded = value.round();
    format!("{rounded:.0}").parse::<u16>().ok()
}

#[cfg(test)]
mod tests {
    use super::u16_from_f32;

    #[test]
    fn rounds_down_below_half() {
        assert_eq!(u16_from_f32(420.4), Some(420));
    }

    #[test]
    fn rounds_up_at_half_and_above() {
        assert_eq!(u16_from_f32(420.5), Some(421));
        assert_eq!(u16_from_f32(421.5), Some(422));
        assert_eq!(u16_from_f32(420.6), Some(421));
    }

    #[test]
    fn negative_is_none() {
        assert_eq!(u16_from_f32(-1.0), None);
    }

    #[test]
    fn nan_is_none() {
        assert_eq!(u16_from_f32(f32::NAN), None);
    }

    #[test]
    fn above_u16_max_is_none() {
        assert_eq!(u16_from_f32(70000.0), None);
    }
}
