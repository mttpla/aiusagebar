use std::time::{Duration, Instant};

pub struct BackoffState {
    pub next_allowed_at:  Instant,
    pub current_interval: Duration,
}

impl BackoffState {
    pub fn new(base: Duration) -> Self {
        Self {
            next_allowed_at:  Instant::now(),
            current_interval: base,
        }
    }

    pub fn on_success(&mut self, base: Duration) {
        self.current_interval = base;
        self.next_allowed_at = Instant::now() + base;
    }

    pub fn on_error(&mut self, factor: u32, cap: Duration) {
        self.current_interval = (self.current_interval * factor).min(cap);
        self.next_allowed_at  = Instant::now() + self.current_interval;
    }

    pub fn is_allowed(&self) -> bool {
        Instant::now() >= self.next_allowed_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_immediately_allowed() {
        assert!(BackoffState::new(Duration::from_secs(300)).is_allowed());
    }

    #[test]
    fn on_success_resets_interval() {
        let base = Duration::from_secs(300);
        let cap  = Duration::from_secs(3600);
        let mut s = BackoffState::new(base);
        s.on_error(2, cap);
        s.on_success(base);
        assert_eq!(s.current_interval, base);
    }

    #[test]
    fn on_success_advances_next_allowed_at() {
        let base = Duration::from_secs(300);
        let mut s = BackoffState::new(base);
        s.on_success(base);
        assert!(!s.is_allowed(), "next poll must be in the future after on_success");
    }

    #[test]
    fn on_error_doubles_interval() {
        let cap = Duration::from_secs(3600);
        let mut s = BackoffState::new(Duration::from_secs(300));
        s.on_error(2, cap);
        assert_eq!(s.current_interval, Duration::from_secs(600));
        s.on_error(2, cap);
        assert_eq!(s.current_interval, Duration::from_secs(1200));
    }

    #[test]
    fn on_error_caps_at_cap() {
        let cap = Duration::from_secs(3600);
        let mut s = BackoffState::new(Duration::from_secs(300));
        for _ in 0..20 {
            s.on_error(2, cap);
        }
        assert_eq!(s.current_interval, cap);
    }

    #[test]
    fn on_error_blocks_is_allowed() {
        let mut s = BackoffState::new(Duration::from_secs(300));
        s.on_error(2, Duration::from_secs(3600));
        assert!(!s.is_allowed());
    }
}
