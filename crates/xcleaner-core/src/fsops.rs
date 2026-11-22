//! Descriptor-based measuring and deletion.
//!
//! Every target is opened with `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS |
//! RESOLVE_NO_XDEV)` relative to the HOME descriptor, so a symlink or a mount
//! point anywhere in the path refuses the target instead of escaping HOME.
//! Below the target, entries are visited by descriptor (`fstatat` +
//! `openat(O_NOFOLLOW)`), never by path string: symlinks are removed as links
//! and never followed, other filesystems and foreign-owned entries are skipped.
#![allow(unsafe_code)]

use std::{
    collections::HashSet,
    ffi::{CStr, CString, OsStr},
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    path::{Component, Path},
};

const RESOLVE_NO_XDEV: u64 = 0x01;
const RESOLVE_NO_SYMLINKS: u64 = 0x04;
const RESOLVE_BENEATH: u64 = 0x08;
/// Deeper trees are left alone (reported as skipped) instead of risking the stack.
const MAX_DEPTH: usize = 256;

#[repr(C)]
struct OpenHow {
    flags: u64,
    mode: u64,
    resolve: u64,
}

/// Size of a measured or deleted tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// Allocated bytes (`st_blocks * 512`), i.e. what deletion frees.
    pub bytes: u64,
    pub files: u64,
    pub dirs: u64,
    /// Bytes in audio/video files (music in a "cache" is user data).
    pub media_bytes: u64,
    /// Bytes of files with more than one hard link: deleting this tree may
    /// not free them (other links can live elsewhere, e.g. pnpm store).
    pub shared_bytes: u64,
}

impl Usage {
    pub fn add(&mut self, other: Usage) {
        self.bytes += other.bytes;
        self.files += other.files;
        self.dirs += other.dirs;
        self.media_bytes += other.media_bytes;
        self.shared_bytes += other.shared_bytes;
    }

    /// More than half of the space is audio/video.
    pub fn mostly_media(&self) -> bool {
        self.bytes > 0 && self.media_bytes * 2 > self.bytes
    }
}

const MEDIA_EXT: &[&str] = &[
    "mp3", "opus", "ogg", "oga", "flac", "m4a", "aac", "wav", "wma", "mp4", "mkv", "webm", "avi",
    "mov", "m4v",
];

fn is_media(name: &[u8]) -> bool {
    let Some(dot) = name.iter().rposition(|b| *b == b'.') else {
        return false;
    };
    let ext = &name[dot + 1..];
    MEDIA_EXT
        .iter()
        .any(|m| m.as_bytes().eq_ignore_ascii_case(ext))
}

#[derive(Debug, Clone, Default)]
pub struct DeleteReport {
    pub freed: Usage,
    /// Entries left in place: foreign owner, other filesystem, too deep, busy.
    pub skipped: u64,
    pub errors: Vec<String>,
}

fn err_last() -> io::Error {
    io::Error::last_os_error()
}

fn cstr(bytes: &[u8]) -> io::Result<CString> {
    CString::new(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in path"))
}

/// Relative path with only normal components.
fn checked_rel(rel: &Path) -> io::Result<CString> {
    if rel.as_os_str().is_empty() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "target must be a plain path below HOME",
        ));
    }
    cstr(rel.as_os_str().as_bytes())
}

fn openat2(dirfd: i32, path: &CStr, flags: i32) -> io::Result<OwnedFd> {
    let how = OpenHow {
        flags: (flags | libc::O_CLOEXEC) as u64,
        mode: 0,
        resolve: RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_XDEV,
    };
    // SAFETY: valid pointers for the duration of the call; result checked.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            dirfd,
            path.as_ptr(),
            &how as *const OpenHow,
            std::mem::size_of::<OpenHow>(),
        )
    };
    if fd < 0 {
        return Err(err_last());
    }
    // SAFETY: the kernel returned a fresh descriptor we now own.
    Ok(unsafe { OwnedFd::from_raw_fd(fd as i32) })
}

fn fstat(fd: i32) -> io::Result<libc::stat> {
    // SAFETY: zeroed stat is a valid out-buffer.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd, &mut st) } != 0 {
        return Err(err_last());
    }
    Ok(st)
}

