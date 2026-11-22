//! Phase 2: system cleaning as root, authenticated with `sudo` in a terminal
//! (same model as XUninstaller; no polkit).
//!
//! ```text
//! GUI (user) ── terminal ── sudo env APPIMAGELAUNCHER_DISABLE=1 <self>
//!                              --root-clean --report <file> <task>…
//! ```
//! The root process receives only task NAMES. It enumerates and validates
//! every target itself (it never trusts paths from the GUI), deletes through
//! the descriptor engine and writes a short report into a file the GUI
//! created in its private XDG_RUNTIME_DIR (checked: regular, owned by
//! the invoking user (--uid, cross-checked with SUDO_UID/DOAS_USER), no symlink).

use crate::fsops::{Home, Usage};
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

pub const ROOT_ARG: &str = "--root-clean";
const COREDUMP_DIR: &str = "/var/lib/systemd/coredump";
const MODULES_DIR: &str = "/usr/lib/modules";
const PACCACHE: &str = "/usr/bin/paccache";
const PKG_DIR: &str = "/var/cache/pacman/pkg";
const SYNC_DIR: &str = "/var/lib/pacman/sync";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootTask {
    /// systemd-coredump crash dumps.
    Coredumps,
    /// `paccache -rk2` + `paccache -ruk0`.
    PacmanCache,
    /// `paccache -rk0`: also the installed versions (opt-in only).
    PacmanCacheAll,
    /// /usr/lib/modules/<ver> of kernels no longer installed.
    KernelModules,
    /// `download-XXXXXX` dirs pacman 7 leaves after interrupted downloads.
    PacmanDownloads,
    /// Sync databases of repositories no longer in pacman.conf (`pacman -Sc`).
    UnusedSyncDbs,
    /// `journalctl --vacuum-time=4weeks` (archived journal files only).
    JournalVacuum,
    /// Rotated logs in /var/log (`*.N`, `*.N.gz`, `*-YYYYMMDD`, `*.old`).
    RotatedLogs,
    /// `flatpak uninstall --unused --system`.
    FlatpakUnused,
    /// Cache of apt / dnf / zypper / xbps / apk / eclean (their own clean).
    PkgCache,
    /// `apt-get clean`: every downloaded .deb (opt-in only).
    AptClean,
    /// apport reports in /var/crash, ABRT problems via its CLI.
    CrashReports,
}

impl RootTask {
    pub const ALL: [RootTask; 12] = [
        RootTask::Coredumps,
        RootTask::PacmanCache,
        RootTask::PacmanCacheAll,
        RootTask::KernelModules,
        RootTask::PacmanDownloads,
        RootTask::UnusedSyncDbs,
        RootTask::JournalVacuum,
        RootTask::RotatedLogs,
        RootTask::FlatpakUnused,
        RootTask::PkgCache,
        RootTask::AptClean,
        RootTask::CrashReports,
    ];

    /// Where the task works (for display).
    pub fn location(self) -> &'static str {
        match self {
            RootTask::Coredumps => COREDUMP_DIR,
            RootTask::PacmanCache | RootTask::PacmanCacheAll | RootTask::PacmanDownloads => PKG_DIR,
            RootTask::KernelModules => MODULES_DIR,
            RootTask::UnusedSyncDbs => SYNC_DIR,
            RootTask::JournalVacuum => "/var/log/journal",
            RootTask::RotatedLogs => "/var/log",
            RootTask::FlatpakUnused => "/var/lib/flatpak",
            RootTask::PkgCache => "/var/cache",
            RootTask::AptClean => "/var/cache/apt/archives",
            RootTask::CrashReports => "/var/crash",
        }
    }

    pub fn arg(self) -> &'static str {
        match self {
            RootTask::Coredumps => "coredumps",
            RootTask::PacmanCache => "pacman-cache",
            RootTask::PacmanCacheAll => "pacman-cache-all",
            RootTask::KernelModules => "kernel-modules",
            RootTask::PacmanDownloads => "pacman-downloads",
            RootTask::UnusedSyncDbs => "unused-sync-dbs",
            RootTask::JournalVacuum => "journal-vacuum",
            RootTask::RotatedLogs => "rotated-logs",
            RootTask::FlatpakUnused => "flatpak-unused",
            RootTask::PkgCache => "pkg-cache",
            RootTask::AptClean => "apt-clean",
            RootTask::CrashReports => "crash-reports",
        }
    }

    /// Works on the package cache or /usr: never on an immutable root.
    pub fn touches_system_image(self) -> bool {
        matches!(
            self,
            RootTask::PacmanCache
                | RootTask::PacmanCacheAll
                | RootTask::PacmanDownloads
                | RootTask::UnusedSyncDbs
                | RootTask::KernelModules
                | RootTask::PkgCache
                | RootTask::AptClean
        )
    }

    pub fn from_arg(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.arg() == s)
    }
}

