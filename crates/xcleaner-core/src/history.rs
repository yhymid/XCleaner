//! Persistent cleaning history in `$XDG_STATE_HOME/xcleaner/history.log`:
//! one line per cleaned item, readable by a human and by an AI debugging.

use std::{fs, io::Write, path::PathBuf};

pub fn state_dir() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".local/state"));
    Some(base.join("xcleaner"))
}

/// Seconds since the epoch → `2026-11-22 01:02:03` (UTC).
pub fn stamp(ts: i64) -> String {
    let date = crate::leftovers::format_date(ts);
    let s = ts.rem_euclid(86_400);
    format!(
        "{date} {:02}:{:02}:{:02} UTC",
        s / 3600,
        s % 3600 / 60,
        s % 60
    )
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Append lines; errors are ignored (history must never block cleaning).
pub fn append(lines: &[String]) {
    if std::env::var_os("XCLEANER_NO_HISTORY").is_some() {
        return;
    }
    let Some(dir) = state_dir() else { return };
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("history.log"))
    {
        for l in lines {
            let _ = writeln!(f, "{l}");
        }
    }
}
