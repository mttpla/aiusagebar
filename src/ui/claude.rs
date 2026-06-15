use chrono::{DateTime, Local};
use tray_icon::menu::Menu;
use crate::provider::{ProviderKind, UsageState};

pub(crate) fn header_label(name: &str, state: &UsageState) -> String {
    match state {
        UsageState::Ok(_, Some(p)) => format!("{} — {}", name, p),
        UsageState::Ok(_, None) => format!("{} — account unavailable", name),
        UsageState::Stale(msg) => format!("{} ⚠  {}", name, msg),
        UsageState::Error(msg) => format!("{} ✕  {}", name, msg),
        UsageState::NotConfigured => format!("{}: not configured", name),
    }
}

pub(crate) fn pct_label(pct: Option<f32>) -> String {
    pct.map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string())
}

fn format_reset_local(iso_utc: &str, now: DateTime<Local>) -> String {
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

/// Returns the number of NSMenu items that `append_claude_section` will append:
/// 1 header + 1 per window when `UsageState::Ok`.
pub(crate) fn section_item_count(state: &UsageState) -> usize {
    match state {
        UsageState::Ok(windows, _) => 1 + windows.len(),
        _ => 1,
    }
}

pub(crate) fn append_claude_section(menu: &Menu, state: &UsageState) -> usize {
    super::append_label(menu, header_label(ProviderKind::Claude.display_name(), state));
    let mut count = 1usize;
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            let reset = w
                .resets_at
                .as_deref()
                .map(|s| format_reset_local(s, now))
                .unwrap_or_else(|| "?".to_string());
            super::append_label(
                menu,
                format!("  {} — {}  resets {}", w.name, pct_label(w.percent_used), reset),
            );
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{LimitWindow, UsageState};

    fn make_ok(profile: Option<&str>) -> UsageState {
        UsageState::Ok(vec![], profile.map(str::to_owned))
    }

    #[test]
    fn header_ok_with_profile() {
        assert_eq!(header_label("Claude", &make_ok(Some("max"))), "Claude — max");
    }

    #[test]
    fn header_ok_no_profile() {
        assert_eq!(header_label("Claude", &make_ok(None)), "Claude — account unavailable");
    }

    #[test]
    fn header_stale() {
        assert_eq!(
            header_label("Claude", &UsageState::Stale("token expired".into())),
            "Claude ⚠  token expired"
        );
    }

    #[test]
    fn header_error() {
        assert_eq!(
            header_label("Claude", &UsageState::Error("network failure".into())),
            "Claude ✕  network failure"
        );
    }

    #[test]
    fn header_not_configured() {
        assert_eq!(
            header_label("Claude", &UsageState::NotConfigured),
            "Claude: not configured"
        );
    }

    #[test]
    fn pct_some() {
        assert_eq!(pct_label(Some(42.5)), "42.5%");
    }

    #[test]
    fn pct_none() {
        assert_eq!(pct_label(None), "—");
    }

    #[test]
    fn append_claude_section_count_ok_two_windows() {
        let state = UsageState::Ok(
            vec![
                LimitWindow { name: "daily".into(), percent_used: Some(50.0), ..Default::default() },
                LimitWindow { name: "monthly".into(), percent_used: Some(20.0), ..Default::default() },
            ],
            Some("max".into()),
        );
        assert_eq!(section_item_count(&state), 3); // 1 header + 2 windows
    }

    #[test]
    fn append_claude_section_count_not_configured() {
        assert_eq!(section_item_count(&UsageState::NotConfigured), 1);
    }

    // ---- format_reset_local ----

    fn now_local_from_utc(rfc3339: &str) -> chrono::DateTime<chrono::Local> {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&chrono::Local)
    }

    #[test]
    fn reset_same_day_returns_hhmm() {
        // now == input instant → guaranteed same local date
        let now = now_local_from_utc("2026-06-13T12:30:00Z");
        let result = format_reset_local("2026-06-13T12:30:00Z", now);
        // shape: HH:MM (exactly 5 chars, no date part)
        assert!(
            chrono::NaiveTime::parse_from_str(&result, "%H:%M").is_ok(),
            "expected HH:MM, got '{}'",
            result
        );
    }

    #[test]
    fn reset_different_day_returns_datetime() {
        // now is 30 days before input → guaranteed different local date
        let now = now_local_from_utc("2026-05-14T12:30:00Z");
        let result = format_reset_local("2026-06-13T12:30:00Z", now);
        // shape: YYYY-MM-DD HH:MM (exactly 16 chars)
        assert!(
            chrono::NaiveDateTime::parse_from_str(&result, "%Y-%m-%d %H:%M").is_ok(),
            "expected YYYY-MM-DD HH:MM, got '{}'",
            result
        );
    }

    #[test]
    fn reset_midnight_cross_valid_shape() {
        // UTC 23:30 on June 13 → local date/time is TZ-dependent
        // assert output is one of the two valid formats (TZ-agnostic)
        let now = now_local_from_utc("2026-06-13T10:00:00Z");
        let result = format_reset_local("2026-06-13T23:30:00Z", now);
        let valid = chrono::NaiveTime::parse_from_str(&result, "%H:%M").is_ok()
            || chrono::NaiveDateTime::parse_from_str(&result, "%Y-%m-%d %H:%M").is_ok();
        assert!(valid, "unexpected format: '{}'", result);
    }

    #[test]
    fn reset_malformed_passthrough() {
        let now = now_local_from_utc("2026-06-13T10:00:00Z");
        assert_eq!(format_reset_local("not-a-date", now), "not-a-date");
    }
}
