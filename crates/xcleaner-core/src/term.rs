//! Running a command in a new terminal window and knowing when it ends.
//!
//! Terminals disagree on argv conventions and many return at once because an
//! existing instance opens the window (docs/research/DISTRO-DESKTOPS-
//! TERMINALS-2026-11.md §4). So the terminal runs our own wrapper
//! (`--term-wrap`): it writes its PID to a status file, runs the command as
//! a child and appends the exit code. The GUI only watches that file and the
//! PID, whatever the terminal does.

use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

pub const WRAP_ARG: &str = "--term-wrap";

/// Generic fallbacks, after `$TERMINAL` and the desktop's own terminal.
const GENERIC: &[&str] = &[
    "kitty",
    "foot",
    "alacritty",
    "ghostty",
    "wezterm",
    "konsole",
    "ptyxis",
    "gnome-terminal",
    "kgx",
    "xfce4-terminal",
    "mate-terminal",
    "tilix",
    "terminator",
    "lxterminal",
    "qterminal",
    "cosmic-term",
    "x-terminal-emulator",
    "xterm",
];

/// Default terminal of a desktop (`XDG_CURRENT_DESKTOP` entries).
fn desktop_terminals(desktop: &str) -> &'static [&'static str] {
    match desktop.to_ascii_uppercase().as_str() {
        "KDE" => &["konsole"],
        "GNOME" | "UBUNTU" => &["ptyxis", "kgx", "gnome-terminal"],
        "X-CINNAMON" | "CINNAMON" => &["gnome-terminal"],
        "XFCE" => &["xfce4-terminal"],
        "MATE" => &["mate-terminal"],
        "LXQT" => &["qterminal"],
        "LXDE" => &["lxterminal"],
        "BUDGIE" | "BUDGIE:GNOME" => &["tilix", "gnome-terminal"],
        "COSMIC" => &["cosmic-term"],
        _ => &[],
    }
}

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// xdg-terminal-exec only picks something defined when configured.
fn xdg_terminal_exec_configured() -> bool {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(h) = std::env::var_os("XDG_CONFIG_HOME") {
        dirs.push(PathBuf::from(h));
    } else if let Some(h) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(h).join(".config"));
    }
    dirs.push(PathBuf::from("/etc/xdg"));
    dirs.iter().any(|d| {
        fs::read_dir(d)
            .map(|rd| {
                rd.flatten().any(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .ends_with("xdg-terminals.list")
                })
            })
            .unwrap_or(false)
    })
}

/// Candidate names in order: `$TERMINAL`, the desktop's terminal,
/// a configured xdg-terminal-exec, generic fallbacks.
fn candidates() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if let Some(t) = std::env::var("TERMINAL")
        .ok()
        .and_then(|t| t.split_whitespace().next().map(str::to_owned))
    {
        v.push(t);
    }
    for d in std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
    {
        v.extend(desktop_terminals(d).iter().map(|s| s.to_string()));
    }
    if xdg_terminal_exec_configured() {
        v.push("xdg-terminal-exec".into());
    }
    v.extend(GENERIC.iter().map(|s| s.to_string()));
    let mut seen = std::collections::HashSet::new();
    v.retain(|n| seen.insert(n.clone()));
    v
}

pub fn find_terminal() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    candidates().into_iter().find_map(|name| {
        let c = Path::new(&name);
        if c.is_absolute() {
            return is_executable(c).then(|| c.to_path_buf());
        }
        std::env::split_paths(&path)
            .map(|d| d.join(&name))
            .find(|p| is_executable(p))
    })
}

/// POSIX shell quoting, for terminals that re-split one command string.
fn shell_quote(a: &std::ffi::OsStr) -> String {
    let s = a.to_string_lossy();
    if !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-=+:,@%".contains(&b))
    {
        return s.into_owned();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// argv for `terminal` so it opens a new window titled `title` running
/// `cmd` (cmd[0] is the program). Flags that make the terminal wait for the
/// command come from each terminal's source; the wrapper does not rely on
/// them, they only avoid reusing a server instance.
pub fn terminal_args(terminal: &Path, title: &str, cmd: &[OsString]) -> Vec<OsString> {
    let name = terminal
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let t = || OsString::from(title);
    let joined = || {
        OsString::from(
            cmd.iter()
                .map(|a| shell_quote(a))
                .collect::<Vec<_>>()
                .join(" "),
        )
    };
    let rest = |mut v: Vec<OsString>| {
        v.extend(cmd.iter().cloned());
        v
    };
    match name.as_str() {
        "kitty" | "foot" => rest(vec!["--title".into(), t()]),
        "alacritty" => rest(vec!["--title".into(), t(), "-e".into()]),
        "ghostty" => rest(vec![format!("--title={title}").into(), "-e".into()]),
        "wezterm" => rest(vec![
            "start".into(),
            "--always-new-process".into(),
            "--".into(),
        ]),
        "konsole" => rest(vec![
            "--separate".into(),
            "-p".into(),
            format!("tabtitle={title}").into(),
            "-e".into(),
        ]),
        "gnome-terminal" => rest(vec!["--wait".into(), "--title".into(), t(), "--".into()]),
        "kgx" | "ptyxis" => rest(vec!["-T".into(), t(), "--".into()]),
        "xfce4-terminal" => rest(vec![
            "--disable-server".into(),
            "-T".into(),
            t(),
            "-x".into(),
        ]),
        "mate-terminal" => rest(vec![
            "--disable-factory".into(),
            "-t".into(),
            t(),
            "-x".into(),
        ]),
        "terminator" => rest(vec!["-u".into(), "-T".into(), t(), "-x".into()]),
        "lxterminal" => rest(vec!["--no-remote".into(), "-t".into(), t(), "-e".into()]),
        // These two re-split the command into words: pass one quoted string.
        "tilix" => vec![
            "--new-process".into(),
            "-t".into(),
            t(),
            "-e".into(),
            joined(),
        ],
        "qterminal" => vec!["-e".into(), joined()],
        "cosmic-term" => rest(vec!["-e".into()]),
        "xdg-terminal-exec" => rest(vec![format!("--title={title}").into()]),
        _ => rest(vec!["-T".into(), t(), "-e".into()]),
    }
}

// ------------------------------------------------------------ status file

/// What the wrapper reported so far.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct WrapStatus {
    pub pid: Option<u32>,
    pub exit: Option<i32>,
}

pub fn parse_status(text: &str) -> WrapStatus {
    let mut s = WrapStatus::default();
    for l in text.lines() {
        match l.split_once('\t') {
            Some(("wrapper", p)) => s.pid = p.trim().parse().ok(),
            Some(("exit", c)) => s.exit = c.trim().parse().ok(),
            _ => {}
        }
    }
    s
}

/// Create an empty private file the wrapper and root helper will write to.
pub fn create_private(path: &Path) -> io::Result<()> {
    let _ = fs::remove_file(path);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map(drop)
}

/// Open the status file for appending: our own regular file, no symlink.
fn open_status(path: &Path) -> io::Result<fs::File> {
    let f = fs::OpenOptions::new()
        .append(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let m = f.metadata()?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.nlink() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "niezaufany plik statusu",
        ));
    }
    Ok(f)
}

