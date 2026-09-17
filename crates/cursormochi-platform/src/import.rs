//! Strict package reads and create-only installation. Never executes package content.
use cursormochi_app::{
    import::IMPORT_ROLES,
    import::{Asset, Package, Plan},
};
use cursormochi_core::{Error, export, windows};
use rustix::fs::{CWD, Mode, OFlags, mkdirat, open, openat};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::fs::MetadataExt,
    },
    path::{Component, Path, PathBuf},
};
const MAX_FILES: usize = 1024;
const MAX_TOTAL: usize = 64 * 1024 * 1024;
const MAX_ARCHIVE: usize = 32 * 1024 * 1024;
fn err(e: impl std::fmt::Display) -> Error {
    Error::Io(e.to_string())
}
fn check(cancel: &dyn Fn() -> bool) -> Result<(), Error> {
    if cancel() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn relative(path: &Path) -> Result<(), Error> {
    let text = path
        .to_str()
        .ok_or(Error::Unsupported("non-UTF8 package path"))?;
    if text.is_empty()
        || text.len() > 1024
        || text.contains(['\\', ':'])
        || text.chars().any(char::is_control)
        || path.components().count() > 16
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::Invalid("unsafe package path"));
    }
    Ok(())
}
fn directory(path: &Path, create: bool) -> Result<OwnedFd, Error> {
    if !path.is_absolute() {
        return Err(Error::Invalid("absolute directory required"));
    }
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut fd = open("/", flags, Mode::empty()).map_err(err)?;
    for c in path.components() {
        if c == Component::RootDir {
            continue;
        }
        let Component::Normal(name) = c else {
            return Err(Error::Invalid("directory traversal"));
        };
        if create {
            match mkdirat(&fd, name, Mode::from_bits_truncate(0o700)) {
                Ok(()) => (),
                Err(rustix::io::Errno::EXIST) => (),
                Err(e) => return Err(err(e)),
            }
        }
        fd = openat(&fd, name, flags, Mode::empty()).map_err(err)?;
    }
    Ok(fd)
}
fn handle_path(fd: &OwnedFd) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", fd.as_raw_fd()))
}
fn read_at(fd: &OwnedFd, name: &std::ffi::OsStr, limit: usize) -> Result<Vec<u8>, Error> {
    let file = File::from(
        openat(
            fd,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(err)?,
    );
    let m = file.metadata().map_err(err)?;
    if !m.is_file() || m.nlink() != 1 {
        return Err(Error::Invalid("package entry is not a regular file"));
    }
    if m.len() > limit as u64 {
        return Err(Error::LimitExceeded("input file bytes"));
    }
    let mut b = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut b)
        .map_err(err)?;
    if b.len() > limit {
        return Err(Error::LimitExceeded("input growth"));
    }
    Ok(b)
}
fn is_cursor(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("cur") || e.eq_ignore_ascii_case("ani"))
}
struct ReadBudget {
    entries: usize,
    bytes: usize,
    pixels: usize,
}
impl ReadBudget {
    fn entry(&mut self, bytes: usize) -> Result<(), Error> {
        self.entries += 1;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(Error::LimitExceeded("package bytes"))?;
        if self.entries > MAX_FILES || self.bytes > MAX_TOTAL {
            Err(Error::LimitExceeded("package entries/expanded bytes"))
        } else {
            Ok(())
        }
    }
    fn asset(
        &mut self,
        p: PathBuf,
        b: &[u8],
        out: &mut Package,
        cancel: &dyn Fn() -> bool,
    ) -> Result<(), Error> {
        check(cancel)?;
        if !is_cursor(&p) {
            out.ignored += 1;
            return Ok(());
        }
        if out.assets.len() >= 128 {
            return Err(Error::LimitExceeded("128 cursor files"));
        }
        let d = windows::decode(b, cancel);
        if let Ok(d) = &d {
            self.pixels += d
                .variants
                .iter()
                .flat_map(|v| &v.frames)
                .map(|f| f.rgba.len())
                .sum::<usize>();
            if self.pixels > MAX_TOTAL {
                return Err(Error::LimitExceeded("package decoded pixels"));
            }
        }
        if matches!(d, Err(Error::Cancelled)) {
            return Err(Error::Cancelled);
        }
        out.assets.push(Asset {
            path: p,
            decoded: d,
        });
        Ok(())
    }
}
fn walk(
    fd: &OwnedFd,
    base: &Path,
    budget: &mut ReadBudget,
    out: &mut Package,
    cancel: &dyn Fn() -> bool,
) -> Result<(), Error> {
    for item in fs::read_dir(handle_path(fd)).map_err(err)? {
        check(cancel)?;
        let item = item.map_err(err)?;
        let name = item.file_name();
        let rel = base.join(&name);
        relative(&rel)?;
        let kind = item.file_type().map_err(err)?;
        if kind.is_dir() {
            budget.entry(0)?;
            let child = openat(
                fd,
                &name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(err)?;
            walk(&child, &rel, budget, out, cancel)?;
        } else if kind.is_file() {
            let bytes = read_at(fd, &name, windows::MAX_INPUT)?;
            budget.entry(bytes.len())?;
            budget.asset(rel, &bytes, out, cancel)?;
        } else {
            return Err(Error::Unsupported(
                "package links and special files are not accepted",
            ));
        }
    }
    Ok(())
}
fn unzip(
    b: &[u8],
    budget: &mut ReadBudget,
    out: &mut Package,
    cancel: &dyn Fn() -> bool,
) -> Result<(), Error> {
    // Bound central-directory allocation before ZipArchive reads its index.
    let start = b.len().saturating_sub(65557);
    let eocd = (start..b.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            b.get(i..i + 4) == Some(b"PK\x05\x06")
                && b.get(i + 20..i + 22)
                    .is_some_and(|s| i + 22 + u16::from_le_bytes([s[0], s[1]]) as usize == b.len())
        })
        .ok_or(Error::Invalid("ZIP end record"))?;
    let h = &b[eocd..];
    let u16at = |i| u16::from_le_bytes([h[i], h[i + 1]]);
    let u32at = |i| u32::from_le_bytes([h[i], h[i + 1], h[i + 2], h[i + 3]]);
    if u16at(4) != 0
        || u16at(6) != 0
        || u16at(8) != u16at(10)
        || u16at(10) as usize > MAX_FILES
        || u32at(12) > 2 * 1024 * 1024
        || u64::from(u32at(16)) + u64::from(u32at(12)) != eocd as u64
    {
        return Err(Error::Unsupported(
            "ZIP64, split ZIP, oversized or nonstandard index",
        ));
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(b)).map_err(err)?;
    // zip stores entries by name and can silently merge duplicate names. Reject
    // a changed index count before iterating the normalized entries.
    if zip.len() != u16at(10) as usize {
        return Err(Error::Invalid(
            "duplicate ZIP names or central-directory count mismatch",
        ));
    }
    if zip.len() > MAX_FILES {
        return Err(Error::LimitExceeded("ZIP entries"));
    }
    let mut names = BTreeSet::new();
    for i in 0..zip.len() {
        check(cancel)?;
        let mut f = zip.by_index(i).map_err(err)?;
        let raw = f.name().trim_end_matches('/');
        let p = PathBuf::from(raw);
        relative(&p)?;
        if !names.insert(p.clone()) {
            return Err(Error::Invalid("duplicate ZIP path"));
        }
        let mode = f.unix_mode().unwrap_or(0) & 0o170000;
        if !matches!(mode, 0 | 0o100000 | 0o040000)
            || f.encrypted()
            || !matches!(
                f.compression(),
                zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
            )
        {
            return Err(Error::Unsupported(
                "ZIP links, special files, encryption or compression",
            ));
        }
        if f.size() > windows::MAX_INPUT as u64 {
            return Err(Error::LimitExceeded("ZIP entry size"));
        }
        budget.entry(f.size() as usize)?;
        if f.is_dir() {
            continue;
        }
        let mut bytes = Vec::new();
        (&mut f)
            .take((windows::MAX_INPUT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        if bytes.len() != f.size() as usize {
            return Err(Error::Invalid("ZIP expanded size mismatch"));
        }
        budget.asset(p, &bytes, out, cancel)?;
    }
    Ok(())
}
pub fn load(path: &Path, cancel: &dyn Fn() -> bool) -> Result<Package, Error> {
    check(cancel)?;
    let meta = fs::symlink_metadata(path).map_err(err)?;
    let mut out = Package::default();
    let mut budget = ReadBudget {
        entries: 0,
        bytes: 0,
        pixels: 0,
    };
    if meta.is_dir() {
        let fd = directory(path, false)?;
        walk(&fd, Path::new(""), &mut budget, &mut out, cancel)?;
    } else if meta.is_file() {
        let parent = path.parent().ok_or(Error::Invalid("input parent"))?;
        let fd = directory(parent, false)?;
        let name = path.file_name().ok_or(Error::Invalid("input filename"))?;
        let archive = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
        let b = read_at(
            &fd,
            name,
            if archive {
                MAX_ARCHIVE
            } else {
                windows::MAX_INPUT
            },
        )?;
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
        {
            unzip(&b, &mut budget, &mut out, cancel)?;
        } else {
            budget.entry(b.len())?;
            budget.asset(PathBuf::from(name), &b, &mut out, cancel)?;
        }
    } else {
        return Err(Error::Unsupported("input links or special files"));
    }
    out.assets.sort_by(|a, b| a.path.cmp(&b.path));
    if out.assets.is_empty() {
        return Err(Error::Missing);
    }
    Ok(out)
}
pub fn user_targets(env: &crate::Environment) -> Vec<PathBuf> {
    let home = PathBuf::from(env.text("HOME"));
    if !home.is_absolute() {
        return Vec::new();
    }
    let data = PathBuf::from(env.text("XDG_DATA_HOME"));
    let mut paths = vec![
        if data.is_absolute() {
            data.join("icons")
        } else {
            home.join(".local/share/icons")
        },
        home.join(".icons"),
    ];
    paths.dedup();
    paths
}
/// Only generated assets enter staging. NOREPLACE is the commit point; after it,
/// a late cancellation reports success, never deletes the newly installed theme.
pub fn install(
    plan: &Plan,
    root: &Path,
    allowed: &[PathBuf],
    cancel: &dyn Fn() -> bool,
) -> Result<PathBuf, Error> {
    check(cancel)?;
    if !allowed.iter().any(|p| p == root) {
        return Err(Error::Invalid("unconfirmed user theme directory"));
    }
    // Revalidate a public plan rather than trusting callers' fields.
    let assignments = plan
        .mapped
        .iter()
        .map(|(role, a)| {
            IMPORT_ROLES
                .iter()
                .position(|(_, names)| names[0] == role)
                .map(|i| (i, a.clone()))
                .ok_or(Error::Invalid("install role"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let validated = cursormochi_app::import::plan(plan.name.as_str(), &assignments, true)?;
    for other in allowed {
        if fs::symlink_metadata(other.join(validated.name.as_str())).is_ok() {
            return Err(Error::Invalid(
                "theme name already exists in a user theme directory",
            ));
        }
    }
    let fd = directory(root, true)?;
    let held = handle_path(&fd);
    let final_path = root.join(validated.name.as_str());
    if fs::symlink_metadata(held.join(validated.name.as_str())).is_ok() {
        return Err(Error::Invalid(
            "theme name already exists; choose a new name",
        ));
    }
    let stage = tempfile::Builder::new()
        .prefix(".cursormochi-stage-")
        .tempdir_in(&held)
        .map_err(err)?;
    let outcome = (|| {
        // Keep index.theme/cursors one level below the search root's entries:
        // a refresh must never classify an uncommitted staging directory.
        let payload = stage.path().join("theme");
        fs::create_dir(&payload).map_err(err)?;
        let cursors = payload.join("cursors");
        fs::create_dir(&cursors).map_err(err)?;
        let write = |p: &Path, b: &[u8]| -> Result<(), Error> {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(p)
                .map_err(err)?;
            f.write_all(b).map_err(err)?;
            f.sync_all().map_err(err)
        };
        write(
            &payload.join("index.theme"),
            format!(
                "[Icon Theme]\nName={}\nComment=Imported by CursorMochi; no explicit inheritance\n",
                validated.name.as_str()
            )
            .as_bytes(),
        )?;
        let mut output_bytes = 0;
        let mut verified_names = Vec::new();
        for (role, a) in &validated.mapped {
            check(cancel)?;
            let source = &a.decoded.as_ref().map_err(Clone::clone)?.variants;
            let prepared = export::desktop_variants(source, cancel)?;
            let b = export::xcursor(&prepared)?;
            let aliases = IMPORT_ROLES
                .iter()
                .find(|(_, names)| names[0] == role)
                .ok_or(Error::Invalid("install role"))?
                .1;
            for alias in aliases {
                check(cancel)?;
                output_bytes += b.len();
                if output_bytes > 128 * 1024 * 1024 {
                    return Err(Error::LimitExceeded("installed theme bytes"));
                }
                let path = cursors.join(alias);
                write(&path, &b)?;
                let saved = fs::read(&path).map_err(err)?;
                let decoded = cursormochi_core::decode(&saved, cancel)?;
                if !export::equivalent(&prepared, &decoded) {
                    return Err(Error::Invalid("staged output comparison failed"));
                }
                verified_names.push(*alias);
            }
        }
        let coverage = cursormochi_app::current_cursor::coverage(verified_names);
        if coverage != validated.system_coverage() {
            return Err(Error::Invalid(
                "staged system coverage differs from reviewed plan",
            ));
        }
        write(&payload.join("conversion-report.txt"),format!("Verified staged Xcursor data against prepared output. Original-size variants preserve decoded input pixels, dimensions, hotspots, frame order and millisecond delays. Added desktop variants are resampled as described below.\nMissing Windows import roles: {}\n{}\nNo explicit parent theme. Desktop loaders may use their own default fallback.\n{}\n",if validated.missing.is_empty() { "none".into() } else { validated.missing.join(", ") },coverage.details(),validated.notes.join("\n")).as_bytes())?;
        File::open(&cursors)
            .and_then(|f| f.sync_all())
            .map_err(err)?;
        File::open(&payload)
            .and_then(|f| f.sync_all())
            .map_err(err)?;
        check(cancel)?;
        rustix::fs::renameat_with(
            CWD,
            &payload,
            &fd,
            validated.name.as_str(),
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(err)?;
        // TempDir owns only the now-empty outer wrapper, never the committed theme.
        Ok(final_path)
    })();
    match outcome {
        Ok(path) => Ok(path),
        Err(e) => match stage.close() {
            Ok(()) => Err(e),
            Err(cleanup) => Err(Error::Io(format!("{e}; staging cleanup failed: {cleanup}"))),
        },
    }
}
