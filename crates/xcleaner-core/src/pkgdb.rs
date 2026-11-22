//! Package databases as evidence for leftovers: what is installed (names,
//! provides, file identities) and when packages were removed. Chosen by
//! which database files exist, not by distro name; several may coexist.
//! Formats: docs/research/DISTRO-PACKAGES-2026-11.md. Only pacman is
//! verified on real data; dpkg and zypper are checked in VMs (v0.3.0).
//! A database without a removal log (dnf history is SQLite, xbps, apk,
//! Nix) contributes installed identities only, so nothing there is ever
//! called a leftover.

use crate::leftovers::{parse_pacman_log, parse_ts, path_identities, EvKind, PkgEvent};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::UNIX_EPOCH,
};

#[derive(Debug, Default, Clone)]
pub struct PkgSystem {
    pub kind: &'static str,
    /// Installed package names.
    pub names: HashSet<String>,
    /// Further identities of installed software: provides, binaries,
    /// desktop IDs, /opt and /etc/xdg names.
    pub identities: Vec<String>,
    /// Package events, oldest first.
    pub events: Vec<PkgEvent>,
}

pub fn detect() -> Vec<PkgSystem> {
    [pacman(), dpkg(), rpm()].into_iter().flatten().collect()
}

// ---------------------------------------------------------------- pacman

fn pacman() -> Option<PkgSystem> {
    let rd = fs::read_dir("/var/lib/pacman/local").ok()?;
    let mut sys = PkgSystem {
        kind: "pacman",
        ..Default::default()
    };
    for e in rd.flatten() {
        let dir = e.path();
        if let Ok(desc) = fs::read_to_string(dir.join("desc")) {
            let mut section = "";
            for l in desc.lines() {
                if l.starts_with('%') {
                    section = l;
                    continue;
                }
                if l.is_empty() {
                    continue;
                }
                match section {
                    "%NAME%" => {
                        sys.names.insert(l.to_string());
                        sys.identities.push(l.to_string());
                    }
                    "%BASE%" => sys.identities.push(l.to_string()),
                    "%PROVIDES%" | "%REPLACES%" => {
                        sys.identities
                            .push(l.split(['=', '<', '>']).next().unwrap_or(l).to_string());
                    }
                    _ => {}
                }
            }
        }
        if let Ok(files) = fs::read_to_string(dir.join("files")) {
            for f in files.lines() {
                sys.identities.extend(path_identities(f));
            }
        }
    }
    if let Ok(log) = fs::read("/var/log/pacman.log") {
        sys.events = parse_pacman_log(&String::from_utf8_lossy(&log));
    }
    Some(sys)
}

// ------------------------------------------------------------------ dpkg

const DPKG_DIR: &str = "/var/lib/dpkg";

/// Seconds east of UTC of the local zone (dpkg and zypper log local time).
fn local_offset() -> i64 {
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&now, &mut tm) }.is_null() {
        return 0;
    }
    tm.tm_gmtoff
}

/// `foo:amd64` → `foo`.
fn strip_arch(p: &str) -> &str {
    p.split(':').next().unwrap_or(p)
}

/// dpkg.log lines: `2025-11-02 10:11:12 remove foo:amd64 1.0-1 <none>`.
/// install / upgrade / remove / purge are the actions that matter.
pub fn parse_dpkg_log(text: &str, off: i64) -> Vec<PkgEvent> {
    text.lines()
        .filter_map(|l| {
            let ts = local_ts_with(l, off)?;
            let mut w = l.get(20..)?.split_whitespace();
            let kind = match w.next()? {
                "install" => EvKind::Install,
                "upgrade" => EvKind::Upgrade,
                "remove" | "purge" => EvKind::Remove,
                _ => return None,
            };
            Some(PkgEvent {
                pkg: strip_arch(w.next()?).to_string(),
                kind,
                ts,
            })
        })
        .collect()
}

