#![forbid(unsafe_code)]
mod worker;
use cursormochi_app::*;
use cursormochi_core::*;
use cursormochi_platform::*;
use gtk::{gdk, prelude::*};
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
use worker::{Job, Output, Worker};
struct State {
    catalog: Catalog,
    selected: Option<ThemeName>,
    preview: Option<Preview>,
    request: u64,
    controller: Controller,
    started: Instant,
    elapsed: Duration,
    last_tick: Instant,
    last_frame: Option<(usize, usize, i32)>,
    status: String,
    diagnostics: String,
    loaded: bool,
    frames_shown: usize,
}
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
                    "themes={} diagnostics={}",
                    catalog.themes.len(),
                    catalog.diagnostics.len()
                );
                for t in catalog.themes.iter().take(100) {
                    let result = repo.preview(&t.name, "left_ptr", &|| false);
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
    app.connect_activate(move |app| build(app, fixture, smoke));
    app.run_with_args(&["cursor-mochi"]);
}
fn build(app: &gtk::Application, fixture: bool, smoke: bool) {
    let env = Environment::capture();
    let paths = if fixture {
        Paths::fixture(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"))
    } else {
        Paths::from_env(&env)
    };
    let settings = if fixture {
        fixture_settings()
    } else {
        GnomeSettings::host(&env)
    };
    let port: Rc<dyn DesktopSettingsPort> = match settings {
        Ok(s) => Rc::new(s),
        Err(e) => Rc::new(ReadOnlySettings(e)),
    };
    let (worker, rx) = Worker::new(Repository::new(paths.clone()));
    let state = Rc::new(RefCell::new(State {
        catalog: Catalog::default(),
        selected: None,
        preview: None,
        request: 0,
        controller: Controller::default(),
        started: Instant::now(),
        elapsed: Duration::ZERO,
        last_tick: Instant::now(),
        last_frame: None,
        status: String::new(),
        diagnostics: format!(
            "CursorMochi {}\ngtk-rs 0.9 / GIO 0.20\nGTK {}.{}.{}\n{POLICY}\n{paths:?}\n{:?}",
            env!("CARGO_PKG_VERSION"),
            gtk::major_version(),
            gtk::minor_version(),
            gtk::micro_version(),
            port.capability()
        ),
        loaded: false,
        frames_shown: 0,
    }));
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(if fixture {
            "CursorMochi · Read-only fixtures"
        } else {
            "CursorMochi"
        })
        .default_width(1000)
        .default_height(720)
        .build();
    let header = gtk::HeaderBar::new();
    let refresh = gtk::Button::with_label("Refresh");
    let copy = gtk::Button::with_label("Copy diagnostics");
    header.pack_start(&refresh);
    header.pack_end(&copy);
    window.set_titlebar(Some(&header));
    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(16);
    root.set_margin_end(16);
    let current = label("Reading current settings…");
    root.append(&current);
    let capability = label(if port.capability().writable {
        "GNOME settings available · Theme lookup paths unverified · Check your pointer after applying"
    } else {
        "Browse and preview only · Desktop settings are read-only in this session"
    });
    capability.add_css_class("dim-label");
    root.append(&capability);
    let pane = gtk::Paned::new(gtk::Orientation::Horizontal);
    pane.set_vexpand(true);
    pane.set_position(280);
    root.append(&pane);
    let left = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search themes"));
    left.append(&search);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();
    left.append(&scroll);
    pane.set_start_child(Some(&left));
    let detail = gtk::Box::new(gtk::Orientation::Vertical, 12);
    detail.set_margin_start(20);
    pane.set_end_child(Some(&detail));
    let title = label("Select a local cursor theme");
    title.add_css_class("title-1");
    detail.append(&title);
    let roles = gtk::ComboBoxText::new();
    for r in [
        "left_ptr",
        "default",
        "pointer",
        "hand2",
        "text",
        "xterm",
        "watch",
        "wait",
        "crosshair",
        "move",
        "not-allowed",
    ] {
        roles.append_text(r)
    }
    roles.set_active(Some(0));
    detail.append(&roles);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let sizes = gtk::ComboBoxText::new();
    sizes.set_tooltip_text(Some(
        "Nominal size stored in the file; does not change the GNOME cursor size",
    ));
    let zoom = gtk::SpinButton::with_range(1., 8., 1.);
    zoom.set_value(1.);
    let hotspot = gtk::CheckButton::with_label("Show hotspot");
    hotspot.set_active(true);
    controls.append(&label("Nominal size"));
    controls.append(&sizes);
    controls.append(&label("Zoom ×"));
    controls.append(&zoom);
    controls.append(&hotspot);
    detail.append(&controls);
    let picture = gtk::Picture::new();
    picture.set_can_shrink(false);
    picture.set_halign(gtk::Align::Center);
    picture.set_valign(gtk::Align::Center);
    let canvas = gtk::Overlay::new();
    let background = gtk::DrawingArea::new();
    background.set_content_height(260);
    background.set_draw_func(|_, cr, w, h| {
        for y in (0..h).step_by(16) {
            for x in (0..w).step_by(16) {
                let c = if (x / 16 + y / 16) % 2 == 0 {
                    0.24
                } else {
                    0.32
                };
                cr.set_source_rgb(c, c, c);
                cr.rectangle(x as f64, y as f64, 16., 16.);
                let _ = cr.fill();
            }
        }
    });
    canvas.set_child(Some(&background));
    canvas.add_overlay(&picture);
    let mark = gtk::DrawingArea::new();
    mark.set_can_target(false);
    canvas.add_overlay(&mark);
    canvas.set_measure_overlay(&picture, true);
    let canvas_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(260)
        .max_content_height(260)
        .child(&canvas)
        .build();
    detail.append(&canvas_scroll);
    let static_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let still = gtk::Picture::new();
    still.set_size_request(48, 48);
    static_box.append(&still);
    static_box.append(&label("First frame · Static preview"));
    detail.append(&static_box);
    let meta =
        label("Previewing actual Xcursor files. Selecting a theme does not change your settings.");
    detail.append(&meta);
    let source = label("");
    source.set_selectable(true);
    source.add_css_class("dim-label");
    detail.append(&source);
    let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let change_size = gtk::CheckButton::with_label("Also change GNOME cursor size");
    let setting_size = gtk::SpinButton::with_range(1., 256., 1.);
    setting_size.set_value(24.);
    setting_size.set_sensitive(false);
    bottom.append(&change_size);
    bottom.append(&setting_size);
    let apply = gtk::Button::with_label("Apply");
    apply.add_css_class("suggested-action");
    apply.set_sensitive(false);
    let undo = gtk::Button::with_label("Undo last change");
    undo.set_sensitive(false);
    bottom.append(&apply);
    bottom.append(&undo);
    root.append(&bottom);
    let status = label("Scanning local themes…");
    root.append(&status);
    window.set_child(Some(&root));
    {
        let size = setting_size.clone();
        change_size.connect_toggled(move |b| size.set_sensitive(b.is_active()));
    }
    {
        let state = state.clone();
        let sizes = sizes.clone();
        let zoom = zoom.clone();
        let hotspot = hotspot.clone();
        mark.set_draw_func(move |_, cr, w, h| {
            if !hotspot.is_active() {
                return;
            }
            let s = state.borrow();
            let Some(p) = &s.preview else { return };
            let vi = sizes.active().unwrap_or(0) as usize;
            let Some(v) = p.variants.get(vi) else { return };
            let f = &v.frames[v.frame_at(s.elapsed.as_millis() as u64)];
            let z = zoom.value();
            let x = (w as f64 - f.width as f64 * z) / 2. + (f.hotspot.0 as f64 + 0.5) * z;
            let y = (h as f64 - f.height as f64 * z) / 2. + (f.hotspot.1 as f64 + 0.5) * z;
            cr.set_source_rgb(1., 0.2, 0.3);
            cr.set_line_width(1.5);
            cr.move_to(x - 6., y);
            cr.line_to(x + 6., y);
            cr.move_to(x, y - 6.);
            cr.line_to(x, y + 6.);
            let _ = cr.stroke();
        });
    }
    let request: Rc<dyn Fn()> = {
        let state = state.clone();
        let worker = worker.clone();
        let roles = roles.clone();
        let picture = picture.clone();
        let still = still.clone();
        let apply = apply.clone();
        let source = source.clone();
        let meta = meta.clone();
        let status = status.clone();
        Rc::new(move || {
            let mut s = state.borrow_mut();
            s.preview = None;
            s.last_frame = None;
            source.set_text("");
            meta.set_text("");
            picture.set_paintable(None::<&gdk::Texture>);
            still.set_paintable(None::<&gdk::Texture>);
            apply.set_sensitive(false);
            if let (Some(t), Some(r)) = (s.selected.clone(), roles.active_text()) {
                s.request = worker.submit(Job::Preview(t, r.to_string()));
                status.set_text("Loading cursor file…");
            }
        })
    };
    {
        let request = request.clone();
        roles.connect_changed(move |_| request());
    }
    {
        let state = state.clone();
        let roles = roles.clone();
        let title = title.clone();
        let request = request.clone();
        list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let index = row.index() as usize;
            let t = state.borrow().catalog.themes.get(index).cloned();
            if let Some(t) = t {
                state.borrow_mut().selected = Some(t.name.clone());
                title.set_text(&t.display);
                roles.remove_all();
                let mut rs = vec!["left_ptr".to_string(), "default".into(), "watch".into()];
                rs.extend(t.roles);
                let mut seen = std::collections::HashSet::new();
                rs.retain(|r| seen.insert(r.clone()));
                for r in rs {
                    roles.append_text(&r)
                }
                roles.set_active(Some(0));
                request();
            }
        });
    }
    {
        let state = state.clone();
        let list = list.clone();
        search.connect_search_changed(move |entry| {
            let query = entry.text().to_lowercase();
            let s = state.borrow();
            for (i, t) in s.catalog.themes.iter().enumerate() {
                if let Some(row) = list.row_at_index(i as i32) {
                    row.set_visible(
                        t.display.to_lowercase().contains(&query)
                            || t.name.as_str().to_lowercase().contains(&query),
                    );
                }
            }
        });
    }
    {
        let state = state.clone();
        let worker = worker.clone();
        let status = status.clone();
        let picture = picture.clone();
        let still = still.clone();
        let source = source.clone();
        let meta = meta.clone();
        let title = title.clone();
        refresh.connect_clicked(move |_| {
            let mut s = state.borrow_mut();
            s.preview = None;
            s.selected = None;
            s.last_frame = None;
            picture.set_paintable(None::<&gdk::Texture>);
            still.set_paintable(None::<&gdk::Texture>);
            source.set_text("");
            meta.set_text("");
            title.set_text("Select a local cursor theme");
            s.request = worker.submit(Job::Scan);
            status.set_text("Refreshing…");
        });
    }
    {
        let state = state.clone();
        let port = port.clone();
        let env = env.clone();
        let window = window.downgrade();
        copy.connect_clicked(move |_| {
            if let Some(w) = window.upgrade() {
                let s = state.borrow();
                let text = redact(
                    &format!(
                        "{}\nbackend={:?}\nsettings={:?}\nstatus={}\nsource={:?}\nmessages={:?}\noperation={:?}",
                        s.diagnostics,
                        gdk::Display::default().map(|d| d.type_().name()),
                        port.read(),
                        s.status,
                        s.preview.as_ref().map(|p| (&p.source, &p.chain)),
                        s.controller.messages,
                        s.controller.last_operation
                    ),
                    &env,
                );
                w.clipboard().set_text(&text);
            }
        });
    }
    {
        let state = state.clone();
        let port = port.clone();
        let size = setting_size.clone();
        let change = change_size.clone();
        let status = status.clone();
        apply.connect_clicked(move |_| {
            let mut s = state.borrow_mut();
            let Some(theme) = s.selected.clone() else {
                return;
            };
            let intent = ChangeIntent {
                theme,
                size: change.is_active().then(|| size.value_as_int()),
                resolved: s.preview.is_some(),
            };
            match s.controller.prepare(port.as_ref(), intent) {
                Ok(plan) => {
                    let result = s.controller.begin(port.as_ref(), plan, false);
                    s.started = Instant::now();
                    s.status = format!("{result:?}");
                    status.set_text(&format!(
                        "{:?} {}",
                        result,
                        s.controller.messages.join("; ")
                    ));
                }
                Err(e) => status.set_text(&e),
            }
        });
    }
    {
        let state = state.clone();
        let port = port.clone();
        let status = status.clone();
        undo.connect_clicked(move |_| {
            let mut s = state.borrow_mut();
            let result = s.controller.begin_undo(port.as_ref());
            s.started = Instant::now();
            s.status = format!("{result:?}");
            status.set_text(&format!("{result:?} {}", s.controller.messages.join("; ")));
        });
    }
    {
        let state = state.clone();
        let status = status.clone();
        window.connect_close_request(move |_| {
            if state.borrow().controller.busy() {
                status.set_text("A settings change is in progress. You can close the window after verification finishes.");
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    state.borrow_mut().request = worker.submit(Job::Scan);
    let weak = window.downgrade();
    let app = app.clone();
    let mut settings_tick = Instant::now() - Duration::from_secs(1);
    let mut writable = port.capability().writable;
    let mut smoke_stage = 0;
    let mut smoke_at = Instant::now();
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let Some(window) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if smoke_stage == 2 {
            if smoke_at.elapsed() < Duration::from_millis(150) {
                return glib::ControlFlow::Continue;
            }
            let snapshot = gtk::Snapshot::new();
            let color = window
                .style_context()
                .lookup_color("theme_bg_color")
                .unwrap_or(gdk::RGBA::WHITE);
            if let Some(parent) = root.parent() {
                parent.snapshot_child(&root, &snapshot);
            }
            if let (Some(node), Some(renderer)) = (snapshot.to_node(), window.renderer()) {
                let opaque = gtk::Snapshot::new();
                opaque.append_color(&color, &node.bounds());
                opaque.append_node(node);
                if let Some(node) = opaque.to_node() {
                    let image = renderer.render_texture(&node, None);
                    if let Err(e) = image.save_to_png("target/qa/fixture-smoke.png") {
                        eprintln!("snapshot: {e}");
                    }
                }
            } else {
                eprintln!("snapshot unavailable");
            }
            app.quit();
            return glib::ControlFlow::Break;
        }
        while let Ok((id, output)) = rx.try_recv() {
            if id != state.borrow().request {
                continue;
            }
            match output {
                Output::Scan(Ok(catalog)) => {
                    while let Some(child) = list.first_child() {
                        list.remove(&child)
                    }
                    let text = format!(
                        "Found {} themes; {} diagnostics",
                        catalog.themes.len(),
                        catalog.diagnostics.len()
                    );
                    {
                        let mut s = state.borrow_mut();
                        let end = s.diagnostics.find("\nscan:").unwrap_or(s.diagnostics.len());
                        s.diagnostics.truncate(end);
                        s.diagnostics
                            .push_str(&format!("\nscan:{:?}", catalog.diagnostics));
                        s.catalog = catalog;
                        s.loaded = true;
                        for t in &s.catalog.themes {
                            let l = label(&if t.display == t.name.as_str() {
                                t.display.clone()
                            } else {
                                format!("{}\n{}", t.display, t.name.as_str())
                            });
                            // Let the ListBoxRow own pointer gestures and keyboard focus.
                            l.set_can_target(false);
                            l.set_margin_top(10);
                            l.set_margin_bottom(10);
                            l.set_margin_start(10);
                            list.append(&l);
                        }
                    }
                    status.set_text(&text);
                    if let Some(row) = list.row_at_index(if fixture { 3 } else { 0 }) {
                        list.select_row(Some(&row));
                    }
                }
                Output::Preview(Ok(p)) => {
                    sizes.remove_all();
                    for v in &p.variants {
                        sizes.append_text(&v.nominal.to_string())
                    }
                    sizes.set_active(Some(0));
                    source.set_text(&redact(
                        &format!(
                            "{:?}\n{}\n{}",
                            p.resolution,
                            p.source.display(),
                            p.chain.join(" → ")
                        ),
                        &env,
                    ));
                    let mut s = state.borrow_mut();
                    s.preview = Some(p);
                    s.elapsed = Duration::ZERO;
                    s.last_frame = None;
                    status.set_text(
                        "Cursor file loaded · Previewing does not change desktop settings",
                    );
                }
                Output::Scan(Err(e)) | Output::Preview(Err(e)) => {
                    status.set_text(&format!("{e} · Select another theme or refresh"));
                    let mut s = state.borrow_mut();
                    s.status = e.to_string();
                    s.preview = None;
                }
            }
        }
        let mut s = state.borrow_mut();
        let now = Instant::now();
        let visible = window.is_mapped()
            && window
                .surface()
                .and_then(|v| v.downcast::<gdk::Toplevel>().ok())
                .is_none_or(|v| !v.state().contains(gdk::ToplevelState::MINIMIZED));
        if visible {
            let delta = now.duration_since(s.last_tick);
            s.elapsed += delta
        }
        s.last_tick = now;
        if s.controller.busy() {
            let expired = s.started.elapsed() > Duration::from_secs(2);
            let result = s.controller.poll(port.as_ref(), expired);
            if result != Status::VerificationPending {
                s.status = format!("{result:?}");
                status.set_text(&if result == Status::Observed {
                    "Settings updated. Please check your actual pointer.".into()
                } else {
                    format!("{result:?} {}", s.controller.messages.join("; "))
                });
            }
        }
        if now.duration_since(settings_tick) >= Duration::from_millis(250) {
            settings_tick = now;
            writable = port.capability().writable;
            current.set_text(&format!(
                "Current settings: {}",
                match port.read() {
                    Ok(snapshot) => {
                        let theme = match snapshot.get(&Key::Theme).map(|s| &s.effective) {
                            Some(Value::Text(t)) => t.as_str(),
                            _ => "Unknown",
                        };
                        let size = match snapshot.get(&Key::Size).map(|s| &s.effective) {
                            Some(Value::Int(n)) => n.to_string(),
                            _ => "Unknown".into(),
                        };
                        format!("Theme: {theme} · GNOME cursor size: {size}")
                    }
                    Err(e) => e,
                }
            ));
        }
        apply.set_sensitive(!s.controller.busy() && s.preview.is_some() && writable);
        undo.set_sensitive(!s.controller.busy() && s.controller.undo.is_some() && writable);
        refresh.set_sensitive(!s.controller.busy());
        if visible && let Some(p) = &s.preview {
            let vi = sizes.active().unwrap_or(0) as usize;
            if let Some(v) = p.variants.get(vi) {
                let fi = v.frame_at(s.elapsed.as_millis() as u64);
                let z = zoom.value_as_int();
                let key = (vi, fi, z);
                if s.last_frame != Some(key) {
                    let f = &v.frames[fi];
                    picture.set_paintable(Some(&texture(f)));
                    picture.set_size_request(f.width as i32 * z, f.height as i32 * z);
                    still.set_paintable(Some(&texture(&v.frames[0])));
                    meta.set_text(&format!("Nominal {} · Actual {} × {} · Frame {}/{} · Hotspot ({}, {})\nOriginal delay {} ms{} · Zoom {}× (logical pixels)",v.nominal,f.width,f.height,fi+1,v.frames.len(),f.hotspot.0,f.hotspot.1,f.delay,if f.delay<16||f.delay>10000{"; playback limited to 16–10000 ms"}else{""},z));
                    s.last_frame = Some(key);
                    s.frames_shown += 1;
                }
                mark.queue_draw();
            }
        }
        if smoke {
            if smoke_at.elapsed() > Duration::from_secs(12) {
                eprintln!("GUI_SMOKE FAIL: timeout");
                app.quit();
                std::process::exit(1)
            }
            if smoke_stage == 0 && s.preview.is_some() && s.frames_shown >= 3 {
                assert!(!apply.is_sensitive());
                assert!(!undo.is_sensitive());
                assert!(port.read().is_ok());
                let row = list.selected_row().expect("fixture theme row selected");
                let target = row
                    .pick(
                        (row.width() / 2) as f64,
                        (row.height() / 2) as f64,
                        gtk::PickFlags::DEFAULT,
                    )
                    .expect("theme row has a pointer target");
                assert!(
                    target.is::<gtk::ListBoxRow>(),
                    "Theme text must not intercept row clicks"
                );
                println!("GUI_SMOKE: theme row pointer hit target PASS");
                println!("GUI_SMOKE: animated fixture, textures, read-only controls PASS");
                drop(s);
                zoom.set_value(3.0);
                assert!(search.grab_focus());
                roles.set_active(Some(1));
                roles.set_active(Some(0));
                smoke_stage = 1;
                smoke_at = Instant::now();
                return glib::ControlFlow::Continue;
            }
            if smoke_stage == 1
                && s.preview.is_some()
                && smoke_at.elapsed() > Duration::from_millis(400)
            {
                println!(
                    "GUI_SMOKE: selection replacement PASS; scale={} backend={}",
                    window.scale_factor(),
                    gdk::Display::default()
                        .map(|d| d.type_().name().to_string())
                        .unwrap_or_default()
                );
                smoke_stage = 2;
                smoke_at = Instant::now();
            }
        }
        glib::ControlFlow::Continue
    });
    window.present();
}
