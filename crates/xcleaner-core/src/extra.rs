//! Discovered (dynamic) items: Steam, Discord updates, the rest of
//! XDG_CACHE_HOME and dead application launchers.

use crate::{
    rules::{Action, Category, Item, Regen, Risk},
    scan::Layout,
};
use std::{
    collections::HashMap,
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

fn is_real_dir(p: &Path) -> bool {
    fs::symlink_metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

fn sorted_names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn item(
    id: String,
    name: String,
    description: String,
    category: Category,
    risk: Risk,
    regen: Regen,
) -> Item {
    Item {
        id,
        name,
        description,
        category,
        risk,
        regen,
        targets: Vec::new(),
        guards: Vec::new(),
        singleton_lock: None,
        media_check: false,
        preselect: false,
    }
}

// ---------------------------------------------------------------- Steam

/// `"name"  "Counter-Strike 2"` from an appmanifest .acf.
fn acf_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let mut parts = l.split('"').filter(|s| !s.trim().is_empty());
        (parts.next()? == key).then(|| parts.next().map(str::to_string))?
    })
}

/// Steam stores shortcut ids both as u32 and as negative i32 directory names.
fn steam_id(name: &str) -> Option<u32> {
    name.parse::<u32>()
        .ok()
        .or_else(|| name.parse::<i32>().ok().map(|v| v as u32))
}

/// Every Steam installation in HOME, relative to it: native, Debian/Ubuntu
/// package, Flathub and Snap. Symlinks (`~/.steam/root`) are resolved but
/// must stay inside HOME; duplicates are dropped by (dev, inode); a root
/// counts only with a `steamapps` directory.
pub fn steam_roots(home: &Path, data: Option<&Path>) -> Vec<PathBuf> {
    let Ok(home_real) = fs::canonicalize(home) else {
        return Vec::new();
    };
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(d) = data {
        cands.push(d.join("Steam"));
    }
    for c in [
        ".local/share/Steam",
        ".steam/debian-installation",
        ".steam/steam",
        ".steam/root",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam",
        ".var/app/com.valvesoftware.Steam/data/Steam",
        ".var/app/com.valvesoftware.Steam/.steam/root",
        "snap/steam/common/.local/share/Steam",
    ] {
        cands.push(PathBuf::from(c));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in cands {
        let Ok(real) = fs::canonicalize(home.join(&c)) else {
            continue;
        };
        let Ok(rel) = real.strip_prefix(&home_real) else {
            continue;
        };
        let Ok(meta) = fs::metadata(&real) else {
            continue;
        };
        if !meta.is_dir() || !is_real_dir(&real.join("steamapps")) {
            continue;
        }
        if seen.insert((meta.dev(), meta.ino())) {
            out.push(rel.to_path_buf());
        }
    }
    out
}

/// Label shown after item names for non-native installations.
fn steam_flavour(root: &Path) -> &'static str {
    let s = root.to_string_lossy();
    if s.starts_with(".var/app/") {
        " (Flatpak)"
    } else if s.starts_with("snap/") {
        " (Snap)"
    } else if s.contains("debian-installation") {
        " (pakiet deb)"
    } else {
        ""
    }
}

pub fn steam_items(layout: &Layout) -> Vec<Item> {
    let mut items = Vec::new();
    for (i, root) in layout.steam_roots.iter().enumerate() {
        // The first root keeps the old ids, so keep.txt entries stay valid.
        let tag = if i == 0 {
            String::new()
        } else {
            format!("{}:", root.display())
        };
        items.extend(steam_root_items(layout, root, &tag, steam_flavour(root)));
    }
    items
}

