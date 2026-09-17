use cursormochi_core::{Error, export, windows};
const CUR: &[u8] = include_bytes!("../../../tests/import-fixtures/normal.cur");
const ANI: &[u8] = include_bytes!("../../../tests/import-fixtures/busy.ani");
#[test]
fn bmp_png_sizes_alpha_hotspot_and_independent_expected_pixels() {
    for bytes in [
        CUR,
        include_bytes!("../../../tests/import-fixtures/text.cur"),
    ] {
        let d = windows::decode(bytes, &|| false).unwrap();
        assert_eq!(
            d.variants.iter().map(|v| v.nominal).collect::<Vec<_>>(),
            [3, 6]
        );
        let f = &d.variants[0].frames[0];
        assert_eq!((f.width, f.height, f.hotspot), (3, 2, (1, 1)));
        assert_eq!(&f.rgba[..8], &[100, 50, 25, 128, 0, 0, 0, 0]);
        let output = export::xcursor(&d.variants).unwrap();
        assert!(export::equivalent(
            &d.variants,
            &cursormochi_core::decode(&output, || false).unwrap()
        ));
    }
}
#[test]
fn ani_reordered_repeated_frames_rates_and_roundtrip() {
    let d = windows::decode(ANI, &|| false).unwrap();
    for v in &d.variants {
        assert_eq!(
            v.frames.iter().map(|f| f.delay).collect::<Vec<_>>(),
            [17, 50, 100]
        );
        assert_eq!(
            v.frames.iter().map(|f| f.rgba[0]).collect::<Vec<_>>(),
            [20, 100, 20]
        );
        assert!(v.frames.iter().all(|f| f.hotspot == (1, 1)));
    }
    assert!(d.notes.iter().any(|n| n.contains("rounded to 17")));
    let output = export::xcursor(&d.variants).unwrap();
    assert!(export::equivalent(
        &d.variants,
        &cursormochi_core::decode(&output, || false).unwrap()
    ));
}
#[test]
fn all_truncations_and_malformed_headers_fail_without_panics() {
    for b in [CUR, ANI] {
        for n in 0..b.len() {
            assert!(windows::decode(&b[..n], &|| false).is_err(), "{n}");
        }
        for n in 0..b.len() {
            let mut damaged = b.to_vec();
            damaged[n] ^= 255;
            let _ = windows::decode(&damaged, &|| false);
        }
    }
    let mut png = include_bytes!("../../../tests/import-fixtures/text.cur").to_vec();
    let end = png.windows(4).position(|w| w == b"IEND").unwrap();
    png[end..end + 4].copy_from_slice(b"IDAT");
    assert!(matches!(
        windows::decode(&png, &|| false),
        Err(Error::Invalid("missing PNG IEND"))
    ));
    assert_eq!(
        windows::decode(CUR, &|| true).unwrap_err(),
        Error::Cancelled
    );
}
#[test]
fn unsupported_bitmap_and_corrupt_sequence_are_not_static_success() {
    let mut b = CUR.to_vec();
    b[38 + 14..38 + 16].copy_from_slice(&1u16.to_le_bytes());
    assert!(matches!(
        windows::decode(&b, &|| false),
        Err(Error::Unsupported(_))
    ));
    let mut b = ANI.to_vec();
    let p = b.windows(4).position(|v| v == b"seq ").unwrap() + 8;
    b[p..p + 4].copy_from_slice(&9u32.to_le_bytes());
    assert!(windows::decode(&b, &|| false).is_err());
    let mut b = CUR.to_vec();
    b[18..22].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(windows::decode(&b, &|| false).is_err());
}

