use serde::Deserialize;
use std::sync::{Mutex, OnceLock};
use crate::http::HttpError;
use crate::provider::{LimitWindow, UsageState, UsageProvider};

#[derive(Deserialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: OauthEntry,
}

#[derive(Deserialize)]
struct OauthEntry {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: u64,
}

pub struct ClaudeCredentials {
    pub access_token: String,
    pub expires_at_ms: u64,
}

pub enum CredLoad {
    NotConfigured,
    Malformed(String),
    Ok(ClaudeCredentials),
}

pub fn parse_credentials_payload(json: Option<String>) -> CredLoad {
    let Some(json) = json else { return CredLoad::NotConfigured; };
    match serde_json::from_str::<CredentialsFile>(&json) {
        Ok(file) => CredLoad::Ok(ClaudeCredentials {
            access_token: file.claude_ai_oauth.access_token,
            expires_at_ms: file.claude_ai_oauth.expires_at,
        }),
        Err(e) => CredLoad::Malformed(e.to_string()),
    }
}

pub fn load_credentials() -> CredLoad {
    parse_credentials_payload(load_credentials_json())
}

fn load_credentials_json() -> Option<String> {
    let account = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    if let Some(json) = crate::keychain::read_generic_password("Claude Code-credentials", &account) {
        return Some(json);
    }
    let path = dirs::home_dir()?.join(".claude").join(".credentials.json");
    std::fs::read_to_string(path).ok()
}

pub fn is_expired(expires_at_ms: u64) -> bool {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    expires_at_ms <= now_ms
}

pub fn format_expiry_date(expires_at_ms: u64) -> String {
    use chrono::{Local, TimeZone};
    let secs = (expires_at_ms / 1000) as i64;
    match Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => "?".to_string(),
    }
}

static USER_AGENT: OnceLock<String> = OnceLock::new();

fn parse_version(s: &str) -> Option<String> {
    s.split_whitespace()
        .find(|t| t.chars().any(|c| c.is_ascii_digit()))
        .map(|t| t.trim_matches(|c: char| !c.is_ascii_digit() && c != '.').to_string())
        .filter(|t| !t.is_empty())
}

fn get_user_agent() -> &'static str {
    USER_AGENT.get_or_init(|| {
        std::process::Command::new("claude")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .as_deref()
            .and_then(parse_version)
            .map(|v| format!("claude-code/{}", v))
            .unwrap_or_else(|| "claude-code/2.1.153".to_string())
    })
}

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Deserialize)]
struct UsageResponse {
    five_hour: WindowData,
    seven_day: WindowData,
}

#[derive(Deserialize)]
struct WindowData {
    utilization: f32,
    resets_at: String,
}

fn parse_response(body: &str) -> Result<[LimitWindow; 2], String> {
    let resp: UsageResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok([
        LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(resp.five_hour.utilization),
            limit: None,
            remaining: None,
            resets_at: Some(resp.five_hour.resets_at),
            unlimited: false,
        },
        LimitWindow {
            name: "7d weekly".to_string(),
            percent_used: Some(resp.seven_day.utilization),
            limit: None,
            remaining: None,
            resets_at: Some(resp.seven_day.resets_at),
            unlimited: false,
        },
    ])
}

pub struct ClaudeProvider {
    last_ok: Mutex<Option<Vec<LimitWindow>>>,
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self { last_ok: Mutex::new(None) }
    }
}

impl ClaudeProvider {
    pub fn new() -> Self { Self::default() }
}

impl UsageProvider for ClaudeProvider {
    fn name(&self) -> &'static str { "Anthropic" }

