//! "Co zajmuje miejsce": size breakdown of one directory below HOME (like
//! ncdu). Read-only. Work is split per grandchild so one huge child
//! (e.g. ~/.local/share) still uses every CPU.

use crate::{
    fsops::{Home, Usage},
    scan::parallel_map,
};
use std::{
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    /// Relative to HOME.
    pub rel: PathBuf,
    pub usage: Usage,
    pub is_dir: bool,
    pub is_symlink: bool,
    /// Measuring failed or stopped at a mount point / foreign owner.
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Breakdown {
    pub rel: PathBuf,
    pub entries: Vec<Entry>,
    pub total: u64,
    pub elapsed: Duration,
}

fn own_blocks(p: &Path) -> u64 {
    fs::symlink_metadata(p)
        .map(|m| m.blocks() * 512)
        .unwrap_or(0)
}

/// Sizes of the entries of `home/rel`, largest first.
pub fn breakdown(home_path: &Path, rel: &Path) -> io::Result<Breakdown> {
    let start = Instant::now();
    let home = Home::open(home_path)?;
    let dir = home_path.join(rel);
    let mut names: Vec<String> = fs::read_dir(&dir)?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    // Jobs: each child, or each grandchild of a child directory.
    let mut jobs: Vec<(usize, PathBuf)> = Vec::new();
    let mut entries: Vec<Entry> = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let child_rel = rel.join(name);
        let meta = fs::symlink_metadata(home_path.join(&child_rel));
        let (is_dir, is_symlink) = meta
            .as_ref()
            .map(|m| (m.is_dir(), m.file_type().is_symlink()))
            .unwrap_or((false, false));
        let mut usage = Usage::default();
        if is_dir {
            usage.bytes = own_blocks(&home_path.join(&child_rel));
            usage.dirs = 1;
            if let Ok(rd) = fs::read_dir(home_path.join(&child_rel)) {
                for g in rd.flatten() {
                    jobs.push((i, child_rel.join(g.file_name())));
                }
            }
        } else if !is_symlink {
            jobs.push((i, child_rel.clone()));
        }
        entries.push(Entry {
            name: name.clone(),
            rel: child_rel,
            usage,
            is_dir,
            is_symlink,
            note: None,
        });
    }
    let results = parallel_map(&jobs, |(_, p)| home.measure(p));
    for ((i, _), r) in jobs.iter().zip(results) {
        match r {
            Ok(Some(u)) => entries[*i].usage.add(u),
            Ok(None) => {}
            Err(e) => {
                entries[*i]
                    .note
                    .get_or_insert_with(|| match e.raw_os_error() {
                        Some(libc::EXDEV) => "zawiera punkt montowania (pominięty)".into(),
                        Some(libc::ELOOP) => "zawiera dowiązanie (pominięte)".into(),
                        Some(libc::EACCES) | Some(libc::EPERM) => {
                            "brak dostępu do części plików".into()
                        }
                        _ => format!("błąd: {e}"),
                    });
            }
        }
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.usage.bytes));
    let total = entries.iter().map(|e| e.usage.bytes).sum();
    Ok(Breakdown {
        rel: rel.to_path_buf(),
        entries,
        total,
        elapsed: start.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakdown_sums_children() {
        let home = std::env::temp_dir().join(format!("xc-usage-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(home.join("a/b")).unwrap();
        fs::write(home.join("a/b/big"), vec![1u8; 200_000]).unwrap();
        fs::write(home.join("small"), b"x").unwrap();
        std::os::unix::fs::symlink("/etc", home.join("link")).unwrap();
        let b = breakdown(&home, Path::new("")).unwrap();
        assert_eq!(b.entries[0].name, "a");
        assert!(b.entries[0].usage.bytes >= 200_000);
        assert_eq!(b.entries[0].usage.files, 1);
        let link = b.entries.iter().find(|e| e.name == "link").unwrap();
        assert!(link.is_symlink && link.usage.bytes == 0);
        fs::remove_dir_all(&home).unwrap();
    }
}
