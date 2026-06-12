//! NSAttributedString helpers for styling muda menu items on macOS.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_app_kit::{
    NSBox, NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSMenu,
    NSMutableParagraphStyle, NSParagraphStyleAttributeName, NSTextAlignment, NSTextField,
    NSTextTab, NSView,
};
use objc2_foundation::{NSArray, NSDictionary, NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize, NSString};
use tray_icon::menu::{ContextMenu, Menu};

use super::{MenuLayout, ProviderKind};

// ── Color ──────────────────────────────────────────────────────────────────

fn srgb(r: f64, g: f64, b: f64) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0)
}

// ── Low-level attribute setters ────────────────────────────────────────────

unsafe fn set_color(mattr: &NSMutableAttributedString, color: &NSColor, range: NSRange) {
    let value: &AnyObject = &*(color as *const NSColor as *const AnyObject);
    mattr.addAttribute_value_range(NSForegroundColorAttributeName, value, range);
}

unsafe fn set_font(mattr: &NSMutableAttributedString, font: &NSFont, range: NSRange) {
    let value: &AnyObject = &*(font as *const NSFont as *const AnyObject);
    mattr.addAttribute_value_range(NSFontAttributeName, value, range);
}

unsafe fn set_para_style(mattr: &NSMutableAttributedString, style: &NSMutableParagraphStyle) {
    let len = mattr.length();
    let range = NSRange { location: 0, length: len };
    let value: &AnyObject = &*(style as *const NSMutableParagraphStyle as *const AnyObject);
    mattr.addAttribute_value_range(NSParagraphStyleAttributeName, value, range);
}

// ── Paragraph style: right tab stop at 290pt ───────────────────────────────

fn refresh_para_style() -> Retained<NSMutableParagraphStyle> {
    let para = NSMutableParagraphStyle::new();
    let options: Retained<NSDictionary<NSString, AnyObject>> = NSDictionary::new();
    let tab = unsafe {
        NSTextTab::initWithTextAlignment_location_options(
            NSTextTab::alloc(),
            NSTextAlignment::Right,
            290.0,
            &options,
        )
    };
    let tabs: Retained<NSArray<NSTextTab>> = NSArray::from_retained_slice(&[tab]);
    para.setTabStops(Some(&tabs));
    para.setDefaultTabInterval(0.0);
    para
}

// ── Attributed string builders ─────────────────────────────────────────────

/// Provider header: brand color, bold 13pt.
fn header_attr_str(text: &str, r: f64, g: f64, b: f64) -> Retained<NSMutableAttributedString> {
    let ns_text = NSString::from_str(text);
    let mattr =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);
    let range = NSRange { location: 0, length: ns_text.length() };
    unsafe {
        set_color(&mattr, &srgb(r, g, b), range);
        set_font(&mattr, &NSFont::boldSystemFontOfSize(13.0), range);
    }
    mattr
}

/// Quit item: red #FF3B30, 13pt.
fn quit_attr_str() -> Retained<NSMutableAttributedString> {
    let ns_text = NSString::from_str("Quit");
    let mattr =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);
    let range = NSRange { location: 0, length: ns_text.length() };
    unsafe {
        set_color(&mattr, &srgb(1.0, 0.231, 0.188), range);
        set_font(&mattr, &NSFont::systemFontOfSize(13.0), range);
    }
    mattr
}

/// Refresh item: "↺ Refresh" in blue 13pt; if `updated` is Some, appends tab +
/// "Updated HH:MM" in secondaryLabelColor 11pt at right tab stop 290pt.
fn refresh_attr_str(updated: Option<&str>) -> Retained<NSMutableAttributedString> {
    let refresh_text = "↺ Refresh";
    let full_text = match updated {
        Some(ts) => format!("↺ Refresh\tUpdated {}", ts),
        None => refresh_text.to_owned(),
    };
    let ns_text = NSString::from_str(&full_text);
    let mattr =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);

    if updated.is_some() {
        let para = refresh_para_style();
        unsafe {
            set_para_style(&mattr, &para);
        }
    }

    let refresh_len = NSString::from_str(refresh_text).length();
    let refresh_range = NSRange { location: 0, length: refresh_len };
    unsafe {
        set_color(&mattr, &srgb(0.078, 0.494, 0.984), refresh_range);
        set_font(&mattr, &NSFont::systemFontOfSize(13.0), refresh_range);
    }

    if let Some(ts) = updated {
        let tab_text = format!("\tUpdated {}", ts);
        let tab_len = NSString::from_str(&tab_text).length();
        let ts_range = NSRange { location: refresh_len, length: tab_len };
        unsafe {
            set_color(&mattr, &NSColor::secondaryLabelColor(), ts_range);
            set_font(&mattr, &NSFont::systemFontOfSize(11.0), ts_range);
        }
    }

    mattr
}

// ── Progress bar helpers ───────────────────────────────────────────────────

fn bar_fill_color(pct: f32) -> Retained<NSColor> {
    if pct < 60.0 {
        srgb(0.204, 0.780, 0.349) // #34C759 green
    } else if pct <= 80.0 {
        srgb(1.0, 0.624, 0.039)   // #FF9F0A amber
    } else {
        srgb(1.0, 0.231, 0.188)   // #FF3B30 red
    }
}