/// What an unprivileged scan can see about one task.
#[derive(Debug, Clone)]
pub struct Probe {
    pub task: RootTask,
    pub bytes: u64,
    pub files: u64,
    pub detail: String,
}

// ------------------------------------------------------------ coredumps

/// `core.cs2.1000.<boot>.<pid>.<usec>.zst` → `cs2` (systemd escapes `-` as `\x2d`).
fn coredump_comm(name: &str) -> Option<String> {
    let rest = name.strip_prefix("core.")?;
    let comm = rest.split('.').next()?;
    Some(comm.replace("\\x2d", "-"))
}

fn coredump_files() -> Vec<(String, fs::Metadata)> {
    let Ok(rd) = fs::read_dir(COREDUMP_DIR) else {
        return Vec::new();
    };
    let mut v: Vec<_> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let m = fs::symlink_metadata(e.path()).ok()?;
            (name.starts_with("core.") && m.is_file()).then_some((name, m))
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn probe_coredumps() -> Option<Probe> {
    let files = coredump_files();
    if files.is_empty() {
        return None;
    }
    let mut by_comm: Vec<(String, u64)> = Vec::new();
    let mut bytes = 0;
    for (name, m) in &files {
        let b = m.blocks() * 512;
        bytes += b;
        let comm = coredump_comm(name).unwrap_or_else(|| "?".into());
        match by_comm.iter_mut().find(|(c, _)| *c == comm) {
            Some((_, s)) => *s += b,
            None => by_comm.push((comm, b)),
        }
    }
    by_comm.sort_by_key(|(_, b)| std::cmp::Reverse(*b));
    let detail = by_comm
        .iter()
        .take(6)
        .map(|(c, b)| format!("{c} {}", crate::format_size(*b)))
        .collect::<Vec<_>>()
        .join(", ");
    Some(Probe {
        task: RootTask::Coredumps,
        bytes,
        files: files.len() as u64,
        detail,
    })
}

// --------------------------------------------------------- pacman cache

/// `==> finished dry run: 12 candidates (disk space saved: 300.12 MiB)`
fn parse_paccache(out: &str) -> (u64, u64) {
    for l in out.lines() {
        let Some(i) = l.find("disk space saved: ") else {
            continue;
        };
        let rest = &l[i + "disk space saved: ".len()..];
        let mut it = rest.trim_end_matches(')').split_whitespace();
        let (Some(num), Some(unit)) = (it.next(), it.next()) else {
            continue;
        };
        let Ok(v) = num.parse::<f64>() else { continue };
        let mult = match unit.trim_end_matches(')') {
            "B" => 1.0,
            "KiB" => 1024.0,
            "MiB" => 1024.0 * 1024.0,
            "GiB" => 1024.0 * 1024.0 * 1024.0,
            "TiB" => 1024f64.powi(4),
            _ => continue,
        };
        let count = l
            .split(": ")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        return ((v * mult) as u64, count);
    }
    (0, 0)
}

/// Package file names a dry run lists with `-v`.
fn paccache_list(args: &[&str]) -> Vec<String> {
    let Ok(out) = crate::host_command(PACCACHE)
        .arg("-v")
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| l.contains(".pkg.tar") && !l.ends_with(".sig"))
        .map(|l| l.rsplit('/').next().unwrap_or(l).to_string())
        .collect()
}

fn package_names(files: &[String], max: usize) -> String {
    let mut names: Vec<&str> = files
        .iter()
        .map(|f| f.split(".pkg.tar").next().unwrap_or(f))
        .collect();
    names.sort();
    let more = names.len().saturating_sub(max);
    let mut s = names.into_iter().take(max).collect::<Vec<_>>().join(", ");
    if more > 0 {
        s.push_str(&format!(" … i {more} więcej"));
    }
    s
}

fn paccache_dry(args: &[&str]) -> (u64, u64) {
    let Ok(out) = crate::host_command(PACCACHE)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return (0, 0);
    };
    parse_paccache(&String::from_utf8_lossy(&out.stdout))
}

fn probe_pacman_cache_all() -> Option<Probe> {
    if !Path::new(PACCACHE).exists() {
        return None;
    }
    let (bytes, n) = paccache_dry(&["-d", "-k0"]);
    (bytes > 0).then(|| Probe {
        task: RootTask::PacmanCacheAll,
        bytes,
        files: n,
        detail: format!(
            "{n} paczek: {}",
            package_names(&paccache_list(&["-d", "-k0"]), 8)
        ),
    })
}

fn probe_pacman_cache() -> Option<Probe> {
    if !Path::new(PACCACHE).exists() {
        return None;
    }
    let (old, n1) = paccache_dry(&["-d", "-k2"]);
    let (removed, n2) = paccache_dry(&["-d", "-uk0"]);
    let bytes = old + removed;
    (bytes > 0).then(|| Probe {
        task: RootTask::PacmanCache,
        bytes,
        files: n1 + n2,
        detail: {
            let mut files = paccache_list(&["-d", "-k2"]);
            files.extend(paccache_list(&["-d", "-uk0"]));
            format!(
                "{} starszych wersji, {} po odinstalowanych pakietach: {}",
                crate::format_size(old),
                crate::format_size(removed),
                package_names(&files, 8)
            )
        },
    })
}

