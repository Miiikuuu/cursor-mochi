use cursormochi_app::ThemeRepository;
use cursormochi_core::*;
use cursormochi_platform::*;
use std::{
    fs,
    os::unix::{ffi::OsStringExt, fs::symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
static ID: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cursormochi-test-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn repo(&self) -> Repository {
        Repository::new(Paths::fixture(self.0.clone()))
    }
    fn put(&self, p: &str, b: &[u8]) {
        let p = self.0.join(p);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b).unwrap();
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/Mochi-Light/cursors/left_ptr"),
    )
    .unwrap()
}
fn theme(s: &str) -> ThemeName {
    ThemeName::new(s).unwrap()
}
#[test]
fn empty_and_cancel() {
    let t = Temp::new();
    assert!(t.repo().scan(&|| false).unwrap().themes.is_empty());
    t.put("A/cursors/left_ptr", &fixture());
    assert_eq!(t.repo().scan(&|| true).unwrap_err(), Error::Cancelled);
}
#[test]
fn same_name_per_role_and_priority() {
    let t = Temp::new();
    let b = fixture();
    t.put("one/A/cursors/left_ptr", &b);
    t.put("two/A/cursors/watch", &b);
    t.put("two/A/cursors/left_ptr", b"invalid");
    let paths = Paths {
        discovery: vec![t.0.join("one"), t.0.join("two")],
        resolution: vec![t.0.join("one"), t.0.join("two")],
        diagnostics: vec![],
    };
    let r = Repository::new(paths);
    assert_eq!(r.scan(&|| false).unwrap().themes[0].locations.len(), 2);
    assert!(
        r.preview(&theme("A"), "left_ptr", &|| false)
            .unwrap()
            .source
            .starts_with(t.0.join("one"))
    );
    assert!(
        r.preview(&theme("A"), "watch", &|| false)
            .unwrap()
            .source
            .starts_with(t.0.join("two"))
    );
}
#[test]
fn inherit_fallback_cycle() {
    let t = Temp::new();
    t.put("B/cursors/left_ptr", &fixture());
    t.put("A/index.theme", b"[Icon Theme]\nInherits=B\n");
    t.put("default/index.theme", b"[Icon Theme]\nInherits=B\n");
    let r = t.repo();
    assert_eq!(
        r.preview(&theme("A"), "left_ptr", &|| false)
            .unwrap()
            .resolution,
        Resolution::Inherited
    );
    assert_eq!(
        r.preview(&theme("missing"), "left_ptr", &|| false)
            .unwrap()
            .resolution,
        Resolution::Fallback
    );
    assert_eq!(r.scan(&|| false).unwrap().themes.len(), 3);
    t.put("B/index.theme", b"[Icon Theme]\nInherits=A\n");
    assert!(matches!(
        r.preview(&theme("A"), "watch", &|| false),
        Err(Error::Invalid("inheritance cycle"))
    ));
}
#[test]
fn links_broken_cycle_escape_and_deleted() {
    let t = Temp::new();
    t.put("A/cursors/left_ptr", &fixture());
    let d = t.0.join("A/cursors");
    symlink("left_ptr", d.join("default")).unwrap();
    symlink("absent", d.join("broken")).unwrap();
    symlink("loop", d.join("loop")).unwrap();
    symlink("/etc/passwd", d.join("escape")).unwrap();
    let r = t.repo();
    let p = r.preview(&theme("A"), "default", &|| false).unwrap();
    assert!(p.chain.iter().any(|s| s.contains('→')));
    for role in ["broken", "loop", "escape"] {
        assert!(r.preview(&theme("A"), role, &|| false).is_err())
    }
    fs::remove_file(d.join("left_ptr")).unwrap();
    assert!(r.preview(&theme("A"), "default", &|| false).is_err());
}
#[test]
fn fifo_and_input_limit() {
    let t = Temp::new();
    let p = t.0.join("pipe");
    rustix::fs::mknodat(
        rustix::fs::CWD,
        &p,
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        0,
    )
    .unwrap();
    assert!(matches!(
        t.repo().read_bounded(&p, 1024),
        Err(Error::Invalid("not a regular file"))
    ));
    t.put("huge", &[0; 100]);
    assert!(matches!(
        t.repo().read_bounded(&t.0.join("huge"), 50),
        Err(Error::LimitExceeded(_))
    ));
}
#[test]
fn non_utf8_is_diagnostic() {
    let t = Temp::new();
    fs::create_dir(t.0.join(std::ffi::OsString::from_vec(vec![0xff]))).unwrap();
    assert!(
        t.repo()
            .scan(&|| false)
            .unwrap()
            .diagnostics
            .iter()
            .any(|s| s.contains("non UTF-8"))
    );
}
#[test]
fn xdg_and_override() {
    let e = Environment {
        vars: [
            ("HOME".into(), "/home/test".into()),
            ("XCURSOR_PATH".into(), "/opt/cursors".into()),
            ("XDG_DATA_HOME".into(), "relative".into()),
            ("XDG_DATA_DIRS".into(), "/opt/share:bad".into()),
        ]
        .into(),
    };
    let p = Paths::from_env(&e);
    assert_eq!(p.resolution, vec![PathBuf::from("/opt/cursors")]);
    assert!(
        p.discovery
            .contains(&PathBuf::from("/home/test/.local/share/icons"))
    );
    assert_eq!(p.diagnostics.len(), 2);
    assert!(!redact("/home/test/name /opt/cursors/theme", &e).contains("/home/test"));
}

