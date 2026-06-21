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

/// Builds an entry, truncating its message to the per-message byte cap.
fn make_entry(level: Level, msg: String, time: String) -> Entry {
    Entry {
        time,
        level,
        message: truncate(&msg, MAX_MSG_BYTES),
    }
}

/// Appends `entry`, evicting the oldest when at capacity. Operates on a passed
/// buffer so the capacity logic is testable without the process-global state.
fn push_entry(buf: &mut VecDeque<Entry>, entry: Entry) {
    if buf.len() == CAPACITY {
        buf.pop_front();
    }
    buf.push_back(entry);
}

fn format_buf(buf: &VecDeque<Entry>) -> String {
    buf.iter()
        .map(|e| format_entry(&e.time, e.level, &e.message))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn push(level: Level, msg: impl Into<String>) {
    let time = chrono::Local::now().format("%H:%M:%S").to_string();
    let entry = make_entry(level, msg.into(), time);
    push_entry(&mut buffer().lock().unwrap(), entry);
}

pub fn is_empty() -> bool {
    buffer().lock().unwrap().is_empty()
}

pub fn format_all() -> String {
    format_buf(&buffer().lock().unwrap())
}

/// Pushes a diagnostic entry, auto-injecting the call-site `file!():line!()`.
/// Usage: `crate::diag!(crate::diag::Level::Err, "fetch failed: {}", e);`
#[macro_export]
macro_rules! diag {
    ($lvl:expr, $($arg:tt)*) => {
        $crate::diag::push($lvl, format!("[{}:{}] {}", file!(), line!(), format_args!($($arg)*)))
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

    #[test]
    fn make_entry_truncates_message() {
        let e = make_entry(Level::Err, "a".repeat(3000), "12:00:00".to_string());
        assert!(e.message.ends_with("… (truncated)"));
        assert!(e.message.len() <= MAX_MSG_BYTES + "… (truncated)".len());
    }

    // Capacity + formatting tested on a local buffer to avoid races on the
    // process-global static (other tests push to it in parallel).
    #[test]
    fn push_entry_caps_at_capacity_and_evicts_oldest() {
        let mut buf = VecDeque::new();
        for i in 0..120 {
            push_entry(&mut buf, make_entry(Level::Info, format!("entry {i}"), "t".to_string()));
        }
        assert_eq!(buf.len(), CAPACITY, "buffer must cap at {CAPACITY}");
        let all = format_buf(&buf);
        assert_eq!(all.lines().count(), CAPACITY);
        assert!(!all.contains("entry 0"), "oldest entry must be evicted");
        assert!(all.contains("entry 119"), "newest entry must be present");
        assert!(all.contains("INF"), "level label must be present");
    }

    #[test]
    fn format_buf_empty_is_empty_string() {
        assert_eq!(format_buf(&VecDeque::new()), "");
    }
}