fn steam_root_items(layout: &Layout, root: &Path, tag: &str, flavour: &str) -> Vec<Item> {
    // "" for the first root, ":<root>" for the others.
    let suffix = tag
        .strip_suffix(':')
        .map(|t| format!(":{t}"))
        .unwrap_or_default();
    let apps = root.join("steamapps");
    let mut names: HashMap<u32, String> = HashMap::new();
    for f in sorted_names(&layout.home.join(&apps)) {
        let Some(id) = f
            .strip_prefix("appmanifest_")
            .and_then(|r| r.strip_suffix(".acf"))
            .and_then(|r| r.parse::<u32>().ok())
        else {
            continue;
        };
        if let Ok(t) = fs::read_to_string(layout.home.join(&apps).join(&f)) {
            names.insert(
                id,
                acf_value(&t, "name").unwrap_or_else(|| format!("gra {id}")),
            );
        }
    }
    // Non-Steam shortcuts (shortcuts.vdf) get shader caches too; their ids
    // are not in appmanifests. Treat ids above 2^31 as shortcuts, not orphans.
    let mut items = Vec::new();
    let sc = apps.join("shadercache");
    for d in sorted_names(&layout.home.join(&sc)) {
        let Some(id) = steam_id(&d) else { continue };
        let rel = sc.join(&d);
        if !is_real_dir(&layout.home.join(&rel)) {
            continue;
        }
        let installed = names.contains_key(&id);
        let mut it = match names.get(&id) {
            Some(game) => item(
                format!("steam-shader:{tag}{d}"),
                format!("Shadery Steam{flavour}: {game}"),
                "Skompilowane shadery gry. Po usunięciu gra przycina, dopóki się nie odbudują."
                    .into(),
                Category::Games,
                Risk::Low,
                Regen::Slow,
            ),
            None if id < 0x8000_0000 => item(
                format!("steam-shader:{tag}{d}"),
                format!("Shadery po odinstalowanej grze{flavour} ({id})"),
                "Gra nie jest już zainstalowana, a jej shadery zostały na dysku.".into(),
                Category::Games,
                Risk::Safe,
                Regen::Never,
            ),
            None => continue,
        };
        // Installed game: keep the folder Steam expects; orphan: remove it all.
        let action = if installed {
            Action::ClearDir
        } else {
            Action::RemoveDir
        };
        it.targets.push((rel, action));
        it.guards = vec!["steam".into()];
        items.push(it);
    }
    let mut logs = item(
        format!("steam-logs{suffix}"),
        format!("Steam{flavour} — logi i zrzuty awarii"),
        "Pliki dziennika i zrzuty awarii klienta Steam.".into(),
        Category::Games,
        Risk::Safe,
        Regen::Never,
    );
    for d in ["logs", "dumps"] {
        logs.targets.push((root.join(d), Action::ClearDir));
    }
    items.push(logs);
    let mut http = item(
        format!("steam-httpcache{suffix}"),
        format!("Steam{flavour} — cache sklepu i obrazków"),
        "Pobrane strony sklepu i grafiki biblioteki.".into(),
        Category::Games,
        Risk::Safe,
        Regen::Same,
    );
    http.targets
        .push((root.join("appcache/httpcache"), Action::ClearDir));
    http.guards = vec!["steam".into()];
    items.push(http);
    items
}

// -------------------------------------------------------------- Discord

fn version_key(s: &str) -> Option<Vec<u64>> {
    let v: Option<Vec<u64>> = s.split('.').map(|p| p.parse().ok()).collect();
    v.filter(|v| v.len() >= 3)
}

/// Discord keeps each self-update in `app-<ver>` (+ `<ver>` with modules).
/// Everything older than the newest version is dead weight.
pub fn discord_items(layout: &Layout) -> Vec<Item> {
    let Some(config) = layout.base(crate::rules::Base::Config) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for app in ["discord", "discordcanary", "discordptb"] {
        let rel = config.join(app);
        let names = sorted_names(&layout.home.join(&rel));
        let versions: Vec<(Vec<u64>, &String)> = names
            .iter()
            .filter_map(|n| Some((version_key(n.strip_prefix("app-").unwrap_or(n))?, n)))
            .collect();
        let Some(newest) = versions.iter().map(|(v, _)| v.clone()).max() else {
            continue;
        };
        let mut it = item(
            format!("discord-old:{app}"),
            format!("{app} — stare wersje po aktualizacji"),
            "Poprzednie wersje programu pobrane przez auto-aktualizację. Używana jest tylko najnowsza.".into(),
            Category::Apps,
            Risk::Safe,
            Regen::Never,
        );
        for (v, n) in &versions {
            if *v != newest && is_real_dir(&layout.home.join(&rel).join(n)) {
                it.targets.push((rel.join(n), Action::RemoveDir));
            }
        }
        if !it.targets.is_empty() {
            it.singleton_lock = Some(rel.join("SingletonLock"));
            it.guards = vec![app.into()];
            items.push(it);
        }
    }
    items
}

