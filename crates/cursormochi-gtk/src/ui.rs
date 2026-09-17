use super::{
    label,
    preview::PreviewPane,
    theme_list::ThemeList,
    worker::{Job, Output, Worker},
};
use cursormochi_app::{
    browser::{Browser, role_choices},
    *,
};
use cursormochi_core::*;
use cursormochi_platform::*;
use gtk::{gdk, prelude::*};
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
pub(super) struct State {
    pub catalog: Catalog,
    pub browser: Browser,
    pub request: u64,
    pub controller: Controller,
    pub scanning: bool,
    pub started: Instant,
    pub current: Snapshot,
    pub catalog_epoch: u64,
}
fn current_theme(snapshot: &Snapshot) -> Option<&str> {
    match snapshot.get(&Key::Theme).map(|s| &s.effective) {
        Some(Value::Text(s)) => Some(s),
        _ => None,
    }
}
fn size(snapshot: &Snapshot) -> i32 {
    match snapshot.get(&Key::Size).map(|s| &s.effective) {
        Some(Value::Int(n)) => *n,
        _ => 24,
    }
}
fn outcome(status: &Status, controller: &Controller) -> String {
    match status {
        Status::Observed => "Settings updated. Please check your actual pointer.".into(),
        Status::NoOp => "Already in use. No settings were changed.".into(),
        Status::Writing | Status::VerificationPending => {
            "Applying settings… waiting for verification.".into()
        }
        _ => format!("{status:?}: {}", controller.messages.join("; ")),
    }
}
#[derive(Clone, Copy)]
enum ThumbnailTarget {
    Theme(usize),
    Role { generation: u64, index: usize },
}
pub fn build(app: &gtk::Application, fixture: bool, smoke: bool) {
    let env = Environment::capture();
    let mut paths = if fixture {
        Paths::fixture(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"))
    } else {
        Paths::from_env(&env)
    };
    let mut import_test_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    import_test_root.pop();
    import_test_root.pop();
    import_test_root.push(format!("target/qa/import-smoke-{}", std::process::id()));
    if smoke {
        paths.discovery.push(import_test_root.clone());
        paths.resolution.push(import_test_root.clone());
    }
    let mut user_theme_roots = cursormochi_platform::import::user_targets(&env);
    if smoke {
        user_theme_roots.push(import_test_root.clone());
    }
    let settings = if fixture {
        super::fixture_settings()
    } else {
        GnomeSettings::host(&env)
    };
    let port: Rc<dyn DesktopSettingsPort> = match settings {
        Ok(s) => Rc::new(s),
        Err(e) => Rc::new(ReadOnlySettings(e)),
    };
    let port: Rc<dyn DesktopSettingsPort> = if smoke {
        Rc::new(super::smoke::NoWrites(port))
    } else {
        port
    };
    let repo = Repository::new(paths.clone());
    let (worker, rx) = Worker::new(repo.clone());
    let (thumb_worker, thumb_rx) = Worker::new(repo.clone());
    let state = Rc::new(RefCell::new(State {
        catalog: Catalog::default(),
        browser: Browser::default(),
        request: 0,
        controller: Controller::default(),
        scanning: true,
        started: Instant::now(),
        current: port.read().unwrap_or_default(),
        catalog_epoch: 0,
    }));
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(if fixture {
            "CursorMochi · Preview mode"
        } else {
            "CursorMochi"
        })
        .default_width(1120)
        .default_height(820)
        .build();
    let trial = super::trial::Trial::new(&window, repo.clone());
    let current_test = super::trial::current::CurrentTest::new(&window, repo.clone(), fixture);
    let css = gtk::CssProvider::new();
    css.load_from_data(include_str!("style.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    let header = gtk::HeaderBar::new();
    header.add_css_class("main-header");
    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    brand.append(&gtk::Image::from_icon_name("input-mouse-symbolic"));
    brand.append(&label("CursorMochi"));
    header.pack_start(&brand);
    let menu = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Test current cursor, diagnostics and information")
        .build();
    let popover = gtk::Popover::new();
    let menu_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    menu_box.add_css_class("menu-content");
    let copy = gtk::Button::with_label("Copy diagnostics");
    let candidates = gtk::Button::with_label("Candidate diagnostics");
    let test_current = gtk::Button::with_label("Test current cursor");
    menu_box.append(&test_current);
    {
        let test = current_test.clone();
        let popover = popover.clone();
        test_current.connect_clicked(move |_| {
            popover.popdown();
            test.present();
        });
    }
    menu_box.append(&copy);
    menu_box.append(&candidates);
    popover.set_child(Some(&menu_box));
    menu.set_popover(Some(&popover));
    header.pack_end(&menu);
    window.set_titlebar(Some(&header));
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.set_child(Some(&root));
    let current = label("Reading current settings…");
    current.add_css_class("current-setting");

    let pane = gtk::Paned::new(gtk::Orientation::Horizontal);
    pane.set_vexpand(true);
    pane.set_position(220);
    pane.set_resize_start_child(false);
    pane.set_shrink_start_child(false);
    root.append(&pane);
    let list = Rc::new(RefCell::new(ThemeList::new()));
    let refresh = list.borrow().refresh.clone();
    pane.set_start_child(Some(&list.borrow().root));
    let right = gtk::Box::new(gtk::Orientation::Vertical, 0);
    right.set_hexpand(true);
    pane.set_end_child(Some(&right));
    let preview = PreviewPane::new();
    preview.configure_inspect();
    preview.append_details(&trial.source_details);
    let try_button = trial.popout.clone();
    try_button.set_sensitive(false);
    let detail_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&preview.root)
        .build();
    let views = gtk::Stack::new();
    views.set_vexpand(true);
    let host = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let detached =
        label("Playground is open in the trial window. Close that window to return it here.");
    detached.add_css_class("detached-message");
    host.append(&detached);
    trial.embed(&host);
    let playground_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&host)
        .build();
    views.add_titled(&playground_scroll, Some("playground"), "Playground");
    let inspect = gtk::Paned::new(gtk::Orientation::Horizontal);
    inspect.set_start_child(Some(&preview.role_browser.root));
    inspect.set_end_child(Some(&detail_scroll));
    inspect.set_position(208);
    inspect.set_resize_start_child(false);
    inspect.set_shrink_start_child(false);
    views.add_titled(&inspect, Some("inspect"), "Inspect");
    let switcher = gtk::StackSwitcher::builder().stack(&views).build();
    header.set_title_widget(Some(&switcher));
    views.set_visible_child_name(if smoke { "inspect" } else { "playground" });
    right.append(&views);
    right.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    footer.add_css_class("action-area");
    right.append(&footer);
    let capability = label(if fixture {
        "Read-only preview. Desktop settings are unchanged."
    } else if port.capability().writable {
        "GNOME settings are available. Theme lookup paths are unverified; check the actual pointer after applying."
    } else {
        "Desktop settings are unavailable in this session. Browsing and preview remain available."
    });
    capability.add_css_class("dim-label");
    capability.set_tooltip_text(Some(&port.capability().details.join("\n")));
    preview.append_details(&capability);
    preview.append_details(&current);
    let opt = gtk::CheckButton::with_label("Apply size");
    opt.set_tooltip_text(Some(
        "Include cursor size in the next Apply. Unchecked keeps the desktop size.",
    ));
    let setting_size = gtk::SpinButton::with_range(1., 256., 1.);
    setting_size.set_value(size(&state.borrow().current) as f64);
    setting_size.set_sensitive(false);
    footer.append(&opt);
    footer.append(&setting_size);
    let status = label("Finding themes…");
    status.add_css_class("operation-status");
    status.set_hexpand(true);
    let visible_status = status.clone();
    let status_message = label("");
    footer.append(&status);
    let apply = gtk::Button::with_label("Apply theme");
    apply.add_css_class("primary-action");
    apply.set_sensitive(false);
    let undo = gtk::Button::with_label("Undo");
    undo.set_sensitive(false);
    footer.append(&undo);
    footer.append(&apply);
    // Operational errors stay visible in the action bar. Routine preview text is
    // rendered as a concise state below, without hiding error diagnostics.
    let status = status_message;
    {
        let n = setting_size.clone();
        opt.connect_toggled(move |b| n.set_sensitive(b.is_active()));
    }
    let request: Rc<dyn Fn()> = {
        let s = state.clone();
        let worker = worker.clone();
        let p = preview.clone();
        let status = status.clone();
        Rc::new(move || {
            p.clear();
            let mut s = s.borrow_mut();
            if s.scanning {
                return;
            }
            if let (Some(t), Some(role)) = (s.browser.selected.clone(), p.roles.active_id()) {
                s.request = worker.submit(Job::Preview(t, role.to_string()));
                p.roles
                    .set_tooltip_text(Some(&format!("Cursor file: {role}")));
                status.set_text("Loading preview…");
            }
        })
    };
    {
        let request = request.clone();
        preview.roles.connect_changed(move |_| request());
    }
    {
        let s = state.clone();
        let p = preview.clone();
        let list_widget = list.borrow().list.clone();
        list_widget.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let theme = {
                let s = s.borrow();
                s.catalog.themes.get(row.index() as usize).cloned()
            };
            if let Some(t) = theme {
                s.borrow_mut().browser.selected = Some(t.name.clone());
                p.title.set_text(&t.display);
                p.title
                    .set_tooltip_text(Some(&format!("{} · {}", t.display, t.name.as_str())));
                p.subtitle.set_text(&format!(
                    "{} · {} source location(s){}",
                    t.availability.label(),
                    t.locations.len(),
                    if t.issues.is_empty() {
                        ""
                    } else {
                        " · Some roles could not be verified"
                    }
                ));
                p.subtitle.set_tooltip_text(
                    (!t.issues.is_empty())
                        .then(|| t.issues.join("\n"))
                        .as_deref(),
                );
                p.role_browser.rebuild(&role_choices(&t.verified_roles));
            }
        });
    }
    {
        let s = state.clone();
        let list = list.clone();
        let search = list.borrow().search.clone();
        search.connect_search_changed(move |entry| {
            let mut s = s.borrow_mut();
            s.browser.query = entry.text().to_string();
            list.borrow().filter(&s.catalog.themes, &s.browser.query);
        });
    }
    let rescan: Rc<dyn Fn()> = {
        let s = state.clone();
        let worker = worker.clone();
        let p = preview.clone();
        let status = status.clone();
        Rc::new(move || {
            p.clear();
            let mut s = s.borrow_mut();
            s.scanning = true;
            s.request = worker.submit(Job::Scan);
            s.catalog_epoch += 1;
            status.set_text("Checking cursor sources…");
        })
    };
    {
        let rescan = rescan.clone();
        refresh.connect_clicked(move |_| rescan());
    }
    let importer = {
        let s = state.clone();
        let rescan = rescan.clone();
        let search = list.borrow().search.clone();
        let installed = Rc::new(move |name: ThemeName| {
            {
                let mut s = s.borrow_mut();
                s.browser.selected = Some(name);
                s.browser.initialized = true;
                s.browser.query.clear();
            }
            search.set_text("");
            rescan();
        });
        let targets = if smoke {
            vec![import_test_root.clone()]
        } else if fixture {
            Vec::new()
        } else {
            cursormochi_platform::import::user_targets(&env)
                .into_iter()
                .filter(|p| paths.resolution.contains(p))
                .collect()
        };
        super::import::Importer::new(&window, targets, repo, installed)
    };
    {
        let importer = importer.clone();
        window.connect_close_request(move |_| {
            let busy = importer.is_busy();
            importer.close();
            if busy {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    let import_button = gtk::Button::with_label("Import…");
    header.pack_start(&import_button);
    {
        let importer = importer.clone();
        import_button.connect_clicked(move |_| importer.present());
    }

    {
        let s = state.clone();
        let env = env.clone();
        let paths = paths.clone();
        let port = port.clone();
        let p = preview.clone();
        let w = window.downgrade();
        copy.connect_clicked(move |_|{if let Some(w)=w.upgrade(){let s=s.borrow();w.clipboard().set_text(&redact(&format!("CursorMochi {}\n{POLICY}\n{paths:?}\nGTK {}.{}.{}; backend={:?}\ncapability={:?}\nsettings={:?}\nselected={:?}\npreview={}\ncandidates={:?}\nscan={:?}\noperation={:?}\nmessages={:?}",env!("CARGO_PKG_VERSION"),gtk::major_version(),gtk::minor_version(),gtk::micro_version(),gdk::Display::default().map(|d|d.type_().name()),port.capability(),port.read(),s.browser.selected,p.source_summary(),s.catalog.candidates,s.catalog.diagnostics,s.controller.last_operation,s.controller.messages),&env));}});
    }
    {
        let s = state.clone();
        let env = env.clone();
        let w = window.downgrade();
        candidates.connect_clicked(move |_| {
            if let Some(w) = w.upgrade() {
                let dialog = gtk::Window::builder()
                    .title("Candidate diagnostics")
                    .transient_for(&w)
                    .modal(true)
                    .default_width(620)
                    .default_height(420)
                    .build();
                let text = {
                    let s = s.borrow();
                    format!(
                        "{} verified themes · {} excluded candidates\n\n{}\n\n{}",
                        s.catalog.themes.len(),
                        s.catalog.candidates.len(),
                        s.catalog
                            .candidates
                            .iter()
                            .map(|t| format!(
                                "{} · {}\n{:?}",
                                t.name.as_str(),
                                t.availability.label(),
                                t.availability
                            ))
                            .collect::<Vec<_>>()
                            .join("\n\n"),
                        s.catalog.diagnostics.join("\n")
                    )
                };
                let l = label(&redact(&text, &env));
                l.set_selectable(true);
                l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                l.add_css_class("menu-content");
                dialog.set_child(Some(&gtk::ScrolledWindow::builder().child(&l).build()));
                dialog.present();
            }
        });
    }
    {
        let s = state.clone();
        let port = port.clone();
        let p = preview.clone();
        let opt = opt.clone();
        let n = setting_size.clone();
        let status = status.clone();
        apply.connect_clicked(move |_| {
            let mut s = s.borrow_mut();
            let Some(theme) = s.browser.selected.clone() else {
                return;
            };
            let valid = s
                .catalog
                .themes
                .iter()
                .any(|t| t.name == theme && t.availability.usable())
                && p.has_data()
                && !s.scanning;
            match s.controller.prepare(
                port.as_ref(),
                ChangeIntent {
                    theme,
                    size: opt.is_active().then(|| n.value_as_int()),
                    resolved: valid,
                },
            ) {
                Ok(plan) => {
                    let result = s.controller.begin(port.as_ref(), plan, false);
                    s.started = Instant::now();
                    status.set_text(&outcome(&result, &s.controller));
                }
                Err(e) => status.set_text(&e),
            }
        });
    }
    {
        let s = state.clone();
        let port = port.clone();
        let status = status.clone();
        undo.connect_clicked(move |_| {
            let mut s = s.borrow_mut();
            let result = s.controller.begin_undo(port.as_ref());
            s.started = Instant::now();
            status.set_text(&outcome(&result, &s.controller));
        });
    }
    {
        let s = state.clone();
        let status = status.clone();
        let trial = trial.clone();
        window.connect_close_request(move |_| {
            if s.borrow().controller.busy() {
                status.set_text("A change is being verified. Please wait before closing.");
                glib::Propagation::Stop
            } else {
                trial.clear();
                trial.window.destroy();
                glib::Propagation::Proceed
            }
        });
    }
    rescan();
    let weak = window.downgrade();
    let mut settings_at = Instant::now() - Duration::from_secs(1);
    let mut thumbs_at = settings_at;
    let mut pending_thumb: Option<(u64, ThumbnailTarget, u64)> = None;
    let mut roles_turn = true;
    let mut writable = port.capability().writable;
    let mut current_readable = true;
    let mut smoke_run = super::smoke::Smoke::new(smoke, &state.borrow().current);
    let mut trial_smoke = super::trial_smoke::TrialSmoke::default();
    let mut import_smoke = super::import_smoke::ImportSmoke::default();
    let mut current_smoke = super::trial::current::Smoke::default();
    let app = app.clone();
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let Some(window) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let trial_testing = smoke_run.trial_ready() && !trial_smoke.done();
        let import_testing = smoke_run.trial_ready() && trial_smoke.done() && !import_smoke.done();
        let current_testing = smoke_run.trial_ready()
            && trial_smoke.done()
            && import_smoke.done()
            && !current_smoke.done();
        if !trial_testing
            && !import_testing
            && !current_testing
            && smoke_run.before_tick(&window, &root, &app)
        {
            return glib::ControlFlow::Continue;
        }
        while let Ok((id, out)) = rx.try_recv() {
            if id != state.borrow().request {
                continue;
            }
            match out {
                Output::Scan(Ok(mut catalog)) => {
                    cursormochi_app::browser::user_themes_first(
                        &mut catalog.themes,
                        &user_theme_roots,
                    );
                    pending_thumb = None;
                    let notice = {
                        let mut s = state.borrow_mut();
                        s.catalog = catalog;
                        s.scanning = false;
                        let current = current_theme(&s.current).map(str::to_owned);
                        let themes = s.catalog.themes.clone();
                        s.browser.reconcile(&themes, current.as_deref())
                    };
                    {
                        let s = state.borrow();
                        let mut l = list.borrow_mut();
                        l.rebuild(&s.catalog.themes, current_theme(&s.current));
                        l.filter(&s.catalog.themes, &s.browser.query);
                    }
                    let selection = {
                        let s = state.borrow();
                        s.browser
                            .selected
                            .as_ref()
                            .and_then(|name| s.catalog.themes.iter().position(|t| &t.name == name))
                    };
                    let target = selection.and_then(|i| list.borrow().list.row_at_index(i as i32));
                    if let Some(row) = target {
                        list.borrow().list.select_row(Some(&row));
                        let row = row.clone();
                        let scroll = list.borrow().scroll.clone();
                        let list_widget = list.borrow().list.clone();
                        glib::idle_add_local_once(move || {
                            if let Some(b) = row.compute_bounds(&list_widget) {
                                scroll
                                    .vadjustment()
                                    .clamp_page(b.y() as f64, (b.y() + b.height()) as f64);
                            }
                        });
                    } else {
                        preview.clear();
                        preview.role_browser.rebuild(&[]);
                        preview.title.set_text("Choose a cursor theme");
                        preview
                            .subtitle
                            .set_text("Select a verified theme from the list.");
                    }
                    status.set_text(&notice.unwrap_or_else(|| {
                        format!(
                            "{} verified themes · {} candidates excluded",
                            state.borrow().catalog.themes.len(),
                            state.borrow().catalog.candidates.len()
                        )
                    }));
                }
                Output::Preview(Ok(p)) => {
                    preview.load(p, &env);
                    status.set_text("Your desktop settings are unchanged.");
                }
                Output::Scan(Err(e)) => {
                    let mut s = state.borrow_mut();
                    s.scanning = false;
                    s.browser.selected = None;
                    s.catalog = Catalog::default();
                    preview.clear();
                    drop(s);
                    preview.role_browser.rebuild(&[]);
                    list.borrow_mut().rebuild(&[], None);
                    list.borrow().filter(&[], "");
                    list.borrow().empty.set_text(
                        "Could not read cursor themes.\nTry Refresh or open diagnostics.",
                    );
                    status.set_text(&format!("Scan failed: {e}"));
                }
                Output::Preview(Err(e)) => {
                    preview.clear();
                    preview.warning.set_text(&format!(
                        "Preview unavailable: {e}. Try another role or refresh."
                    ));
                    preview.warning.set_visible(true);
                    status
                        .set_text("Cannot apply this selection while its preview is unavailable.");
                }
                Output::Thumbnail(_) | Output::Trial(_) | Output::Audit(_) => (),
            }
        }
        while let Ok((id, out)) = thumb_rx.try_recv() {
            if let Some((expected, target, epoch)) = pending_thumb
                && id == expected
            {
                pending_thumb = None;
                if epoch == state.borrow().catalog_epoch {
                    match target {
                        ThumbnailTarget::Role { generation, index } => {
                            if let Output::Thumbnail(result) = out {
                                preview.role_browser.finish(generation, index, result);
                            }
                        }
                        ThumbnailTarget::Theme(index) => {
                            let mut l = list.borrow_mut();
                            let near = l.near_indices();
                            if near.contains(&index)
                                && let Some(row) = l.rows.get_mut(index)
                            {
                                row.thumb_done = true;
                                if let Output::Thumbnail(Ok((frame, _))) = out {
                                    row.image.set_paintable(
                                        super::thumbnail::paintable(&frame).as_ref(),
                                    );
                                } else {
                                    row.image.set_tooltip_text(Some(
                                        "Thumbnail unavailable. Refresh to verify the source.",
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        let now = Instant::now();
        let mut s = state.borrow_mut();
        if s.controller.busy() {
            let expired = s.started.elapsed() > Duration::from_secs(2);
            let result = s.controller.poll(port.as_ref(), expired);
            if result != Status::VerificationPending {
                status.set_text(&outcome(&result, &s.controller));
            }
        }
        if now.duration_since(settings_at) >= Duration::from_millis(250) {
            settings_at = now;
            writable = port.capability().writable;
            match port.read() {
                Ok(snapshot) => {
                    current_readable = true;
                    if status.text().starts_with("Current settings unavailable:") {
                        status.set_text("");
                    }
                    s.current = snapshot;
                    let name = current_theme(&s.current).unwrap_or("Unknown");
                    let missing = if !s.scanning
                        && !s.catalog.themes.iter().any(|t| t.name.as_str() == name)
                    {
                        " · Not in the verified list"
                    } else {
                        ""
                    };
                    current.set_text(&format!(
                        "Current setting: {name} · Cursor size {}{missing}",
                        size(&s.current)
                    ));
                    list.borrow().update_current(Some(name));
                    if !opt.is_active() {
                        setting_size.set_value(size(&s.current) as f64);
                    }
                }
                Err(e) => {
                    current_readable = false;
                    current.set_text(&format!("Current settings unavailable: {e}"));
                    status.set_text(&format!("Current settings unavailable: {e}"));
                }
            }
        }
        let selected = s.browser.selected.as_ref().map(ThemeName::as_str);
        let same = selected.is_some()
            && selected == current_theme(&s.current)
            && (!opt.is_active() || setting_size.value_as_int() == size(&s.current));
        let busy = s.controller.busy();
        apply.set_label("Apply theme");
        let message = status.text();
        let routine = message.is_empty()
            || message == "Your desktop settings are unchanged."
            || message.contains("verified themes")
            || message.starts_with("Already in use.")
            || message == "Settings updated. Please check your actual pointer.";
        visible_status.set_text(if routine {
            if same {
                "In use"
            } else if !writable {
                "Read-only"
            } else {
                "Ready"
            }
        } else {
            &message
        });
        visible_status.set_tooltip_text(Some(&message));
        let reason = if busy {
            "A settings change is being verified."
        } else if s.scanning {
            "Wait for theme verification."
        } else if !writable {
            "Desktop settings are read-only in this session."
        } else if !preview.has_data() {
            "Select a verified theme and load its preview first."
        } else if same {
            "This theme and the requested size already match the current settings."
        } else {
            "Apply the selected theme to GNOME settings."
        };
        apply.set_tooltip_text(Some(reason));
        apply.set_sensitive(writable && !busy && !s.scanning && preview.has_data() && !same);
        undo.set_label("Undo");
        undo.set_sensitive(writable && !busy && s.controller.undo.is_some());
        undo.set_tooltip_text(Some(if s.controller.undo.is_some() {
            "Restore this session's last successful change, unless settings changed externally."
        } else {
            "No change to undo in this session."
        }));
        refresh.set_sensitive(!busy && !s.scanning);
        list.borrow().root.set_sensitive(!s.scanning);
        preview.roles.set_sensitive(!s.scanning && !busy);
        preview
            .role_browser
            .root
            .set_sensitive(!s.scanning && !busy);
        opt.set_sensitive(!busy && writable);
        setting_size.set_sensitive(!busy && writable && opt.is_active());
        let visible = window.is_mapped()
            && window
                .surface()
                .and_then(|v| v.downcast::<gdk::Toplevel>().ok())
                .is_none_or(|v| !v.state().contains(gdk::ToplevelState::MINIMIZED));
        preview.tick(visible && views.visible_child_name().as_deref() == Some("inspect"));
        if visible && !s.scanning && now.duration_since(thumbs_at) >= Duration::from_millis(80) {
            thumbs_at = now;
            let mut l = list.borrow_mut();
            let near = l.near_indices();
            let role_near = preview.role_browser.near_indices();
            let generation = preview.role_browser.generation.get();
            if pending_thumb.is_some_and(|(_, target, _)| match target {
                ThumbnailTarget::Theme(i) => !near.contains(&i),
                ThumbnailTarget::Role {
                    generation: old,
                    index,
                } => old != generation || !role_near.contains(&index),
            }) {
                pending_thumb = None;
            }
            if pending_thumb.is_none() {
                let theme = near
                    .into_iter()
                    .find(|i| !l.rows[*i].thumb_done)
                    .and_then(|i| {
                        s.catalog.themes.get(i).and_then(|t| {
                            t.availability.role().map(|role| {
                                (ThumbnailTarget::Theme(i), t.name.clone(), role.to_string())
                            })
                        })
                    });
                let roles = preview.role_browser.rows.borrow();
                let role = role_near
                    .into_iter()
                    .find(|i| !roles[*i].done)
                    .and_then(|index| {
                        s.browser.selected.clone().map(|theme| {
                            (
                                ThumbnailTarget::Role { generation, index },
                                theme,
                                roles[index].id.clone(),
                            )
                        })
                    });
                let next = if roles_turn {
                    role.or(theme)
                } else {
                    theme.or(role)
                };
                if let Some((target, theme, role)) = next {
                    roles_turn = !roles_turn;
                    let id = thumb_worker.submit(Job::Thumbnail(
                        theme,
                        role,
                        44 * window.scale_factor().max(1) as u32,
                    ));
                    pending_thumb = Some((id, target, s.catalog_epoch));
                }
            }
        }
        let selected = if s.scanning {
            None
        } else {
            s.browser.selected.clone()
        };
        try_button.set_sensitive(selected.is_some());
        trial.select(selected, s.catalog_epoch);
        trial.tick();
        current_test.tick(
            current_readable
                .then(|| current_theme(&s.current))
                .flatten(),
            if current_readable {
                size(&s.current)
            } else {
                0
            },
            s.catalog_epoch,
        );
        drop(s);
        detached.set_visible(trial.window.is_visible());
        if trial_testing {
            trial_smoke.tick(&trial, &state, &list, &try_button, &views, &window);
        }
        if import_testing {
            import_smoke.tick(&importer, &state, &import_test_root);
            if import_smoke.done() {
                window.present();
                smoke_run.settle_captures();
            }
        }
        if current_testing {
            current_smoke.tick(&current_test, &test_current);
            if current_smoke.done() {
                window.present();
                smoke_run.settle_captures();
            }
        }
        smoke_run.tick(
            &window,
            &state,
            &list,
            &preview,
            &apply,
            &undo,
            &menu,
            &copy,
            &opt,
            &setting_size,
            &rescan,
            &port,
        );
        glib::ControlFlow::Continue
    });
    window.present();
}
