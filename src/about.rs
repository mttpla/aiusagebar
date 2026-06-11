use chrono::Datelike;

pub fn copyright_year_str(current_year: i32) -> String {
    if current_year == 2026 {
        "2026".to_string()
    } else {
        format!("2026\u{2013}{}", current_year)
    }
}

pub fn is_italian() -> bool {
    std::env::var("LANG")
        .unwrap_or_default()
        .to_lowercase()
        .starts_with("it")
}

pub fn body_text(version: &str, copyright_year: &str, italian: bool) -> String {
    let tagline = if italian {
        "Monitor in sola lettura. Non invia prompt, non consuma quota, non modifica credenziali."
    } else {
        "A read-only monitor. Never sends prompts, never spends quota, never modifies credentials."
    };
    format!(
        "AIUsageBar {version}\n\
         \u{00a9} {copyright_year} Matteo Paoli \u{00b7} MIT License\n\
         https://github.com/mttpla/aiusagebar\n\
         \n\
         {tagline}\n\
         \n\
         This software is provided \"as is\", without warranty of any kind.\n\
         The author is not liable for any damages arising from its use."
    )
}

#[cfg(target_os = "macos")]
pub fn show() {
    // implemented in Task 2
    let _ = (copyright_year_str(0), is_italian(), body_text("", "", false));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copyright_year_start_year_is_just_2026() {
        assert_eq!(copyright_year_str(2026), "2026");
    }

    #[test]
    fn copyright_year_after_start_year_shows_range() {
        assert_eq!(copyright_year_str(2027), "2026\u{2013}2027");
    }

    #[test]
    fn body_english_contains_english_tagline() {
        let body = body_text("0.1.0", "2026", false);
        assert!(body.contains("read-only monitor"));
        assert!(!body.contains("sola lettura"));
    }

    #[test]
    fn body_italian_contains_italian_tagline() {
        let body = body_text("0.1.0", "2026", true);
        assert!(body.contains("sola lettura"));
        assert!(!body.contains("read-only monitor"));
    }

    #[test]
    fn body_contains_version_and_year() {
        let body = body_text("1.2.3", "2026\u{2013}2028", false);
        assert!(body.contains("1.2.3"));
        assert!(body.contains("2026\u{2013}2028"));
    }

    #[test]
    fn body_contains_github_url() {
        let body = body_text("0.1.0", "2026", false);
        assert!(body.contains("https://github.com/mttpla/aiusagebar"));
    }

    #[test]
    fn body_contains_disclaimer() {
        let body = body_text("0.1.0", "2026", false);
        assert!(body.contains("as is"));
        assert!(body.contains("not liable"));
    }
}
