//! Package caches of non-pacman distros, through each manager's own clean
//! command (docs/research/DISTRO-ROOT-CLEANING-2026-11.md §1). The probes
//! only stat cache files or run a documented simulation; nothing here could
//! be run on the owner's Arch machine, so every manager is verified in VMs
//! (v0.3.0).

use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Stdio,
};

/// A non-pacman package manager present on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    Apt,
    Dnf5,
    Dnf,
    Zypper,
    Xbps,
    Apk,
    Eclean,
}

impl Manager {
    fn binary(self) -> &'static str {
        match self {
            Manager::Apt => "/usr/bin/apt-get",
            Manager::Dnf5 => "/usr/bin/dnf5",
            Manager::Dnf => "/usr/bin/dnf",
            Manager::Zypper => "/usr/bin/zypper",
            Manager::Xbps => "/usr/bin/xbps-remove",
            Manager::Apk => "/sbin/apk",
            Manager::Eclean => "/usr/bin/eclean",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Manager::Apt => "apt",
            Manager::Dnf5 => "dnf5",
            Manager::Dnf => "dnf",
            Manager::Zypper => "zypper",
            Manager::Xbps => "xbps",
            Manager::Apk => "apk",
            Manager::Eclean => "eclean (Gentoo)",
        }
    }

    /// Clean command (root), without the binary.
    fn clean_args(self) -> &'static [&'static str] {
        match self {
            Manager::Apt => &["autoclean"],
            Manager::Dnf5 | Manager::Dnf => &["clean", "packages"],
            // Only packages: --all / --metadata would force a full refresh.
            Manager::Zypper => &["--non-interactive", "clean"],
            Manager::Xbps => &["-O", "-y"],
            Manager::Apk => &["cache", "clean"],
            Manager::Eclean => &["distfiles"],
        }
    }

    /// Cache directories the command works in.
    fn cache_dirs(self) -> Vec<PathBuf> {
        let p = |s: &str| PathBuf::from(s);
        match self {
            Manager::Apt => vec![p("/var/cache/apt/archives")],
            Manager::Dnf5 => vec![p("/var/cache/libdnf5")],
            Manager::Dnf => vec![p("/var/cache/dnf")],
            Manager::Zypper => vec![p("/var/cache/zypp/packages")],
            Manager::Xbps => vec![p("/var/cache/xbps")],
            Manager::Apk => vec![p("/etc/apk/cache")],
            Manager::Eclean => vec![p("/var/cache/distfiles"), p("/usr/portage/distfiles")],
        }
    }

    /// Package file suffixes in the cache.
    fn pkg_suffixes(self) -> &'static [&'static str] {
        match self {
            Manager::Apt => &[".deb"],
            Manager::Dnf5 | Manager::Dnf | Manager::Zypper => &[".rpm"],
            Manager::Xbps => &[".xbps"],
            Manager::Apk => &[".apk"],
            Manager::Eclean => &[""],
        }
    }

    /// The probe size is exact (the command empties the cache) or only an
    /// upper bound (it removes the obsolete part).
    pub fn exact(self) -> bool {
        matches!(self, Manager::Dnf5 | Manager::Dnf | Manager::Zypper)
    }
}

/// Managers present. dnf5 wins over the dnf4 compat name; pacman systems
/// are handled by paccache in root.rs and are not listed here.
pub fn managers() -> Vec<Manager> {
    let mut v: Vec<Manager> = [
        Manager::Apt,
        Manager::Dnf5,
        Manager::Dnf,
        Manager::Zypper,
        Manager::Xbps,
        Manager::Apk,
        Manager::Eclean,
    ]
    .into_iter()
    .filter(|m| Path::new(m.binary()).exists())
    .collect();
    if v.contains(&Manager::Dnf5) {
        v.retain(|m| *m != Manager::Dnf);
    }
    v
}

/// Regular package files below `dir` (bounded depth, no symlinks).
fn cached_files(dir: &Path, suffixes: &[&str], depth: usize, out: &mut Vec<(PathBuf, u64)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(m) = fs::symlink_metadata(e.path()) else {
            continue;
        };
        let name = e.file_name().to_string_lossy().to_string();
        if m.is_dir() && depth > 0 && name != "partial" {
            cached_files(&e.path(), suffixes, depth - 1, out);
        } else if m.is_file() && name != "lock" && suffixes.iter().any(|s| name.ends_with(s)) {
            out.push((e.path(), m.blocks() * 512));
        }
    }
}

fn cache_size(m: Manager) -> (u64, u64) {
    let mut files = Vec::new();
    for d in m.cache_dirs() {
        cached_files(&d, m.pkg_suffixes(), 4, &mut files);
    }
    (files.iter().map(|(_, b)| b).sum(), files.len() as u64)
}

/// `Del <pkg> <ver> [<size>]` lines of `apt-get -s autoclean` (simulation
/// needs no root and takes no lock). Returns the package/version pairs.
pub fn parse_apt_autoclean(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            (w.next()? == "Del").then_some(())?;
            Some((w.next()?.to_string(), w.next()?.to_string()))
        })
        .collect()
}

/// apt stores `pkg_<ver with : as %3a>_<arch>.deb`.
fn apt_deb_prefix(pkg: &str, ver: &str) -> String {
    format!("{pkg}_{}_", ver.replace(':', "%3a"))
}

