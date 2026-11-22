//! XCleaner core: rules, scanning and safe deletion. No GTK here.
#![cfg(target_os = "linux")]

pub mod artifacts;
pub mod extra;
pub mod fsops;
pub mod history;
pub mod keep;
pub mod leftovers;
pub mod procs;
pub mod protect;
pub mod rules;
pub mod scan;

/// True when running with effective UID 0.
pub fn is_root() -> bool {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() == 0 }
}

/// Human-readable size with binary units (e.g. "1,5 GiB" in Polish style).
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{:.1} {}", v, UNITS[u]).replace('.', ",")
    }
}

/// Polish plural: pl(n, "plik", "pliki", "plików").
pub fn pl(n: u64, one: &str, few: &str, many: &str) -> String {
    let w = if n == 1 {
        one
    } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
        few
    } else {
        many
    };
    format!("{n} {w}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_plurals() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1,5 KiB");
        assert_eq!(pl(1, "plik", "pliki", "plików"), "1 plik");
        assert_eq!(pl(3, "plik", "pliki", "plików"), "3 pliki");
        assert_eq!(pl(12, "plik", "pliki", "plików"), "12 plików");
        assert_eq!(pl(22, "plik", "pliki", "plików"), "22 pliki");
    }
}
