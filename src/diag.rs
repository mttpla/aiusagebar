use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

const CAPACITY: usize = 100;
const MAX_MSG_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Err,
    Info,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Err => "ERR",
            Level::Info => "INF",
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    time: String,
    level: Level,
    message: String,
}

static DIAG: OnceLock<Mutex<VecDeque<Entry>>> = OnceLock::new();

fn buffer() -> &'static Mutex<VecDeque<Entry>> {
    DIAG.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

/// Truncates `s` to at most `max_bytes`, respecting char boundaries, appending
/// "… (truncated)" when truncation occurs.
pub fn truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… (truncated)", &s[..end])
}

fn format_entry(time: &str, level: Level, message: &str) -> String {
    format!("[{} {}] {}", time, level.label(), message)
}

pub fn push(level: Level, msg: impl Into<String>) {
    let message = truncate(&msg.into(), MAX_MSG_BYTES);
    let time = chrono::Local::now().format("%H:%M:%S").to_string();
    let entry = Entry { time, level, message };
    let mut buf = buffer().lock().unwrap();
    if buf.len() == CAPACITY {
        buf.pop_front();
    }
    buf.push_back(entry);
}

pub fn is_empty() -> bool {
    buffer().lock().unwrap().is_empty()
}

pub fn format_all() -> String {
    let buf = buffer().lock().unwrap();
    buf.iter()
        .map(|e| format_entry(&e.time, e.level, &e.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Pushes a diagnostic entry, auto-injecting the call-site `file!():line!()`.
/// Usage: `crate::diag!(crate::diag::Level::Err, "fetch failed: {}", e);`
#[macro_export]
macro_rules! diag {
    ($lvl:expr, $($arg:tt)*) => {
        $crate::diag::push($lvl, format!("[{}:{}] {}", file!(), line!(), format!($($arg)*)))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 2048), "hello");
    }

    #[test]
    fn truncate_long_string_appends_marker() {
        let s = "a".repeat(3000);
        let out = truncate(&s, 2048);
        assert!(out.ends_with("… (truncated)"), "got tail: {:?}", &out[out.len().saturating_sub(20)..]);
        assert!(out.len() <= 2048 + "… (truncated)".len());
    }

    #[test]
    fn truncate_respects_char_boundary() {
        // 'é' is 2 bytes; cutting at an odd byte must not panic and must stay valid UTF-8.
        let s = "é".repeat(2000); // 4000 bytes
        let out = truncate(&s, 2049); // odd boundary inside a char
        assert!(out.is_char_boundary(out.len()));
        assert!(out.ends_with("… (truncated)"));
    }

    #[test]
    fn format_entry_has_time_level_and_message() {
        let line = format_entry("12:34:56", Level::Err, "boom");
        assert_eq!(line, "[12:34:56 ERR] boom");
    }

    #[test]
    fn format_entry_info_label() {
        assert!(format_entry("00:00:00", Level::Info, "x").contains("INF"));
    }

    // Single test that mutates the global buffer, to avoid cross-test races.
    #[test]
    fn push_is_empty_capacity_and_format_all() {
        assert!(is_empty(), "buffer must start empty in a fresh test process");
        push(Level::Err, "first");
        assert!(!is_empty());
        let all = format_all();
        assert!(all.contains("first"), "got: {all}");
        assert!(all.contains("ERR"), "got: {all}");

        // Overflow capacity: push 120 more, oldest must be evicted.
        for i in 0..120 {
            push(Level::Info, format!("entry {i}"));
        }
        let all = format_all();
        let line_count = all.lines().count();
        assert_eq!(line_count, 100, "buffer must cap at 100 lines, got {line_count}");
        assert!(!all.contains("first"), "oldest entry must be evicted");
        assert!(all.contains("entry 119"), "newest entry must be present");
    }
}