#[test]
fn rgb24_and_mask_is_transparent_but_screen_xor_is_unsupported() {
    let mut b = vec![0, 0, 2, 0, 1, 0, 2, 1, 0, 0, 1, 0, 0, 0];
    b.extend(52u32.to_le_bytes());
    b.extend(22u32.to_le_bytes());
    b.extend(40u32.to_le_bytes());
    b.extend(2u32.to_le_bytes());
    b.extend(2u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(24u16.to_le_bytes());
    b.extend([0; 24]);
    b.extend([50, 100, 200, 0, 0, 0, 0, 0]);
    b.extend([0x40, 0, 0, 0]);
    let d = windows::decode(&b, &|| false).unwrap();
    assert_eq!(
        &*d.variants[0].frames[0].rgba,
        &[200, 100, 50, 255, 0, 0, 0, 0]
    );
    b[65] = 255;
    assert!(matches!(
        windows::decode(&b, &|| false),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn ani_timing_and_preallocation_limits_fail_explicitly() {
    let mut b = ANI.to_vec();
    let rate = b.windows(4).position(|w| w == b"rate").unwrap() + 8;
    for delay in [0, 601, u32::MAX] {
        b[rate..rate + 4].copy_from_slice(&delay.to_le_bytes());
        assert!(matches!(
            windows::decode(&b, &|| false),
            Err(Error::Unsupported(_))
        ));
    }
    let mut b = ANI.to_vec();
    let h = b.windows(4).position(|w| w == b"anih").unwrap() + 8;
    b[h + 8..h + 12].copy_from_slice(&257u32.to_le_bytes());
    assert!(matches!(
        windows::decode(&b, &|| false),
        Err(Error::LimitExceeded(_))
    ));
    let mut b = CUR.to_vec();
    b[4..6].copy_from_slice(&33u16.to_le_bytes());
    assert!(matches!(
        windows::decode(&b, &|| false),
        Err(Error::LimitExceeded(_))
    ));
    assert!(matches!(
        windows::decode(&vec![0; windows::MAX_INPUT + 1], &|| false),
        Err(Error::LimitExceeded(_))
    ));
}

#[test]
fn ani_header_inclusive_length_and_consistent_image_hints() {
    // Original synthetic fixture, reproducing the header conventions of the local
    // Miku package without including or redistributing any third-party artwork.
    let mut b = ANI.to_vec();
    let h = b.windows(4).position(|w| w == b"anih").unwrap() + 8;
    // Both synthetic size variants use 32-bit BMP and a single plane.
    b[h + 20..h + 24].copy_from_slice(&32u32.to_le_bytes());
    b[h + 24..h + 28].copy_from_slice(&1u32.to_le_bytes());
    let size = b.len() as u32;
    b[4..8].copy_from_slice(&size.to_le_bytes());
    let actual = windows::decode(&b, &|| false).unwrap();
    let expected = windows::decode(ANI, &|| false).unwrap();
    assert!(export::equivalent(&actual.variants, &expected.variants));
    assert!(actual.notes.iter().any(|n| n.contains("8-byte")));
    // Compatibility must not admit a genuinely truncated last frame.
    assert!(windows::decode(&b[..b.len() - 8], &|| false).is_err());
    b[h + 12..h + 16].copy_from_slice(&160u32.to_le_bytes());
    assert!(
        windows::decode(&b, &|| false).is_err(),
        "conflicting width hint"
    );
}

#[test]
fn explicit_160_pixel_ani_hints_preserve_geometry_hotspots_and_sequence() {
    let b = include_bytes!("../../../tests/ani-compat/busy.ani");
    let d = windows::decode(b, &|| false).unwrap();
    assert_eq!(d.variants.len(), 1);
    let v = &d.variants[0];
    assert_eq!(v.nominal, 160);
    assert_eq!(v.frames.len(), 3);
    for f in &v.frames {
        assert_eq!((f.width, f.height, f.hotspot), (160, 160, (1, 1)));
    }
    assert_eq!(
        v.frames
            .iter()
            .map(|f| (f.rgba[0], f.delay))
            .collect::<Vec<_>>(),
        [(20, 17), (100, 50), (20, 100)]
    );
    for end in [b.len() - 1, b.len() - 8, b.len() - 16] {
        assert!(windows::decode(&b[..end], &|| false).is_err());
    }
    let mut bad = b.to_vec();
    let h = bad.windows(4).position(|w| w == b"anih").unwrap() + 8;
    for (offset, value) in [(12, 159u32), (16, 159), (20, 24), (24, 2)] {
        bad.copy_from_slice(b);
        bad[h + offset..h + offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(windows::decode(&bad, &|| false).is_err());
    }
}
