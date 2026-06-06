pub mod claude;

#[derive(Debug, Clone, PartialEq)]
pub struct LimitWindow {
    pub name: String,
    pub percent_used: Option<f32>,
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub resets_at: Option<String>,
    pub unlimited: bool,
}

#[derive(Debug, Clone)]
pub enum UsageState {
    NotConfigured,
    Stale(String),
    Ok(Vec<LimitWindow>),
    Error(String),
}

pub trait UsageProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn fetch(&self) -> UsageState;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_window_fields() {
        let w = LimitWindow {
            name: "5h".to_string(),
            percent_used: Some(42.0),
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        };
        assert_eq!(w.percent_used, Some(42.0));
        assert!(!w.unlimited);
    }

    #[test]
    fn usage_state_error_carries_message() {
        let s = UsageState::Error("timeout".to_string());
        if let UsageState::Error(msg) = s {
            assert_eq!(msg, "timeout");
        } else {
            panic!("wrong variant");
        }
    }
}
