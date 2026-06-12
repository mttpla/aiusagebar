pub mod claude;
pub mod copilot;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LimitWindow {
    pub name: String,
    pub percent_used: Option<f32>,
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub resets_at: Option<String>,
    pub unlimited: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UsageState {
    NotConfigured,
    Stale(String),
    Ok(Vec<LimitWindow>, Option<String>),
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
    fn limit_window_default() {
        let w = LimitWindow::default();
        assert_eq!(w.name, "");
        assert!(w.percent_used.is_none());
    }
}
