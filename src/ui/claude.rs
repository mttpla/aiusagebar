use chrono::Local;
use tray_icon::menu::{Menu, MenuId, MenuItem};
use crate::provider::{ProviderKind, UsageState};

pub(crate) fn header_label(name: &str, state: &UsageState) -> String {
    match state {
        UsageState::Ok(_, Some(p)) => format!("{} — {}", name, p),
        UsageState::Ok(_, None) => format!("{} — account unavailable", name),
        UsageState::Stale(msg) => format!("{} ⚠  {}", name, msg),
        UsageState::Error(msg) => format!("{} ✕  {}", name, msg),
        UsageState::NotConfigured => format!("{} — not signed in · Setup…", name),
    }
}

pub(crate) fn pct_label(pct: Option<f32>) -> String {
    pct.map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string())
}

/// Returns the number of NSMenu items that `append_claude_section` will append:
/// 1 header + 1 per window when `UsageState::Ok`.
pub(crate) fn section_item_count(state: &UsageState) -> usize {
    match state {
        UsageState::Ok(windows, _) => 1 + windows.len(),
        _ => 1,
    }
}

pub(crate) fn append_claude_section(menu: &Menu, state: &UsageState) -> Option<MenuId> {
    if let UsageState::NotConfigured = state {
        let item = MenuItem::new(
            header_label(ProviderKind::Claude.display_name(), state),
            true,
            None,
        );
        let id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        return Some(id);
    }
    super::append_label(menu, header_label(ProviderKind::Claude.display_name(), state));
    if let UsageState::Ok(windows, _) = state {
        let now = Local::now();
        for w in windows {
            let reset = w
                .resets_at
                .as_deref()
                .map(|s| super::time::format_reset_local(s, now))
                .unwrap_or_else(|| "?".to_string());
            super::append_label(
                menu,
                format!("  {} — {}  resets {}", w.name, pct_label(w.percent_used), reset),
            );
        }
    }
    None
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
            "Claude — not signed in · Setup…"
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

}
