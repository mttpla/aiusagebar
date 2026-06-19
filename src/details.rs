pub fn prepare_content(raw_json: Option<&str>) -> String {
    match raw_json {
        None => "No data yet".to_string(),
        Some(body) => serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| body.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_content_none_is_no_data_yet() {
        assert_eq!(prepare_content(None), "No data yet");
    }

    #[test]
    fn prepare_content_valid_json_pretty_prints() {
        let input = r#"{"a":1,"b":2}"#;
        let out = prepare_content(Some(input));
        assert!(out.contains('\n'), "expected newlines from pretty-print, got: {out}");
        assert!(out.contains('"'));
    }

    #[test]
    fn prepare_content_invalid_json_returns_raw() {
        let input = "not json at all";
        assert_eq!(prepare_content(Some(input)), input);
    }

    #[test]
    fn prepare_content_empty_string_returns_empty() {
        assert_eq!(prepare_content(Some("")), "");
    }
}
