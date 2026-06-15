use chrono::{DateTime, Local};

pub(crate) fn format_reset_local(iso_utc: &str, now: DateTime<Local>) -> String {
    match DateTime::parse_from_rfc3339(iso_utc) {
        Ok(dt) => {
            let local = dt.with_timezone(&Local);
            if local.date_naive() == now.date_naive() {
                local.format("%H:%M").to_string()
            } else {
                local.format("%Y-%m-%d %H:%M").to_string()
            }
        }
        Err(_) => iso_utc.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_from_utc(rfc3339: &str) -> DateTime<Local> {
        DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&Local)
    }

    #[test]
    fn same_day_returns_hhmm_shape() {
        let now = now_from_utc("2026-06-13T12:30:00Z");
        let result = format_reset_local("2026-06-13T12:30:00Z", now);
        assert!(
            chrono::NaiveTime::parse_from_str(&result, "%H:%M").is_ok(),
            "expected HH:MM, got '{result}'"
        );
    }

    #[test]
    fn different_day_returns_datetime_shape() {
        let now = now_from_utc("2026-05-14T12:30:00Z");
        let result = format_reset_local("2026-06-13T12:30:00Z", now);
        assert!(
            chrono::NaiveDateTime::parse_from_str(&result, "%Y-%m-%d %H:%M").is_ok(),
            "expected YYYY-MM-DD HH:MM, got '{result}'"
        );
    }

    #[test]
    fn midnight_cross_valid_shape() {
        let now = now_from_utc("2026-06-13T10:00:00Z");
        let result = format_reset_local("2026-06-13T23:30:00Z", now);
        let valid = chrono::NaiveTime::parse_from_str(&result, "%H:%M").is_ok()
            || chrono::NaiveDateTime::parse_from_str(&result, "%Y-%m-%d %H:%M").is_ok();
        assert!(valid, "unexpected format: '{result}'");
    }

    #[test]
    fn malformed_passthrough() {
        let now = now_from_utc("2026-06-13T10:00:00Z");
        assert_eq!(format_reset_local("not-a-date", now), "not-a-date");
    }
}
