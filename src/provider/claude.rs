use serde::Deserialize;
use std::sync::{Mutex, OnceLock};
use crate::http::{GetResult, HttpError};
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
const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";

#[derive(Deserialize)]
struct ProfileAccount {
    email: String,
    has_claude_pro: bool,
    has_claude_max: bool,
}

#[derive(Deserialize)]
struct ProfileResponse {
    account: ProfileAccount,
}

struct ProfileData {
    email: String,
    plan: String,
}

fn plan_label(has_max: bool, has_pro: bool) -> &'static str {
    if has_max { "max" } else if has_pro { "pro" } else { "free" }
}

fn parse_profile_response(body: &str) -> Result<ProfileData, String> {
    let resp: ProfileResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok(ProfileData {
        email: resp.account.email,
        plan: plan_label(resp.account.has_claude_max, resp.account.has_claude_pro).to_string(),
    })
}

#[derive(Deserialize)]
struct UsageResponse {
    five_hour: WindowData,
    seven_day: WindowData,
}

#[derive(Deserialize)]
struct WindowData {
    utilization: f32,
    resets_at: Option<String>,
}

fn parse_response(body: &str) -> Result<[LimitWindow; 2], String> {
    let resp: UsageResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok([
        LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(resp.five_hour.utilization),
            limit: None,
            remaining: None,
            resets_at: resp.five_hour.resets_at,
            unlimited: false,
        },
        LimitWindow {
            name: "7d weekly".to_string(),
            percent_used: Some(resp.seven_day.utilization),
            limit: None,
            remaining: None,
            resets_at: resp.seven_day.resets_at,
            unlimited: false,
        },
    ])
}

pub struct ClaudeProvider {
    last_ok: Mutex<Option<Vec<LimitWindow>>>,
    profile: Mutex<Option<ProfileData>>,
    last_raw_json: Mutex<Option<String>>,
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self {
            last_ok: Mutex::new(None),
            profile: Mutex::new(None),
            last_raw_json: Mutex::new(None),
        }
    }
}

impl ClaudeProvider {
    pub fn new() -> Self { Self::default() }
}

fn fetch_profile(token: &str, ua: &str) -> Option<ProfileData> {
    let (result, _) = crate::http::get(PROFILE_URL, token, &[("User-Agent", ua)]);
    result.ok().and_then(|body| parse_profile_response(&body).ok())
}

fn do_fetch(
    creds: CredLoad,
    http: &dyn Fn(&str) -> GetResult,
    last_ok: &Mutex<Option<Vec<LimitWindow>>>,
    last_raw_json: &Mutex<Option<String>>,
    profile_string: Option<String>,
) -> (UsageState, Option<HttpError>) {
    let creds = match creds {
        CredLoad::NotConfigured => return (UsageState::NotConfigured, None),
        CredLoad::Malformed(e) => return (UsageState::Error(format!("Malformed credentials: {}", e)), None),
        CredLoad::Ok(c) => c,
    };
    if is_expired(creds.expires_at_ms) {
        let date = format_expiry_date(creds.expires_at_ms);
        return (UsageState::Stale(format!("Expired on {} — run: claude login", date)), None);
    }
    let (result, raw) = http(&creds.access_token);
    if let Some(body) = raw {
        *last_raw_json.lock().unwrap() = Some(body);
    }
    match result {
        Ok(body) => match parse_response(&body) {
            Ok(windows) => {
                let windows = windows.to_vec();
                *last_ok.lock().unwrap() = Some(windows.clone());
                (UsageState::Ok(windows, profile_string), None)
            }
            Err(e) => (UsageState::Error(format!("Parse error: {}", e)), None),
        },
        Err(HttpError::Unauthorized) => {
            (UsageState::Stale("Token rejected — run: claude login".to_string()), Some(HttpError::Unauthorized))
        }
        Err(HttpError::RateLimited) => {
            let state = last_ok
                .lock()
                .unwrap()
                .clone()
                .map(|w| UsageState::Ok(w, profile_string))
                .unwrap_or_else(|| UsageState::Error("Rate limited (no cache)".to_string()));
            (state, Some(HttpError::RateLimited))
        }
        Err(HttpError::ServerError(c)) => (UsageState::Error(format!("Server error {c}")), Some(HttpError::ServerError(c))),
        Err(HttpError::Other(e)) => (UsageState::Error(e), None),
    }
}

impl UsageProvider for ClaudeProvider {
    fn kind(&self) -> crate::provider::ProviderKind { crate::provider::ProviderKind::Claude }

