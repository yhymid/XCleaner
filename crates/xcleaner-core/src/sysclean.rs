//! System tasks that work on every systemd / Flatpak distro: old journal
//! files, rotated logs in /var/log and unused Flatpak runtimes.
//! Facts and commands: docs/research/DISTRO-ROOT-CLEANING-2026-11.md §4, §6.
//! Probes run as the user and only read; the root side recomputes targets.

use crate::fsops::Home;
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime},
};

// -------------------------------------------------------------- journal

const JOURNAL_DIR: &str = "/var/log/journal";
const JOURNALCTL: &str = "/usr/bin/journalctl";
/// What the task passes to `journalctl --vacuum-time=`.
pub const JOURNAL_KEEP: &str = "4weeks";
const JOURNAL_KEEP_SECS: u64 = 28 * 86_400;

/// Archived journal files (`name@….journal`, `….journal~`) older than the
/// vacuum age. Active files are never touched by --vacuum-time.
fn old_journal_files() -> Vec<(PathBuf, u64)> {
    if !Path::new(JOURNALCTL).exists() {
        return Vec::new();
    }
    let cutoff = SystemTime::now() - Duration::from_secs(JOURNAL_KEEP_SECS);
    let mut out = Vec::new();
    let Ok(machines) = fs::read_dir(JOURNAL_DIR) else {
        return out;
    };
    for m in machines.flatten() {
        let Ok(rd) = fs::read_dir(m.path()) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let archived =
                name.contains('@') && (name.ends_with(".journal") || name.ends_with(".journal~"));
            let Ok(meta) = fs::symlink_metadata(e.path()) else {
                continue;
            };
            if archived && meta.is_file() && meta.modified().is_ok_and(|t| t < cutoff) {
                out.push((e.path(), meta.blocks() * 512));
            }
        }
    }
    out
}

pub fn probe_journal() -> Option<(u64, u64, String)> {
    let f = old_journal_files();
    let bytes: u64 = f.iter().map(|(_, b)| b).sum();
    (!f.is_empty()).then(|| {
        (
            bytes,
            f.len() as u64,
            format!("{} archiwalnych plików starszych niż 4 tygodnie", f.len()),
        )
    })
}

pub fn root_journal() -> (u64, Vec<String>) {
    crate::root::say(&format!("• Dziennik systemd: starsze niż {JOURNAL_KEEP}…"));
    let before: u64 = old_journal_files().iter().map(|(_, b)| b).sum();
    let out = crate::host_command(JOURNALCTL)
        .arg(format!("--vacuum-time={JOURNAL_KEEP}"))
        .stdin(Stdio::null())
        .status();
    let after: u64 = old_journal_files().iter().map(|(_, b)| b).sum();
    match out {
        Ok(s) if s.success() => (before.saturating_sub(after), Vec::new()),
        Ok(s) => (
            before.saturating_sub(after),
            vec![format!("journalctl: kod {}", s.code().unwrap_or(-1))],
        ),
        Err(e) => (0, vec![format!("journalctl: {e}")]),
    }
}

// --------------------------------------------------------- rotated logs

const LOG_DIR: &str = "/var/log";
/// Never touched: audit trail, installer logs, journal, login records.
const LOG_SKIP_DIRS: &[&str] = &["journal", "audit", "installer", "anaconda", "private"];
const LOG_SKIP_PREFIX: &[&str] = &["wtmp", "btmp", "lastlog", "faillog"];
const COMPRESSED: &[&str] = &[".gz", ".xz", ".zst", ".bz2"];

