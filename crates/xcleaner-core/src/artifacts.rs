//! Build artifacts of software projects anywhere in HOME (kondo/npkill idea,
//! own implementation). An artifact dir counts only when the project marker
//! sits next to it (npkill-style `next_to`), plus extra proof where a name is
//! generic (`build` needs CMakeCache.txt, `obj` needs project.assets.json).
//! Everything here is rebuilt by the project's build tool.

use crate::{
    rules::{Action, Category, Item, Regen, Risk},
    scan::Layout,
};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

/// (kind, marker next to it, artifact dir name, extra file the artifact must contain)
struct Matcher {
    kind: &'static str,
    marker: &'static str,
    artifact: &'static str,
    proof: Option<&'static str>,
}

const fn m(
    kind: &'static str,
    marker: &'static str,
    artifact: &'static str,
    proof: Option<&'static str>,
) -> Matcher {
    Matcher {
        kind,
        marker,
        artifact,
        proof,
    }
}

const MATCHERS: &[Matcher] = &[
    m("Rust", "Cargo.toml", "target", None),
    m("Node", "package.json", "node_modules", None),
    m("Node", "package.json", ".turbo", None),
    m("Next.js", "package.json", ".next", None),
    m("Nuxt", "package.json", ".nuxt", None),
    m("SvelteKit", "package.json", ".svelte-kit", None),
    m("Angular", "angular.json", ".angular", None),
    m("Maven", "pom.xml", "target", None),
    m("Gradle", "build.gradle", "build", None),
    m("Gradle", "build.gradle", ".gradle", None),
    m("Gradle", "build.gradle.kts", "build", None),
    m("Gradle", "build.gradle.kts", ".gradle", None),
    m("SBT", "build.sbt", "target", None),
    m("CMake", "CMakeLists.txt", "build", Some("CMakeCache.txt")),
    m("Godot", "project.godot", ".godot", None),
    m("Zig", "build.zig", ".zig-cache", None),
    m("Zig", "build.zig", "zig-cache", None),
    m("Zig", "build.zig", "zig-out", None),
    m("Dart", "pubspec.yaml", ".dart_tool", None),
    m("Dart", "pubspec.yaml", "build", None),
    m("Elixir", "mix.exs", "_build", None),
    m("Swift", "Package.swift", ".build", None),
    m("Haskell", "stack.yaml", ".stack-work", None),
    m("Haskell", "cabal.project", "dist-newstyle", None),
    m(
        ".NET",
        "obj/project.assets.json",
        "obj",
        Some("project.assets.json"),
    ),
    m("Unity", "ProjectSettings", "Library", None),
    m("Unity", "ProjectSettings", "Temp", None),
    m("Python", "pyproject.toml", ".mypy_cache", None),
    m("Python", "pyproject.toml", ".pytest_cache", None),
    m("Python", "pyproject.toml", ".ruff_cache", None),
    m("Python", "pyproject.toml", ".tox", None),
    m("Terraform", ".terraform.lock.hcl", ".terraform", None),
];

/// Directories never descended into (still matched as artifacts above).
const NO_DESCEND: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    ".cache",
    ".local",
    ".config",
    ".var",
    ".steam",
    ".wine",
    ".cargo",
    ".rustup",
    ".npm",
    ".nvm",
    ".pyenv",
    ".venv",
    "venv",
    "node_modules",
    "__pycache__",
    "steamapps",
    "compatdata",
    "Steam",
];

const MAX_DEPTH: usize = 7;
/// Untouched this long → preselected.
pub const STALE_DAYS: u64 = 30;

#[derive(Debug, Clone)]
pub struct Project {
    pub dir: PathBuf,
    pub kinds: Vec<&'static str>,
    pub artifacts: Vec<PathBuf>,
    /// Newest mtime among the marker and the artifact dirs.
    pub last_used: Option<SystemTime>,
}

fn has_marker(dir: &Path, marker: &str) -> bool {
    fs::symlink_metadata(dir.join(marker)).is_ok()
}

fn is_real_dir(p: &Path) -> Option<fs::Metadata> {
    fs::symlink_metadata(p).ok().filter(|m| m.is_dir())
}

/// Artifact dirs of one directory, if it is a project.
fn project_at(dir: &Path) -> Option<Project> {
    let mut p = Project {
        dir: dir.to_path_buf(),
        kinds: Vec::new(),
        artifacts: Vec::new(),
        last_used: None,
    };
    for mt in MATCHERS {
        let art = dir.join(mt.artifact);
        if p.artifacts.contains(&art) || !has_marker(dir, mt.marker) {
            continue;
        }
        let Some(meta) = is_real_dir(&art) else {
            continue;
        };
        if let Some(proof) = mt.proof {
            if !has_marker(&art, proof) {
                continue;
            }
        }
        if !p.kinds.contains(&mt.kind) {
            p.kinds.push(mt.kind);
        }
        p.artifacts.push(art);
        let t = meta.modified().ok();
        let mk = fs::symlink_metadata(dir.join(mt.marker))
            .ok()
            .and_then(|m| m.modified().ok());
        p.last_used = p.last_used.max(t).max(mk);
    }
    (!p.artifacts.is_empty()).then_some(p)
}

