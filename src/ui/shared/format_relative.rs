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
}
