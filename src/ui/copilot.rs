use tray_icon::menu::Menu;
use crate::provider::{LimitWindow, ProviderKind, UsageState};

pub(crate) fn header_label(name: &str, state: &UsageState) -> String {
    match state {
        UsageState::Ok(_, Some(p)) => format!("{} — {}", name, p),
        UsageState::Ok(_, None) => format!("{} — account unavailable", name),
        UsageState::Stale(msg) => format!("{} ⚠  {}", name, msg),
        UsageState::Error(msg) => format!("{} ✕  {}", name, msg),
        UsageState::NotConfigured => format!("{}: not configured", name),
    }
}

pub(crate) fn row_label(window: &LimitWindow) -> String {
    let pct = window
        .percent_used
        .map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string());
    let reset = window.resets_at.as_deref().unwrap_or("?");
    format!("  {} — {}  resets {}", window.name, pct, reset)
}

/// Returns the number of NSMenu items that `append_copilot_section` will append:
/// 1 header + 1 per window when `UsageState::Ok`.
pub(crate) fn section_item_count(state: &UsageState) -> usize {
    match state {
        UsageState::Ok(windows, _) => 1 + windows.len(),
        _ => 1,
    }
}

pub(crate) fn append_copilot_section(menu: &Menu, state: &UsageState) -> usize {
    super::append_label(menu, header_label(ProviderKind::Copilot.display_name(), state));
    let mut count = 1usize;
    if let UsageState::Ok(windows, _) = state {
        for w in windows {
            super::append_label(menu, row_label(w));
            count += 1;
        }
    }
    count
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

    #[test]
    fn row_with_pct_and_reset() {
        let w = make_window("Daily", Some(42.5), Some("2026-06-13 00:00"));
        assert_eq!(row_label(&w), "  Daily — 42.5%  resets 2026-06-13 00:00");
    }

    #[test]
    fn row_no_pct_no_reset() {
        let w = make_window("Daily", None, None);
        assert_eq!(row_label(&w), "  Daily — —  resets ?");
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
        assert_eq!(section_item_count(&UsageState::NotConfigured), 1);
    }
}