/// Walk HOME (bounded, no hidden dirs, no symlinks, same filesystem).
pub fn find_projects(home: &Path) -> Vec<Project> {
    let Ok(root) = fs::symlink_metadata(home) else {
        return Vec::new();
    };
    let dev = root.dev();
    let mut out = Vec::new();
    let mut stack = vec![(home.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let project = project_at(&dir);
        let skip: Vec<PathBuf> = project
            .as_ref()
            .map(|p| p.artifacts.clone())
            .unwrap_or_default();
        if let Some(p) = project {
            out.push(p);
        }
        if depth >= MAX_DEPTH {
            continue;
        }
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name();
            let n = name.to_string_lossy();
            if n.starts_with('.') || NO_DESCEND.contains(&n.as_ref()) {
                continue;
            }
            let path = e.path();
            if skip.contains(&path) {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            // DirEntry::metadata does not follow symlinks.
            if meta.is_dir() && meta.dev() == dev {
                stack.push((path, depth + 1));
            }
        }
    }
    out.sort_by(|a, b| a.dir.cmp(&b.dir));
    out
}

pub fn artifact_items(layout: &Layout) -> Vec<Item> {
    let now = SystemTime::now();
    find_projects(&layout.home)
        .into_iter()
        .filter_map(|p| {
            let rel_dir = p.dir.strip_prefix(&layout.home).ok()?.to_path_buf();
            let name = p.dir.file_name()?.to_string_lossy().to_string();
            let age_days = p
                .last_used
                .and_then(|t| now.duration_since(t).ok())
                .unwrap_or(Duration::ZERO)
                .as_secs()
                / 86_400;
            let stale = age_days >= STALE_DAYS;
            let targets: Vec<(PathBuf, Action)> = p
                .artifacts
                .iter()
                .filter_map(|a| Some((a.strip_prefix(&layout.home).ok()?.to_path_buf(), Action::RemoveDir)))
                .collect();
            let arts: Vec<String> = p
                .artifacts
                .iter()
                .filter_map(|a| a.file_name().map(|n| n.to_string_lossy().to_string()))
                .collect();
            Some(Item {
                id: format!("artifacts:{}", rel_dir.display()),
                name: format!("{name} — artefakty budowania ({})", p.kinds.join(", ")),
                description: format!(
                    "Projekt ~/{} · {} · ostatnio budowany {}. Odtworzy się przy następnym budowaniu; kod źródłowy zostaje.",
                    rel_dir.display(),
                    arts.join(", "),
                    match age_days {
                        0 => "dziś".to_string(),
                        1 => "wczoraj".to_string(),
                        d => format!("{d} dni temu"),
                    }
                ),
                category: Category::Projects,
                risk: if stale { Risk::Safe } else { Risk::Low },
                regen: Regen::Slow,
                targets,
                guards: Vec::new(),
                singleton_lock: None,
                media_check: false,
                preselect: false,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_only_proven_artifacts() {
        let home = std::env::temp_dir().join(format!("xc-art-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        // Rust project.
        fs::create_dir_all(home.join("Projects/app/target/debug")).unwrap();
        fs::write(home.join("Projects/app/Cargo.toml"), b"[package]").unwrap();
        // node_modules without package.json: not a project.
        fs::create_dir_all(home.join("Projects/loose/node_modules")).unwrap();
        // CMake build without CMakeCache.txt: user files, not an artifact.
        fs::create_dir_all(home.join("Projects/cm/build")).unwrap();
        fs::write(home.join("Projects/cm/CMakeLists.txt"), b"").unwrap();
        // Hidden dirs are never searched.
        fs::create_dir_all(home.join(".hidden/p/target")).unwrap();
        fs::write(home.join(".hidden/p/Cargo.toml"), b"").unwrap();
        // Nested project inside node_modules is not descended.
        fs::create_dir_all(home.join("Projects/web/node_modules/dep/node_modules")).unwrap();
        fs::write(home.join("Projects/web/package.json"), b"{}").unwrap();
        fs::write(
            home.join("Projects/web/node_modules/dep/package.json"),
            b"{}",
        )
        .unwrap();

        let found = find_projects(&home);
        let dirs: Vec<_> = found
            .iter()
            .map(|p| p.dir.strip_prefix(&home).unwrap().to_path_buf())
            .collect();
        assert_eq!(
            dirs,
            vec![PathBuf::from("Projects/app"), PathBuf::from("Projects/web")]
        );
        assert_eq!(found[0].artifacts, vec![home.join("Projects/app/target")]);
        fs::remove_dir_all(&home).unwrap();
    }
}
