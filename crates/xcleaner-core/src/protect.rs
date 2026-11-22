//! Hard deny-list. Checked when items are discovered AND again right before
//! deletion, so no rule (built-in or future TOML) can reach these paths.

use std::path::{Component, Path};

/// Paths (relative to HOME) that are never deleted, nor anything inside them.
const NEVER_INSIDE: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".pki",
    ".password-store",
    ".local/share/keyrings",
    ".local/share/kwalletd",
    ".local/bin",
    ".claude",
    ".codex",
    ".config/gh",
    ".config/hypr",
    ".config/fish",
    ".config/kitty",
    ".config/quickshell",
    ".config/systemd",
    ".config/autostart",
    ".local/share/Steam/userdata",
    ".local/share/Steam/config",
    ".local/share/Steam/steamapps/common",
    ".local/share/Steam/steamapps/compatdata",
    ".local/share/Steam/steamapps/workshop",
    ".mozilla",
    ".zen",
    ".wine",
    ".minecraft",
    ".local/share/PrismLauncher/instances",
    // Desktop user data that looks like cache but is not
    // (docs/research/DISTRO-DESKTOPS-TERMINALS-2026-11.md).
    ".local/share/gvfs-metadata", // emblems, custom icons, folder view settings
    ".local/share/nautilus",      // Starred files
    ".local/share/akonadi",       // KMail / KOrganizer database
    ".local/share/kactivitymanagerd", // activity history and Kickoff favourites
    ".local/share/cinnamon",      // installed applets, desklets, extensions
    ".local/share/evolution",     // local mail (POP, "On This Computer")
    ".local/share/gnome-shell/application_state",
    ".config/xfce4",
    ".config/cosmic",
    // Flatpak and Snap app data / config (`*` = one path component).
    ".var/app/*/data",
    ".var/app/*/config",
    "snap/*/common/.mozilla",
    "snap/*/common/.thunderbird",
];

/// Paths that are containers: allowed below a deeper rule, never as a target itself.
const NEVER_EXACT: &[&str] = &[
    ".cache",
    ".config",
    ".local",
    ".local/share",
    ".local/state",
    ".local/share/Steam",
    ".local/share/Steam/steamapps",
    ".steam",
    ".var",
    ".var/app",
    ".cargo",
    ".npm",
    ".rustup",
    // Not `.var/app/*` or `.var/app/*/cache`: those are legitimate targets
    // (leftovers of uninstalled Flatpaks, Flatpak cache).
    "snap",
    "snap/*",
    "snap/*/common",
    "snap/*/current",
];

/// Component-wise prefix match; `*` matches exactly one component.
fn pattern_prefix(rel: &Path, pat: &str) -> bool {
    let mut r = rel.components();
    for p in Path::new(pat).components() {
        match r.next() {
            Some(c) if p.as_os_str() == "*" || c == p => {}
            _ => return false,
        }
    }
    true
}

fn pattern_exact(rel: &Path, pat: &str) -> bool {
    rel.components().count() == Path::new(pat).components().count() && pattern_prefix(rel, pat)
}

/// Parts of a Steam installation that are never deleted, relative to its root.
const STEAM_NEVER_INSIDE: &[&str] = &[
    "userdata",
    "config",
    "steamapps/common",
    "steamapps/compatdata",
    "steamapps/workshop",
];

/// Containers inside a Steam installation: never a target themselves.
const STEAM_NEVER_EXACT: &[&str] = &["", "steamapps", "steamapps/shadercache"];

/// Refusal for paths inside any of the given Steam roots (relative to HOME).
pub fn steam_refusal(roots: &[std::path::PathBuf], rel: &Path) -> Option<&'static str> {
    for root in roots {
        let Ok(sub) = rel.strip_prefix(root) else {
            continue;
        };
        if STEAM_NEVER_EXACT.iter().any(|p| sub == Path::new(p)) {
            return Some("katalog zbiorczy Steam");
        }
        if STEAM_NEVER_INSIDE.iter().any(|p| sub.starts_with(p)) {
            return Some("chroniona lokalizacja (klucze, konfiguracja, dane gier)");
        }
    }
    None
}

/// Why a target is refused, or None when it may be cleaned.
pub fn refusal(rel: &Path) -> Option<&'static str> {
    let comps: Vec<_> = rel.components().collect();
    if comps.is_empty() || comps.iter().any(|c| !matches!(c, Component::Normal(_))) {
        return Some("ścieżka spoza katalogu domowego");
    }
    // Top-level HOME entries (Documents, Pictures, .config…) are never targets.
    if comps.len() < 2 {
        return Some("katalog najwyższego poziomu w HOME");
    }
    if NEVER_EXACT.iter().any(|p| pattern_exact(rel, p)) {
        return Some("katalog zbiorczy");
    }
    if NEVER_INSIDE.iter().any(|p| pattern_prefix(rel, p)) {
        return Some("chroniona lokalizacja (klucze, konfiguracja, dane gier)");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_dangerous_paths() {
        for p in [
            ".config",
            ".local/share",
            ".ssh/id_ed25519",
            ".local/share/keyrings/login.keyring",
            ".local/share/Steam/userdata/123/config",
            ".local/share/Steam/steamapps/compatdata/292030/pfx",
            "Documents",
            "Obrazy",
            ".local/bin/steam",
            ".config/hypr/hyprland.lua",
            ".local/share/gvfs-metadata/home",
            ".local/share/nautilus/tags",
            ".var/app/org.gimp.GIMP/config/GIMP",
            ".var/app/org.gimp.GIMP/data",
            "snap/firefox",
            "snap/firefox/common/.mozilla/firefox",
        ] {
            assert!(refusal(Path::new(p)).is_some(), "{p}");
        }
    }

    #[test]
    fn steam_protection_follows_every_root() {
        let roots = vec![
            std::path::PathBuf::from(".steam/debian-installation"),
            std::path::PathBuf::from(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        ];
        for p in [
            ".steam/debian-installation",
            ".steam/debian-installation/userdata/1/config",
            ".steam/debian-installation/steamapps/compatdata/730",
            ".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/Game",
            ".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps",
        ] {
            assert!(steam_refusal(&roots, Path::new(p)).is_some(), "{p}");
        }
        for p in [
            ".steam/debian-installation/steamapps/shadercache/730",
            ".steam/debian-installation/logs",
            ".steam/other/userdata",
        ] {
            assert_eq!(steam_refusal(&roots, Path::new(p)), None, "{p}");
        }
    }

    #[test]
    fn allows_cache_leaves() {
        for p in [
            ".cache/thumbnails",
            ".config/discordcanary/Cache",
            ".local/share/Steam/steamapps/shadercache/730",
            ".local/share/Steam/logs",
            ".var/app/org.gimp.GIMP/cache/gimp",
            ".var/app/org.gimp.GIMP/cache",
            ".var/app/org.gimp.GIMP",
            "snap/firefox/common/.cache/mozilla",
            "snap/firefox/common/.cache",
            "snap/firefox/7",
            ".local/share/nautilusx",
        ] {
            assert_eq!(refusal(Path::new(p)), None, "{p}");
        }
    }
}
