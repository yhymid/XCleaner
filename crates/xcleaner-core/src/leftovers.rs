//! Leftovers of uninstalled applications in ~/.config, ~/.local/share,
//! ~/.local/state and ~/.var/app.
//!
//! Design: docs/research/LEFTOVERS-DETECTION-2026-11.md. A directory is a
//! leftover only when three facts agree:
//! 1. its name equals a package whose last pacman.log event is "removed";
//! 2. nothing installed claims the name (packages, provides, binaries, desktop
//!    IDs, PATH, Flatpak, running processes) and it is not a shared name;
//! 3. it was not created after the removal (someone else made it).
//!
//! Birth time 0–30 min after the package's install upgrades it to "pewne".

use crate::{
    rules::{Action, Base, Category, Item, Regen, Risk},
    scan::Layout,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

/// Lowercase alphanumerics only: `Mullvad VPN` ≡ `mullvad-vpn` ≡ `mullvadvpn`.
pub fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Package name without packaging suffixes, squashed.
pub fn pkg_core(name: &str) -> String {
    let mut n = name;
    loop {
        let before = n;
        for suf in [
            "-bin",
            "-git",
            "-appimage",
            "-nightly",
            "-wayland",
            "-native",
        ] {
            if let Some(s) = n.strip_suffix(suf) {
                n = s;
            }
        }
        if n == before {
            break;
        }
    }
    squash(n)
}

/// Names that are shared toolkit/session/runtime state: never a leftover.
const SHARED: &[&str] = &[
    "gtk-2.0",
    "gtk-3.0",
    "gtk-4.0",
    "gtkrc",
    "gtkrc-2.0",
    "glib-2.0",
    "dconf",
    "ibus",
    "fontconfig",
    "gstreamer-1.0",
    "webkitgtk-4.0",
    "webkitgtk-4.1",
    "webkitgtk-6.0",
    "gegl-0.4",
    "QtProject.conf",
    "kdeglobals",
    "kde.org",
    "Trolltech.conf",
    "systemd",
    "environment.d",
    "autostart",
    "menus",
    "mimeapps.list",
    "user-dirs.dirs",
    "user-dirs.locale",
    "pulse",
    "pipewire",
    "wireplumber",
    "goa-1.0",
    "procps",
    "gvfs-metadata",
    "folks",
    "dbus-1",
    "applications",
    "icons",
    "fonts",
    "mime",
    "sounds",
    "desktop-directories",
    "Trash",
    "keyrings",
    "recently-used.xbel",
    "user-places.xbel",
    "icc",
    "vulkan",
    "man",
    "flatpak",
    "containers",
    "Steam",
    "pki",
    "IsolatedStorage",
    "Sentry",
    "openvr",
    "openxr",
    "unity3d",
    "Valve",
    "Epic",
    "networkmanagement",
    "orca",
    "hypr",
    "fish",
    "kitty",
    "quickshell",
    "xdg-desktop-portal",
    "nautilus",
    "gnome-session",
    "kwalletd",
    "java",
    "nvim",
    "vim",
    "git",
    "gh",
    "htop",
    "mc",
    "xfce4",
    "Kvantum",
    "qt5ct",
    "qt6ct",
    "nwg-look",
    "baloo",
    "kded5",
    "kded6",
    "akonadi",
    "zsh",
    "bash",
    "user-places.xbel.bak",
];

/// Renamed binaries and data names of packages (verified on the owner's
/// machine): package name → names it creates in HOME.
const ALIASES: &[(&str, &[&str])] = &[
    ("profile-sync-daemon", &["psd"]),
    ("plasma-login-manager", &["plasmalogin"]),
    ("bs-manager", &["bsmanager"]),
    ("dorion", &["com.spikehd.dorion"]),
];

/// Signs of personal data inside a leftover (shown as a warning).
const PERSONAL_MARKERS: &[&str] = &[
    "Cookies",
    "Login Data",
    "places.sqlite",
    "key4.db",
    "logins.json",
    "drive_c",
    "saves",
    "Saves",
    "savedata",
    "Local Storage",
    "IndexedDB",
    "Songs",
    "CustomLevels",
    "instances",
    "worlds",
];

#[derive(Debug, Clone)]
pub struct Removed {
    pub package: String,
    pub installed_at: Option<i64>,
    pub removed_at: i64,
}

/// Everything known about installed and removed software.
#[derive(Debug, Default, Clone)]
pub struct Evidence {
    /// Squashed identities of installed things.
    pub installed: HashSet<String>,
    /// Squashed name → removed package.
    pub removed: HashMap<String, Removed>,
    /// Installed Flatpak app IDs (exact).
    pub flatpaks: HashSet<String>,
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-06-24T04:59:58+0200` → unix seconds.
pub fn parse_ts(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, se) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut off = 0;
    if let Some(tz) = s.get(19..24) {
        let sign = match tz.as_bytes()[0] {
            b'+' => 1,
            b'-' => -1,
            _ => 0,
        };
        if sign != 0 {
            off =
                sign * (tz[1..3].parse::<i64>().ok()? * 3600 + tz[3..5].parse::<i64>().ok()? * 60);
        }
    }
    Some(days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + se - off)
}

pub fn format_date(ts: i64) -> String {
    // Civil date from days (inverse of days_from_civil), UTC is fine for a date.
    let z = ts.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

impl Evidence {
    /// Feed pacman.log text. Last event per package wins.
    pub fn add_pacman_log(&mut self, log: &str, installed_pkgs: &HashSet<String>) {
        let mut last_install: HashMap<&str, i64> = HashMap::new();
        let mut removed: HashMap<&str, (Option<i64>, i64)> = HashMap::new();
        for line in log.lines() {
            let Some(rest) = line.strip_prefix('[') else {
                continue;
            };
            let Some((ts, rest)) = rest.split_once(']') else {
                continue;
            };
            let Some(rest) = rest.trim_start().strip_prefix("[ALPM] ") else {
                continue;
            };
            let Some(t) = parse_ts(ts) else { continue };
            let mut it = rest.splitn(3, ' ');
            let (Some(ev), Some(pkg)) = (it.next(), it.next()) else {
                continue;
            };
            match ev {
                "installed" | "reinstalled" => {
                    last_install.insert(pkg, t);
                    removed.remove(pkg);
                }
                "upgraded" | "downgraded" => {
                    removed.remove(pkg);
                }
                "removed" => {
                    removed.insert(pkg, (last_install.get(pkg).copied(), t));
                }
                _ => {}
            }
        }
        for (pkg, (inst, rm)) in removed {
            if installed_pkgs.contains(pkg) {
                continue;
            }
            let r = Removed {
                package: pkg.to_string(),
                installed_at: inst,
                removed_at: rm,
            };
            let mut names = vec![pkg_core(pkg), squash(pkg)];
            for (p, al) in ALIASES {
                if pkg_core(pkg) == pkg_core(p) {
                    names.extend(al.iter().map(|a| squash(a)));
                }
            }
            for n in names {
                if n.len() >= 2 {
                    self.removed.entry(n).or_insert_with(|| r.clone());
                }
            }
        }
    }

    fn add_installed(&mut self, name: &str) {
        let s = squash(name);
        if !s.is_empty() {
            self.installed.insert(s);
        }
        let c = pkg_core(name);
        if !c.is_empty() {
            self.installed.insert(c);
        }
        // Reverse-DNS IDs also count by their last component.
        if name.matches('.').count() >= 2 {
            if let Some(leaf) = name.rsplit('.').next() {
                self.installed.insert(squash(leaf));
            }
        }
    }

    /// Read the real system (no root needed).
    pub fn from_system(home: &Path) -> Self {
        let mut ev = Evidence::default();
        let mut pkgs = HashSet::new();
        if let Ok(rd) = fs::read_dir("/var/lib/pacman/local") {
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
                                pkgs.insert(l.to_string());
                                ev.add_installed(l);
                            }
                            "%BASE%" => ev.add_installed(l),
                            "%PROVIDES%" | "%REPLACES%" => {
                                let n = l.split(['=', '<', '>']).next().unwrap_or(l);
                                ev.add_installed(n);
                            }
                            _ => {}
                        }
                    }
                }
                if let Ok(files) = fs::read_to_string(dir.join("files")) {
                    for f in files.lines() {
                        let name = if let Some(b) = f.strip_prefix("usr/bin/") {
                            b
                        } else if let Some(d) = f
                            .strip_prefix("usr/share/applications/")
                            .and_then(|d| d.strip_suffix(".desktop"))
                        {
                            d
                        } else if let Some(o) = f.strip_prefix("opt/") {
                            o.split('/').next().unwrap_or("")
                        } else if let Some(x) = f.strip_prefix("etc/xdg/") {
                            x.split('/').next().unwrap_or("")
                        } else {
                            continue;
                        };
                        if !name.is_empty() && !name.contains('/') {
                            ev.add_installed(name);
                        }
                    }
                }
            }
        }
        if let Ok(log) = fs::read("/var/log/pacman.log") {
            ev.add_pacman_log(&String::from_utf8_lossy(&log), &pkgs);
        }
        if let Some(path) = std::env::var_os("PATH") {
            for d in std::env::split_paths(&path) {
                if let Ok(rd) = fs::read_dir(d) {
                    for e in rd.flatten() {
                        if let Some(n) = e.file_name().to_str() {
                            ev.add_installed(n);
                        }
                    }
                }
            }
        }
        for dir in [
            PathBuf::from("/usr/share/applications"),
            home.join(".local/share/applications"),
            home.join(".local/share/flatpak/exports/share/applications"),
            PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        ] {
            let Ok(rd) = fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let Some(id) = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_suffix(".desktop"))
                else {
                    continue;
                };
                // A dead user launcher does not prove anything is installed.
                if p.starts_with(home) {
                    if let Ok(t) = fs::read_to_string(&p) {
                        if crate::extra::dead_reason(&t, home, &|_| true).is_some() {
                            continue;
                        }
                        add_desktop_fields(&mut ev, &t);
                    }
                } else if let Ok(t) = fs::read_to_string(&p) {
                    add_desktop_fields(&mut ev, &t);
                }
                ev.add_installed(id);
            }
        }
        for dir in [
            home.join(".local/share/flatpak/app"),
            PathBuf::from("/var/lib/flatpak/app"),
        ] {
            if let Ok(rd) = fs::read_dir(dir) {
                for e in rd.flatten() {
                    if let Some(n) = e.file_name().to_str() {
                        ev.flatpaks.insert(n.to_string());
                        ev.add_installed(n);
                    }
                }
            }
        }
        if let Ok(rd) = fs::read_dir("/proc") {
            for e in rd.flatten() {
                if let Ok(c) = fs::read_to_string(e.path().join("comm")) {
                    ev.add_installed(c.trim());
                }
            }
        }
        ev
    }
}

fn add_desktop_fields(ev: &mut Evidence, text: &str) {
    for l in text.lines() {
        if let Some(v) = l.strip_prefix("StartupWMClass=") {
            ev.add_installed(v.trim());
        } else if let Some(v) = l.strip_prefix("Exec=") {
            if let Some(prog) = v
                .split_whitespace()
                .find(|t| !t.contains('=') && *t != "env")
            {
                ev.add_installed(prog.rsplit('/').next().unwrap_or(prog));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Exact,
    Strong,
}

impl Tier {
    pub fn label(self) -> &'static str {
        match self {
            Tier::Exact => "pewne",
            Tier::Strong => "prawie pewne",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Verdict {
    pub tier: Tier,
    pub why: String,
}

fn is_shared(name: &str) -> bool {
    let n = squash(name.trim_start_matches('.'));
    SHARED.iter().any(|s| squash(s) == n)
}

/// Name candidates for one directory entry.
fn candidate_names(name: &str) -> Vec<String> {
    let base = name.trim_start_matches('.');
    let mut v = vec![squash(base)];
    // Files: foo.conf / foorc / foo.json.
    for ext in [
        ".conf", ".json", ".ini", ".toml", ".yaml", ".yml", ".xml", ".cfg",
    ] {
        if let Some(s) = base.strip_suffix(ext) {
            v.push(squash(s));
        }
    }
    // Reverse-DNS: also the last component.
    if base.matches('.').count() >= 2 {
        if let Some(leaf) = base.rsplit('.').next() {
            v.push(squash(leaf));
        }
    }
    v.retain(|s| s.len() >= 2);
    v.dedup();
    v
}

/// Judge one entry. `birth` = creation time (unix s) when the filesystem knows it.
pub fn judge(name: &str, birth: Option<i64>, ev: &Evidence) -> Option<Verdict> {
    if is_shared(name) {
        return None;
    }
    let names = candidate_names(name);
    if names.iter().any(|n| ev.installed.contains(n)) {
        return None;
    }
    let r = names.iter().find_map(|n| ev.removed.get(n))?;
    if let Some(b) = birth {
        // Created after the removal: another program made it.
        if b > r.removed_at + 60 {
            return None;
        }
    }
    let removed = format!(
        "pakiet {} usunięto {}",
        r.package,
        format_date(r.removed_at)
    );
    match (birth, r.installed_at) {
        (Some(b), Some(i)) if b >= i - 5 && b <= i + 1800 => Some(Verdict {
            tier: Tier::Exact,
            why: format!(
                "{removed}; katalog powstał {} s po instalacji pakietu",
                b - i
            ),
        }),
        _ => Some(Verdict {
            tier: Tier::Strong,
            why: removed,
        }),
    }
}

/// A Flatpak app's data dir whose app is installed nowhere.
fn flatpak_orphan(id: &str, ev: &Evidence) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    parts.len() >= 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
        && !ev.flatpaks.contains(id)
}

fn birth_time(p: &Path) -> Option<i64> {
    let m = fs::symlink_metadata(p).ok()?;
    let t = m.created().ok()?;
    Some(t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64)
}

fn personal_markers(p: &Path) -> Vec<&'static str> {
    let mut found = Vec::new();
    let check = |dir: &Path, found: &mut Vec<&'static str>| {
        for m in PERSONAL_MARKERS {
            if dir.join(m).exists() && !found.contains(m) {
                found.push(*m);
            }
        }
    };
    check(p, &mut found);
    if let Ok(rd) = fs::read_dir(p) {
        for e in rd.flatten().take(64) {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                check(&e.path(), &mut found);
            }
        }
    }
    found
}

/// Only `NativeMessagingHosts/` inside: a stub made by a browser extension installer.
fn is_native_messaging_stub(p: &Path) -> bool {
    let Ok(rd) = fs::read_dir(p) else {
        return false;
    };
    let names: Vec<_> = rd.flatten().map(|e| e.file_name()).collect();
    !names.is_empty() && names.iter().all(|n| n == "NativeMessagingHosts")
}

pub fn leftover_items(layout: &Layout, ev: &Evidence) -> Vec<Item> {
    let mut items = Vec::new();
    let mut roots: Vec<(PathBuf, &str)> = Vec::new();
    if let Some(c) = layout.base(Base::Config) {
        roots.push((c, "~/.config"));
    }
    if let Some(d) = layout.base(Base::Data) {
        roots.push((d, "~/.local/share"));
    }
    roots.push((PathBuf::from(".local/state"), "~/.local/state"));
    for (root, label) in &roots {
        let Ok(rd) = fs::read_dir(layout.home.join(root)) else {
            continue;
        };
        let mut names: Vec<String> = rd
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        for name in names {
            let rel = root.join(&name);
            let abs = layout.home.join(&rel);
            let Ok(meta) = fs::symlink_metadata(&abs) else {
                continue;
            };
            if meta.file_type().is_symlink() || (meta.is_dir() && is_native_messaging_stub(&abs)) {
                continue;
            }
            let Some(v) = judge(&name, birth_time(&abs), ev) else {
                continue;
            };
            let action = if meta.is_dir() {
                Action::RemoveDir
            } else {
                Action::RemoveFile
            };
            items.push(leftover_item(
                &rel, &name, label, v.tier, v.why, &abs, action,
            ));
        }
    }
    // psd "unsync" backups: <browser>-back-ovfs.base after psd is gone.
    if let Some(c) = layout.base(Base::Config) {
        if !ev.installed.contains("psd") && ev.removed.contains_key("psd") {
            if let Ok(rd) = fs::read_dir(layout.home.join(&c)) {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with("-back-ovfs.base")
                        && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                    {
                        let rel = c.join(&name);
                        items.push(leftover_item(
                            &rel,
                            &name,
                            "~/.config",
                            Tier::Strong,
                            "kopia zapasowa profilu po usuniętym profile-sync-daemon".into(),
                            &e.path(),
                            Action::RemoveDir,
                        ));
                    }
                }
            }
        }
    }
    let var_app = PathBuf::from(".var/app");
    if let Ok(rd) = fs::read_dir(layout.home.join(&var_app)) {
        for e in rd.flatten() {
            let Some(id) = e.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) && flatpak_orphan(&id, ev) {
                items.push(leftover_item(
                    &var_app.join(&id),
                    &id,
                    "~/.var/app",
                    Tier::Exact,
                    "aplikacja Flatpak nie jest zainstalowana".into(),
                    &e.path(),
                    Action::RemoveDir,
                ));
            }
        }
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

fn leftover_item(
    rel: &Path,
    name: &str,
    root: &str,
    tier: Tier,
    why: String,
    abs: &Path,
    action: Action,
) -> Item {
    let markers = if action == Action::RemoveDir {
        personal_markers(abs)
    } else {
        Vec::new()
    };
    let mut description = format!("{}: {why}. Program nie jest zainstalowany.", tier.label());
    if !markers.is_empty() {
        description.push_str(&format!(
            "\nUwaga: mogą tu być Twoje dane ({}).",
            markers.join(", ")
        ));
    }
    Item {
        id: format!("leftover:{}", rel.display()),
        name: format!("{name} ({root})"),
        description,
        category: Category::Leftovers,
        risk: Risk::Review,
        regen: Regen::Never,
        targets: vec![(rel.to_path_buf(), action)],
        guards: vec![name.to_lowercase()],
        singleton_lock: None,
        media_check: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "\
[2026-10-01T10:00:00+0200] [ALPM] installed discord (0.0.1-1)
[2026-10-21T12:00:00+0200] [ALPM] removed discord (0.0.1-1)
[2026-10-01T10:00:00+0200] [ALPM] installed ibus (1-1)
[2026-10-02T10:00:00+0200] [ALPM] removed ibus (1-1)
[2026-10-03T10:00:00+0200] [ALPM] installed profile-sync-daemon (1-1)
[2026-11-18T08:20:11+0100] [ALPM] removed profile-sync-daemon (1-1)
[2026-10-03T10:00:00+0200] [ALPM] installed zed (1-1)
[2026-10-04T10:00:00+0200] [ALPM] removed zed (1-1)
[2026-10-05T10:00:00+0200] [ALPM] installed zed (1-1)
";

    fn ev() -> Evidence {
        let mut ev = Evidence::default();
        let installed: HashSet<String> = ["zed", "discord-canary"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        for p in &installed {
            ev.add_installed(p);
        }
        ev.add_pacman_log(LOG, &installed);
        ev
    }

    #[test]
    fn timestamps() {
        assert_eq!(parse_ts("1970-01-01T01:00:00+0100"), Some(0));
        assert_eq!(
            format_date(parse_ts("2026-10-21T12:00:00+0200").unwrap()),
            "2026-10-21"
        );
    }

    #[test]
    fn judges_with_vetoes() {
        let ev = ev();
        let inst = parse_ts("2026-10-01T10:00:00+0200").unwrap();
        // Removed package, born right after install → pewne.
        assert_eq!(
            judge("discord", Some(inst + 5), &ev).unwrap().tier,
            Tier::Exact
        );
        // No birth time → prawie pewne.
        assert_eq!(judge("discord", None, &ev).unwrap().tier, Tier::Strong);
        // Born after removal (e.g. by discord-canary) → not a leftover.
        assert!(judge(
            "discord",
            Some(parse_ts("2026-10-22T00:00:00+0200").unwrap()),
            &ev
        )
        .is_none());
        // Installed sibling with a channel word stays different.
        assert!(judge("discordcanary", None, &ev).is_none());
        // Shared name wins over removed evidence.
        assert!(judge("ibus", None, &ev).is_none());
        // Reinstalled later → not removed.
        assert!(judge("zed", None, &ev).is_none());
        // Alias from a renamed binary.
        assert!(judge("psd", None, &ev).is_some());
        // Unknown names are never offered.
        assert!(judge("music-opus", None, &ev).is_none());
    }

    #[test]
    fn flatpak_ids() {
        let mut ev = Evidence::default();
        ev.flatpaks.insert("com.hypixel.HytaleLauncher".into());
        assert!(!flatpak_orphan("com.hypixel.HytaleLauncher", &ev));
        assert!(flatpak_orphan("org.gone.App", &ev));
        assert!(!flatpak_orphan("notanid", &ev));
    }
}