// ---------------------------------------------------- rest of ~/.cache

/// Cache entries that hold state despite living in XDG_CACHE_HOME.
/// ~/.cache entries that hold user state, not cache (see
/// docs/research/DISTRO-DESKTOPS-TERMINALS-2026-11.md): never preselected.
const CACHE_STATEFUL: &[&str] = &[
    "keepassxc",
    "sessions",  // XFCE saved sessions
    "xfce4",     // clipman / appfinder history
    "obexd",     // files received over Bluetooth
    "ibus",      // libpinyin / libbopomofo user dictionaries
    "JetBrains", // IDE Local History
    "kioexec",   // copies of remote files being edited
    "evolution", // offline mail copies, may hold unsynced changes
    "tracker3",  // rebuilds, but costs a full reindex and loses tags
];

fn is_stateful_cache(name: &str) -> bool {
    CACHE_STATEFUL.iter().any(|s| s.eq_ignore_ascii_case(name))
}

/// One item per top-level entry of XDG_CACHE_HOME not handled by a named
/// rule. The XDG spec defines this directory as non-essential data.
pub fn cache_rest_items(layout: &Layout, covered: &[PathBuf]) -> Vec<Item> {
    let Some(cache) = layout.base(crate::rules::Base::Cache) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for name in sorted_names(&layout.home.join(&cache)) {
        let rel = cache.join(&name);
        if covered
            .iter()
            .any(|c| c.starts_with(&rel) || rel.starts_with(c))
        {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(layout.home.join(&rel)) else {
            continue;
        };
        let action = if meta.is_dir() {
            Action::ClearDir
        } else if meta.is_file() {
            Action::RemoveFile
        } else {
            continue;
        };
        let stateful = is_stateful_cache(&name);
        let mut it = item(
            format!("cache:{name}"),
            name.to_string(),
            if stateful {
                "Ten program trzyma w cache także swój stan — sprawdź przed usunięciem.".into()
            } else {
                "Pamięć podręczna programu (~/.cache). Program odtworzy ją sam, gdy będzie potrzebna.".into()
            },
            Category::System,
            if stateful { Risk::Review } else { Risk::Safe },
            Regen::Same,
        );
        it.targets.push((rel, action));
        // Best effort: a process with the directory's name.
        it.guards = vec![name.to_lowercase()];
        it.media_check = true;
        items.push(it);
    }
    items
}

// ------------------------------------------------------- Flatpak cache

/// The app itself, or something in its cache directory, keeps state there.
fn cache_dir_is_stateful(app: &str, cache_abs: &Path) -> bool {
    is_stateful_cache(app) || sorted_names(cache_abs).iter().any(|n| is_stateful_cache(n))
}

/// `~/.var/app/<id>/cache` of installed Flatpak apps (orphans are leftovers).
/// Inside the sandbox it is `/var/cache` (and `cache/tmp` is `/var/tmp`).
pub fn flatpak_cache_items(layout: &Layout) -> Vec<Item> {
    let var_app = PathBuf::from(".var/app");
    let mut items = Vec::new();
    for id in sorted_names(&layout.home.join(&var_app)) {
        if !flatpak_installed(&layout.home, &id) {
            continue;
        }
        let rel = var_app.join(&id).join("cache");
        let abs = layout.home.join(&rel);
        if !is_real_dir(&abs) {
            continue;
        }
        let leaf = id.rsplit('.').next().unwrap_or(&id).to_lowercase();
        let stateful = cache_dir_is_stateful(&leaf, &abs);
        let mut it = item(
            format!("flatpak-cache:{id}"),
            format!("{id} — cache Flatpak"),
            if stateful {
                "Cache aplikacji Flatpak, ale ta aplikacja trzyma tam także swój stan — sprawdź przed usunięciem."
                    .into()
            } else {
                "Pamięć podręczna aplikacji Flatpak. Ustawienia i dane (config, data) zostają."
                    .into()
            },
            Category::Apps,
            if stateful { Risk::Review } else { Risk::Safe },
            Regen::Same,
        );
        it.targets.push((rel, Action::ClearDir));
        // A running instance (cache/tmp is its /var/tmp) blocks the clean.
        it.guards = vec![format!("flatpak:{}", id.to_lowercase()), leaf];
        it.media_check = true;
        items.push(it);
    }
    items
}

// ----------------------------------------------------------------- Snap

/// Revision directory names: `123`, or `x1` for sideloaded snaps.
fn is_snap_revision(name: &str) -> bool {
    let digits = name.strip_prefix('x').unwrap_or(name);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// `~/snap/<name>` of installed snaps: the app cache in `common/.cache`
/// (snap desktop helpers set XDG_CACHE_HOME there) and data of revisions
/// that are no longer installed (snapd removes the data of revisions it
/// removes, so these are left behind by failed or manual operations).
pub fn snap_items(layout: &Layout) -> Vec<Item> {
    snap_items_in(layout, Path::new("/snap"))
}

fn snap_items_in(layout: &Layout, snap_root: &Path) -> Vec<Item> {
    let base = PathBuf::from("snap");
    let mut items = Vec::new();
    for name in sorted_names(&layout.home.join(&base)) {
        let installed = snap_root.join(&name);
        if name.starts_with('.')
            || !is_real_dir(&layout.home.join(&base).join(&name))
            || !installed.is_dir()
        {
            continue;
        }
        let guards = vec![format!("snap:{}", name.to_lowercase()), name.to_lowercase()];
        let cache = base.join(&name).join("common/.cache");
        let cache_abs = layout.home.join(&cache);
        if is_real_dir(&cache_abs) && !sorted_names(&cache_abs).is_empty() {
            let stateful = cache_dir_is_stateful(&name, &cache_abs);
            let mut it = item(
                format!("snap-cache:{name}"),
                format!("{name} — cache Snap"),
                if stateful {
                    "Cache aplikacji Snap, ale ta aplikacja trzyma tam także swój stan — sprawdź przed usunięciem."
                        .into()
                } else {
                    "Pamięć podręczna aplikacji Snap (~/snap/…/common/.cache). Ustawienia i dane zostają.".into()
                },
                Category::Apps,
                if stateful { Risk::Review } else { Risk::Safe },
                Regen::Same,
            );
            it.targets.push((cache, Action::ClearDir));
            it.guards = guards.clone();
            it.media_check = true;
            items.push(it);
        }
        let current = fs::read_link(layout.home.join(&base).join(&name).join("current"))
            .ok()
            .and_then(|t| t.file_name().map(|n| n.to_string_lossy().to_string()));
        let mut stale = item(
            format!("snap-old-revisions:{name}"),
            format!("{name} — dane nieistniejących rewizji Snap"),
            "Dane rewizji, których nie ma już w /snap. snap revert ich nie użyje; aktualna rewizja i common zostają."
                .into(),
            Category::Leftovers,
            Risk::Low,
            Regen::Never,
        );
        for rev in sorted_names(&layout.home.join(&base).join(&name)) {
            let rel = base.join(&name).join(&rev);
            if !is_snap_revision(&rev)
                || current.as_deref() == Some(rev.as_str())
                || installed.join(&rev).exists()
                || !is_real_dir(&layout.home.join(&rel))
            {
                continue;
            }
            stale.targets.push((rel, Action::RemoveDir));
        }
        if !stale.targets.is_empty() {
            stale.guards = guards;
            items.push(stale);
        }
    }
    items
}

// ------------------------------------------------------ dead launchers

fn split_exec(exec: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            '\\' if quoted => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            ' ' | '\t' if !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn in_path(prog: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(prog).exists()))
        .unwrap_or(false)
}

fn program_missing(prog: &str, home: &Path) -> bool {
    let p = if let Some(rest) = prog.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(prog)
    };
    if p.is_absolute() {
        !p.exists()
    } else {
        !in_path(prog)
    }
}

