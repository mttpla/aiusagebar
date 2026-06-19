use crate::http::{GetResult, HttpError};
use crate::provider::{LimitWindow, UsageState};
use std::sync::Mutex;

fn parse_copilot_response(body: &str) -> Result<Vec<LimitWindow>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;

    let Some(snapshots) = v.get("quota_snapshots").and_then(|s| s.as_object()) else {
        return Ok(vec![]);
    };

    let login = v.get("login").and_then(|l| l.as_str()).unwrap_or("unknown");
    let resets_at = v
        .get("quota_reset_date_utc")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string());

    let mut windows = Vec::new();

    for (key, snap) in snapshots {
        if snap.get("unlimited").and_then(|u| u.as_bool()).unwrap_or(false) {
            continue;
        }
        let Some(percent_remaining) = snap.get("percent_remaining").and_then(|p| p.as_f64()) else {
            continue;
        };
        let percent_used = (100.0 - percent_remaining as f32).clamp(0.0, 100.0);
        let limit = snap
            .get("entitlement")
            .and_then(|e| e.as_u64())
            .map(|e| e as u32);
        let remaining = snap
            .get("remaining")
            .and_then(|r| r.as_u64())
            .map(|r| r as u32);

        windows.push(LimitWindow {
            name: format!("{} / {}", login, key),
            percent_used: Some(percent_used),
            limit,
            remaining,
            resets_at: resets_at.clone(),
            unlimited: false,
        });
    }

    Ok(windows)
}

pub fn do_copilot_fetch(
    tokens: Vec<(String, String)>,
    http: &dyn Fn(&str) -> GetResult,
) -> (UsageState, Option<HttpError>, Option<String>) {
    if tokens.is_empty() {
        return (UsageState::NotConfigured, None, None);
    }

    let mut ok_windows: Vec<LimitWindow> = Vec::new();
    let mut stale_accounts: Vec<String> = Vec::new();
    let mut error_msgs: Vec<String> = Vec::new();
    let mut backoff_err: Option<HttpError> = None;
    let mut raw_buf = String::new();

    for (account, token) in &tokens {
        let (result, raw) = http(token);
        if let Some(body) = raw {
            if !raw_buf.is_empty() {
                raw_buf.push('\n');
            }
            raw_buf.push_str(&format!("--- @{} ---\n{}", account, body));
        }
        match result {
            Ok(body) => match parse_copilot_response(&body) {
                Ok(windows) => ok_windows.extend(windows),
                Err(e) => error_msgs.push(format!("@{} — {}", account, e)),
            },
            Err(HttpError::Unauthorized) => stale_accounts.push(account.clone()),
            Err(HttpError::RateLimited) => {
                error_msgs.push(format!("@{} — rate limited", account));
                backoff_err = Some(HttpError::RateLimited);
            }
            Err(HttpError::ServerError(c)) => {
                error_msgs.push(format!("@{} — server error {c}", account));
                if backoff_err.is_none() {
                    backoff_err = Some(HttpError::ServerError(c));
                }
            }
            Err(HttpError::Other(e)) => error_msgs.push(format!("@{} — {}", account, e)),
        }
    }

    let raw_json = if raw_buf.is_empty() { None } else { Some(raw_buf) };

    if !ok_windows.is_empty() {
        for account in stale_accounts {
            ok_windows.push(LimitWindow {
                name: format!("@{} — token expired, re-login", account),
                percent_used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                unlimited: false,
            });
        }
        for msg in error_msgs {
            ok_windows.push(LimitWindow {
                name: msg,
                percent_used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                unlimited: false,
            });
        }
        return (UsageState::Ok(ok_windows, None), backoff_err, raw_json);
    }

    if !stale_accounts.is_empty() {
        return (
            UsageState::Stale("Copilot tokens expired — run: copilot auth login".to_string()),
            None,
            raw_json,
        );
    }

    (UsageState::Error(error_msgs.join("; ")), backoff_err, raw_json)
}

fn load_copilot_tokens() -> Vec<(String, String)> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut tokens: Vec<(String, String)> = Vec::new();
    for (account, password) in crate::keychain::enumerate_generic_passwords("copilot-cli") {
        if seen.insert(password.clone()) {
            tokens.push((account, password));
        }
    }
    tokens
}

