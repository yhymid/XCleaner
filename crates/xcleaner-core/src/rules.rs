//! Built-in cleaning rules. Every rule names exact locations; nothing is
//! guessed from a name match. Paths are relative to their XDG base.

use crate::i18n::T;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    System,
    Browsers,
    Apps,
    Leftovers,
    Projects,
    Development,
    SystemRoot,
    Games,
    Trash,
    Privacy,
}

impl Category {
    pub const ALL: [Category; 10] = [
        Category::System,
        Category::Browsers,
        Category::Apps,
        Category::Leftovers,
        Category::Projects,
        Category::Development,
        Category::SystemRoot,
        Category::Games,
        Category::Trash,
        Category::Privacy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::System => tr!("System and desktop", "System i pulpit"),
            Category::Browsers => tr!("Browsers", "Przeglądarki"),
            Category::Apps => tr!("Apps (Electron/Chromium)", "Aplikacje (Electron/Chromium)"),
            Category::Leftovers => tr!(
                "Leftovers of uninstalled apps",
                "Pozostałości po odinstalowanych aplikacjach"
            ),
            Category::Projects => tr!(
                "Projects — build artifacts",
                "Projekty — artefakty budowania"
            ),
            Category::Development => tr!("Development and packages", "Programowanie i pakiety"),
            Category::SystemRoot => tr!("System (needs your password)", "System (wymaga hasła)"),
            Category::Games => tr!("Games", "Gry"),
            Category::Trash => tr!("Trash", "Kosz"),
            Category::Privacy => tr!("Privacy", "Prywatność"),
        }
    }
}

/// How dangerous it is to clean; decides the default selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    /// Pure cache; rebuilt silently. Selected by default.
    Safe,
    /// Rebuilt, but with a visible cost (shader stutter, re-download).
    Low,
    /// Loses something the user may want (trash, history). Never preselected.
    Review,
}

