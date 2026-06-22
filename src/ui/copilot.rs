use chrono::{DateTime, Local};
use tray_icon::menu::{Menu, MenuId, MenuItem};
use crate::provider::{LimitWindow, ProviderKind, UsageState};

pub(crate) fn header_label(name: &str, state: &UsageState) -> String {
    match state {
        UsageState::Ok(_, Some(p)) => format!("{} — {}", name, p),
        UsageState::Ok(_, None) => name.to_string(),
        UsageState::Stale(msg) => format!("{} ⚠  {}", name, msg),
        UsageState::Error(msg) => format!("{} ✕  {}", name, msg),
        UsageState::NotConfigured => format!("{} — not signed in · Setup…", name),
    }
}

pub(crate) fn row_label(window: &LimitWindow, now: DateTime<Local>) -> String {
    let pct = window
        .percent_used
        .map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string());
    let reset = window
        .resets_at
        .as_deref()
        .map(|s| super::time::format_reset_local(s, now))
        .unwrap_or_else(|| "?".to_string());
    format!("  {} — {}  resets {}", window.name, pct, reset)
}

/// Returns the number of NSMenu items that `append_copilot_section` will append:
/// 1 header + 1 per window when `UsageState::Ok`, else 1 header.
pub(crate) fn section_item_count(state: &UsageState) -> usize {
    match state {
        UsageState::Ok(windows, _) => 1 + windows.len(),
        _ => 1,
    }
}

pub(crate) fn append_copilot_section(menu: &Menu, state: &UsageState) -> Option<MenuId> {
    if let UsageState::NotConfigured = state {
        let item = MenuItem::new(
            header_label(ProviderKind::Copilot.display_name(), state),
            true,
            None,
        );
        let setup_id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        return Some(setup_id);
    }
    super::append_label(menu, header_label(ProviderKind::Copilot.display_name(), state));
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            super::append_label(menu, row_label(w, now));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LimitWindow;

    fn make_window(name: &str, pct: Option<f32>, resets_at: Option<&str>) -> LimitWindow {
        LimitWindow {
            name: name.to_owned(),
            percent_used: pct,
            limit: None,
            remaining: None,
            resets_at: resets_at.map(str::to_owned),
            unlimited: false,
        }
    }

    fn now_from_utc(rfc3339: &str) -> DateTime<Local> {
        DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&Local)
    }

    #[test]
    fn row_same_day_reset_shows_hhmm() {
        let now = now_from_utc("2026-06-13T10:00:00Z");
        let w = make_window("Daily", Some(42.5), Some("2026-06-13T12:30:00Z"));
        let label = row_label(&w, now);
        assert!(label.starts_with("  Daily — 42.5%  resets "), "got: {label}");
        let reset_part = label.trim_start_matches("  Daily — 42.5%  resets ");
        assert!(
            chrono::NaiveTime::parse_from_str(reset_part, "%H:%M").is_ok(),
            "expected HH:MM, got '{reset_part}'"
        );
    }

    #[test]
    fn row_different_day_reset_shows_datetime() {
        let now = now_from_utc("2026-05-14T10:00:00Z");
        let w = make_window("Daily", Some(42.5), Some("2026-06-13T12:30:00Z"));
        let label = row_label(&w, now);
        let reset_part = label.trim_start_matches("  Daily — 42.5%  resets ");
        assert!(
            chrono::NaiveDateTime::parse_from_str(reset_part, "%Y-%m-%d %H:%M").is_ok(),
            "expected YYYY-MM-DD HH:MM, got '{reset_part}'"
        );
    }

    #[test]
    fn row_malformed_resets_at_passthrough() {
        let now = now_from_utc("2026-06-13T10:00:00Z");
        let w = make_window("Daily", Some(42.5), Some("not-a-date"));
        assert_eq!(row_label(&w, now), "  Daily — 42.5%  resets not-a-date");
    }

    #[test]
    fn row_no_pct_no_reset() {
        let now = now_from_utc("2026-06-13T10:00:00Z");
        let w = make_window("Daily", None, None);
        assert_eq!(row_label(&w, now), "  Daily — —  resets ?");
    }

    #[test]
    fn append_copilot_section_count_ok_one_window() {
        use crate::provider::UsageState;
        let state = UsageState::Ok(
            vec![make_window("monthly", Some(10.0), None)],
            None,
        );
        assert_eq!(section_item_count(&state), 2); // 1 header + 1 window
    }

    #[test]
    fn append_copilot_section_count_not_configured() {
        use crate::provider::UsageState;
        assert_eq!(section_item_count(&UsageState::NotConfigured), 1); // header only
    }

    #[test]
    fn header_ok_no_profile_shows_name_only() {
        let state = UsageState::Ok(vec![], None);
        assert_eq!(header_label("GitHub Copilot", &state), "GitHub Copilot");
    }

    #[test]
    fn header_not_configured() {
        let state = UsageState::NotConfigured;
        assert_eq!(
            header_label("GitHub Copilot", &state),
            "GitHub Copilot — not signed in · Setup…"
        );
    }
}
