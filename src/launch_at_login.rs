const LABEL: &str = "com.mttpla.aiusagebar";

#[allow(dead_code)]
fn plist_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| {
        h.join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist"))
    })
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn uid() -> Result<u32, String> {
    let out = std::process::Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("id -u failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    String::from_utf8(out.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .parse::<u32>()
        .map_err(|e| e.to_string())
}

#[cfg(debug_assertions)]
pub fn enable() -> Result<(), String> {
    eprintln!("[launch_at_login] skipped in debug build");
    Ok(())
}

// launchctl exit codes we treat as benign.
const LAUNCHCTL_ALREADY_LOADED: i32 = 36;
const LAUNCHCTL_NOT_LOADED: i32 = 3;

#[cfg(not(debug_assertions))]
pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let binary = exe.to_str().ok_or("non-UTF8 binary path")?;
    let plist = plist_path().ok_or("no home directory")?;
    let content = plist_content(binary);
    let needs_write = std::fs::read_to_string(&plist).map(|cur| cur != content).unwrap_or(true);
    if needs_write {
        if let Some(parent) = plist.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&plist, &content).map_err(|e| e.to_string())?;
    }
    let uid = uid()?;
    let plist_str = plist.to_str().ok_or("non-UTF8 plist path")?;
    let out = std::process::Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{uid}"), plist_str])
        .output()
        .map_err(|e| e.to_string())?;
    // Plist written = launch-at-login registered. Bootstrap is best-effort (immediate start).
    // macOS 13+ auto-manages LaunchAgents from ~/Library/LaunchAgents/ without needing bootstrap.
    let code = out.status.code().unwrap_or(-1);
    if !out.status.success() && code != LAUNCHCTL_ALREADY_LOADED {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        eprintln!("[launch_at_login] bootstrap warning (plist registered, will start at next login): {msg}");
    }
    Ok(())
}

#[allow(dead_code)]
pub fn disable() -> Result<(), String> {
    let uid = uid()?;
    let out = std::process::Command::new("launchctl")
        .args(["bootout", &format!("gui/{uid}/{LABEL}")])
        .output()
        .map_err(|e| e.to_string())?;
    let code = out.status.code().unwrap_or(-1);
    let result = if out.status.success() || code == LAUNCHCTL_ALREADY_LOADED || code == LAUNCHCTL_NOT_LOADED {
        Ok(())
    } else {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if msg.is_empty() { format!("launchctl bootout exited with code {code}") } else { msg })
    };
    if let Some(p) = plist_path() {
        let _ = std::fs::remove_file(p);
    }
    result
}

#[allow(dead_code)]
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
