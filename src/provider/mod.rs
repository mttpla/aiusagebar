pub(crate) mod claude;
pub(crate) mod copilot;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct LimitWindow {
    pub(crate) name: String,
    pub(crate) percent_used: Option<f32>,
    pub(crate) limit: Option<u32>,
    pub(crate) remaining: Option<u32>,
    pub(crate) resets_at: Option<String>,
    pub(crate) unlimited: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UsageState {
    NotConfigured,
    Stale(String),
    Ok(Vec<LimitWindow>, Option<String>),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ProviderKind {
    Claude,
    Copilot,
}

impl ProviderKind {
    pub(crate) fn display_name(&self) -> &'static str {
        match self {
            ProviderKind::Claude => "Claude",
            ProviderKind::Copilot => "Copilot",
        }
    }
}

pub(crate) trait UsageProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    /// Returns the usage state plus the raw HTTP error that caused it, if any.
    /// Only `RateLimited` and `ServerError` errors trigger backoff in the caller.
    fn fetch_with_http_error(&self) -> (UsageState, Option<crate::http::HttpError>);
    /// Returns the last raw HTTP response body received by this provider, if any.
    fn raw_json(&self) -> Option<String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::claude::ClaudeProvider;
    use crate::provider::copilot::CopilotProvider;

    #[test]
    fn limit_window_default() {
        let w = LimitWindow::default();
        assert_eq!(w.name, "");
        assert!(w.percent_used.is_none());
    }

    #[test]
    fn provider_kind_display_name_claude() {
        assert_eq!(ProviderKind::Claude.display_name(), "Claude");
    }

    #[test]
    fn provider_kind_display_name_copilot() {
        assert_eq!(ProviderKind::Copilot.display_name(), "Copilot");
    }

    #[test]
    fn claude_provider_kind_is_claude() {
        let p = ClaudeProvider::new();
        assert_eq!(p.kind(), ProviderKind::Claude);
    }

    #[test]
    fn copilot_provider_kind_is_copilot() {
        let p = CopilotProvider::new();
        assert_eq!(p.kind(), ProviderKind::Copilot);
    }
}
