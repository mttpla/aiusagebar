use tray_icon::menu::Menu;
use crate::provider::UsageState;

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

pub fn append_claude_section(menu: &Menu, state: &UsageState) {
    super::append_label(menu, header_label("Claude", state));
    if let UsageState::Ok(windows, _) = state {
        for w in windows {
            let reset = w.resets_at.as_deref().unwrap_or("?");
            super::append_label(
                menu,
                format!("  {} — {}  resets {}", w.name, pct_label(w.percent_used), reset),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::UsageState;

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
}
