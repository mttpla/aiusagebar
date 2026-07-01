//! NSAttributedString helpers for styling muda menu items on macOS.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_app_kit::{
    NSBox, NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSMenu,
    NSTextAlignment, NSTextField, NSView,
};
use objc2_foundation::{NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize, NSString};
use tray_icon::menu::{ContextMenu, Menu};

use super::MenuLayout;
use crate::provider::ProviderKind;

const CONTAINER_W: f64 = 290.0;
const BAR_MARGIN: f64 = 8.0;

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

// ── Footer item text (pure, no NSKit) ─────────────────────────────────────

/// Returns (display_string, utf16_len_of_refresh_prefix).
/// The offset is used to apply blue color to only "↺ Refresh" in the
/// attributed string; the remainder ("  ·  Updated HH:MM") is secondary.
fn refresh_display_text(updated: Option<&str>) -> (String, usize) {
    const LABEL: &str = "↺ Refresh";
    let offset = LABEL.encode_utf16().count();
    let text = match updated {
        None => LABEL.to_owned(),
        Some(ts) => format!("{LABEL}  ·  Updated {ts}"),
    };
    (text, offset)
}

fn about_display_text() -> &'static str {
    "ℹ About AIUsageBar"
}

fn quit_display_text() -> &'static str {
    "Quit"
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

/// About item: system labelColor, 13pt — matches Refresh/Quit weight.
fn about_attr_str() -> Retained<NSMutableAttributedString> {
    let ns_text = NSString::from_str(about_display_text());
    let mattr =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);
    let range = NSRange { location: 0, length: ns_text.length() };
    unsafe {
        set_color(&mattr, &NSColor::labelColor(), range);
        set_font(&mattr, &NSFont::systemFontOfSize(13.0), range);
    }
    mattr
}

/// Quit item: red #FF3B30, 13pt.
fn quit_attr_str() -> Retained<NSMutableAttributedString> {
    let ns_text = NSString::from_str(quit_display_text());
    let mattr =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);
    let range = NSRange { location: 0, length: ns_text.length() };
    unsafe {
        set_color(&mattr, &srgb(1.0, 0.231, 0.188), range);
        set_font(&mattr, &NSFont::systemFontOfSize(13.0), range);
    }
    mattr
}

/// Refresh item: "↺ Refresh" in blue 13pt; if `updated` is Some, appends
/// "  ·  Updated HH:MM" in secondaryLabelColor 11pt. No tab stop — avoids
/// NSMenu width expansion triggered by paragraph-style tab stops.
fn refresh_attr_str(updated: Option<&str>) -> Retained<NSMutableAttributedString> {
    let (text, refresh_len) = refresh_display_text(updated);
    let ns_text = NSString::from_str(&text);
    let mattr =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);
    let refresh_range = NSRange { location: 0, length: refresh_len };
    unsafe {
        set_color(&mattr, &srgb(0.078, 0.494, 0.984), refresh_range);
        set_font(&mattr, &NSFont::systemFontOfSize(13.0), refresh_range);
        if updated.is_some() {
            let full_len = ns_text.length();
            let tail_range = NSRange { location: refresh_len, length: full_len - refresh_len };
            set_color(&mattr, &NSColor::secondaryLabelColor(), tail_range);
            set_font(&mattr, &NSFont::systemFontOfSize(11.0), tail_range);
        }
    }
    mattr
}

// ── Progress bar helpers ───────────────────────────────────────────────────

/// Bar fill color zone, selected by percent used. Pure classification split out
/// from color construction so the threshold logic is testable without AppKit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarZone {
    Green,
    Amber,
    Red,
}

fn bar_zone(pct: f32) -> BarZone {
    if pct < crate::settings::BAR_WARN_PCT {
        BarZone::Green
    } else if pct <= crate::settings::BAR_ALERT_PCT {
        BarZone::Amber
    } else {
        BarZone::Red
    }
}

fn bar_fill_color(pct: f32) -> Retained<NSColor> {
    match bar_zone(pct) {
        BarZone::Green => srgb(0.204, 0.780, 0.349), // #34C759 green
        BarZone::Amber => srgb(1.0, 0.624, 0.039),   // #FF9F0A amber
        BarZone::Red => srgb(1.0, 0.231, 0.188),     // #FF3B30 red
    }
}

