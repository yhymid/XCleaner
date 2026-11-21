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
    ".var/app",
    ".wine",
    ".minecraft",
    ".local/share/PrismLauncher/instances",
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
    ".cargo",
    ".npm",
    ".rustup",
];

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
    if NEVER_EXACT.iter().any(|p| rel == Path::new(p)) {
        return Some("katalog zbiorczy");
    }
    if NEVER_INSIDE.iter().any(|p| rel.starts_with(p)) {
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
        ] {
            assert!(refusal(Path::new(p)).is_some(), "{p}");
        }
    }

    #[test]
    fn allows_cache_leaves() {
        for p in [
            ".cache/thumbnails",
            ".config/discordcanary/Cache",
            ".local/share/Steam/steamapps/shadercache/730",
            ".local/share/Steam/logs",
        ] {
            assert_eq!(refusal(Path::new(p)), None, "{p}");
        }
    }
}
