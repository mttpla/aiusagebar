use serde::Deserialize;

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<serde_json::Value>,
}

pub(crate) fn parse_release(json: &str) -> Option<String> {
    let release: GithubRelease = serde_json::from_str(json).ok()?;
    if release.assets.is_empty() {
        return None;
    }
    let tag = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
    if tag.is_empty() {
        return None;
    }
    Some(tag.to_owned())
}

pub fn check() -> Option<String> {
    let json = crate::http::get_public(
        "https://api.github.com/repos/mttpla/aiusagebar/releases/latest",
    )
    .ok()?;
    let remote = parse_release(&json)?;
    is_newer(env!("CARGO_PKG_VERSION"), &remote).then_some(remote)
}

pub(crate) fn is_newer(current: &str, remote: &str) -> bool {
    fn parse(v: &str) -> Option<(u32, u32, u32)> {
        let mut parts = v.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse::<u32>().ok()?;
        Some((major, minor, patch))
    }
    match (parse(current), parse(remote)) {
        (Some(c), Some(r)) => r > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse_release};

    #[test]
    fn patch_bump_is_newer() {
        assert!(is_newer("0.3.2", "0.3.3"));
    }

    #[test]
    fn minor_bump_is_newer() {
        assert!(is_newer("0.3.2", "0.4.0"));
    }

    #[test]
    fn major_bump_is_newer() {
        assert!(is_newer("0.3.2", "1.0.0"));
    }

    #[test]
    fn same_version_not_newer() {
        assert!(!is_newer("0.4.0", "0.4.0"));
    }

    #[test]
    fn current_ahead_not_newer() {
        assert!(!is_newer("0.5.0", "0.4.0"));
    }

    #[test]
    fn malformed_remote_false() {
        assert!(!is_newer("0.3.2", "not-a-version"));
    }

    #[test]
    fn empty_remote_false() {
        assert!(!is_newer("0.3.2", ""));
    }

    #[test]
    fn malformed_current_false() {
        assert!(!is_newer("bad", "0.4.0"));
    }

    #[test]
    fn valid_newer_with_assets_returns_version() {
        let json = r#"{"tag_name":"v0.4.0","assets":[{"name":"aiusagebar-macos-arm64-v0.4.0"}]}"#;
        assert_eq!(parse_release(json), Some("0.4.0".to_owned()));
    }

    #[test]
    fn valid_same_version_with_assets_returns_version() {
        // parse_release only extracts; is_newer decides "new enough"
        let json = r#"{"tag_name":"v0.3.2","assets":[{"name":"aiusagebar-macos-arm64-v0.3.2"}]}"#;
        assert_eq!(parse_release(json), Some("0.3.2".to_owned()));
    }

    #[test]
    fn empty_assets_returns_none() {
        let json = r#"{"tag_name":"v0.4.0","assets":[]}"#;
        assert_eq!(parse_release(json), None);
    }

    #[test]
    fn github_404_body_returns_none() {
        let json = r#"{"message":"Not Found","documentation_url":"https://docs.github.com/rest"}"#;
        assert_eq!(parse_release(json), None);
    }

    #[test]
    fn malformed_json_returns_none() {
        assert_eq!(parse_release("not json at all"), None);
    }

    #[test]
    fn tag_without_v_prefix_returned_as_is() {
        let json = r#"{"tag_name":"0.4.0","assets":[{"name":"bin"}]}"#;
        assert_eq!(parse_release(json), Some("0.4.0".to_owned()));
    }
}