pub struct CopilotProvider {
    last_raw_json: Mutex<Option<String>>,
}

impl CopilotProvider {
    pub fn new() -> Self {
        Self {
            last_raw_json: Mutex::new(None),
        }
    }
}

impl crate::provider::UsageProvider for CopilotProvider {
    fn kind(&self) -> crate::provider::ProviderKind {
        crate::provider::ProviderKind::Copilot
    }

    fn raw_json(&self) -> Option<String> {
        self.last_raw_json.lock().unwrap().clone()
    }

    fn fetch_with_http_error(&self) -> (UsageState, Option<crate::http::HttpError>) {
        let (state, err, raw) = do_copilot_fetch(
            load_copilot_tokens(),
            &|token| {
                crate::http::get(
                    "https://api.github.com/copilot_internal/user",
                    token,
                    &[("User-Agent", "aiusagebar/0.1")],
                )
            },
        );
        *self.last_raw_json.lock().unwrap() = raw;
        (state, err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpError;
    use crate::provider::UsageState;

    // ── parse_copilot_response ────────────────────────────────────────────

    #[test]
    fn parse_single_limited_snapshot() {
        let body = r#"{
            "login": "mttpla",
            "quota_reset_date_utc": "2026-07-01T00:00:00Z",
            "quota_snapshots": {
                "premium_interactions": {
                    "entitlement": 7000,
                    "remaining": 6604,
                    "percent_remaining": 94.3,
                    "unlimited": false
                }
            }
        }"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].name, "mttpla / premium_interactions");
        assert!((windows[0].percent_used.unwrap() - 5.7).abs() < 0.1);
        assert_eq!(windows[0].remaining, Some(6604));
        assert_eq!(windows[0].limit, Some(7000));
        assert_eq!(windows[0].resets_at.as_deref(), Some("2026-07-01T00:00:00Z"));
        assert!(!windows[0].unlimited);
    }

    #[test]
    fn parse_skips_unlimited_snapshots() {
        let body = r#"{
            "login": "mttpla",
            "quota_reset_date_utc": "2026-07-01T00:00:00Z",
            "quota_snapshots": {
                "chat":        { "unlimited": true },
                "completions": { "unlimited": true }
            }
        }"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 0);
    }

