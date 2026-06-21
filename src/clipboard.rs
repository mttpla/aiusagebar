//! Clipboard helper.

/// Copies `text` to the macOS general pasteboard, replacing its contents.
#[cfg(target_os = "macos")]
pub fn copy(text: &str) {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;
    unsafe {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let ns = NSString::from_str(text);
        pb.setString_forType(&ns, NSPasteboardTypeString);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn copy(_text: &str) {}

#[cfg(test)]
mod tests {
    #[test]
    fn copy_has_expected_signature() {
        let _: fn(&str) = super::copy;
    }
}