// ------------------------------------------------------- kernel modules

fn running_release() -> String {
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Version dirs under /usr/lib/modules that some installed package owns,
/// read straight from the local pacman DB (one `pacman -Qqo` per dir took
/// ~250 ms each). `None` when the DB cannot be read: then nothing is stale.
/// Which package database decides who owns a module dir.
enum ModuleOwners {
    /// pacman (`/var/lib/pacman/local/*/files`) or dpkg (`info/*.list`):
    /// the set of owned versions, read from files.
    Set(std::collections::HashSet<String>),
    /// rpm: asked per directory (`rpm -qf`, rpmdb is world-readable).
    Rpm,
}

/// `usr/lib/modules/<v>/…` (pacman) or `/lib/modules/<v>/…`,
/// `/usr/lib/modules/<v>/…` (dpkg: usrmerge aliases are not unified).
fn module_version_of(line: &str) -> Option<&str> {
    let l = line.trim_start_matches('/');
    let rest = l
        .strip_prefix("usr/lib/modules/")
        .or_else(|| l.strip_prefix("lib/modules/"))?;
    rest.split('/').next().filter(|v| !v.is_empty())
}

fn owned_from_files(
    dir: &str,
    file_of: impl Fn(&fs::DirEntry) -> Option<PathBuf>,
) -> Option<std::collections::HashSet<String>> {
    let rd = fs::read_dir(dir).ok()?;
    let mut owned = std::collections::HashSet::new();
    let mut seen_any = false;
    for e in rd.flatten() {
        let Some(f) = file_of(&e) else { continue };
        let Ok(text) = fs::read_to_string(f) else {
            continue;
        };
        seen_any = true;
        for l in text.lines() {
            if let Some(v) = module_version_of(l) {
                owned.insert(v.to_string());
            }
        }
    }
    seen_any.then_some(owned)
}

fn module_owners() -> Option<ModuleOwners> {
    if let Some(s) = owned_from_files("/var/lib/pacman/local", |e| Some(e.path().join("files"))) {
        return Some(ModuleOwners::Set(s));
    }
    if let Some(s) = owned_from_files("/var/lib/dpkg/info", |e| {
        e.file_name()
            .to_string_lossy()
            .ends_with(".list")
            .then(|| e.path())
    }) {
        return Some(ModuleOwners::Set(s));
    }
    Path::new("/usr/bin/rpm")
        .exists()
        .then_some(ModuleOwners::Rpm)
}

/// `rpm -qf` exits 0 when some package owns the path. Unknown = owned.
fn rpm_owns(path: &Path) -> bool {
    crate::host_command("/usr/bin/rpm")
        .arg("-qf")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(true)
}

/// A read-only or image-based root where the package manager does not own
/// the system files (SteamOS A/B, ostree, NixOS, transactional-update):
/// no task may touch /usr or the package cache there.
pub fn immutable_root() -> Option<&'static str> {
    if Path::new("/run/ostree-booted").exists() {
        return Some("ostree");
    }
    if Path::new("/etc/NIXOS").exists() {
        return Some("NixOS");
    }
    if Path::new("/usr/sbin/transactional-update").exists() {
        return Some("transactional-update");
    }
    let os = fs::read_to_string("/etc/os-release").unwrap_or_default();
    if os
        .lines()
        .any(|l| l.trim() == "ID=steamos" || l.trim() == "ID=\"steamos\"")
    {
        return Some("SteamOS");
    }
    None
}

/// Module dirs of kernels that are gone: real directory, plain name, not
/// the running kernel, owned by no package.
pub fn stale_module_dirs() -> Vec<String> {
    if immutable_root().is_some() {
        return Vec::new();
    }
    let running = running_release();
    let Some(owners) = module_owners() else {
        return Vec::new();
    };
    let Ok(rd) = fs::read_dir(MODULES_DIR) else {
        return Vec::new();
    };
    let mut out: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let m = fs::symlink_metadata(e.path()).ok()?;
            let plain = !name.is_empty() && !name.starts_with('.') && !name.contains('/');
            let owned = match &owners {
                ModuleOwners::Set(set) => set.contains(&name),
                ModuleOwners::Rpm => rpm_owns(&e.path()),
            };
            (plain && m.is_dir() && name != running && !owned).then_some(name)
        })
        .collect();
    out.sort();
    out
}

fn probe_modules() -> Option<Probe> {
    let dirs = stale_module_dirs();
    if dirs.is_empty() {
        return None;
    }
    let base = Home::open(Path::new(MODULES_DIR)).ok()?;
    let mut total = Usage::default();
    for d in &dirs {
        if let Ok(Some(u)) = base.measure(Path::new(d)) {
            total.add(u);
        }
    }
    Some(Probe {
        task: RootTask::KernelModules,
        bytes: total.bytes,
        files: total.files,
        detail: dirs.join(", "),
    })
}