fn apt_autoclean_size() -> (u64, u64) {
    let Ok(out) = crate::host_command(Manager::Apt.binary())
        .args(["-s", "autoclean"])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return (0, 0);
    };
    let dels = parse_apt_autoclean(&String::from_utf8_lossy(&out.stdout));
    let mut files = Vec::new();
    cached_files(
        Path::new("/var/cache/apt/archives"),
        &[".deb"],
        0,
        &mut files,
    );
    let bytes = files
        .iter()
        .filter(|(p, _)| {
            let n = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            dels.iter()
                .any(|(pkg, ver)| n.starts_with(&apt_deb_prefix(pkg, ver)))
        })
        .map(|(_, b)| b)
        .sum();
    (bytes, dels.len() as u64)
}

/// Probe for the regular task: (bytes, files, detail).
pub fn probe() -> Option<(u64, u64, String)> {
    let ms = managers();
    let mut bytes = 0;
    let mut files = 0;
    let mut parts = Vec::new();
    for m in &ms {
        let (b, n) = if *m == Manager::Apt {
            apt_autoclean_size()
        } else {
            cache_size(*m)
        };
        if n == 0 {
            continue;
        }
        bytes += b;
        files += n;
        parts.push(if m.exact() {
            trf!("{}: {n} packages", "{}: {n} paczek", m.label())
        } else {
            trf!(
                "{}: {n} files, frees up to {}",
                "{}: {n} plików, zwolni do {}",
                m.label(),
                crate::format_size(b)
            )
        });
    }
    (files > 0).then(|| (bytes, files, parts.join("; ")))
}

/// Probe for `apt-get clean` (everything downloaded; opt-in).
pub fn probe_apt_clean() -> Option<(u64, u64, String)> {
    if !managers().contains(&Manager::Apt) {
        return None;
    }
    let (b, n) = cache_size(Manager::Apt);
    (n > 0).then(|| {
        (
            b,
            n,
            trf!("{n} downloaded .deb packages", "{n} pobranych paczek .deb"),
        )
    })
}

fn run(bin: &str, args: &[&str]) -> Result<(), String> {
    match crate::host_command(bin)
        .args(args)
        .stdin(Stdio::null())
        .status()
    {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(trf!(
            "{bin}: code {}",
            "{bin}: kod {}",
            s.code().unwrap_or(-1)
        )),
        Err(e) => Err(format!("{bin}: {e}")),
    }
}

fn busy() -> Option<(u64, Vec<String>)> {
    crate::locks::busy_package_manager().map(|pm| {
        (
            0,
            vec![trf!(
                "a package manager is running: {pm} — skipped",
                "działa menedżer pakietów: {pm} — pominięto"
            )],
        )
    })
}

pub fn root_clean() -> (u64, Vec<String>) {
    crate::root::say(tr!(
        "• Package manager cache…",
        "• Cache menedżera pakietów…"
    ));
    if let Some(r) = busy() {
        return r;
    }
    let mut freed = 0;
    let mut errs = Vec::new();
    for m in managers() {
        crate::root::say(&format!("  {} {}", m.binary(), m.clean_args().join(" ")));
        let before = cache_size(m).0;
        if let Err(e) = run(m.binary(), m.clean_args()) {
            errs.push(e);
        }
        freed += before.saturating_sub(cache_size(m).0);
    }
    (freed, errs)
}

pub fn root_apt_clean() -> (u64, Vec<String>) {
    crate::root::say("• apt-get clean…");
    if let Some(r) = busy() {
        return r;
    }
    let before = cache_size(Manager::Apt).0;
    let errs = run(Manager::Apt.binary(), &["clean"])
        .err()
        .into_iter()
        .collect();
    (before.saturating_sub(cache_size(Manager::Apt).0), errs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apt_simulation_and_file_names() {
        let out = "Reading package lists...\nDel libfoo1 1:2.3-1 [123 kB]\nDel bar 0.9 [1 MB]\n";
        assert_eq!(
            parse_apt_autoclean(out),
            vec![
                ("libfoo1".into(), "1:2.3-1".into()),
                ("bar".into(), "0.9".into())
            ]
        );
        assert_eq!(apt_deb_prefix("libfoo1", "1:2.3-1"), "libfoo1_1%3a2.3-1_");
    }

    #[test]
    fn cache_walk_skips_partial_lock_and_symlinks() {
        let d = std::env::temp_dir().join(format!("xc-pkgc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("repo/packages")).unwrap();
        fs::create_dir_all(d.join("partial")).unwrap();
        fs::write(d.join("repo/packages/a.rpm"), b"x").unwrap();
        fs::write(d.join("partial/b.rpm"), b"x").unwrap();
        fs::write(d.join("lock"), b"").unwrap();
        std::os::unix::fs::symlink(d.join("repo/packages/a.rpm"), d.join("c.rpm")).unwrap();
        let mut v = Vec::new();
        cached_files(&d, &[".rpm"], 4, &mut v);
        assert_eq!(v.len(), 1);
        assert!(v[0].0.ends_with("repo/packages/a.rpm"));
        fs::remove_dir_all(&d).unwrap();
    }
}
