//! Old kernels, only through the package manager and never preselected
//! (docs/research/DISTRO-ROOT-CLEANING-2026-11.md §2). Files in /boot are
//! never deleted by hand: hooks regenerate initramfs and bootloader entries.
//! The running kernel is always kept. Verified in VMs only (v0.3.0).

use std::{collections::BTreeSet, path::Path, process::Stdio};

const APT_GET: &str = "/usr/bin/apt-get";
const DPKG_QUERY: &str = "/usr/bin/dpkg-query";
const DNF5: &str = "/usr/bin/dnf5";
const DNF: &str = "/usr/bin/dnf";
const ZYPPER: &str = "/usr/bin/zypper";
const RPM: &str = "/usr/bin/rpm";

fn running_release() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// `6.1.0-27-amd64` → `6.1.0-27`: the part every package of one kernel shares.
fn release_core(release: &str) -> &str {
    let mut end = 0;
    let mut dashes = 0;
    for (i, c) in release.char_indices() {
        if c == '-' {
            dashes += 1;
            if dashes == 2 {
                break;
            }
        }
        end = i + c.len_utf8();
    }
    &release[..end]
}

/// A versioned Debian kernel package (`linux-image-6.1.0-26-amd64`), not a
/// meta package (`linux-image-amd64`, `linux-generic`), and not one of the
/// running kernel.
pub fn is_old_kernel_package(name: &str, running: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "linux-image-unsigned-",
        "linux-image-",
        "linux-headers-",
        "linux-modules-extra-",
        "linux-modules-",
    ];
    let Some(rest) = PREFIXES.iter().find_map(|p| name.strip_prefix(p)) else {
        return false;
    };
    let versioned = rest.chars().next().is_some_and(|c| c.is_ascii_digit());
    let core = release_core(running);
    versioned && !core.is_empty() && !rest.starts_with(core)
}

/// Packages an apt simulation would remove (`Remv x [..]` / `Purg x [..]`).
pub fn parse_apt_removals(out: &str) -> BTreeSet<String> {
    out.lines()
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            matches!(w.next()?, "Remv" | "Purg").then_some(())?;
            Some(w.next()?.to_string())
        })
        .collect()
}