// ---------------------------------------------- pacman download dirs

/// `download-` + 6 alphanumerics (pacman's mkdtemp template).
fn is_download_dir_name(name: &str) -> bool {
    name.strip_prefix("download-")
        .is_some_and(|r| r.len() == 6 && r.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// Stale download dirs: right name, real directory, untouched for an hour,
/// and pacman not running. Emptiness is enforced by rmdir itself.
fn stale_download_dirs() -> Vec<String> {
    if crate::locks::busy_package_manager().is_some() {
        return Vec::new();
    }
    let hour_ago = std::time::SystemTime::now() - Duration::from_secs(3600);
    let Ok(rd) = fs::read_dir(PKG_DIR) else {
        return Vec::new();
    };
    let mut v: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let m = fs::symlink_metadata(e.path()).ok()?;
            let old = m.modified().ok().is_some_and(|t| t < hour_ago);
            (is_download_dir_name(&name) && m.is_dir() && old).then_some(name)
        })
        .collect();
    v.sort();
    v
}

fn probe_pacman_downloads() -> Option<Probe> {
    let dirs = stale_download_dirs();
    (!dirs.is_empty()).then(|| Probe {
        task: RootTask::PacmanDownloads,
        bytes: 0,
        files: dirs.len() as u64,
        detail: format!("{} katalogów", dirs.len()),
    })
}

fn root_pacman_downloads() -> (u64, Vec<String>) {
    say("• Puste katalogi po przerwanych pobraniach pacmana…");
    let dirs = stale_download_dirs();
    let mut removed = 0;
    let mut freed = 0;
    let mut errs = Vec::new();
    for d in &dirs {
        let Ok(c) = std::ffi::CString::new(format!("{PKG_DIR}/{d}")) else {
            continue;
        };
        // rmdir(2) removes only an empty directory and never follows a symlink.
        if unsafe { libc::rmdir(c.as_ptr()) } == 0 {
            removed += 1;
            continue;
        }
        let e = io::Error::last_os_error();
        if e.raw_os_error() != Some(libc::ENOTEMPTY) {
            errs.push(format!("{d}: {e}"));
            continue;
        }
        // Not empty: abandoned partial downloads. pacman -Sc itself fails on
        // such a dir ("Error reading fd"). Only a day-old dir that holds
        // nothing but regular files is emptied.
        match empty_abandoned_download(&Path::new(PKG_DIR).join(d)) {
            Ok(Some(b)) => {
                freed += b;
                if unsafe { libc::rmdir(c.as_ptr()) } == 0 {
                    removed += 1;
                } else {
                    errs.push(format!("{d}: {}", io::Error::last_os_error()));
                }
            }
            Ok(None) => say(&format!("  {d}: pominięto (świeży albo nie tylko pliki)")),
            Err(e) => errs.push(format!("{d}: {e}")),
        }
    }
    say(&format!(
        "  usunięto {removed} z {} katalogów, zwolniono {}",
        dirs.len(),
        crate::format_size(freed)
    ));
    (freed, errs)
}

/// Delete the files of an abandoned download dir. `Ok(None)` (nothing
/// touched) unless it is older than a day and holds only regular files.
fn empty_abandoned_download(dir: &Path) -> io::Result<Option<u64>> {
    let m = fs::symlink_metadata(dir)?;
    let day_ago = std::time::SystemTime::now() - Duration::from_secs(86_400);
    if !m.is_dir() || m.modified()? > day_ago {
        return Ok(None);
    }
    let mut files = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        let fm = fs::symlink_metadata(e.path())?;
        if !fm.is_file() {
            return Ok(None);
        }
        files.push((e.path(), fm.blocks() * 512));
    }
    let mut freed = 0;
    for (p, b) in files {
        fs::remove_file(&p)?;
        freed += b;
    }
    Ok(Some(freed))
}

// ------------------------------------------------- unused sync databases

/// `extra.db`, `extra.files.sig` → `extra`.
fn sync_db_repo(file: &str) -> Option<&str> {
    let base = file.strip_suffix(".sig").unwrap_or(file);
    base.strip_suffix(".db")
        .or_else(|| base.strip_suffix(".files"))
}

fn configured_repos() -> Option<std::collections::HashSet<String>> {
    let out = crate::host_command("/usr/bin/pacman-conf")
        .arg("--repo-list")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let set: std::collections::HashSet<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    // An empty list means a broken config: never treat everything as unused.
    (!set.is_empty()).then_some(set)
}

