use std::time::{Duration, Instant};

pub(crate) struct BackoffState {
    base: Duration,
    factor: u32,
    cap: Duration,
    current_interval: Duration,
    next_allowed_at: Instant,
}

impl BackoffState {
    pub(crate) fn new(base: Duration, factor: u32, cap: Duration) -> Self {
        Self {
            base,
            factor,
            cap,
            current_interval: base,
            next_allowed_at: Instant::now(),
        }
    }

    pub(crate) fn on_success(&mut self) {
        self.current_interval = self.base;
        self.next_allowed_at = Instant::now() + self.base;
    }

    pub(crate) fn on_error(&mut self) {
        self.current_interval = (self.current_interval * self.factor).min(self.cap);
        self.next_allowed_at = Instant::now() + self.current_interval;
    }

    pub(crate) fn is_allowed(&self) -> bool {
        Instant::now() >= self.next_allowed_at
    }

    pub(crate) fn next_allowed_at(&self) -> Instant {
        self.next_allowed_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FACTOR: u32 = 2;

    fn cap() -> Duration {
        Duration::from_secs(3600)
    }

    fn base() -> Duration {
        Duration::from_secs(300)
    }

    #[test]
    fn new_state_is_immediately_allowed() {
        assert!(BackoffState::new(base(), FACTOR, cap()).is_allowed());
    }

    #[test]
    fn on_success_resets_interval() {
        let mut s = BackoffState::new(base(), FACTOR, cap());
        s.on_error();
        s.on_success();
        assert_eq!(s.current_interval, base());
    }

    #[test]
    fn on_success_advances_next_allowed_at() {
        let mut s = BackoffState::new(base(), FACTOR, cap());
        s.on_success();
        assert!(!s.is_allowed(), "next poll must be in the future after on_success");
    }

    #[test]
    fn on_error_doubles_interval() {
        let mut s = BackoffState::new(base(), FACTOR, cap());
        s.on_error();
        assert_eq!(s.current_interval, Duration::from_secs(600));
        s.on_error();
        assert_eq!(s.current_interval, Duration::from_secs(1200));
    }

    #[test]
    fn on_error_caps_at_cap() {
        let mut s = BackoffState::new(base(), FACTOR, cap());
        for _ in 0..20 {
            s.on_error();
        }
        assert_eq!(s.current_interval, cap());
    }

    #[test]
    fn on_error_blocks_is_allowed() {
        let mut s = BackoffState::new(base(), FACTOR, cap());
        s.on_error();
        assert!(!s.is_allowed());
    }
}