impl Risk {
    pub fn label(self) -> &'static str {
        match self {
            Risk::Safe => tr!("safe", "bezpieczne"),
            Risk::Low => tr!("low risk", "niskie ryzyko"),
            Risk::Review => tr!("review first", "do przejrzenia"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regen {
    /// The app recreates it with the same content on demand.
    Same,
    /// Recreated, but slowly (compilation, download).
    Slow,
    /// Gone for good.
    Never,
}

impl Regen {
    pub fn label(self) -> &'static str {
        match self {
            Regen::Same => tr!("rebuilt automatically", "odtwarza się samo"),
            Regen::Slow => tr!("rebuilt slowly", "odtwarza się powoli"),
            Regen::Never => tr!("gone for good", "nie wraca"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Home,
    Cache,
    Config,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Delete everything inside the directory, keep the directory.
    ClearDir,
    /// Like ClearDir, but these top-level names are state and stay.
    ClearDirKeep(&'static [&'static str]),
    /// Delete one regular file.
    RemoveFile,
    /// Delete the directory together with everything in it.
    RemoveDir,
    /// Done by the root helper (`sudo` in a terminal), never in-process.
    Root(crate::root::RootTask),
}

#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub base: Base,
    pub rel: &'static str,
    pub action: Action,
}

const fn dir(base: Base, rel: &'static str) -> Target {
    Target {
        base,
        rel,
        action: Action::ClearDir,
    }
}

const fn file(base: Base, rel: &'static str) -> Target {
    Target {
        base,
        rel,
        action: Action::RemoveFile,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub name: T,
    pub description: T,
    pub category: Category,
    pub risk: Risk,
    pub regen: Regen,
    pub targets: &'static [Target],
    /// Process names; while one runs the rule is not cleaned.
    pub guards: &'static [&'static str],
}

use Base::*;

pub static RULES: &[Rule] = &[
    Rule {
        id: "thumbnails",
        name: T::new("File thumbnails", "Miniatury plików"),
        description: T::new("Image and video previews made by the file manager.", "Podglądy obrazów i filmów tworzone przez menedżer plików."),
        category: Category::System,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "thumbnails")],
        guards: &[],
    },
    Rule {
        id: "fontconfig",
        name: T::new("Font cache (fontconfig)", "Cache czcionek (fontconfig)"),
        description: T::new("Font index; fc-cache rebuilds it automatically.", "Indeks czcionek; fc-cache odbudowuje go automatycznie."),
        category: Category::System,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "fontconfig")],
        guards: &[],
    },
    Rule {
        id: "gstreamer",
        name: T::new("GStreamer plugin registry", "Rejestr wtyczek GStreamer"),
        description: T::new("List of media plugins; rebuilt on the next playback.", "Lista wtyczek multimediów; odbudowywana przy następnym odtwarzaniu."),
        category: Category::System,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "gstreamer-1.0")],
        guards: &[],
    },
    Rule {
        id: "mesa-shaders",
        name: T::new("Mesa / RADV shaders", "Shadery Mesa / RADV"),
        description: T::new("Compiled GPU shaders. Games may stutter for a while until they are rebuilt.", "Skompilowane shadery GPU. Po usunięciu gry mogą chwilę przycinać, zanim się odbudują."),
        category: Category::Games,
        risk: Risk::Low,
        regen: Regen::Slow,
        targets: &[
            dir(Cache, "mesa_shader_cache"),
            dir(Cache, "mesa_shader_cache_db"),
            dir(Cache, "radv_builtin_shaders"),
        ],
        // Mesa opens cache entries per lookup; unlinking under a running game
        // only means a recompile, so no process guard.
        guards: &[],
    },
    Rule {
        id: "chrome-cache",
        name: T::new("Google Chrome — cache", "Google Chrome — pamięć podręczna"),
        description: T::new("Downloaded pages and images. History, passwords and cookies stay.", "Pobrane strony i obrazy. Historia, hasła i ciasteczka zostają."),
        category: Category::Browsers,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "google-chrome")],
        guards: &["chrome", "google-chrome", "google-chrome-stable"],
    },
    Rule {
        id: "chromium-cache",
        name: T::new("Chromium — cache", "Chromium — pamięć podręczna"),
        description: T::new("Downloaded pages and images. History, passwords and cookies stay.", "Pobrane strony i obrazy. Historia, hasła i ciasteczka zostają."),
        category: Category::Browsers,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "chromium")],
        guards: &["chromium"],
    },
    Rule {
        id: "firefox-cache",
        name: T::new("Firefox — cache", "Firefox — pamięć podręczna"),
        description: T::new("Downloaded pages and images. The profile (history, passwords) lives elsewhere and stays.", "Pobrane strony i obrazy. Profil (historia, hasła) jest w innym miejscu i zostaje."),
        category: Category::Browsers,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "mozilla")],
        guards: &["firefox", "firefox-bin"],
    },
    Rule {
        id: "zen-cache",
        name: T::new("Zen Browser — cache", "Zen Browser — pamięć podręczna"),
        description: T::new("Downloaded pages and images. The profile (history, passwords) stays.", "Pobrane strony i obrazy. Profil (historia, hasła) zostaje."),
        category: Category::Browsers,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "zen")],
        guards: &["zen", "zen-bin"],
    },
    Rule {
        id: "aur-helpers",
        name: T::new("AUR builds (yay / paru / pikaur)", "Kompilacje AUR (yay / paru / pikaur)"),
        description: T::new("Downloaded sources and built AUR packages. They are downloaded again on the next update.", "Pobrane źródła i zbudowane paczki AUR. Przy następnej aktualizacji pobiorą się ponownie."),
        category: Category::Development,
        risk: Risk::Low,
        regen: Regen::Slow,
        // yay's vcs.json and paru's devel.toml track -git package commits; only
        // `yay -Y --gendb` rebuilds them, so they are state, not cache.
        targets: &[
            Target { base: Cache, rel: "yay", action: Action::ClearDirKeep(&["vcs.json"]) },
            Target { base: Cache, rel: "paru", action: Action::ClearDirKeep(&["devel.toml", "devel.json"]) },
            dir(Cache, "pikaur"),
        ],
        guards: &["yay", "paru", "pikaur", "makepkg", "pacman"],
    },
    Rule {
        id: "cargo-cache",
        name: T::new("Cargo — downloaded crates", "Cargo — pobrane paczki"),
        description: T::new(".crate archives and git clones from the registry. Cargo downloads them again when building.", "Archiwa .crate i klony git z rejestru. Cargo pobierze je ponownie przy budowaniu."),
        category: Category::Development,
        risk: Risk::Low,
        regen: Regen::Slow,
        targets: &[
            dir(Home, ".cargo/registry/cache"),
            dir(Home, ".cargo/registry/src"),
            dir(Home, ".cargo/git/checkouts"),
        ],
        guards: &["cargo", "rustc", "rust-analyzer"],
    },
    Rule {
        id: "npm-cache",
        name: T::new("npm — cache and logs", "npm — cache i logi"),
        description: T::new("Cache of downloaded packages and npm logs.", "Pamięć podręczna pobranych paczek i logi npm."),
        category: Category::Development,
        risk: Risk::Safe,
        regen: Regen::Slow,
        targets: &[dir(Home, ".npm/_cacache"), dir(Home, ".npm/_logs"), dir(Home, ".npm/_npx")],
        guards: &["npm", "npx"],
    },
    Rule {
        id: "pnpm-cache",
        name: T::new("pnpm — metadata cache", "pnpm — cache metadanych"),
        description: T::new("pnpm package metadata (not the package store).", "Metadane paczek pnpm (nie magazyn paczek)."),
        category: Category::Development,
        risk: Risk::Safe,
        regen: Regen::Slow,
        targets: &[dir(Cache, "pnpm")],
        guards: &["pnpm"],
    },
    Rule {
        id: "pip-uv-cache",
        name: T::new("pip / uv — Python cache", "pip / uv — cache Pythona"),
        description: T::new("Downloaded Python packages.", "Pobrane paczki Pythona."),
        category: Category::Development,
        risk: Risk::Safe,
        regen: Regen::Slow,
        targets: &[dir(Cache, "pip"), dir(Cache, "uv")],
        guards: &["pip", "uv"],
    },
    Rule {
        id: "go-cache",
        name: T::new("Go — build cache", "Go — cache kompilacji"),
        description: T::new("Go build results; rebuilt on the next build.", "Wyniki kompilacji Go; odbudowywane przy następnym budowaniu."),
        category: Category::Development,
        risk: Risk::Safe,
        regen: Regen::Slow,
        targets: &[dir(Cache, "go-build")],
        guards: &["go"],
    },
    Rule {
        id: "prism-cache",
        name: T::new("Prism Launcher — downloads", "Prism Launcher — pobrane pliki"),
        description: T::new("Cache of downloaded Minecraft versions, libraries and mods. Instances and worlds stay.", "Cache pobranych wersji Minecrafta, bibliotek i modów. Instancje i światy zostają."),
        category: Category::Games,
        risk: Risk::Low,
        regen: Regen::Slow,
        targets: &[dir(Data, "PrismLauncher/cache")],
        guards: &["prismlauncher"],
    },
    Rule {
        id: "bedrock-cache",
        name: T::new("Minecraft Bedrock (bedrock-on-linux) — cache", "Minecraft Bedrock (bedrock-on-linux) — cache"),
        description: T::new("Files downloaded by the Bedrock launcher. Worlds stay.", "Pobrane pliki launchera Bedrock. Światy zostają."),
        category: Category::Games,
        risk: Risk::Low,
        regen: Regen::Slow,
        targets: &[dir(Data, "bedrock-on-linux/cache")],
        guards: &["mcpelauncher-client", "mcpelauncher-ui-qt", "bedrock-on-linux"],
    },
    Rule {
        id: "trash",
        name: T::new("Trash", "Kosz"),
        description: T::new("Files moved to the trash. Once emptied they cannot be restored.", "Pliki przeniesione do kosza. Po wyczyszczeniu nie da się ich przywrócić."),
        category: Category::Trash,
        risk: Risk::Review,
        regen: Regen::Never,
        // directorysizes caches the sizes of trashed dirs (Trash spec); stale once emptied.
        targets: &[
            dir(Data, "Trash/files"),
            dir(Data, "Trash/info"),
            dir(Data, "Trash/expunged"),
            file(Data, "Trash/directorysizes"),
        ],
        guards: &[],
    },
    Rule {
        id: "recent-files",
        name: T::new("Recently opened files list", "Lista ostatnio otwieranych plików"),
        description: T::new("recently-used.xbel — history of files opened in GTK apps.", "recently-used.xbel — historia otwieranych plików w aplikacjach GTK."),
        category: Category::Privacy,
        risk: Risk::Review,
        regen: Regen::Never,
        targets: &[file(Data, "recently-used.xbel")],
        guards: &[],
    },
];