fn fstatat(dirfd: i32, name: &CStr) -> io::Result<libc::stat> {
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: name is NUL-terminated, st is a valid buffer.
    if unsafe { libc::fstatat(dirfd, name.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) } != 0 {
        return Err(err_last());
    }
    Ok(st)
}

fn is_dir(st: &libc::stat) -> bool {
    st.st_mode & libc::S_IFMT == libc::S_IFDIR
}

/// Names in a directory (without `.`/`..`), read through a duplicate fd.
fn read_names(dirfd: i32) -> io::Result<Vec<CString>> {
    // SAFETY: dup of a valid fd; ownership passes to the DIR stream.
    let dup = unsafe { libc::fcntl(dirfd, libc::F_DUPFD_CLOEXEC, 0) };
    if dup < 0 {
        return Err(err_last());
    }
    let dir = unsafe { libc::fdopendir(dup) };
    if dir.is_null() {
        let e = err_last();
        unsafe { libc::close(dup) };
        return Err(e);
    }
    // fdopendir on a dup shares the offset with the original; rewind.
    unsafe { libc::rewinddir(dir) };
    let mut names = Vec::new();
    loop {
        // SAFETY: dir is a valid stream until closedir below.
        let ent = unsafe { libc::readdir64(dir) };
        if ent.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*ent).d_name.as_ptr()) };
        let b = name.to_bytes();
        if b != b"." && b != b".." {
            names.push(name.to_owned());
        }
    }
    unsafe { libc::closedir(dir) };
    Ok(names)
}