/// Why a .desktop launcher is dead, or None when it still works or is unclear.
pub fn dead_reason(
    text: &str,
    home: &Path,
    flatpak_installed: &dyn Fn(&str) -> bool,
) -> Option<String> {
    let mut in_entry = false;
    let mut exec = None;
    let mut try_exec = None;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_entry = l == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some(v) = l.strip_prefix("Exec=") {
            exec = Some(v.to_string());
        } else if let Some(v) = l.strip_prefix("TryExec=") {
            try_exec = Some(v.to_string());
        }
    }
    if let Some(t) = try_exec.filter(|t| !t.is_empty()) {
        if program_missing(&t, home) {
            return Some(format!("brak programu {t}"));
        }
    }
    let tokens = split_exec(&exec?);
    let mut i = 0;
    let mut wineprefix = None;
    if tokens.first().map(String::as_str) == Some("env") {
        i = 1;
        while let Some(t) = tokens.get(i) {
            if let Some((k, v)) = t.split_once('=') {
                if k == "WINEPREFIX" {
                    wineprefix = Some(v.to_string());
                }
                i += 1;
            } else {
                break;
            }
        }
    }
    let prog = tokens.get(i)?;
    if prog.contains('%') || prog.contains('$') {
        return None;
    }
    if program_missing(prog, home) {
        return Some(format!("brak programu {prog}"));
    }
    let base = prog.rsplit('/').next().unwrap_or(prog);
    if base == "flatpak" && tokens.get(i + 1).map(String::as_str) == Some("run") {
        let id = tokens[i + 2..].iter().find(|t| !t.starts_with('-'))?;
        if !flatpak_installed(id) {
            return Some(format!("aplikacja Flatpak {id} nie jest zainstalowana"));
        }
    }
    if matches!(base, "wine" | "wine64") {
        if let Some(pfx) = wineprefix {
            if !Path::new(&pfx).is_dir() {
                return Some(format!("brak prefiksu Wine {pfx}"));
            }
        }
    }
    None
}

