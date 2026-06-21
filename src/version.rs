pub(crate) fn format_version(cargo: &str, git: &str) -> String {
    let git = git.strip_prefix('v').unwrap_or(git);
    if git.starts_with(cargo) {
        git.to_string()
    } else {
        format!("{cargo}+{git}")
    }
}

pub(crate) fn app_version() -> String {
    format_version(env!("CARGO_PKG_VERSION"), env!("VERGEN_GIT_DESCRIBE"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_tag_returns_git_string() {
        assert_eq!(format_version("0.1.0", "0.1.0"), "0.1.0");
    }

    #[test]
    fn exact_tag_with_v_prefix_strips_v() {
        assert_eq!(format_version("0.1.0", "v0.1.0"), "0.1.0");
    }

    #[test]
    fn commits_after_tag_returns_git_string() {
        assert_eq!(
            format_version("0.1.0", "0.1.0-3-gabcdef"),
            "0.1.0-3-gabcdef"
        );
    }

    #[test]
    fn commits_after_v_tag_strips_v() {
        assert_eq!(
            format_version("0.1.0", "v0.1.0-3-gabcdef"),
            "0.1.0-3-gabcdef"
        );
    }

    #[test]
    fn no_tags_returns_cargo_plus_hash() {
        assert_eq!(
            format_version("0.1.0", "gabcdef"),
            "0.1.0+gabcdef"
        );
    }
}