    #[test]
    fn parse_mixed_limited_and_unlimited() {
        let body = r#"{
            "login": "mttpla",
            "quota_reset_date_utc": "2026-07-01T00:00:00Z",
            "quota_snapshots": {
                "premium_interactions": { "entitlement": 7000, "remaining": 500, "percent_remaining": 50.0, "unlimited": false },
                "chat": { "unlimited": true }
            }
        }"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 1);
        assert!((windows[0].percent_used.unwrap() - 50.0).abs() < 0.1);
    }

    #[test]
    fn parse_missing_quota_snapshots_returns_empty() {
        // Account without Copilot access — no quota_snapshots field
        let body = r#"{"login": "mttpla", "copilot_plan": "individual", "access_type_sku": "no_access"}"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 0);
    }

    #[test]
    fn parse_missing_percent_remaining_skips_snapshot() {
        let body = r#"{
            "login": "mttpla",
            "quota_reset_date_utc": "2026-07-01T00:00:00Z",
            "quota_snapshots": { "mystery": { "unlimited": false } }
        }"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 0);
    }

    // ── do_copilot_fetch ──────────────────────────────────────────────────

    fn valid_body() -> String {
        r#"{"login":"mttpla","quota_reset_date_utc":"2026-07-01T00:00:00Z","quota_snapshots":{"premium_interactions":{"entitlement":7000,"remaining":6604,"percent_remaining":94.3,"unlimited":false}}}"#.to_string()
    }

    fn tok(account: &str, token: &str) -> (String, String) {
        (account.to_string(), token.to_string())
    }

    #[test]
    fn fetch_empty_tokens_returns_not_configured() {
        let (state, _, _) = do_copilot_fetch(vec![], &|_| unreachable!());
        assert_eq!(state, UsageState::NotConfigured);
    }

    #[test]
    fn fetch_all_401_returns_stale() {
        let (state, _, _) = do_copilot_fetch(
            vec![tok("alice", "tok")],
            &|_| (Err(HttpError::Unauthorized), None),
        );
        assert!(matches!(state, UsageState::Stale(_)));
    }

    #[test]
    fn fetch_200_valid_returns_ok_with_windows() {
        let (state, _, _) = do_copilot_fetch(
            vec![tok("alice", "tok")],
            &|_| (Ok(valid_body()), Some(valid_body())),
        );
        assert!(matches!(state, UsageState::Ok(ref w, _) if !w.is_empty()));
    }

    #[test]
    fn fetch_mixed_success_and_401_returns_ok_with_sentinel() {
        let tokens = vec![tok("good_account", "good"), tok("bad_account", "bad")];
        let (state, _, _) = do_copilot_fetch(tokens, &|tok| {
            if tok == "good" { (Ok(valid_body()), Some(valid_body())) } else { (Err(HttpError::Unauthorized), None) }
        });
        let UsageState::Ok(windows, _) = state else { panic!("expected Ok") };
        assert!(windows.iter().any(|w| w.percent_used.is_some()), "real window missing");
        assert!(
            windows.iter().any(|w| w.percent_used.is_none() && w.name.contains("@bad_account") && w.name.contains("expired")),
            "sentinel window missing or missing account name"
        );
    }

    #[test]
    fn fetch_other_error_returns_error() {
        let (state, _, _) = do_copilot_fetch(
            vec![tok("alice", "tok")],
            &|_| (Err(HttpError::Other("connection refused".to_string())), None),
        );
        assert!(matches!(state, UsageState::Error(ref s) if s.contains("connection refused")));
    }

    #[test]
    fn fetch_error_sentinel_contains_account_name() {
        let tokens = vec![tok("good_account", "good"), tok("bad_account", "bad")];
        let (state, _, _) = do_copilot_fetch(tokens, &|tok| {
            if tok == "good" { (Ok(valid_body()), Some(valid_body())) } else { (Err(HttpError::Other("timeout".to_string())), None) }
        });
        let UsageState::Ok(windows, _) = state else { panic!("expected Ok") };
        assert!(
            windows.iter().any(|w| w.name.contains("@bad_account") && w.name.contains("timeout")),
            "error sentinel missing account name"
        );
    }

    #[test]
    fn fetch_200_bad_body_returns_error() {
        let (state, _, _) = do_copilot_fetch(
            vec![tok("alice", "tok")],
            &|_| (Ok("not json".to_string()), Some("not json".to_string())),
        );
        assert!(matches!(state, UsageState::Error(_)));
    }

    #[test]
    fn fetch_single_account_raw_json_stored() {
        let body = valid_body();
        let (_, _, raw) = do_copilot_fetch(
            vec![tok("alice", "tok_a")],
            &|_| (Ok(body.clone()), Some(body.clone())),
        );
        let raw = raw.unwrap();
        assert!(raw.contains("--- @alice ---"), "missing account header");
        assert!(raw.contains(&body), "missing body");
    }

    #[test]
    fn fetch_multi_account_raw_json_has_both_sections() {
        let body1 = r#"{"login":"alice"}"#.to_string();
        let body2 = r#"{"login":"bob"}"#.to_string();
        let (_, _, raw) = do_copilot_fetch(
            vec![tok("alice", "tok_a"), tok("bob", "tok_b")],
            &|tok| {
                if tok == "tok_a" {
                    (Ok(body1.clone()), Some(body1.clone()))
                } else {
                    (Ok(body2.clone()), Some(body2.clone()))
                }
            },
        );
        let raw = raw.unwrap();
        assert!(raw.contains("--- @alice ---"));
        assert!(raw.contains("--- @bob ---"));
        assert!(raw.contains(&body1));
        assert!(raw.contains(&body2));
    }

    #[test]
    fn fetch_empty_tokens_raw_json_is_none() {
        let (_, _, raw) = do_copilot_fetch(vec![], &|_| unreachable!());
        assert!(raw.is_none());
    }

    #[test]
    fn parse_non_object_quota_snapshots_returns_empty() {
        // Malformed response: quota_snapshots is not an object — treated same as missing
        let body = r#"{"login":"mttpla","quota_reset_date_utc":"2026-07-01T00:00:00Z","quota_snapshots":42}"#;
        let windows = parse_copilot_response(body).unwrap();
        assert_eq!(windows.len(), 0);
    }
}
