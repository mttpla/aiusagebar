use crate::provider::UsageState;
use crate::settings::DEFAULT_ALERT_THRESHOLD_PCT;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconKind {
    Normal,
    Alert,
    Unavailable,
}

impl IconKind {
    pub fn for_state(state: &UsageState) -> Self {
        match state {
            UsageState::Ok(windows) => {
                if windows.iter().any(|w| w.percent_used.unwrap_or(0.0) >= DEFAULT_ALERT_THRESHOLD_PCT) {
                    IconKind::Alert
                } else {
                    IconKind::Normal
                }
            }
            _ => IconKind::Unavailable,
        }
    }

    pub fn for_providers(states: &[&UsageState]) -> Self {
        states.iter().fold(IconKind::Normal, |best, s| {
            match (best, IconKind::for_state(s)) {
                (IconKind::Alert, _) | (_, IconKind::Alert) => IconKind::Alert,
                (IconKind::Unavailable, _) | (_, IconKind::Unavailable) => IconKind::Unavailable,
                _ => IconKind::Normal,
            }
        })
    }
}

static ICON_NORMAL_PNG: &[u8] = include_bytes!("../icons/brain_normal.png");
static ICON_ALERT_PNG: &[u8] = include_bytes!("../icons/brain_alert.png");
static ICON_UNAVAILABLE_PNG: &[u8] = include_bytes!("../icons/brain_unavailable.png");

pub struct Icons {
    normal: tray_icon::Icon,
    alert: tray_icon::Icon,
    unavailable: tray_icon::Icon,
}

impl Icons {
    pub fn load() -> Self {
        Self {
            normal: parse(ICON_NORMAL_PNG),
            alert: parse(ICON_ALERT_PNG),
            unavailable: parse(ICON_UNAVAILABLE_PNG),
        }
    }

    pub fn get(&self, kind: IconKind) -> tray_icon::Icon {
        match kind {
            IconKind::Normal => self.normal.clone(),
            IconKind::Alert => self.alert.clone(),
            IconKind::Unavailable => self.unavailable.clone(),
        }
    }
}

fn parse(bytes: &[u8]) -> tray_icon::Icon {
    let img = image::load_from_memory(bytes)
        .expect("failed to decode icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("failed to create icon")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LimitWindow;

    fn window(pct: Option<f32>) -> LimitWindow {
        LimitWindow {
            name: "t".into(),
            percent_used: pct,
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        }
    }

    #[test]
    fn normal_under_threshold() {
        let s = UsageState::Ok(vec![window(Some(79.9))]);
        assert_eq!(IconKind::for_state(&s), IconKind::Normal);
    }

    #[test]
    fn alert_at_threshold() {
        let s = UsageState::Ok(vec![window(Some(80.0))]);
        assert_eq!(IconKind::for_state(&s), IconKind::Alert);
    }

    #[test]
    fn unavailable_on_error() {
        assert_eq!(IconKind::for_state(&UsageState::Error("x".into())), IconKind::Unavailable);
    }

    #[test]
    fn unavailable_on_stale() {
        assert_eq!(IconKind::for_state(&UsageState::Stale("x".into())), IconKind::Unavailable);
    }

    #[test]
    fn unavailable_on_not_configured() {
        assert_eq!(IconKind::for_state(&UsageState::NotConfigured), IconKind::Unavailable);
    }

    #[test]
    fn normal_when_percent_unknown() {
        let s = UsageState::Ok(vec![window(None)]);
        assert_eq!(IconKind::for_state(&s), IconKind::Normal);
    }

    #[test]
    fn alert_when_any_window_above_threshold() {
        let s = UsageState::Ok(vec![window(Some(50.0)), window(Some(90.0))]);
        assert_eq!(IconKind::for_state(&s), IconKind::Alert);
    }

    #[test]
    fn alert_ignores_none_with_high_other() {
        let s = UsageState::Ok(vec![window(None), window(Some(85.0))]);
        assert_eq!(IconKind::for_state(&s), IconKind::Alert);
    }

    #[test]
    fn normal_when_all_windows_none() {
        let s = UsageState::Ok(vec![window(None), window(None)]);
        assert_eq!(IconKind::for_state(&s), IconKind::Normal);
    }

    #[test]
    fn fold_alert_beats_error() {
        let high = UsageState::Ok(vec![window(Some(90.0))]);
        let err = UsageState::Error("boom".into());
        assert_eq!(IconKind::for_providers(&[&high, &err]), IconKind::Alert);
    }

    #[test]
    fn fold_alert_beats_unavailable_regardless_of_order() {
        let high = UsageState::Ok(vec![window(Some(85.0))]);
        let stale = UsageState::Stale("old".into());
        assert_eq!(IconKind::for_providers(&[&stale, &high]), IconKind::Alert);
    }

    #[test]
    fn fold_error_beats_normal() {
        let ok = UsageState::Ok(vec![window(Some(50.0))]);
        let err = UsageState::Error("boom".into());
        assert_eq!(IconKind::for_providers(&[&ok, &err]), IconKind::Unavailable);
    }

    #[test]
    fn fold_all_normal() {
        let a = UsageState::Ok(vec![window(Some(10.0))]);
        let b = UsageState::Ok(vec![window(Some(30.0))]);
        assert_eq!(IconKind::for_providers(&[&a, &b]), IconKind::Normal);
    }

    #[test]
    fn fold_empty_is_normal() {
        assert_eq!(IconKind::for_providers(&[]), IconKind::Normal);
    }

    #[test]
    fn alert_when_percent_high_regardless_of_unlimited_flag() {
        let w = LimitWindow {
            name: "t".into(),
            percent_used: Some(90.0),
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: true,
        };
        let s = UsageState::Ok(vec![w]);
        assert_eq!(IconKind::for_state(&s), IconKind::Alert);
    }
}