fn flatpak_installed(home: &Path, id: &str) -> bool {
    home.join(".local/share/flatpak/app").join(id).exists()
        || Path::new("/var/lib/flatpak/app").join(id).exists()
}

pub fn dead_launcher_items(layout: &Layout) -> Vec<Item> {
    let Some(data) = layout.base(crate::rules::Base::Data) else {
        return Vec::new();
    };
    let rel_dir = data.join("applications");
    let abs_dir = layout.home.join(&rel_dir);
    let home = layout.home.clone();
    let mut it = item(
        "dead-launchers".into(),
        "Martwe skróty aplikacji".into(),
        String::new(),
        Category::System,
        Risk::Safe,
        Regen::Never,
    );
    let mut reasons = Vec::new();
    for name in sorted_names(&abs_dir) {
        if !name.ends_with(".desktop") {
            continue;
        }
        let p = abs_dir.join(&name);
        let Ok(meta) = fs::symlink_metadata(&p) else {
            continue;
        };
        let why = if meta.file_type().is_symlink() {
            (!p.exists()).then(|| "zepsuty link".to_string())
        } else if meta.is_file() {
            fs::read_to_string(&p)
                .ok()
                .and_then(|t| dead_reason(&t, &home, &|id| flatpak_installed(&home, id)))
        } else {
            None
        };
        if let Some(why) = why {
            reasons.push(format!("{name}: {why}"));
            it.targets.push((rel_dir.join(&name), Action::RemoveFile));
        }
    }
    if it.targets.is_empty() {
        return Vec::new();
    }
    it.description = format!(
        "Skróty w menu do programów, których już nie ma.\n{}",
        reasons
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    vec![it]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_cache_and_stale_revisions() {
        let t = std::env::temp_dir().join(format!("xc-snap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&t);
        let home = t.join("home");
        let snap = t.join("snap");
        // Installed snap with revisions 10 (current) and 11 in /snap.
        fs::create_dir_all(snap.join("firefox/10")).unwrap();
        fs::create_dir_all(snap.join("firefox/11")).unwrap();
        for d in ["10", "11", "7", "common/.cache/mozilla"] {
            fs::create_dir_all(home.join("snap/firefox").join(d)).unwrap();
        }
        std::os::unix::fs::symlink("10", home.join("snap/firefox/current")).unwrap();
        // Not installed any more: handled by leftovers, not here.
        fs::create_dir_all(home.join("snap/gone/3")).unwrap();
        let layout = Layout::with_home(home.clone());
        let items = snap_items_in(&layout, &snap);
        let ids: Vec<_> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["snap-cache:firefox", "snap-old-revisions:firefox"]
        );
        assert_eq!(
            items[1].targets,
            vec![(PathBuf::from("snap/firefox/7"), Action::RemoveDir)]
        );
        assert!(items[0].guards.contains(&"snap:firefox".to_string()));
        assert!(is_snap_revision("x2") && !is_snap_revision("current") && !is_snap_revision("x"));
        fs::remove_dir_all(&t).unwrap();
    }

    #[test]
    fn steam_roots_are_found_once_each() {
        let home = std::env::temp_dir().join(format!("xc-steam-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        let native = home.join(".local/share/Steam/steamapps");
        let flat = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps");
        fs::create_dir_all(&native).unwrap();
        fs::create_dir_all(&flat).unwrap();
        // Not a Steam root: no steamapps.
        fs::create_dir_all(home.join(".steam/debian-installation")).unwrap();
        // ~/.steam/root -> native: same root, listed once.
        std::os::unix::fs::symlink(home.join(".local/share/Steam"), home.join(".steam/root"))
            .unwrap();
        // A symlink leaving HOME is ignored.
        std::os::unix::fs::symlink("/usr", home.join(".steam/steam")).unwrap();
        let roots = steam_roots(&home, Some(Path::new(".local/share")));
        assert_eq!(
            roots,
            vec![
                PathBuf::from(".local/share/Steam"),
                PathBuf::from(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
            ]
        );
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn stateful_cache_dirs_are_never_preselected() {
        for n in [
            "JetBrains",
            "jetbrains",
            "sessions",
            "obexd",
            "tracker3",
            "keepassxc",
        ] {
            assert!(is_stateful_cache(n), "{n}");
        }
        assert!(!is_stateful_cache("fontconfig"));
    }

    #[test]
    fn acf_and_ids() {
        let t = "\"AppState\"\n{\n\t\"appid\"\t\t\"730\"\n\t\"name\"\t\t\"Counter-Strike 2\"\n}";
        assert_eq!(acf_value(t, "name").as_deref(), Some("Counter-Strike 2"));
        assert_eq!(steam_id("-767600689"), Some(3527366607));
        assert_eq!(version_key("1.0.2506"), Some(vec![1, 0, 2506]));
        assert_eq!(version_key("module_data"), None);
    }

    #[test]
    fn dead_launcher_detection() {
        let home = Path::new("/nonexistent-home");
        let no_fp = |_: &str| false;
        let d = |s: &str| dead_reason(s, home, &no_fp);
        assert!(d("[Desktop Entry]\nExec=/opt/gone/app %U\n").is_some());
        assert!(
            d("[Desktop Entry]\nExec=\"/home/x/Apps/My App.AppImage\" --no-sandbox\n").is_some()
        );
        assert!(d("[Desktop Entry]\nExec=sh -c 'echo hi'\n").is_none());
        assert!(
            d("[Desktop Entry]\nExec=flatpak run --branch=stable com.example.Gone\n").is_some()
        );
        assert!(
            d("[Desktop Entry]\nExec=env WINEPREFIX=/nope/pfx wine C:\\\\x.exe\n").is_some()
                || !in_path("wine")
        );
        assert!(d("[Desktop Entry]\nTryExec=definitely-not-a-binary-xyz\nExec=sh\n").is_some());
        // Exec in another group is ignored.
        assert!(d("[Desktop Entry]\nExec=sh\n[Desktop Action x]\nExec=/gone\n").is_none());
    }

    #[test]
    fn discord_keeps_newest() {
        let home = std::env::temp_dir().join(format!("xc-discord-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        for d in ["app-1.0.9", "app-1.0.10", "1.0.10", "1.0.9", "Cache"] {
            fs::create_dir_all(home.join(".config/discord").join(d)).unwrap();
        }
        let items = discord_items(&Layout::with_home(home.clone()));
        let t: Vec<_> = items[0].targets.iter().map(|(p, _)| p.clone()).collect();
        assert_eq!(
            t,
            vec![
                PathBuf::from(".config/discord/1.0.9"),
                PathBuf::from(".config/discord/app-1.0.9")
            ]
        );
        fs::remove_dir_all(&home).unwrap();
    }
}
