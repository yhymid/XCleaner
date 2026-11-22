//! Is a package manager running? Read-only, no root needed.
//!
//! pacman marks a transaction with the existence of `db.lck`. Every other
//! package manager locks a file that always exists (fcntl or flock), so its
//! presence means nothing: the lock itself is looked up in `/proc/locks`
//! (mode 0444) by inode. Matching by inode only (btrfs reports a different
//! device in stat than in /proc/locks) can only err towards "busy", which
//! skips a task and never deletes more. Sources:
//! docs/research/DISTRO-PACKAGES-2026-11.md, DISTRO-ROOT-CLEANING-2026-11.md.

use std::{fs, os::unix::fs::MetadataExt, path::Path};

const PACMAN_LOCK: &str = "/var/lib/pacman/db.lck";

/// (package manager, lock file locked with fcntl/flock).
const LOCKED_FILES: &[(&str, &str)] = &[
    ("apt", "/var/lib/dpkg/lock-frontend"),
    ("dpkg", "/var/lib/dpkg/lock"),
    ("apt", "/var/lib/apt/lists/lock"),
    ("apt", "/var/cache/apt/archives/lock"),
    ("rpm", "/var/lib/rpm/.rpm.lock"),
    ("rpm", "/usr/lib/sysimage/rpm/.rpm.lock"),
    ("dnf5", "/run/dnf/rpmtransaction.lock"),
    ("xbps", "/var/db/xbps/lock"),
    ("apk", "/lib/apk/db/lock"),
    ("portage", "/var/db/.pkg.portage_lockfile"),
];

/// zypper writes its PID here while it runs.
const ZYPP_PID: &str = "/run/zypp.pid";

/// Inode numbers that hold a lock, from `/proc/locks` text. Lines look like
/// `1: POSIX  ADVISORY  WRITE 4767 00:20:25501875 0 EOF`; waiters start with
/// ` -> ` and hold nothing.
fn locked_inodes(text: &str) -> std::collections::HashSet<u64> {
    text.lines()
        .filter(|l| !l.contains("->"))
        .filter_map(|l| {
            let dev_ino = l.split_whitespace().nth(5)?;
            dev_ino.rsplit(':').next()?.parse().ok()
        })
        .collect()
}

fn pid_alive_in(text: &str) -> bool {
    text.trim()
        .parse::<u32>()
        .is_ok_and(|p| p > 0 && Path::new("/proc").join(p.to_string()).exists())
}

/// The package manager that is running right now, if any.
pub fn busy_package_manager() -> Option<String> {
    busy_in(
        Path::new(PACMAN_LOCK),
        &fs::read_to_string("/proc/locks").unwrap_or_default(),
        LOCKED_FILES,
        Path::new(ZYPP_PID),
    )
}

fn busy_in(
    pacman_lock: &Path,
    proc_locks: &str,
    files: &[(&str, &str)],
    zypp_pid: &Path,
) -> Option<String> {
    if pacman_lock.exists() {
        return Some(format!("pacman ({})", pacman_lock.display()));
    }
    let held = locked_inodes(proc_locks);
    if !held.is_empty() {
        for (pm, f) in files {
            if fs::metadata(f).is_ok_and(|m| held.contains(&m.ino())) {
                return Some(format!("{pm} ({f})"));
            }
        }
    }
    if fs::read_to_string(zypp_pid).is_ok_and(|t| pid_alive_in(&t)) {
        return Some(format!("zypper ({})", zypp_pid.display()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_locks() {
        let t = "1: POSIX  ADVISORY  WRITE 4767 00:20:25501875 0 EOF\n\
                 2: FLOCK  ADVISORY  WRITE 99 fd:01:77 0 EOF\n\
                 2: -> FLOCK  ADVISORY  WRITE 100 fd:01:88 0 EOF\n";
        let s = locked_inodes(t);
        assert!(s.contains(&25501875) && s.contains(&77));
        assert!(!s.contains(&88), "a waiter holds nothing");
    }

    #[test]
    fn detects_each_kind_of_lock() {
        let d = std::env::temp_dir().join(format!("xc-locks-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        let lock = d.join("lock-frontend");
        fs::write(&lock, b"").unwrap();
        let ino = fs::metadata(&lock).unwrap().ino();
        let lock_s = lock.to_string_lossy().to_string();
        let files = [("apt", lock_s.as_str())];
        let none = d.join("missing");
        // Lock file exists but nobody holds it.
        assert_eq!(busy_in(&none, "", &files, &none), None);
        // Held per /proc/locks.
        let pl = format!("1: POSIX  ADVISORY  WRITE 1 00:20:{ino} 0 EOF\n");
        assert!(busy_in(&none, &pl, &files, &none)
            .unwrap()
            .starts_with("apt"));
        // pacman: the file's existence.
        assert!(busy_in(&lock, "", &[], &none)
            .unwrap()
            .starts_with("pacman"));
        // zypper: a live PID.
        let zp = d.join("zypp.pid");
        fs::write(&zp, std::process::id().to_string()).unwrap();
        assert!(busy_in(&none, "", &[], &zp).unwrap().starts_with("zypper"));
        fs::write(&zp, "0").unwrap();
        assert_eq!(busy_in(&none, "", &[], &zp), None);
        fs::remove_dir_all(&d).unwrap();
    }
}