/// `boot.log.3`, `syslog.2.gz`, `messages-20251102(.gz)`, `Xorg.0.log.old`.
/// A bare `x.gz` is not matched: it can be an active log.
pub fn is_rotated_log(name: &str) -> bool {
    if LOG_SKIP_PREFIX.iter().any(|p| name.starts_with(p)) {
        return false;
    }
    let base = COMPRESSED
        .iter()
        .find_map(|c| name.strip_suffix(c))
        .unwrap_or(name);
    if base.ends_with(".old") && base.len() > 4 {
        return true;
    }
    if let Some((stem, n)) = base.rsplit_once('.') {
        if !stem.is_empty()
            && !n.is_empty()
            && n.len() <= 3
            && n.bytes().all(|b| b.is_ascii_digit())
        {
            return true;
        }
    }
    if let Some((stem, d)) = base.rsplit_once('-') {
        if !stem.is_empty()
            && d.len() == 8
            && d.bytes().all(|b| b.is_ascii_digit())
            && d.starts_with("20")
        {
            return true;
        }
    }
    false
}

/// Rotated logs, relative to /var/log: top level and one level of
/// subdirectories owned by root. Regular files with one link only.
fn rotated_logs() -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    let root = Path::new(LOG_DIR);
    let mut dirs = vec![PathBuf::new()];
    if let Ok(rd) = fs::read_dir(root) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let Ok(m) = fs::symlink_metadata(e.path()) else {
                continue;
            };
            if m.is_dir()
                && m.uid() == 0
                && m.mode() & 0o022 == 0
                && !LOG_SKIP_DIRS.contains(&name.as_str())
            {
                dirs.push(PathBuf::from(name));
            }
        }
    }
    for d in dirs {
        let Ok(rd) = fs::read_dir(root.join(&d)) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let Ok(m) = fs::symlink_metadata(e.path()) else {
                continue;
            };
            if m.is_file() && m.nlink() == 1 && m.uid() == 0 && is_rotated_log(&name) {
                out.push((d.join(&name), m.blocks() * 512));
            }
        }
    }
    out.sort();
    out
}

pub fn probe_rotated_logs() -> Option<(u64, u64, String)> {
    let f = rotated_logs();
    let bytes: u64 = f.iter().map(|(_, b)| b).sum();
    let names: Vec<String> = f
        .iter()
        .take(6)
        .map(|(p, _)| p.display().to_string())
        .collect();
    let more = f.len().saturating_sub(6);
    (!f.is_empty()).then(|| {
        let mut d = names.join(", ");
        if more > 0 {
            d.push_str(&format!(" … i {more} więcej"));
        }
        (bytes, f.len() as u64, d)
    })
}

pub fn root_rotated_logs() -> (u64, Vec<String>) {
    crate::root::say("• Stare, zrotowane logi w /var/log…");
    let base = match Home::open(Path::new(LOG_DIR)) {
        Ok(b) => b,
        Err(e) => return (0, vec![format!("{LOG_DIR}: {e}")]),
    };
    let mut freed = 0;
    let mut errs = Vec::new();
    for (rel, _) in rotated_logs() {
        // openat2 beneath /var/log, no symlinks, root-owned only.
        match base.remove_file(&rel) {
            Ok(r) => {
                freed += r.freed.bytes;
                errs.extend(
                    r.errors
                        .into_iter()
                        .map(|e| format!("{}: {e}", rel.display())),
                );
            }
            Err(e) => errs.push(format!("{}: {e}", rel.display())),
        }
    }
    crate::root::say(&format!("  zwolniono {}", crate::format_size(freed)));
    (freed, errs)
}

// --------------------------------------------------------------- flatpak

const FLATPAK: &str = "/usr/bin/flatpak";
const FLATPAK_SYSTEM: &str = "/var/lib/flatpak";

/// Refs `flatpak uninstall --unused --system` would remove. flatpak has no
/// dry run; answering "n" to its prompt cancels (checked on the Deck).
/// Output rows: ` 1.\t   \torg.gnome.Platform\t51\tr`.
pub fn parse_unused(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|l| {
            let mut f = l.split('\t').map(str::trim).filter(|s| !s.is_empty());
            let num = f.next()?;
            if !(num.ends_with('.') && num[..num.len() - 1].bytes().all(|b| b.is_ascii_digit())) {
                return None;
            }
            let id = f.next()?.to_string();
            let branch = f.next()?.to_string();
            id.contains('.').then_some((id, branch))
        })
        .collect()
}

