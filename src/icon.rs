use crate::provider::UsageState;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum IconKind {
    Normal,
    Alert,
    Unavailable,
}

impl IconKind {
    pub(crate) fn for_state(state: &UsageState, threshold: f32) -> Self {
        match state {
            UsageState::Ok(windows, _) => {
                if windows.iter().any(|w| w.percent_used.unwrap_or(0.0) >= threshold) {
                    IconKind::Alert
                } else {
                    IconKind::Normal
                }
            }
            _ => IconKind::Unavailable,
        }
    }

    pub(crate) fn for_providers(states: &[&UsageState], threshold: f32) -> Self {
        states.iter().fold(IconKind::Normal, |best, s| {
            match (best, IconKind::for_state(s, threshold)) {
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

pub(crate) struct Icons {
    normal: tray_icon::Icon,
    alert: tray_icon::Icon,
    unavailable: tray_icon::Icon,
}

impl Icons {
    pub(crate) fn load() -> Self {
        Self {
            normal: parse(ICON_NORMAL_PNG),
            alert: parse(ICON_ALERT_PNG),
            unavailable: parse(ICON_UNAVAILABLE_PNG),
        }
    }

    pub(crate) fn get(&self, kind: IconKind) -> tray_icon::Icon {
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
        let s = UsageState::Ok(vec![window(Some(79.9))], None);
        assert_eq!(IconKind::for_state(&s, 80.0), IconKind::Normal);
    }

    #[test]
    fn alert_at_threshold() {
        let s = UsageState::Ok(vec![window(Some(80.0))], None);
        assert_eq!(IconKind::for_state(&s, 80.0), IconKind::Alert);
    }

    #[test]
    fn unavailable_on_error() {
        assert_eq!(IconKind::for_state(&UsageState::Error("x".into()), 80.0), IconKind::Unavailable);
    }

    #[test]
    fn unavailable_on_stale() {
        assert_eq!(IconKind::for_state(&UsageState::Stale("x".into()), 80.0), IconKind::Unavailable);
    }

    #[test]
    fn unavailable_on_not_configured() {
        assert_eq!(IconKind::for_state(&UsageState::NotConfigured, 80.0), IconKind::Unavailable);
    }

    #[test]
    fn normal_when_percent_unknown() {
        let s = UsageState::Ok(vec![window(None)], None);
        assert_eq!(IconKind::for_state(&s, 80.0), IconKind::Normal);
    }

    #[test]
    fn alert_when_any_window_above_threshold() {
        let s = UsageState::Ok(vec![window(Some(50.0)), window(Some(90.0))], None);
        assert_eq!(IconKind::for_state(&s, 80.0), IconKind::Alert);
    }

    #[test]
    fn alert_ignores_none_with_high_other() {
        let s = UsageState::Ok(vec![window(None), window(Some(85.0))], None);
        assert_eq!(IconKind::for_state(&s, 80.0), IconKind::Alert);
    }

    #[test]
    fn normal_when_all_windows_none() {
        let s = UsageState::Ok(vec![window(None), window(None)], None);
        assert_eq!(IconKind::for_state(&s, 80.0), IconKind::Normal);
    }

    #[test]
    fn fold_alert_beats_error() {
        let high = UsageState::Ok(vec![window(Some(90.0))], None);
        let err = UsageState::Error("boom".into());
        assert_eq!(IconKind::for_providers(&[&high, &err], 80.0), IconKind::Alert);
    }

    #[test]
    fn fold_alert_beats_unavailable_regardless_of_order() {
        let high = UsageState::Ok(vec![window(Some(85.0))], None);
        let stale = UsageState::Stale("old".into());
        assert_eq!(IconKind::for_providers(&[&stale, &high], 80.0), IconKind::Alert);
    }

    #[test]
    fn fold_error_beats_normal() {
        let ok = UsageState::Ok(vec![window(Some(50.0))], None);
        let err = UsageState::Error("boom".into());
        assert_eq!(IconKind::for_providers(&[&ok, &err], 80.0), IconKind::Unavailable);
    }

    #[test]
    fn fold_all_normal() {
        let a = UsageState::Ok(vec![window(Some(10.0))], None);
        let b = UsageState::Ok(vec![window(Some(30.0))], None);
        assert_eq!(IconKind::for_providers(&[&a, &b], 80.0), IconKind::Normal);
    }

    #[test]
    fn fold_empty_is_normal() {
        assert_eq!(IconKind::for_providers(&[], 80.0), IconKind::Normal);
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
        let s = UsageState::Ok(vec![w], None);
        assert_eq!(IconKind::for_state(&s, 80.0), IconKind::Alert);
    }
}
