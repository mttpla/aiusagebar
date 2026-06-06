use serde::Deserialize;

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

pub fn load_credentials() -> Option<ClaudeCredentials> {
    let json = load_credentials_json()?;
    let file: CredentialsFile = serde_json::from_str(&json).ok()?;
    Some(ClaudeCredentials {
        access_token: file.claude_ai_oauth.access_token,
        expires_at_ms: file.claude_ai_oauth.expires_at,
    })
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
    expires_at_ms < now_ms
}

pub fn format_expiry_date(expires_at_ms: u64) -> String {
    use chrono::{TimeZone, Utc};
    let secs = (expires_at_ms / 1000) as i64;
    match Utc.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => "?".to_string(),
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
    fn format_expiry_date_known_timestamp() {
        // 1749081600000 ms = 2025-06-05 00:00:00 UTC
        assert_eq!(format_expiry_date(1749081600000), "2025-06-05");
    }
}
