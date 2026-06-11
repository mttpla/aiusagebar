pub mod claude;
pub mod copilot;

#[derive(Debug, Clone, PartialEq)]
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
