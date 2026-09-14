use cursormochi_core::*;
const MOTION: &[u8] = include_bytes!("../../../tests/fixtures/Mochi-Motion/cursors/left_ptr");
const STATIC: &[u8] = include_bytes!("../../../tests/fixtures/Mochi-Light/cursors/left_ptr");
#[test]
fn golden_animation() {
    let v = decode(MOTION, || false).unwrap();
    let v = &v[0];
    assert_eq!(
        v.frames.iter().map(|f| f.delay).collect::<Vec<_>>(),
        [80, 160, 240, 0]
    );
    assert_eq!(&v.frames[1].rgba[..4], &[0x60, 0x40, 0x20, 0x80]);
    assert_eq!(v.frames[1].hotspot, (1, 2));
    for (time, index) in [
        (0, 0),
        (79, 0),
        (80, 1),
        (239, 1),
        (240, 2),
        (480, 3),
        (496, 0),
        (496 * 1000 + 240, 2),
    ] {
        assert_eq!(v.frame_at(time), index);
    }
}
#[test]
fn multiple_nominal_sizes() {
    let v = decode(STATIC, || false).unwrap();
    assert_eq!(v.iter().map(|v| v.nominal).collect::<Vec<_>>(), [24, 32]);
    assert_eq!(v[0].frames[0].width, 18);
}
#[test]
fn hostile_offsets_dimensions_hotspots() {
    for (offset, value) in [
        (4, 15),
        (24, u32::MAX),
        (28, 35),
        (44, 0),
        (48, u32::MAX),
        (52, 18),
        (56, 24),
    ] {
        let mut b = MOTION.to_vec(); // Motion's first chunk starts at 64, not 28. Use static single-index transformations below.
        b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        let _ = decode(&b, || false);
    }
    let mut b = STATIC.to_vec();
    let p = u32::from_le_bytes(b[24..28].try_into().unwrap()) as usize;
    for (field, value) in [
        (16, 0u32),
        (16, 1025),
        (20, u32::MAX),
        (24, 18),
        (28, 24),
        (0, u32::MAX),
    ] {
        let mut bad = b.clone();
        bad[p + field..p + field + 4].copy_from_slice(&value.to_le_bytes());
        assert!(decode(&bad, || false).is_err())
    }
    b[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode(&b, || false).is_err());
}
#[test]
fn repeated_toc_bounded_before_amplification() {
    let n = 2049u32;
    let start = 16 + 12 * n;
    let mut b = b"Xcur".to_vec();
    for v in [16, 65536, n] {
        b.extend(v.to_le_bytes())
    }
    for _ in 0..n {
        for v in [0xfffd0002, 24, start] {
            b.extend(v.to_le_bytes())
        }
    }
    for v in [36u32, 0xfffd0002, 24, 1, 1, 1, 0, 0, 20, 0xffffffff] {
        b.extend(v.to_le_bytes())
    }
    assert!(matches!(
        decode(&b, || false),
        Err(Error::LimitExceeded("frame count"))
    ))
}
#[test]
fn duplicate_large_pixels_hit_total_budget() {
    let n = 17u32;
    let start = 16 + 12 * n;
    let mut b = b"Xcur".to_vec();
    for v in [16, 65536, n] {
        b.extend(v.to_le_bytes())
    }
    for _ in 0..n {
        for v in [0xfffd0002, 1024, start] {
            b.extend(v.to_le_bytes())
        }
    }
    for v in [36u32, 0xfffd0002, 1024, 1, 1024, 1024, 0, 0, 20] {
        b.extend(v.to_le_bytes())
    }
    b.resize(b.len() + 1024 * 1024 * 4, 0);
    assert!(matches!(
        decode(&b, || false),
        Err(Error::LimitExceeded("decoded bytes"))
    ));
}