fn bar_fill_width(pct: Option<f32>) -> f64 {
    pct.map(|p| (p / 100.0 * 270.0) as f64)
        .unwrap_or(0.0)
        .clamp(0.0, 270.0)
}

fn format_reset(window: &crate::provider::LimitWindow) -> String {
    use chrono::DateTime;
    let Some(ref resets_at) = window.resets_at else {
        return String::new();
    };
    let name = window.name.to_lowercase();
    if name.contains("5h") || name.contains("session") {
        if let Ok(dt) = DateTime::parse_from_rfc3339(resets_at) {
            let now = chrono::Local::now();
            let secs = dt.signed_duration_since(now).num_seconds().max(0);
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            return if h > 0 {
                format!("resets in {}h {}m", h, m)
            } else {
                format!("resets in {}m", m)
            };
        }
        resets_at.clone()
    } else if name.contains("7d") || name.contains("weekly") {
        if let Ok(dt) = DateTime::parse_from_rfc3339(resets_at) {
            let local = dt.with_timezone(&chrono::Local);
            return format!("resets {}", local.format("%b %-d"));
        }
        resets_at.clone()
    } else {
        format!("resets {}", resets_at)
    }
}

// ── Style pass ─────────────────────────────────────────────────────────────

unsafe fn apply_to_item(ns_menu: &NSMenu, idx: usize, attr: &NSMutableAttributedString) {
    if let Some(item) = ns_menu.itemAtIndex(idx as isize) {
        item.setAttributedTitle(Some(attr));
    }
}

pub(super) fn style_menu(menu: &Menu, layout: &MenuLayout) {
    let ns_menu_ptr = menu.ns_menu() as *const NSMenu;
    if ns_menu_ptr.is_null() {
        return;
    }
    let ns_menu: &NSMenu = unsafe { &*ns_menu_ptr };

    unsafe {
        for (idx, kind) in &layout.header_indices {
            if let Some(item) = ns_menu.itemAtIndex(*idx as isize) {
                let title = item.title();
                let text = title.to_string();
                let (r, g, b) = match kind {
                    ProviderKind::Claude => (0.788, 0.333, 0.118),
                    ProviderKind::Copilot => (0.431, 0.251, 0.788),
                };
                let attr = header_attr_str(&text, r, g, b);
                item.setAttributedTitle(Some(&attr));
            }
        }

        let refresh = refresh_attr_str(layout.last_updated.as_deref());
        apply_to_item(ns_menu, layout.refresh_idx, &refresh);

        let quit = quit_attr_str();
        apply_to_item(ns_menu, layout.quit_idx, &quit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LimitWindow;

    fn make_window(name: &str, pct: Option<f32>, resets_at: Option<&str>) -> LimitWindow {
        LimitWindow {
            name: name.to_owned(),
            percent_used: pct,
            resets_at: resets_at.map(str::to_owned),
            ..Default::default()
        }
    }

    // bar_fill_width

    #[test]
    fn bar_fill_width_50_pct() {
        let w = bar_fill_width(Some(50.0));
        assert!((w - 135.0).abs() < 0.01, "got {w}");
    }

    #[test]
    fn bar_fill_width_100_pct() {
        let w = bar_fill_width(Some(100.0));
        assert!((w - 270.0).abs() < 0.01, "got {w}");
    }

    #[test]
    fn bar_fill_width_over_100_clamped() {
        assert!((bar_fill_width(Some(150.0)) - 270.0).abs() < 0.01);
    }

    #[test]
    fn bar_fill_width_none_is_zero() {
        assert_eq!(bar_fill_width(None), 0.0);
    }

    // format_reset

    #[test]
    fn format_reset_none_resets_at_returns_empty() {
        let w = make_window("5h session", None, None);
        assert_eq!(format_reset(&w), "");
    }

    #[test]
    fn format_reset_7d_window_returns_absolute_date() {
        let w = make_window("7d weekly", None, Some("2026-06-20T08:00:00Z"));
        assert_eq!(format_reset(&w), "resets Jun 20");
    }

    #[test]
    fn format_reset_5h_window_future_returns_relative_format() {
        use chrono::{Duration, Local};
        let future = (Local::now() + Duration::hours(3) + Duration::minutes(30))
            .to_rfc3339();
        let w = make_window("5h session", None, Some(&future));
        let s = format_reset(&w);
        assert!(s.starts_with("resets in"), "got: {s}");
        assert!(s.contains('h') || s.contains('m'), "got: {s}");
    }

    #[test]
    fn format_reset_5h_window_past_returns_zero() {
        let past = "2020-01-01T00:00:00Z";
        let w = make_window("5h session", None, Some(past));
        let s = format_reset(&w);
        assert_eq!(s, "resets in 0m", "got: {s}");
    }

    #[test]
    fn format_reset_unknown_window_returns_raw_with_prefix() {
        let w = make_window("Daily", None, Some("2026-06-20T08:00:00Z"));
        let s = format_reset(&w);
        assert_eq!(s, "resets 2026-06-20T08:00:00Z");
    }
}