fn bar_fill_width(pct: Option<f32>, bar_w: f64) -> f64 {
    pct.map(|p| (p / 100.0) as f64 * bar_w)
        .unwrap_or(0.0)
        .clamp(0.0, bar_w)
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
        // Default (Copilot windows, unknown names): short local date, fallback to raw.
        if let Ok(dt) = DateTime::parse_from_rfc3339(resets_at) {
            let local = dt.with_timezone(&chrono::Local);
            return format!("resets {}", local.format("%b %-d"));
        }
        format!("resets {}", resets_at)
    }
}

fn format_money(spent: f64, budget: f64, currency: &str) -> String {
    let sym = if currency == "USD" {
        "$".to_string()
    } else {
        format!("{} ", currency)
    };
    format!("{sym}{spent:.2} / {sym}{budget:.2}")
}

fn format_detail(window: &crate::provider::LimitWindow) -> String {
    match (window.spent, window.budget) {
        (Some(spent), Some(budget)) => {
            let currency = window.currency.as_deref().unwrap_or("USD");
            format_money(spent, budget, currency)
        }
        _ => format_reset(window),
    }
}

unsafe fn make_progress_row_view(window: &crate::provider::LimitWindow) -> objc2::rc::Retained<NSView> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSBoxType;
    let mtm = MainThreadMarker::new().expect("make_progress_row_view must be called on the main thread");

    let bar_w = CONTAINER_W - 2.0 * BAR_MARGIN;
    let name_w: f64 = 155.0;
    let pct_x = BAR_MARGIN + name_w;
    let pct_w = CONTAINER_W - pct_x;

    let container = NSView::initWithFrame(
        mtm.alloc(),
        NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize { width: CONTAINER_W, height: 42.0 },
        },
    );

    // name label — gray 11.5pt, top-left
    let name_field = NSTextField::labelWithString(&NSString::from_str(&window.name), mtm);
    name_field.setFont(Some(&NSFont::systemFontOfSize(11.5)));
    name_field.setTextColor(Some(&NSColor::secondaryLabelColor()));
    name_field.setFrame(NSRect {
        origin: NSPoint { x: BAR_MARGIN, y: 26.0 },
        size: NSSize { width: name_w, height: 14.0 },
    });
    container.addSubview(&name_field);

    // pct label — bold 11.5pt, threshold color (or secondary if unknown), right-aligned, top-right
    let pct_str = window
        .percent_used
        .map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "—".to_string());
    let pct_field = NSTextField::labelWithString(&NSString::from_str(&pct_str), mtm);
    pct_field.setFont(Some(&NSFont::boldSystemFontOfSize(11.5)));
    let pct_text_color: Retained<NSColor> = if let Some(pct) = window.percent_used {
        bar_fill_color(pct)
    } else {
        NSColor::secondaryLabelColor()
    };
    pct_field.setTextColor(Some(&pct_text_color));
    pct_field.setAlignment(NSTextAlignment::Right);
    pct_field.setFrame(NSRect {
        origin: NSPoint { x: pct_x, y: 26.0 },
        size: NSSize { width: pct_w, height: 14.0 },
    });
    container.addSubview(&pct_field);

    // bar background — separatorColor
    let bar_bg: objc2::rc::Retained<NSBox> =
        objc2::msg_send![mtm.alloc::<NSBox>(), initWithFrame: NSRect {
            origin: NSPoint { x: BAR_MARGIN, y: 18.0 },
            size: NSSize { width: bar_w, height: 4.0 },
        }];
    bar_bg.setBoxType(NSBoxType::Custom);
    bar_bg.setFillColor(&NSColor::separatorColor());
    bar_bg.setBorderWidth(0.0_f64);
    container.addSubview(&bar_bg);

    // bar fill — threshold color
    let fill_w = bar_fill_width(window.percent_used, bar_w);
    if fill_w > 0.0 {
        let bar_fill: objc2::rc::Retained<NSBox> =
            objc2::msg_send![mtm.alloc::<NSBox>(), initWithFrame: NSRect {
                origin: NSPoint { x: BAR_MARGIN, y: 18.0 },
                size: NSSize { width: fill_w, height: 4.0 },
            }];
        bar_fill.setBoxType(NSBoxType::Custom);
        bar_fill.setFillColor(&bar_fill_color(window.percent_used.unwrap_or(0.0)));
        bar_fill.setBorderWidth(0.0_f64);
        container.addSubview(&bar_fill);
    }

    // detail line — gray 10.5pt, bottom
    let detail = format_detail(window);
    if !detail.is_empty() {
        let detail_field = NSTextField::labelWithString(&NSString::from_str(&detail), mtm);
        detail_field.setFont(Some(&NSFont::systemFontOfSize(10.5)));
        detail_field.setTextColor(Some(&NSColor::secondaryLabelColor()));
        detail_field.setFrame(NSRect {
            origin: NSPoint { x: BAR_MARGIN, y: 2.0 },
            size: NSSize { width: bar_w, height: 14.0 },
        });
        container.addSubview(&detail_field);
    }

    container
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

        // Footer order in append_footer: Refresh, separator, About, Quit
        // → About sits at refresh_idx + 2.
        let about = about_attr_str();
        apply_to_item(ns_menu, layout.refresh_idx + 2, &about);

        let quit = quit_attr_str();
        apply_to_item(ns_menu, layout.quit_idx, &quit);

        for (idx, window) in &layout.window_items {
            if let Some(item) = ns_menu.itemAtIndex(*idx as isize) {
                let view = make_progress_row_view(window);
                item.setView(Some(&view));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LimitWindow;

    // ── Bar color zone thresholds (pure classification) ──────────────────

    #[test]
    fn bar_zone_zero_is_green() {
        assert_eq!(bar_zone(0.0), BarZone::Green);
    }

    #[test]
    fn bar_zone_just_below_warn_is_green() {
        assert_eq!(bar_zone(crate::settings::BAR_WARN_PCT - 0.1), BarZone::Green);
    }

    #[test]
    fn bar_zone_at_warn_boundary_is_amber() {
        // `< BAR_WARN_PCT` is green, so the boundary value itself is amber.
        assert_eq!(bar_zone(crate::settings::BAR_WARN_PCT), BarZone::Amber);
    }

    #[test]
    fn bar_zone_between_boundaries_is_amber() {
        assert_eq!(bar_zone(70.0), BarZone::Amber);
    }

    #[test]
    fn bar_zone_at_alert_boundary_is_amber() {
        // `<= BAR_ALERT_PCT` is amber, so the boundary value is still amber.
        assert_eq!(bar_zone(crate::settings::BAR_ALERT_PCT), BarZone::Amber);
    }

    #[test]
    fn bar_zone_just_above_alert_is_red() {
        assert_eq!(bar_zone(crate::settings::BAR_ALERT_PCT + 0.1), BarZone::Red);
    }

    #[test]
    fn bar_zone_hundred_is_red() {
        assert_eq!(bar_zone(100.0), BarZone::Red);
    }

    // ── Footer item text (pure fns — no main thread required) ─────────────

    #[test]
    fn refresh_display_text_none_contains_label() {
        let (text, _) = refresh_display_text(None);
        assert!(text.contains("↺ Refresh"), "got: {text:?}");
    }

    #[test]
    fn refresh_display_text_none_has_no_tab() {
        let (text, _) = refresh_display_text(None);
        assert!(!text.contains('\t'), "tab found in: {text:?}");
    }

    #[test]
    fn refresh_display_text_none_has_no_updated() {
        let (text, _) = refresh_display_text(None);
        assert!(!text.contains("Updated"), "got: {text:?}");
    }

    #[test]
    fn refresh_display_text_some_contains_label() {
        let (text, _) = refresh_display_text(Some("12:34"));
        assert!(text.contains("↺ Refresh"), "got: {text:?}");
    }

    #[test]
    fn refresh_display_text_some_contains_timestamp() {
        let (text, _) = refresh_display_text(Some("12:34"));
        assert!(text.contains("12:34"), "got: {text:?}");
    }

    #[test]
    fn refresh_display_text_some_has_no_tab() {
        let (text, _) = refresh_display_text(Some("12:34"));
        assert!(!text.contains('\t'), "tab found in: {text:?}");
    }

    #[test]
    fn refresh_display_text_offset_matches_refresh_label_utf16_len() {
        let (_, offset) = refresh_display_text(None);
        let expected: usize = "↺ Refresh".encode_utf16().count();
        assert_eq!(offset, expected, "offset should cover only '↺ Refresh'");
    }

    #[test]
    fn about_display_text_contains_about() {
        assert!(about_display_text().contains("About"), "got: {:?}", about_display_text());
    }

    #[test]
    fn about_display_text_contains_app_name() {
        assert!(about_display_text().contains("AIUsageBar"), "got: {:?}", about_display_text());
    }

    #[test]
    fn about_display_text_has_no_tab() {
        assert!(!about_display_text().contains('\t'), "tab found in about text");
    }

    #[test]
    fn quit_display_text_is_quit() {
        assert!(quit_display_text().starts_with("Quit"), "got: {:?}", quit_display_text());
    }

    #[test]
    fn quit_display_text_has_no_tab() {
        assert!(!quit_display_text().contains('\t'), "tab found in quit text");
    }

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
        let bar_w = CONTAINER_W - 2.0 * BAR_MARGIN;
        let w = bar_fill_width(Some(50.0), bar_w);
        assert!((w - bar_w / 2.0).abs() < 0.01, "got {w}");
    }

    #[test]
    fn bar_fill_width_100_pct() {
        let bar_w = CONTAINER_W - 2.0 * BAR_MARGIN;
        let w = bar_fill_width(Some(100.0), bar_w);
        assert!((w - bar_w).abs() < 0.01, "got {w}");
    }

    #[test]
    fn bar_fill_width_over_100_clamped() {
        let bar_w = CONTAINER_W - 2.0 * BAR_MARGIN;
        assert!((bar_fill_width(Some(150.0), bar_w) - bar_w).abs() < 0.01);
    }

    #[test]
    fn bar_fill_width_none_is_zero() {
        assert_eq!(bar_fill_width(None, CONTAINER_W - 2.0 * BAR_MARGIN), 0.0);
    }

    // format_reset

    #[test]
    fn format_reset_none_resets_at_returns_empty() {
        let w = make_window("5h session", None, None);
        assert_eq!(format_reset(&w), "");
    }

    #[test]
    fn format_reset_7d_window_returns_absolute_date() {
        use chrono::DateTime;
        let ts = "2026-06-20T08:00:00Z";
        let dt = DateTime::parse_from_rfc3339(ts).unwrap();
        let expected = format!("resets {}", dt.with_timezone(&chrono::Local).format("%b %-d"));
        let w = make_window("7d weekly", None, Some(ts));
        assert_eq!(format_reset(&w), expected);
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
    fn format_reset_unknown_window_formats_as_short_date() {
        use chrono::DateTime;
        let ts = "2026-06-20T08:00:00Z";
        let dt = DateTime::parse_from_rfc3339(ts).unwrap();
        let expected = format!("resets {}", dt.with_timezone(&chrono::Local).format("%b %-d"));
        let w = make_window("Daily", None, Some(ts));
        assert_eq!(format_reset(&w), expected);
    }

    #[test]
    fn format_reset_copilot_window_formats_as_short_date() {
        use chrono::DateTime;
        let ts = "2026-07-01T00:00:00.000Z";
        let dt = DateTime::parse_from_rfc3339(ts).unwrap();
        let expected = format!("resets {}", dt.with_timezone(&chrono::Local).format("%b %-d"));
        let w = make_window("matteo / premium_interactions", None, Some(ts));
        assert_eq!(format_reset(&w), expected);
    }

    #[test]
    fn format_reset_unparseable_falls_back_to_raw() {
        let w = make_window("monthly", None, Some("not-a-date"));
        assert_eq!(format_reset(&w), "resets not-a-date");
    }

    #[test]
    fn format_money_usd_uses_dollar_symbol() {
        assert_eq!(super::format_money(0.0, 50.0, "USD"), "$0.00 / $50.00");
    }

    #[test]
    fn format_money_non_usd_prefixes_currency_code() {
        assert_eq!(super::format_money(1.5, 20.0, "EUR"), "EUR 1.50 / EUR 20.00");
    }

    #[test]
    fn format_detail_money_present_formats_dollars() {
        let mut w = make_window("Spend", Some(0.0), None);
        w.spent = Some(0.0);
        w.budget = Some(50.0);
        w.currency = Some("USD".to_string());
        assert_eq!(super::format_detail(&w), "$0.00 / $50.00");
    }

    #[test]
    fn format_detail_money_absent_falls_back_to_reset() {
        let w = make_window("7d weekly", Some(15.0), Some("2026-07-01T00:00:00+00:00"));
        assert_eq!(super::format_detail(&w), super::format_reset(&w));
    }
}
