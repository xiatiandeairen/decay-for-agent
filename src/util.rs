use std::path::Path;

// --- Path utilities ---

/// Source code file extensions recognized by decay.
const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "swift", "py", "js", "ts", "tsx", "jsx", "mjs", "go", "java",
    "kt", "kts", "rb", "php", "c", "cpp", "cc", "h", "hpp", "m", "mm", "cs",
    "sh", "bash", "zsh",
];

/// Check if a file path has a recognized source code extension.
pub fn is_source_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    SOURCE_EXTENSIONS.contains(&ext.as_str())
}

/// Normalize a path to always use forward slashes (for DB storage).
pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

// --- Time utilities ---

/// Format a unix timestamp as ISO 8601 UTC string.
pub fn format_timestamp(ts: i64) -> String {
    let secs_per_day: i64 = 86400;
    let days = ts / secs_per_day;
    let remaining = ts % secs_per_day;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    let mut y: i64 = 1970;
    let mut d = days;
    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let months_days: &[i64] = if is_leap_year(y) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m: i64 = 1;
    for &md in months_days {
        if d < md {
            break;
        }
        d -= md;
        m += 1;
    }

    format!(
        "{y:04}-{m:02}-{:02}T{hours:02}:{minutes:02}:{seconds:02}Z",
        d + 1
    )
}

/// Format current UTC time.
pub fn now_utc() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    // Same logic as format_timestamp but date-only with HH:MM
    let secs_per_day: u64 = 86400;
    let now_u = now as u64;
    let days = now_u / secs_per_day;
    let remaining = now_u % secs_per_day;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;

    let mut y: u64 = 1970;
    let mut d = days;
    loop {
        let diy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if d < diy {
            break;
        }
        d -= diy;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays: &[u64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m: u64 = 1;
    for &md in mdays {
        if d < md {
            break;
        }
        d -= md;
        m += 1;
    }
    format!("{y:04}-{m:02}-{:02} {hours:02}:{minutes:02} UTC", d + 1)
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_source_file() {
        assert!(is_source_file("src/main.rs"));
        assert!(is_source_file("app.swift"));
        assert!(is_source_file("index.tsx"));
        assert!(!is_source_file("data.json"));
        assert!(!is_source_file("style.css"));
        assert!(!is_source_file("README.md"));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(Path::new("src/main.rs")), "src/main.rs");
        // On Unix this is a no-op, but on Windows it would convert \ to /
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(1767225600), "2026-01-01T00:00:00Z");
    }
}
