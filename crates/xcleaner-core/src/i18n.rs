//! Interface language: English by default, Polish on request.
//!
//! Every user-facing text sits next to its translation in the code:
//! `tr!("en", "pl")` gives a `&'static str`, `trf!("en {x}", "pl {x}")`
//! formats (the compiler checks both strings), and [`T`] holds a pair in
//! static tables. The choice is stored in
//! `$XDG_CONFIG_HOME/xcleaner/settings.txt` as `lang=en` or `lang=pl`.

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Pl,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Pl];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Pl => "pl",
        }
    }

    /// Short label for the language switch.
    pub fn short(self) -> &'static str {
        match self {
            Lang::En => "EN",
            Lang::Pl => "PL",
        }
    }

    pub fn parse(s: &str) -> Option<Lang> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en" | "english" => Some(Lang::En),
            "pl" | "polski" | "polish" => Some(Lang::Pl),
            _ => None,
        }
    }
}

// Global: the scan runs on worker threads. Tests run in parallel threads,
// so each test thread has its own setting there.
#[cfg(not(test))]
static POLISH: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
thread_local! {
    static POLISH: AtomicBool = const { AtomicBool::new(false) };
}

pub fn set(lang: Lang) {
    #[cfg(not(test))]
    POLISH.store(lang == Lang::Pl, Ordering::Relaxed);
    #[cfg(test)]
    POLISH.with(|p| p.store(lang == Lang::Pl, Ordering::Relaxed));
}

pub fn current() -> Lang {
    if is_pl() {
        Lang::Pl
    } else {
        Lang::En
    }
}

pub fn is_pl() -> bool {
    #[cfg(not(test))]
    return POLISH.load(Ordering::Relaxed);
    #[cfg(test)]
    return POLISH.with(|p| p.load(Ordering::Relaxed));
}

/// A text in both languages, for static tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct T {
    pub en: &'static str,
    pub pl: &'static str,
}

impl T {
    pub const fn new(en: &'static str, pl: &'static str) -> T {
        T { en, pl }
    }

    pub fn get(self) -> &'static str {
        if is_pl() {
            self.pl
        } else {
            self.en
        }
    }
}

/// `tr!("English", "Polski")` → `&'static str` in the current language.
#[macro_export]
macro_rules! tr {
    ($en:literal, $pl:literal $(,)?) => {
        if $crate::i18n::is_pl() {
            $pl
        } else {
            $en
        }
    };
}

/// `trf!("Freed {x}", "Zwolniono {x}", ...)` → `String`, like `format!`.
#[macro_export]
macro_rules! trf {
    ($en:literal, $pl:literal $(, $arg:expr)* $(,)?) => {
        if $crate::i18n::is_pl() {
            format!($pl $(, $arg)*)
        } else {
            format!($en $(, $arg)*)
        }
    };
}

/// `n` with the right noun form: English one/many, Polish one/few/many
/// (`count(3, ["file", "files"], ["plik", "pliki", "plików"])`).
pub fn count(n: u64, en: [&str; 2], pl: [&str; 3]) -> String {
    let w = if is_pl() {
        if n == 1 {
            pl[0]
        } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
            pl[1]
        } else {
            pl[2]
        }
    } else if n == 1 {
        en[0]
    } else {
        en[1]
    };
    format!("{n} {w}")
}

fn settings_path() -> Option<std::path::PathBuf> {
    Some(crate::keep::config_dir()?.join("settings.txt"))
}

/// Language handed to helper processes (the terminal wrapper).
pub const ENV: &str = "XCLEANER_LANG";

/// Startup choice: `$XCLEANER_LANG`, else the saved setting, else English.
pub fn from_env_or_saved() -> Lang {
    std::env::var(ENV)
        .ok()
        .and_then(|v| Lang::parse(&v))
        .unwrap_or_else(load_saved)
}

/// The saved language (English when nothing is saved).
pub fn load_saved() -> Lang {
    settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| {
            t.lines()
                .filter_map(|l| l.trim().strip_prefix("lang="))
                .find_map(Lang::parse)
        })
        .unwrap_or_default()
}

/// Remember the language for the next start.
pub fn save(lang: Lang) -> std::io::Result<()> {
    let p = settings_path().ok_or_else(|| std::io::Error::other("no config directory"))?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = p.with_extension("txt.tmp");
    std::fs::write(&tmp, format!("lang={}\n", lang.code()))?;
    std::fs::rename(&tmp, &p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_forms_and_parse() {
        set(Lang::Pl);
        assert_eq!(
            count(1, ["file", "files"], ["plik", "pliki", "plików"]),
            "1 plik"
        );
        assert_eq!(
            count(3, ["file", "files"], ["plik", "pliki", "plików"]),
            "3 pliki"
        );
        assert_eq!(
            count(12, ["file", "files"], ["plik", "pliki", "plików"]),
            "12 plików"
        );
        assert_eq!(
            count(22, ["file", "files"], ["plik", "pliki", "plików"]),
            "22 pliki"
        );
        let n = 5;
        assert_eq!(trf!("{n} items", "{n} pozycji"), "5 pozycji");
        assert_eq!(tr!("Clean", "Wyczyść"), "Wyczyść");
        assert_eq!(T::new("Trash", "Kosz").get(), "Kosz");
        set(Lang::En);
        assert_eq!(
            count(1, ["file", "files"], ["plik", "pliki", "plików"]),
            "1 file"
        );
        assert_eq!(
            count(0, ["file", "files"], ["plik", "pliki", "plików"]),
            "0 files"
        );
        assert_eq!(trf!("{n} items", "{n} pozycji"), "5 items");
        assert_eq!(crate::format_size(1536), "1.5 KiB");
        assert_eq!(Lang::parse("PL"), Some(Lang::Pl));
        assert_eq!(Lang::parse("de"), None);
        assert_eq!(Lang::default(), Lang::En);
    }
}
