use std::time::Duration;

pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(300);
pub const DEFAULT_ALERT_THRESHOLD_PCT: f32 = 80.0;

pub struct Settings {
    pub poll_interval: Duration,
    pub alert_threshold_pct: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            poll_interval: DEFAULT_POLL_INTERVAL,
            alert_threshold_pct: DEFAULT_ALERT_THRESHOLD_PCT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_poll_interval_is_five_minutes() {
        let s = Settings::default();
        assert_eq!(s.poll_interval, Duration::from_secs(300));
    }

    #[test]
    fn default_alert_threshold_is_eighty_percent() {
        assert_eq!(Settings::default().alert_threshold_pct, 80.0_f32);
    }
}
