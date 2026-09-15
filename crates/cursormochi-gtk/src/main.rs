#![forbid(unsafe_code)]
mod preview;
mod smoke;
mod theme_list;
mod ui;
mod worker;
use cursormochi_app::*;
use cursormochi_core::*;
use cursormochi_platform::*;
use gtk::{gdk, prelude::*};
use std::path::PathBuf;
fn label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .selectable(false)
        .build()
}
fn texture(f: &Frame) -> gdk::MemoryTexture {
    gdk::MemoryTexture::new(
        f.width as i32,
        f.height as i32,
        gdk::MemoryFormat::R8g8b8a8Premultiplied,
        &glib::Bytes::from_owned(f.rgba.clone()),
        f.width as usize * 4,
    )
}
fn fixture_settings() -> Result<GnomeSettings, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/schemas");
    let source =
        gio::SettingsSchemaSource::from_directory(path, None, false).map_err(|e| e.to_string())?;
    let schema = source
        .lookup("io.github.cursormochi.test", false)
        .ok_or("fixture schema missing; run glib-compile-schemas tests/schemas")?;
    GnomeSettings::injected(schema, &gio::memory_settings_backend_new(), false)
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|a| a == "--help") {
        println!(
            "cursor-mochi [--fixture] [--smoke-test] [--diagnose]\n--fixture: bundled assets and explicitly injected read-only memory settings\n--smoke-test: fixture GUI interaction self-check; exits automatically\n--diagnose: read-only, redacted local discovery report without a display"
        );
        return;
    }
    if args.iter().any(|a| a == "--diagnose") {
        let env = Environment::capture();
        let paths = Paths::from_env(&env);
        let repo = Repository::new(paths.clone());
        println!(
            "{}",
            redact(
                &format!(
                    "CursorMochi {}\n{POLICY}\n{paths:?}",
                    env!("CARGO_PKG_VERSION")
                ),
                &env
            )
        );
        match repo.scan(&|| false) {
            Ok(catalog) => {
                println!(
                    "themes={} excluded={} diagnostics={}",
                    catalog.themes.len(),
                    catalog.candidates.len(),
                    catalog.diagnostics.len()
                );
                for t in catalog.themes.iter().take(100) {
                    let result = repo.preview(
                        &t.name,
                        t.availability.role().unwrap_or("left_ptr"),
                        &|| false,
                    );
                    let text = match result {
                        Ok(p) => format!(
                            "{}: {:?} {} variants={}",
                            t.name.as_str(),
                            p.resolution,
                            p.source.display(),
                            p.variants.len()
                        ),
                        Err(e) => format!("{}: {e}", t.name.as_str()),
                    };
                    println!("{}", redact(&text, &env));
                }
                for d in &catalog.diagnostics {
                    println!("{}", redact(d, &env));
                }
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let smoke = args.iter().any(|a| a == "--smoke-test");
    let fixture = smoke || args.iter().any(|a| a == "--fixture");
    let app = gtk::Application::builder()
        .application_id("io.github.cursormochi.CursorMochi")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| ui::build(app, fixture, smoke));
    app.run_with_args(&["cursor-mochi"]);
}
