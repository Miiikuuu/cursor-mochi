use cursormochi_app::*;
use cursormochi_core::ThemeName;
use cursormochi_platform::*;
fn backend(writable: bool) -> GnomeSettings {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/schemas");
    let source = gio::SettingsSchemaSource::from_directory(path, None, false).unwrap();
    let schema = source.lookup("io.github.cursormochi.test", false).unwrap();
    GnomeSettings::injected(schema, &gio::memory_settings_backend_new(), writable).unwrap()
}
#[test]
fn explicit_memory_apply_reset_and_independent_read() {
    let p = backend(true);
    let before = p.read().unwrap();
    let mut c = Controller::default();
    let plan = c
        .prepare(
            &p,
            ChangeIntent {
                theme: ThemeName::new("B").unwrap(),
                size: None,
                resolved: true,
            },
        )
        .unwrap();
    assert_eq!(c.begin(&p, plan, false), Status::Writing);
    assert_eq!(c.poll(&p, false), Status::Observed);
    assert_eq!(p.read().unwrap()[&Key::Size], before[&Key::Size]);
    c.begin_undo(&p);
    assert_eq!(c.poll(&p, false), Status::Observed);
    assert_eq!(p.read().unwrap(), before);
}
#[test]
fn readonly_and_schema_range() {
    let p = backend(false);
    assert!(!p.capability().writable);
    assert!(
        p.write(Key::Theme, Some(&Value::Text("bad".into())))
            .is_err()
    );
    assert!(p.validate(Key::Size, &Value::Int(-1)).is_err());
    assert!(p.validate(Key::Size, &Value::Text("32".into())).is_err());
    assert_eq!(p.read().unwrap()[&Key::Theme].user, None);
}
#[test]
fn schema_without_keys_and_non_gnome() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/schemas");
    let source = gio::SettingsSchemaSource::from_directory(path, None, false).unwrap();
    let schema = source
        .lookup("io.github.cursormochi.missing", false)
        .unwrap();
    assert!(GnomeSettings::injected(schema, &gio::memory_settings_backend_new(), true).is_err());
    assert!(source.lookup("nonexistent", false).is_none());
}
#[test]
fn notifications_do_not_write() {
    let p = backend(true);
    let before = p.read().unwrap();
    let called = std::rc::Rc::new(std::cell::Cell::new(0));
    let capture = called.clone();
    p.connect_changed(move || capture.set(capture.get() + 1));
    assert_eq!(p.read().unwrap(), before);
    assert!(p.read().unwrap().values().all(|v| v.user.is_none()));
    p.write(Key::Theme, Some(&Value::Text("notify".into())))
        .unwrap();
    while glib::MainContext::default().pending() {
        glib::MainContext::default().iteration(false);
    }
    assert!(called.get() > 0);
}
