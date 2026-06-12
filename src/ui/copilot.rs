use tray_icon::menu::Menu;
use crate::provider::{LimitWindow, UsageState};

pub(crate) fn row_label(window: &LimitWindow) -> String {
    let pct = window
        .percent_used
        .map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string());
    let reset = window.resets_at.as_deref().unwrap_or("?");
    format!("  {} — {}  resets {}", window.name, pct, reset)
}

pub fn append_copilot_section(menu: &Menu, state: &UsageState) {
    match state {
        UsageState::NotConfigured => {
            super::append_label(menu, "Copilot: not configured");
        }
        UsageState::Stale(msg) => {
            super::append_label(menu, format!("Copilot ⚠  {}", msg));
        }
        UsageState::Error(msg) => {
            super::append_label(menu, format!("Copilot ✕  {}", msg));
        }
        UsageState::Ok(windows, profile) => {
            let header = match profile {
                Some(p) => format!("Copilot — {}", p),
                None => "Copilot — account unavailable".to_string(),
            };
            super::append_label(menu, header);
            for w in windows {
                super::append_label(menu, row_label(w));
            }
        }
    }
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
}
