use chrono::Datelike;

const START_YEAR: i32 = 2026;

#[cfg(target_os = "macos")]
const ABOUT_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/about-icon.png"));

pub fn copyright_year_str(current_year: i32) -> String {
    if current_year == START_YEAR {
        START_YEAR.to_string()
    } else {
        format!("{START_YEAR}\u{2013}{current_year}")
    }
}

pub fn is_italian() -> bool {
    std::env::var("LANG")
        .unwrap_or_default()
        .to_lowercase()
        .starts_with("it")
}

pub fn body_text(copyright_year: &str, italian: bool) -> String {
    let tagline = if italian {
        "Monitor in sola lettura. Non invia prompt, non consuma quota, non modifica credenziali."
    } else {
        "A read-only monitor. Never sends prompts, never spends quota, never modifies credentials."
    };
    format!(
        "\u{00a9} {copyright_year} Matteo Paoli \u{00b7} MIT License\n\
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
    use chrono::Local;
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSAlert, NSAlertSecondButtonReturn, NSImage, NSTextField, NSTextAlignment};
    use objc2_foundation::{NSData, NSPoint, NSRect, NSSize, NSString};

    let version = crate::version::app_version();
    let year_str = copyright_year_str(Local::now().year());
    let body = body_text(&year_str, is_italian());

    let mtm = MainThreadMarker::new().expect("show() must be called on the main thread");
    let alert = NSAlert::new(mtm);
    let icon_data = NSData::with_bytes(ABOUT_ICON);
    let icon = NSImage::initWithData(NSImage::alloc(), &icon_data);
    if let Some(ref img) = icon {
        img.setTemplate(true);
    }
    unsafe { alert.setIcon(icon.as_deref()) };
    alert.setMessageText(&NSString::from_str(&format!("AIUsageBar {version}")));

    // Centered body via NSTextField accessory view (NSAlert has no built-in center alignment).
    let tf = NSTextField::wrappingLabelWithString(&NSString::from_str(&body), mtm);
    tf.setAlignment(NSTextAlignment::Center);
    tf.setFrame(NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize { width: 460.0, height: 160.0 },
    });
    alert.setAccessoryView(Some(&tf));

    alert.addButtonWithTitle(&NSString::from_str("OK"));
    alert.addButtonWithTitle(&NSString::from_str("www.matteopaoli.it"));
    let response = alert.runModal();
    if response == NSAlertSecondButtonReturn {
        let _ = std::process::Command::new("open")
            .arg("https://www.matteopaoli.it")
            .spawn();
    }
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
        let body = body_text("2026", false);
        assert!(body.contains("read-only monitor"));
        assert!(!body.contains("sola lettura"));
    }

    #[test]
    fn body_italian_contains_italian_tagline() {
        let body = body_text("2026", true);
        assert!(body.contains("sola lettura"));
        assert!(!body.contains("read-only monitor"));
    }

    #[test]
    fn body_contains_year() {
        let body = body_text("2026\u{2013}2028", false);
        assert!(body.contains("2026\u{2013}2028"));
    }

    #[test]
    fn body_contains_github_url() {
        let body = body_text("2026", false);
        assert!(body.contains("https://github.com/mttpla/aiusagebar"));
    }

    #[test]
    fn body_contains_disclaimer() {
        let body = body_text("2026", false);
        assert!(body.contains("as is"));
        assert!(body.contains("not liable"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn about_icon_is_valid_png() {
        use image::GenericImageView;
        let img = image::load_from_memory(ABOUT_ICON).expect("ABOUT_ICON must be a valid PNG");
        assert_eq!(img.width(), 128, "icon must be 128px wide");
        assert_eq!(img.height(), 128, "icon must be 128px tall");
        assert!(
            img.pixels().any(|(_, _, p)| p.0[3] > 0),
            "icon must have at least one non-transparent pixel"
        );
    }
}