    fn fetch(&self) -> UsageState {
        let creds = match load_credentials() {
            CredLoad::NotConfigured => return UsageState::NotConfigured,
            CredLoad::Malformed(e) => return UsageState::Error(format!("Malformed credentials: {}", e)),
            CredLoad::Ok(c) => c,
        };
        if is_expired(creds.expires_at_ms) {
            let date = format_expiry_date(creds.expires_at_ms);
            return UsageState::Stale(format!("Expired on {} — run: claude login", date));
        }
        let ua = get_user_agent();
        match crate::http::get(USAGE_URL, &creds.access_token, &[("User-Agent", ua)]) {
            Ok(body) => match parse_response(&body) {
                Ok(windows) => {
                    let windows = windows.to_vec();
                    *self.last_ok.lock().unwrap() = Some(windows.clone());
                    UsageState::Ok(windows)
                }
                Err(e) => UsageState::Error(format!("Parse error: {}", e)),
            },
            Err(HttpError::Unauthorized) => {
                UsageState::Stale("Token rejected — run: claude login".to_string())
            }
            Err(HttpError::RateLimited) => {
                self.last_ok
                    .lock()
                    .unwrap()
                    .clone()
                    .map(UsageState::Ok)
                    .unwrap_or_else(|| UsageState::Error("Rate limited (no cache)".to_string()))
            }
            Err(HttpError::Other(e)) => UsageState::Error(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_credentials_json() {
        let json = r#"{"claudeAiOauth":{"accessToken":"tok123","expiresAt":9999999999000}}"#;
        let file: CredentialsFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.claude_ai_oauth.access_token, "tok123");
        assert_eq!(file.claude_ai_oauth.expires_at, 9999999999000);
    }

    #[test]
    fn parse_missing_expires_at_fails() {
        let json = r#"{"claudeAiOauth":{"accessToken":"tok123"}}"#;
        assert!(serde_json::from_str::<CredentialsFile>(json).is_err());
    }

    #[test]
    fn expired_epoch_plus_1s_is_expired() {
        assert!(is_expired(1_000));
    }

    #[test]
    fn far_future_token_not_expired() {
        assert!(!is_expired(9_999_999_999_000));
    }

    #[test]
    fn format_expiry_date_yyyy_mm_dd_shape() {
        let s = format_expiry_date(1749081600000);
        let bytes = s.as_bytes();
        assert_eq!(bytes.len(), 10, "got {s}");
        assert_eq!(bytes[4], b'-');
        assert_eq!(bytes[7], b'-');
    }

    #[test]
    fn parse_valid_response() {
        let body = r#"{
            "five_hour": {"utilization": 39.0, "resets_at": "2026-06-06T14:00:00Z"},
            "seven_day":  {"utilization": 15.0, "resets_at": "2026-06-10T08:00:00Z"}
        }"#;
        let windows = super::parse_response(body).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].name, "5h session");
        assert_eq!(windows[0].percent_used, Some(39.0));
        assert_eq!(windows[1].name, "7d weekly");
        assert_eq!(windows[1].percent_used, Some(15.0));
    }

    #[test]
    fn parse_missing_field_is_error() {
        assert!(super::parse_response("{}").is_err());
    }

    #[test]
    fn parse_version_first_token_numeric() {
        assert_eq!(super::parse_version("2.1.153 (Claude Code)"), Some("2.1.153".to_string()));
    }

    #[test]
    fn parse_version_skips_leading_words() {
        assert_eq!(super::parse_version("Claude Code 2.1.153"), Some("2.1.153".to_string()));
    }

    #[test]
    fn parse_version_none_on_empty() {
        assert_eq!(super::parse_version(""), None);
    }

    #[test]
    fn parse_version_trims_leading_alpha() {
        assert_eq!(super::parse_version("v2.1.153"), Some("2.1.153".to_string()));
    }

    #[test]
    fn parse_version_trims_trailing_suffix() {
        assert_eq!(super::parse_version("2.1.153-beta"), Some("2.1.153".to_string()));
    }

    #[test]
    fn load_result_missing_is_not_configured() {
        assert!(matches!(super::parse_credentials_payload(None), super::CredLoad::NotConfigured));
    }

    #[test]
    fn load_result_corrupt_is_malformed() {
        let bad = Some("{not json".to_string());
        assert!(matches!(super::parse_credentials_payload(bad), super::CredLoad::Malformed(_)));
    }

    #[test]
    fn load_result_valid_is_ok() {
        let good = Some(r#"{"claudeAiOauth":{"accessToken":"t","expiresAt":1}}"#.to_string());
        assert!(matches!(super::parse_credentials_payload(good), super::CredLoad::Ok(_)));
    }
}
