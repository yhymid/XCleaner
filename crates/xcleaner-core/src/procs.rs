//! Running-process detection from /proc (no external tools).

use std::{collections::HashSet, fs, path::Path};

/// Lower-cased names of running processes: `comm` (kernel-truncated to 15
/// bytes) plus the basename of `exe` and of `argv[0]` when readable.
#[derive(Debug, Default, Clone)]
pub struct Processes {
    names: HashSet<String>,
}

impl Processes {
    pub fn snapshot() -> Self {
        let mut p = Self::from_proc(Path::new("/proc"));
        let uid = unsafe { libc::geteuid() };
        p.add_flatpak_instances(
            &Path::new("/run/user")
                .join(uid.to_string())
                .join(".flatpak"),
        );
        p
    }

    /// `flatpak:<app id>` for every running Flatpak instance: each one has
    /// `<instance>/info` with `[Application] name=<id>` (what `flatpak ps` reads).
    fn add_flatpak_instances(&mut self, dir: &Path) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let Ok(info) = fs::read_to_string(e.path().join("info")) else {
                continue;
            };
            if let Some(id) = flatpak_app_name(&info) {
                self.names.insert(format!("flatpak:{}", id.to_lowercase()));
            }
        }
    }

    pub fn from_proc(proc_root: &Path) -> Self {
        let mut names = HashSet::new();
        let Ok(rd) = fs::read_dir(proc_root) else {
            return Self { names };
        };
        let me = std::process::id().to_string();
        for e in rd.flatten() {
            let pid = e.file_name();
            let pid = pid.to_string_lossy();
            if !pid.bytes().all(|b| b.is_ascii_digit()) || pid == me {
                continue;
            }
            let dir = e.path();
            if let Ok(comm) = fs::read_to_string(dir.join("comm")) {
                names.insert(comm.trim().to_lowercase());
            }
            if let Ok(exe) = fs::read_link(dir.join("exe")) {
                if let Some(n) = exe.file_name() {
                    names.insert(
                        n.to_string_lossy()
                            .trim_end_matches(" (deleted)")
                            .to_lowercase(),
                    );
                }
            }
            // Snap apps run in a `snap.<name>.<app>` cgroup scope.
            if let Ok(cg) = fs::read_to_string(dir.join("cgroup")) {
                if let Some(name) = snap_name_from_cgroup(&cg) {
                    names.insert(format!("snap:{}", name.to_lowercase()));
                }
            }
            if let Ok(cmd) = fs::read(dir.join("cmdline")) {
                if let Some(arg0) = cmd.split(|b| *b == 0).next() {
                    if let Some(base) = arg0.rsplit(|b| *b == b'/').next() {
                        if !base.is_empty() {
                            names.insert(String::from_utf8_lossy(base).to_lowercase());
                        }
                    }
                }
            }
        }
        Self { names }
    }

    #[cfg(test)]
    pub fn with(names: &[&str]) -> Self {
        Self {
            names: names.iter().map(|s| s.to_lowercase()).collect(),
        }
    }

    /// First guard name that is running. `comm` is truncated to 15 bytes, so a
    /// long guard also matches its 15-byte prefix.
    pub fn first_running<'a>(&self, guards: &[&'a str]) -> Option<&'a str> {
        guards.iter().copied().find(|g| {
            let g_l = g.to_lowercase();
            self.names.contains(&g_l) || (g_l.len() > 15 && self.names.contains(&g_l[..15]))
        })
    }

    #[cfg(test)]
    pub fn add_name(&mut self, n: &str) {
        self.names.insert(n.to_lowercase());
    }

    pub fn pid_alive(pid: u32) -> bool {
        Path::new("/proc").join(pid.to_string()).exists()
    }
}

/// `name=` of the `[Application]` group of a Flatpak instance info file.
fn flatpak_app_name(info: &str) -> Option<&str> {
    let mut in_app = false;
    for l in info.lines() {
        let l = l.trim();
        if l.starts_with('[') {
            in_app = l == "[Application]";
        } else if in_app {
            if let Some(v) = l.strip_prefix("name=") {
                return Some(v.trim()).filter(|v| !v.is_empty());
            }
        }
    }
    None
}

/// `firefox` from a cgroup line like
/// `0::/user.slice/.../snap.firefox.firefox-1234.scope`.
fn snap_name_from_cgroup(cg: &str) -> Option<&str> {
    cg.lines()
        .flat_map(|l| l.rsplit('/').next())
        .find_map(|leaf| leaf.strip_prefix("snap."))
        .and_then(|r| r.split('.').next())
        .filter(|n| !n.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatpak_and_snap_names() {
        let info = "[Application]\nname=org.mozilla.firefox\nruntime=runtime/x\n\n[Instance]\ninstance-id=1\n";
        assert_eq!(flatpak_app_name(info), Some("org.mozilla.firefox"));
        assert_eq!(
            flatpak_app_name("[Runtime]\nname=org.gnome.Platform\n"),
            None
        );
        assert_eq!(
            snap_name_from_cgroup("0::/user.slice/user-1000.slice/user@1000.service/app.slice/snap.firefox.firefox-1a2b.scope\n"),
            Some("firefox")
        );
        assert_eq!(
            snap_name_from_cgroup("0::/user.slice/session-2.scope\n"),
            None
        );
    }

    #[test]
    fn matches_truncated_comm() {
        let p = Processes::with(&["steamwebhelper", "discordcanary"]);
        assert_eq!(
            p.first_running(&["firefox", "DiscordCanary"]),
            Some("DiscordCanary")
        );
        assert_eq!(p.first_running(&["firefox"]), None);
        let long = Processes::with(&["google-chrome-s"]);
        assert_eq!(
            long.first_running(&["google-chrome-stable"]),
            Some("google-chrome-stable")
        );
    }

    #[test]
    fn snapshot_sees_something() {
        // At least the test runner's parent shell/cargo should be visible.
        assert!(!Processes::snapshot().names.is_empty());
    }
}
