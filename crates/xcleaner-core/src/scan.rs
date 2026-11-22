//! Resolve rules against HOME, measure them in parallel, clean selected ones.

use crate::{
    fsops::{DeleteReport, Home, Usage},
    procs::Processes,
    rules::{Action, Base, Category, Item, Regen, Risk, CHROMIUM_CACHE_DIRS, RULES},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

/// XDG base directories, as paths relative to HOME. A base pointing outside
/// HOME is ignored in phase 1 (home-only).
#[derive(Debug, Clone)]
pub struct Layout {
    pub home: PathBuf,
    cache: Option<PathBuf>,
    config: Option<PathBuf>,
    data: Option<PathBuf>,
}

fn xdg_rel(home: &Path, var: &str, default: &str) -> Option<PathBuf> {
    let abs = std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(default));
    abs.strip_prefix(home)
        .ok()
        .filter(|r| !r.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

impl Layout {
    pub fn from_env() -> Option<Self> {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        if !home.is_absolute() || home == Path::new("/") {
            return None;
        }
        Some(Self::with_home(home))
    }

    pub fn with_home(home: PathBuf) -> Self {
        Self {
            cache: xdg_rel(&home, "XDG_CACHE_HOME", ".cache"),
            config: xdg_rel(&home, "XDG_CONFIG_HOME", ".config"),
            data: xdg_rel(&home, "XDG_DATA_HOME", ".local/share"),
            home,
        }
    }

    pub fn base(&self, base: Base) -> Option<PathBuf> {
        match base {
            Base::Home => Some(PathBuf::new()),
            Base::Cache => self.cache.clone(),
            Base::Config => self.config.clone(),
            Base::Data => self.data.clone(),
        }
    }
}

/// Live status of one item after measuring.
#[derive(Debug, Clone)]
pub struct Found {
    pub item: Item,
    pub usage: Usage,
    /// Why it cannot be cleaned right now (e.g. app running).
    pub blocked: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub found: Vec<Found>,
    pub elapsed: Duration,
}

impl ScanResult {
    pub fn total(&self) -> u64 {
        self.found.iter().map(|f| f.usage.bytes).sum()
    }
}

fn static_items(layout: &Layout) -> Vec<Item> {
    RULES
        .iter()
        .filter_map(|r| {
            let targets: Vec<_> = r
                .targets
                .iter()
                .filter_map(|t| Some((layout.base(t.base)?.join(t.rel), t.action)))
                .collect();
            (!targets.is_empty()).then(|| Item {
                id: r.id.to_string(),
                name: r.name.to_string(),
                description: r.description.to_string(),
                category: r.category,
                risk: r.risk,
                regen: r.regen,
                targets,
                guards: r.guards.iter().map(|s| s.to_string()).collect(),
                singleton_lock: None,
                media_check: false,
                preselect: false,
            })
        })
        .collect()
}

fn is_real_dir(p: &Path) -> bool {
    fs::symlink_metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

/// Chromium/Electron profiles directly in XDG_CONFIG_HOME: a `Local State`
/// file marks the user-data dir; profiles are it and its children that hold
/// `Preferences`. Backup copies made by profile-sync-daemon are skipped.
fn chromium_items(layout: &Layout) -> Vec<Item> {
    let Some(config) = layout.config.clone() else {
        return Vec::new();
    };
    let Ok(rd) = fs::read_dir(layout.home.join(&config)) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    let mut names: Vec<String> = rd
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    for name in names {
        if name.contains("-back-ovfs") || name.contains("-backup") || name.starts_with('.') {
            continue;
        }
        let rel = config.join(&name);
        let abs = layout.home.join(&rel);
        if !is_real_dir(&abs) || !abs.join("Local State").is_file() {
            continue;
        }
        let mut profiles = vec![PathBuf::new()];
        if let Ok(sub) = fs::read_dir(&abs) {
            let mut extra: Vec<PathBuf> = sub
                .flatten()
                .filter(|e| e.path().join("Preferences").is_file() && is_real_dir(&e.path()))
                .map(|e| PathBuf::from(e.file_name()))
                .collect();
            extra.sort();
            profiles.extend(extra);
        }
        let mut targets = Vec::new();
        for prof in &profiles {
            for c in CHROMIUM_CACHE_DIRS {
                let t = rel.join(prof).join(c);
                if is_real_dir(&layout.home.join(&t)) {
                    targets.push((t, Action::ClearDir));
                }
            }
        }
        if targets.is_empty() {
            continue;
        }
        let (category, guards) = known_chromium(&name);
        items.push(Item {
            id: format!("chromium-app:{name}"),
            name: format!("{name} — pamięć podręczna"),
            description:
                "Cache stron, kodu i shaderów GPU. Logowanie, ustawienia i wiadomości zostają."
                    .into(),
            category,
            risk: Risk::Safe,
            regen: Regen::Same,
            targets,
            guards,
            singleton_lock: Some(rel.join("SingletonLock")),
            media_check: false,
            preselect: false,
        });
    }
    items
}

fn known_chromium(name: &str) -> (Category, Vec<String>) {
    let g = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
    match name {
        "google-chrome" => (Category::Browsers, g(&["chrome", "google-chrome-stable"])),
        "chromium" => (Category::Browsers, g(&["chromium"])),
        "BraveSoftware" => (Category::Browsers, g(&["brave"])),
        "vivaldi" => (Category::Browsers, g(&["vivaldi-bin"])),
        "microsoft-edge" => (Category::Browsers, g(&["msedge"])),
        "discord" => (Category::Apps, g(&["discord", "Discord"])),
        "discordcanary" => (Category::Apps, g(&["discordcanary", "DiscordCanary"])),
        "discordptb" => (Category::Apps, g(&["discordptb", "DiscordPTB"])),
        "VSCodium" => (Category::Apps, g(&["codium", "vscodium"])),
        "Code" => (Category::Apps, g(&["code"])),
        _ => (Category::Apps, Vec::new()),
    }
}

/// Directories directly in XDG_CACHE_HOME carrying a valid CACHEDIR.TAG
/// (https://bford.info/cachedir/) that no named rule covers.
fn cachedir_tag_items(layout: &Layout, covered: &[PathBuf]) -> Vec<Item> {
    const SIG: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55";
    let Some(cache) = layout.cache.clone() else {
        return Vec::new();
    };
    let Ok(rd) = fs::read_dir(layout.home.join(&cache)) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    let mut names: Vec<String> = rd
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    for name in names {
        let rel = cache.join(&name);
        if covered.iter().any(|c| c == &rel) || !is_real_dir(&layout.home.join(&rel)) {
            continue;
        }
        let tag = layout.home.join(&rel).join("CACHEDIR.TAG");
        let ok = fs::read(&tag).map(|b| b.starts_with(SIG)).unwrap_or(false);
        if !ok {
            continue;
        }
        items.push(Item {
            id: format!("cachedir-tag:{name}"),
            name: format!("{name} (oznaczony jako cache)"),
            description: "Katalog ma znacznik CACHEDIR.TAG — program sam zadeklarował, że to zbędna pamięć podręczna.".into(),
            category: Category::Development,
            risk: Risk::Low,
            regen: Regen::Slow,
            targets: vec![(rel, Action::ClearDir)],
            guards: Vec::new(),
            singleton_lock: None,
            media_check: true,
            preselect: false,
        });
    }
    items
}

/// All candidate items for this HOME (not yet measured).
pub fn discover(layout: &Layout) -> Vec<Item> {
    let covered = |items: &[Item]| -> Vec<PathBuf> {
        items
            .iter()
            .flat_map(|i| i.targets.iter().map(|(p, _)| p.clone()))
            .collect()
    };
    let evidence = crate::leftovers::Evidence::from_system(&layout.home);
    let leftovers = crate::leftovers::leftover_items(layout, &evidence);
    let mut items = static_items(layout);
    items.extend(crate::extra::steam_items(layout));
    items.extend(crate::extra::discord_items(layout));
    items.extend(chromium_items(layout));
    items.extend(crate::extra::dead_launcher_items(layout));
    let c = covered(&items);
    items.extend(cachedir_tag_items(layout, &c));
    let c = covered(&items);
    items.extend(crate::extra::cache_rest_items(layout, &c));
    // A cache item inside a leftover belongs to an app that is gone: the
    // leftover entry covers it, so do not count the same bytes twice.
    let gone: Vec<PathBuf> = leftovers
        .iter()
        .flat_map(|i| i.targets.iter().map(|(p, _)| p.clone()))
        .collect();
    items.retain(|i| {
        !i.targets
            .iter()
            .all(|(p, _)| gone.iter().any(|g| p.starts_with(g)))
    });
    items.extend(leftovers);
    // Hard deny-list: drop any target a rule should never have produced.
    for it in &mut items {
        it.targets
            .retain(|(p, _)| crate::protect::refusal(p).is_none());
    }
    items.retain(|i| !i.targets.is_empty());
    items
}

fn singleton_owner_alive(home: &Path, lock: &Path) -> bool {
    let Ok(target) = fs::read_link(home.join(lock)) else {
        return false;
    };
    let t = target.to_string_lossy();
    let Some((host, pid)) = t.rsplit_once('-') else {
        return false;
    };
    let my_host = fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
    let Ok(pid) = pid.parse::<u32>() else {
        return false;
    };
    host == my_host.trim() && Processes::pid_alive(pid)
}

fn blocked_reason(item: &Item, procs: &Processes, home: &Path) -> Option<String> {
    let guards: Vec<&str> = item.guards.iter().map(String::as_str).collect();
    if let Some(g) = procs.first_running(&guards) {
        return Some(format!("działa {g} — zamknij program i skanuj ponownie"));
    }
    if let Some(lock) = &item.singleton_lock {
        if singleton_owner_alive(home, lock) {
            return Some("aplikacja jest uruchomiona — zamknij ją i skanuj ponownie".into());
        }
    }
    None
}

/// Run `f` over `items` on all CPUs, preserving order.
fn parallel_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(items.len().max(1));
    let next = AtomicUsize::new(0);
    let mut out: Vec<(usize, R)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..n)
            .map(|_| {
                s.spawn(|| {
                    let mut local = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= items.len() {
                            break local;
                        }
                        local.push((i, f(&items[i])));
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    out.sort_by_key(|(i, _)| *i);
    out.into_iter().map(|(_, r)| r).collect()
}

/// Discover and measure everything. Items that are empty or missing are dropped.
pub fn scan(layout: &Layout) -> std::io::Result<ScanResult> {
    let start = Instant::now();
    let home = Home::open(&layout.home)?;
    let procs = Processes::snapshot();
    let items = discover(layout);
    // Measure per target so one huge directory does not serialize the rest.
    let jobs: Vec<(usize, PathBuf)> = items
        .iter()
        .enumerate()
        .flat_map(|(i, it)| it.targets.iter().map(move |(p, _)| (i, p.clone())))
        .collect();
    let measured = parallel_map(&jobs, |(_, p)| home.measure(p));
    let mut found: Vec<Found> = items
        .into_iter()
        .map(|item| Found {
            blocked: None,
            usage: Usage::default(),
            errors: Vec::new(),
            item,
        })
        .collect();
    for ((i, p), r) in jobs.iter().zip(measured) {
        match r {
            Ok(Some(u)) => found[*i].usage.add(u),
            Ok(None) => {}
            Err(e) => found[*i].errors.push(format!("{}: {e}", p.display())),
        }
    }
    found.retain(|f| f.usage.files > 0 || !f.errors.is_empty());
    for f in &mut found {
        f.blocked = blocked_reason(&f.item, &procs, &layout.home);
        if f.item.preselect && f.usage.shared_bytes * 2 > f.usage.bytes {
            f.item.preselect = false;
        }
        if f.usage.shared_bytes * 10 > f.usage.bytes {
            f.item.description.push_str(&format!(
                "\nTwarde linki: {} jest współdzielone z innymi miejscami — realnie zwolni się mniej.",
                crate::format_size(f.usage.shared_bytes)
            ));
        }
        if f.item.media_check && f.usage.mostly_media() {
            f.item.risk = Risk::Review;
            f.item.regen = Regen::Never;
            f.item.description = "Głównie muzyka lub filmy — to raczej Twoje pliki, nie śmieci. Sprawdź przed usunięciem.".into();
        }
    }
    Ok(ScanResult {
        found,
        elapsed: start.elapsed(),
    })
}

#[derive(Debug, Clone, Default)]
pub struct CleanReport {
    pub freed: Usage,
    pub skipped: u64,
    pub errors: Vec<String>,
    /// Items not cleaned because their app started meanwhile.
    pub blocked: Vec<String>,
}

/// Clean the given items. Guards are re-checked right before deleting.
pub fn clean(layout: &Layout, items: &[Item]) -> std::io::Result<CleanReport> {
    let home = Home::open(&layout.home)?;
    let procs = Processes::snapshot();
    let mut rep = CleanReport::default();
    for item in items {
        if let Some(why) = blocked_reason(item, &procs, &layout.home) {
            rep.blocked.push(format!("{}: {why}", item.name));
            continue;
        }
        for (rel, action) in &item.targets {
            if let Some(why) = crate::protect::refusal(rel) {
                rep.errors
                    .push(format!("{}: odmowa — {why}", rel.display()));
                continue;
            }
            let r: std::io::Result<DeleteReport> = match action {
                Action::ClearDir => home.clear_dir(rel),
                Action::ClearDirKeep(keep) => home.clear_dir_keep(rel, keep),
                Action::RemoveFile => home.remove_file(rel),
                Action::RemoveDir => home.remove_dir(rel),
            };
            match r {
                Ok(d) => {
                    rep.freed.add(d.freed);
                    rep.skipped += d.skipped;
                    rep.errors.extend(
                        d.errors
                            .into_iter()
                            .map(|e| format!("{}: {e}", rel.display())),
                    );
                }
                Err(e) => rep.errors.push(format!("{}: {e}", rel.display())),
            }
        }
    }
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("xcleaner-scan-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn discovers_electron_and_cachedir_tag_but_not_login_data() {
        let home = tmp("disc");
        let app = home.join(".config/someapp");
        fs::create_dir_all(app.join("Cache/Cache_Data")).unwrap();
        fs::create_dir_all(app.join("Local Storage")).unwrap();
        fs::write(app.join("Local State"), b"{}").unwrap();
        fs::write(app.join("Cache/Cache_Data/x"), b"x").unwrap();
        fs::create_dir_all(home.join(".config/google-chrome-back-ovfs.base/Cache")).unwrap();
        fs::write(
            home.join(".config/google-chrome-back-ovfs.base/Local State"),
            b"{}",
        )
        .unwrap();
        let tagged = home.join(".cache/build-target");
        fs::create_dir_all(&tagged).unwrap();
        fs::write(
            tagged.join("CACHEDIR.TAG"),
            b"Signature: 8a477f597d28d172789f06886806bc55\n",
        )
        .unwrap();
        fs::create_dir_all(home.join(".cache/untagged")).unwrap();

        let layout = Layout {
            cache: Some(".cache".into()),
            config: Some(".config".into()),
            data: Some(".local/share".into()),
            home: home.clone(),
        };
        let items = discover(&layout);
        let app_item = items
            .iter()
            .find(|i| i.id == "chromium-app:someapp")
            .unwrap();
        assert_eq!(
            app_item.targets,
            vec![(PathBuf::from(".config/someapp/Cache"), Action::ClearDir)]
        );
        assert!(!items
            .iter()
            .any(|i| i.id.starts_with("chromium-app:") && i.id.contains("back-ovfs")));
        assert!(items.iter().any(|i| i.id == "cachedir-tag:build-target"));
        assert!(!items.iter().any(|i| i.id == "cachedir-tag:untagged"));

        let res = scan(&layout).unwrap();
        assert!(res
            .found
            .iter()
            .any(|f| f.item.id == "chromium-app:someapp" && f.usage.files == 1));
        let sel: Vec<Item> = res.found.iter().map(|f| f.item.clone()).collect();
        let rep = clean(&layout, &sel).unwrap();
        assert!(rep.errors.is_empty(), "{:?}", rep.errors);
        assert!(app.join("Local Storage").is_dir());
        assert!(app.join("Cache").is_dir());
        assert!(!app.join("Cache/Cache_Data").exists());
        fs::remove_dir_all(&home).unwrap();
    }
}