fn unused_sync_dbs() -> Vec<(String, u64)> {
    let Some(repos) = configured_repos() else {
        return Vec::new();
    };
    let Ok(rd) = fs::read_dir(SYNC_DIR) else {
        return Vec::new();
    };
    let mut v: Vec<(String, u64)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let m = fs::symlink_metadata(e.path()).ok()?;
            let repo = sync_db_repo(&name)?;
            (m.is_file() && !repos.contains(repo)).then(|| (name.clone(), m.blocks() * 512))
        })
        .collect();
    v.sort();
    v
}

fn probe_unused_sync_dbs() -> Option<Probe> {
    let v = unused_sync_dbs();
    (!v.is_empty()).then(|| Probe {
        task: RootTask::UnusedSyncDbs,
        bytes: v.iter().map(|(_, b)| b).sum(),
        files: v.len() as u64,
        detail: v
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    })
}

fn root_unused_sync_dbs() -> (u64, Vec<String>) {
    say("• Bazy repozytoriów usuniętych z pacman.conf…");
    if let Some(pm) = crate::locks::busy_package_manager() {
        return (
            0,
            vec![format!("działa menedżer pakietów: {pm} — pominięto")],
        );
    }
    let mut freed = 0;
    let mut errs = Vec::new();
    for (name, b) in unused_sync_dbs() {
        say(&format!("  {SYNC_DIR}/{name}"));
        match fs::remove_file(Path::new(SYNC_DIR).join(&name)) {
            Ok(()) => freed += b,
            Err(e) => errs.push(format!("{name}: {e}")),
        }
    }
    (freed, errs)
}

fn probe_with(task: RootTask, f: fn() -> Option<(u64, u64, String)>) -> Option<Probe> {
    f().map(|(bytes, files, detail)| Probe {
        task,
        bytes,
        files,
        detail,
    })
}

/// Read-only look at everything phase 2 could clean.
pub fn probe_all() -> Vec<Probe> {
    let immutable = immutable_root().is_some();
    [
        probe_coredumps(),
        probe_pacman_cache(),
        probe_pacman_cache_all(),
        probe_modules(),
        probe_pacman_downloads(),
        probe_unused_sync_dbs(),
        probe_with(RootTask::JournalVacuum, crate::sysclean::probe_journal),
        probe_with(RootTask::RotatedLogs, crate::sysclean::probe_rotated_logs),
        probe_with(
            RootTask::FlatpakUnused,
            crate::sysclean::probe_flatpak_unused,
        ),
    ]
    .into_iter()
    .flatten()
    .filter(|p| !(immutable && p.task.touches_system_image()))
    .collect()
}

// ------------------------------------------------------ privileged side

struct Report {
    file: fs::File,
}

impl Report {
    fn line(&mut self, kind: &str, task: RootTask, msg: &str) {
        let _ = writeln!(
            self.file,
            "{kind}\t{}\t{}",
            task.arg(),
            msg.replace(['\n', '\t'], " ")
        );
    }
}

/// Open the GUI's report file safely: no symlink, regular, owned by the
/// user who started the clean.
fn open_report(path: &Path, uid: u32) -> io::Result<fs::File> {
    let f = fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let m = f.metadata()?;
    if !m.is_file() || m.uid() != uid || m.nlink() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "niezaufany plik raportu",
        ));
    }
    f.set_len(0)?;
    Ok(f)
}

pub(crate) fn say(msg: &str) {
    println!("{msg}");
    let _ = io::stdout().flush();
}

/// The user the report belongs to: `--uid` (sudo, doas and su all work),
/// which must agree with what sudo (SUDO_UID) or doas (DOAS_USER) say.
/// Older callers without `--uid` still work under sudo.
fn invoking_uid(arg: Option<u32>) -> Result<u32, String> {
    let sudo = std::env::var("SUDO_UID")
        .ok()
        .and_then(|u| u.parse::<u32>().ok());
    let doas = std::env::var("DOAS_USER")
        .ok()
        .and_then(|n| uid_of_user(&n));
    let uid = arg
        .or(sudo)
        .or(doas)
        .ok_or("brak --uid — uruchom z programu XCleaner")?;
    if uid == 0 {
        return Err("--uid 0: raport musi należeć do zwykłego użytkownika".into());
    }
    for (src, v) in [("SUDO_UID", sudo), ("DOAS_USER", doas)] {
        if v.is_some_and(|v| v != uid) {
            return Err(format!("--uid {uid} nie zgadza się z {src}"));
        }
    }
    Ok(uid)
}

fn uid_of_user(name: &str) -> Option<u32> {
    let c = std::ffi::CString::new(name).ok()?;
    let pw = unsafe { libc::getpwnam(c.as_ptr()) };
    (!pw.is_null()).then(|| unsafe { (*pw).pw_uid })
}

