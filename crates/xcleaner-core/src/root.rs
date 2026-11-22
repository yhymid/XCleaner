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
//! SUDO_UID, no symlink).

use crate::fsops::{Home, Usage};
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const ROOT_ARG: &str = "--root-clean";
const COREDUMP_DIR: &str = "/var/lib/systemd/coredump";
const MODULES_DIR: &str = "/usr/lib/modules";
const PACCACHE: &str = "/usr/bin/paccache";
const PKG_DIR: &str = "/var/cache/pacman/pkg";
const PACMAN_LOCK: &str = "/var/lib/pacman/db.lck";
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
}

impl RootTask {
    pub const ALL: [RootTask; 6] = [
        RootTask::Coredumps,
        RootTask::PacmanCache,
        RootTask::PacmanCacheAll,
        RootTask::KernelModules,
        RootTask::PacmanDownloads,
        RootTask::UnusedSyncDbs,
    ];

    /// Where the task works (for display).
    pub fn location(self) -> &'static str {
        match self {
            RootTask::Coredumps => COREDUMP_DIR,
            RootTask::PacmanCache | RootTask::PacmanCacheAll | RootTask::PacmanDownloads => PKG_DIR,
            RootTask::KernelModules => MODULES_DIR,
            RootTask::UnusedSyncDbs => SYNC_DIR,
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
        }
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
    let Ok(out) = Command::new(PACCACHE)
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
    let Ok(out) = Command::new(PACCACHE)
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
fn owned_module_versions() -> Option<std::collections::HashSet<String>> {
    let mut owned = std::collections::HashSet::new();
    let rd = fs::read_dir("/var/lib/pacman/local").ok()?;
    let mut seen_any = false;
    for e in rd.flatten() {
        let Ok(files) = fs::read_to_string(e.path().join("files")) else {
            continue;
        };
        seen_any = true;
        for l in files.lines() {
            if let Some(rest) = l.strip_prefix("usr/lib/modules/") {
                if let Some(v) = rest.split('/').next().filter(|v| !v.is_empty()) {
                    owned.insert(v.to_string());
                }
            }
        }
    }
    seen_any.then_some(owned)
}

/// Module dirs of kernels that are gone: real directory, plain name, not
/// the running kernel, owned by no package.
pub fn stale_module_dirs() -> Vec<String> {
    let running = running_release();
    let Some(owned) = owned_module_versions() else {
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
            (plain && m.is_dir() && name != running && !owned.contains(&name)).then_some(name)
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
    if Path::new(PACMAN_LOCK).exists() {
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
    let out = Command::new("/usr/bin/pacman-conf")
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
    if Path::new(PACMAN_LOCK).exists() {
        return (0, vec!["pacman działa (db.lck) — pominięto".into()]);
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

/// Read-only look at everything phase 2 could clean.
pub fn probe_all() -> Vec<Probe> {
    [
        probe_coredumps(),
        probe_pacman_cache(),
        probe_pacman_cache_all(),
        probe_modules(),
        probe_pacman_downloads(),
        probe_unused_sync_dbs(),
    ]
    .into_iter()
    .flatten()
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

/// Open the GUI's report file safely: no symlink, regular, owned by SUDO_UID.
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

fn say(msg: &str) {
    println!("{msg}");
    let _ = io::stdout().flush();
}

/// Entry point of `--root-clean`. Returns the process exit code.
pub fn run_root(args: &[String]) -> i32 {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("--root-clean wymaga uruchomienia przez sudo");
        return 2;
    }
    let Some(uid) = std::env::var("SUDO_UID")
        .ok()
        .and_then(|u| u.parse::<u32>().ok())
    else {
        eprintln!("brak SUDO_UID — uruchom przez sudo z programu XCleaner");
        return 2;
    };
    let mut report_path = None;
    let mut tasks = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--report" => report_path = it.next().map(PathBuf::from),
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
    let mut report = match open_report(&report_path, uid) {
        Ok(f) => Report { file: f },
        Err(e) => {
            eprintln!("raport: {e}");
            return 2;
        }
    };
    say("XCleaner — czyszczenie systemu (root)\n");
    let mut errors = 0;
    for t in tasks {
        let (freed, errs) = match t {
            RootTask::Coredumps => root_coredumps(),
            RootTask::PacmanCache => root_paccache(&[&["-r", "-k2"], &["-r", "-u", "-k0"]]),
            RootTask::PacmanCacheAll => root_paccache(&[&["-r", "-k0"]]),
            RootTask::KernelModules => root_modules(),
            RootTask::PacmanDownloads => root_pacman_downloads(),
            RootTask::UnusedSyncDbs => root_unused_sync_dbs(),
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
    if Path::new(PACMAN_LOCK).exists() {
        return (0, vec!["pacman działa (db.lck) — pominięto".into()]);
    }
    for args in runs {
        say(&format!("• Cache pacmana: paccache {}…", args.join(" ")));
        match Command::new(PACCACHE)
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

/// Environment AppImage runtimes inject that host programs must not inherit.
const FOREIGN_ENV: &[&str] = &[
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "LD_AUDIT",
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "GTK_DATA_PREFIX",
    "GIO_MODULE_DIR",
    "GIO_EXTRA_MODULES",
    "GDK_PIXBUF_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    "GSETTINGS_SCHEMA_DIR",
    "PYTHONPATH",
    "PYTHONHOME",
    "PERLLIB",
    "QT_PLUGIN_PATH",
];

pub const TERMINALS: &[&str] = &[
    "kitty",
    "foot",
    "alacritty",
    "wezterm",
    "konsole",
    "gnome-terminal",
    "kgx",
    "xfce4-terminal",
    "qterminal",
    "terminator",
    "xdg-terminal-exec",
    "xterm",
];
const TITLE: &str = "XCleaner — hasło sudo";

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

pub fn find_terminal() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let preferred = std::env::var("TERMINAL")
        .ok()
        .and_then(|t| t.split_whitespace().next().map(str::to_owned));
    preferred
        .iter()
        .map(String::as_str)
        .chain(TERMINALS.iter().copied())
        .find_map(|name| {
            let c = Path::new(name);
            if c.is_absolute() {
                return is_executable(c).then(|| c.to_path_buf());
            }
            std::env::split_paths(&path)
                .map(|d| d.join(name))
                .find(|p| is_executable(p))
        })
}

pub fn terminal_args(terminal: &Path, program: &str, args: &[OsString]) -> Vec<OsString> {
    let name = terminal
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut out: Vec<OsString> = match name.as_str() {
        "kitty" | "foot" => vec!["--title".into(), TITLE.into()],
        "alacritty" => vec!["--title".into(), TITLE.into(), "-e".into()],
        "wezterm" => vec!["start".into(), "--".into()],
        "gnome-terminal" | "kgx" => vec!["--title".into(), TITLE.into(), "--".into()],
        "xfce4-terminal" | "terminator" => vec!["-T".into(), TITLE.into(), "-x".into()],
        "xdg-terminal-exec" => vec![],
        "xterm" => vec!["-T".into(), TITLE.into(), "-e".into()],
        _ => vec!["-e".into()],
    };
    out.push(program.into());
    out.extend(args.iter().cloned());
    out
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

/// Run the tasks as root via `sudo` in a terminal and wait for the result.
pub fn launch(tasks: &[RootTask]) -> RootOutcome {
    let fail = |m: &str| RootOutcome {
        not_done: Some(m.into()),
        ..Default::default()
    };
    let Some(terminal) = find_terminal() else {
        return fail("nie znaleziono emulatora terminala (np. kitty, foot, konsole)");
    };
    let Some(me) = self_path().and_then(|p| p.to_str().map(str::to_owned)) else {
        return fail("nie można ustalić ścieżki programu");
    };
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(std::env::temp_dir);
    let report = runtime.join(format!("xcleaner-root-{}.txt", std::process::id()));
    let _ = fs::remove_file(&report);
    if let Err(e) = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&report)
    {
        return fail(&format!("plik raportu: {e}"));
    }
    let mut sudo_args: Vec<OsString> = vec![
        "env".into(),
        "APPIMAGELAUNCHER_DISABLE=1".into(),
        me.clone().into(),
        ROOT_ARG.into(),
        "--report".into(),
        report.clone().into(),
    ];
    sudo_args.extend(tasks.iter().map(|t| OsString::from(t.arg())));
    let args = terminal_args(&terminal, "sudo", &sudo_args);
    let mut cmd = Command::new(&terminal);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for v in FOREIGN_ENV {
        cmd.env_remove(v);
    }
    let start = Instant::now();
    match cmd.status() {
        Ok(_) => {}
        Err(e) => {
            let _ = fs::remove_file(&report);
            return fail(&format!("terminal {}: {e}", terminal.display()));
        }
    }
    // Terminals like gnome-terminal return at once: then wait for "done".
    if start.elapsed() < Duration::from_millis(1500) {
        let deadline = Instant::now() + Duration::from_secs(600);
        while Instant::now() < deadline {
            if fs::read_to_string(&report)
                .map(|t| t.lines().any(|l| l == "done"))
                .unwrap_or(false)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    let text = fs::read_to_string(&report).unwrap_or_default();
    let _ = fs::remove_file(&report);
    parse_report(&text)
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
    fn task_args_roundtrip_and_terminal_args() {
        for t in RootTask::ALL {
            assert_eq!(RootTask::from_arg(t.arg()), Some(t));
        }
        assert_eq!(RootTask::from_arg("/etc"), None);
        let a = terminal_args(Path::new("/usr/bin/alacritty"), "sudo", &["x".into()]);
        assert_eq!(a[2], "-e");
        assert_eq!(a[3], "sudo");
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
