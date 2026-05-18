// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `format_relative` — humanized "time ago" formatter shared across redesign
// surfaces.
//
// Per Phase 5 file inventory: lives under `src/ui/shared/` (not
// `src/ui/home/`) because other surfaces (Workspace header, Step 5 success
// banner) also format relative times.
//
// Output buckets (matching the wireframe's card meta strings — e.g.
// "2 hours ago", "yesterday", "last week", "last month"):
//   - < 1 minute      → "just now"
//   - < 1 hour        → "<N> minute(s) ago"
//   - < 1 day         → "<N> hour(s) ago"
//   - exactly 1 day   → "yesterday"
//   - < 7 days        → "<N> days ago"
//   - < 14 days       → "last week"
//   - < ~30 days      → "<N> weeks ago"
//   - < ~60 days      → "last month"
//   - >= ~60 days     → "<N> months ago"
//   - future/clock-skew → "just now" (never render a negative delta)
//
// SPEC: §3.2 (card meta lines), §3.1 (Home).

// rationale: `#[must_use]` on trivial formatting helpers is churn (Cat 3).
#![allow(clippy::must_use_candidate)]

use std::time::Duration;

use chrono::{DateTime, Utc};

/// Average days per month used purely for the coarse "months ago" bucket.
const DAYS_PER_MONTH: i64 = 30;

/// Format `ts` relative to *now* (`Utc::now()`).
pub fn relative_time(ts: DateTime<Utc>) -> String {
    relative_time_from(ts, Utc::now())
}

/// Format `ts` relative to an explicit `now` — the testable core.
pub fn relative_time_from(ts: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let delta = now.signed_duration_since(ts);
    // Clock skew / a future timestamp → clamp to "just now" rather than
    // rendering "-3 minutes ago".
    let secs = delta.num_seconds();
    if secs < 60 {
        return "just now".to_string();
    }

    let minutes = delta.num_minutes();
    if minutes < 60 {
        return plural(minutes, "minute");
    }

    let hours = delta.num_hours();
    if hours < 24 {
        return plural(hours, "hour");
    }

    let days = delta.num_days();
    if days == 1 {
        return "yesterday".to_string();
    }
    if days < 7 {
        return format!("{days} days ago");
    }
    if days < 14 {
        return "last week".to_string();
    }
    if days < DAYS_PER_MONTH {
        let weeks = days / 7;
        return plural(weeks, "week");
    }
    if days < DAYS_PER_MONTH * 2 {
        return "last month".to_string();
    }

    let months = days / DAYS_PER_MONTH;
    plural(months, "month")
}

/// `1 minute ago` vs `5 minutes ago`.
fn plural(n: i64, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{n} {unit}s ago")
    }
}

/// Format an install **run duration** as the wireframe's `<MM:SS>` clock
/// (per the Phase-7 plan P7.T4 / SPEC §9.2 success banner `ran <MM:SS> ·
/// finished <relative>`). Lives here per L9 (the same `src/ui/shared/`
/// file as `relative_time`, NOT a duplicate of Run-2's
/// `shell_statusbar::format_elapsed` — that is a separate, not-refactored
/// statusbar helper; any dedupe is a propose-only follow-up).
///
/// Buckets (plan P7.T4 — `<MM:SS>` for runs < 60 minutes; `<H:MM:SS>` for
/// runs ≥ 60 minutes):
///   - `< 60 min`  → `M:SS`  (minutes NOT zero-padded — wireframe
///     `ran 4:12`, `screens.jsx:3234`; seconds always 2-digit)
///   - `>= 60 min` → `H:MM:SS` (hours NOT zero-padded; minutes + seconds
///     2-digit)
///
/// Sub-second precision is discarded (whole seconds — an install run is
/// minutes-scale; the wireframe shows whole seconds).
pub fn format_install_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours == 0 {
        // `<MM:SS>` — minutes un-padded (wireframe `4:12`), seconds 2-digit.
        format!("{minutes}:{seconds:02}")
    } else {
        // `<H:MM:SS>` — hours un-padded, minutes + seconds 2-digit.
        format!("{hours}:{minutes:02}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn just_now_for_sub_minute() {
        let n = now();
        assert_eq!(relative_time_from(n, n), "just now");
        assert_eq!(relative_time_from(n - Duration::seconds(30), n), "just now");
    }

    #[test]
    fn future_timestamp_clamps_to_just_now() {
        let n = now();
        // Clock skew: ts is 5 minutes in the future.
        assert_eq!(relative_time_from(n + Duration::minutes(5), n), "just now");
    }

    #[test]
    fn minutes_bucket_singular_and_plural() {
        let n = now();
        assert_eq!(
            relative_time_from(n - Duration::minutes(1), n),
            "1 minute ago"
        );
        assert_eq!(
            relative_time_from(n - Duration::minutes(45), n),
            "45 minutes ago"
        );
    }

    #[test]
    fn hours_bucket() {
        let n = now();
        assert_eq!(relative_time_from(n - Duration::hours(1), n), "1 hour ago");
        assert_eq!(relative_time_from(n - Duration::hours(2), n), "2 hours ago");
        assert_eq!(
            relative_time_from(n - Duration::hours(23), n),
            "23 hours ago"
        );
    }

    #[test]
    fn yesterday_and_days() {
        let n = now();
        assert_eq!(relative_time_from(n - Duration::hours(25), n), "yesterday");
        assert_eq!(relative_time_from(n - Duration::days(3), n), "3 days ago");
        assert_eq!(relative_time_from(n - Duration::days(6), n), "6 days ago");
    }

    #[test]
    fn weeks_and_months() {
        let n = now();
        assert_eq!(relative_time_from(n - Duration::days(8), n), "last week");
        assert_eq!(relative_time_from(n - Duration::days(20), n), "2 weeks ago");
        assert_eq!(relative_time_from(n - Duration::days(35), n), "last month");
        assert_eq!(
            relative_time_from(n - Duration::days(75), n),
            "2 months ago"
        );
    }

    #[test]
    fn install_duration_under_an_hour_is_m_ss() {
        use std::time::Duration as StdDuration;
        // Wireframe `ran 4:12` (`screens.jsx:3234`): minutes un-padded,
        // seconds always 2-digit.
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(4 * 60 + 12)),
            "4:12"
        );
        // Sub-minute: 0:SS.
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(7)),
            "0:07"
        );
        // Zero.
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(0)),
            "0:00"
        );
        // 59:59 is still the M:SS bucket (< 60 min).
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(59 * 60 + 59)),
            "59:59"
        );
        // Sub-second precision is discarded (whole seconds).
        assert_eq!(
            super::format_install_duration(StdDuration::from_millis(12_900)),
            "0:12"
        );
    }

    #[test]
    fn install_duration_at_or_over_an_hour_is_h_mm_ss() {
        use std::time::Duration as StdDuration;
        // Exactly 60 min flips to the H:MM:SS bucket (plan P7.T4: `>= 60
        // minutes`).
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(60 * 60)),
            "1:00:00"
        );
        // 1h 02m 09s — hours un-padded, minutes + seconds 2-digit.
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(3600 + 2 * 60 + 9)),
            "1:02:09"
        );
        // Multi-hour EET install.
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(3 * 3600 + 45 * 60 + 5)),
            "3:45:05"
        );
    }
}