    fn fetch_with_http_error(&self) -> (UsageState, Option<HttpError>) {
        let ua = get_user_agent();
        let creds = load_credentials();

        {
            let mut profile = self.profile.lock().unwrap();
            if profile.is_none() {
                if let CredLoad::Ok(ref c) = creds {
                    *profile = fetch_profile(&c.access_token, ua);
                }
            }
        }

        let profile_string = self
            .profile
            .lock()
            .unwrap()
            .as_ref()
            .map(|p| format!("{} ({})", p.email, p.plan));

        let (state, http_err) = do_fetch(
            creds,
            &|token| crate::http::get(USAGE_URL, token, &[("User-Agent", ua)]),
            &self.last_ok,
            &self.last_raw_json,
            profile_string,
        );

        if matches!(state, UsageState::Stale(_) | UsageState::Error(_)) {
            *self.profile.lock().unwrap() = None;
        }

        (state, http_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{LimitWindow, UsageState};
    use crate::http::HttpError;
    use std::sync::Mutex;

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
    fn parse_response_null_resets_at_is_ok() {
        let body = r#"{"five_hour":{"utilization":10.0,"resets_at":null},"seven_day":{"utilization":5.0,"resets_at":null}}"#;
        let windows = super::parse_response(body).unwrap();
        assert_eq!(windows[0].resets_at, None);
        assert_eq!(windows[1].resets_at, None);
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

    fn ok_creds() -> CredLoad {
        CredLoad::Ok(ClaudeCredentials {
            access_token: "tok".to_string(),
            expires_at_ms: 9_999_999_999_000,
        })
    }

    fn empty_cache() -> Mutex<Option<Vec<LimitWindow>>> {
        Mutex::new(None)
    }

    fn valid_body() -> &'static str {
        r#"{"five_hour":{"utilization":50.0,"resets_at":"2026-12-01T00:00:00Z"},"seven_day":{"utilization":20.0,"resets_at":"2026-12-07T00:00:00Z"}}"#
    }

    #[test]
    fn do_fetch_not_configured() {
        let (state, _) = super::do_fetch(CredLoad::NotConfigured, &|_| unreachable!(), &empty_cache(), &Mutex::new(None), None);
        assert_eq!(state, UsageState::NotConfigured);
    }

    #[test]
    fn do_fetch_malformed_creds() {
        let (state, _) = super::do_fetch(
            CredLoad::Malformed("bad json".to_string()),
            &|_| unreachable!(),
            &empty_cache(),
            &Mutex::new(None),
            None,
        );
        assert!(matches!(state, UsageState::Error(ref e) if e.contains("Malformed")));
    }

    #[test]
    fn do_fetch_expired_token_returns_stale() {
        let creds = CredLoad::Ok(ClaudeCredentials {
            access_token: "tok".to_string(),
            expires_at_ms: 1_000,
        });
        let (state, _) = super::do_fetch(creds, &|_| unreachable!(), &empty_cache(), &Mutex::new(None), None);
        assert!(matches!(state, UsageState::Stale(ref s) if s.contains("Expired on")));
    }

    #[test]
    fn do_fetch_401_returns_stale() {
        let (state, _) = super::do_fetch(
            ok_creds(),
            &|_| (Err(HttpError::Unauthorized), None),
            &empty_cache(),
            &Mutex::new(None),
            None,
        );
        assert!(matches!(state, UsageState::Stale(ref s) if s.contains("Token rejected")));
    }

    #[test]
    fn do_fetch_429_no_cache_returns_error() {
        let (state, _) = super::do_fetch(
            ok_creds(),
            &|_| (Err(HttpError::RateLimited), None),
            &empty_cache(),
            &Mutex::new(None),
            None,
        );
        assert!(matches!(state, UsageState::Error(ref s) if s.contains("Rate limited")));
    }

    #[test]
    fn do_fetch_429_with_cache_returns_cached_ok() {
        let cache = Mutex::new(Some(vec![LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(42.0),
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        }]));
        let (state, _) = super::do_fetch(ok_creds(), &|_| (Err(HttpError::RateLimited), None), &cache, &Mutex::new(None), None);
        assert!(matches!(state, UsageState::Ok(ref w, _) if w[0].percent_used == Some(42.0)));
    }

    #[test]
    fn do_fetch_200_bad_body_returns_error() {
        let (state, _) = super::do_fetch(
            ok_creds(),
            &|_| (Ok("garbage".to_string()), Some("garbage".to_string())),
            &empty_cache(),
            &Mutex::new(None),
            None,
        );
        assert!(matches!(state, UsageState::Error(ref s) if s.contains("Parse error")));
    }

    #[test]
    fn do_fetch_200_valid_returns_ok_and_populates_cache() {
        let cache = empty_cache();
        let (state, _) = super::do_fetch(ok_creds(), &|_| (Ok(valid_body().to_string()), Some(valid_body().to_string())), &cache, &Mutex::new(None), None);
        assert!(matches!(state, UsageState::Ok(ref w, _) if w.len() == 2));
        assert_eq!(cache.lock().unwrap().as_ref().map(|v| v.len()), Some(2), "cache must be populated with 2 windows after success");
    }

    #[test]
    fn parse_profile_max_plan() {
        let body = r#"{"account":{"email":"a@b.com","has_claude_pro":true,"has_claude_max":true}}"#;
        let pd = super::parse_profile_response(body).unwrap();
        assert_eq!(pd.email, "a@b.com");
        assert_eq!(pd.plan, "max");
    }

    #[test]
    fn parse_profile_pro_plan() {
        let body = r#"{"account":{"email":"a@b.com","has_claude_pro":true,"has_claude_max":false}}"#;
        let pd = super::parse_profile_response(body).unwrap();
        assert_eq!(pd.plan, "pro");
    }

    #[test]
    fn parse_profile_free_plan() {
        let body = r#"{"account":{"email":"a@b.com","has_claude_pro":false,"has_claude_max":false}}"#;
        let pd = super::parse_profile_response(body).unwrap();
        assert_eq!(pd.plan, "free");
    }

    #[test]
    fn parse_profile_missing_account_field_is_error() {
        assert!(super::parse_profile_response("{}").is_err());
    }

    #[test]
    fn do_fetch_passes_profile_string_into_ok() {
        let cache = empty_cache();
        let (state, _) = super::do_fetch(
            ok_creds(),
            &|_| (Ok(valid_body().to_string()), Some(valid_body().to_string())),
            &cache,
            &Mutex::new(None),
            Some("a@b.com (pro)".to_string()),
        );
        assert!(
            matches!(state, UsageState::Ok(_, ref p) if p.as_deref() == Some("a@b.com (pro)")),
            "profile string must be preserved in Ok variant"
        );
    }

    #[test]
    fn do_fetch_none_profile_propagates_to_ok() {
        let cache = empty_cache();
        let (state, _) = super::do_fetch(
            ok_creds(),
            &|_| (Ok(valid_body().to_string()), Some(valid_body().to_string())),
            &cache,
            &Mutex::new(None),
            None,
        );
        assert!(matches!(state, UsageState::Ok(_, None)));
    }

    #[test]
    fn do_fetch_429_with_cache_includes_profile_string() {
        let cache = Mutex::new(Some(vec![LimitWindow {
            name: "5h session".to_string(),
            percent_used: Some(42.0),
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        }]));
        let (state, _) = super::do_fetch(
            ok_creds(),
            &|_| (Err(HttpError::RateLimited), None),
            &cache,
            &Mutex::new(None),
            Some("a@b.com (pro)".to_string()),
        );
        assert!(
            matches!(state, UsageState::Ok(ref w, ref p)
                if w[0].percent_used == Some(42.0) && p.as_deref() == Some("a@b.com (pro)")),
            "cached Ok must carry profile string on rate limit"
        );
    }

    #[test]
    fn do_fetch_stores_raw_body_on_200() {
        let raw_cache: Mutex<Option<String>> = Mutex::new(None);
        let _ = super::do_fetch(
            ok_creds(),
            &|_| (Ok(valid_body().to_string()), Some(valid_body().to_string())),
            &empty_cache(),
            &raw_cache,
            None,
        );
        assert_eq!(raw_cache.lock().unwrap().as_deref(), Some(valid_body()));
    }

    #[test]
    fn do_fetch_stores_raw_body_on_401() {
        let raw_cache: Mutex<Option<String>> = Mutex::new(None);
        let _ = super::do_fetch(
            ok_creds(),
            &|_| (Err(HttpError::Unauthorized), Some(r#"{"error":"unauthorized"}"#.to_string())),
            &empty_cache(),
            &raw_cache,
            None,
        );
        assert_eq!(
            raw_cache.lock().unwrap().as_deref(),
            Some(r#"{"error":"unauthorized"}"#)
        );
    }

    #[test]
    fn do_fetch_does_not_store_raw_on_network_error() {
        let raw_cache: Mutex<Option<String>> = Mutex::new(None);
        let _ = super::do_fetch(
            ok_creds(),
            &|_| (Err(HttpError::Other("connection refused".into())), None),
            &empty_cache(),
            &raw_cache,
            None,
        );
        assert!(raw_cache.lock().unwrap().is_none());
    }
}
