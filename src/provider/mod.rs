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

/// Returns the diagnostic message to log for a provider that ended a fetch in a
/// non-happy state (`Error` or `Stale`), or `None` for happy/neutral states
/// (`Ok`, `NotConfigured`). Pure — used by the `refresh_all` boundary so every
/// provider error leaves a diagnostic trace without per-leaf instrumentation.
pub(crate) fn state_diag_message(name: &str, state: &UsageState) -> Option<String> {
    match state {
        UsageState::Error(msg) | UsageState::Stale(msg) => Some(format!("{}: {}", name, msg)),
        UsageState::Ok(..) | UsageState::NotConfigured => None,
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

    #[test]
    fn diag_message_error_includes_name_and_msg() {
        let s = UsageState::Error("boom".to_string());
        assert_eq!(state_diag_message("Claude", &s), Some("Claude: boom".to_string()));
    }

    #[test]
    fn diag_message_stale_includes_name_and_msg() {
        let s = UsageState::Stale("Expired on 2026-06-17 — run: claude login".to_string());
        assert_eq!(
            state_diag_message("Claude", &s),
            Some("Claude: Expired on 2026-06-17 — run: claude login".to_string())
        );
    }

    #[test]
    fn diag_message_ok_is_none() {
        let s = UsageState::Ok(vec![], Some("max".to_string()));
        assert_eq!(state_diag_message("Claude", &s), None);
    }

    #[test]
    fn diag_message_not_configured_is_none() {
        assert_eq!(state_diag_message("Copilot", &UsageState::NotConfigured), None);
    }
}