/// Paragraphs of /var/lib/dpkg/status: (package, status, provides).
pub fn parse_dpkg_status(text: &str) -> Vec<(String, String, Vec<String>)> {
    let mut out = Vec::new();
    for para in text.split("\n\n") {
        let mut pkg = None;
        let mut status = String::new();
        let mut provides = Vec::new();
        for l in para.lines() {
            if let Some(v) = l.strip_prefix("Package: ") {
                pkg = Some(v.trim().to_string());
            } else if let Some(v) = l.strip_prefix("Status: ") {
                status = v.trim().to_string();
            } else if let Some(v) = l.strip_prefix("Provides: ") {
                provides = v
                    .split(',')
                    .filter_map(|p| p.split_whitespace().next())
                    .map(|p| strip_arch(p).to_string())
                    .collect();
            }
        }
        if let Some(p) = pkg {
            out.push((p, status, provides));
        }
    }
    out
}

/// `info/<pkg>.list` or `info/<pkg>:<arch>.list`.
fn dpkg_list(pkg: &str) -> Option<PathBuf> {
    let info = Path::new(DPKG_DIR).join("info");
    let plain = info.join(format!("{pkg}.list"));
    if plain.exists() {
        return Some(plain);
    }
    fs::read_dir(&info)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&format!("{pkg}:")) && n.ends_with(".list"))
        })
}

/// dpkg.log, dpkg.log.1, dpkg.log.N.gz, oldest first (gzip via the host tool).
fn dpkg_logs() -> Vec<String> {
    let mut files: Vec<(u32, PathBuf)> = fs::read_dir("/var/log")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            let rest = n.strip_prefix("dpkg.log")?;
            let num = match rest.trim_start_matches('.').trim_end_matches(".gz") {
                "" => 0,
                d => d.parse().ok()?,
            };
            Some((num, e.path()))
        })
        .collect();
    files.sort_by_key(|a| std::cmp::Reverse(a.0));
    files
        .into_iter()
        .filter_map(|(_, p)| {
            if p.extension().is_some_and(|x| x == "gz") {
                let o = crate::host_command("gzip")
                    .arg("-dc")
                    .arg(&p)
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .ok()?;
                o.status
                    .success()
                    .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
            } else {
                fs::read(&p)
                    .ok()
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
            }
        })
        .collect()
}

fn dpkg() -> Option<PkgSystem> {
    let status = fs::read_to_string(Path::new(DPKG_DIR).join("status")).ok()?;
    let mut sys = PkgSystem {
        kind: "dpkg",
        ..Default::default()
    };
    let off = local_offset();
    for log in dpkg_logs() {
        sys.events.extend(parse_dpkg_log(&log, off));
    }
    let mut rc = Vec::new();
    for (pkg, st, provides) in parse_dpkg_status(&status) {
        if st.ends_with(" installed") {
            sys.identities.push(pkg.clone());
            sys.identities.extend(provides);
            if let Some(list) = dpkg_list(&pkg).and_then(|l| fs::read_to_string(l).ok()) {
                for f in list.lines() {
                    sys.identities.extend(path_identities(f));
                }
            }
            sys.names.insert(pkg);
        } else if st.ends_with(" config-files") {
            rc.push(pkg);
        }
    }
    // Removed but config kept ("rc"): if the log no longer has the removal,
    // its .list file (kept with the leftover conffiles) dates it.
    let logged: HashSet<&str> = sys.events.iter().map(|e| e.pkg.as_str()).collect();
    let mut extra = Vec::new();
    for pkg in rc.iter().filter(|p| !logged.contains(p.as_str())) {
        let Some(t) = dpkg_list(pkg)
            .and_then(|l| fs::metadata(l).ok())
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        else {
            continue;
        };
        extra.push(PkgEvent {
            pkg: pkg.clone(),
            kind: EvKind::Remove,
            ts: t.as_secs() as i64,
        });
    }
    extra.sort_by_key(|e| e.ts);
    sys.events.extend(extra);
    Some(sys)
}

// ----------------------------------------------------------- rpm / zypper

const ZYPP_HISTORY: &str = "/var/log/zypp/history";

