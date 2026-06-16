pub fn check() -> Option<String> {
    todo!()
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
    use super::is_newer;

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
}
