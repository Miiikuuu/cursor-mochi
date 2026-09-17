//! Standard Xcursor writer. Input is the shared premultiplied preview model.
use crate::{Error, Variant};
/// Desktop variants cover common logical sizes and their 2× backing buffers.
/// Existing native variants are retained unchanged; additions are resampled.
pub const DESKTOP_SIZES: &[u32] = &[16, 24, 32, 48, 64, 96, 128];
pub fn desktop_variants(
    source: &[Variant],
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<Variant>, Error> {
    if source.is_empty() {
        return Err(Error::Missing);
    }
    let native_sizes = source
        .iter()
        .map(|v| v.nominal)
        .collect::<std::collections::BTreeSet<_>>();
    if native_sizes.len() != source.len()
        || source.iter().any(|v| v.nominal == 0 || v.nominal > 256)
    {
        return Err(Error::Invalid("desktop source sizes"));
    }
    let mut output = Vec::new();
    let mut bytes = 0usize;
    let mut frames = 0usize;
    let mut sizes = std::collections::BTreeSet::new();
    for size in source
        .iter()
        .map(|v| v.nominal)
        .chain(DESKTOP_SIZES.iter().copied())
    {
        if !sizes.insert(size) {
            continue;
        }
        let v = source
            .iter()
            .min_by_key(|v| (v.nominal.abs_diff(size), std::cmp::Reverse(v.nominal)))
            .ok_or(Error::Missing)?;
        frames += v.frames.len();
        if frames > 2048 {
            return Err(Error::LimitExceeded("desktop export frames"));
        }
        let v = crate::resize::variant(
            v,
            size,
            (16 * 1024 * 1024usize).saturating_sub(bytes),
            cancel,
        )?;
        bytes += v.frames.iter().map(|f| f.rgba.len()).sum::<usize>();
        output.push(v);
    }
    output.sort_by_key(|v| v.nominal);
    Ok(output)
}
pub fn xcursor(variants: &[Variant]) -> Result<Vec<u8>, Error> {
    let count: usize = variants.iter().map(|v| v.frames.len()).sum();
    if count == 0 || count > 2048 {
        return Err(Error::LimitExceeded("output frames"));
    }
    let mut total = 16 + count * 12;
    let mut sizes = std::collections::BTreeSet::new();
    for v in variants {
        if v.nominal == 0 || v.frames.is_empty() || !sizes.insert(v.nominal) {
            return Err(Error::Invalid("output variants"));
        }
        for f in &v.frames {
            if f.width == 0
                || f.height == 0
                || f.width > 256
                || f.height > 256
                || f.hotspot.0 >= f.width
                || f.hotspot.1 >= f.height
                || f.rgba.len() != f.width as usize * f.height as usize * 4
            {
                return Err(Error::Invalid("output frame"));
            }
            total = total
                .checked_add(36 + f.rgba.len())
                .ok_or(Error::LimitExceeded("output bytes"))?;
        }
    }
    if total > 16 * 1024 * 1024 {
        return Err(Error::LimitExceeded("Xcursor output bytes"));
    }
    let mut b = Vec::with_capacity(total);
    let put = |b: &mut Vec<u8>, n: u32| b.extend(n.to_le_bytes());
    b.extend(b"Xcur");
    for n in [16, 65536, count as u32] {
        put(&mut b, n);
    }
    let mut offset = 16 + count * 12;
    for v in variants {
        for f in &v.frames {
            for n in [0xfffd0002, v.nominal, offset as u32] {
                put(&mut b, n);
            }
            offset += 36 + f.rgba.len();
        }
    }
    for v in variants {
        for f in &v.frames {
            for n in [
                36,
                0xfffd0002,
                v.nominal,
                1,
                f.width,
                f.height,
                f.hotspot.0,
                f.hotspot.1,
                f.delay,
            ] {
                put(&mut b, n);
            }
            for p in f.rgba.as_chunks::<4>().0 {
                b.extend([p[2], p[1], p[0], p[3]]);
            }
        }
    }
    let actual = crate::decode(&b, || false)?;
    if !equivalent(variants, &actual) {
        return Err(Error::Invalid("Xcursor roundtrip verification"));
    }
    Ok(b)
}
pub fn equivalent(a: &[Variant], b: &[Variant]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.nominal == b.nominal
                && a.frames.len() == b.frames.len()
                && a.frames.iter().zip(&b.frames).all(|(a, b)| {
                    (a.width, a.height, a.hotspot, a.delay)
                        == (b.width, b.height, b.hotspot, b.delay)
                        && a.rgba == b.rgba
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;
    #[test]
    fn native_variants_win_and_generated_variants_keep_sequence_and_limits() {
        let v = Variant {
            nominal: 32,
            frames: vec![Frame {
                width: 32,
                height: 16,
                hotspot: (31, 15),
                delay: 123,
                rgba: vec![128; 32 * 16 * 4].into(),
            }],
        };
        let result = desktop_variants(std::slice::from_ref(&v), &|| false).unwrap();
        assert_eq!(
            result.iter().map(|v| v.nominal).collect::<Vec<_>>(),
            DESKTOP_SIZES
        );
        let same = result.iter().find(|v| v.nominal == 32).unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &v.frames[0].rgba,
            &same.frames[0].rgba
        ));
        for r in &result {
            assert_eq!(r.frames[0].delay, 123);
        }
        assert!(matches!(
            desktop_variants(&[v.clone(), v.clone()], &|| false),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            desktop_variants(std::slice::from_ref(&v), &|| true),
            Err(Error::Cancelled)
        ));
        let mut big = v;
        big.frames = vec![big.frames[0].clone(); 257];
        assert!(matches!(
            desktop_variants(&[big], &|| false),
            Err(Error::LimitExceeded(_))
        ));
    }
}
