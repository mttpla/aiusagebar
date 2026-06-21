pub(crate) fn show(provider_name: &str, raw_json: Option<&str>) {
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSAlert, NSFont, NSScrollView, NSTextView};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    let content = prepare_content(raw_json);
    let mtm = MainThreadMarker::new().expect("show() must be called on the main thread");
    let frame = NSRect { origin: NSPoint { x: 0.0, y: 0.0 }, size: NSSize { width: 600.0, height: 300.0 } };
    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), frame);
    scroll.setHasVerticalScroller(true);
    scroll.setHasHorizontalScroller(false);
    scroll.setAutohidesScrollers(true);
    let tv = NSTextView::initWithFrame(NSTextView::alloc(mtm), frame);
    tv.setEditable(false);
    tv.setSelectable(true);
    let font = NSFont::monospacedSystemFontOfSize_weight(12.0, 0.0);
    tv.setFont(Some(&font));
    tv.setString(&NSString::from_str(&content));
    scroll.setDocumentView(Some(&tv));
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(&format!("Details \u{2014} {}", provider_name)));
    alert.setAccessoryView(Some(&scroll));
    alert.addButtonWithTitle(&NSString::from_str("OK"));
    alert.runModal();
}

pub(crate) fn prepare_content(raw_json: Option<&str>) -> String {
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