/// Cache subdirectories of a Chromium/Electron profile that hold only
/// re-downloadable or re-compilable data. Login state (Cookies, Local Storage,
/// IndexedDB, Session Storage) is deliberately absent.
pub const CHROMIUM_CACHE_DIRS: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "GPUPersistentCache",
    "DawnCache",
    "DawnGraphiteCache",
    "DawnWebGPUCache",
    "GrShaderCache",
    "ShaderCache",
    "Service Worker/CacheStorage",
    "Service Worker/ScriptCache",
    "CachedData",
    "Crashpad/completed",
    "Crashpad/pending",
    "logs",
];

/// A rule resolved against the real HOME (static or discovered).
#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: Category,
    pub risk: Risk,
    pub regen: Regen,
    /// (path relative to HOME, action)
    pub targets: Vec<(PathBuf, Action)>,
    pub guards: Vec<String>,
    /// Chromium SingletonLock to check for a live owner.
    pub singleton_lock: Option<PathBuf>,
    /// Re-rate as Review when the measured content is mostly audio/video.
    pub media_check: bool,
    /// Selected by default even though the risk is not Safe (owner decision
    /// for leftovers without personal data).
    pub preselect: bool,
}

/// How a target is shown to the user.
pub fn target_label(path: &std::path::Path, action: &Action) -> String {
    match action {
        Action::Root(t) => t.location().to_string(),
        _ => format!("~/{}", path.display()),
    }
}

impl Item {
    pub fn default_selected(&self) -> bool {
        self.risk == Risk::Safe || self.preselect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_unique_and_paths_plain() {
        let mut ids = HashSet::new();
        for r in RULES {
            assert!(ids.insert(r.id), "duplicate id {}", r.id);
            for t in r.targets {
                assert!(!t.rel.is_empty() && !t.rel.starts_with('/') && !t.rel.contains(".."));
            }
        }
    }

    #[test]
    fn no_rule_targets_a_protected_path() {
        for r in RULES {
            for t in r.targets {
                let base = match t.base {
                    Base::Home => "",
                    Base::Cache => ".cache/",
                    Base::Config => ".config/",
                    Base::Data => ".local/share/",
                };
                let p = format!("{base}{}", t.rel);
                assert_eq!(
                    crate::protect::refusal(std::path::Path::new(&p)),
                    None,
                    "{}: {p}",
                    r.id
                );
            }
        }
    }

    #[test]
    fn nothing_personal_is_preselected() {
        for r in RULES {
            if matches!(r.category, Category::Trash | Category::Privacy) {
                assert_eq!(r.risk, Risk::Review, "{}", r.id);
            }
        }
    }
}