/// Entry point of `--root-clean`. Returns the process exit code.
pub fn run_root(args: &[String]) -> i32 {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("--root-clean wymaga uprawnień roota (sudo, doas albo su)");
        return 2;
    }
    let mut report_path = None;
    let mut uid_arg: Option<u32> = None;
    let mut tasks = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--report" => report_path = it.next().map(PathBuf::from),
            "--uid" => uid_arg = it.next().and_then(|u| u.parse().ok()),
            other => match RootTask::from_arg(other) {
                Some(t) if !tasks.contains(&t) => tasks.push(t),
                Some(_) => {}
                None => {
                    eprintln!("nieznane zadanie: {other}");
                    return 2;
                }
            },
        }
    }
    let Some(report_path) = report_path else {
        eprintln!("brak --report");
        return 2;
    };
    let uid = match invoking_uid(uid_arg) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let mut report = match open_report(&report_path, uid) {
        Ok(f) => Report { file: f },
        Err(e) => {
            eprintln!("raport: {e}");
            return 2;
        }
    };
    say("XCleaner — czyszczenie systemu (root)\n");
    let mut errors = 0;
    let immutable = immutable_root();
    for t in tasks {
        let (freed, errs) = match t {
            t if immutable.is_some() && t.touches_system_image() => (
                0,
                vec![format!(
                    "system {} jest niezmienny — pominięto",
                    immutable.unwrap_or_default()
                )],
            ),
            RootTask::Coredumps => root_coredumps(),
            RootTask::PacmanCache => root_paccache(&[&["-r", "-k2"], &["-r", "-u", "-k0"]]),
            RootTask::PacmanCacheAll => root_paccache(&[&["-r", "-k0"]]),
            RootTask::KernelModules => root_modules(),
            RootTask::PacmanDownloads => root_pacman_downloads(),
            RootTask::UnusedSyncDbs => root_unused_sync_dbs(),
            RootTask::JournalVacuum => crate::sysclean::root_journal(),
            RootTask::RotatedLogs => crate::sysclean::root_rotated_logs(),
            RootTask::FlatpakUnused => crate::sysclean::root_flatpak_unused(),
            RootTask::PkgCache => crate::pkgcache::root_clean(),
            RootTask::AptClean => crate::pkgcache::root_apt_clean(),
            RootTask::CrashReports => crate::sysclean::root_crash_reports(),
        };
        report.line("freed", t, &freed.to_string());
        for e in &errs {
            report.line("error", t, e);
            say(&format!("  błąd: {e}"));
        }
        errors += errs.len();
    }
    let _ = writeln!(report.file, "done");
    if errors > 0 {
        say("\nWystąpiły błędy (szczegóły w oknie XCleaner). Naciśnij Enter, aby zamknąć.");
        let _ = io::stdin().lock().read_line(&mut String::new());
    } else {
        say("\nGotowe.");
        std::thread::sleep(Duration::from_millis(800));
    }
    0
}

fn root_coredumps() -> (u64, Vec<String>) {
    say("• Zrzuty awarii (coredumpy)…");
    let base = match Home::open(Path::new(COREDUMP_DIR)) {
        Ok(b) => b,
        Err(e) => return (0, vec![format!("{COREDUMP_DIR}: {e}")]),
    };
    let mut freed = 0;
    let mut errs = Vec::new();
    for (name, _) in coredump_files() {
        match base.remove_file(Path::new(&name)) {
            Ok(r) => freed += r.freed.bytes,
            Err(e) => errs.push(format!("{name}: {e}")),
        }
    }
    say(&format!("  zwolniono {}", crate::format_size(freed)));
    (freed, errs)
}

fn root_paccache(runs: &[&[&str]]) -> (u64, Vec<String>) {
    let mut freed = 0;
    let mut errs = Vec::new();
    if let Some(pm) = crate::locks::busy_package_manager() {
        return (
            0,
            vec![format!("działa menedżer pakietów: {pm} — pominięto")],
        );
    }
    for args in runs {
        say(&format!("• Cache pacmana: paccache {}…", args.join(" ")));
        match crate::host_command(PACCACHE)
            .args(*args)
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .output()
        {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                // Real runs print "finished: N packages removed (disk space saved: X)".
                freed += parse_paccache(&text).0;
                if !o.status.success() {
                    errs.push(format!(
                        "paccache {}: {}",
                        args.join(" "),
                        String::from_utf8_lossy(&o.stderr).trim()
                    ));
                }
            }
            Err(e) => errs.push(format!("paccache: {e}")),
        }
    }
    say(&format!("  zwolniono {}", crate::format_size(freed)));
    (freed, errs)
}

fn root_modules() -> (u64, Vec<String>) {
    say("• Moduły po usuniętych kernelach…");
    let base = match Home::open(Path::new(MODULES_DIR)) {
        Ok(b) => b,
        Err(e) => return (0, vec![format!("{MODULES_DIR}: {e}")]),
    };
    let mut freed = 0;
    let mut errs = Vec::new();
    // Recomputed as root: nothing from the GUI decides what goes.
    for d in stale_module_dirs() {
        say(&format!("  {MODULES_DIR}/{d}"));
        match base.remove_dir(Path::new(&d)) {
            Ok(r) => {
                freed += r.freed.bytes;
                errs.extend(r.errors.into_iter().map(|e| format!("{d}: {e}")));
            }
            Err(e) => errs.push(format!("{d}: {e}")),
        }
    }
    say(&format!("  zwolniono {}", crate::format_size(freed)));
    (freed, errs)
}

