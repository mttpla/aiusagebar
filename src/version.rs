pub fn format_version(cargo: &str, git: &str) -> String {
    if git.starts_with(cargo) {
        git.to_string()
    } else {
        format!("{cargo}+{git}")
    }
}

pub fn app_version() -> String {
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
    fn commits_after_tag_returns_git_string() {
        assert_eq!(
            format_version("0.1.0", "0.1.0-3-gabcdef"),
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
