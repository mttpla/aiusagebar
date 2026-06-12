//! NSAttributedString helpers for styling muda menu items on macOS.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_app_kit::{
    NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSMenu,
    NSMutableParagraphStyle, NSParagraphStyleAttributeName, NSTextAlignment, NSTextTab,
};
use objc2_foundation::{NSArray, NSDictionary, NSMutableAttributedString, NSRange, NSString};
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