fn unused_flatpak_refs() -> Vec<(String, String)> {
    if !Path::new(FLATPAK).exists() || !Path::new(FLATPAK_SYSTEM).join("runtime").is_dir() {
        return Vec::new();
    }
    let Ok(mut child) = crate::host_command(FLATPAK)
        .args(["uninstall", "--unused", "--system"])
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Vec::new();
    };
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(b"n\n");
    }
    child
        .wait_with_output()
        .map(|o| parse_unused(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

/// Approximate size of a runtime deploy (its files are hard links into the
/// shared ostree repo, so this is what goes once no other ref uses them).
fn runtime_bytes(id: &str, branch: &str) -> u64 {
    let dir = Path::new(FLATPAK_SYSTEM).join("runtime").join(id);
    let Ok(base) = Home::open(&dir) else { return 0 };
    fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|arch| {
            let rel = Path::new(&arch.file_name()).join(branch);
            base.measure(&rel).ok().flatten()
        })
        .map(|u| u.bytes)
        .sum()
}

pub fn probe_flatpak_unused() -> Option<(u64, u64, String)> {
    let refs = unused_flatpak_refs();
    let bytes = refs.iter().map(|(i, b)| runtime_bytes(i, b)).sum();
    (!refs.is_empty()).then(|| {
        let names: Vec<String> = refs.iter().map(|(i, b)| format!("{i} {b}")).collect();
        (bytes, refs.len() as u64, names.join(", "))
    })
}

fn free_bytes(path: &str) -> Option<u64> {
    let c = std::ffi::CString::new(path).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    (unsafe { libc::statvfs(c.as_ptr(), &mut s) } == 0)
        .then(|| s.f_bavail as u64 * s.f_frsize as u64)
}

pub fn root_flatpak_unused() -> (u64, Vec<String>) {
    crate::root::say("• Nieużywane runtime’y Flatpak…");
    if !Path::new(FLATPAK).exists() {
        return (0, Vec::new());
    }
    let before = free_bytes(FLATPAK_SYSTEM);
    let st = crate::host_command(FLATPAK)
        .args([
            "uninstall",
            "--unused",
            "--system",
            "-y",
            "--noninteractive",
        ])
        .stdin(Stdio::null())
        .status();
    // flatpak reports no sizes: the change in free space is the best measure.
    let freed = match (before, free_bytes(FLATPAK_SYSTEM)) {
        (Some(b), Some(a)) => a.saturating_sub(b),
        _ => 0,
    };
    match st {
        Ok(s) if s.success() => (freed, Vec::new()),
        Ok(s) => (
            freed,
            vec![format!("flatpak: kod {}", s.code().unwrap_or(-1))],
        ),
        Err(e) => (0, vec![format!("flatpak: {e}")]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotated_log_names() {
        for n in [
            "boot.log.1",
            "syslog.2.gz",
            "messages-20251102",
            "secure-20251102.xz",
            "Xorg.0.log.old",
            "dpkg.log.12.zst",
        ] {
            assert!(is_rotated_log(n), "{n}");
        }
        for n in [
            "boot.log",
            "app.gz",
            "wtmp.1",
            "btmp.1",
            "Xorg.0.log",
            "pacman.log",
            ".old",
            "x-1234",
        ] {
            assert!(!is_rotated_log(n), "{n}");
        }
    }

    #[test]
    fn flatpak_unused_output() {
        let out = "\n\n 1.\t   \torg.freedesktop.Platform.GL.default\t26.08\tr\n 2.\t   \torg.gnome.Platform\t51\tr\n\nProceed with these changes to the system installation? [Y/n]: n\n";
        assert_eq!(
            parse_unused(out),
            vec![
                (
                    "org.freedesktop.Platform.GL.default".to_string(),
                    "26.08".to_string()
                ),
                ("org.gnome.Platform".to_string(), "51".to_string()),
            ]
        );
        assert!(parse_unused("Nothing unused to uninstall\n").is_empty());
    }
}