#[test]
fn icon_only_candidate_is_not_a_verified_cursor_theme() {
    let t = Temp::new();
    t.put("Icons/index.theme", b"[Icon Theme]\nInherits=hicolor\n");
    t.put("default/cursors/left_ptr", &fixture());
    let r = t.repo();
    assert_eq!(
        r.preview(&theme("Icons"), "left_ptr", &|| false)
            .unwrap()
            .resolution,
        Resolution::Fallback
    );
    assert!(
        !r.scan(&|| false)
            .unwrap()
            .themes
            .iter()
            .any(|t| t.name.as_str() == "Icons")
    );
}

#[test]
fn classification_keeps_explicit_inheritance_partial_and_default() {
    let t = Temp::new();
    t.put("Parent/cursors/watch", &fixture());
    t.put("Child/index.theme", b"[Icon Theme]\nInherits=Parent\n");
    t.put("default/index.theme", b"[Icon Theme]\nInherits=Parent\n");
    t.put("Icons/index.theme", b"[Icon Theme]\nInherits=hicolor\n");
    t.put("hicolor/index.theme", b"[Icon Theme]\nName=Hicolor\n");
    let r = t.repo();
    let c = r.scan(&|| false).unwrap();
    assert_eq!(c.themes.len(), 3);
    let child = c
        .themes
        .iter()
        .find(|t| t.name.as_str() == "Child")
        .unwrap();
    assert!(matches!(child.availability, Availability::Inherited { .. }));
    assert_eq!(child.verified_roles, ["watch"]);
    assert!(matches!(
        c.candidates
            .iter()
            .find(|t| t.name.as_str() == "Icons")
            .unwrap()
            .availability,
        Availability::NoCursorSource
    ));
    assert_eq!(
        r.preview(&theme("Icons"), "watch", &|| false)
            .unwrap()
            .resolution,
        Resolution::Fallback
    );
    assert!(matches!(
        r.preview(&theme("Child"), "left_ptr", &|| false),
        Err(Error::Missing)
    ));
}
#[test]
fn classification_reports_broken_candidates_but_keeps_partial_valid_theme() {
    let t = Temp::new();
    t.put("Broken/cursors/left_ptr", b"bad");
    t.put("Missing/index.theme", b"[Icon Theme]\nInherits=absent\n");
    t.put("Loop/index.theme", b"[Icon Theme]\nInherits=Loop\n");
    t.put("Partial/cursors/watch", &fixture());
    t.put("Partial/cursors/left_ptr", b"bad");
    let c = t.repo().scan(&|| false).unwrap();
    assert_eq!(c.themes.len(), 1);
    assert_eq!(c.themes[0].name.as_str(), "Partial");
    assert_eq!(c.candidates.len(), 3);
    assert!(
        c.candidates
            .iter()
            .all(|t| matches!(t.availability, Availability::Invalid(_)))
    );
    assert!(c.diagnostics.iter().any(|d| d.contains("cycle")));
    assert!(c.diagnostics.iter().any(|d| d.contains("Missing parent")));
}
#[test]
fn classification_probe_limit_does_not_claim_verification() {
    let t = Temp::new();
    for i in 0..70 {
        t.put(&format!("Limited/cursors/{i:03}"), b"bad")
    }
    t.put("Limited/cursors/zzz-valid", &fixture());
    let c = t.repo().scan(&|| false).unwrap();
    assert!(c.themes.is_empty());
    assert!(matches!(
        c.candidates[0].availability,
        Availability::Unverified(_)
    ));
}

#[test]
fn malformed_metadata_and_broken_cursor_directory_are_diagnostic_candidates() {
    let t = Temp::new();
    t.put("BadMetadata/index.theme", &[255, 254]);
    fs::create_dir_all(t.0.join("BadDirectory")).unwrap();
    symlink("absent", t.0.join("BadDirectory/cursors")).unwrap();
    let c = t.repo().scan(&|| false).unwrap();
    assert!(c.themes.is_empty());
    assert_eq!(c.candidates.len(), 2);
    assert!(
        c.candidates
            .iter()
            .all(|t| matches!(t.availability, Availability::Invalid(_)))
    );
}