// ------------------------------------------------------- GUI-side launch

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// The program root should run: the AppImage file when packaged (root
/// cannot enter the user's FUSE mount), otherwise this executable.
fn self_path() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && is_executable(p))
        .or_else(|| std::env::current_exe().ok())
}

#[derive(Debug, Clone, Default)]
pub struct RootOutcome {
    pub freed: u64,
    /// Freed bytes per task (task arg, bytes), in report order.
    pub per_task: Vec<(String, u64)>,
    pub errors: Vec<String>,
    /// The privileged step did not finish (password cancelled, no terminal…).
    pub not_done: Option<String>,
}

pub fn parse_report(text: &str) -> RootOutcome {
    let mut o = RootOutcome::default();
    let mut done = false;
    for l in text.lines() {
        let mut p = l.splitn(3, '\t');
        match (p.next(), p.next(), p.next()) {
            (Some("freed"), Some(t), Some(n)) => {
                let b = n.parse::<u64>().unwrap_or(0);
                o.freed += b;
                o.per_task.push((t.to_string(), b));
            }
            (Some("error"), Some(t), Some(m)) => o.errors.push(format!("{t}: {m}")),
            (Some("done"), _, _) => done = true,
            _ => {}
        }
    }
    if !done {
        o.not_done = Some(
            "czyszczenie systemu nie zostało wykonane (anulowano hasło lub zamknięto terminal)"
                .into(),
        );
    }
    o
}

