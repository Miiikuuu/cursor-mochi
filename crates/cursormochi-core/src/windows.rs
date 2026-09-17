//! Bounded CUR/ANI container adapter. Pixel decoding belongs to the MIT `ico` crate.
use crate::{Error, Frame, Variant, word};
use std::{collections::BTreeMap, io::Cursor};
pub const MAX_INPUT: usize = 16 * 1024 * 1024;
pub const MAX_PIXELS: usize = 32 * 1024 * 1024;
pub const MAX_FRAMES: usize = 256;
#[derive(Clone, Debug)]
pub struct Decoded {
    /// Premultiplied RGBA, matching the existing Xcursor preview model.
    pub variants: Vec<Variant>,
    pub notes: Vec<String>,
}
fn half(b: &[u8], o: usize) -> Result<u16, Error> {
    let s = b.get(o..o + 2).ok_or(Error::Invalid("truncated CUR"))?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}
fn invalid(e: impl std::fmt::Display) -> Error {
    Error::Io(format!("CUR decoder: {e}"))
}
/// Inspect payload sizes before passing any bytes to third-party allocation code.
fn preflight(b: &[u8]) -> Result<usize, Error> {
    if half(b, 0)? != 0 || half(b, 2)? != 2 {
        return Err(Error::Unsupported(
            "expected CUR, including inside ANI; ICO has no hotspot",
        ));
    }
    let count = half(b, 4)? as usize;
    if count == 0 || count > 32 {
        return Err(Error::LimitExceeded("CUR variants: 1–32"));
    }
    let mut decoded = 0;
    let mut encoded = 0;
    let mut sizes = std::collections::BTreeSet::new();
    for i in 0..count {
        let o = 6 + i * 16;
        let entry = b.get(o..o + 16).ok_or(Error::Invalid("CUR directory"))?;
        let w = if entry[0] == 0 {
            256
        } else {
            u32::from(entry[0])
        };
        let h = if entry[1] == 0 {
            256
        } else {
            u32::from(entry[1])
        };
        if !sizes.insert(w.max(h)) {
            return Err(Error::Unsupported(
                "ambiguous variants with the same nominal size",
            ));
        }
        if u32::from(half(entry, 4)?) >= w || u32::from(half(entry, 6)?) >= h {
            return Err(Error::Invalid("CUR hotspot outside image"));
        }
        let offset = word(entry, 12)? as usize;
        let len = word(entry, 8)? as usize;
        encoded += len;
        decoded += (w * h * 4) as usize;
        if encoded > MAX_INPUT || decoded > MAX_PIXELS {
            return Err(Error::LimitExceeded("CUR allocation budget"));
        }
        if offset < 6 + count * 16 {
            return Err(Error::Invalid("payload overlaps CUR directory"));
        }
        let p = b
            .get(offset..offset.checked_add(len).ok_or(Error::Invalid("CUR span"))?)
            .ok_or(Error::Invalid("CUR payload span"))?;
        if p.starts_with(b"\x89PNG\r\n\x1a\n") {
            let mut pos = 8;
            let mut first = true;
            let mut ended = false;
            while pos < p.len() {
                let hdr = p.get(pos..pos + 8).ok_or(Error::Invalid("PNG chunk"))?;
                let n = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) as usize;
                let end = pos.checked_add(n + 12).ok_or(Error::Invalid("PNG span"))?;
                let chunk = p.get(pos + 8..end).ok_or(Error::Invalid("PNG length"))?;
                if !first && &hdr[4..8] == b"IHDR" {
                    return Err(Error::Invalid("duplicate PNG IHDR"));
                }
                if &hdr[4..8] == b"IEND" {
                    if n != 0 || end != p.len() {
                        return Err(Error::Invalid("PNG IEND/trailing data"));
                    }
                    ended = true;
                }
                if first {
                    if &hdr[4..8] != b"IHDR" || n != 13 {
                        return Err(Error::Invalid("PNG IHDR"));
                    }
                    let pw = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    let ph = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                    if (pw, ph) != (w, h) {
                        return Err(Error::Invalid("PNG dimensions disagree with CUR"));
                    }
                    if chunk[8..13] != [8, 6, 0, 0, 0] {
                        return Err(Error::Unsupported("PNG requires RGBA8, noninterlaced"));
                    }
                    first = false;
                }
                // Reject animation and compressed ancillary metadata before png allocates it.
                if !matches!(
                    &hdr[4..8],
                    b"IHDR" | b"IDAT" | b"IEND" | b"sRGB" | b"gAMA" | b"cHRM" | b"pHYs"
                ) {
                    return Err(Error::Unsupported(
                        "PNG chunk (APNG, profiles and text are not supported)",
                    ));
                }
                pos = end;
            }
            if !ended {
                return Err(Error::Invalid("missing PNG IEND"));
            }
        } else {
            if word(p, 0)? != 40 || word(p, 4)? != w || word(p, 8)? != h * 2 || half(p, 12)? != 1 {
                return Err(Error::Unsupported(
                    "BMP requires bottom-up BITMAPINFOHEADER matching CUR dimensions",
                ));
            }
            let bits = half(p, 14)?;
            if !matches!(bits, 24 | 32) || word(p, 16)? != 0 || word(p, 32)? != 0 {
                return Err(Error::Unsupported(
                    "BMP supports only uncompressed 24/32-bit without palette",
                ));
            }
            let stride = (w as usize * bits as usize).div_ceil(32) * 4;
            let mask_stride = (w as usize).div_ceil(32) * 4;
            let mask_start = 40 + stride * h as usize;
            if p.len() != mask_start + mask_stride * h as usize {
                return Err(Error::Invalid("BMP pixels/AND mask length"));
            }
            let has_alpha = bits == 32
                && (0..h as usize)
                    .any(|y| (0..w as usize).any(|x| p[40 + y * stride + x * 4 + 3] != 0));
            if bits == 32 && !has_alpha {
                return Err(Error::Unsupported(
                    "32-bit legacy zero-alpha AND/XOR cursor",
                ));
            }
            for y in 0..h as usize {
                for x in 0..w as usize {
                    if p[mask_start + y * mask_stride + x / 8] & (0x80 >> (x % 8)) != 0 {
                        let pixel = 40 + y * stride + x * (bits as usize / 8);
                        if (bits == 32 && p[pixel + 3] != 0)
                            || (bits == 24 && p[pixel..pixel + 3] != [0, 0, 0])
                        {
                            return Err(Error::Unsupported(
                                "screen-dependent AND/XOR or conflicting alpha mask",
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(decoded)
}
fn cur(b: &[u8], cancel: &dyn Fn() -> bool, budget: &mut usize) -> Result<Vec<Variant>, Error> {
    let n = preflight(b)?;
    *budget = budget
        .checked_add(n)
        .ok_or(Error::LimitExceeded("decoded pixels"))?;
    if *budget > MAX_PIXELS {
        return Err(Error::LimitExceeded("decoded pixels"));
    }
    let dir = ico::IconDir::read(Cursor::new(b)).map_err(invalid)?;
    let mut variants = Vec::new();
    for e in dir.entries() {
        if cancel() {
            return Err(Error::Cancelled);
        }
        let img = e.decode().map_err(invalid)?;
        let hotspot = e
            .cursor_hotspot()
            .ok_or(Error::Invalid("missing CUR hotspot"))?;
        let mut rgba = img.rgba_data().to_vec();
        for p in rgba.as_chunks_mut::<4>().0 {
            for c in 0..3 {
                p[c] = ((u16::from(p[c]) * u16::from(p[3]) + 127) / 255) as u8;
            }
        }
        variants.push(Variant {
            nominal: img.width().max(img.height()),
            frames: vec![Frame {
                width: img.width(),
                height: img.height(),
                hotspot: (u32::from(hotspot.0), u32::from(hotspot.1)),
                delay: 0,
                rgba: rgba.into(),
            }],
        });
    }
    variants.sort_by_key(|v| v.nominal);
    Ok(variants)
}
type Chunk<'a> = (&'a [u8], &'a [u8]);
fn chunks(b: &[u8]) -> Result<Vec<Chunk<'_>>, Error> {
    let mut out = Vec::new();
    let mut o = 0;
    while o < b.len() {
        if out.len() >= 1024 {
            return Err(Error::LimitExceeded("RIFF chunks"));
        }
        let id = b.get(o..o + 4).ok_or(Error::Invalid("RIFF chunk id"))?;
        let n = word(b, o + 4)? as usize;
        let end = (o + 8).checked_add(n).ok_or(Error::Invalid("RIFF span"))?;
        out.push((
            id,
            b.get(o + 8..end).ok_or(Error::Invalid("RIFF chunk span"))?,
        ));
        o = end + (n % 2);
        if o > b.len() {
            return Err(Error::Invalid("RIFF padding"));
        }
    }
    Ok(out)
}
pub fn decode(b: &[u8], cancel: &dyn Fn() -> bool) -> Result<Decoded, Error> {
    if cancel() {
        return Err(Error::Cancelled);
    }
    if b.len() > MAX_INPUT {
        return Err(Error::LimitExceeded("CUR/ANI input"));
    }
    let mut budget = 0;
    let mut notes = vec!["Nominal size is max(width, height). RGBA is premultiplied to Xcursor ARGB; partial-alpha colors round to 8 bits and hidden RGB is discarded. PNG gamma metadata is not color-managed.".into()];
    if !b.starts_with(b"RIFF") {
        return Ok(Decoded {
            variants: cur(b, cancel, &mut budget)?,
            notes,
        });
    }
    if b.get(8..12) != Some(b"ACON") {
        return Err(Error::Invalid("ANI RIFF/ACON length"));
    }
    let declared = word(b, 4)? as usize;
    if declared == b.len() {
        // Some writers include the RIFF header itself in this field. Only this
        // exact discrepancy is allowed; every nested chunk still has to fit and
        // consume the actual envelope. Never pad or synthesize missing data.
        notes.push("ANI compatibility: RIFF length includes the 8-byte header; all actual chunks and frames were validated without padding or dropping data.".into());
    } else if declared != b.len() - 8 {
        return Err(Error::Invalid("ANI RIFF/ACON length"));
    }
    let mut header = None;
    let mut rates = None;
    let mut seq = None;
    let mut icons = Vec::new();
    let mut fram = false;
    for (id, p) in chunks(&b[12..])? {
        match id {
            b"anih" if header.is_none() => header = Some(p),
            b"rate" if rates.is_none() => rates = Some(p),
            b"seq " if seq.is_none() => seq = Some(p),
            b"LIST" if p.starts_with(b"fram") && !fram => {
                fram = true;
                for (kind, data) in chunks(&p[4..])? {
                    if kind != b"icon" {
                        return Err(Error::Unsupported("ANI frame chunk"));
                    }
                    icons.push(data);
                }
            }
            b"LIST" if p.starts_with(b"INFO") => {
                let _ = chunks(&p[4..])?;
            }
            b"JUNK" => (),
            _ => return Err(Error::Unsupported("unknown or duplicate ANI chunk")),
        }
    }
    let h = header.ok_or(Error::Invalid("missing ANI header"))?;
    if h.len() != 36 || word(h, 0)? != 36 {
        return Err(Error::Unsupported("ANI header size"));
    }
    let frames = word(h, 4)? as usize;
    let steps = word(h, 8)? as usize;
    let flags = word(h, 32)?;
    if frames == 0 || frames > MAX_FRAMES || steps == 0 || steps > MAX_FRAMES {
        return Err(Error::LimitExceeded("ANI frames/steps: 1–256"));
    }
    if !matches!(flags, 1 | 3) {
        return Err(Error::Unsupported("ANI raw bitmap or flags"));
    }
    let hints = [word(h, 12)?, word(h, 16)?, word(h, 20)?, word(h, 24)?];
    if hints[0] > 256
        || hints[1] > 256
        || !matches!(hints[2], 0 | 24 | 32)
        || !matches!(hints[3], 0 | 1)
    {
        return Err(Error::Unsupported("ANI dimension/depth/plane hints"));
    }
    if hints != [0; 4] {
        notes.push(format!("ANI image hints: {}×{}, {}-bit, {} plane(s); nonzero hints checked against every embedded CUR image. Embedded geometry and hotspots are preserved.", hints[0], hints[1], hints[2], hints[3]));
    }
    if icons.len() != frames || (flags == 3) != seq.is_some() || (flags == 1 && frames != steps) {
        return Err(Error::Invalid("ANI sequence/frame count"));
    }
    for array in [rates, seq].into_iter().flatten() {
        if array.len() != steps * 4 {
            return Err(Error::Invalid("ANI rate/sequence length"));
        }
    }
    let mut decoded = Vec::new();
    for bytes in icons {
        let variants = cur(bytes, cancel, &mut budget)?;
        for v in &variants {
            let f = &v.frames[0];
            if (hints[0] != 0 && hints[0] != f.width) || (hints[1] != 0 && hints[1] != f.height) {
                return Err(Error::Invalid("ANI dimensions disagree with embedded CUR"));
            }
        }
        // cur() already preflighted all entry and payload spans. Check depth
        // hints against the payloads too, not the CUR hotspot directory fields.
        for i in 0..half(bytes, 4)? as usize {
            let offset = word(bytes, 6 + i * 16 + 12)? as usize;
            let payload = &bytes[offset..];
            let depth = if payload.starts_with(b"\x89PNG\r\n\x1a\n") {
                32
            } else {
                u32::from(half(payload, 14)?)
            };
            if hints[2] != 0 && hints[2] != depth {
                return Err(Error::Invalid("ANI depth disagrees with embedded CUR"));
            }
        }
        decoded.push(variants);
    }
    let keys: Vec<_> = decoded[0].iter().map(|v| v.nominal).collect();
    if decoded
        .iter()
        .any(|v| v.iter().map(|v| v.nominal).collect::<Vec<_>>() != keys)
    {
        return Err(Error::Unsupported(
            "ANI frames have different size variants",
        ));
    }
    let mut groups: BTreeMap<u32, Vec<Frame>> = keys.into_iter().map(|k| (k, Vec::new())).collect();
    let mut output = 0;
    for i in 0..steps {
        if cancel() {
            return Err(Error::Cancelled);
        }
        let index = seq
            .map(|s| word(s, i * 4))
            .transpose()?
            .map_or(i, |n| n as usize);
        let source = decoded
            .get(index)
            .ok_or(Error::Invalid("ANI sequence index"))?;
        let jiffies = rates
            .map(|r| word(r, i * 4))
            .transpose()?
            .unwrap_or(word(h, 28)?);
        if jiffies == 0 || jiffies > 600 {
            return Err(Error::Unsupported("ANI delay must be 1–600 jiffies"));
        }
        let delay = (jiffies * 1000 + 30) / 60;
        if jiffies % 3 != 0 {
            notes.push(format!("Step {}: {jiffies}/60 seconds rounded to {delay} ms (Xcursor integer milliseconds).",i+1));
        }
        for v in source {
            let mut f = v.frames[0].clone();
            f.delay = delay;
            output += f.rgba.len();
            if output > MAX_PIXELS {
                return Err(Error::LimitExceeded("expanded ANI sequence pixels"));
            }
            groups
                .get_mut(&v.nominal)
                .ok_or(Error::Invalid("ANI variant"))?
                .push(f);
        }
    }
    Ok(Decoded {
        variants: groups
            .into_iter()
            .map(|(nominal, frames)| Variant { nominal, frames })
            .collect(),
        notes,
    })
}
