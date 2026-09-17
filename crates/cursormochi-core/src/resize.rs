//! Bounded resampling of the shared premultiplied frame model, not a decoder.
use crate::{Error, Frame, Variant};

pub fn variant(
    source: &Variant,
    size: u32,
    budget: usize,
    cancel: &dyn Fn() -> bool,
) -> Result<Variant, Error> {
    if source.nominal == 0 || size == 0 || size > 256 || source.frames.is_empty() {
        return Err(Error::Invalid("resize variant"));
    }
    if source.frames.len() > 256 {
        return Err(Error::LimitExceeded("resize animation frames"));
    }
    let scaled = |n: u32| {
        (u64::from(n) * u64::from(size) + u64::from(source.nominal) / 2) / u64::from(source.nominal)
    };
    let mut layouts = Vec::new();
    let mut bytes = 0usize;
    // Validate and reserve the entire animation budget before pixel allocation.
    for f in &source.frames {
        if cancel() {
            return Err(Error::Cancelled);
        }
        if f.width == 0
            || f.height == 0
            || f.width > 1024
            || f.height > 1024
            || f.hotspot.0 >= f.width
            || f.hotspot.1 >= f.height
            || u64::from(f.width) * u64::from(f.height) * 4 != f.rgba.len() as u64
        {
            return Err(Error::Invalid("resize frame pixels or hotspot"));
        }
        let (w, h) = (scaled(f.width).max(1), scaled(f.height).max(1));
        if w > 256 || h > 256 {
            return Err(Error::LimitExceeded("resized dimensions"));
        }
        bytes += (w * h * 4) as usize;
        if bytes > budget.min(16 * 1024 * 1024) {
            return Err(Error::LimitExceeded("resized animation pixels"));
        }
        layouts.push((w as u32, h as u32));
    }
    if source.nominal == size {
        return Ok(source.clone());
    }
    let mut frames = Vec::with_capacity(source.frames.len());
    for (f, (w, h)) in source.frames.iter().zip(layouts) {
        let mut rgba = vec![0; (w * h * 4) as usize];
        // Exact integer area weights avoid transparent-edge color fringes.
        // Integer upscaling preserves source pixels; reduction averages areas.
        let area = u64::from(f.width) * u64::from(f.height);
        for y in 0..h {
            if cancel() {
                return Err(Error::Cancelled);
            }
            let (y0, y1) = (y * f.height, (y + 1) * f.height);
            for x in 0..w {
                let (x0, x1) = (x * f.width, (x + 1) * f.width);
                let mut sum = [0u64; 4];
                for sy in y0 / h..y1.div_ceil(h) {
                    let wy = y1.min((sy + 1) * h) - y0.max(sy * h);
                    for sx in x0 / w..x1.div_ceil(w) {
                        let wx = x1.min((sx + 1) * w) - x0.max(sx * w);
                        let offset = ((sy * f.width + sx) * 4) as usize;
                        for (channel, total) in sum.iter_mut().enumerate() {
                            *total +=
                                u64::from(f.rgba[offset + channel]) * u64::from(wx) * u64::from(wy);
                        }
                    }
                }
                let offset = ((y * w + x) * 4) as usize;
                for (channel, total) in sum.iter().enumerate() {
                    rgba[offset + channel] = ((total + area / 2) / area) as u8;
                }
            }
        }
        frames.push(Frame {
            width: w,
            height: h,
            hotspot: (
                scaled(f.hotspot.0).min(u64::from(w - 1)) as u32,
                scaled(f.hotspot.1).min(u64::from(h - 1)) as u32,
            ),
            delay: f.delay,
            rgba: rgba.into(),
        });
    }
    Ok(Variant {
        nominal: size,
        frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Variant {
        Variant {
            nominal: 32,
            frames: vec![Frame {
                width: 4,
                height: 2,
                hotspot: (3, 1),
                delay: 73,
                rgba: [vec![128, 0, 0, 128], vec![0, 0, 0, 0]]
                    .concat()
                    .repeat(4)
                    .into(),
            }],
        }
    }
    #[test]
    fn area_filter_preserves_premultiplied_alpha_aspect_and_hotspot() {
        let v = variant(&source(), 16, 100, &|| false).unwrap();
        let f = &v.frames[0];
        assert_eq!((f.width, f.height, f.hotspot, f.delay), (2, 1, (1, 0), 73));
        assert_eq!(&*f.rgba, &[64, 0, 0, 64, 64, 0, 0, 64]);
        let v = variant(&source(), 64, 1000, &|| false).unwrap();
        assert_eq!(
            (v.frames[0].width, v.frames[0].height, v.frames[0].hotspot),
            (8, 4, (6, 2))
        );
        assert_eq!(&v.frames[0].rgba[..8], &[128, 0, 0, 128, 128, 0, 0, 128]);
    }
    #[test]
    fn exact_size_reuses_pixels_and_limits_and_cancellation_are_enforced() {
        let v = source();
        let same = variant(&v, 32, 100, &|| false).unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &v.frames[0].rgba,
            &same.frames[0].rgba
        ));
        assert!(matches!(
            variant(&v, 32, 1, &|| false),
            Err(Error::LimitExceeded(_))
        ));
        assert_eq!(
            variant(&v, 32, 100, &|| true).unwrap_err(),
            Error::Cancelled
        );
        let mut bad = v.clone();
        bad.frames[0].hotspot.0 = 4;
        assert!(matches!(
            variant(&bad, 32, 100, &|| false),
            Err(Error::Invalid(_))
        ));
        bad = v;
        bad.nominal = 1;
        assert!(matches!(
            variant(&bad, 256, 100, &|| false),
            Err(Error::LimitExceeded(_))
        ));
    }
}