/// Run the tasks as root (sudo, doas or su) in a terminal and wait for the result.
///
/// The terminal runs our wrapper (see `term.rs`), which records its PID and
/// the exit code of sudo in a status file. The end is: `done` in the report,
/// an exit code, or the wrapper PID gone (window closed).
pub fn launch(tasks: &[RootTask]) -> RootOutcome {
    let fail = |m: &str| RootOutcome {
        not_done: Some(m.into()),
        ..Default::default()
    };
    let Some(elevate) = crate::term::find_elevate() else {
        return fail("brak sudo, doas i su");
    };
    let Some(terminal) = crate::term::find_terminal() else {
        return fail("nie znaleziono emulatora terminala (np. kitty, foot, konsole)");
    };
    let Some(me) = self_path() else {
        return fail("nie można ustalić ścieżki programu");
    };
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(std::env::temp_dir);
    let report = runtime.join(format!("xcleaner-root-{}.txt", std::process::id()));
    let status = runtime.join(format!("xcleaner-root-{}.wrap", std::process::id()));
    for f in [&report, &status] {
        if let Err(e) = crate::term::create_private(f) {
            return fail(&format!("plik raportu: {e}"));
        }
    }
    let cleanup = || {
        let _ = fs::remove_file(&report);
        let _ = fs::remove_file(&status);
    };
    let uid = unsafe { libc::getuid() };
    let mut root_cmd: Vec<OsString> = vec![
        "env".into(),
        "APPIMAGELAUNCHER_DISABLE=1".into(),
        me.clone().into(),
        ROOT_ARG.into(),
        "--uid".into(),
        uid.to_string().into(),
        "--report".into(),
        report.clone().into(),
    ];
    root_cmd.extend(tasks.iter().map(|t| OsString::from(t.arg())));
    let mut cmd: Vec<OsString> = vec![
        me.into(),
        crate::term::WRAP_ARG.into(),
        status.clone().into(),
        "--".into(),
    ];
    cmd.extend(elevate.wrap(&root_cmd));
    let title = format!(
        "XCleaner — {} ({})",
        elevate.password_hint(),
        elevate.label()
    );
    let args = crate::term::terminal_args(&terminal, &title, &cmd);
    let mut child = match crate::host_command(&terminal)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            cleanup();
            return fail(&format!("terminal {}: {e}", terminal.display()));
        }
    };
    let start = Instant::now();
    let mut pid: Option<u32> = None;
    let mut gone = None;
    loop {
        std::thread::sleep(Duration::from_millis(300));
        let rep = fs::read_to_string(&report).unwrap_or_default();
        if rep.lines().any(|l| l == "done") {
            break;
        }
        let st = crate::term::parse_status(&fs::read_to_string(&status).unwrap_or_default());
        pid = pid.or(st.pid);
        if let Some(code) = st.exit {
            if code != 0 {
                gone = Some(format!(
                    "czyszczenie systemu nie zostało wykonane ({} zakończyło się kodem {code}: złe hasło lub anulowano)",
                    elevate.label()
                ));
            }
            break;
        }
        match pid {
            Some(p) if !crate::procs::Processes::pid_alive(p) => {
                gone = Some("czyszczenie systemu przerwane: zamknięto okno terminala".into());
                break;
            }
            Some(_) => {}
            None if start.elapsed() > Duration::from_secs(30) => {
                let _ = child.try_wait();
                gone = Some(format!(
                    "terminal {} nie uruchomił polecenia",
                    terminal.display()
                ));
                break;
            }
            None => {}
        }
    }
    // The terminal may still be open (or be a server client): reap it later.
    if matches!(child.try_wait(), Ok(None)) {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
    let text = fs::read_to_string(&report).unwrap_or_default();
    cleanup();
    let mut o = parse_report(&text);
    if o.not_done.is_some() {
        if let Some(g) = gone {
            o.not_done = Some(g);
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paccache_output() {
        let dry = "==> finished dry run: 12 candidates (disk space saved: 300.50 MiB)\n";
        let (b, n) = parse_paccache(dry);
        assert_eq!(n, 12);
        assert_eq!(b, (300.5 * 1024.0 * 1024.0) as u64);
        assert_eq!(
            parse_paccache("==> no candidate packages found for pruning\n"),
            (0, 0)
        );
        let real = "==> finished: 3 packages removed (disk space saved: 1.25 GiB)\n";
        assert_eq!(
            parse_paccache(real).0,
            (1.25 * 1024.0 * 1024.0 * 1024.0) as u64
        );
    }

    #[test]
    fn coredump_names() {
        assert_eq!(
            coredump_comm("core.cs2.1000.abc.153806.1795.zst").as_deref(),
            Some("cs2")
        );
        assert_eq!(
            coredump_comm("core.google\\x2dchrome.1000.a.1.2.zst").as_deref(),
            Some("google-chrome")
        );
    }

    #[test]
    fn report_parsing() {
        let o = parse_report("freed\tcoredumps\t100\nerror\tkernel-modules\tx: busy\ndone\n");
        assert_eq!(o.freed, 100);
        assert_eq!(o.per_task, vec![("coredumps".to_string(), 100)]);
        assert_eq!(o.errors.len(), 1);
        assert!(o.not_done.is_none());
        assert!(parse_report("freed\tcoredumps\t5\n").not_done.is_some());
    }

    #[test]
    fn invoking_uid_rules() {
        // Tests do not run under sudo/doas: only --uid is seen.
        if std::env::var_os("SUDO_UID").is_none() && std::env::var_os("DOAS_USER").is_none() {
            assert_eq!(invoking_uid(Some(1000)), Ok(1000));
            assert!(invoking_uid(Some(0)).is_err());
            assert!(invoking_uid(None).is_err());
        }
    }

    #[test]
    fn module_paths_of_every_package_db() {
        assert_eq!(
            module_version_of("usr/lib/modules/6.18.6-2/kernel/x.ko"),
            Some("6.18.6-2")
        );
        assert_eq!(
            module_version_of("/lib/modules/6.1.0-27-amd64/modules.dep"),
            Some("6.1.0-27-amd64")
        );
        assert_eq!(module_version_of("/usr/lib/modules/6.12.0"), Some("6.12.0"));
        assert_eq!(module_version_of("/usr/lib/modules/"), None);
        assert_eq!(module_version_of("/usr/lib/firmware/x"), None);
    }

    #[test]
    fn task_args_roundtrip() {
        for t in RootTask::ALL {
            assert_eq!(RootTask::from_arg(t.arg()), Some(t));
        }
        assert_eq!(RootTask::from_arg("/etc"), None);
    }

    #[test]
    fn sync_db_names_and_package_list() {
        assert_eq!(sync_db_repo("extra.db"), Some("extra"));
        assert_eq!(sync_db_repo("cachyos-v3.files.sig"), Some("cachyos-v3"));
        assert_eq!(sync_db_repo("README"), None);
        let files = vec![
            "b-1-1-x86_64.pkg.tar.zst".to_string(),
            "a-2-1-any.pkg.tar.zst".to_string(),
        ];
        assert_eq!(package_names(&files, 1), "a-2-1-any … i 1 więcej");
    }

    #[test]
    fn abandoned_download_needs_old_dir_with_files_only() {
        let d = std::env::temp_dir().join(format!("xc-dl-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("x.pkg.tar.zst.part"), b"x").unwrap();
        // Fresh dir: untouched.
        assert_eq!(empty_abandoned_download(&d).unwrap(), None);
        assert!(d.join("x.pkg.tar.zst.part").exists());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn download_dir_names() {
        assert!(is_download_dir_name("download-0d9PlD"));
        assert!(!is_download_dir_name("download-"));
        assert!(!is_download_dir_name("download-abc/de"));
        assert!(!is_download_dir_name("linux-7.3.5.pkg.tar.zst"));
    }

    #[test]
    fn running_kernel_is_never_stale() {
        let running = running_release();
        assert!(!stale_module_dirs().contains(&running));
    }
}