/// Entry point of `--term-wrap <status> -- <program> <args…>`.
pub fn run_wrapper(args: &[String]) -> i32 {
    let (Some(status), Some("--")) = (args.first(), args.get(1).map(String::as_str)) else {
        eprintln!("użycie: {WRAP_ARG} <plik> -- <program> …");
        return 2;
    };
    let Some(program) = args.get(2) else {
        eprintln!("brak programu");
        return 2;
    };
    let mut f = match open_status(Path::new(status)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("status: {e}");
            return 2;
        }
    };
    let _ = writeln!(f, "wrapper\t{}", std::process::id());
    let code = match std::process::Command::new(program)
        .args(&args[3..])
        .status()
    {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("{program}: {e}");
            127
        }
    };
    let _ = writeln!(f, "exit\t{code}");
    if code != 0 {
        // Keep the window open so the message can be read.
        eprintln!("\nZakończono z błędem (kod {code}). Naciśnij Enter, aby zamknąć okno.");
        let mut s = String::new();
        let _ = io::stdin().read_line(&mut s);
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_per_terminal() {
        let cmd: Vec<OsString> = vec!["/opt/My Apps/x.AppImage".into(), "--term-wrap".into()];
        let a = |t: &str| terminal_args(Path::new(t), "T", &cmd);
        assert_eq!(
            a("/usr/bin/gnome-terminal")[..4],
            ["--wait", "--title", "T", "--"]
        );
        assert_eq!(a("konsole")[0], "--separate");
        assert_eq!(a("xfce4-terminal")[0], "--disable-server");
        assert_eq!(a("wezterm")[1], "--always-new-process");
        // Separate argv: the path with a space stays one argument.
        assert_eq!(a("kitty")[2], "/opt/My Apps/x.AppImage");
        // Re-splitting terminals get one shell-quoted string.
        assert_eq!(
            a("tilix").last().unwrap(),
            "'/opt/My Apps/x.AppImage' --term-wrap"
        );
        assert_eq!(
            a("qterminal"),
            vec![
                OsString::from("-e"),
                "'/opt/My Apps/x.AppImage' --term-wrap".into()
            ]
        );
        assert_eq!(shell_quote("it's".as_ref()), "'it'\\''s'");
    }

    #[test]
    fn desktop_comes_before_generic() {
        assert_eq!(desktop_terminals("KDE"), &["konsole"]);
        assert_eq!(desktop_terminals("gnome")[0], "ptyxis");
        assert!(desktop_terminals("Hyprland").is_empty());
    }

    #[test]
    fn status_parsing() {
        assert_eq!(parse_status(""), WrapStatus::default());
        assert_eq!(
            parse_status("wrapper\t42\nexit\t1\n"),
            WrapStatus {
                pid: Some(42),
                exit: Some(1)
            }
        );
    }

    #[test]
    fn wrapper_records_pid_and_exit_code() {
        let p = std::env::temp_dir().join(format!("xc-wrap-{}", std::process::id()));
        create_private(&p).unwrap();
        let s = p.to_string_lossy().to_string();
        let code = run_wrapper(&[s.clone(), "--".into(), "true".into()]);
        assert_eq!(code, 0);
        let st = parse_status(&fs::read_to_string(&p).unwrap());
        assert_eq!(st.pid, Some(std::process::id()));
        assert_eq!(st.exit, Some(0));
        // A symlink is refused.
        let l = p.with_extension("lnk");
        let _ = fs::remove_file(&l);
        std::os::unix::fs::symlink(&p, &l).unwrap();
        assert_eq!(
            run_wrapper(&[l.to_string_lossy().into(), "--".into(), "true".into()]),
            2
        );
        fs::remove_file(&l).unwrap();
        fs::remove_file(&p).unwrap();
    }
}
