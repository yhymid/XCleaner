//! Built-in cleaning rules. Every rule names exact locations; nothing is
//! guessed from a name match. Paths are relative to their XDG base.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    System,
    Browsers,
    Apps,
    Leftovers,
    Development,
    Games,
    Trash,
    Privacy,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::System,
        Category::Browsers,
        Category::Apps,
        Category::Leftovers,
        Category::Development,
        Category::Games,
        Category::Trash,
        Category::Privacy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::System => "System i pulpit",
            Category::Browsers => "Przeglądarki",
            Category::Apps => "Aplikacje (Electron/Chromium)",
            Category::Leftovers => "Pozostałości po odinstalowanych aplikacjach",
            Category::Development => "Programowanie i pakiety",
            Category::Games => "Gry",
            Category::Trash => "Kosz",
            Category::Privacy => "Prywatność",
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
            Risk::Safe => "bezpieczne",
            Risk::Low => "niskie ryzyko",
            Risk::Review => "do przejrzenia",
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
            Regen::Same => "odtwarza się samo",
            Regen::Slow => "odtwarza się powoli",
            Regen::Never => "nie wraca",
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
    pub name: &'static str,
    pub description: &'static str,
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
        name: "Miniatury plików",
        description: "Podglądy obrazów i filmów tworzone przez menedżer plików.",
        category: Category::System,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "thumbnails")],
        guards: &[],
    },
    Rule {
        id: "fontconfig",
        name: "Cache czcionek (fontconfig)",
        description: "Indeks czcionek; fc-cache odbudowuje go automatycznie.",
        category: Category::System,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "fontconfig")],
        guards: &[],
    },
    Rule {
        id: "gstreamer",
        name: "Rejestr wtyczek GStreamer",
        description: "Lista wtyczek multimediów; odbudowywana przy następnym odtwarzaniu.",
        category: Category::System,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "gstreamer-1.0")],
        guards: &[],
    },
    Rule {
        id: "mesa-shaders",
        name: "Shadery Mesa / RADV",
        description: "Skompilowane shadery GPU. Po usunięciu gry mogą chwilę przycinać, zanim się odbudują.",
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
        name: "Google Chrome — pamięć podręczna",
        description: "Pobrane strony i obrazy. Historia, hasła i ciasteczka zostają.",
        category: Category::Browsers,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "google-chrome")],
        guards: &["chrome", "google-chrome", "google-chrome-stable"],
    },
    Rule {
        id: "chromium-cache",
        name: "Chromium — pamięć podręczna",
        description: "Pobrane strony i obrazy. Historia, hasła i ciasteczka zostają.",
        category: Category::Browsers,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "chromium")],
        guards: &["chromium"],
    },
    Rule {
        id: "firefox-cache",
        name: "Firefox — pamięć podręczna",
        description: "Pobrane strony i obrazy. Profil (historia, hasła) jest w innym miejscu i zostaje.",
        category: Category::Browsers,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "mozilla")],
        guards: &["firefox", "firefox-bin"],
    },
    Rule {
        id: "zen-cache",
        name: "Zen Browser — pamięć podręczna",
        description: "Pobrane strony i obrazy. Profil (historia, hasła) zostaje.",
        category: Category::Browsers,
        risk: Risk::Safe,
        regen: Regen::Same,
        targets: &[dir(Cache, "zen")],
        guards: &["zen", "zen-bin"],
    },
    Rule {
        id: "aur-helpers",
        name: "Kompilacje AUR (yay / paru / pikaur)",
        description: "Pobrane źródła i zbudowane paczki AUR. Przy następnej aktualizacji pobiorą się ponownie.",
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
        name: "Cargo — pobrane paczki",
        description: "Archiwa .crate i klony git z rejestru. Cargo pobierze je ponownie przy budowaniu.",
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
        name: "npm — cache i logi",
        description: "Pamięć podręczna pobranych paczek i logi npm.",
        category: Category::Development,
        risk: Risk::Safe,
        regen: Regen::Slow,
        targets: &[dir(Home, ".npm/_cacache"), dir(Home, ".npm/_logs"), dir(Home, ".npm/_npx")],
        guards: &["npm", "npx"],
    },
    Rule {
        id: "pnpm-cache",
        name: "pnpm — cache metadanych",
        description: "Metadane paczek pnpm (nie magazyn paczek).",
        category: Category::Development,
        risk: Risk::Safe,
        regen: Regen::Slow,
        targets: &[dir(Cache, "pnpm")],
        guards: &["pnpm"],
    },
    Rule {
        id: "pip-uv-cache",
        name: "pip / uv — cache Pythona",
        description: "Pobrane paczki Pythona.",
        category: Category::Development,
        risk: Risk::Safe,
        regen: Regen::Slow,
        targets: &[dir(Cache, "pip"), dir(Cache, "uv")],
        guards: &["pip", "uv"],
    },
    Rule {
        id: "go-cache",
        name: "Go — cache kompilacji",
        description: "Wyniki kompilacji Go; odbudowywane przy następnym budowaniu.",
        category: Category::Development,
        risk: Risk::Safe,
        regen: Regen::Slow,
        targets: &[dir(Cache, "go-build")],
        guards: &["go"],
    },
    Rule {
        id: "prism-cache",
        name: "Prism Launcher — pobrane pliki",
        description: "Cache pobranych wersji Minecrafta, bibliotek i modów. Instancje i światy zostają.",
        category: Category::Games,
        risk: Risk::Low,
        regen: Regen::Slow,
        targets: &[dir(Data, "PrismLauncher/cache")],
        guards: &["prismlauncher"],
    },
    Rule {
        id: "bedrock-cache",
        name: "Minecraft Bedrock (bedrock-on-linux) — cache",
        description: "Pobrane pliki launchera Bedrock. Światy zostają.",
        category: Category::Games,
        risk: Risk::Low,
        regen: Regen::Slow,
        targets: &[dir(Data, "bedrock-on-linux/cache")],
        guards: &["mcpelauncher-client", "mcpelauncher-ui-qt", "bedrock-on-linux"],
    },
    Rule {
        id: "trash",
        name: "Kosz",
        description: "Pliki przeniesione do kosza. Po wyczyszczeniu nie da się ich przywrócić.",
        category: Category::Trash,
        risk: Risk::Review,
        regen: Regen::Never,
        targets: &[dir(Data, "Trash/files"), dir(Data, "Trash/info"), dir(Data, "Trash/expunged")],
        guards: &[],
    },
    Rule {
        id: "recent-files",
        name: "Lista ostatnio otwieranych plików",
        description: "recently-used.xbel — historia otwieranych plików w aplikacjach GTK.",
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
