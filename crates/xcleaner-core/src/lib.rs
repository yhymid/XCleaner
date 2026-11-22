//! XCleaner core: rules, scanning and safe deletion. No GTK here.
#![cfg(target_os = "linux")]

pub mod artifacts;
pub mod extra;
pub mod fsops;
pub mod history;
pub mod keep;
pub mod leftovers;
pub mod locks;
pub mod procs;
pub mod protect;
pub mod root;
pub mod rules;
pub mod scan;
pub mod term;
pub mod usage;

/// True when running with effective UID 0.
/// Variables the AppImage runtime and its GTK hook set for the bundled
/// libraries. Host programs (pacman, a terminal, a file manager) must not
/// inherit them: they would load our libraries or plugins instead of theirs.
pub const FOREIGN_ENV: &[&str] = &[
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "LD_AUDIT",
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "GTK_DATA_PREFIX",
    "GTK_IM_MODULE_FILE",
    "GIO_MODULE_DIR",
    "GIO_EXTRA_MODULES",
    "GI_TYPELIB_PATH",
    "GDK_PIXBUF_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    "GSETTINGS_SCHEMA_DIR",
    "PYTHONPATH",
    "PYTHONHOME",
    "PERLLIB",
    "QT_PLUGIN_PATH",
    "APPDIR",
    "APPIMAGE",
    "ARGV0",
    "OWD",
    "FUSERMOUNT_PROG",
];

/// `XDG_DATA_DIRS` without the entries inside the AppImage mount, or None
/// when nothing needs to change (not an AppImage, or nothing of ours in it).
/// `Some("")` means the variable should be unset.
pub fn host_xdg_data_dirs() -> Option<String> {
    let appdir = std::env::var("APPDIR").ok().filter(|a| !a.is_empty())?;
    let dirs = std::env::var("XDG_DATA_DIRS").ok()?;
    Some(strip_appdir(&dirs, &appdir)).filter(|d| *d != dirs)
}

fn strip_appdir(dirs: &str, appdir: &str) -> String {
    let appdir = appdir.trim_end_matches('/');
    dirs.split(':')
        .filter(|d| !d.is_empty() && *d != appdir && !d.starts_with(&format!("{appdir}/")))
        .collect::<Vec<_>>()
        .join(":")
}

/// Strip the AppImage environment from a command that runs a host program.
pub fn sanitize_env(cmd: &mut std::process::Command) {
    for v in FOREIGN_ENV {
        cmd.env_remove(v);
    }
    match host_xdg_data_dirs() {
        Some(d) if d.is_empty() => {
            cmd.env_remove("XDG_DATA_DIRS");
        }
        Some(d) => {
            cmd.env("XDG_DATA_DIRS", d);
        }
        None => {}
    }
}

/// `Command::new` for a host program, with [`sanitize_env`] applied.
pub fn host_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut c = std::process::Command::new(program);
    sanitize_env(&mut c);
    c
}

pub fn is_root() -> bool {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() == 0 }
}

/// CLI output piped into `head` etc.: exit quietly on a closed pipe like
/// coreutils, instead of a panic (and, with panic=abort, a core dump).
pub fn default_sigpipe() {
    // SAFETY: resetting a signal disposition has no preconditions.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

/// Human-readable size with binary units (e.g. "1,5 GiB" in Polish style).
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{:.1} {}", v, UNITS[u]).replace('.', ",")
    }
}

/// Polish plural: pl(n, "plik", "pliki", "plików").
pub fn pl(n: u64, one: &str, few: &str, many: &str) -> String {
    let w = if n == 1 {
        one
    } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
        few
    } else {
        many
    };
    format!("{n} {w}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_command_drops_appimage_env() {
        let c = super::host_command("true");
        let removed: Vec<_> = c
            .get_envs()
            .filter(|(_, v)| v.is_none())
            .map(|(k, _)| k.to_string_lossy().to_string())
            .collect();
        for v in [
            "LD_LIBRARY_PATH",
            "GTK_PATH",
            "GI_TYPELIB_PATH",
            "APPIMAGE",
            "APPDIR",
        ] {
            assert!(removed.iter().any(|r| r == v), "{v}");
        }
    }

    #[test]
    fn appdir_entries_leave_xdg_data_dirs() {
        let a = "/tmp/.mount_XCleanAB";
        assert_eq!(
            super::strip_appdir(
                "/tmp/.mount_XCleanAB/usr/share:/usr/local/share:/usr/share",
                a
            ),
            "/usr/local/share:/usr/share"
        );
        assert_eq!(super::strip_appdir("/tmp/.mount_XCleanAB/usr/share", a), "");
        // A sibling mount with a common prefix is not ours.
        assert_eq!(
            super::strip_appdir("/tmp/.mount_XCleanABC/usr/share", a),
            "/tmp/.mount_XCleanABC/usr/share"
        );
    }

    use super::*;

    #[test]
    fn sizes_and_plurals() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1,5 KiB");
        assert_eq!(pl(1, "plik", "pliki", "plików"), "1 plik");
        assert_eq!(pl(3, "plik", "pliki", "plików"), "3 pliki");
        assert_eq!(pl(12, "plik", "pliki", "plików"), "12 plików");
        assert_eq!(pl(22, "plik", "pliki", "plików"), "22 pliki");
    }
}
