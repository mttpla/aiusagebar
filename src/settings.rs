use std::time::Duration;

pub(crate) const DEFAULT_POLL_INTERVAL: Duration       = Duration::from_secs(300);
pub(crate) const DEFAULT_ALERT_THRESHOLD_PCT: f32      = 80.0;
pub(crate) const DEFAULT_BACKOFF_FACTOR: u32           = 2;
pub(crate) const DEFAULT_BACKOFF_CAP: Duration         = Duration::from_secs(3600);

pub(crate) struct Settings {
    pub(crate) poll_interval:       Duration,
    pub(crate) alert_threshold_pct: f32,
    pub(crate) backoff_factor:      u32,
    pub(crate) backoff_cap:         Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            poll_interval:       DEFAULT_POLL_INTERVAL,
            alert_threshold_pct: DEFAULT_ALERT_THRESHOLD_PCT,
            backoff_factor:      DEFAULT_BACKOFF_FACTOR,
            backoff_cap:         DEFAULT_BACKOFF_CAP,
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
        assert_eq!(Settings::default().alert_threshold_pct, 80.0_f32);
    }

    #[test]
    fn default_backoff_factor_is_two() {
        assert_eq!(Settings::default().backoff_factor, 2);
    }

    #[test]
    fn default_backoff_cap_is_one_hour() {
        assert_eq!(Settings::default().backoff_cap, Duration::from_secs(3600));
    }
}
