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
        Self::from_proc(Path::new("/proc"))
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

    pub fn pid_alive(pid: u32) -> bool {
        Path::new("/proc").join(pid.to_string()).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