fn open_child_dir(dirfd: i32, name: &CStr) -> io::Result<OwnedFd> {
    // SAFETY: plain openat; result checked.
    let fd = unsafe {
        libc::openat(
            dirfd,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(err_last());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// A HOME directory handle; all targets are resolved beneath it.
pub struct Home {
    fd: OwnedFd,
    pub uid: u32,
}

impl Home {
    pub fn open(home: &Path) -> io::Result<Self> {
        let p = cstr(home.as_os_str().as_bytes())?;
        // SAFETY: plain open; result checked.
        let fd = unsafe {
            libc::open(
                p.as_ptr(),
                libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(err_last());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Ok(Self {
            fd,
            uid: unsafe { libc::geteuid() },
        })
    }

    fn open_target(&self, rel: &Path, flags: i32) -> io::Result<OwnedFd> {
        openat2(self.fd.as_raw_fd(), &checked_rel(rel)?, flags)
    }

    /// Size of a file or directory tree. `Ok(None)` when it does not exist.
    pub fn measure(&self, rel: &Path) -> io::Result<Option<Usage>> {
        // O_PATH opens nothing: a FIFO (e.g. ~/.steam/steam.pipe) or a device
        // would block or react to a real open(2).
        let fd = match self.open_target(rel, libc::O_PATH | libc::O_NOFOLLOW) {
            Ok(fd) => fd,
            Err(e) if e.raw_os_error() == Some(libc::ENOENT) => return Ok(None),
            Err(e) => return Err(e),
        };
        let st = fstat(fd.as_raw_fd())?;
        let mut usage = Usage::default();
        if is_dir(&st) {
            let dir =
                self.open_target(rel, libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW)?;
            let dst = fstat(dir.as_raw_fd())?;
            if dst.st_ino != st.st_ino || dst.st_dev != st.st_dev {
                return Err(io::Error::other("directory changed during measuring"));
            }
            let mut seen = HashSet::new();
            measure_dir(dir.as_raw_fd(), st.st_dev, 0, &mut seen, &mut usage);
        } else {
            usage.bytes = st.st_blocks as u64 * 512;
            usage.files = 1;
            if rel.file_name().is_some_and(|n| is_media(n.as_bytes())) {
                usage.media_bytes = usage.bytes;
            }
        }
        Ok(Some(usage))
    }

    /// Delete everything inside a directory, keeping the directory itself.
    pub fn clear_dir(&self, rel: &Path) -> io::Result<DeleteReport> {
        self.clear_dir_keep(rel, &[])
    }

    /// Like [`Home::clear_dir`], but top-level entries named in `keep` stay.
    pub fn clear_dir_keep(&self, rel: &Path, keep: &[&str]) -> io::Result<DeleteReport> {
        let fd = match self.open_target(rel, libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW)
        {
            Ok(fd) => fd,
            Err(e) if e.raw_os_error() == Some(libc::ENOENT) => return Ok(DeleteReport::default()),
            Err(e) => return Err(e),
        };
        let st = fstat(fd.as_raw_fd())?;
        if st.st_uid != self.uid {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "target owned by another user",
            ));
        }
        let mut rep = DeleteReport::default();
        clear_contents(fd.as_raw_fd(), st.st_dev, self.uid, 0, keep, &mut rep);
        Ok(rep)
    }

    /// Delete a whole directory tree including the directory itself.
    pub fn remove_dir(&self, rel: &Path) -> io::Result<DeleteReport> {
        let name = rel
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no directory name"))?;
        let opened;
        // A direct child of the base uses the base's O_PATH descriptor,
        // which is a valid dirfd for fstatat/openat/unlinkat.
        let pfd = match rel.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(parent) => {
                opened = self.open_target(
                    parent,
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
                )?;
                opened.as_raw_fd()
            }
            None => {
                checked_rel(rel)?;
                self.fd.as_raw_fd()
            }
        };
        let cname = cstr(name.as_bytes())?;
        let st = match fstatat(pfd, &cname) {
            Ok(st) => st,
            Err(e) if e.raw_os_error() == Some(libc::ENOENT) => return Ok(DeleteReport::default()),
            Err(e) => return Err(e),
        };
        let pst = fstat(pfd)?;
        if !is_dir(&st) || st.st_uid != self.uid || st.st_dev != pst.st_dev {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "not a user-owned directory",
            ));
        }
        let child = open_child_dir(pfd, &cname)?;
        let cst = fstat(child.as_raw_fd())?;
        if cst.st_ino != st.st_ino || cst.st_dev != st.st_dev {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "directory changed during check",
            ));
        }
        let mut rep = DeleteReport::default();
        if clear_contents(child.as_raw_fd(), st.st_dev, self.uid, 0, &[], &mut rep) {
            drop(child);
            unlink(pfd, &cname, true)?;
            rep.freed.bytes += st.st_blocks as u64 * 512;
            rep.freed.dirs += 1;
        }
        Ok(rep)
    }

    /// Delete one regular file or symlink (never a directory).
    pub fn remove_file(&self, rel: &Path) -> io::Result<DeleteReport> {
        let parent = rel.parent().filter(|p| !p.as_os_str().is_empty());
        let name = rel
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
        let opened;
        let pfd = match parent {
            Some(p) => {
                opened =
                    self.open_target(p, libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW)?;
                opened.as_raw_fd()
            }
            // O_PATH descriptors are valid dirfds for fstatat/unlinkat.
            None => self.fd.as_raw_fd(),
        };
        let cname = cstr(name.as_bytes())?;
        let mut rep = DeleteReport::default();
        let st = match fstatat(pfd, &cname) {
            Ok(st) => st,
            Err(e) if e.raw_os_error() == Some(libc::ENOENT) => return Ok(rep),
            Err(e) => return Err(e),
        };
        if is_dir(&st) || st.st_uid != self.uid {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "not a user-owned file",
            ));
        }
        unlink(pfd, &cname, false)?;
        rep.freed.bytes = st.st_blocks as u64 * 512;
        rep.freed.files = 1;
        Ok(rep)
    }
}

fn unlink(dirfd: i32, name: &CStr, dir: bool) -> io::Result<()> {
    let flags = if dir { libc::AT_REMOVEDIR } else { 0 };
    // SAFETY: name is NUL-terminated.
    if unsafe { libc::unlinkat(dirfd, name.as_ptr(), flags) } != 0 {
        return Err(err_last());
    }
    Ok(())
}