fn output(bin: &str, args: &[&str]) -> Option<String> {
    if !Path::new(bin).exists() {
        return None;
    }
    let o = crate::host_command(bin)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Old kernel packages apt itself considers removable (it protects the
/// running and the newest kernel), filtered to versioned kernel packages.
fn apt_old_kernels() -> BTreeSet<String> {
    let running = running_release();
    output(APT_GET, &["-s", "autoremove", "--purge"])
        .map(|o| {
            parse_apt_removals(&o)
                .into_iter()
                .filter(|p| is_old_kernel_package(p, &running))
                .collect()
        })
        .unwrap_or_default()
}

fn dpkg_installed_kib(pkgs: &BTreeSet<String>) -> u64 {
    let mut args = vec!["-W", "-f=${Installed-Size}\\n"];
    args.extend(pkgs.iter().map(String::as_str));
    output(DPKG_QUERY, &args)
        .map(|o| o.lines().filter_map(|l| l.trim().parse::<u64>().ok()).sum())
        .unwrap_or(0)
}

/// Installed versions of an rpm kernel package.
fn rpm_versions(pkg: &str) -> Vec<String> {
    output(
        RPM,
        &["-q", pkg, "--qf", "%{VERSION}-%{RELEASE}.%{ARCH}\\n"],
    )
    .map(|o| {
        o.lines()
            .map(str::to_string)
            .filter(|l| !l.contains(' '))
            .collect()
    })
    .unwrap_or_default()
}

/// `installonly_limit` from dnf.conf (default 3).
fn dnf_limit() -> usize {
    let conf = std::fs::read_to_string("/etc/dnf/dnf.conf").unwrap_or_default();
    // The last setting wins, as in dnf.
    conf.lines()
        .rev()
        .filter_map(|l| l.trim().strip_prefix("installonly_limit"))
        .find_map(|r| r.trim_start().strip_prefix('=')?.trim().parse().ok())
        .unwrap_or(3)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Apt,
    Dnf,
    Zypper,
}

fn backend() -> Option<Backend> {
    if Path::new(APT_GET).exists() && Path::new(DPKG_QUERY).exists() {
        Some(Backend::Apt)
    } else if Path::new(DNF5).exists() || Path::new(DNF).exists() {
        Some(Backend::Dnf)
    } else if Path::new(ZYPPER).exists() {
        Some(Backend::Zypper)
    } else {
        None
    }
}

pub fn probe() -> Option<(u64, u64, String)> {
    match backend()? {
        Backend::Apt => {
            let pkgs = apt_old_kernels();
            (!pkgs.is_empty()).then(|| {
                let names: Vec<&str> = pkgs.iter().map(String::as_str).collect();
                (
                    dpkg_installed_kib(&pkgs) * 1024,
                    pkgs.len() as u64,
                    names.join(", "),
                )
            })
        }
        Backend::Dnf => {
            let v = rpm_versions("kernel-core");
            let limit = dnf_limit();
            (v.len() > limit).then(|| {
                (
                    0,
                    (v.len() - limit) as u64,
                    format!(
                        "{} kerneli przy limicie {limit} (dnf remove --oldinstallonly)",
                        v.len()
                    ),
                )
            })
        }
        Backend::Zypper => {
            // Vendor default multiversion.kernels = latest,latest-1,running.
            let v = rpm_versions("kernel-default");
            (v.len() > 3).then(|| {
                (
                    0,
                    (v.len() - 3) as u64,
                    format!("{} kerneli (zypper purge-kernels)", v.len()),
                )
            })
        }
    }
}

fn run(bin: &str, args: &[&str]) -> Result<(), String> {
    match crate::host_command(bin)
        .args(args)
        .stdin(Stdio::null())
        .status()
    {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{bin}: kod {}", s.code().unwrap_or(-1))),
        Err(e) => Err(format!("{bin}: {e}")),
    }
}

pub fn root_clean() -> (u64, Vec<String>) {
    crate::root::say("• Stare kernele (menedżer pakietów)…");
    if let Some(pm) = crate::locks::busy_package_manager() {
        return (
            0,
            vec![format!("działa menedżer pakietów: {pm} — pominięto")],
        );
    }
    match backend() {
        Some(Backend::Apt) => {
            let pkgs = apt_old_kernels();
            if pkgs.is_empty() {
                return (0, Vec::new());
            }
            let list: Vec<&str> = pkgs.iter().map(String::as_str).collect();
            // The plan must remove exactly these packages and nothing else.
            let mut sim = vec!["-s", "purge"];
            sim.extend(&list);
            let plan = output(APT_GET, &sim).map(|o| parse_apt_removals(&o));
            if plan.as_ref() != Some(&pkgs) {
                return (
                    0,
                    vec![format!(
                        "apt chce usunąć więcej niż stare kernele ({}) — przerwano",
                        plan.map(|p| p.into_iter().collect::<Vec<_>>().join(", "))
                            .unwrap_or_default()
                    )],
                );
            }
            let bytes = dpkg_installed_kib(&pkgs) * 1024;
            crate::root::say(&format!("  apt-get -y purge {}", list.join(" ")));
            let mut args = vec!["-y", "purge"];
            args.extend(&list);
            match run(APT_GET, &args) {
                Ok(()) => (bytes, Vec::new()),
                Err(e) => (0, vec![e]),
            }
        }
        Some(Backend::Dnf) => {
            let bin = if Path::new(DNF5).exists() { DNF5 } else { DNF };
            // dnf keeps the running kernel; dependencies stay installed.
            let r = run(
                bin,
                &[
                    "-y",
                    "remove",
                    "--oldinstallonly",
                    "--setopt=clean_requirements_on_remove=False",
                ],
            );
            (0, r.err().into_iter().collect())
        }
        Some(Backend::Zypper) => (
            0,
            run(ZYPPER, &["--non-interactive", "purge-kernels"])
                .err()
                .into_iter()
                .collect(),
        ),
        None => (0, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debian_kernel_package_filter() {
        let r = "6.1.0-27-amd64";
        assert_eq!(release_core(r), "6.1.0-27");
        assert!(is_old_kernel_package("linux-image-6.1.0-26-amd64", r));
        assert!(is_old_kernel_package("linux-headers-6.1.0-26-common", r));
        assert!(is_old_kernel_package(
            "linux-modules-extra-6.8.0-45-generic",
            "6.8.0-47-generic"
        ));
        assert!(
            !is_old_kernel_package("linux-image-6.1.0-27-amd64", r),
            "running"
        );
        assert!(
            !is_old_kernel_package("linux-headers-6.1.0-27-common", r),
            "running"
        );
        assert!(!is_old_kernel_package("linux-image-amd64", r), "meta");
        assert!(!is_old_kernel_package("linux-generic", r), "meta");
        assert!(!is_old_kernel_package("linux-firmware", r));
        assert!(!is_old_kernel_package("libreoffice", r));
    }

    #[test]
    fn apt_simulation_removals() {
        let out = "NOTE: simulation\nPurg linux-image-6.1.0-26-amd64 [6.1.112-1]\nRemv libfoo [1.0]\nInst x\n";
        let s = parse_apt_removals(out);
        assert_eq!(s.len(), 2);
        assert!(s.contains("linux-image-6.1.0-26-amd64") && s.contains("libfoo"));
    }
}
