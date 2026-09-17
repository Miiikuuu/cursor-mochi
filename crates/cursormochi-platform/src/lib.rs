#![forbid(unsafe_code)]
mod cache;
mod classification;
pub mod import;
mod settings;
use cursormochi_app::{Catalog, ThemeRepository};
use cursormochi_core::*;
pub use settings::*;
use std::{
    collections::{BTreeMap, HashSet},
    ffi::OsString,
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};
pub const POLICY: &str = "xcursor-ordered-v1; host loader paths unverified";
#[derive(Clone, Debug)]
pub struct Environment {
    pub vars: BTreeMap<String, OsString>,
}
impl Environment {
    pub fn capture() -> Self {
        Self {
            vars: std::env::vars_os()
                .filter_map(|(k, v)| k.into_string().ok().map(|k| (k, v)))
                .collect(),
        }
    }
    pub fn text(&self, k: &str) -> String {
        self.vars
            .get(k)
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .into()
    }
}
#[derive(Clone, Debug)]
pub struct Paths {
    pub discovery: Vec<PathBuf>,
    pub resolution: Vec<PathBuf>,
    pub diagnostics: Vec<String>,
}
impl Paths {
    pub fn from_env(e: &Environment) -> Self {
        let mut p = Self {
            discovery: vec![],
            resolution: vec![],
            diagnostics: vec![],
        };
        let home = e
            .vars
            .get("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute());
        let mut explicit = vec![];
        if let Some(v) = e.vars.get("XCURSOR_PATH") {
            for path in std::env::split_paths(v) {
                let path = if let Ok(rest) = path.strip_prefix("~") {
                    home.as_ref().map(|h| h.join(rest)).unwrap_or(path)
                } else {
                    path
                };
                if path.is_absolute() {
                    explicit.push(path)
                } else {
                    p.diagnostics
                        .push("XCURSOR_PATH: ignored relative/empty entry".into())
                }
            }
        }
        p.discovery.extend(explicit.clone());
        let data = e
            .vars
            .get("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);
        match data {
            Some(d) if d.is_absolute() => p.discovery.push(d.join("icons")),
            Some(_) => {
                p.diagnostics
                    .push("XDG_DATA_HOME: relative path rejected; default used".into());
                if let Some(h) = &home {
                    p.discovery.push(h.join(".local/share/icons"))
                }
            }
            None => {
                if let Some(h) = &home {
                    p.discovery.push(h.join(".local/share/icons"))
                }
            }
        }
        if let Some(h) = &home {
            p.discovery.push(h.join(".icons"))
        }
        let dirs = e
            .vars
            .get("XDG_DATA_DIRS")
            .filter(|v| !v.is_empty())
            .cloned()
            .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        for d in std::env::split_paths(&dirs) {
            if d.is_absolute() {
                p.discovery.push(d.join("icons"))
            } else {
                p.diagnostics
                    .push("XDG_DATA_DIRS: relative entry rejected".into())
            }
        }
        // Ubuntu libXcursor compiled default includes /usr/share/pixmaps.
        p.discovery.push("/usr/share/pixmaps".into());
        let mut seen = HashSet::new();
        p.discovery.retain(|v| seen.insert(v.clone()));
        p.resolution = if explicit.is_empty() {
            let mut v = vec![];
            if let Some(h) = home {
                v.push(h.join(".icons"))
            }
            v.extend([
                PathBuf::from("/usr/share/icons"),
                PathBuf::from("/usr/share/pixmaps"),
            ]);
            v
        } else {
            explicit
        };
        p
    }
    pub fn fixture(root: PathBuf) -> Self {
        Self {
            discovery: vec![root.clone()],
            resolution: vec![root],
            diagnostics: vec!["fixture: isolated read-only assets".into()],
        }
    }
}
#[derive(Clone)]
pub struct Repository {
    pub paths: Paths,
    allowed: Vec<PathBuf>,
    cache: std::sync::Arc<std::sync::Mutex<cache::Cache>>,
}
fn io(e: impl std::fmt::Display) -> Error {
    Error::Io(e.to_string())
}
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => (),
            _ => out.push(c.as_os_str()),
        }
    }
    out
}
impl Repository {
    pub fn new(paths: Paths) -> Self {
        let allowed = paths
            .discovery
            .iter()
            .chain(paths.resolution.iter())
            .filter_map(|p| fs::canonicalize(p).ok())
            .collect();
        Self {
            paths,
            allowed,
            cache: std::sync::Arc::new(std::sync::Mutex::new(cache::Cache::new(128 * 1024 * 1024))),
        }
    }
    pub fn refreshed(&self) -> Self {
        let mut repo = Self::new(self.paths.clone());
        repo.cache = self.cache.clone();
        repo
    }
    fn permitted(&self, p: &Path) -> bool {
        self.allowed.iter().any(|r| p.starts_with(r))
    }
    fn resolve_link(&self, p: &Path) -> Result<(PathBuf, Vec<String>), Error> {
        let mut current = normalize(p);
        let mut seen = HashSet::new();
        let mut trace = vec![];
        for _ in 0..=32 {
            if !seen.insert(current.clone()) {
                return Err(Error::Invalid("symlink cycle"));
            }
            let mut prefix = PathBuf::new();
            let parts: Vec<_> = current.components().collect();
            let mut next = None;
            for (i, c) in parts.iter().enumerate() {
                prefix.push(c.as_os_str());
                let m = fs::symlink_metadata(&prefix).map_err(io)?;
                if m.file_type().is_symlink() {
                    let t = fs::read_link(&prefix).map_err(io)?;
                    let target = normalize(&if t.is_absolute() {
                        t
                    } else {
                        prefix.parent().unwrap_or(Path::new("/")).join(t)
                    });
                    if !self.permitted(&target) && !target.starts_with("/etc/alternatives") {
                        return Err(Error::Invalid("symlink target outside declared roots"));
                    }
                    trace.push(format!("{} → {}", prefix.display(), target.display()));
                    let mut t = target;
                    for c in &parts[i + 1..] {
                        t.push(c.as_os_str())
                    }
                    next = Some(normalize(&t));
                    break;
                }
            }
            match next {
                Some(n) => current = n,
                None => {
                    if !self.permitted(&current) {
                        return Err(Error::Invalid("outside declared roots"));
                    }
                    return Ok((current, trace));
                }
            }
        }
        Err(Error::LimitExceeded("symlink depth"))
    }
    pub fn read_bounded(
        &self,
        p: &Path,
        limit: usize,
    ) -> Result<(Vec<u8>, PathBuf, Vec<String>), Error> {
        let (target, trace) = self.resolve_link(p)?;
        let fd = rustix::fs::open(
            &target,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::NONBLOCK
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(io)?;
        let file = File::from(fd);
        let m = file.metadata().map_err(io)?;
        if !m.is_file() {
            return Err(Error::Invalid("not a regular file"));
        }
        if m.len() > limit as u64 {
            return Err(Error::LimitExceeded("file bytes"));
        }
        use std::os::fd::AsRawFd;
        let actual = fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd())).map_err(io)?;
        if !self.permitted(&actual) {
            return Err(Error::Invalid("opened handle outside declared roots"));
        }
        let mut b = vec![];
        file.take((limit + 1) as u64)
            .read_to_end(&mut b)
            .map_err(io)?;
        if b.len() > limit {
            return Err(Error::LimitExceeded("file growth"));
        }
        Ok((b, actual, trace))
    }
    fn index(&self, p: &Path) -> Result<(Option<String>, Vec<ThemeName>), Error> {
        let (b, _, _) = self.read_bounded(&p.join("index.theme"), 256 * 1024)?;
        let s = std::str::from_utf8(&b).map_err(|_| Error::Invalid("metadata UTF-8"))?;
        Ok(metadata(s))
    }
    fn find(
        &self,
        t: &ThemeName,
        role: &str,
        stack: &mut Vec<String>,
        cancel: &dyn Fn() -> bool,
        visits: &mut usize,
    ) -> Result<Preview, Error> {
        if cancel() {
            return Err(Error::Cancelled);
        }
        *visits += 1;
        if *visits > 4096 {
            return Err(Error::LimitExceeded("inheritance visits"));
        }
        if stack.len() >= 32 {
            return Err(Error::LimitExceeded("inheritance depth"));
        }
        if stack.iter().any(|s| s == t.as_str()) {
            return Err(Error::Invalid("inheritance cycle"));
        }
        stack.push(t.as_str().into());
        let result = (|| {
            let mut inherits = None;
            for root in &self.paths.resolution {
                if cancel() {
                    return Err(Error::Cancelled);
                }
                let dir = root.join(t.as_str());
                let cursor = dir.join("cursors").join(role);
                if fs::symlink_metadata(&cursor).is_ok() {
                    let (b, source, links) = self.read_bounded(&cursor, 16 * 1024 * 1024)?;
                    let cached = self.cache.lock().ok().and_then(|mut c| c.get(&source, &b));
                    let variants = if let Some(v) = cached {
                        v
                    } else {
                        let v = decode(&b, cancel)?;
                        if let Ok(mut cache) = self.cache.lock() {
                            cache.insert(source.clone(), b, v.clone());
                        }
                        v
                    };
                    if cancel() {
                        return Err(Error::Cancelled);
                    }
                    let mut chain = stack.clone();
                    chain.extend(links);
                    return Ok(Preview {
                        requested: t.clone(),
                        role: role.into(),
                        source,
                        chain,
                        resolution: if stack.len() > 1 {
                            Resolution::Inherited
                        } else {
                            Resolution::Direct
                        },
                        variants,
                    });
                }
                if inherits.is_none()
                    && let Ok((_, i)) = self.index(&dir)
                    && !i.is_empty()
                {
                    inherits = Some(i)
                }
            }
            let mut error = Error::Missing;
            for parent in inherits.unwrap_or_default() {
                match self.find(&parent, role, stack, cancel, visits) {
                    Ok(p) => return Ok(p),
                    Err(e) => {
                        if e == Error::Cancelled {
                            return Err(e);
                        }
                        if e != Error::Missing {
                            error = e
                        }
                    }
                }
            }
            Err(error)
        })();
        stack.pop();
        result
    }
}
impl ThemeRepository for Repository {
    fn scan(&self, cancel: &dyn Fn() -> bool) -> Result<Catalog, Error> {
        let mut records: BTreeMap<String, Theme> = BTreeMap::new();
        let mut diagnostics = self.paths.diagnostics.clone();
        let mut entries = 0;
        let mut role_entries = 0;
        for (priority, root) in self.paths.discovery.iter().enumerate() {
            let dirs = match fs::read_dir(root) {
                Ok(d) => d,
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        diagnostics.push(format!("scan {}: {e}", root.display()))
                    }
                    continue;
                }
            };
            for entry in dirs {
                if cancel() {
                    return Err(Error::Cancelled);
                }
                entries += 1;
                if entries > 16384 {
                    return Err(Error::LimitExceeded("theme entries"));
                }
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        diagnostics.push(e.to_string());
                        continue;
                    }
                };
                let raw = entry.file_name();
                let Some(name) = raw.to_str() else {
                    diagnostics.push(format!(
                        "Invalid theme name (non UTF-8): {:?}",
                        entry.path()
                    ));
                    continue;
                };
                let Ok(name) = ThemeName::new(name) else {
                    continue;
                };
                let dir = entry.path();
                let safe = match self.resolve_link(&dir) {
                    Ok((p, _)) => p,
                    Err(e) => {
                        diagnostics.push(format!("{}: {e}", dir.display()));
                        continue;
                    }
                };
                if !safe.is_dir() {
                    continue;
                }
                let mut candidate_error = None;
                let (display, inherits) = match self.index(&dir) {
                    Ok(v) => v,
                    Err(e) => {
                        if dir.join("index.theme").symlink_metadata().is_ok() {
                            candidate_error = Some(e.to_string());
                            diagnostics.push(format!("{}: {e}", dir.display()))
                        }
                        (None, vec![])
                    }
                };
                let mut roles = vec![];
                if let Ok((cursors, _)) = self.resolve_link(&dir.join("cursors"))
                    && let Ok(files) = fs::read_dir(cursors)
                {
                    for (n, file) in files.enumerate() {
                        role_entries += 1;
                        if role_entries > 65536 {
                            return Err(Error::LimitExceeded("catalog role entries"));
                        }
                        if cancel() {
                            return Err(Error::Cancelled);
                        }
                        if n >= 8192 {
                            diagnostics
                                .push(format!("{}: LimitExceeded role entries", dir.display()));
                            break;
                        }
                        if let Ok(file) = file {
                            if let Some(s) = file.file_name().to_str() {
                                if ThemeName::new(s).is_ok() {
                                    roles.push(s.into())
                                }
                            } else {
                                diagnostics.push(format!("{:?}: non UTF-8 role", file.path()))
                            }
                        }
                    }
                }
                if roles.is_empty() && dir.join("cursors").symlink_metadata().is_ok() {
                    match self.resolve_link(&dir.join("cursors")) {
                        Err(e) => candidate_error = Some(e.to_string()),
                        Ok((path, _)) => {
                            if let Err(e) = fs::read_dir(path) {
                                candidate_error = Some(e.to_string());
                            }
                        }
                    }
                }
                if roles.is_empty()
                    && inherits.is_empty()
                    && !dir.join("index.theme").is_file()
                    && candidate_error.is_none()
                {
                    continue;
                }
                let record = records
                    .entry(name.as_str().into())
                    .or_insert_with(|| Theme {
                        name: name.clone(),
                        display: display.unwrap_or_else(|| name.as_str().into()),
                        locations: vec![],
                        roles: vec![],
                        verified_roles: vec![],
                        availability: Availability::Unverified("Not probed yet".into()),
                        issues: vec![],
                    });
                if let Some(e) = candidate_error {
                    record.availability = Availability::Invalid(e);
                }
                record.locations.push(Location {
                    directory: dir,
                    priority,
                    inherits,
                });
                record.roles.extend(roles);
            }
        }
        let mut themes: Vec<_> = records.into_values().collect();
        for t in &mut themes {
            t.roles.sort();
            t.roles.dedup()
        }
        themes.sort_by(|a, b| {
            a.display
                .to_lowercase()
                .cmp(&b.display.to_lowercase())
                .then(a.name.as_str().cmp(b.name.as_str()))
        });
        self.classify(themes, diagnostics, cancel)
    }
    fn preview(
        &self,
        theme: &ThemeName,
        role: &str,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Preview, Error> {
        ThemeName::new(role)?;
        let mut visits = 0;
        let mut p = match self.find(theme, role, &mut vec![], cancel, &mut visits) {
            Ok(p) => p,
            Err(Error::Missing) if theme.as_str() != "default" => {
                let mut p = self.find(
                    &ThemeName::new("default")?,
                    role,
                    &mut vec![],
                    cancel,
                    &mut visits,
                )?;
                p.resolution = Resolution::Fallback;
                p
            }
            Err(e) => return Err(e),
        };
        p.requested = theme.clone();
        Ok(p)
    }
}
pub fn redact(s: &str, e: &Environment) -> String {
    let mut out = s.to_owned();
    if let Some(home) = e.vars.get("HOME").and_then(|s| s.to_str())
        && !home.is_empty()
    {
        out = out.replace(home, "$HOME")
    }
    // Also redact arbitrary user roots supplied through XCURSOR_PATH/XDG.
    for key in ["XCURSOR_PATH", "XDG_DATA_HOME", "XDG_DATA_DIRS"] {
        if let Some(v) = e.vars.get(key) {
            for p in std::env::split_paths(v) {
                if !p.starts_with("/usr")
                    && !p.starts_with("/etc")
                    && let Some(s) = p.to_str()
                    && !s.is_empty()
                {
                    out = out.replace(s, "$USER_PATH")
                }
            }
        }
    }
    out
}