fn measure_dir(
    dirfd: i32,
    dev: u64,
    depth: usize,
    seen: &mut HashSet<(u64, u64)>,
    usage: &mut Usage,
) {
    let Ok(st) = fstat(dirfd) else { return };
    usage.bytes += st.st_blocks as u64 * 512;
    usage.dirs += 1;
    if depth >= MAX_DEPTH {
        return;
    }
    let Ok(names) = read_names(dirfd) else { return };
    for name in names {
        let Ok(st) = fstatat(dirfd, &name) else {
            continue;
        };
        if st.st_dev != dev {
            continue;
        }
        if is_dir(&st) {
            if let Ok(child) = open_child_dir(dirfd, &name) {
                measure_dir(child.as_raw_fd(), dev, depth + 1, seen, usage);
            }
            continue;
        }
        if st.st_nlink > 1 && !seen.insert((st.st_dev, st.st_ino)) {
            continue;
        }
        let b = st.st_blocks as u64 * 512;
        if st.st_nlink > 1 {
            usage.shared_bytes += b;
        }
        usage.bytes += b;
        usage.files += 1;
        if is_media(name.to_bytes()) {
            usage.media_bytes += b;
        }
    }
}

/// Remove the contents of `dirfd`. Returns true when the directory is now empty.
fn clear_contents(
    dirfd: i32,
    dev: u64,
    uid: u32,
    depth: usize,
    keep: &[&str],
    rep: &mut DeleteReport,
) -> bool {
    if depth >= MAX_DEPTH {
        rep.skipped += 1;
        return false;
    }
    let names = match read_names(dirfd) {
        Ok(n) => n,
        Err(e) => {
            rep.errors.push(format!("readdir: {e}"));
            return false;
        }
    };
    let mut empty = true;
    for name in names {
        if keep.iter().any(|k| k.as_bytes() == name.to_bytes()) {
            empty = false;
            continue;
        }
        let st = match fstatat(dirfd, &name) {
            Ok(st) => st,
            Err(e) if e.raw_os_error() == Some(libc::ENOENT) => continue,
            Err(e) => {
                rep.errors.push(format!(
                    "{}: {e}",
                    OsStr::from_bytes(name.to_bytes()).to_string_lossy()
                ));
                empty = false;
                continue;
            }
        };
        if st.st_uid != uid || st.st_dev != dev {
            rep.skipped += 1;
            empty = false;
            continue;
        }
        if is_dir(&st) {
            let child = match open_child_dir(dirfd, &name) {
                Ok(c) => c,
                Err(e) => {
                    rep.errors.push(format!(
                        "{}: {e}",
                        OsStr::from_bytes(name.to_bytes()).to_string_lossy()
                    ));
                    empty = false;
                    continue;
                }
            };
            // The opened directory must be the one we just inspected.
            match fstat(child.as_raw_fd()) {
                Ok(c) if c.st_ino == st.st_ino && c.st_dev == st.st_dev => {}
                _ => {
                    rep.skipped += 1;
                    empty = false;
                    continue;
                }
            }
            if !clear_contents(child.as_raw_fd(), dev, uid, depth + 1, &[], rep) {
                empty = false;
                continue;
            }
            drop(child);
            match unlink(dirfd, &name, true) {
                Ok(()) => {
                    rep.freed.bytes += st.st_blocks as u64 * 512;
                    rep.freed.dirs += 1;
                }
                Err(e) => {
                    rep.errors.push(format!(
                        "{}: {e}",
                        OsStr::from_bytes(name.to_bytes()).to_string_lossy()
                    ));
                    empty = false;
                }
            }
        } else {
            match unlink(dirfd, &name, false) {
                Ok(()) => {
                    // A hard-linked file only frees space with its last link.
                    if st.st_nlink <= 1 {
                        rep.freed.bytes += st.st_blocks as u64 * 512;
                    }
                    rep.freed.files += 1;
                }
                Err(e) if e.raw_os_error() == Some(libc::ENOENT) => {}
                Err(e) => {
                    rep.errors.push(format!(
                        "{}: {e}",
                        OsStr::from_bytes(name.to_bytes()).to_string_lossy()
                    ));
                    empty = false;
                }
            }
        }
    }
    empty
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("xcleaner-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn clears_contents_keeps_dir_and_never_follows_symlinks() {
        let home = tmp("clear");
        let outside = tmp("outside");
        fs::write(outside.join("precious"), b"keep me").unwrap();
        let cache = home.join(".cache/app");
        fs::create_dir_all(cache.join("a/b")).unwrap();
        fs::write(cache.join("a/b/f"), vec![1u8; 10_000]).unwrap();
        fs::write(cache.join("top"), b"x").unwrap();
        std::os::unix::fs::symlink(&outside, cache.join("link")).unwrap();

        let h = Home::open(&home).unwrap();
        let before = h.measure(Path::new(".cache/app")).unwrap().unwrap();
        assert_eq!(before.files, 3); // f, top, link
        let rep = h.clear_dir(Path::new(".cache/app")).unwrap();
        assert!(rep.errors.is_empty(), "{:?}", rep.errors);
        assert!(cache.is_dir());
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
        assert!(outside.join("precious").exists());
        fs::remove_dir_all(&home).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn refuses_symlinked_target_and_dotdot() {
        let home = tmp("refuse");
        let outside = tmp("refuse-out");
        fs::write(outside.join("precious"), b"keep").unwrap();
        fs::create_dir_all(home.join(".cache")).unwrap();
        std::os::unix::fs::symlink(&outside, home.join(".cache/evil")).unwrap();
        let h = Home::open(&home).unwrap();
        assert!(h.clear_dir(Path::new(".cache/evil")).is_err());
        assert!(h.clear_dir(Path::new("../x")).is_err());
        assert!(h.clear_dir(Path::new("/etc")).is_err());
        assert!(outside.join("precious").exists());
        fs::remove_dir_all(&home).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn keep_list_preserves_state_files() {
        let home = tmp("keep");
        let yay = home.join(".cache/yay");
        fs::create_dir_all(yay.join("pkg")).unwrap();
        fs::write(yay.join("vcs.json"), b"{}").unwrap();
        fs::write(yay.join("pkg/PKGBUILD"), b"x").unwrap();
        let h = Home::open(&home).unwrap();
        h.clear_dir_keep(Path::new(".cache/yay"), &["vcs.json"])
            .unwrap();
        assert!(yay.join("vcs.json").exists());
        assert!(!yay.join("pkg").exists());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn remove_dir_deletes_tree_and_dir() {
        let home = tmp("rmdir");
        let old = home.join(".config/discord/app-1.0.1");
        fs::create_dir_all(old.join("modules")).unwrap();
        fs::write(old.join("modules/x.node"), b"x").unwrap();
        let h = Home::open(&home).unwrap();
        let rep = h
            .remove_dir(Path::new(".config/discord/app-1.0.1"))
            .unwrap();
        assert!(rep.errors.is_empty());
        assert!(!old.exists());
        assert!(home.join(".config/discord").is_dir());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn counts_media_bytes() {
        let home = tmp("media");
        let m = home.join(".cache/music");
        fs::create_dir_all(&m).unwrap();
        fs::write(m.join("song.OPUS"), vec![0u8; 64 * 1024]).unwrap();
        fs::write(m.join("index.db"), b"x").unwrap();
        let u = Home::open(&home)
            .unwrap()
            .measure(Path::new(".cache/music"))
            .unwrap()
            .unwrap();
        assert!(u.mostly_media(), "{u:?}");
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn measuring_a_fifo_does_not_block() {
        let home = tmp("fifo");
        fs::create_dir_all(home.join(".steam")).unwrap();
        let p = CString::new(home.join(".steam/steam.pipe").as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(p.as_ptr(), 0o600) }, 0);
        let h = Home::open(&home).unwrap();
        let u = h.measure(Path::new(".steam/steam.pipe")).unwrap().unwrap();
        assert_eq!(u.files, 1);
        let d = h.measure(Path::new(".steam")).unwrap().unwrap();
        assert_eq!(d.files, 1);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn missing_target_is_not_an_error() {
        let home = tmp("missing");
        let h = Home::open(&home).unwrap();
        assert_eq!(h.measure(Path::new(".cache/nope")).unwrap(), None);
        assert_eq!(
            h.clear_dir(Path::new(".cache/nope")).unwrap().freed,
            Usage::default()
        );
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn removes_single_file() {
        let home = tmp("file");
        fs::create_dir_all(home.join(".local/share")).unwrap();
        fs::write(home.join(".local/share/recently-used.xbel"), b"<xbel/>").unwrap();
        let h = Home::open(&home).unwrap();
        let rep = h
            .remove_file(Path::new(".local/share/recently-used.xbel"))
            .unwrap();
        assert_eq!(rep.freed.files, 1);
        assert!(!home.join(".local/share/recently-used.xbel").exists());
        fs::remove_dir_all(&home).unwrap();
    }
}
