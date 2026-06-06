const LABEL: &str = "com.mttpla.aiusagebar";

fn plist_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| {
        h.join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist"))
    })
}

fn plist_content(binary_path: &str) -> String {
    let safe_path = binary_path
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{safe_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#
    )
}

#[cfg(debug_assertions)]
pub fn enable() -> Result<(), String> {
    eprintln!("[launch_at_login] skipped in debug build");
    Ok(())
}

#[cfg(not(debug_assertions))]
pub fn enable() -> Result<(), String> {
    todo!("implement enable")
}

pub fn disable() -> Result<(), String> {
    todo!("implement disable")
}

pub fn is_enabled() -> bool {
    plist_path().map(|p| p.exists()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_content_contains_label_and_binary() {
        let xml = plist_content("/opt/homebrew/bin/aiusagebar");
        assert!(xml.contains("<string>com.mttpla.aiusagebar</string>"));
        assert!(xml.contains("<string>/opt/homebrew/bin/aiusagebar</string>"));
        assert!(xml.contains("<array>\n        <string>/opt/homebrew/bin/aiusagebar</string>"));
        assert!(xml.contains("<true/>"));
        let keep_alive_pos = xml.find("KeepAlive").unwrap();
        assert!(xml[keep_alive_pos..].contains("<false/>"));
    }
}
