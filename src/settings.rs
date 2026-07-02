use std::time::Duration;

pub(crate) const DEFAULT_POLL_INTERVAL: Duration       = Duration::from_secs(300);
/// Default threshold (percent) that flips the tray icon to its alert state.
/// Icon/notify only — bar fill color uses BAR_WARN_PCT / BAR_ALERT_PCT.
pub(crate) const DEFAULT_ICON_ALERT_PCT: f32           = 80.0;
pub(crate) const DEFAULT_BACKOFF_FACTOR: u32           = 2;
pub(crate) const DEFAULT_BACKOFF_CAP: Duration         = Duration::from_secs(3600);
pub(crate) const DEFAULT_UPDATE_CHECK_INTERVAL_HOURS: i64 = 24;

/// HTTP request timeout for the shared ureq agent.
pub(crate) const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
/// Max messages retained in the in-memory diagnostic log ring buffer.
pub(crate) const DIAG_LOG_MAX_MESSAGES: usize = 100;
/// Progress-bar color zone boundaries (percent). Separate from the icon/notify
/// alert threshold — these drive bar fill color only.
pub(crate) const BAR_WARN_PCT: f32 = 60.0;
pub(crate) const BAR_ALERT_PCT: f32 = 80.0;

pub(crate) struct Settings {
    pub(crate) poll_interval:       Duration,
    pub(crate) icon_alert_pct: f32,
    pub(crate) backoff_factor:      u32,
    pub(crate) backoff_cap:         Duration,
    pub(crate) update_check_interval_hours: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            poll_interval:       DEFAULT_POLL_INTERVAL,
            icon_alert_pct: DEFAULT_ICON_ALERT_PCT,
            backoff_factor:      DEFAULT_BACKOFF_FACTOR,
            backoff_cap:         DEFAULT_BACKOFF_CAP,
            update_check_interval_hours: DEFAULT_UPDATE_CHECK_INTERVAL_HOURS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_poll_interval_is_five_minutes() {
        assert_eq!(Settings::default().poll_interval, Duration::from_secs(300));
    }

    #[test]
    fn default_alert_threshold_is_eighty_percent() {
        assert_eq!(Settings::default().icon_alert_pct, 80.0_f32);
    }

    #[test]
    fn default_backoff_factor_is_two() {
        assert_eq!(Settings::default().backoff_factor, 2);
    }

    #[test]
    fn default_backoff_cap_is_one_hour() {
        assert_eq!(Settings::default().backoff_cap, Duration::from_secs(3600));
    }

    #[test]
    fn default_update_check_interval_is_24_hours() {
        assert_eq!(Settings::default().update_check_interval_hours, 24);
    }
}
