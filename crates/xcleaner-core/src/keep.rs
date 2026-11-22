//! User keep-list ("nie ruszaj tego"): item ids, one per line, in
//! `$XDG_CONFIG_HOME/xcleaner/keep.txt`. Plain text so it can be edited by hand.

use std::{collections::BTreeSet, fs, io, path::PathBuf};

#[derive(Debug, Default, Clone)]
pub struct KeepList {
    ids: BTreeSet<String>,
}

pub fn config_dir() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    Some(base.join("xcleaner"))
}

fn path() -> Option<PathBuf> {
    // Tests and power users can point elsewhere.
    if let Some(p) = std::env::var_os("XCLEANER_KEEP_FILE") {
        return Some(PathBuf::from(p));
    }
    Some(config_dir()?.join("keep.txt"))
}

impl KeepList {
    pub fn load() -> Self {
        let ids = path()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|t| Self::parse(&t))
            .unwrap_or_default();
        Self { ids }
    }

    fn parse(text: &str) -> BTreeSet<String> {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    fn save(&self) -> io::Result<()> {
        let p = path().ok_or_else(|| io::Error::other("brak HOME"))?;
        if let Some(d) = p.parent() {
            fs::create_dir_all(d)?;
        }
        let mut body = String::from(
            "# XCleaner: pozycje, których nie pokazywać ani nie czyścić (jedna na linię)\n",
        );
        for id in &self.ids {
            body.push_str(id);
            body.push('\n');
        }
        fs::write(p, body)
    }

    pub fn add(id: &str) -> io::Result<()> {
        let mut k = Self::load();
        k.ids.insert(id.to_string());
        k.save()
    }

    pub fn clear() -> io::Result<()> {
        Self::default().save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ignores_comments_and_blanks() {
        let s = KeepList::parse("# x\n\nleftover:.config/go\n  cache:gh \n");
        assert_eq!(s.len(), 2);
        assert!(s.contains("cache:gh"));
    }
}
