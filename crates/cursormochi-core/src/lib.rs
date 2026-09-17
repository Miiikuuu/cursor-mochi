#![forbid(unsafe_code)]
pub mod export;
pub mod resize;
pub mod windows;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeName(String);
impl ThemeName {
    pub fn new(s: &str) -> Result<Self, Error> {
        if s.is_empty() || s == "." || s == ".." || s.contains(['/', '\\', '\0']) {
            return Err(Error::Invalid("invalid theme/role name"));
        }
        Ok(Self(s.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid(&'static str),
    Unsupported(&'static str),
    LimitExceeded(&'static str),
    Missing,
    Cancelled,
    Io(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug)]
pub struct Location {
    pub directory: PathBuf,
    pub priority: usize,
    pub inherits: Vec<ThemeName>,
}
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: ThemeName,
    pub display: String,
    pub locations: Vec<Location>,
    pub roles: Vec<String>,
    pub verified_roles: Vec<String>,
    pub availability: Availability,
    pub issues: Vec<String>,
}
/// Evidence about this theme without the automatic default-theme fallback.
#[derive(Clone, Debug)]
pub enum Availability {
    Direct {
        role: String,
        source: PathBuf,
    },
    Inherited {
        role: String,
        source: PathBuf,
        chain: Vec<String>,
    },
    NoCursorSource,
    Invalid(String),
    Unverified(String),
}
impl Availability {
    pub fn usable(&self) -> bool {
        matches!(self, Self::Direct { .. } | Self::Inherited { .. })
    }
    pub fn role(&self) -> Option<&str> {
        match self {
            Self::Direct { role, .. } | Self::Inherited { role, .. } => Some(role),
            _ => None,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Direct { .. } => "Installed",
            Self::Inherited { .. } => "Inherited",
            Self::NoCursorSource => "No cursor source",
            Self::Invalid(_) => "Invalid",
            Self::Unverified(_) => "Unverified",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub hotspot: (u32, u32),
    pub delay: u32,
    pub rgba: Arc<[u8]>,
}
impl Frame {
    pub fn playback_delay(&self) -> u64 {
        u64::from(self.delay.clamp(16, 10_000))
    }
}
#[derive(Clone, Debug)]
pub struct Variant {
    pub nominal: u32,
    pub frames: Vec<Frame>,
}
impl Variant {
    pub fn frame_at(&self, elapsed: u64) -> usize {
        let total: u64 = self.frames.iter().map(Frame::playback_delay).sum();
        let mut t = elapsed % total.max(1);
        for (i, f) in self.frames.iter().enumerate() {
            if t < f.playback_delay() {
                return i;
            }
            t -= f.playback_delay();
        }
        0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Direct,
    Inherited,
    Fallback,
}
#[derive(Clone, Debug)]
pub struct Preview {
    pub requested: ThemeName,
    pub role: String,
    pub source: PathBuf,
    pub chain: Vec<String>,
    pub resolution: Resolution,
    pub variants: Vec<Variant>,
}
fn word(b: &[u8], o: usize) -> Result<u32, Error> {
    let end = o.checked_add(4).ok_or(Error::Invalid("offset overflow"))?;
    Ok(u32::from_le_bytes(
        b.get(o..end)
            .ok_or(Error::Invalid("truncated input"))?
            .try_into()
            .map_err(|_| Error::Invalid("word length"))?,
    ))
}
pub fn decode(b: &[u8], cancelled: impl Fn() -> bool) -> Result<Vec<Variant>, Error> {
    if b.len() > 16 * 1024 * 1024 {
        return Err(Error::LimitExceeded("input bytes"));
    }
    if b.get(..4) != Some(b"Xcur") {
        return Err(Error::Unsupported("not Xcursor"));
    }
    let header = word(b, 4)? as usize;
    let count = word(b, 12)? as usize;
    if header < 16 {
        return Err(Error::Invalid("file header"));
    }
    if word(b, 8)? >> 16 != 1 {
        return Err(Error::Unsupported("file version"));
    }
    if count > 4096 {
        return Err(Error::LimitExceeded("TOC entries"));
    }
    let end = header
        .checked_add(
            count
                .checked_mul(12)
                .ok_or(Error::Invalid("TOC overflow"))?,
        )
        .ok_or(Error::Invalid("TOC overflow"))?;
    if end > b.len() {
        return Err(Error::Invalid("truncated TOC"));
    }
    let mut groups: BTreeMap<u32, Vec<Frame>> = BTreeMap::new();
    let mut bytes = 0usize;
    let mut frames = 0;
    for i in 0..count {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        let t = header + i * 12;
        if word(b, t)? != 0xfffd0002 {
            continue;
        }
        frames += 1;
        if frames > 2048 {
            return Err(Error::LimitExceeded("frame count"));
        }
        let size = word(b, t + 4)?;
        let p = word(b, t + 8)? as usize;
        let h = word(b, p)? as usize;
        if p < end || h < 36 || word(b, p + 4)? != 0xfffd0002 || word(b, p + 8)? != size {
            return Err(Error::Invalid("image header/TOC mismatch"));
        }
        if word(b, p + 12)? != 1 {
            return Err(Error::Unsupported("image version"));
        }
        let (w, hgt, x, y, delay) = (
            word(b, p + 16)?,
            word(b, p + 20)?,
            word(b, p + 24)?,
            word(b, p + 28)?,
            word(b, p + 32)?,
        );
        if w == 0 || hgt == 0 || size == 0 || x >= w || y >= hgt {
            return Err(Error::Invalid("dimensions or hotspot"));
        }
        if w > 1024 || hgt > 1024 {
            return Err(Error::LimitExceeded("frame dimensions"));
        }
        let n = (w as usize)
            .checked_mul(hgt as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(Error::LimitExceeded("pixel overflow"))?;
        bytes = bytes
            .checked_add(n)
            .ok_or(Error::LimitExceeded("output overflow"))?;
        if bytes > 64 * 1024 * 1024 {
            return Err(Error::LimitExceeded("decoded bytes"));
        }
        let start = p.checked_add(h).ok_or(Error::Invalid("offset overflow"))?;
        let finish = start
            .checked_add(n)
            .ok_or(Error::Invalid("offset overflow"))?;
        let raw = b
            .get(start..finish)
            .ok_or(Error::Invalid("truncated pixels"))?;
        let mut rgba = Vec::with_capacity(n);
        for pixel in raw.as_chunks::<4>().0 {
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
        groups.entry(size).or_default().push(Frame {
            width: w,
            height: hgt,
            hotspot: (x, y),
            delay,
            rgba: rgba.into(),
        });
    }
    if groups.is_empty() {
        return Err(Error::Missing);
    }
    Ok(groups
        .into_iter()
        .map(|(nominal, frames)| Variant { nominal, frames })
        .collect())
}
pub fn metadata(s: &str) -> (Option<String>, Vec<ThemeName>) {
    let mut section = false;
    let mut name = None;
    let mut inherits = Vec::new();
    for l in s.lines().map(str::trim) {
        if l.starts_with('[') {
            section = l == "[Icon Theme]";
            continue;
        }
        if !section || l.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            match k.trim() {
                "Name" => name = Some(v.trim().to_owned()),
                "Inherits" if inherits.is_empty() => {
                    inherits = v
                        .split(|c: char| {
                            c == ',' || c == ';' || c == ':' || c.is_ascii_whitespace()
                        })
                        .filter_map(|s| ThemeName::new(s.trim()).ok())
                        .take(32)
                        .collect()
                }
                _ => (),
            }
        }
    }
    (name, inherits)
}
#[cfg(test)]
mod tests {
    use super::*;
    pub fn sample() -> Vec<u8> {
        let mut b = b"Xcur".to_vec();
        for n in [
            16u32, 65536, 1, 0xfffd0002, 24, 28, 36, 0xfffd0002, 24, 1, 2, 1, 1, 0, 0,
        ] {
            b.extend(n.to_le_bytes())
        }
        b.extend([20, 40, 60, 128, 0, 0, 0, 0]);
        b
    }
    #[test]
    fn colors_and_non_square() {
        let v = decode(&sample(), || false).unwrap();
        assert_eq!(&*v[0].frames[0].rgba, &[60, 40, 20, 128, 0, 0, 0, 0]);
        assert_eq!(v[0].frames[0].hotspot, (1, 0));
        assert_eq!(v[0].frames[0].playback_delay(), 16);
    }
    #[test]
    fn truncations() {
        let b = sample();
        for n in 0..b.len() {
            assert!(decode(&b[..n], || false).is_err())
        }
    }
    #[test]
    fn names() {
        for s in ["", "..", "a/b", "a\\b", "a\0"] {
            assert!(ThemeName::new(s).is_err())
        }
    }
    #[test]
    fn limits_and_cancel() {
        let mut b = sample();
        b[12..16].copy_from_slice(&4097u32.to_le_bytes());
        assert!(matches!(decode(&b, || false), Err(Error::LimitExceeded(_))));
        assert_eq!(decode(&sample(), || true).unwrap_err(), Error::Cancelled);
    }
    #[test]
    fn malformed_never_panics() {
        let mut seed = 42u32;
        for len in 0..512 {
            let mut b = vec![0; len];
            for v in &mut b {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                *v = (seed >> 24) as u8;
            }
            let _ = decode(&b, || false);
        }
    }
}
