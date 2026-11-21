//! Discovered (dynamic) items: Steam, Discord updates, the rest of
//! XDG_CACHE_HOME and dead application launchers.

use crate::{
    rules::{Action, Category, Item, Regen, Risk},
    scan::Layout,
};
use std::{
    collections::HashMap,
    fs,
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

pub fn steam_items(layout: &Layout) -> Vec<Item> {
    let Some(data) = layout.base(crate::rules::Base::Data) else {
        return Vec::new();
    };
    let root = data.join("Steam");
    let abs = layout.home.join(&root);
    if !is_real_dir(&abs) {
        return Vec::new();
    }
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
                format!("steam-shader:{d}"),
                format!("Shadery Steam: {game}"),
                "Skompilowane shadery gry. Po usunięciu gra przycina, dopóki się nie odbudują."
                    .into(),
                Category::Games,
                Risk::Low,
                Regen::Slow,
            ),
            None if id < 0x8000_0000 => item(
                format!("steam-shader:{d}"),
                format!("Shadery po odinstalowanej grze ({id})"),
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
        "steam-logs".into(),
        "Steam — logi i zrzuty awarii".into(),
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
        "steam-httpcache".into(),
        "Steam — cache sklepu i obrazków".into(),
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
const CACHE_STATEFUL: &[&str] = &["keepassxc"];

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
        let stateful = CACHE_STATEFUL.iter().any(|s| s.eq_ignore_ascii_case(&name));
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