/// `2025-11-02 10:11:12|remove |foo|1.0-1.1|x86_64|root@host|…`.
pub fn parse_zypp_history(text: &str, off: i64) -> Vec<PkgEvent> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('|');
            let ts = local_ts_with(f.next()?, off)?;
            let kind = match f.next()?.trim() {
                "install" => EvKind::Install,
                "remove" => EvKind::Remove,
                _ => return None,
            };
            Some(PkgEvent {
                pkg: f.next()?.trim().to_string(),
                kind,
                ts,
            })
        })
        .collect()
}

/// `2025-11-02 10:11:12` in a zone `off` seconds east of UTC → unix seconds.
fn local_ts_with(s: &str, off: i64) -> Option<i64> {
    Some(parse_ts(s.get(..19)?)? - off)
}

fn rpm_query(fmt: &str) -> Option<String> {
    let o = crate::host_command("/usr/bin/rpm")
        .args(["-qa", "--qf", fmt])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

fn rpm() -> Option<PkgSystem> {
    if !Path::new("/usr/bin/rpm").exists() {
        return None;
    }
    let mut sys = PkgSystem {
        kind: "rpm",
        ..Default::default()
    };
    // Three queries for the whole database instead of one per package.
    for n in rpm_query("%{NAME}\\n")?.lines() {
        sys.names.insert(n.to_string());
        sys.identities.push(n.to_string());
    }
    if let Some(p) = rpm_query("[%{PROVIDENAME}\\n]") {
        sys.identities.extend(
            p.lines()
                .filter(|l| !l.contains('(') && !l.starts_with('/'))
                .map(str::to_string),
        );
    }
    if let Some(f) = rpm_query("[%{FILENAMES}\\n]") {
        for l in f.lines() {
            sys.identities.extend(path_identities(l));
        }
    }
    // zypper keeps a text history; dnf's is SQLite (not read yet).
    if let Ok(h) = fs::read_to_string(ZYPP_HISTORY) {
        sys.events = parse_zypp_history(&h, local_offset());
    }
    Some(sys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpkg_log_events() {
        let log = "2025-11-01 09:00:00 startup archives unpack\n\
                   2025-11-01 09:00:01 install foo:amd64 <none> 1.0-1\n\
                   2025-11-01 09:00:02 status installed foo:amd64 1.0-1\n\
                   2025-11-02 10:00:00 upgrade foo:amd64 1.0-1 1.1-1\n\
                   2025-11-03 11:00:00 remove foo:amd64 1.1-1 <none>\n\
                   2025-11-03 11:00:05 purge bar:all 2 <none>\n";
        let e = parse_dpkg_log(log, 3600);
        let kinds: Vec<_> = e.iter().map(|x| (x.pkg.as_str(), x.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("foo", EvKind::Install),
                ("foo", EvKind::Upgrade),
                ("foo", EvKind::Remove),
                ("bar", EvKind::Remove)
            ]
        );
        // Local time with a +1 h zone.
        assert_eq!(e[0].ts, parse_ts("2025-11-01T09:00:01+0100").unwrap());
    }

    #[test]
    fn dpkg_status_paragraphs() {
        let st =
            "Package: foo\nStatus: install ok installed\nProvides: foo-bin (= 1), libfoo:amd64\n\n\
                  Package: gone\nStatus: deinstall ok config-files\n\n";
        let p = parse_dpkg_status(st);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].2, vec!["foo-bin".to_string(), "libfoo".into()]);
        assert!(p[1].1.ends_with(" config-files"));
    }

    #[test]
    fn zypp_history_events() {
        let h = "# 2025-11-02 10:00:00 foo.rpm installed ok\n\
                 2025-11-02 10:00:00|install|foo|1.0-1.1|x86_64||repo-oss|abc|\n\
                 2025-11-03 12:00:00|remove |foo|1.0-1.1|x86_64|root@pc|\n\
                 2025-11-03 12:00:01|command|root@pc|'zypper' 'rm' 'foo'|\n";
        let e = parse_zypp_history(h, 0);
        assert_eq!(e.len(), 2);
        assert_eq!((e[1].pkg.as_str(), e[1].kind), ("foo", EvKind::Remove));
    }
}
