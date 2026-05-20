// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Duration;

use chrono::{DateTime, Utc};

const DAYS_PER_MONTH: i64 = 30;

#[must_use]
pub fn relative_time(ts: DateTime<Utc>) -> String {
    relative_time_from(ts, Utc::now())
}

#[must_use]
pub fn relative_time_from(ts: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let delta = now.signed_duration_since(ts);
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

fn plural(n: i64, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{n} {unit}s ago")
    }
}

#[must_use]
pub fn format_install_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours == 0 {
        format!("{minutes}:{seconds:02}")
    } else {
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
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(4 * 60 + 12)),
            "4:12"
        );
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(7)),
            "0:07"
        );
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(0)),
            "0:00"
        );
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(59 * 60 + 59)),
            "59:59"
        );
        assert_eq!(
            super::format_install_duration(StdDuration::from_millis(12_900)),
            "0:12"
        );
    }

    #[test]
    fn install_duration_at_or_over_an_hour_is_h_mm_ss() {
        use std::time::Duration as StdDuration;
        assert_eq!(
            super::format_install_duration(StdDuration::from_hours(1)),
            "1:00:00"
        );
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(3600 + 2 * 60 + 9)),
            "1:02:09"
        );
        assert_eq!(
            super::format_install_duration(StdDuration::from_secs(3 * 3600 + 45 * 60 + 5)),
            "3:45:05"
        );
    }
}
