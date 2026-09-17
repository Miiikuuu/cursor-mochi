use cursormochi_app::import::{plan, suggestion};
use cursormochi_platform::import::{install, load};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
};
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/import-fixtures")
        .canonicalize()
        .unwrap()
}
fn originals_preserved(
    source: &[cursormochi_core::Variant],
    output: &[cursormochi_core::Variant],
) -> bool {
    let originals = output
        .iter()
        .filter(|v| source.iter().any(|s| s.nominal == v.nominal))
        .cloned()
        .collect::<Vec<_>>();
    cursormochi_core::export::equivalent(source, &originals)
}
fn demo() -> cursormochi_app::import::Plan {
    let p = load(&fixtures().join("theme.zip"), &|| false).unwrap();
    let a = p
        .assets
        .into_iter()
        .map(|a| (suggestion(&a.path).unwrap(), a))
        .collect::<Vec<_>>();
    plan("Imported-Test", &a, true).unwrap()
}
fn zip(path: &Path, entries: &[(&str, &[u8])]) {
    let f = fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    for (name, data) in entries {
        z.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap();
}
#[test]
fn directory_zip_and_individual_file_decode_without_writing_source() {
    let p = load(&fixtures(), &|| false).unwrap();
    assert_eq!(p.assets.len(), 4);
    assert_eq!(p.assets.iter().filter(|a| a.decoded.is_err()).count(), 1);
    let p = load(&fixtures().join("theme.zip"), &|| false).unwrap();
    assert_eq!(p.assets.len(), 3);
    assert_eq!(p.ignored, 2);
    assert_eq!(p.roots(), [PathBuf::from("Sample")]);
    let p = load(&fixtures().join("busy.ani"), &|| false).unwrap();
    assert_eq!(
        p.assets[0].decoded.as_ref().unwrap().variants[0]
            .frames
            .len(),
        3
    );
}
#[test]
fn staged_install_is_create_only_verifies_all_pixels_and_does_not_execute() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("icons");
    let p = demo();
    let result = install(&p, &root, std::slice::from_ref(&root), &|| false).unwrap();
    assert!(result.join("index.theme").is_file());
    assert!(!result.join("install.inf").exists());
    assert!(!result.join("launch.sh").exists());
    for (role, a) in &p.mapped {
        let data = fs::read(result.join("cursors").join(role)).unwrap();
        assert!(originals_preserved(
            &a.decoded.as_ref().unwrap().variants,
            &cursormochi_core::decode(&data, || false).unwrap()
        ));
    }
    fs::write(result.join("sentinel"), "keep").unwrap();
    assert!(install(&p, &root, std::slice::from_ref(&root), &|| false).is_err());
    assert_eq!(fs::read_to_string(result.join("sentinel")).unwrap(), "keep");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert!(
        install(
            &p,
            &t.path().join("unconfirmed"),
            std::slice::from_ref(&root),
            &|| false
        )
        .is_err()
    );
    assert!(!t.path().join("unconfirmed").exists());
}
#[test]
fn large_ani_exports_real_desktop_sizes_with_scaled_hotspots_and_animation() {
    let asset = load(
        &fixtures().parent().unwrap().join("ani-compat/busy.ani"),
        &|| false,
    )
    .unwrap()
    .assets
    .remove(0);
    let original = asset.decoded.as_ref().unwrap().variants.clone();
    let p = plan("Desktop-Sizes", &[(4, asset)], true).unwrap();
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("icons");
    let installed = install(&p, &root, std::slice::from_ref(&root), &|| false).unwrap();
    let output = cursormochi_core::decode(
        &fs::read(installed.join("cursors/progress")).unwrap(),
        || false,
    )
    .unwrap();
    for size in [32, 64] {
        let v = output
            .iter()
            .find(|v| v.nominal == size)
            .expect("real desktop-size variant missing");
        assert_eq!(v.frames.len(), original[0].frames.len());
        for (f, source) in v.frames.iter().zip(&original[0].frames) {
            assert_eq!((f.width, f.height), (size, size));
            assert_eq!(f.hotspot, (0, 0));
            assert_eq!(f.delay, source.delay);
        }
    }
    let preserved = output
        .iter()
        .filter(|v| v.nominal == 160)
        .cloned()
        .collect::<Vec<_>>();
    assert!(originals_preserved(&original, &preserved));
    let report = fs::read_to_string(installed.join("conversion-report.txt")).unwrap();
    assert!(report.contains("resampled"));
}
#[test]
fn complete_windows_import_reports_uncovered_system_roles() {
    let asset = load(&fixtures().join("busy.ani"), &|| false)
        .unwrap()
        .assets
        .remove(0);
    let assignments = (0..cursormochi_app::import::IMPORT_ROLES.len())
        .map(|i| (i, asset.clone()))
        .collect::<Vec<_>>();
    let p = plan("All-Windows-Roles", &assignments, true).unwrap();
    assert!(p.missing.is_empty());
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("icons");
    let installed = install(&p, &root, std::slice::from_ref(&root), &|| false).unwrap();
    let report = fs::read_to_string(installed.join("conversion-report.txt")).unwrap();
    assert!(
        report.contains("System files: 23/34 covered · 11 missing"),
        "{report}"
    );
    assert!(report.contains("Grab (grab)"));
    assert!(report.contains("Zoom in (zoom-in)"));
    assert!(!report.contains("Column resize (col-resize)"));
}
#[test]
fn window_edges_corners_and_pane_dividers_resolve_to_confirmed_resize_sources() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("icons");
    let asset = load(&fixtures().join("busy.ani"), &|| false)
        .unwrap()
        .assets
        .remove(0);
    let assignments = [5, 6, 9, 13]
        .into_iter()
        .map(|i| {
            let mut a = asset.clone();
            for v in &mut a.decoded.as_mut().unwrap().variants {
                for f in &mut v.frames {
                    Arc::make_mut(&mut f.rgba)[..4].copy_from_slice(&[i as u8 * 8, 0, 0, 255]);
                }
            }
            (i, a)
        })
        .collect::<Vec<_>>();
    let p = plan("Window-Resize", &assignments, true).unwrap();
    let installed = install(&p, &root, std::slice::from_ref(&root), &|| false).unwrap();
    for (canonical, aliases) in [
        (
            "ew-resize",
            vec![
                "e-resize",
                "w-resize",
                "right_side",
                "left_side",
                "col-resize",
            ],
        ),
        (
            "ns-resize",
            vec![
                "n-resize",
                "s-resize",
                "top_side",
                "bottom_side",
                "row-resize",
            ],
        ),
        (
            "nwse-resize",
            vec![
                "nw-resize",
                "se-resize",
                "top_left_corner",
                "bottom_right_corner",
            ],
        ),
        (
            "nesw-resize",
            vec![
                "ne-resize",
                "sw-resize",
                "top_right_corner",
                "bottom_left_corner",
            ],
        ),
    ] {
        let original = fs::read(installed.join("cursors").join(canonical)).unwrap();
        for alias in aliases {
            let path = installed.join("cursors").join(alias);
            assert!(path.is_file(), "window resize cursor {alias} missing");
            let bytes = fs::read(path).unwrap();
            assert_eq!(bytes, original, "{alias} must preserve its assigned axis");
            assert!(originals_preserved(
                &p.mapped
                    .iter()
                    .find(|(role, _)| role == canonical)
                    .unwrap()
                    .1
                    .decoded
                    .as_ref()
                    .unwrap()
                    .variants,
                &cursormochi_core::decode(&bytes, || false).unwrap()
            ));
        }
    }
    let p = plan("No-Diagonals", &assignments[..2], true).unwrap();
    let installed = install(&p, &root, std::slice::from_ref(&root), &|| false).unwrap();
    assert!(p.missing.contains(&"Resize diagonally"));
    assert!(p.missing.contains(&"Resize other diagonal"));
    for corner in ["nw-resize", "ne-resize", "sw-resize", "se-resize"] {
        assert!(!installed.join("cursors").join(corner).exists());
    }
}
#[test]
fn numbered_windows_diagonals_survive_default_mapping_and_export_all_corners() {
    let package = load(&fixtures().join("diagonals.zip"), &|| false).unwrap();
    let assignments = package
        .assets
        .into_iter()
        .map(|a| (suggestion(&a.path).unwrap(), a))
        .collect::<Vec<_>>();
    assert_eq!(
        assignments.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
        [9, 13]
    );
    assert!(plan("Numbered-Diagonals", &assignments, false).is_err());
    let p = plan("Numbered-Diagonals", &assignments, true).unwrap();
    assert!(!p.missing.contains(&"Resize diagonally"));
    assert!(!p.missing.contains(&"Resize other diagonal"));
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("icons");
    let installed = install(&p, &root, std::slice::from_ref(&root), &|| false).unwrap();
    for (role, names) in [
        (9, ["nw-resize", "se-resize"]),
        (13, ["ne-resize", "sw-resize"]),
    ] {
        let expected = &assignments
            .iter()
            .find(|(i, _)| *i == role)
            .unwrap()
            .1
            .decoded
            .as_ref()
            .unwrap()
            .variants;
        for name in names {
            let bytes = fs::read(installed.join("cursors").join(name)).unwrap();
            assert!(originals_preserved(
                expected,
                &cursormochi_core::decode(&bytes, || false).unwrap()
            ));
        }
    }
}
#[test]
fn cancellation_and_failed_generation_clean_staging() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("icons");
    let count = std::cell::Cell::new(0);
    let cancel = || {
        count.set(count.get() + 1);
        count.get() > 5
    };
    assert!(install(&demo(), &root, std::slice::from_ref(&root), &cancel).is_err());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    let mut p = demo();
    p.mapped[0].1.decoded.as_mut().unwrap().variants[0].frames[0].hotspot = (999, 999);
    assert!(install(&p, &root, std::slice::from_ref(&root), &|| false).is_err());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
}
#[test]
fn concurrent_same_name_install_cannot_replace_winner() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("icons");
    let barrier = Arc::new(Barrier::new(2));
    let mut joins = Vec::new();
    for _ in 0..2 {
        let b = barrier.clone();
        let root = root.clone();
        joins.push(std::thread::spawn(move || {
            let p = demo();
            b.wait();
            install(&p, &root, std::slice::from_ref(&root), &|| false)
        }));
    }
    assert_eq!(
        joins
            .into_iter()
            .filter_map(|j| j.join().unwrap().ok())
            .count(),
        1
    );
    assert_eq!(fs::read_dir(root).unwrap().count(), 1);
}
#[test]
fn package_and_destination_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    fs::create_dir(&source).unwrap();
    symlink(fixtures().join("normal.cur"), source.join("normal.cur")).unwrap();
    assert!(load(&source, &|| false).is_err());
    fs::remove_file(source.join("normal.cur")).unwrap();
    fs::write(
        t.path().join("original.cur"),
        fs::read(fixtures().join("normal.cur")).unwrap(),
    )
    .unwrap();
    fs::hard_link(t.path().join("original.cur"), source.join("normal.cur")).unwrap();
    assert!(load(&source, &|| false).is_err());
    let real = t.path().join("real");
    fs::create_dir(&real).unwrap();
    let link = t.path().join("icons");
    symlink(&real, &link).unwrap();
    assert!(install(&demo(), &link, std::slice::from_ref(&link), &|| false).is_err());
    assert_eq!(fs::read_dir(&real).unwrap().count(), 0);
}
#[test]
fn zip_paths_duplicates_links_and_limits_are_rejected() {
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("bad.zip");
    let cur = fs::read(fixtures().join("normal.cur")).unwrap();
    for name in [
        "../escape.cur",
        "/abs.cur",
        "C:/escape.cur",
        "a\\escape.cur",
        "a/../../escape.cur",
    ] {
        zip(&path, &[(name, &cur)]);
        assert!(load(&path, &|| false).is_err(), "{name}");
    }
    zip(&path, &[("a.cur", &cur), ("b.cur", &cur)]);
    let mut bytes = fs::read(&path).unwrap();
    for i in 0..bytes.len().saturating_sub(4) {
        if &bytes[i..i + 5] == b"b.cur" {
            bytes[i] = b'a';
        }
    }
    fs::write(&path, bytes).unwrap();
    assert!(
        load(&path, &|| false).is_err(),
        "duplicate normalized ZIP entry"
    );
    // Link entry is rejected even though nothing is ever extracted to disk.
    let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    z.add_symlink(
        "normal.cur",
        "/etc/passwd",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    z.finish().unwrap();
    assert!(load(&path, &|| false).is_err());
    let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    for i in 0..1025 {
        z.start_file(format!("{i}.txt"), zip::write::SimpleFileOptions::default())
            .unwrap();
    }
    z.finish().unwrap();
    assert!(load(&path, &|| false).is_err());
    zip(&path, &[("normal.cur", &cur)]);
    let mut bytes = fs::read(&path).unwrap();
    let pos = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
    bytes[pos + 24..pos + 28].copy_from_slice(&u32::MAX.to_le_bytes());
    fs::write(&path, bytes).unwrap();
    assert!(load(&path, &|| false).is_err());
    assert!(load(&fixtures(), &|| true).is_err());
}

#[test]
fn refreshing_during_staging_never_exposes_an_applicable_theme() {
    use cursormochi_app::ThemeRepository;
    use cursormochi_platform::{Paths, Repository};
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("icons");
    let repo = Repository::new(Paths::fixture(root.clone()));
    let probe = || {
        let catalog = repo.refreshed().scan(&|| false).unwrap();
        assert!(
            catalog.themes.is_empty(),
            "uncommitted staging must not be an applicable theme"
        );
        false
    };
    install(&demo(), &root, std::slice::from_ref(&root), &probe).unwrap();
    let catalog = repo.refreshed().scan(&|| false).unwrap();
    assert_eq!(catalog.themes.len(), 1);
    assert_eq!(catalog.themes[0].name.as_str(), "Imported-Test");
}
